//! ipemu-engine CLI。headless (run / validate / inspect / consent / catalog / lqr) と IPC サーバ (serve / attach、M2)。
//! run はローカル manifest (`--manifest`) か、サーバの構成 (`--server --launch`、M3) を動かす。M5: capability / 同意。
//!
//! 終了コード: 0 成功、1 一般エラー、3 未同意の capability あり、4 拒否された capability あり。

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use ipemu_core::hal::LogLevel;
use ipemu_engine::{
    bootstrap, BootstrapOptions, Cache, Credential, FileConsent, FirmwareSource, LaunchError,
    Recorder, Session, SessionOptions, TraceHash,
};
use std::path::PathBuf;
use std::sync::Arc;

pub const EXIT_CONSENT_REQUIRED: i32 = 3;
pub const EXIT_CONSENT_DENIED: i32 = 4;

/// capability / 同意まわりの共通オプション。
#[derive(Args, Debug, Clone, Default)]
struct CapabilityArgs {
    /// データディレクトリ (同意ストア `ipemu/consent.json` と fs capability の実体 `ipemu/data/`)。
    /// 省略時は IPEMU_DATA_DIR、なければ OS の既定
    #[arg(long)]
    data_dir: Option<PathBuf>,
    /// 事前同意 (headless 用)。`{"version":1,"grants":["<sha256>:<capability>"]}`。
    /// `*:<capability>` (アーティファクト不問) は --no-signature-check のときだけ使える
    #[arg(long)]
    consent_file: Option<PathBuf>,
    /// 同意ストアを読み書きしない (今回の回答だけで判断する)
    #[arg(long)]
    no_consent_store: bool,
}

impl CapabilityArgs {
    fn apply(&self, opts: &mut SessionOptions) -> anyhow::Result<()> {
        opts.data_dir = self.data_dir.clone();
        opts.no_consent_store = self.no_consent_store;
        if let Some(p) = &self.consent_file {
            // ワイルドカードはローカル開発モード (allow_unhashed) でのみ
            opts.consent = Some(Arc::new(FileConsent::load(p, opts.allow_unhashed)?));
        }
        Ok(())
    }
}

#[derive(Parser)]
#[command(
    name = "ipemu-engine",
    about = "WasmEmuSil エンジン (headless CLI + IPC サーバ)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

/// サーバ接続の共通引数 (M3)。
#[derive(Args, Clone, Default)]
struct ServerArgs {
    /// 配信サーバの URL (例 https://ipemu.example)
    #[arg(long, env = "IPEMU_SERVER")]
    server: Option<String>,
    /// クライアント API キー (headless / CI 用)
    #[arg(long, env = "IPEMU_API_KEY", conflicts_with_all = ["token", "ticket"])]
    api_key: Option<String>,
    /// 利用者のセッショントークン (シェル経由)
    #[arg(long, env = "IPEMU_TOKEN", conflicts_with = "ticket")]
    token: Option<String>,
    /// ipemu:// リンクの単回チケット (短命トークンに交換する)
    #[arg(long)]
    ticket: Option<String>,
    /// キャッシュディレクトリ (既定: IPEMU_CACHE_DIR か OS のキャッシュ領域の ipemu/)
    #[arg(long)]
    cache_dir: Option<PathBuf>,
    /// 信頼する公開鍵 keyId=hex (複数可。同梱鍵と IPEMU_TRUSTED_KEYS に追加)
    #[arg(long = "trusted-key")]
    trusted_keys: Vec<String>,
    /// サーバに問い合わせずキャッシュから起動する
    #[arg(long)]
    offline: bool,
    /// hello / telemetry を送らない
    #[arg(long)]
    no_telemetry: bool,
}

impl ServerArgs {
    fn credential(&self) -> Option<Credential> {
        if let Some(t) = &self.ticket {
            Some(Credential::Ticket(t.clone()))
        } else if let Some(t) = &self.token {
            Some(Credential::Token(t.clone()))
        } else {
            self.api_key.clone().map(Credential::ApiKey)
        }
    }
    fn cache(&self) -> anyhow::Result<Cache> {
        Cache::new(self.cache_dir.clone().unwrap_or_else(Cache::default_dir))
    }
    fn trusted_keys(&self) -> anyhow::Result<ipemu_sign::TrustedKeys> {
        let mut keys = ipemu_engine::bootstrap::default_trusted_keys()?;
        for k in &self.trusted_keys {
            for e in ipemu_sign::TrustedKeys::parse_list(k)?.to_entries() {
                keys.insert_hex(&e.key_id, &e.public_key)?;
            }
        }
        Ok(keys)
    }
}

