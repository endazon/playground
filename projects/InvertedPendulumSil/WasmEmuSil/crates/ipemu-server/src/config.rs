//! 設定 (環境変数)。06-server-app.md の「運用」に対応する。

use anyhow::Context;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    /// 待ち受けアドレス。既定 127.0.0.1:8080 (TLS 終端はリバースプロキシ)
    pub listen: String,
    /// アーティファクトの置き場 (`<store>/artifacts/sha256/ab/cd/<hex>`)
    pub store_dir: PathBuf,
    /// SQLite ファイル。既定 `<store>/db.sqlite`
    pub db_path: PathBuf,
    /// 署名鍵ファイル (ipemu_sign::SigningKeyFile)。無ければ起動ごとに生成して警告する (開発用)
    pub signing_key_path: Option<PathBuf>,
    /// 初回起動時に作る admin のパスワード。無ければ生成してログに出す
    pub admin_bootstrap_password: Option<String>,
    /// 外部から見た URL (ipemu:// リンクに載せる)。既定は http://<listen>
    pub public_url: String,
    /// SPA のビルド成果物 (apps/web/dist)。無ければ API だけ動く
    pub web_dir: Option<PathBuf>,
    /// 部品 1 個あたりの上限バイト数 (既定 32MB)
    pub max_artifact_bytes: usize,
    /// ログインセッションの寿命 (秒)
    pub session_ttl_secs: i64,
    /// チケット交換で得る短命トークンの寿命 (秒)
    pub launch_token_ttl_secs: i64,
    /// 単回チケットの寿命 (秒)。URL に載るので短くする
    pub ticket_ttl_secs: i64,
    /// `X-Forwarded-For` の先頭を送信元 IP として信用するか (`IPEMU_TRUST_PROXY=1`)。
    /// リバースプロキシの背後に置くときだけ有効にする (直接公開で有効にすると送信元を偽れる)
    pub trust_proxy: bool,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let store_dir: PathBuf = std::env::var("IPEMU_STORE")
            .unwrap_or_else(|_| "store".into())
            .into();
        let db_path = std::env::var("IPEMU_DB")
            .map(PathBuf::from)
            .unwrap_or_else(|_| store_dir.join("db.sqlite"));
        let listen = std::env::var("IPEMU_LISTEN").unwrap_or_else(|_| "127.0.0.1:8080".into());
        let public_url = std::env::var("IPEMU_PUBLIC_URL")
            .unwrap_or_else(|_| format!("http://{listen}"))
            .trim_end_matches('/')
            .to_owned();
        let max_artifact_bytes = match std::env::var("IPEMU_MAX_ARTIFACT_BYTES") {
            Ok(v) => v.parse().context("IPEMU_MAX_ARTIFACT_BYTES は整数")?,
            Err(_) => 32 * 1024 * 1024,
        };
        Ok(Self {
            listen,
            store_dir,
            db_path,
            signing_key_path: std::env::var("IPEMU_SIGNING_KEY").ok().map(PathBuf::from),
            admin_bootstrap_password: std::env::var("IPEMU_ADMIN_BOOTSTRAP_PASSWORD")
                .ok()
                .filter(|s| !s.is_empty()),
            public_url,
            web_dir: std::env::var("IPEMU_WEB_DIR")
                .ok()
                .map(PathBuf::from)
                .or_else(|| {
                    // 開発時の既定: ワークスペースからの相対
                    let p = PathBuf::from("apps/web/dist");
                    p.join("index.html").exists().then_some(p)
                }),
            max_artifact_bytes,
            session_ttl_secs: 12 * 3600,
            launch_token_ttl_secs: 3600,
            ticket_ttl_secs: 60,
            trust_proxy: std::env::var("IPEMU_TRUST_PROXY").as_deref() == Ok("1"),
        })
    }

    /// テスト用: 一時ディレクトリ配下にすべてを置く。
    pub fn for_test(dir: &std::path::Path) -> Self {
        Self {
            listen: "127.0.0.1:0".into(),
            store_dir: dir.join("store"),
            db_path: dir.join("store/db.sqlite"),
            signing_key_path: None,
            admin_bootstrap_password: Some("admin-password".into()),
            public_url: "http://127.0.0.1:0".into(),
            web_dir: None,
            max_artifact_bytes: 32 * 1024 * 1024,
            session_ttl_secs: 3600,
            launch_token_ttl_secs: 600,
            ticket_ttl_secs: 60,
            trust_proxy: false,
        }
    }
}
