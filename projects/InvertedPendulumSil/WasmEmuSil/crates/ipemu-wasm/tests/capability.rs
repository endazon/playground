//! capability の実施の結合テスト: 検証用部品 `tcp-reporter` (wasi:sockets + wasi:filesystem) を使う。
//!
//! 前提: `scripts/build-components.sh` で build/components/tcp-reporter.wasm があること (無ければ skip)。

use ipemu_core::capability::Capabilities;
use ipemu_core::Part;
use ipemu_wasm::{Runtime, WasiGrant, WasmPart};
use std::io::{BufRead, BufReader};
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn reporter_path() -> Option<PathBuf> {
    let p =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../build/components/tcp-reporter.wasm");
    if p.exists() {
        Some(p)
    } else {
        eprintln!(
            "SKIP: {} がありません (scripts/build-components.sh を先に実行)",
            p.display()
        );
        None
    }
}

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ipemu-cap-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// 受けた行を集めるローカル TCP リスナ。
struct Sink {
    addr: SocketAddr,
    lines: Arc<Mutex<Vec<String>>>,
}

fn spawn_sink(bind: SocketAddr) -> Sink {
    let l = TcpListener::bind(bind).unwrap();
    let addr = l.local_addr().unwrap();
    let lines = Arc::new(Mutex::new(Vec::new()));
    let out = lines.clone();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
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
    Sink { addr, lines }
}

fn caps(json: serde_json::Value) -> Capabilities {
    serde_json::from_value(json).unwrap()
}

fn run_steps(part: &mut WasmPart, value: f64, n: usize) -> Vec<f64> {
    let mut out = vec![0.0; part.describe().scalar_output_count()];
    for _ in 0..n {
        part.set_inputs(&[value]).unwrap();
        part.step(10_000_000).unwrap(); // 10 ms
    }
    part.get_outputs(&mut out).unwrap();
    out
}

#[test]
fn imports_of_reporter_match_expected_set() {
    let Some(p) = reporter_path() else { return };
    let bytes = std::fs::read(p).unwrap();
    let imports = ipemu_core::imports::component_imports(&bytes).unwrap();
    let names: Vec<&str> = imports
        .iter()
        .filter(|i| !i.types_only)
        .map(|i| i.name.as_str())
        .collect();
    assert!(names.contains(&"wasi:sockets/tcp@0.2.12"), "{names:?}");
    assert!(names.contains(&"wasi:filesystem/types@0.2.12"), "{names:?}");
    assert!(names.contains(&"wasi:io/streams@0.2.12"), "{names:?}");
    assert!(
        !names.iter().any(|n| n.starts_with("wasi:cli/")),
        "wasi:cli が混ざっている: {names:?}"
    );
    assert!(
        !names.iter().any(|n| n.starts_with("wasi:clocks/")),
        "{names:?}"
    );

    // V7: 宣言なしなら拒否、宣言ありなら通る
    let (errs, _) = Capabilities::default().check_imports(&imports);
    assert!(!errs.is_empty());
    let ok = caps(serde_json::json!({
        "net": { "tcpConnect": ["127.0.0.1:1"] }, "fs": [{ "path": "logs", "mode": "rw" }]
    }));
    let (errs, warns) = ok.check_imports(&imports);
    assert!(errs.is_empty(), "{errs:?}");
    assert!(warns.is_empty(), "{warns:?}");
}

#[test]
fn instantiation_fails_without_grant() {
    let Some(p) = reporter_path() else { return };
    let rt = Runtime::new(Default::default()).unwrap();
    let c = WasmPart::compile_file(&rt, &p).unwrap();
    let err = WasmPart::instantiate(&rt, &c, "rep")
        .err()
        .expect("宣言なしで実体化できてはいけない");
    let msg = format!("{err:#}");
    assert!(msg.contains("wasi:"), "{msg}");

    // fs だけ付与しても sockets が無いので失敗する
    let dir = temp_dir("fs-only");
    let g = WasiGrant::from_capabilities(
        &caps(serde_json::json!({ "fs": [{ "path": "logs", "mode": "rw" }] })),
        &dir,
    )
    .unwrap();
    let err = WasmPart::instantiate_with(&rt, &c, "rep", g)
        .err()
        .expect("sockets なしで実体化できてはいけない");
    assert!(format!("{err:#}").contains("wasi:sockets"), "{err:#}");
}

