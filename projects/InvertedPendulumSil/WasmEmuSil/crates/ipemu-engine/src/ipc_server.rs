//! IPC サーバ: ループバック WebSocket (tokio + tungstenite)。07-client-app.md「IPC 契約」。
//!
//! - `127.0.0.1:<ephemeral>` で待ち受け、接続時にトークンを要求する (`?token=` または `Authorization: Bearer`)
//! - 文字列フレーム = JSON-RPC 2.0 の要求 → 制御器に渡して応答を返す
//! - バイナリフレーム = state (トリプルバッファから `state_hz` で読む) と scope、通知の JSON を全接続へ配る
//! - 1 シェル + 複数の観測専用接続 (CLI / テスト) を許す。遅い受け手は古いフレームを取りこぼす (broadcast)
//! - ブラウザからの接続 (`Origin` ヘッダあり) は許可リスト (既定 `app://ui`) に一致しなければトークンが正しくても拒否する。
//!   トークン漏えい時に任意の Web ページがローカルエンジンへつながる経路を塞ぐ多層防御。`Origin` の無い接続
//!   (CLI / テスト / シェル main) はトークンのみ

use crate::consent_ipc::ConsentBroker;
use crate::control::{ControllerHandle, Outbound};
use crate::triple_buffer::Reader;
use futures_util::{SinkExt, StreamExt};
use ipemu_ipc::*;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::handshake::server::{
    ErrorResponse, Request as HsRequest, Response as HsResponse,
};
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 0 で ephemeral
    pub port: u16,
    pub token: String,
    /// state フレームの送信レート
    pub state_hz: f64,
    /// `Origin` ヘッダ付き (ブラウザ) の接続で許す origin。既定は `app://ui` だけ
    pub allowed_origins: Vec<String>,
}

/// 既定の許可 origin (シェルが配信するシミュレータ UI)。
pub const DEFAULT_ALLOWED_ORIGINS: &[&str] = &["app://ui"];

/// 起動済みサーバ。
pub struct Server {
    pub addr: SocketAddr,
    pub token: String,
    pub discovery: Discovery,
    accept_task: tokio::task::JoinHandle<()>,
    pump_task: tokio::task::JoinHandle<()>,
}

impl Server {
    pub fn abort(&self) {
        self.accept_task.abort();
        self.pump_task.abort();
    }
    /// 終了するまで待つ (通常は abort されるまで戻らない)。
    pub async fn wait(self) {
        let _ = self.accept_task.await;
        let _ = self.pump_task.await;
    }
}

/// ランダムなトークン (32 バイト hex)。
pub fn random_token() -> String {
    use rand::RngCore;
    let mut b = [0u8; 32];
    rand::rng().fill_bytes(&mut b);
    hex::encode(b)
}

