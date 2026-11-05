//! 同意 (consent) とエンジンの起動可否の結合テスト。
//!
//! 前提: build/components/*.wasm (tcp-reporter を含む)。無ければ skip。

use ipemu_core::consent::{consent_key, ConsentAnswer, ConsentEntry, ConsentRequest};
use ipemu_engine::consent::{ConsentFile, FileConsent};
use ipemu_engine::{ConsentProvider, ConsentStore, LaunchError, Session, SessionOptions};
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn ready() -> bool {
    let p = root().join("build/components/tcp-reporter.wasm");
    if !p.exists() {
        eprintln!("SKIP: {} がありません", p.display());
        return false;
    }
    true
}

fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ipemu-consent-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// tcp-reporter 1 個 + encoder (capability なし) の manifest を一時ディレクトリに書く。
fn write_manifest(dir: &Path, port: u16, extra_caps: serde_json::Value) -> PathBuf {
    let comps = root().join("build/components");
    let mut caps = serde_json::json!({
        "net": { "tcpConnect": [format!("127.0.0.1:{port}")] },
        "fs": [{ "path": "logs", "mode": "rw" }]
    });
    if let (Some(a), Some(b)) = (caps.as_object_mut(), extra_caps.as_object()) {
        for (k, v) in b {
            a.insert(k.clone(), v.clone());
        }
    }
    let m = serde_json::json!({
        "manifestVersion": 1, "id": "consent-test", "revision": 2, "name": "consent test",
        "scheduler": { "baseStepNs": 1000000, "pacing": "fast", "snapshotDivider": 10 },
        "artifacts": {
            "tcp-reporter@0.1.0": { "path": comps.join("tcp-reporter.wasm") },
            "encoder@0.1.0": { "path": comps.join("encoder.wasm") }
        },
        "components": [
            { "instance": "enc", "artifact": "encoder@0.1.0", "params": { "ppr": 100, "mode": "angular" } },
            { "instance": "rep", "artifact": "tcp-reporter@0.1.0",
              "params": { "host": "127.0.0.1", "port": port, "everyMs": 20, "file": "logs/out.csv" },
              "capabilities": caps }
        ],
        "wires": [ { "from": "enc.count", "to": "rep.value" } ]
    });
    let p = dir.join("test.manifest.json");
    std::fs::write(&p, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    p
}

fn listener() -> (u16, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let lines = Arc::new(Mutex::new(Vec::new()));
    let out = lines.clone();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let mut r = BufReader::new(s);
            let mut line = String::new();
            while let Ok(n) = r.read_line(&mut line) {
                if n == 0 {
                    break;
                }
                out.lock().unwrap().push(line.trim_end().to_owned());
                line.clear();
            }
        }
    });
    (port, lines)
}

fn sha_of(p: &Path) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(std::fs::read(p).unwrap()))
}

#[test]
fn consent_store_roundtrip_and_expiry() {
    let dir = temp("store");
    let path = dir.join("ipemu").join("consent.json");
    let mut s = ConsentStore::open(&path).unwrap();
    let key = consent_key(&"ab".repeat(32), "clock");
    s.set(
        key.clone(),
        ConsentEntry {
            granted: true,
            at: "2026-11-05T00:00:00Z".into(),
            manifest_id: "m".into(),
            revision: 1,
            remember_until: Some("2026-12-01T00:00:00Z".into()),
        },
    );
    s.save().unwrap();
    let s2 = ConsentStore::open(&path).unwrap();
    assert!(s2
        .get(&key, "2026-11-06T00:00:00Z")
        .is_some_and(|e| e.granted));
    assert!(
        s2.get(&key, "2026-12-01T00:00:00Z").is_none(),
        "期限切れは無視"
    );
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"manifestId\": \"m\""), "{text}");
    assert!(text.contains(&key));
    // 壊れたファイルは黙って捨てない
    std::fs::write(&path, "{broken").unwrap();
    assert!(ConsentStore::open(&path).is_err());
}

