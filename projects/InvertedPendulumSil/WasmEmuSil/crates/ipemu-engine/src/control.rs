//! スケジューラスレッド上で動く制御器。IPC からの操作はチャネルに積まれ、**ステップ境界でだけ**状態を変える
//! (02-architecture.md「プロセスとスレッド」)。Wasmtime の Store を触るのはこのスレッドだけ。
//!
//! - 開始 / 停止 / リセット / ステップ、ペーシング変更
//! - パラメータ変更 (wasm 部品は `init` し直し)、ファームウェアの差し替え (wasm ホット / native は再起動要求)
//! - スナップショットをトリプルバッファへ、イベント・スコープを送信キューへ
//! - `firmware.watch`: ビルド成果物の更新を検知して自動で差し替える

use crate::bootstrap::{bootstrap, BootstrapOptions, Telemetry};
use crate::cache::Cache;
use crate::pacing::{Due, Pacer, MAX_BATCH};
use crate::session::{FirmwareSource, FirmwareSwap, ParamChange, Session, SessionOptions};
use crate::triple_buffer::{triple_buffer, Reader, Writer};
use anyhow::Context as _;
use ipemu_core::hal::LogLevel;
use ipemu_ipc::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant, SystemTime};

/// I/O 側から制御器へ送る命令。
pub enum Command {
    Rpc {
        body: RequestBody,
        reply: tokio::sync::oneshot::Sender<Result<serde_json::Value, RpcError>>,
    },
    Shutdown,
}

/// 制御器から I/O 側へ流すもの (通知の JSON 文字列、またはバイナリフレーム)。
#[derive(Debug, Clone)]
pub enum Outbound {
    Text(String),
    Binary(Vec<u8>),
}

/// 制御器の設定 (CLI の引数から)。
#[derive(Clone)]
pub struct ControllerConfig {
    pub debug_friendly: bool,
    /// `catalog.local` が走査するディレクトリ (`*.manifest.json`)
    pub catalog_dirs: Vec<PathBuf>,
    /// 監視ファイルを確認する間隔
    pub watch_interval: Duration,
    /// データディレクトリ (同意ストアと fs capability の実体)。None なら OS 既定
    pub data_dir: Option<PathBuf>,
    /// 同意の問い合わせ先 (IPC の ConsentBroker、または --consent-file)。None なら未同意は起動不可
    pub consent: Option<std::sync::Arc<dyn crate::consent::ConsentProvider>>,
    /// 同意ストアを読み書きしない (テスト用)
    pub no_consent_store: bool,
    /// bootstrap のキャッシュ (None なら `Cache::default_dir`)
    pub cache_dir: Option<PathBuf>,
    /// manifest 署名の検証に使う公開鍵 (同梱鍵 + 追加分)
    pub trusted_keys: ipemu_sign::TrustedKeys,
    /// 署名のない配信 manifest を許す (`--no-signature-check`)
    pub allow_unsigned: bool,
    /// サーバに問い合わせずキャッシュだけで起動する
    pub offline: bool,
    /// hello / telemetry を送る (D18)
    pub telemetry: bool,
}

impl std::fmt::Debug for ControllerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControllerConfig")
            .field("debug_friendly", &self.debug_friendly)
            .field("catalog_dirs", &self.catalog_dirs)
            .field("watch_interval", &self.watch_interval)
            .field("data_dir", &self.data_dir)
            .field("consent", &self.consent.as_ref().map(|c| c.name()))
            .field("no_consent_store", &self.no_consent_store)
            .field("cache_dir", &self.cache_dir)
            .field("allow_unsigned", &self.allow_unsigned)
            .field("offline", &self.offline)
            .field("telemetry", &self.telemetry)
            .finish()
    }
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            debug_friendly: false,
            catalog_dirs: Vec::new(),
            watch_interval: Duration::from_millis(300),
            data_dir: None,
            consent: None,
            no_consent_store: false,
            cache_dir: None,
            trusted_keys: ipemu_sign::TrustedKeys::default(),
            allow_unsigned: false,
            offline: false,
            telemetry: true,
        }
    }
}

/// I/O 側が持つハンドル。
#[derive(Clone)]
pub struct ControllerHandle {
    tx: Sender<Command>,
}

impl ControllerHandle {
    /// 要求を制御器に渡して結果を待つ。
    pub async fn call(&self, body: RequestBody) -> Result<serde_json::Value, RpcError> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(Command::Rpc { body, reply })
            .map_err(|_| RpcError::new(codes::INTERNAL_ERROR, "制御器が停止しています"))?;
        rx.await
            .map_err(|_| RpcError::new(codes::INTERNAL_ERROR, "制御器が応答しませんでした"))?
    }
    /// 同期版 (テスト・CLI 用)。
    pub fn call_blocking(&self, body: RequestBody) -> Result<serde_json::Value, RpcError> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(Command::Rpc { body, reply })
            .map_err(|_| RpcError::new(codes::INTERNAL_ERROR, "制御器が停止しています"))?;
        rx.blocking_recv()
            .map_err(|_| RpcError::new(codes::INTERNAL_ERROR, "制御器が応答しませんでした"))?
    }
    pub fn shutdown(&self) {
        let _ = self.tx.send(Command::Shutdown);
    }
}

/// `spawn` の戻り値。
pub struct Spawned {
    pub handle: ControllerHandle,
    pub state: Reader<StateFrame>,
    pub outbound: tokio::sync::mpsc::UnboundedReceiver<Outbound>,
    pub join: std::thread::JoinHandle<()>,
}