/// `id` または `id@rev` を分解する。
fn parse_launch(s: &str) -> anyhow::Result<(String, Option<u32>)> {
    match s.split_once('@') {
        Some((id, rev)) => {
            Ok((
                id.to_owned(),
                Some(rev.parse().with_context(|| {
                    format!("--launch の revision `{rev}` が整数ではありません")
                })?),
            ))
        }
        None => Ok((s.to_owned(), None)),
    }
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)] // ServerArgs を flatten する Run / Catalog が大きいだけ
enum Cmd {
    /// manifest を組み立てて仮想時間で実行する (--manifest か --server --launch のどちらか)
    Run {
        /// ローカルの manifest ファイル
        #[arg(long, required_unless_present = "launch", conflicts_with = "launch")]
        manifest: Option<PathBuf>,
        /// サーバの構成 `id` または `id@rev` (--server が必要)
        #[arg(long, requires = "server")]
        launch: Option<String>,
        #[command(flatten)]
        srv: ServerArgs,
        /// slot=wasm:<path> | slot=native:<path>
        #[arg(long = "firmware")]
        firmware: Vec<String>,
        /// instance.key=value (JSON 値)
        #[arg(long = "param")]
        params: Vec<String>,
        #[arg(long, default_value_t = 10.0)]
        run_seconds: f64,
        /// スナップショットを CSV に記録する
        #[arg(long)]
        record: Option<PathBuf>,
        /// ローカル manifest (sha256 / 署名なし) や、未公開 revision の署名なし配信を許す
        #[arg(long)]
        no_signature_check: bool,
        /// 部品 / FW のログを表示するレベル (trace|debug|info|warn|error|off)
        #[arg(long, default_value = "info")]
        log: String,
        /// 進捗を N 秒ごとに 1 行出す (0 で無効)
        #[arg(long, default_value_t = 1.0)]
        print_every: f64,
        #[command(flatten)]
        cap: CapabilityArgs,
    },
    /// IPC サーバを立てる (シェルが spawn する形)。stdout の 1 行目に {"port","token",...} を出す
    Serve {
        /// 起動直後に組み立てる manifest (省略時はシェルからの session.launch を待つ)
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// slot=wasm:<path> | slot=native:<path>
        #[arg(long = "firmware")]
        firmware: Vec<String>,
        /// instance.key=value (JSON 値)
        #[arg(long = "param")]
        params: Vec<String>,
        #[arg(long)]
        no_signature_check: bool,
        /// realtime | fast | step (省略時は manifest の scheduler.pacing)
        #[arg(long)]
        pacing: Option<String>,
        #[arg(long, default_value_t = 1.0)]
        speed: f64,
        /// 組み立て後すぐ開始する
        #[arg(long)]
        auto_start: bool,
        /// 待ち受けポート (0 = ephemeral)
        #[arg(long, default_value_t = 0)]
        port: u16,
        /// 接続トークン (省略時は IPEMU_IPC_TOKEN、それも無ければ乱数)
        #[arg(long)]
        token: Option<String>,
        /// state フレームの送信レート [Hz]
        #[arg(long, default_value_t = 50.0)]
        state_hz: f64,
        /// slot=wasm:<path>: ビルド成果物の更新を監視して自動差替 (firmware.watch)
        #[arg(long = "watch")]
        watch: Vec<String>,
        /// 実時間の追いつきを止め、wasm 呼び出しの予算を無制限にする (デバッガ用)
        #[arg(long)]
        debug_friendly: bool,
        /// catalog.local が走査するディレクトリ (*.manifest.json)
        #[arg(long = "catalog-dir")]
        catalog_dirs: Vec<PathBuf>,
        /// stdin が閉じたら終了する (シェルが spawn したときの親死活監視)
        #[arg(long)]
        exit_on_stdin_close: bool,
        /// 同意ダイアログの回答を待つ上限 [s]
        #[arg(long, default_value_t = 600)]
        consent_timeout: u64,
        /// bootstrap のキャッシュディレクトリ (既定: IPEMU_CACHE_DIR か OS のキャッシュ領域の ipemu/)
        #[arg(long)]
        cache_dir: Option<PathBuf>,
        /// 信頼する公開鍵 keyId=hex (複数可。同梱鍵と IPEMU_TRUSTED_KEYS に追加)
        #[arg(long = "trusted-key")]
        trusted_keys: Vec<String>,
        /// サーバに問い合わせずキャッシュから起動する
        #[arg(long)]
        offline: bool,
        /// hello / telemetry を送らない
        #[arg(long)]
        no_telemetry: bool,
        /// `Origin` ヘッダ付き (ブラウザ) の接続で許す origin を追加する (既定 app://ui)。
        /// Vite の dev server から UI を開くときは `--allow-origin http://localhost:5173`
        #[arg(long = "allow-origin")]
        allow_origins: Vec<String>,
        #[command(flatten)]
        cap: CapabilityArgs,
    },
    /// 動作中のエンジンに接続して describe と state を端末に流す (デバッグ用)
    Attach {
        #[arg(long)]
        port: u16,
        /// 省略時は IPEMU_IPC_TOKEN
        #[arg(long)]
        token: Option<String>,
        /// state を表示する間隔 [s] (0 で表示しない)
        #[arg(long, default_value_t = 0.5)]
        every: f64,
        /// 接続後に送る要求 (JSON-RPC の method)。例: run.start
        #[arg(long = "send")]
        send: Vec<String>,
        /// この秒数で切断する (0 で Ctrl-C まで)
        #[arg(long, default_value_t = 0.0)]
        seconds: f64,
    },
    /// manifest を検証する (サーバと同じ規則)。実行はしないので capability は deny-all で付与し、同意は求めない
    Validate {
        manifest: PathBuf,
        #[arg(long)]
        no_signature_check: bool,
    },
    /// 同意 (consent) の確認・編集
    Consent {
        #[command(subcommand)]
        cmd: ConsentCmd,
    },
    /// 部品 wasm の describe() を表示する
    Inspect { wasm: PathBuf },
    /// サーバのカタログ (--server と資格情報) またはローカルカタログ (--local) を表示する
    Catalog {
        #[command(flatten)]
        srv: ServerArgs,
        /// キャッシュ済み構成の一覧 (サーバに問い合わせない)
        #[arg(long)]
        local: bool,
        /// JSON で出力する
        #[arg(long)]
        json: bool,
    },
    /// 離散 LQR ゲインを設計して firmware.params 用の JSON を出す
    Lqr {
        #[arg(long, default_value_t = 0.6)]
        length: f64,
        #[arg(long, default_value_t = 0.005)]
        ts: f64,
        #[arg(long, value_delimiter = ',', default_value = "40,60,1,0.5")]
        q: Vec<f64>,
        #[arg(long, default_value_t = 0.6)]
        r: f64,
    },
}

