//! 結合テスト (09-verification.md の L2 / L4 / L5 / L8)。
//!
//! 前提: `scripts/build-components.sh` と `scripts/build-firmware.sh` で build/ に成果物があること。
//! 無ければ理由を表示して skip する (CI を置かない方針なので、手元の check.sh が両方を順に回す)。

use ipemu_core::hal::LogLevel;
use ipemu_engine::{FirmwareSource, Session, SessionOptions, TraceHash};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn manifest() -> PathBuf {
    root().join("dev/cartpole.manifest.json")
}

fn artifacts_ready() -> bool {
    let need = [
        "build/components/plant-cartpole.wasm",
        "build/components/motor-driver.wasm",
        "build/components/encoder.wasm",
        "build/components/comm-link.wasm",
        "build/firmware/fw-balance-c.wasm",
        "build/firmware/fw-balance-rs.wasm",
    ];
    let missing: Vec<_> = need.iter().filter(|p| !root().join(p).exists()).collect();
    if !missing.is_empty() {
        eprintln!("SKIP: ビルド成果物がありません ({missing:?})。scripts/build-components.sh と scripts/build-firmware.sh を先に実行してください");
        return false;
    }
    true
}

fn native_ext() -> &'static str {
    if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    }
}

struct Run {
    hash: String,
    rows: Vec<(f64, Vec<f64>)>,
    columns: Vec<String>,
    logs: Vec<String>,
}

fn run(
    seconds: f64,
    firmware: Option<FirmwareSource>,
    params: Vec<(&str, &str, serde_json::Value)>,
) -> Run {
    let mut opts = SessionOptions {
        allow_unhashed: true,
        ..Default::default()
    };
    if let Some(f) = firmware {
        opts.firmware_overrides.insert("balance".into(), f);
    }
    for (i, k, v) in params {
        opts.param_overrides.push((i.into(), k.into(), v));
    }
    let mut s = Session::load(&manifest(), &opts).expect("session");
    let columns = s.snapshot_ports.clone();
    let mut hash = TraceHash::default();
    let mut rows = Vec::new();
    let mut logs = Vec::new();
    s.run_for(seconds, |t, v, l| {
        hash.push_row(t, v);
        rows.push((t as f64 * 1e-9, v.to_vec()));
        for (inst, lvl, msg) in l.drain(..) {
            if matches!(lvl, LogLevel::Info | LogLevel::Warn | LogLevel::Error) {
                logs.push(format!("{inst}: {msg}"));
            }
        }
    })
    .expect("run");
    Run {
        hash: hash.finish(),
        rows,
        columns,
        logs,
    }
}

fn col(r: &Run, name: &str) -> usize {
    r.columns
        .iter()
        .position(|c| c == name)
        .unwrap_or_else(|| panic!("列 {name} がありません"))
}

fn wrap(a: f64) -> f64 {
    ip_model::math::wrap_angle(a)
}

/// 最後の `tail_s` 秒で倒立を維持しているか (|θ| と |x| の最大値)。
fn assert_balanced(r: &Run, tail_s: f64) {
    let th = col(r, "plant.theta");
    let x = col(r, "plant.x");
    let en = col(r, "driver.drive-enabled");
    let t_end = r.rows.last().unwrap().0;
    let mut worst_th: f64 = 0.0;
    let mut worst_x: f64 = 0.0;
    for (t, v) in &r.rows {
        if *t >= t_end - tail_s {
            worst_th = worst_th.max(wrap(v[th]).abs());
            worst_x = worst_x.max(v[x].abs());
            assert_eq!(v[en], 1.0, "t={t}: ドライブが無効になっています");
        }
    }
    assert!(
        worst_th < 0.05,
        "倒立を維持していない: max|θ|={worst_th} rad"
    );
    assert!(worst_x < 0.15, "台車が流れている: max|x|={worst_x} m");
    assert!(
        r.logs.iter().any(|l| l.contains("SwingUp -> Balance")),
        "Balance へ遷移していない: {:?}",
        r.logs
    );
}

#[test]
fn c_firmware_wasm_swings_up_and_balances() {
    if !artifacts_ready() {
        return;
    }
    let r = run(20.0, None, vec![]);
    assert_balanced(&r, 5.0);
}

#[test]
fn rust_firmware_wasm_swings_up_and_balances() {
    if !artifacts_ready() {
        return;
    }
    let r = run(
        20.0,
        Some(FirmwareSource::Wasm(
            root().join("build/firmware/fw-balance-rs.wasm"),
        )),
        vec![],
    );
    assert_balanced(&r, 5.0);
}

#[test]
fn native_firmwares_swing_up_and_balance() {
    if !artifacts_ready() {
        return;
    }
    for name in ["fw-balance-c", "fw-balance-rs"] {
        let p = root().join(format!("build/firmware/{name}.{}", native_ext()));
        if !p.exists() {
            eprintln!("SKIP: {} がありません", p.display());
            continue;
        }
        let r = run(20.0, Some(FirmwareSource::Native(p)), vec![]);
        assert_balanced(&r, 5.0);
    }
}

#[test]
fn same_manifest_is_deterministic() {
    if !artifacts_ready() {
        return;
    }
    let a = run(8.0, None, vec![]);
    let b = run(8.0, None, vec![]);
    assert_eq!(a.hash, b.hash, "同じ manifest で軌道が一致しない");
}