#[test]
fn requests_are_listed_without_instantiation() {
    if !ready() {
        return;
    }
    let dir = temp("list");
    let m = write_manifest(&dir, 9, serde_json::json!({}));
    let (manifest, reqs) =
        ipemu_engine::session::list_consent_requests(&m, &SessionOptions::default()).unwrap();
    assert_eq!(manifest.id, "consent-test");
    let sha = sha_of(&root().join("build/components/tcp-reporter.wasm"));
    let keys: Vec<&str> = reqs.iter().map(|r| r.request_id.as_str()).collect();
    assert_eq!(
        keys,
        [
            format!("{sha}:net.tcpConnect:127.0.0.1:9"),
            format!("{sha}:fs:rw:logs")
        ]
    );
    assert_eq!(reqs[0].instances, ["rep"]);
    assert_eq!(reqs[0].revision, 2);
    assert!(reqs[0].reason.contains("TCP"));
}

#[test]
fn launch_requires_consent_and_denial_blocks_everything() {
    if !ready() {
        return;
    }
    let dir = temp("launch");
    let (port, lines) = listener();
    let m = write_manifest(&dir, port, serde_json::json!({}));
    let base = SessionOptions {
        allow_unhashed: true,
        data_dir: Some(dir.join("data")),
        ..Default::default()
    };

    // 1) 問い合わせ先なし → 未同意で起動不可
    let err = Session::load(&m, &base)
        .err()
        .expect("未同意で起動できてはいけない");
    match err.downcast_ref::<LaunchError>() {
        Some(LaunchError::ConsentRequired(rs)) => assert_eq!(rs.len(), 2),
        other => panic!("{other:?}: {err:#}"),
    }

    // 2) 片方だけ同意 → 残りが未同意なので起動不可 (部分起動しない)
    let sha = sha_of(&root().join("build/components/tcp-reporter.wasm"));
    let partial = Arc::new(
        FileConsent::from_file(
            ConsentFile {
                version: 1,
                grants: vec![format!("{sha}:fs:rw:logs")],
                denies: vec![],
                remember: false,
            },
            false,
        )
        .unwrap(),
    );
    let opts = SessionOptions {
        consent: Some(partial),
        ..base.clone()
    };
    match Session::load(&m, &opts)
        .err()
        .unwrap()
        .downcast_ref::<LaunchError>()
    {
        Some(LaunchError::ConsentRequired(rs)) => {
            assert_eq!(rs[0].capability, format!("net.tcpConnect:127.0.0.1:{port}"))
        }
        other => panic!("{other:?}"),
    }

    // 3) 拒否 → 起動不可
    let deny_file = ConsentFile {
        version: 1,
        grants: vec!["*:fs:rw:logs".into()],
        denies: vec![format!("*:net.tcpConnect:127.0.0.1:{port}")],
        remember: false,
    };
    assert!(
        FileConsent::from_file(deny_file.clone(), false).is_err(),
        "ワイルドカードは開発モード限定"
    );
    let deny = Arc::new(FileConsent::from_file(deny_file, true).unwrap());
    let opts = SessionOptions {
        consent: Some(deny),
        ..base.clone()
    };
    assert!(matches!(
        Session::load(&m, &opts).err().unwrap().downcast_ref::<LaunchError>(),
        Some(LaunchError::ConsentDenied(rs)) if rs.len() == 1
    ));

    // 4) 全部同意 (記憶する) → 起動して値が届く。ストアにも残る
    struct Remember;
    impl ConsentProvider for Remember {
        fn ask(&self, rs: &[ConsentRequest]) -> anyhow::Result<Vec<ConsentAnswer>> {
            Ok(rs
                .iter()
                .map(|r| ConsentAnswer {
                    request_id: r.request_id.clone(),
                    grant: true,
                    remember: true,
                })
                .collect())
        }
        fn name(&self) -> &'static str {
            "test"
        }
    }
    let opts = SessionOptions {
        consent: Some(Arc::new(Remember)),
        ..base.clone()
    };
    let mut s = Session::load(&m, &opts).unwrap();
    assert_eq!(s.capability_plan.entries.len(), 1);
    s.run_for(0.1, |_, _, _| {}).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert_eq!(lines.lock().unwrap().len(), 5, "20 ms ごとに 100 ms 分");
    let file = dir.join("data/ipemu/data/logs/out.csv");
    assert_eq!(std::fs::read_to_string(&file).unwrap().lines().count(), 5);
    let store = ConsentStore::open(&dir.join("data/ipemu/consent.json")).unwrap();
    assert_eq!(store.entries().count(), 2);
    assert!(store
        .entries()
        .all(|(_, e)| e.granted && e.manifest_id == "consent-test" && e.revision == 2));

    // 5) 記憶済みなので問い合わせ先なしでも起動できる
    let s = Session::load(&m, &base).unwrap();
    assert!(!s.diagnostics.has_errors());
    drop(s);

    // 6) 部品を更新 (sha が変わる) と再同意になる: 別の wasm (encoder) を tcp-reporter の位置に置いた manifest
    let m2 = {
        let text = std::fs::read_to_string(&m).unwrap();
        let text = text.replace("tcp-reporter.wasm", "encoder.wasm");
        let p = dir.join("swapped.manifest.json");
        std::fs::write(&p, text).unwrap();
        p
    };
    match Session::load(&m2, &base)
        .err()
        .unwrap()
        .downcast_ref::<LaunchError>()
    {
        Some(LaunchError::ConsentRequired(rs)) => assert_eq!(rs.len(), 2),
        other => panic!("{other:?}"),
    }
}