#[test]
fn tcp_to_allowed_host_and_file_in_data_dir() {
    let Some(p) = reporter_path() else { return };
    let sink = spawn_sink("127.0.0.1:0".parse().unwrap());
    let dir = temp_dir("ok");
    let rt = Runtime::new(Default::default()).unwrap();
    let c = WasmPart::compile_file(&rt, &p).unwrap();
    let g = WasiGrant::from_capabilities(
        &caps(serde_json::json!({
            "net": { "tcpConnect": [format!("127.0.0.1:{}", sink.addr.port())] },
            "fs": [{ "path": "logs", "mode": "rw" }]
        })),
        &dir,
    )
    .unwrap();
    let mut part = WasmPart::instantiate_with(&rt, &c, "rep", g).unwrap();
    assert!(
        part.describe().deterministic,
        "clock / random を与えていないので決定性は保たれる"
    );
    part.init(&serde_json::json!({ "host": "127.0.0.1", "port": sink.addr.port(), "everyMs": 10, "file": "logs/report.csv" }))
        .unwrap();
    let out = run_steps(&mut part, 1.5, 5);
    assert_eq!(
        out,
        [5.0, 0.0, 0.0, 5.0 * "10000000,1.5\n".len() as f64],
        "sent/errors/last-error/file-bytes"
    );

    std::thread::sleep(Duration::from_millis(200));
    let lines = sink.lines.lock().unwrap().clone();
    assert_eq!(lines.len(), 5, "{lines:?}");
    assert_eq!(lines[0], "10000000,1.5");
    assert_eq!(lines[4], "50000000,1.5");

    // ファイルはデータディレクトリ配下 (preopen) にだけできる
    let file = dir.canonicalize().unwrap().join("logs").join("report.csv");
    let text = std::fs::read_to_string(&file).unwrap();
    assert_eq!(text.lines().count(), 5);
    assert!(part.take_denied_destinations().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tcp_to_undeclared_host_is_denied() {
    let Some(p) = reporter_path() else { return };
    let allowed = spawn_sink("127.0.0.1:0".parse().unwrap());
    let other = spawn_sink("127.0.0.1:0".parse().unwrap());
    let rt = Runtime::new(Default::default()).unwrap();
    let c = WasmPart::compile_file(&rt, &p).unwrap();
    let g = WasiGrant::from_capabilities(
        &caps(serde_json::json!({
            "net": { "tcpConnect": [format!("127.0.0.1:{}", allowed.addr.port())] },
            "fs": [{ "path": "logs", "mode": "ro" }]
        })),
        &temp_dir("deny"),
    )
    .unwrap();
    let mut part = WasmPart::instantiate_with(&rt, &c, "rep", g).unwrap();
    // 宣言にないポートへ
    part.init(
        &serde_json::json!({ "host": "127.0.0.1", "port": other.addr.port(), "everyMs": 10 }),
    )
    .unwrap();
    let out = run_steps(&mut part, 2.0, 3);
    assert_eq!(out[0], 0.0, "送れてはいけない");
    assert_eq!(out[1], 3.0);
    assert_eq!(out[2], 1.0, "last-error = access denied");
    std::thread::sleep(Duration::from_millis(100));
    assert!(other.lines.lock().unwrap().is_empty());
    assert!(allowed.lines.lock().unwrap().is_empty());
    let denied = part.take_denied_destinations();
    assert_eq!(denied.len(), 3, "{denied:?}");
    assert!(denied[0].contains("TcpConnect"), "{denied:?}");
}

#[test]
fn name_lookup_is_limited_to_declared_names() {
    let Some(p) = reporter_path() else { return };
    // localhost が解決する先頭アドレスで待ち受ける (IPv4 / IPv6 どちらでもよい)
    let first = ("localhost", 0u16)
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();
    let sink = spawn_sink(first);
    let rt = Runtime::new(Default::default()).unwrap();
    let c = WasmPart::compile_file(&rt, &p).unwrap();
    let g = WasiGrant::from_capabilities(
        &caps(serde_json::json!({
            "net": { "tcpConnect": [format!("localhost:{}", sink.addr.port())] },
            "fs": [{ "path": "logs", "mode": "ro" }]
        })),
        &temp_dir("name"),
    )
    .unwrap();
    let mut part = WasmPart::instantiate_with(&rt, &c, "rep", g).unwrap();
    part.init(&serde_json::json!({ "host": "localhost", "port": sink.addr.port(), "everyMs": 10 }))
        .unwrap();
    let out = run_steps(&mut part, 3.0, 2);
    assert_eq!(
        out[..3],
        [2.0, 0.0, 0.0],
        "許可した名前は解決して接続できる"
    );
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(sink.lines.lock().unwrap().len(), 2);

    // 宣言にない名前は解決自体を拒否する
    let mut part2 = WasmPart::instantiate_with(
        &rt,
        &c,
        "rep2",
        WasiGrant::from_capabilities(
            &caps(serde_json::json!({
                "net": { "tcpConnect": [format!("localhost:{}", sink.addr.port())] },
                "fs": [{ "path": "logs", "mode": "ro" }]
            })),
            &temp_dir("name2"),
        )
        .unwrap(),
    )
    .unwrap();
    part2.init(&serde_json::json!({ "host": "other.invalid", "port": sink.addr.port(), "everyMs": 10 })).unwrap();
    let out = run_steps(&mut part2, 3.0, 1);
    assert_eq!(out[..3], [0.0, 1.0, 1.0]);
    assert!(part2.take_denied_destinations()[0].contains("resolve other.invalid"));
}

#[test]
fn file_access_is_confined_to_preopen() {
    let Some(p) = reporter_path() else { return };
    let sink = spawn_sink("127.0.0.1:0".parse().unwrap());
    let dir = temp_dir("fs");
    let rt = Runtime::new(Default::default()).unwrap();
    let c = WasmPart::compile_file(&rt, &p).unwrap();
    let make = |mode: &str, tag: &str| {
        let g = WasiGrant::from_capabilities(
            &caps(serde_json::json!({
                "net": { "tcpConnect": [format!("127.0.0.1:{}", sink.addr.port())] },
                "fs": [{ "path": "logs", "mode": mode }]
            })),
            &dir,
        )
        .unwrap();
        WasmPart::instantiate_with(&rt, &c, tag, g).unwrap()
    };

    // `..` で preopen の外へ出ようとする → ファイル失敗 (TCP は成功)
    let mut escape = make("rw", "escape");
    escape.init(&serde_json::json!({ "host": "127.0.0.1", "port": sink.addr.port(), "everyMs": 10, "file": "logs/../escape.txt" })).unwrap();
    let out = run_steps(&mut escape, 1.0, 2);
    assert_eq!(out, [2.0, 2.0, 4.0, 0.0]);
    assert!(!dir.join("escape.txt").exists());
    assert!(!dir.join("logs").join("escape.txt").exists());

    // 宣言にない preopen 名
    let mut nopre = make("rw", "nopre");
    nopre.init(&serde_json::json!({ "host": "127.0.0.1", "port": sink.addr.port(), "everyMs": 10, "file": "other/x.txt" })).unwrap();
    assert_eq!(run_steps(&mut nopre, 1.0, 1)[2], 4.0);

    // ro では書けない
    let mut ro = make("ro", "ro");
    ro.init(&serde_json::json!({ "host": "127.0.0.1", "port": sink.addr.port(), "everyMs": 10, "file": "logs/ro.txt" })).unwrap();
    assert_eq!(run_steps(&mut ro, 1.0, 1)[2], 4.0);
    assert!(!dir.join("logs").join("ro.txt").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clock_grant_marks_part_nondeterministic() {
    let Some(p) = reporter_path() else { return };
    let rt = Runtime::new(Default::default()).unwrap();
    let c = WasmPart::compile_file(&rt, &p).unwrap();
    let g = WasiGrant::from_capabilities(
        &caps(serde_json::json!({
            "net": { "tcpConnect": ["127.0.0.1:1"] },
            "fs": [{ "path": "logs", "mode": "ro" }],
            "clock": true
        })),
        &temp_dir("clock"),
    )
    .unwrap();
    let part = WasmPart::instantiate_with(&rt, &c, "rep", g).unwrap();
    assert!(!part.describe().deterministic);
}

#[test]
fn describe_sandboxed_reads_describe_without_manifest_and_without_io() {
    let Some(p) = reporter_path() else { return };
    let rt = Runtime::new(Default::default()).unwrap();
    let bytes = std::fs::read(&p).unwrap();
    let (desc, imports) = WasmPart::describe_component_sandboxed(&rt, &bytes).unwrap();
    assert_eq!(desc.name, "tcp-reporter");
    assert!(
        desc.deterministic,
        "clock / random を import しないので決定性は保たれる"
    );
    assert!(imports
        .iter()
        .any(|i| i.name.starts_with("wasi:sockets/tcp@")));
    // 通常の describe_component (capability なし) では読めない
    let c = WasmPart::compile(&rt, &bytes).unwrap();
    assert!(WasmPart::describe_component(&rt, &c).is_err());
    // WASI を使わない部品でも同じ API で読める
    let enc = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../build/components/encoder.wasm");
    let (d2, i2) =
        WasmPart::describe_component_sandboxed(&rt, &std::fs::read(enc).unwrap()).unwrap();
    assert_eq!(d2.name, "encoder");
    assert!(i2
        .iter()
        .all(|i| i.types_only || i.name.starts_with("ip:emu/")));
}