/// サーバを起動する。戻った時点で待ち受けている (port 確定)。
pub async fn start(
    cfg: ServerConfig,
    ctrl: ControllerHandle,
    mut state_r: Reader<StateFrame>,
    mut out_rx: tokio::sync::mpsc::UnboundedReceiver<Outbound>,
    consent: Option<Arc<ConsentBroker>>,
) -> anyhow::Result<Server> {
    let listener = TcpListener::bind(("127.0.0.1", cfg.port)).await?;
    let addr = listener.local_addr()?;
    let (bcast, _) = broadcast::channel::<Arc<Message>>(1024);

    // 配信ポンプ: state は state_hz でトリプルバッファを読み、通知 / scope は来た順に流す
    let pump = {
        let bcast = bcast.clone();
        let period = Duration::from_secs_f64(1.0 / cfg.state_hz.max(1.0));
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(period);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = tick.tick() => {
                        if state_r.update() {
                            if let Ok(bytes) = encode_frame(FrameKind::State, state_r.front()) {
                                let _ = bcast.send(Arc::new(Message::Binary(bytes.into())));
                            }
                        }
                    }
                    o = out_rx.recv() => match o {
                        Some(Outbound::Text(t)) => { let _ = bcast.send(Arc::new(Message::Text(t.into()))); }
                        Some(Outbound::Binary(b)) => { let _ = bcast.send(Arc::new(Message::Binary(b.into()))); }
                        None => break,
                    }
                }
            }
        })
    };

    let token = Arc::new(cfg.token.clone());
    let origins: Arc<Vec<String>> = Arc::new(
        cfg.allowed_origins
            .iter()
            .map(|o| o.trim().trim_end_matches('/').to_ascii_lowercase())
            .filter(|o| !o.is_empty())
            .collect(),
    );
    let accept = {
        let bcast = bcast.clone();
        let token = token.clone();
        let origins = origins.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, peer)) = listener.accept().await else {
                    continue;
                };
                let ctrl = ctrl.clone();
                let rx = bcast.subscribe();
                let token = token.clone();
                let consent = consent.clone();
                let origins = origins.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        handle_connection(stream, peer, token, origins, ctrl, rx, consent).await
                    {
                        eprintln!("ipc: {peer}: {e}");
                    }
                });
            }
        })
    };

    let discovery = Discovery {
        port: addr.port(),
        token: cfg.token.clone(),
        pid: std::process::id(),
        ipc_version: IPC_VERSION,
        engine_version: env!("CARGO_PKG_VERSION").into(),
    };
    Ok(Server {
        addr,
        token: cfg.token,
        discovery,
        accept_task: accept,
        pump_task: pump,
    })
}

/// ハンドシェイクでトークンを照合する。`?token=` か `Authorization: Bearer`。
fn check_token(req: &HsRequest, expected: &str) -> bool {
    let from_query = req.uri().query().and_then(|q| {
        q.split('&').find_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            (k == "token").then(|| percent_decode(v))
        })
    });
    let from_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(str::to_owned);
    let got = from_query.or(from_header);
    // 長さ一致 + 定数時間比較 (ループバックとはいえ癖をつけておく)
    match got {
        Some(g) if g.len() == expected.len() => {
            g.bytes()
                .zip(expected.bytes())
                .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                == 0
        }
        _ => false,
    }
}