#[test]
fn v7_rejects_undeclared_imports_before_consent() {
    if !ready() {
        return;
    }
    let dir = temp("v7");
    let m = write_manifest(&dir, 9, serde_json::json!({}));
    // net の宣言を消す → tcp-reporter は wasi:sockets を import するので V7
    let text = std::fs::read_to_string(&m).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["components"][1]["capabilities"] =
        serde_json::json!({ "fs": [{ "path": "logs", "mode": "rw" }] });
    std::fs::write(&m, serde_json::to_vec(&v).unwrap()).unwrap();
    let opts = SessionOptions {
        allow_unhashed: true,
        data_dir: Some(dir.join("data")),
        validate_only: true,
        no_consent_store: true,
        ..Default::default()
    };
    let err = format!("{:#}", Session::load(&m, &opts).err().unwrap());
    assert!(err.contains("V7") && err.contains("wasi:sockets"), "{err}");
}

#[test]
fn v8_warns_on_overdeclaration_and_v12_checks_hash() {
    if !ready() {
        return;
    }
    let dir = temp("v8");
    let (port, _) = listener();
    let m = write_manifest(&dir, port, serde_json::json!({ "random": true }));
    let opts = SessionOptions {
        allow_unhashed: true,
        data_dir: Some(dir.join("data")),
        validate_only: true,
        no_consent_store: true,
        ..Default::default()
    };
    let s = Session::load(&m, &opts).unwrap();
    let rules: Vec<&str> = s.diagnostics.warnings().map(|d| d.rule).collect();
    assert!(rules.contains(&"V8"), "{rules:?}");
    assert!(rules.contains(&"V10"), "{rules:?}");
    drop(s);

    // approval.capabilitiesHash が合わなければ V12
    let text = std::fs::read_to_string(&m).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["approval"] = serde_json::json!({ "approvedBy": "a", "approvedAt": "2026-11-05T00:00:00Z", "capabilitiesHash": "0".repeat(64) });
    std::fs::write(&m, serde_json::to_vec(&v).unwrap()).unwrap();
    let err = format!("{:#}", Session::load(&m, &opts).err().unwrap());
    assert!(err.contains("V12"), "{err}");

    // 正しいハッシュなら通る
    let manifest = ipemu_core::Manifest::from_json(&std::fs::read_to_string(&m).unwrap()).unwrap();
    v["approval"]["capabilitiesHash"] = serde_json::json!(ipemu_core::capabilities_hash(&manifest));
    std::fs::write(&m, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(Session::load(&m, &opts).is_ok());
}

#[test]
fn validate_only_instantiates_with_deny_all_and_no_io() {
    if !ready() {
        return;
    }
    let dir = temp("validate");
    let (port, lines) = listener();
    let m = write_manifest(&dir, port, serde_json::json!({}));
    // init の中で TCP 接続を試みる設定にする
    let text = std::fs::read_to_string(&m).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["components"][1]["params"]["connectOnInit"] = serde_json::json!(true);
    std::fs::write(&m, serde_json::to_vec(&v).unwrap()).unwrap();

    let opts = SessionOptions {
        allow_unhashed: true,
        data_dir: Some(dir.join("data")),
        validate_only: true,
        no_consent_store: true,
        ..Default::default()
    };
    // 同意なしで通る (問い合わせ先もストアもない) が、I/O は一切起きない
    let s = Session::load(&m, &opts).unwrap();
    assert!(!s.diagnostics.has_errors());
    let rep = s
        .engine
        .graph()
        .instances
        .iter()
        .position(|i| i == "rep")
        .unwrap();
    let d = &s.engine.graph().descs[rep];
    let errors = d.output_index("errors").unwrap();
    let last = d.output_index("last-error").unwrap();
    assert_eq!(
        s.engine.output(rep, errors),
        1.0,
        "init 中の接続は拒否されて失敗として数えられる"
    );
    assert_eq!(s.engine.output(rep, last), 1.0, "access denied");
    drop(s);
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(
        lines.lock().unwrap().is_empty(),
        "validate で外部へ届いてはいけない"
    );
    assert!(
        !dir.join("data").exists(),
        "validate はデータディレクトリに触れない"
    );

    // 同じ manifest を run 相当 (同意あり・実グラント) で読むと init 中の接続が届く
    struct Yes;
    impl ConsentProvider for Yes {
        fn ask(&self, rs: &[ConsentRequest]) -> anyhow::Result<Vec<ConsentAnswer>> {
            Ok(rs
                .iter()
                .map(|r| ConsentAnswer {
                    request_id: r.request_id.clone(),
                    grant: true,
                    remember: false,
                })
                .collect())
        }
        fn name(&self) -> &'static str {
            "yes"
        }
    }
    let opts = SessionOptions {
        validate_only: false,
        consent: Some(Arc::new(Yes)),
        ..opts
    };
    let s = Session::load(&m, &opts).unwrap();
    assert_eq!(s.engine.output(rep, errors), 0.0);
    drop(s);
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(lines.lock().unwrap().len(), 1);
}

