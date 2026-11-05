//! IPC サーバの結合テスト: 接続 → describe → run → state 受信 → wasm ホット差替で状態が保持される、など。
//!
//! 前提は closed_loop.rs と同じ (build/ に成果物があること。無ければ skip)。

use ipemu_engine::control::{self, ControllerConfig};
use ipemu_engine::ipc_server::client::{Client, Incoming};
use ipemu_engine::ipc_server::{self, ServerConfig};
use ipemu_ipc::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn artifacts_ready() -> bool {
    let need = [
        "build/components/plant-cartpole.wasm",
        "build/firmware/fw-balance-c.wasm",
        "build/firmware/fw-balance-rs.wasm",
    ];
    let missing: Vec<_> = need.iter().filter(|p| !root().join(p).exists()).collect();
    if !missing.is_empty() {
        eprintln!("SKIP: ビルド成果物がありません ({missing:?})");
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

struct Harness {
    server: ipc_server::Server,
    ctrl: control::ControllerHandle,
}

async fn start_server() -> Harness {
    let control::Spawned {
        handle,
        state,
        outbound,
        join: _join,
    } = control::spawn(ControllerConfig {
        catalog_dirs: vec![root().join("dev")],
        ..Default::default()
    })
    .unwrap();
    let server = ipc_server::start(
        ServerConfig {
            port: 0,
            token: "test-token".into(),
            state_hz: 100.0,
        },
        handle.clone(),
        state,
        outbound,
        None,
    )
    .await
    .unwrap();
    Harness {
        server,
        ctrl: handle,
    }
}

fn local_launch(pacing: PacingMode, auto_start: bool) -> LaunchParams {
    LaunchParams {
        source: LaunchSource::Local {
            manifest_path: root()
                .join("dev/cartpole.manifest.json")
                .display()
                .to_string(),
            allow_unhashed: true,
        },
        firmware: BTreeMap::new(),
        params: Vec::new(),
        pacing: Some(PacingConfig {
            mode: pacing,
            speed: 1.0,
        }),
        auto_start,
    }
}

async fn next_state(c: &mut Client, timeout: Duration) -> Option<StateFrame> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        match tokio::time::timeout_at(deadline, c.incoming.recv()).await {
            Ok(Some(Incoming::State(f))) => return Some(f),
            Ok(Some(_)) => continue,
            _ => return None,
        }
    }
}

/// 仮想時刻が `t_ns` 以上の state フレームが来るまで待つ (古いフレームは読み飛ばす)。
async fn state_at_least(c: &mut Client, t_ns: i64, timeout: Duration) -> Option<StateFrame> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        match tokio::time::timeout_at(deadline, c.incoming.recv()).await {
            Ok(Some(Incoming::State(f))) if f.t_ns >= t_ns => return Some(f),
            Ok(Some(_)) => continue,
            _ => return None,
        }
    }
}