/// `Origin` ヘッダの検査。ヘッダが無ければ (ブラウザ以外) 通す。あれば許可リストとの厳密一致 (大文字小文字と末尾 `/` は無視)。
fn check_origin(req: &HsRequest, allowed: &[String]) -> bool {
    match req.headers().get("origin").and_then(|h| h.to_str().ok()) {
        None => true,
        Some(o) => {
            let o = o.trim().trim_end_matches('/').to_ascii_lowercase();
            allowed.contains(&o)
        }
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[allow(clippy::result_large_err)] // accept_hdr_async のコールバック型 (ErrorResponse) に合わせる
async fn handle_connection(
    stream: TcpStream,
    _peer: SocketAddr,
    token: Arc<String>,
    origins: Arc<Vec<String>>,
    ctrl: ControllerHandle,
    mut rx: broadcast::Receiver<Arc<Message>>,
    consent: Option<Arc<ConsentBroker>>,
) -> anyhow::Result<()> {
    let tok = token.clone();
    #[allow(clippy::result_large_err)] // tungstenite のコールバック型に合わせる
    fn gate(
        token_ok: bool,
        origin_ok: bool,
        resp: HsResponse,
    ) -> Result<HsResponse, ErrorResponse> {
        use tokio_tungstenite::tungstenite::http::StatusCode;
        if !token_ok {
            let mut r = ErrorResponse::new(Some("unauthorized".into()));
            *r.status_mut() = StatusCode::UNAUTHORIZED;
            return Err(r);
        }
        if !origin_ok {
            // トークンは合っているが、許可していない Web origin からの接続
            let mut r = ErrorResponse::new(Some("origin not allowed".into()));
            *r.status_mut() = StatusCode::FORBIDDEN;
            return Err(r);
        }
        Ok(resp)
    }
    let ws =
        tokio_tungstenite::accept_hdr_async(stream, move |req: &HsRequest, resp: HsResponse| {
            gate(check_token(req, &tok), check_origin(req, &origins), resp)
        })
        .await?;
    let (mut sink, mut source) = ws.split();
    // 応答と配信を 1 本のキューに合流させる (sink は 1 箇所からしか書かない)
    let (tx, mut q) = tokio::sync::mpsc::channel::<Message>(256);

    let writer = tokio::spawn(async move {
        loop {
            tokio::select! {
                m = q.recv() => match m {
                    Some(m) => { if sink.send(m).await.is_err() { break; } }
                    None => break,
                },
                b = rx.recv() => match b {
                    Ok(m) => { if sink.send((*m).clone()).await.is_err() { break; } }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue, // 遅い受け手: 古いフレームは捨てる
                    Err(broadcast::error::RecvError::Closed) => break,
                },
            }
        }
        let _ = sink.close().await;
    });

    while let Some(msg) = source.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(_) => break,
        };
        match msg {
            Message::Text(text) => {
                let reply = match Request::parse(&text) {
                    Ok(req) => {
                        let id = req.id.clone();
                        // 同意の回答は制御器を経由しない (制御器は launch 中にブロックして回答を待っている)
                        let result = match (&req.body, &consent) {
                            (RequestBody::ConsentAnswer(a), Some(b)) => b
                                .answer(ipemu_core::consent::ConsentAnswer {
                                    request_id: a.request_id.clone(),
                                    grant: a.grant,
                                    remember: a.remember,
                                })
                                .map(|_| serde_json::json!({})),
                            _ => ctrl.call(req.body).await,
                        };
                        match result {
                            Ok(v) => serde_json::to_string(&Response {
                                jsonrpc: "2.0".into(),
                                id,
                                result: Some(v),
                                error: None,
                            }),
                            Err(e) => serde_json::to_string(&Response::err(id, e)),
                        }
                    }
                    Err((Some(id), e)) => serde_json::to_string(&Response::err(id, e)),
                    Err((None, e)) => serde_json::to_string(&Response::err_no_id(e)),
                };
                if let Ok(r) = reply {
                    if tx.send(Message::Text(r.into())).await.is_err() {
                        break;
                    }
                }
            }
            Message::Ping(p) => {
                let _ = tx.send(Message::Pong(p)).await;
            }
            Message::Close(_) => break,
            // バイナリは受け付けない (操作はすべて JSON-RPC)
            _ => {}
        }
    }
    drop(tx);
    let _ = writer.await;
    Ok(())
}

/// 接続クライアント (CLI の `attach` とテスト用)。
pub mod client {
    use super::*;
    use std::collections::HashMap;
    use tokio::sync::{oneshot, Mutex};

    pub type WsStream =
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

    /// 受信したストリーム (state / scope / 通知) を受け取る側。
    #[derive(Debug, Clone)]
    pub enum Incoming {
        State(StateFrame),
        Scope(ScopeFrame),
        Notification(Notification),
        /// 解釈できなかった文字列 (応答以外)
        Other(String),
    }

    /// ハンドシェイクだけを試す (テスト用)。`origin` を渡すとブラウザのように `Origin` ヘッダを付ける。
    /// 成功なら接続を閉じて Ok、拒否なら HTTP 状態コード。
    pub async fn probe_handshake(
        port: u16,
        token: &str,
        origin: Option<&str>,
    ) -> Result<(), Option<u16>> {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let url = format!("ws://127.0.0.1:{port}/?token={token}");
        let mut req = url.into_client_request().map_err(|_| None)?;
        if let Some(o) = origin {
            req.headers_mut()
                .insert("origin", o.parse().map_err(|_| None)?);
        }
        match tokio_tungstenite::connect_async(req).await {
            Ok((mut ws, _)) => {
                let _ = ws.close(None).await;
                Ok(())
            }
            Err(tokio_tungstenite::tungstenite::Error::Http(resp)) => {
                Err(Some(resp.status().as_u16()))
            }
            Err(_) => Err(None),
        }
    }