#[test]
fn c_and_rust_firmware_produce_equivalent_trajectories() {
    if !artifacts_ready() {
        return;
    }
    let c = run(20.0, None, vec![]);
    let r = run(
        20.0,
        Some(FirmwareSource::Wasm(
            root().join("build/firmware/fw-balance-rs.wasm"),
        )),
        vec![],
    );
    let th = col(&c, "plant.theta");
    let x = col(&c, "plant.x");
    let mut worst_th: f64 = 0.0;
    let mut worst_x: f64 = 0.0;
    for (a, b) in c.rows.iter().zip(&r.rows) {
        worst_th = worst_th.max((wrap(a.1[th]) - wrap(b.1[th])).abs());
        worst_x = worst_x.max((a.1[x] - b.1[x]).abs());
    }
    // 同じ仕様の 2 実装。演算順の違いで最終ビットは一致しないことがあるが、軌道は一致する
    assert!(
        worst_th < 1e-3 && worst_x < 1e-3,
        "C 版と Rust 版の軌道差 Δθ={worst_th:e} Δx={worst_x:e}"
    );
    if c.hash == r.hash {
        eprintln!("C 版と Rust 版はビット一致");
    } else {
        eprintln!("C 版と Rust 版は許容誤差内 (Δθ={worst_th:e}, Δx={worst_x:e})");
    }
}

#[test]
fn balance_from_hanging_trips_fall_detection_and_driver_disables() {
    if !artifacts_ready() {
        return;
    }
    // 吊り下げ状態で LQR を直接始めると転倒検知 → FAULT → ドライブ無効 (enable 立ち下げ) になる
    let r = run(
        3.0,
        None,
        vec![("mcu", "firmware", serde_json::json!(null))],
    );
    // ↑ firmware は components[].firmware なので params 上書きでは届かない。代わりに FW params の autoStart を変える
    let _ = r;
    let mut opts = SessionOptions {
        allow_unhashed: true,
        ..Default::default()
    };
    opts.param_overrides.clear();
    let text = std::fs::read_to_string(manifest()).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mcu = v["components"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["instance"] == "mcu")
        .unwrap();
    mcu["firmware"]["params"]["autoStart"] = serde_json::json!("balance");
    let tmp = std::env::temp_dir().join(format!("ipemu-fault-{}.json", std::process::id()));
    // path は manifest 相対なので、絶対パスに書き換える
    for (_, a) in v["artifacts"].as_object_mut().unwrap() {
        let p = a["path"].as_str().unwrap().to_owned();
        a["path"] = serde_json::json!(root()
            .join("dev")
            .join(p)
            .canonicalize()
            .unwrap()
            .to_string_lossy());
    }
    std::fs::write(&tmp, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    let mut s = Session::load(&tmp, &opts).expect("session");
    let mut logs = Vec::new();
    let mut last = Vec::new();
    s.run_for(3.0, |_, vals, l| {
        last = vals.to_vec();
        logs.extend(l.drain(..).map(|(i, _, m)| format!("{i}: {m}")));
    })
    .unwrap();
    let _ = std::fs::remove_file(&tmp);
    let en = s
        .snapshot_ports
        .iter()
        .position(|c| c == "driver.drive-enabled")
        .unwrap();
    let volt = s
        .snapshot_ports
        .iter()
        .position(|c| c == "driver.voltage")
        .unwrap();
    assert!(logs.iter().any(|l| l.contains("転倒検知")), "{logs:?}");
    assert_eq!(last[en], 0.0, "FAULT 後はドライブが無効");
    assert_eq!(last[volt], 0.0);
}

/// ゴールデントレース: tests/golden/cartpole-c.csv と比較する。更新は IPEMU_UPDATE_GOLDEN=1。
#[test]
fn golden_trace_matches() {
    if !artifacts_ready() {
        return;
    }
    let path = root().join("tests/golden/cartpole-c.csv");
    let r = run(12.0, None, vec![]);
    let th = col(&r, "plant.theta");
    let x = col(&r, "plant.x");
    let u = col(&r, "driver.voltage");
    // 100ms ごとに θ, x, u を記録
    let mut out = String::from("t_s,theta,x,voltage\n");
    let mut sampled = Vec::new();
    for (i, (t, v)) in r.rows.iter().enumerate() {
        if i % 10 == 0 {
            out.push_str(&format!("{t:.3},{},{},{}\n", v[th], v[x], v[u]));
            sampled.push((*t, v[th], v[x], v[u]));
        }
    }
    if std::env::var("IPEMU_UPDATE_GOLDEN").is_ok() || !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &out).unwrap();
        eprintln!("ゴールデンを書き出しました: {}", path.display());
        return;
    }
    let golden = std::fs::read_to_string(&path).unwrap();
    if golden == out {
        return; // ビット一致
    }
    // 一致しなければ許容誤差で判定し、最大差を報告する
    let mut worst = (0.0f64, 0.0f64);
    for (line, s) in golden.lines().skip(1).zip(&sampled) {
        let f: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
        worst.0 = worst.0.max((wrap(f[1]) - wrap(s.1)).abs());
        worst.1 = worst.1.max((f[2] - s.2).abs());
    }
    panic!(
        "ゴールデンと不一致: max Δθ={:e} Δx={:e} (意図した変更なら IPEMU_UPDATE_GOLDEN=1 で更新)",
        worst.0, worst.1
    );
}