/// スケジューラスレッドを起動して、ハンドル・スナップショット読み手・送信キューを返す。
pub fn spawn(cfg: ControllerConfig) -> anyhow::Result<Spawned> {
    let (out_tx, out_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut s = spawn_with_outbound(cfg, out_tx)?;
    s.outbound = out_rx;
    Ok(s)
}

/// 送信キューの送り手を外から渡す形 (同意ブローカなど、制御器以外も通知を流したいとき)。
/// 戻り値の `outbound` はダミー (閉じた受け手) なので、呼び出し側が持つ受け手を使うこと。
pub fn spawn_with_outbound(
    cfg: ControllerConfig,
    out_tx: tokio::sync::mpsc::UnboundedSender<Outbound>,
) -> anyhow::Result<Spawned> {
    let (tx, rx) = mpsc::channel();
    let (state_w, state_r) = triple_buffer(StateFrame::default());
    let (_dummy_tx, out_rx) = tokio::sync::mpsc::unbounded_channel();
    let join = std::thread::Builder::new()
        .name("ipemu-scheduler".into())
        .spawn(move || Controller::new(cfg, rx, state_w, out_tx).run())
        .context("スケジューラスレッドを起動できません")?;
    Ok(Spawned {
        handle: ControllerHandle { tx },
        state: state_r,
        outbound: out_rx,
        join,
    })
}

struct ScopeSub {
    id: u32,
    refs: Vec<(usize, usize)>,
    divider: u64,
    flush_samples: usize,
    t0_ns: i64,
    channels: Vec<Vec<f64>>,
}

struct Watch {
    slot: String,
    source: FirmwareSource,
    path: PathBuf,
    /// 直近に装着したときのファイル (mtime, len)
    seen: Option<(SystemTime, u64)>,
}

struct Controller {
    cfg: ControllerConfig,
    rx: Receiver<Command>,
    state_w: Writer<StateFrame>,
    out: tokio::sync::mpsc::UnboundedSender<Outbound>,
    session: Option<Session>,
    launch: Option<LaunchParams>,
    run: RunState,
    fault: Option<String>,
    pacing: PacingConfig,
    pacer: Pacer,
    epoch: u32,
    pending_steps: u64,
    scopes: Vec<ScopeSub>,
    next_scope_id: u32,
    watches: Vec<Watch>,
    last_watch_check: Instant,
    last_state_publish: Instant,
    values_buf: Vec<f64>,
    /// サーバ配信のセッションでの実行要約の送信先 (ローカル manifest / オフラインなら None)
    telemetry: Option<Telemetry>,
    telemetry_started: bool,
    /// キャッシュ構成でのオフライン起動
    offline: bool,
    /// 組み立てに使った manifest の生 JSON (manifest.refresh の差分判定用。Manifest 構造体の再 serialize は既定値が埋まるので比較に使えない)
    loaded_manifest_json: Option<serde_json::Value>,
}

impl Controller {
    fn new(
        cfg: ControllerConfig,
        rx: Receiver<Command>,
        state_w: Writer<StateFrame>,
        out: tokio::sync::mpsc::UnboundedSender<Outbound>,
    ) -> Self {
        let debug_friendly = cfg.debug_friendly;
        Self {
            cfg,
            rx,
            state_w,
            out,
            session: None,
            launch: None,
            run: RunState::Idle,
            fault: None,
            pacing: PacingConfig::default(),
            pacer: Pacer::new(1_000_000, 1.0, 200, 50, debug_friendly),
            epoch: 0,
            pending_steps: 0,
            scopes: Vec::new(),
            next_scope_id: 1,
            watches: Vec::new(),
            last_watch_check: Instant::now(),
            last_state_publish: Instant::now(),
            values_buf: Vec::new(),
            telemetry: None,
            telemetry_started: false,
            offline: false,
            loaded_manifest_json: None,
        }
    }

    // ------------------------------------------------------------------ main loop

    fn run(mut self) {
        loop {
            // 1. 操作キュー (実行中は待たない。停止中は眠りながら待つ)
            let active = self.is_advancing();
            let cmd = if active {
                match self.rx.try_recv() {
                    Ok(c) => Some(c),
                    Err(mpsc::TryRecvError::Empty) => None,
                    Err(mpsc::TryRecvError::Disconnected) => return,
                }
            } else {
                match self.rx.recv_timeout(self.cfg.watch_interval) {
                    Ok(c) => Some(c),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            };
            if let Some(cmd) = cmd {
                match cmd {
                    Command::Shutdown => return,
                    Command::Rpc { body, reply } => {
                        let r = self.handle(body);
                        let _ = reply.send(r);
                    }
                }
                // 続けて溜まっている命令も処理する (ステップより操作を優先)
                while let Ok(cmd) = self.rx.try_recv() {
                    match cmd {
                        Command::Shutdown => return,
                        Command::Rpc { body, reply } => {
                            let r = self.handle(body);
                            let _ = reply.send(r);
                        }
                    }
                }
            }

            // 2. 監視ファイル
            if self.last_watch_check.elapsed() >= self.cfg.watch_interval {
                self.last_watch_check = Instant::now();
                self.check_watches();
            }

            // 3. ステップ
            if !self.is_advancing() {
                self.pacer.mark_idle();
                self.publish_state(true);
                continue;
            }
            let t_ns = self
                .session
                .as_ref()
                .map(|s| s.engine.time_ns())
                .unwrap_or(0);
            let steps = match self.pacing.mode {
                PacingMode::Realtime => match self.pacer.due(t_ns) {
                    Due::Steps(n) => n,
                    Due::Sleep(d) => {
                        self.publish_state(false);
                        std::thread::sleep(d.min(Duration::from_millis(2)));
                        continue;
                    }
                },
                // fast: 壁時計 2ms ぶんだけ回して操作キューに戻る
                PacingMode::Fast => MAX_BATCH,
                PacingMode::Step => {
                    let n = self.pending_steps.min(MAX_BATCH);
                    self.pending_steps -= n;
                    n
                }
            };
            self.advance(steps, self.pacing.mode == PacingMode::Fast);
            self.publish_state(false);
        }
    }

    fn is_advancing(&self) -> bool {
        self.session.is_some()
            && self.run == RunState::Running
            && (self.pacing.mode != PacingMode::Step || self.pending_steps > 0)
    }

    /// `steps` ステップ進める。`time_boxed` なら壁時計 2ms で打ち切る (fast 用)。
    fn advance(&mut self, steps: u64, time_boxed: bool) {
        let started = Instant::now();
        let mut done = 0u64;
        let div = self
            .session
            .as_ref()
            .map(|s| s.snapshot_divider())
            .unwrap_or(1);
        while done < steps {
            let Some(s) = self.session.as_mut() else {
                break;
            };
            if let Err(e) = s.step() {
                let msg = format!("{e:#}");
                self.enter_fault(msg);
                break;
            }
            done += 1;
            self.sample_scopes();
            if s_tick(self.session.as_ref()).is_multiple_of(div) {
                self.publish_state_if_due();
            }
            if time_boxed
                && done.is_multiple_of(64)
                && started.elapsed() >= Duration::from_millis(2)
            {
                break;
            }
        }
        if time_boxed && self.pacing.mode == PacingMode::Step {
            // (到達しない) step モードでは time_boxed=false
        }
        let t_ns = self
            .session
            .as_ref()
            .map(|s| s.engine.time_ns())
            .unwrap_or(0);
        self.pacer.record(done, started.elapsed(), t_ns);
        self.drain_logs();
    }

    fn enter_fault(&mut self, msg: String) {
        self.run = RunState::Fault;
        self.fault = Some(msg.clone());
        self.pending_steps = 0;
        self.flush_scopes();
        let t = self.t_ns();
        self.event(EventKind::Fault, Level::Error, None, msg, None);
        self.emit_run_event(t);
    }

    fn t_ns(&self) -> i64 {
        self.session
            .as_ref()
            .map(|s| s.engine.time_ns())
            .unwrap_or(0)
    }

    // ------------------------------------------------------------------ publish

    fn publish_state_if_due(&mut self) {
        // fast モードでは仮想 10ms ごとに来るので、壁時計 4ms 以上あいたときだけ書く
        if self.last_state_publish.elapsed() >= Duration::from_millis(4) {
            self.publish_state(true);
        }
    }

    fn publish_state(&mut self, force: bool) {
        if !force && self.last_state_publish.elapsed() < Duration::from_millis(4) {
            return;
        }
        self.last_state_publish = Instant::now();
        let Some(s) = self.session.as_ref() else {
            return;
        };
        s.snapshot_values(&mut self.values_buf);
        let f = self.state_w.back_mut();
        f.t_ns = s.engine.time_ns();
        f.tick = s.engine.tick();
        f.epoch = self.epoch;
        f.state = self.run;
        f.pacing = self.pacing.mode;
        f.speed = self.pacing.speed;
        f.values.clear();
        f.values.extend_from_slice(&self.values_buf);
        f.stats = self.pacer.stats();
        self.state_w.publish();
    }

    fn event(
        &self,
        kind: EventKind,
        level: Level,
        instance: Option<String>,
        message: String,
        data: Option<serde_json::Value>,
    ) {
        let n = Notification::new(NotificationBody::Event(EventNotice {
            t_ns: self.t_ns(),
            kind,
            level,
            instance,
            message,
            data,
        }));
        if let Ok(text) = serde_json::to_string(&n) {
            let _ = self.out.send(Outbound::Text(text));
        }
    }

    fn progress(&self, stage: BootstrapStage, message: impl Into<String>, fraction: Option<f64>) {
        let n = Notification::new(NotificationBody::BootstrapProgress(BootstrapProgress {
            stage,
            message: message.into(),
            fraction,
            offline: false,
        }));
        if let Ok(text) = serde_json::to_string(&n) {
            let _ = self.out.send(Outbound::Text(text));
        }
    }

    /// 実行要約の送信 (best-effort。失敗しても止めない)
    fn send_telemetry(&self, event: &str, detail: serde_json::Value) {
        if let Some(t) = &self.telemetry {
            if let Err(e) = t.send(event, detail) {
                eprintln!("telemetry: {e:#}");
            }
        }
    }

    fn open_cache(&self) -> Result<Cache, RpcError> {
        Cache::new(
            self.cfg
                .cache_dir
                .clone()
                .unwrap_or_else(Cache::default_dir),
        )
        .map_err(RpcError::from)
    }

    fn emit_run_event(&self, _t: i64) {
        let st = self.run_status();
        let msg = match self.run {
            RunState::Idle => "停止",
            RunState::Running => "実行中",
            RunState::Fault => "FAULT",
        };
        self.event(
            EventKind::Run,
            if self.run == RunState::Fault {
                Level::Error
            } else {
                Level::Info
            },
            None,
            msg.to_owned(),
            Some(serde_json::to_value(st).unwrap_or_default()),
        );
    }

    fn drain_logs(&mut self) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let logs = s.engine.drain_logs();
        for (inst, lvl, msg) in logs {
            let level = match lvl {
                LogLevel::Trace => Level::Trace,
                LogLevel::Debug => Level::Debug,
                LogLevel::Info => Level::Info,
                LogLevel::Warn => Level::Warn,
                LogLevel::Error => Level::Error,
            };
            self.event(EventKind::Log, level, Some(inst), msg, None);
        }
    }

    // ------------------------------------------------------------------ scope

    fn sample_scopes(&mut self) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let tick = s.engine.tick();
        let t_ns = s.engine.time_ns();
        let mut flush: Vec<usize> = Vec::new();
        for (k, sub) in self.scopes.iter_mut().enumerate() {
            if !tick.is_multiple_of(sub.divider) {
                continue;
            }
            if sub.channels[0].is_empty() {
                sub.t0_ns = t_ns;
            }
            for (ch, (i, p)) in sub.refs.iter().enumerate() {
                sub.channels[ch].push(s.engine.output(*i, *p));
            }
            if sub.channels[0].len() >= sub.flush_samples {
                flush.push(k);
            }
        }
        for k in flush {
            self.flush_scope(k);
        }
    }

    fn flush_scope(&mut self, k: usize) {
        let base = self
            .session
            .as_ref()
            .map(|s| s.engine.base_step_ns())
            .unwrap_or(1);
        let sub = &mut self.scopes[k];
        if sub.channels[0].is_empty() {
            return;
        }
        let frame = ScopeFrame {
            subscription_id: sub.id,
            epoch: self.epoch,
            t0_ns: sub.t0_ns,
            period_ns: base * sub.divider as i64,
            channels: sub.channels.iter_mut().map(std::mem::take).collect(),
        };
        if let Ok(bytes) = encode_frame(FrameKind::Scope, &frame) {
            let _ = self.out.send(Outbound::Binary(bytes));
        }
    }

    fn flush_scopes(&mut self) {
        for k in 0..self.scopes.len() {
            self.flush_scope(k);
        }
    }

    // ------------------------------------------------------------------ watch

    fn check_watches(&mut self) {
        if self.session.is_none() {
            return;
        }
        let mut due: Vec<(String, FirmwareSource)> = Vec::new();
        for w in &mut self.watches {
            let Ok(meta) = std::fs::metadata(&w.path) else {
                continue;
            };
            let Ok(mtime) = meta.modified() else { continue };
            let now = (mtime, meta.len());
            if w.seen == Some(now) {
                continue;
            }
            // 書き込み途中のファイルを読まないよう、更新から 200ms 置く
            if SystemTime::now()
                .duration_since(mtime)
                .map(|d| d < Duration::from_millis(200))
                .unwrap_or(false)
            {
                continue;
            }
            w.seen = Some(now);
            due.push((w.slot.clone(), w.source.clone()));
        }
        for (slot, source) in due {
            match self.load_firmware(&slot, source.clone()) {
                Ok(r) => {
                    self.event(
                        EventKind::Firmware,
                        Level::Info,
                        None,
                        format!("watch: slot `{slot}` を更新 ({})", r.message),
                        Some(serde_json::to_value(&r).unwrap_or_default()),
                    );
                }
                Err(e) => self.event(
                    EventKind::Firmware,
                    Level::Warn,
                    None,
                    format!("watch: slot `{slot}` の差し替えに失敗: {}", e.message),
                    None,
                ),
            }
        }
    }

    // ------------------------------------------------------------------ RPC

    fn handle(&mut self, body: RequestBody) -> Result<serde_json::Value, RpcError> {
        fn j<T: serde::Serialize>(v: T) -> Result<serde_json::Value, RpcError> {
            serde_json::to_value(v).map_err(|e| RpcError::new(codes::INTERNAL_ERROR, e.to_string()))
        }
        match body {
            RequestBody::SessionLaunch(p) => j(self.launch(p)?),
            RequestBody::SessionDescribe(_) => j(DescribeResult {
                session: self.describe().ok(),
                engine: self.engine_info(),
            }),
            RequestBody::CatalogLocal(_) => j(self.catalog_local()),
            RequestBody::RunStart(_) => {
                self.require_session()?;
                if self.run == RunState::Fault {
                    return Err(RpcError::invalid_state(
                        "FAULT 中です。run.reset で復帰してください",
                    ));
                }
                if self.run != RunState::Running {
                    self.run = RunState::Running;
                    self.pacer.resync(self.t_ns());
                    self.emit_run_event(self.t_ns());
                    if !self.telemetry_started {
                        self.telemetry_started = true;
                        self.send_telemetry(
                            "started",
                            serde_json::json!({"pacing": self.pacing.mode}),
                        );
                    }
                }
                j(self.run_status())
            }
            RequestBody::RunStop(_) => {
                self.require_session()?;
                if self.run == RunState::Running {
                    self.run = RunState::Idle;
                    self.pending_steps = 0;
                    self.flush_scopes();
                    self.emit_run_event(self.t_ns());
                }
                self.publish_state(true);
                j(self.run_status())
            }
            RequestBody::RunReset(_) => {
                let s = self.session.as_mut().ok_or_else(RpcError::no_session)?;
                s.reset().map_err(RpcError::from)?;
                self.epoch = self.epoch.wrapping_add(1);
                self.fault = None;
                self.pending_steps = 0;
                for sub in &mut self.scopes {
                    sub.channels.iter_mut().for_each(Vec::clear);
                }
                if self.run == RunState::Fault {
                    self.run = RunState::Idle;
                }
                self.pacer.resync(0);
                self.drain_logs();
                self.emit_run_event(0);
                self.publish_state(true);
                j(self.run_status())
            }
            RequestBody::RunStep(p) => {
                self.require_session()?;
                if p.n == 0 {
                    return Err(RpcError::invalid_params("n は 1 以上"));
                }
                if self.run == RunState::Fault {
                    return Err(RpcError::invalid_state(
                        "FAULT 中です。run.reset で復帰してください",
                    ));
                }
                if self.run == RunState::Running && self.pacing.mode != PacingMode::Step {
                    return Err(RpcError::invalid_state(
                        "実行中です。run.stop するか pacing を step にしてください",
                    ));
                }
                if self.pacing.mode == PacingMode::Step && self.run == RunState::Running {
                    // 実行中 (step モード) ならループ側で消化する
                    self.pending_steps = self.pending_steps.saturating_add(p.n);
                } else {
                    // 停止中は同期的に進める (上限あり。大きい n は複数回に分けて呼ぶ)
                    let n = p.n.min(1_000_000);
                    let mut left = n;
                    while left > 0 && self.run != RunState::Fault {
                        let b = left.min(MAX_BATCH);
                        self.advance(b, false);
                        left -= b;
                    }
                    self.flush_scopes();
                    self.publish_state(true);
                }
                j(self.run_status())
            }
            RequestBody::RunSetPacing(p) => {
                self.require_session()?;
                if !(p.speed.is_finite() && p.speed > 0.0) {
                    return Err(RpcError::invalid_params("speed は正の有限値"));
                }
                self.pacing = p;
                self.pacer.set_speed(p.speed, self.t_ns());
                self.publish_state(true);
                j(self.run_status())
            }
            RequestBody::ParamSet(p) => {
                let s = self.session.as_mut().ok_or_else(RpcError::no_session)?;
                let (change, params) = s
                    .set_param(&p.instance, &p.key, p.value.clone())
                    .map_err(RpcError::from)?;
                if let Some(l) = self.launch.as_mut() {
                    l.params
                        .retain(|o| !(o.instance == p.instance && o.key == p.key));
                    l.params.push(p.clone());
                }
                let r = ParamSetResult {
                    reinitialized: change == ParamChange::Reinitialized,
                    restart_required: change == ParamChange::RestartRequired,
                    params,
                };
                self.event(
                    EventKind::Param,
                    Level::Info,
                    Some(p.instance.clone()),
                    if r.restart_required {
                        format!("{} は再起動で反映されます", p.key)
                    } else {
                        format!("{} を変更 (部品を init し直しました)", p.key)
                    },
                    Some(serde_json::to_value(&r).unwrap_or_default()),
                );
                j(r)
            }
            RequestBody::FirmwareLoad(p) => {
                let source = FirmwareSource::parse(&p.source).map_err(RpcError::invalid_params)?;
                let r = self.load_firmware(&p.slot, source)?;
                self.event(
                    EventKind::Firmware,
                    Level::Info,
                    None,
                    r.message.clone(),
                    Some(serde_json::to_value(&r).unwrap_or_default()),
                );
                j(r)
            }
            RequestBody::FirmwareWatch(p) => {
                self.require_session()?;
                self.watches.retain(|w| w.slot != p.slot);
                if let Some(src) = p.source {
                    let source = FirmwareSource::parse(&src).map_err(RpcError::invalid_params)?;
                    let path =
                        match &source {
                            FirmwareSource::Wasm(p) | FirmwareSource::Native(p) => p.clone(),
                            FirmwareSource::Default => return Err(RpcError::invalid_params(
                                "firmware.watch は wasm:<path> / native:<path> だけ監視できます",
                            )),
                        };
                    let seen = std::fs::metadata(&path)
                        .ok()
                        .and_then(|m| m.modified().ok().map(|t| (t, m.len())));
                    self.watches.push(Watch {
                        slot: p.slot,
                        source,
                        path,
                        seen,
                    });
                }
                j(Empty {})
            }
            RequestBody::ScopeSubscribe(p) => j(self.scope_subscribe(p)?),
            // consent.answer は IPC サーバが ConsentBroker に直接渡す (制御器は launch 中ブロックしているため)。
            // ここに来るのは broker を使わない構成 (テスト等) だけ
            RequestBody::ConsentAnswer(p) => Err(RpcError::not_found(format!(
                "同意要求 `{}` は問い合わせ中ではありません",
                p.request_id
            ))),
            RequestBody::ManifestRefresh(_) => j(self.manifest_refresh()?),
        }
    }

    fn require_session(&self) -> Result<(), RpcError> {
        if self.session.is_none() {
            Err(RpcError::no_session())
        } else {
            Ok(())
        }
    }

    fn run_status(&self) -> RunStatus {
        let (t_ns, tick) = self
            .session
            .as_ref()
            .map(|s| (s.engine.time_ns(), s.engine.tick()))
            .unwrap_or((0, 0));
        RunStatus {
            state: self.run,
            pacing: self.pacing,
            t_ns,
            tick,
            epoch: self.epoch,
            fault: self.fault.clone(),
        }
    }

    fn engine_info(&self) -> EngineInfo {
        EngineInfo {
            version: env!("CARGO_PKG_VERSION").into(),
            ipc_version: IPC_VERSION,
            pid: std::process::id(),
            debug_friendly: self.pacer.debug_friendly(),
        }
    }

    // ------------------------------------------------------------------ launch / describe

    fn launch(&mut self, p: LaunchParams) -> Result<SessionDescription, RpcError> {
        // 取得元: ローカル manifest か、サーバ (M3 の bootstrap。キャッシュにそろえて同じ Session::load に渡す)
        let mut bootstrapped = None;
        let (manifest_path, allow_unhashed) = match &p.source {
            LaunchSource::Local {
                manifest_path,
                allow_unhashed,
            } => (PathBuf::from(manifest_path), *allow_unhashed),
            LaunchSource::Server {
                server,
                manifest_id,
                revision,
                credential,
                offline,
            } => {
                let cache = self.open_cache()?;
                // manifest id はパスに使う前に検査する (監査 A4 / D55)。不正なら呼び出し側のパラメータ誤り
                let mut bo = BootstrapOptions::new(server, manifest_id, cache)
                    .map_err(|e| RpcError::invalid_params(format!("{e:#}")))?;
                bo.revision = *revision;
                bo.credential = credential.as_ref().map(to_bootstrap_credential);
                bo.trusted_keys = self.cfg.trusted_keys.clone();
                bo.allow_unsigned = self.cfg.allow_unsigned;
                bo.offline = self.cfg.offline || *offline;
                bo.telemetry = self.cfg.telemetry;
                if bo.credential.is_none() && !bo.offline {
                    return Err(RpcError::invalid_params(
                        "source.credential がありません (オフライン起動なら offline: true)",
                    ));
                }
                self.progress(
                    BootstrapStage::Manifest,
                    format!("{server} から `{manifest_id}` を取得"),
                    Some(0.05),
                );
                let b = match bootstrap(&bo) {
                    Ok(b) => b,
                    Err(e) => {
                        self.progress(BootstrapStage::Failed, format!("{e:#}"), None);
                        return Err(RpcError::engine(format!("{e:#}")));
                    }
                };
                self.progress(
                    BootstrapStage::Artifacts,
                    format!(
                        "rev {} {} (取得 {} / キャッシュ {}, 署名 {})",
                        b.revision,
                        if b.from_cache {
                            "[オフライン / キャッシュ構成]"
                        } else {
                            "from server"
                        },
                        b.downloaded,
                        b.cache_hits,
                        b.verified_key_id.as_deref().unwrap_or("なし")
                    ),
                    Some(0.25),
                );
                let path = b.manifest_path.clone();
                bootstrapped = Some((b, bo.cache));
                // 配信 manifest は sha256 必須 (allow_unhashed は署名の有無には効かない)
                (path, false)
            }
        };
        let mut opts = SessionOptions {
            allow_unhashed,
            data_dir: self.cfg.data_dir.clone(),
            consent: self.cfg.consent.clone(),
            no_consent_store: self.cfg.no_consent_store,
            ..Default::default()
        };
        for (slot, src) in &p.firmware {
            opts.firmware_overrides.insert(
                slot.clone(),
                FirmwareSource::parse(src).map_err(RpcError::invalid_params)?,
            );
        }
        for o in &p.params {
            opts.param_overrides
                .push((o.instance.clone(), o.key.clone(), o.value.clone()));
        }
        if self.pacer.debug_friendly() {
            opts.runtime = Some(ipemu_wasm::runtime::RuntimeOptions {
                call_budget: None,
                ..Default::default()
            });
        }
        if let Some((b, cache)) = &bootstrapped {
            opts = b.session_options(cache, opts);
        }

        // 実行中のセッションがあれば止めて置き換える (v1 は同時 1 セッション: D16)
        if self.session.is_some() {
            self.run = RunState::Idle;
            self.flush_scopes();
            self.event(
                EventKind::Session,
                Level::Info,
                None,
                "セッションを置き換えます".into(),
                None,
            );
        }
        self.send_telemetry("stopped", serde_json::json!({"reason": "replaced"}));
        self.session = None;
        self.scopes.clear();
        self.watches.clear();
        self.fault = None;
        self.pending_steps = 0;
        self.telemetry = None;
        self.telemetry_started = false;
        self.offline = false;

        self.progress(
            BootstrapStage::Manifest,
            format!("{} を読み込み", manifest_path.display()),
            Some(0.1),
        );
        self.progress(
            BootstrapStage::Consent,
            "capability の確認 (未同意があればシェルに問い合わせ)",
            Some(0.3),
        );
        self.progress(
            BootstrapStage::Compile,
            "部品とファームウェアをコンパイル",
            Some(0.4),
        );
        let session = match Session::load(&manifest_path, &opts) {
            Ok(s) => s,
            Err(e) => {
                if let Some((b, _)) = &bootstrapped {
                    if let Some(t) = b.telemetry() {
                        let _ = t.send(
                            "error",
                            serde_json::json!({"phase": "load", "error": format!("{e:#}")}),
                        );
                    }
                }
                self.progress(BootstrapStage::Failed, format!("{e:#}"), None);
                return Err(RpcError::engine(format!("{e:#}")));
            }
        };
        self.loaded_manifest_json = match &bootstrapped {
            Some((b, _)) => Some(b.manifest.clone()),
            None => std::fs::read_to_string(&manifest_path)
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok()),
        };
        if let Some((b, _)) = &bootstrapped {
            self.telemetry = b.telemetry();
            self.offline = b.from_cache;
            if b.from_cache {
                self.event(
                    EventKind::Engine,
                    Level::Warn,
                    None,
                    "サーバに届かないため、キャッシュ構成で起動しました (オフライン)".into(),
                    None,
                );
            }
        }
        self.progress(BootstrapStage::Instantiate, "組み立て完了", Some(0.9));

        let sched = &session.manifest.scheduler;
        self.pacing = p.pacing.unwrap_or(PacingConfig {
            mode: match sched.pacing {
                ipemu_core::manifest::Pacing::Realtime => PacingMode::Realtime,
                ipemu_core::manifest::Pacing::Fast => PacingMode::Fast,
                ipemu_core::manifest::Pacing::Step => PacingMode::Step,
            },
            speed: sched.speed,
        });
        self.pacer = Pacer::new(
            sched.base_step_ns,
            self.pacing.speed,
            sched.max_slip_ms,
            sched.step_budget_ms,
            self.pacer.debug_friendly(),
        );
        self.epoch = self.epoch.wrapping_add(1);
        self.run = RunState::Idle;
        self.session = Some(session);
        self.launch = Some(p.clone());
        self.drain_logs();
        if p.auto_start {
            self.run = RunState::Running;
            self.pacer.resync(0);
        }
        let desc = self.describe()?;
        self.progress(BootstrapStage::Ready, "ready", Some(1.0));
        self.event(
            EventKind::Session,
            Level::Info,
            None,
            format!(
                "`{}` rev {} を組み立てました",
                desc.manifest.id, desc.manifest.revision
            ),
            Some(serde_json::to_value(&desc).unwrap_or_default()),
        );
        if p.auto_start {
            self.emit_run_event(0);
        }
        self.publish_state(true);
        Ok(desc)
    }

    fn describe(&mut self) -> Result<SessionDescription, RpcError> {
        let run = self.run_status();
        let engine = self.engine_info();
        let launch = self.launch.clone().ok_or_else(RpcError::no_session)?;
        let watches: BTreeMap<String, String> = self
            .watches
            .iter()
            .map(|w| (w.slot.clone(), w.path.display().to_string()))
            .collect();
        let s = self.session.as_mut().ok_or_else(RpcError::no_session)?;
        // 装着中の形は可変借用が要るので先に集める
        let mut cpu_flavors: Vec<(String, Option<&'static str>)> = Vec::new();
        for (slot, _) in s
            .slots()
            .map(|(a, b)| (a.to_owned(), b))
            .collect::<Vec<_>>()
        {
            let f = s.firmware_flavor(&slot);
            cpu_flavors.push((slot, f));
        }
        let s = &*s;
        let m = &s.manifest;
        let graph = s.engine.graph();
        let mut parts = Vec::with_capacity(graph.instances.len());
        for (i, inst) in graph.instances.iter().enumerate() {
            let spec = &m.components[i];
            let d = &graph.descs[i];
            let firmware = spec.firmware.as_ref().map(|f| FirmwareInfo {
                slot: f.slot.clone(),
                world: f.world.clone(),
                flavor: cpu_flavors
                    .iter()
                    .find(|(sl, _)| sl == &f.slot)
                    .and_then(|(_, fl)| fl.map(str::to_owned)),
                source: match s.options.firmware_overrides.get(&f.slot) {
                    Some(FirmwareSource::Wasm(p)) => format!("wasm:{}", p.display()),
                    Some(FirmwareSource::Native(p)) => format!("native:{}", p.display()),
                    _ => "default".into(),
                },
                default: f.default.clone(),
                allow_override: f.allow_override.clone(),
                watching: watches.get(&f.slot).cloned(),
            });
            parts.push(PartInfo {
                instance: inst.clone(),
                kind: spec.kind.clone().unwrap_or_else(|| "part".into()),
                artifact: spec.artifact.clone(),
                name: d.name.clone(),
                version: d.version.clone(),
                inputs: d.inputs.iter().map(port_info).collect(),
                outputs: d.outputs.iter().map(port_info).collect(),
                params: spec.params.clone(),
                rate_divider: spec.rate_divider,
                deterministic: d.deterministic,
                doc: d.doc.clone(),
                firmware,
            });
        }
        let wires = m
            .wires
            .iter()
            .map(|w| WireInfo {
                from: w.from.clone(),
                to: w.to.clone(),
                latency: w.latency,
                gain: w.gain,
                offset: w.offset,
            })
            .collect();
        // capability: 組み立てに成功した = 計画の全件に同意済み (未同意・拒否なら Session::load が失敗している)
        let mut capabilities = Vec::new();
        for e in &s.capability_plan.entries {
            for c in e.capabilities.items() {
                capabilities.push(CapabilityState {
                    instance: e.instance.clone(),
                    capability: c.to_string(),
                    consent: ConsentState::Granted,
                    artifact_sha256: Some(e.artifact_sha256.clone()),
                });
            }
        }
        let ui = ui_bundle_info(&s.manifest_dir, m, &s.options);
        let overrides = Overrides {
            firmware: s
                .options
                .firmware_overrides
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        match v {
                            FirmwareSource::Default => "default".to_owned(),
                            FirmwareSource::Wasm(p) => format!("wasm:{}", p.display()),
                            FirmwareSource::Native(p) => format!("native:{}", p.display()),
                        },
                    )
                })
                .collect(),
            params: s
                .options
                .param_overrides
                .iter()
                .map(|(i, k, v)| ParamOverride {
                    instance: i.clone(),
                    key: k.clone(),
                    value: v.clone(),
                })
                .collect(),
        };
        let warnings = s
            .diagnostics
            .warnings()
            .map(|d| DiagnosticInfo {
                severity: "warning".into(),
                rule: d.rule.to_string(),
                message: d.message.clone(),
            })
            .collect();
        Ok(SessionDescription {
            manifest: ManifestSummary {
                id: m.id.clone(),
                revision: m.revision,
                name: m.name.clone(),
                description: m.description.clone(),
                source: launch.source.clone(),
                base_step_ns: m.scheduler.base_step_ns,
                snapshot_divider: m.scheduler.snapshot_divider,
            },
            parts,
            wires,
            snapshot_ports: s.snapshot_ports.clone(),
            run,
            capabilities,
            overrides,
            credential: match &launch.source {
                LaunchSource::Local { .. } => CredentialKind::None,
                LaunchSource::Server { credential, .. } => match credential {
                    Some(Credential::ApiKey { .. }) => CredentialKind::ApiKey,
                    Some(_) => CredentialKind::User,
                    None => CredentialKind::None,
                },
            },
            ui,
            offline: self.offline,
            warnings,
            engine,
        })
    }

    fn load_firmware(
        &mut self,
        slot: &str,
        source: FirmwareSource,
    ) -> Result<FirmwareLoadResult, RpcError> {
        let s = self.session.as_mut().ok_or_else(RpcError::no_session)?;
        let outcome = s
            .swap_firmware(slot, source.clone())
            .map_err(RpcError::from)?;
        // 再起動後に同じ構成を再現できるよう launch にも記録する
        let src_text = match &source {
            FirmwareSource::Default => "default".to_owned(),
            FirmwareSource::Wasm(p) => format!("wasm:{}", p.display()),
            FirmwareSource::Native(p) => format!("native:{}", p.display()),
        };
        if let Some(l) = self.launch.as_mut() {
            l.firmware.insert(slot.to_owned(), src_text.clone());
        }
        let desc_overrides = self.describe()?.overrides;
        Ok(match outcome {
            FirmwareSwap::Hot { flavor } => FirmwareLoadResult {
                restart_required: false,
                hot: true,
                flavor: Some(flavor.to_owned()),
                overrides: desc_overrides,
                message: format!("slot `{slot}` を {src_text} にホット差替しました (状態は保持)"),
            },
            FirmwareSwap::RestartRequired => FirmwareLoadResult {
                restart_required: true,
                hot: false,
                flavor: Some("native".into()),
                overrides: desc_overrides,
                message: format!(
                    "slot `{slot}` を {src_text} にするにはエンジンの再起動が必要です (native 形はプロセス内で入れ替えません)"
                ),
            },
        })
    }

    fn scope_subscribe(
        &mut self,
        p: ScopeSubscribeParams,
    ) -> Result<ScopeSubscribeResult, RpcError> {
        let s = self.session.as_ref().ok_or_else(RpcError::no_session)?;
        if let Some(id) = p.subscription_id {
            self.scopes.retain(|x| x.id != id);
        }
        if p.ports.is_empty() {
            return Ok(ScopeSubscribeResult {
                subscription_id: p.subscription_id.unwrap_or(0),
                ports: Vec::new(),
                period_ns: 0,
            });
        }
        let refs =
            p.ports
                .iter()
                .map(|port| {
                    s.engine.graph().resolve_output(port).ok_or_else(|| {
                        RpcError::not_found(format!("出力ポート `{port}` がありません"))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
        let base = s.engine.base_step_ns();
        if !(p.rate_hz.is_finite() && p.rate_hz > 0.0) {
            return Err(RpcError::invalid_params("rateHz は正の有限値"));
        }
        let divider = ((1e9 / p.rate_hz) / base as f64).round().max(1.0) as u64;
        let period_ns = base * divider as i64;
        let window_s = if p.window_s.is_finite() && p.window_s > 0.0 {
            p.window_s
        } else {
            0.05
        };
        let flush_samples = ((window_s * 1e9) / period_ns as f64)
            .round()
            .clamp(1.0, 100_000.0) as usize;
        let id = p.subscription_id.unwrap_or_else(|| {
            let id = self.next_scope_id;
            self.next_scope_id += 1;
            id
        });
        self.next_scope_id = self.next_scope_id.max(id + 1);
        self.scopes.push(ScopeSub {
            id,
            refs: refs.clone(),
            divider,
            flush_samples,
            t0_ns: 0,
            channels: vec![Vec::with_capacity(flush_samples); refs.len()],
        });
        Ok(ScopeSubscribeResult {
            subscription_id: id,
            ports: p.ports,
            period_ns,
        })
    }

    fn manifest_refresh(&mut self) -> Result<ManifestRefreshResult, RpcError> {
        let launch = self.launch.clone().ok_or_else(RpcError::no_session)?;
        let s = self.session.as_ref().ok_or_else(RpcError::no_session)?;
        match &launch.source {
            LaunchSource::Local { manifest_path, .. } => {
                // ローカル: ファイルを読み直して検証だけ行い、差分を報告する (置き換えは session.launch で)
                let text = std::fs::read_to_string(manifest_path)
                    .map_err(|e| RpcError::engine(format!("{manifest_path}: {e}")))?;
                let json: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| RpcError::engine(e.to_string()))?;
                let mut diags = ipemu_core::validate::Diagnostics::default();
                ipemu_core::validate::validate_schema(&json, &mut diags);
                let revision = json.get("revision").and_then(|r| r.as_u64()).unwrap_or(0) as u32;
                Ok(ManifestRefreshResult {
                    changed: revision != s.manifest.revision
                        || self.loaded_manifest_json.as_ref() != Some(&json),
                    revision,
                    diagnostics: diags
                        .items
                        .iter()
                        .map(|d| DiagnosticInfo {
                            severity: format!("{:?}", d.severity).to_lowercase(),
                            rule: d.rule.to_string(),
                            message: d.message.clone(),
                        })
                        .collect(),
                })
            }
            LaunchSource::Server {
                server,
                manifest_id,
                revision,
                credential,
                ..
            } => {
                // サーバへ問い合わせ直し、最新 (または指定 revision) が今のものと違うかを報告する (置き換えは session.launch で)
                let mut bo = BootstrapOptions::new(server, manifest_id, self.open_cache()?)
                    .map_err(|e| RpcError::invalid_params(format!("{e:#}")))?;
                bo.revision = *revision;
                bo.credential = credential.as_ref().map(to_bootstrap_credential);
                bo.trusted_keys = self.cfg.trusted_keys.clone();
                bo.allow_unsigned = self.cfg.allow_unsigned;
                bo.telemetry = false;
                let b = bootstrap(&bo).map_err(RpcError::from)?;
                Ok(ManifestRefreshResult {
                    changed: b.revision != s.manifest.revision
                        || self.loaded_manifest_json.as_ref() != Some(&b.manifest),
                    revision: b.revision,
                    diagnostics: Vec::new(),
                })
            }
        }
    }

    fn catalog_local(&self) -> CatalogLocalResult {
        let mut entries = Vec::new();
        // bootstrap のキャッシュにある構成 (サーバに届かなくても起動できる)
        if let Ok(cache) = self.open_cache() {
            for e in cache.load_catalog().entries {
                entries.push(CatalogEntry {
                    id: e.manifest_id.clone(),
                    revision: e.revision,
                    name: e.name.clone(),
                    description: None,
                    server: Some(e.server.clone()),
                    source: LaunchSource::Server {
                        server: e.server.clone(),
                        manifest_id: e.manifest_id.clone(),
                        revision: Some(e.revision),
                        credential: None,
                        offline: true,
                    },
                    last_launched: Some(e.last_launched_at.clone()),
                });
            }
        }
        for dir in &self.cfg.catalog_dirs {
            let Ok(rd) = std::fs::read_dir(dir) else {
                continue;
            };
            let mut paths: Vec<PathBuf> = rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.ends_with(".manifest.json"))
                })
                .collect();
            paths.sort();
            for p in paths {
                let Ok(text) = std::fs::read_to_string(&p) else {
                    continue;
                };
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
                    continue;
                };
                let abs = p.canonicalize().unwrap_or(p.clone());
                entries.push(CatalogEntry {
                    id: v["id"].as_str().unwrap_or("?").to_owned(),
                    revision: v["revision"].as_u64().unwrap_or(0) as u32,
                    name: v["name"].as_str().unwrap_or("").to_owned(),
                    description: v["description"].as_str().map(str::to_owned),
                    server: None,
                    source: LaunchSource::Local {
                        manifest_path: abs.display().to_string(),
                        allow_unhashed: true,
                    },
                    last_launched: None,
                });
            }
        }
        CatalogLocalResult { entries }
    }
}

