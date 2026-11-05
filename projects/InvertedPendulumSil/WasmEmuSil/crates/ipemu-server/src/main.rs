//! ipemu-server CLI: `serve` (既定) / `keygen`。

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ipemu-server", about = "WasmEmuSil 配信サーバ (M3/M4)")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// 環境変数の設定で待ち受ける (IPEMU_LISTEN, IPEMU_STORE, IPEMU_DB, IPEMU_SIGNING_KEY, IPEMU_ADMIN_BOOTSTRAP_PASSWORD, IPEMU_PUBLIC_URL, IPEMU_WEB_DIR, IPEMU_MAX_ARTIFACT_BYTES, IPEMU_TRUST_PROXY)
    Serve,
    /// Ed25519 署名鍵を生成する。`<out>/signing.json` (秘密、0600) と `<out>/public.json` (クライアントに配る) を書く
    Keygen {
        #[arg(long, default_value = "dev/keys")]
        out: PathBuf,
        #[arg(long, default_value = "dev-2026")]
        key_id: String,
        /// 既存のファイルを上書きする
        #[arg(long)]
        force: bool,
    },
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,sqlx=warn"));
    if std::env::var("IPEMU_LOG_FORMAT").as_deref() == Ok("text") {
        fmt().with_env_filter(filter).init();
    } else {
        fmt().json().with_env_filter(filter).init();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd.unwrap_or(Cmd::Serve) {
        Cmd::Serve => {
            init_tracing();
            let cfg = ipemu_server::config::Config::from_env()?;
            tracing::info!(listen = %cfg.listen, store = %cfg.store_dir.display(), public_url = %cfg.public_url, web = ?cfg.web_dir, "starting");
            let app = ipemu_server::App::start(cfg).await?;
            tracing::info!(url = %app.base_url, key_id = %app.state.key_id, public_key = %app.state.public_key_entry().public_key, "listening");
            tokio::signal::ctrl_c().await?;
            tracing::info!("shutting down");
            app.stop().await;
            Ok(())
        }
        Cmd::Keygen { out, key_id, force } => {
            std::fs::create_dir_all(&out)?;
            let priv_path = out.join("signing.json");
            let pub_path = out.join("public.json");
            if priv_path.exists() && !force {
                anyhow::bail!("{} が既にあります (--force で上書き)", priv_path.display());
            }
            let kf = ipemu_sign::SigningKeyFile::generate(&key_id);
            std::fs::write(&priv_path, serde_json::to_string_pretty(&kf)?)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&priv_path, std::fs::Permissions::from_mode(0o600))?;
            }
            let pubk = kf.public_entry()?;
            std::fs::write(
                &pub_path,
                serde_json::to_string_pretty(&serde_json::json!({ "keys": [pubk] }))?,
            )?;
            println!("signing key : {}", priv_path.display());
            println!("public key  : {}", pub_path.display());
            println!("keyId       : {}", pubk.key_id);
            println!("publicKey   : {}", pubk.public_key);
            println!(
                "engine 側   : --trusted-key {}={}  (または IPEMU_TRUSTED_KEYS)",
                pubk.key_id, pubk.public_key
            );
            Ok(())
        }
    }
}