    pub struct Client {
        tx: tokio::sync::mpsc::Sender<Message>,
        pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Response>>>>,
        next_id: std::sync::atomic::AtomicI64,
        pub incoming: tokio::sync::mpsc::UnboundedReceiver<Incoming>,
        _reader: tokio::task::JoinHandle<()>,
        _writer: tokio::task::JoinHandle<()>,
    }

    impl Client {
        pub async fn connect(port: u16, token: &str) -> anyhow::Result<Self> {
            let url = format!("ws://127.0.0.1:{port}/?token={token}");
            let (ws, _) = tokio_tungstenite::connect_async(url).await?;
            let (mut sink, mut source) = ws.split();
            let (tx, mut q) = tokio::sync::mpsc::channel::<Message>(64);
            let writer = tokio::spawn(async move {
                while let Some(m) = q.recv().await {
                    if sink.send(m).await.is_err() {
                        break;
                    }
                }
            });
            let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Response>>>> = Default::default();
            let (in_tx, incoming) = tokio::sync::mpsc::unbounded_channel();
            let p2 = pending.clone();
            let reader = tokio::spawn(async move {
                while let Some(Ok(m)) = source.next().await {
                    match m {
                        Message::Text(t) => {
                            if let Ok(r) = serde_json::from_str::<Response>(&t) {
                                if let RequestId::Num(n) = r.id {
                                    if let Some(w) = p2.lock().await.remove(&n) {
                                        let _ = w.send(r);
                                        continue;
                                    }
                                }
                            }
                            if let Ok(n) = serde_json::from_str::<Notification>(&t) {
                                let _ = in_tx.send(Incoming::Notification(n));
                            } else {
                                let _ = in_tx.send(Incoming::Other(t.to_string()));
                            }
                        }
                        Message::Binary(b) => {
                            if let Some(f) = decode_frame::<StateFrame>(FrameKind::State, &b) {
                                let _ = in_tx.send(Incoming::State(f));
                            } else if let Some(f) = decode_frame::<ScopeFrame>(FrameKind::Scope, &b)
                            {
                                let _ = in_tx.send(Incoming::Scope(f));
                            }
                        }
                        Message::Close(_) => break,
                        _ => {}
                    }
                }
            });
            Ok(Self {
                tx,
                pending,
                next_id: std::sync::atomic::AtomicI64::new(1),
                incoming,
                _reader: reader,
                _writer: writer,
            })
        }

        /// 要求を送って応答を待つ。
        pub async fn call(&self, body: RequestBody) -> Result<serde_json::Value, RpcError> {
            let id = self
                .next_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let (w, r) = oneshot::channel();
            self.pending.lock().await.insert(id, w);
            let req = Request::new(id, body);
            let text = serde_json::to_string(&req)
                .map_err(|e| RpcError::new(codes::INTERNAL_ERROR, e.to_string()))?;
            self.tx
                .send(Message::Text(text.into()))
                .await
                .map_err(|_| RpcError::new(codes::INTERNAL_ERROR, "接続が閉じています"))?;
            let resp = r
                .await
                .map_err(|_| RpcError::new(codes::INTERNAL_ERROR, "応答なしで接続が閉じました"))?;
            match (resp.result, resp.error) {
                (_, Some(e)) => Err(e),
                (Some(v), None) => Ok(v),
                (None, None) => Ok(serde_json::Value::Null),
            }
        }

        /// 型付きの呼び出し。
        pub async fn call_as<T: serde::de::DeserializeOwned>(
            &self,
            body: RequestBody,
        ) -> Result<T, RpcError> {
            let v = self.call(body).await?;
            serde_json::from_value(v)
                .map_err(|e| RpcError::new(codes::INTERNAL_ERROR, format!("応答の形が不正: {e}")))
        }
    }
}