fn wrap(a: f64) -> f64 {
    let mut a = (a + std::f64::consts::PI) % (2.0 * std::f64::consts::PI);
    if a < 0.0 {
        a += 2.0 * std::f64::consts::PI;
    }
    a - std::f64::consts::PI
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejects_bad_token_and_accepts_good_one() {
    let h = start_server().await;
    let port = h.server.addr.port();
    assert!(Client::connect(port, "wrong").await.is_err());
    let c = Client::connect(port, "test-token").await.unwrap();
    let d: DescribeResult = c
        .call_as(RequestBody::SessionDescribe(Empty {}))
        .await
        .unwrap();
    assert!(d.session.is_none());
    assert_eq!(d.engine.ipc_version, IPC_VERSION);
    // セッションなしの操作は NO_SESSION
    let e = c.call(RequestBody::RunStart(Empty {})).await.unwrap_err();
    assert_eq!(e.code, codes::NO_SESSION);
    // 未知メソッド / 構文エラーの応答
    h.server.abort();
    h.ctrl.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn catalog_local_lists_dev_manifest() {
    let h = start_server().await;
    let c = Client::connect(h.server.addr.port(), "test-token")
        .await
        .unwrap();
    let cat: CatalogLocalResult = c
        .call_as(RequestBody::CatalogLocal(Empty {}))
        .await
        .unwrap();
    assert!(cat.entries.iter().any(|e| e.id == "dev-cartpole"));
    h.server.abort();
    h.ctrl.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn launch_run_and_receive_state_then_hot_swap_firmware_keeps_state() {
    if !artifacts_ready() {
        return;
    }
    let h = start_server().await;
    let mut c = Client::connect(h.server.addr.port(), "test-token")
        .await
        .unwrap();

    // 1. launch (fast, 自動開始なし)
    let d: SessionDescription = c
        .call_as(RequestBody::SessionLaunch(local_launch(
            PacingMode::Fast,
            false,
        )))
        .await
        .unwrap();
    assert_eq!(d.manifest.id, "dev-cartpole");
    assert_eq!(d.run.state, RunState::Idle);
    assert!(d.parts.iter().any(|p| p.kind == "builtin:cpu-virtual"));
    let mcu = d.parts.iter().find(|p| p.instance == "mcu").unwrap();
    let fw = mcu.firmware.as_ref().unwrap();
    assert_eq!(fw.slot, "balance");
    assert_eq!(fw.flavor.as_deref(), Some("wasm"));
    assert_eq!(fw.source, "default");
    assert!(d.ui.is_some(), "dev manifest は ui.bundle を持つ");
    let theta_i = d
        .snapshot_ports
        .iter()
        .position(|p| p == "plant.theta")
        .unwrap();
    let x_i = d
        .snapshot_ports
        .iter()
        .position(|p| p == "plant.x")
        .unwrap();
    let en_i = d
        .snapshot_ports
        .iter()
        .position(|p| p == "driver.drive-enabled")
        .unwrap();

    // 2. run.start → state フレームが届き、仮想時刻が進む
    let st: RunStatus = c.call_as(RequestBody::RunStart(Empty {})).await.unwrap();
    assert_eq!(st.state, RunState::Running);
    let f1 = next_state(&mut c, Duration::from_secs(5))
        .await
        .expect("state frame");
    assert_eq!(f1.values.len(), d.snapshot_ports.len());
    // 仮想 7 秒 (倒立領域に入るまで) 回す
    let mut f = f1;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while f.t_ns < 7_000_000_000 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "fast で 7 s 進むのが遅すぎる"
        );
        f = next_state(&mut c, Duration::from_secs(5))
            .await
            .expect("state frame");
    }
    assert_eq!(f.state, RunState::Running);
    assert_eq!(f.pacing, PacingMode::Fast);
    assert!(
        f.stats.realtime_ratio > 1.0,
        "fast は実時間より速い: {:?}",
        f.stats
    );
    let th = wrap(f.values[theta_i]);
    assert!(th.abs() < 0.3, "7 s で倒立しているはず: theta={th}");

    // 3. 停止 → ホット差替 (C → Rust)。t_ns とプラント状態がそのまま
    let _: RunStatus = c.call_as(RequestBody::RunStop(Empty {})).await.unwrap();
    let before: DescribeResult = c
        .call_as(RequestBody::SessionDescribe(Empty {}))
        .await
        .unwrap();
    let before = before.session.unwrap();
    let t_before = before.run.t_ns;
    let frame_before = state_at_least(&mut c, t_before, Duration::from_secs(2))
        .await
        .expect("state after stop");
    let x_before = frame_before.values[x_i];
    let th_before = frame_before.values[theta_i];
    assert_eq!(frame_before.t_ns, t_before);

    let r: FirmwareLoadResult = c
        .call_as(RequestBody::FirmwareLoad(FirmwareLoadParams {
            slot: "balance".into(),
            source: format!(
                "wasm:{}",
                root().join("build/firmware/fw-balance-rs.wasm").display()
            ),
        }))
        .await
        .unwrap();
    assert!(r.hot && !r.restart_required, "{r:?}");
    assert_eq!(r.flavor.as_deref(), Some("wasm"));
    assert!(r
        .overrides
        .firmware
        .get("balance")
        .unwrap()
        .starts_with("wasm:"));

    let after: DescribeResult = c
        .call_as(RequestBody::SessionDescribe(Empty {}))
        .await
        .unwrap();
    let after = after.session.unwrap();
    assert_eq!(after.run.t_ns, t_before, "ホット差替で仮想時刻は戻らない");
    assert_eq!(after.run.epoch, before.run.epoch, "Epoch も進まない");
    let fw = after
        .parts
        .iter()
        .find(|p| p.instance == "mcu")
        .unwrap()
        .firmware
        .as_ref()
        .unwrap();
    assert!(fw.source.ends_with("fw-balance-rs.wasm"));

    // 1 ステップ進めてもプラントは連続 (初期状態 theta=π に戻っていない)
    let st: RunStatus = c
        .call_as(RequestBody::RunStep(StepParams { n: 1 }))
        .await
        .unwrap();
    assert_eq!(st.t_ns, t_before + after.manifest.base_step_ns);
    let f = state_at_least(&mut c, st.t_ns, Duration::from_secs(2))
        .await
        .expect("state after step");
    assert_eq!(f.t_ns, st.t_ns);
    assert!(
        (f.values[x_i] - x_before).abs() < 1e-3,
        "x が飛んだ: {} → {}",
        x_before,
        f.values[x_i]
    );
    assert!(
        (f.values[theta_i] - th_before).abs() < 1e-2,
        "theta が飛んだ: {} → {}",
        th_before,
        f.values[theta_i]
    );

    // 4. 新しい FW が制御を引き継ぐ: 2 秒回しても倒立のまま
    let st: RunStatus = c
        .call_as(RequestBody::RunStep(StepParams { n: 2000 }))
        .await
        .unwrap();
    let f = state_at_least(&mut c, st.t_ns, Duration::from_secs(5))
        .await
        .expect("state after 2 s");
    assert!(f.t_ns >= t_before + 2_000_000_000);
    let th = wrap(f.values[theta_i]);
    assert!(th.abs() < 0.3, "差替後も倒立を維持: theta={th}");
    assert_eq!(f.values[en_i], 1.0, "ドライブ有効のまま");

    // 5. reset → Epoch が進み、時刻 0 に戻る
    let st: RunStatus = c.call_as(RequestBody::RunReset(Empty {})).await.unwrap();
    assert_eq!(st.t_ns, 0);
    assert_eq!(st.epoch, before.run.epoch + 1);
    assert_eq!(st.state, RunState::Idle);

    h.server.abort();
    h.ctrl.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_firmware_requires_restart_and_param_set_reinitializes() {
    if !artifacts_ready() {
        return;
    }
    let native = root().join(format!("build/firmware/fw-balance-c.{}", native_ext()));
    let h = start_server().await;
    let c = Client::connect(h.server.addr.port(), "test-token")
        .await
        .unwrap();
    let _: SessionDescription = c
        .call_as(RequestBody::SessionLaunch(local_launch(
            PacingMode::Step,
            false,
        )))
        .await
        .unwrap();

    // native → restartRequired、上書きが記録される
    let r: FirmwareLoadResult = c
        .call_as(RequestBody::FirmwareLoad(FirmwareLoadParams {
            slot: "balance".into(),
            source: format!("native:{}", native.display()),
        }))
        .await
        .unwrap();
    assert!(r.restart_required && !r.hot, "{r:?}");
    assert_eq!(
        r.overrides.firmware.get("balance").map(String::as_str),
        Some(format!("native:{}", native.display()).as_str())
    );
    // 存在しない slot
    let e = c
        .call(RequestBody::FirmwareLoad(FirmwareLoadParams {
            slot: "nope".into(),
            source: "default".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(e.code, codes::ENGINE_ERROR);

    // param.set: wasm 部品は init し直し、describe に反映
    let r: ParamSetResult = c
        .call_as(RequestBody::ParamSet(ParamOverride {
            instance: "plant".into(),
            key: "pendulumLength".into(),
            value: serde_json::json!(0.5),
        }))
        .await
        .unwrap();
    assert!(r.reinitialized && !r.restart_required);
    assert_eq!(r.params["pendulumLength"], 0.5);
    let d: DescribeResult = c
        .call_as(RequestBody::SessionDescribe(Empty {}))
        .await
        .unwrap();
    let d = d.session.unwrap();
    assert_eq!(
        d.parts
            .iter()
            .find(|p| p.instance == "plant")
            .unwrap()
            .params["pendulumLength"],
        0.5
    );
    assert!(d
        .overrides
        .params
        .iter()
        .any(|o| o.instance == "plant" && o.key == "pendulumLength"));
    // 組み込み部品は再起動要
    let r: ParamSetResult = c
        .call_as(RequestBody::ParamSet(ParamOverride {
            instance: "mcu".into(),
            key: "tickHz".into(),
            value: serde_json::json!(2000),
        }))
        .await
        .unwrap();
    assert!(r.restart_required);

    // 1 ステップ = baseStep、決定的
    let s1: RunStatus = c
        .call_as(RequestBody::RunStep(StepParams { n: 10 }))
        .await
        .unwrap();
    assert_eq!(s1.t_ns, 10 * d.manifest.base_step_ns);
    assert_eq!(s1.tick, 10);
    h.server.abort();
    h.ctrl.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scope_subscription_streams_chunks_and_realtime_paces() {
    if !artifacts_ready() {
        return;
    }
    let h = start_server().await;
    let mut c = Client::connect(h.server.addr.port(), "test-token")
        .await
        .unwrap();
    let d: SessionDescription = c
        .call_as(RequestBody::SessionLaunch(local_launch(
            PacingMode::Realtime,
            true,
        )))
        .await
        .unwrap();
    assert_eq!(d.run.state, RunState::Running);
    let sub: ScopeSubscribeResult = c
        .call_as(RequestBody::ScopeSubscribe(ScopeSubscribeParams {
            ports: vec!["plant.theta".into(), "mcu.pwm0".into()],
            rate_hz: 1000.0,
            window_s: 0.05,
            subscription_id: None,
        }))
        .await
        .unwrap();
    assert_eq!(sub.period_ns, 1_000_000);
    // 未知ポートは NOT_FOUND
    let e = c
        .call(RequestBody::ScopeSubscribe(ScopeSubscribeParams {
            ports: vec!["plant.nope".into()],
            rate_hz: 1000.0,
            window_s: 0.05,
            subscription_id: None,
        }))
        .await
        .unwrap_err();
    assert_eq!(e.code, codes::NOT_FOUND);

    let wall = std::time::Instant::now();
    let mut frames = 0;
    let mut last_state: Option<StateFrame> = None;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while frames < 5 {
        match tokio::time::timeout_at(deadline, c.incoming.recv()).await {
            Ok(Some(Incoming::Scope(f))) => {
                assert_eq!(f.subscription_id, sub.subscription_id);
                assert_eq!(f.channels.len(), 2);
                assert_eq!(f.channels[0].len(), 50, "window 50ms @1kHz");
                assert_eq!(f.period_ns, 1_000_000);
                frames += 1;
            }
            Ok(Some(Incoming::State(s))) => last_state = Some(s),
            Ok(Some(_)) => {}
            _ => panic!("scope フレームが届かない"),
        }
    }
    // realtime: 5 フレーム = 仮想 250ms は、壁時計でもおおむね 250ms 以上かかる
    assert!(
        wall.elapsed() >= Duration::from_millis(200),
        "realtime なのに速すぎる: {:?}",
        wall.elapsed()
    );
    let s = last_state.expect("state");
    assert_eq!(s.pacing, PacingMode::Realtime);

    // 購読解除
    let r: ScopeSubscribeResult = c
        .call_as(RequestBody::ScopeSubscribe(ScopeSubscribeParams {
            ports: vec![],
            rate_hz: 0.0,
            window_s: 0.0,
            subscription_id: Some(sub.subscription_id),
        }))
        .await
        .unwrap();
    assert!(r.ports.is_empty());

    // 倍速に変更すると ratio が上がる
    let st: RunStatus = c
        .call_as(RequestBody::RunSetPacing(PacingConfig {
            mode: PacingMode::Realtime,
            speed: 4.0,
        }))
        .await
        .unwrap();
    assert_eq!(st.pacing.speed, 4.0);
    h.server.abort();
    h.ctrl.shutdown();
}

/// 実バイナリ: `serve` が stdout 1 行目に接続情報を出し、stdin を閉じると終了する。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serve_binary_prints_discovery_and_exits_on_stdin_close() {
    if !artifacts_ready() {
        return;
    }
    use tokio::io::{AsyncBufReadExt, BufReader};
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_ipemu-engine"))
        .args([
            "serve",
            "--manifest",
            root().join("dev/cartpole.manifest.json").to_str().unwrap(),
            "--no-signature-check",
            "--pacing",
            "fast",
            "--auto-start",
            "--exit-on-stdin-close",
        ])
        .env("IPEMU_IPC_TOKEN", "bin-token")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let line = tokio::time::timeout(Duration::from_secs(30), lines.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let disc: Discovery = serde_json::from_str(&line).unwrap();
    assert_eq!(disc.token, "bin-token");
    assert!(disc.port > 0);

    let mut c = Client::connect(disc.port, &disc.token).await.unwrap();
    // launch はバックグラウンドで進むので、セッションが見えるまで待つ
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        let d: DescribeResult = c
            .call_as(RequestBody::SessionDescribe(Empty {}))
            .await
            .unwrap();
        if let Some(s) = d.session {
            assert_eq!(s.manifest.id, "dev-cartpole");
            break;
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let f = next_state(&mut c, Duration::from_secs(10))
        .await
        .expect("state");
    assert_eq!(f.state, RunState::Running);

    drop(child.stdin.take());
    let status = tokio::time::timeout(Duration::from_secs(10), child.wait())
        .await
        .expect("stdin を閉じたら終了する")
        .unwrap();
    assert!(status.success() || status.code().is_some());
}
