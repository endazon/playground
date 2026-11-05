//! ipemu-engine CLI (headless)。ローカル manifest (`--manifest`) か、サーバの構成 (`--server --launch`) を動かす。
//! M5: capability / 同意。
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
#[command(name = "ipemu-engine", about = "WasmEmuSil エンジン (headless)")]
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
                    let mut bo = BootstrapOptions::new(&server, &id, cache.clone())?;
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