fn s_tick(s: Option<&Session>) -> u64 {
    s.map(|s| s.engine.tick()).unwrap_or(0)
}

fn port_info(p: &ipemu_core::PortDesc) -> PortInfo {
    PortInfo {
        name: p.name.clone(),
        kind: match p.kind {
            ipemu_core::PortKind::Scalar => "scalar".into(),
            ipemu_core::PortKind::Bytes => "bytes".into(),
        },
        unit: p.unit.clone(),
        default: p.default,
        doc: p.doc.clone(),
    }
}

fn to_bootstrap_credential(c: &Credential) -> crate::bootstrap::Credential {
    match c {
        Credential::SessionToken { token } => crate::bootstrap::Credential::Token(token.clone()),
        Credential::Ticket { ticket } => crate::bootstrap::Credential::Ticket(ticket.clone()),
        Credential::ApiKey { key } => crate::bootstrap::Credential::ApiKey(key.clone()),
    }
}

/// manifest の `ui.bundle` を UiBundleInfo に写す。アーティファクトは単一ファイル (tar.gz)。
/// 配信 manifest では bootstrap の resolver がキャッシュ (`sha256/..`) のパスを返す。
fn ui_bundle_info(
    manifest_dir: &Path,
    m: &ipemu_core::Manifest,
    opts: &SessionOptions,
) -> Option<UiBundleInfo> {
    let ui = m.ui.as_ref()?;
    let bundle = ui.get("bundle")?.as_str()?;
    let art = m.artifacts.get(bundle)?;
    let path = match opts
        .artifact_resolver
        .as_ref()
        .and_then(|r| (r.0)(bundle, art).ok().flatten())
    {
        Some(p) => p,
        None => {
            let rel = art
                .path
                .clone()
                .or_else(|| art.url.as_ref()?.strip_prefix("file:").map(str::to_owned))?;
            manifest_dir.join(rel)
        }
    };
    let path = path.canonicalize().unwrap_or(path);
    Some(UiBundleInfo {
        artifact: bundle.to_owned(),
        path: path.display().to_string(),
        sha256: art.sha256.clone(),
        views: ui
            .get("views")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
    })
}