#[test]
fn cli_exits_nonzero_without_consent_file() {
    if !ready() {
        return;
    }
    let dir = temp("cli");
    let (port, lines) = listener();
    let m = write_manifest(&dir, port, serde_json::json!({}));
    let exe = env!("CARGO_BIN_EXE_ipemu-engine");
    let data = dir.join("data");

    let out = std::process::Command::new(exe)
        .args(["run", "--manifest"])
        .arg(&m)
        .args([
            "--no-signature-check",
            "--run-seconds",
            "0.1",
            "--print-every",
            "0",
            "--data-dir",
        ])
        .arg(&data)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("同意されていない"));

    // consent list → 鍵を得て consent-file を書く
    let out = std::process::Command::new(exe)
        .args(["consent", "list", "--manifest"])
        .arg(&m)
        .args(["--no-signature-check", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let reqs: Vec<ConsentRequest> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(reqs.len(), 2);
    let cf = dir.join("consent.json");
    std::fs::write(
        &cf,
        serde_json::to_vec(&ConsentFile {
            version: 1,
            grants: reqs.iter().map(|r| r.request_id.clone()).collect(),
            denies: vec![],
            remember: false,
        })
        .unwrap(),
    )
    .unwrap();

    let out = std::process::Command::new(exe)
        .args(["run", "--manifest"])
        .arg(&m)
        .args([
            "--no-signature-check",
            "--run-seconds",
            "0.1",
            "--print-every",
            "0",
            "--data-dir",
        ])
        .arg(&data)
        .arg("--consent-file")
        .arg(&cf)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert_eq!(lines.lock().unwrap().len(), 5);

    // 拒否 → 4
    std::fs::write(
        &cf,
        serde_json::to_vec(&ConsentFile {
            version: 1,
            grants: vec![],
            denies: vec!["*:fs:rw:logs".into()],
            remember: false,
        })
        .unwrap(),
    )
    .unwrap();
    let out = std::process::Command::new(exe)
        .args(["run", "--manifest"])
        .arg(&m)
        .args(["--no-signature-check", "--run-seconds", "0.1", "--data-dir"])
        .arg(&data)
        .arg("--consent-file")
        .arg(&cf)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(4),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