#[derive(Subcommand)]
enum ConsentCmd {
    /// manifest が要求する capability と同意の鍵を列挙する (--consent-file を書くための材料)
    List {
        #[arg(long)]
        manifest: PathBuf,
        /// slot=wasm:<path> | slot=native:<path>
        #[arg(long = "firmware")]
        firmware: Vec<String>,
        #[arg(long)]
        no_signature_check: bool,
        /// JSON で出す
        #[arg(long)]
        json: bool,
    },
    /// 同意ストアの内容を表示する
    Show {
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    /// 同意ストアから鍵を削除する (次回は再同意になる)。`--all` で全消去
    Forget {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        keys: Vec<String>,
        #[arg(long)]
        all: bool,
    },
}

fn parse_firmware_overrides(firmware: &[String], opts: &mut SessionOptions) -> anyhow::Result<()> {
    for f in firmware {
        let (slot, src) = f
            .split_once('=')
            .context("--firmware は slot=wasm:<path> の形式")?;
        opts.firmware_overrides
            .insert(slot.to_owned(), FirmwareSource::parse(src)?);
    }
    Ok(())
}

fn parse_level(s: &str) -> Option<LogLevel> {
    match s {
        "trace" => Some(LogLevel::Trace),
        "debug" => Some(LogLevel::Debug),
        "info" => Some(LogLevel::Info),
        "warn" => Some(LogLevel::Warn),
        "error" => Some(LogLevel::Error),
        _ => None,
    }
}

fn level_rank(l: LogLevel) -> u8 {
    match l {
        LogLevel::Trace => 0,
        LogLevel::Debug => 1,
        LogLevel::Info => 2,
        LogLevel::Warn => 3,
        LogLevel::Error => 4,
    }
}

fn main() {
    if let Err(e) = real_main() {
        eprintln!("error: {e:#}");
        let code = match e.downcast_ref::<LaunchError>() {
            Some(LaunchError::ConsentRequired(_)) => EXIT_CONSENT_REQUIRED,
            Some(LaunchError::ConsentDenied(_)) => EXIT_CONSENT_DENIED,
            None => 1,
        };
        std::process::exit(code);
    }
}

fn real_main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Run {
            manifest,
            launch,
            srv,
            firmware,
            params,
            run_seconds,
            record,
            no_signature_check,
            log,
            print_every,
            cap,
        } => {
            // 配信 manifest では sha256 を必須にする (--no-signature-check は署名の有無にだけ効く)
            let mut opts = SessionOptions {
                allow_unhashed: no_signature_check && manifest.is_some(),
                ..Default::default()
            };
            cap.apply(&mut opts)?;
            parse_firmware_overrides(&firmware, &mut opts)?;
            for p in params {
                let (lhs, rhs) = p
                    .split_once('=')
                    .context("--param は instance.key=value の形式")?;
                let (inst, key) = lhs
                    .split_once('.')
                    .context("--param は instance.key=value の形式")?;
                let val: serde_json::Value =
                    serde_json::from_str(rhs).unwrap_or(serde_json::Value::String(rhs.to_owned()));
                opts.param_overrides
                    .push((inst.to_owned(), key.to_owned(), val));
            }
            let min_level = parse_level(&log);

            // サーバ経由: bootstrap でキャッシュに揃えてから、同じ Session::load に渡す
            let mut telemetry = None;
            let manifest = match (&manifest, &launch) {
                (Some(p), _) => p.clone(),
                (None, Some(l)) => {
                    let (id, rev) = parse_launch(l)?;
                    let server = srv
                        .server
                        .clone()
                        .context("--launch には --server が必要です")?;
                    let cache = srv.cache()?;
                    let mut bo = BootstrapOptions::new(&server, &id, cache.clone());
                    bo.revision = rev;
                    bo.credential = srv.credential();
                    bo.trusted_keys = srv.trusted_keys()?;
                    bo.allow_unsigned = no_signature_check;
                    bo.offline = srv.offline;
                    bo.telemetry = !srv.no_telemetry;
                    let b = bootstrap(&bo)?;
                    eprintln!(
                        "bootstrap: `{}` rev {} {} (取得 {} / キャッシュ {}, 署名 {})",
                        b.manifest_id,
                        b.revision,
                        if b.from_cache {
                            "[オフライン / キャッシュ構成]"
                        } else {
                            "from server"
                        },
                        b.downloaded,
                        b.cache_hits,
                        b.verified_key_id.as_deref().unwrap_or("なし")
                    );
                    opts = b.session_options(&cache, opts);
                    telemetry = b.telemetry();
                    b.manifest_path.clone()
                }
                (None, None) => anyhow::bail!("--manifest か --launch を指定してください"),
            };
            let mut session = match Session::load(&manifest, &opts) {
                Ok(s) => s,
                Err(e) => {
                    if let Some(t) = &telemetry {
                        let _ = t.send(
                            "error",
                            serde_json::json!({"phase": "load", "error": format!("{e:#}")}),
                        );
                    }
                    return Err(e);
                }
            };
            if let Some(t) = &telemetry {
                if let Err(e) = t.send("started", serde_json::json!({"runSeconds": run_seconds})) {
                    eprintln!("warning: telemetry: {e:#}");
                }
            }
            for w in session.diagnostics.warnings() {
                eprintln!("warning[{}]: {}", w.rule, w.message);
            }
            for e in &session.capability_plan.entries {
                let caps: Vec<String> = e
                    .capabilities
                    .items()
                    .iter()
                    .map(|c| c.to_string())
                    .collect();
                eprintln!(
                    "capability `{}` ({}): {}",
                    e.instance,
                    e.artifact,
                    caps.join(", ")
                );
            }
            eprintln!(
                "manifest `{}` rev {}: 部品 {} 個, 配線 {} 本, 実行順 [{}], baseStep {} ns",
                session.manifest.id,
                session.manifest.revision,
                session.engine.graph().instances.len(),
                session.engine.graph().wires.len(),
                session
                    .engine
                    .graph()
                    .order
                    .iter()
                    .map(|&i| session.engine.graph().instances[i].as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                session.engine.base_step_ns()
            );

            let columns = session.snapshot_ports.clone();
            let mut recorder = match &record {
                Some(p) => Some(Recorder::new(
                    std::io::BufWriter::new(std::fs::File::create(p)?),
                    &columns,
                )?),
                None => None,
            };
            let mut hash = TraceHash::default();
            let mut next_print_ns = if print_every > 0.0 {
                (print_every * 1e9) as i64
            } else {
                i64::MAX
            };
            let mut last: Vec<f64> = Vec::new();
            let started = std::time::Instant::now();
            session.run_for(run_seconds, |t_ns, values, logs| {
                hash.push_row(t_ns, values);
                if let Some(r) = recorder.as_mut() {
                    let _ = r.row(t_ns, values);
                }
                for (inst, lvl, msg) in logs.drain(..) {
                    if min_level.is_some_and(|m| level_rank(lvl) >= level_rank(m)) {
                        eprintln!("[{:>9.3}s] {inst}: {msg}", t_ns as f64 * 1e-9);
                    }
                }
                if t_ns >= next_print_ns {
                    next_print_ns += (print_every * 1e9) as i64;
                    let line: Vec<String> = columns
                        .iter()
                        .zip(values)
                        .take(6)
                        .map(|(c, v)| format!("{c}={v:.4}"))
                        .collect();
                    eprintln!("[{:>9.3}s] {}", t_ns as f64 * 1e-9, line.join(" "));
                }
                last.clear();
                last.extend_from_slice(values);
            })?;
            let elapsed = started.elapsed();
            let h = hash.finish();
            if let Some(t) = &telemetry {
                if let Err(e) = t.send(
                    "stopped",
                    serde_json::json!({"virtualSeconds": run_seconds, "wallSeconds": elapsed.as_secs_f64(), "traceHash": h}),
                ) {
                    eprintln!("warning: telemetry: {e:#}");
                }
            }
            println!("trace-hash {h}");
            println!(
                "virtual {run_seconds:.3} s in {:.3} s wall ({:.1}x realtime)",
                elapsed.as_secs_f64(),
                run_seconds / elapsed.as_secs_f64().max(1e-9)
            );
            for (c, v) in columns.iter().zip(&last) {
                println!("{c} = {v}");
            }
            Ok(())
        }
        Cmd::Serve {
            manifest,
            firmware,
            params,
            no_signature_check,
            pacing,
            speed,
            auto_start,
            port,
            token,
            state_hz,
            watch,
            debug_friendly,
            catalog_dirs,
            exit_on_stdin_close,
            consent_timeout,
            allow_origins,
            cache_dir,
            trusted_keys,
            offline,
            no_telemetry,
            cap,
        } => serve(ServeArgs {
            manifest,
            firmware,
            params,
            no_signature_check,
            pacing,
            speed,
            auto_start,
            port,
            token,
            state_hz,
            watch,
            debug_friendly,
            catalog_dirs,
            exit_on_stdin_close,
            consent_timeout,
            allow_origins,
            cache_dir,
            trusted_keys,
            offline,
            no_telemetry,
            cap,
        }),
        Cmd::Attach {
            port,
            token,
            every,
            send,
            seconds,
        } => attach(port, token, every, send, seconds),
        Cmd::Validate {
            manifest,
            no_signature_check,
        } => {
            // 検証だけ: deny-all で実体化し、同意は求めず、I/O も起きない (D34)
            let opts = SessionOptions {
                allow_unhashed: no_signature_check,
                validate_only: true,
                no_consent_store: true,
                ..Default::default()
            };
            match Session::load(&manifest, &opts) {
                Ok(s) => {
                    print!("{}", s.diagnostics);
                    println!(
                        "ok: {} 部品, {} 配線",
                        s.engine.graph().instances.len(),
                        s.engine.graph().wires.len()
                    );
                    Ok(())
                }
                Err(e) => {
                    eprintln!("{e:#}");
                    std::process::exit(1);
                }
            }
        }
        Cmd::Catalog { srv, local, json } => {
            if local || srv.server.is_none() {
                let cache = srv.cache()?;
                let cat = cache.load_catalog();
                if json {
                    println!("{}", serde_json::to_string_pretty(&cat)?);
                } else {
                    println!("ローカルカタログ ({})", cache.root().display());
                    for e in &cat.entries {
                        let m = cache.get_manifest(&e.server, &e.manifest_id, e.revision)?;
                        let complete = m
                            .as_ref()
                            .map(|m| cache.missing_artifacts(m).is_empty())
                            .unwrap_or(false);
                        println!(
                            "  {}@{}{}  {}  [{}]  {}  最終起動 {}",
                            e.manifest_id,
                            e.revision,
                            if e.is_default { " (default)" } else { "" },
                            e.name,
                            e.server,
                            if complete { "起動可" } else { "不完全" },
                            e.last_launched_at
                        );
                    }
                }
                return Ok(());
            }
            let server = srv.server.clone().unwrap();
            let cred = srv
                .credential()
                .context("--api-key / --token / --ticket のいずれかが必要です")?;
            let cat = ipemu_engine::bootstrap::remote_catalog(
                &server,
                &cred,
                std::time::Duration::from_secs(30),
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&cat)?);
            } else {
                for it in cat["items"].as_array().cloned().unwrap_or_default() {
                    println!(
                        "{}  default={}  {}",
                        it["id"].as_str().unwrap_or(""),
                        it["defaultRevision"],
                        it["name"].as_str().unwrap_or("")
                    );
                    for r in it["revisions"].as_array().cloned().unwrap_or_default() {
                        println!(
                            "    rev {}  {}  {}",
                            r["revision"],
                            r["state"].as_str().unwrap_or(""),
                            r["name"].as_str().unwrap_or("")
                        );
                    }
                }
            }
            Ok(())
        }
        Cmd::Inspect { wasm } => {
            let bytes =
                std::fs::read(&wasm).with_context(|| format!("{} を読めません", wasm.display()))?;
            let rt = ipemu_wasm::Runtime::new(Default::default())?;
            // deny-all で実体化して describe() を読む (I/O は拒否される)。サーバの登録時と同じ API
            let (desc, imports) = ipemu_wasm::WasmPart::describe_component_sandboxed(&rt, &bytes)?;
            let out = serde_json::json!({
                "describe": desc,
                "imports": imports,
                // この部品を動かすのに最低限要る宣言の目安 (値はダミー)
                "requiredCapabilities": ipemu_core::Capabilities::covering_imports(&imports),
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
            Ok(())
        }
        Cmd::Consent { cmd } => consent_cmd(cmd),
        Cmd::Lqr { length, ts, q, r } => {
            anyhow::ensure!(q.len() == 4, "--q は 4 要素");
            let p = ip_model::Params {
                pendulum_length: length,
                ..Default::default()
            };
            let qd = [q[0], q[1], q[2], q[3]];
            let (sol, lin, _, _) =
                ip_model::lqr::design_gains(&p, ts, &qd, r).context("LQR が解けません")?;
            let round15 = |v: f64| -> f64 { format!("{v:.15e}").parse().unwrap() };
            let out = serde_json::json!({
                "k0": round15(sol.k[0]), "k1": round15(sol.k[1]), "k2": round15(sol.k[2]), "k3": round15(sol.k[3]),
                "controlHz": 1.0 / ts, "pendulumLength": length,
                "_unstablePoleRadPerSec": lin.unstable_pole, "_iterations": sol.iterations,
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
            Ok(())
        }
    }
}

fn consent_cmd(cmd: ConsentCmd) -> anyhow::Result<()> {
    use ipemu_engine::{ConsentStore, DataDirs};
    match cmd {
        ConsentCmd::List {
            manifest,
            firmware,
            no_signature_check,
            json,
        } => {
            let mut opts = SessionOptions {
                allow_unhashed: no_signature_check,
                ..Default::default()
            };
            parse_firmware_overrides(&firmware, &mut opts)?;
            let (m, reqs) = ipemu_engine::session::list_consent_requests(&manifest, &opts)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&reqs)?);
            } else {
                println!(
                    "manifest `{}` rev {}: 同意が要る capability {} 件",
                    m.id,
                    m.revision,
                    reqs.len()
                );
                for r in &reqs {
                    println!("  {}", r.request_id);
                    println!("      {}  [{}]", r.reason, r.instances.join(", "));
                }
                if !reqs.is_empty() {
                    println!("--consent-file 用:");
                    let f = ipemu_engine::consent::ConsentFile {
                        version: 1,
                        grants: reqs.iter().map(|r| r.request_id.clone()).collect(),
                        denies: Vec::new(),
                        remember: false,
                    };
                    println!("{}", serde_json::to_string_pretty(&f)?);
                }
            }
            Ok(())
        }
        ConsentCmd::Show { data_dir } => {
            let dirs = DataDirs::resolve(data_dir.as_deref());
            let store = ConsentStore::open(&dirs.consent_file())?;
            println!("{}", store.path().display());
            for (k, e) in store.entries() {
                println!(
                    "  {} {}  at {} ({} rev {})",
                    if e.granted { "granted" } else { "DENIED " },
                    k,
                    e.at,
                    e.manifest_id,
                    e.revision
                );
            }
            Ok(())
        }
        ConsentCmd::Forget {
            data_dir,
            keys,
            all,
        } => {
            let dirs = DataDirs::resolve(data_dir.as_deref());
            let mut store = ConsentStore::open(&dirs.consent_file())?;
            let mut n = 0;
            if all {
                let ks: Vec<String> = store.entries().map(|(k, _)| k.clone()).collect();
                for k in ks {
                    store.remove(&k);
                    n += 1;
                }
            } else {
                for k in &keys {
                    if store.remove(k) {
                        n += 1;
                    }
                }
            }
            store.save()?;
            println!("{n} 件を削除しました");
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// serve / attach (M2)
// ---------------------------------------------------------------------------

struct ServeArgs {
    manifest: Option<PathBuf>,
    firmware: Vec<String>,
    params: Vec<String>,
    no_signature_check: bool,
    pacing: Option<String>,
    speed: f64,
    auto_start: bool,
    port: u16,
    token: Option<String>,
    state_hz: f64,
    watch: Vec<String>,
    debug_friendly: bool,
    catalog_dirs: Vec<PathBuf>,
    exit_on_stdin_close: bool,
    consent_timeout: u64,
    allow_origins: Vec<String>,
    cache_dir: Option<PathBuf>,
    trusted_keys: Vec<String>,
    offline: bool,
    no_telemetry: bool,
    cap: CapabilityArgs,
}

fn parse_kv_list(items: &[String], what: &str) -> anyhow::Result<Vec<(String, String)>> {
    items
        .iter()
        .map(|f| {
            f.split_once('=')
                .map(|(a, b)| (a.to_owned(), b.to_owned()))
                .with_context(|| format!("{what} は slot=wasm:<path> の形式"))
        })
        .collect()
}

fn serve(a: ServeArgs) -> anyhow::Result<()> {
    use ipemu_engine::control::{self, ControllerConfig};
    use ipemu_engine::ipc_server::{self, ServerConfig};
    use ipemu_ipc::*;

    let token = a
        .token
        .or_else(|| {
            std::env::var("IPEMU_IPC_TOKEN")
                .ok()
                .filter(|t| !t.is_empty())
        })
        .unwrap_or_else(ipc_server::random_token);
    let debug_friendly = a.debug_friendly || debugger_present();

    // launch パラメータ (CLI → IPC と同じ形にして、シェルからの launch と同じ経路を通す)
    let launch = match &a.manifest {
        Some(m) => {
            let mut fw = std::collections::BTreeMap::new();
            for (slot, src) in parse_kv_list(&a.firmware, "--firmware")? {
                FirmwareSource::parse(&src)?;
                fw.insert(slot, src);
            }
            let mut params = Vec::new();
            for (lhs, rhs) in parse_kv_list(&a.params, "--param")? {
                let (inst, key) = lhs
                    .split_once('.')
                    .context("--param は instance.key=value の形式")?;
                let val: serde_json::Value =
                    serde_json::from_str(&rhs).unwrap_or(serde_json::Value::String(rhs.clone()));
                params.push(ParamOverride {
                    instance: inst.to_owned(),
                    key: key.to_owned(),
                    value: val,
                });
            }
            let pacing = match a.pacing.as_deref() {
                None => None,
                Some("realtime") => Some(PacingMode::Realtime),
                Some("fast") => Some(PacingMode::Fast),
                Some("step") => Some(PacingMode::Step),
                Some(other) => anyhow::bail!("--pacing `{other}` は realtime | fast | step"),
            }
            .map(|mode| PacingConfig {
                mode,
                speed: a.speed,
            });
            let abs = m.canonicalize().unwrap_or(m.clone());
            Some(LaunchParams {
                source: LaunchSource::Local {
                    manifest_path: abs.display().to_string(),
                    allow_unhashed: a.no_signature_check,
                },
                firmware: fw,
                params,
                pacing,
                auto_start: a.auto_start,
            })
        }
        None => None,
    };
    let watches = parse_kv_list(&a.watch, "--watch")?;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    // 同意: --consent-file があればそれ (headless)、なければ IPC の同意ダイアログ (シェル)
    let mut cap_opts = SessionOptions {
        allow_unhashed: a.no_signature_check,
        ..Default::default()
    };
    a.cap.apply(&mut cap_opts)?;

    // 署名検証の公開鍵: 同梱 + 追加 (M3 の ServerArgs と同じ規則)
    let mut trusted_keys = ipemu_engine::bootstrap::default_trusted_keys()?;
    for k in &a.trusted_keys {
        for e in ipemu_sign::TrustedKeys::parse_list(k)?.to_entries() {
            trusted_keys.insert_hex(&e.key_id, &e.public_key)?;
        }
    }

    rt.block_on(async move {
        // 制御器 → I/O の送信キューを先に作り、同意ブローカにも持たせる
        let (out_tx, out_rx) = tokio::sync::mpsc::unbounded_channel();
        let broker = Arc::new(ipemu_engine::consent_ipc::ConsentBroker::new(
            out_tx.clone(),
            std::time::Duration::from_secs(a.consent_timeout),
        ));
        let consent: Arc<dyn ipemu_engine::ConsentProvider> = match cap_opts.consent.clone() {
            Some(file) => file,
            None => broker.clone(),
        };
        let control::Spawned {
            handle: ctrl,
            state: state_r,
            outbound: _unused,
            join,
        } = control::spawn_with_outbound(
            ControllerConfig {
                debug_friendly,
                catalog_dirs: a.catalog_dirs,
                data_dir: cap_opts.data_dir.clone(),
                consent: Some(consent),
                no_consent_store: cap_opts.no_consent_store,
                cache_dir: a.cache_dir.clone(),
                trusted_keys,
                allow_unsigned: a.no_signature_check,
                offline: a.offline,
                telemetry: !a.no_telemetry,
                ..Default::default()
            },
            out_tx,
        )?;
        let server = ipc_server::start(
            ServerConfig {
                port: a.port,
                token,
                state_hz: a.state_hz,
                allowed_origins: ipc_server::DEFAULT_ALLOWED_ORIGINS
                    .iter()
                    .map(|s| s.to_string())
                    .chain(a.allow_origins.iter().cloned())
                    .collect(),
            },
            ctrl.clone(),
            state_r,
            out_rx,
            Some(broker),
        )
        .await?;
        // 1 行目: 接続情報 (シェルはこれを読んで接続する)。以降 stdout には何も出さない
        {
            use std::io::Write;
            let mut so = std::io::stdout().lock();
            writeln!(so, "{}", serde_json::to_string(&server.discovery)?)?;
            so.flush()?;
        }
        eprintln!(
            "ipemu-engine serve: ws://127.0.0.1:{} (pid {}){}",
            server.addr.port(),
            std::process::id(),
            if debug_friendly {
                " [debug-friendly]"
            } else {
                ""
            }
        );

        if let Some(l) = launch {
            match ctrl.call(RequestBody::SessionLaunch(l)).await {
                Ok(v) => eprintln!(
                    "launched `{}` rev {}",
                    v["manifest"]["id"].as_str().unwrap_or("?"),
                    v["manifest"]["revision"]
                ),
                Err(e) => eprintln!("launch failed: {} ({})", e.message, e.code),
            }
            for (slot, src) in watches {
                if let Err(e) = ctrl
                    .call(RequestBody::FirmwareWatch(FirmwareWatchParams {
                        slot: slot.clone(),
                        source: Some(src.clone()),
                    }))
                    .await
                {
                    eprintln!("watch {slot}={src}: {}", e.message);
                } else {
                    eprintln!("watching {slot} = {src}");
                }
            }
        }

        // 終了条件: Ctrl-C、stdin の EOF (親が死んだ)、制御器スレッドの終了
        let stdin_closed = async {
            if a.exit_on_stdin_close {
                use tokio::io::AsyncReadExt;
                let mut stdin = tokio::io::stdin();
                let mut buf = [0u8; 256];
                loop {
                    match stdin.read(&mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                }
            } else {
                std::future::pending::<()>().await;
            }
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => eprintln!("ipemu-engine: ctrl-c"),
            _ = stdin_closed => eprintln!("ipemu-engine: stdin closed, exiting"),
        }
        server.abort();
        ctrl.shutdown();
        let _ = tokio::task::spawn_blocking(move || join.join()).await;
        Ok::<(), anyhow::Error>(())
    })
}

/// デバッガの接続を検出する (Linux: TracerPid、他 OS は未実装で false)。
fn debugger_present() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(s) = std::fs::read_to_string("/proc/self/status") {
            return s
                .lines()
                .find_map(|l| l.strip_prefix("TracerPid:"))
                .map(|v| v.trim() != "0")
                .unwrap_or(false);
        }
    }
    false
}

fn attach(
    port: u16,
    token: Option<String>,
    every: f64,
    send: Vec<String>,
    seconds: f64,
) -> anyhow::Result<()> {
    use ipemu_engine::ipc_server::client::{Client, Incoming};
    use ipemu_ipc::*;
    let token = token
        .or_else(|| std::env::var("IPEMU_IPC_TOKEN").ok())
        .context("--token か IPEMU_IPC_TOKEN が必要です")?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async move {
        let mut c = Client::connect(port, &token).await?;
        let d: DescribeResult = c.call_as(RequestBody::SessionDescribe(Empty {})).await?;
        println!("{}", serde_json::to_string_pretty(&d)?);
        let ports = d.session.as_ref().map(|s| s.snapshot_ports.clone()).unwrap_or_default();
        for m in send {
            let r = c
                .call(serde_json::from_value(serde_json::json!({"method": m, "params": {}}))
                    .with_context(|| format!("--send {m}"))?)
                .await;
            println!("{m}: {}", match r { Ok(v) => v.to_string(), Err(e) => format!("error {}: {}", e.code, e.message) });
        }
        let started = std::time::Instant::now();
        let mut last_print = std::time::Instant::now() - std::time::Duration::from_secs(10);
        let deadline = async {
            if seconds > 0.0 {
                tokio::time::sleep(std::time::Duration::from_secs_f64(seconds)).await
            } else {
                std::future::pending::<()>().await
            }
        };
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                _ = &mut deadline => break,
                _ = tokio::signal::ctrl_c() => break,
                m = c.incoming.recv() => match m {
                    None => break,
                    Some(Incoming::State(f)) => {
                        if every > 0.0 && last_print.elapsed().as_secs_f64() >= every {
                            last_print = std::time::Instant::now();
                            let cols: Vec<String> = ports.iter().zip(&f.values).take(6)
                                .map(|(c, v)| format!("{c}={v:.4}")).collect();
                            println!("[{:>9.3}s] {:?} x{:.2} {} (ratio {:.2}, slip {:.0}ms)",
                                f.t_ns as f64 * 1e-9, f.state, f.speed, cols.join(" "),
                                f.stats.realtime_ratio, f.stats.slip_ms_total);
                        }
                    }
                    Some(Incoming::Scope(f)) => println!("scope #{} t0={} n={}", f.subscription_id, f.t0_ns, f.channels.first().map(|c| c.len()).unwrap_or(0)),
                    Some(Incoming::Notification(n)) => println!("{}", serde_json::to_string(&n)?),
                    Some(Incoming::Other(t)) => println!("? {t}"),
                }
            }
        }
        let _ = started;
        Ok::<(), anyhow::Error>(())
    })
}
