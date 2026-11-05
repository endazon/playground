//! ipemu-server — 配信 API + 管理 API + SPA 配信 (06-server-app.md)。
//!
//! テストから in-process で立ち上げられるよう、`App::start` が任意ポートで listen して URL を返す。

pub mod api;
pub mod audit;
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod inspect;
pub mod manifests;
pub mod state;
pub mod store;
pub mod util;
pub mod web;

use anyhow::Context;
use axum::http::header;
use axum::routing::get;
use axum::Router;
use config::Config;
use state::{AppState, Inner};
use std::sync::Arc;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

/// 署名鍵を読む。設定がなければ起動ごとに生成する (開発用。公開した manifest は再起動後に検証できなくなる)。
fn load_signing_key(cfg: &Config) -> anyhow::Result<ipemu_sign::SigningKeyFile> {
    match &cfg.signing_key_path {
        Some(p) => {
            let text = std::fs::read_to_string(p)
                .with_context(|| format!("IPEMU_SIGNING_KEY {} を読めません", p.display()))?;
            Ok(ipemu_sign::SigningKeyFile::parse(&text)?)
        }
        None => {
            let kf = ipemu_sign::SigningKeyFile::generate("ephemeral");
            tracing::warn!(
                public_key = %kf.public_entry()?.public_key,
                "IPEMU_SIGNING_KEY が未設定のため一時鍵で署名します (再起動で変わります。`ipemu-server keygen` で鍵を作ってください)"
            );
            Ok(kf)
        }
    }
}

pub async fn build_state(cfg: Config) -> anyhow::Result<AppState> {
    std::fs::create_dir_all(&cfg.store_dir)
        .with_context(|| format!("IPEMU_STORE {} を作れません", cfg.store_dir.display()))?;
    let pool = db::open(&cfg.db_path).await?;
    if let Some(pw) = db::bootstrap_admin(&pool, cfg.admin_bootstrap_password.as_deref()).await? {
        tracing::warn!(password = %pw, "初回起動: admin ユーザを作成しました (IPEMU_ADMIN_BOOTSTRAP_PASSWORD 未設定のため生成)");
    }
    let store = store::ArtifactStore::new(&cfg.store_dir)?;
    let kf = load_signing_key(&cfg)?;
    Ok(AppState(Arc::new(Inner {
        signing_key: kf.signing_key()?,
        key_id: kf.key_id.clone(),
        pool,
        store,
        config: cfg,
        throttle: Default::default(),
    })))
}

pub fn build_router(state: AppState) -> Router {
    let max_upload = state.config.max_artifact_bytes;
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .nest("/api/auth", api::auth::router())
        .nest("/api/v1", api::v1::router())
        .nest("/api/admin", api::admin::router(max_upload))
        .fallback(web::serve)
        // Bearer を SPA に持たせる構成の緩和策 (D36): inline script を禁止し、接続先と埋め込み元を自分自身に限る。
        // style の 'unsafe-inline' は CodeMirror が動的に <style> を注入するために必要 (スクリプトではない)
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            axum::http::HeaderValue::from_static(
                "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'self'; frame-ancestors 'none'; form-action 'self'",
            ),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            axum::http::HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            axum::http::HeaderValue::from_static("same-origin"),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// 起動済みサーバ (テスト用のハンドル)。
pub struct App {
    pub state: AppState,
    pub base_url: String,
    handle: tokio::task::JoinHandle<()>,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

impl App {
    /// `cfg.listen` で listen を始める。ポート 0 なら OS が割り当てる。
    pub async fn start(cfg: Config) -> anyhow::Result<App> {
        let listener = tokio::net::TcpListener::bind(&cfg.listen)
            .await
            .with_context(|| format!("{} で listen できません", cfg.listen))?;
        let addr = listener.local_addr()?;
        let state = build_state(cfg).await?;
        let router = build_router(state.clone());
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let handle = tokio::spawn(async move {
            // 送信元 IP をログイン抑止の鍵に使うため ConnectInfo を付ける (D44)
            let _ = axum::serve(
                listener,
                router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .with_graceful_shutdown(async {
                let _ = rx.await;
            })
            .await;
        });
        Ok(App {
            state,
            base_url: format!("http://{addr}"),
            handle,
            shutdown: Some(tx),
        })
    }

    pub async fn stop(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = self.handle.await;
    }
}
