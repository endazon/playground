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
            allowed_origins: vec!["app://ui".into()],
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

/// ブラウザからの接続 (`Origin` あり) は許可リストに一致しなければトークンが正しくても 403。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn origin_header_is_checked_for_browser_connections() {
    use ipemu_engine::ipc_server::client::probe_handshake;
    let h = start_server().await;
    let port = h.server.addr.port();
    // Origin 不一致 → 403 (トークンは正しい)
    assert_eq!(
        probe_handshake(port, "test-token", Some("https://evil.example")).await,
        Err(Some(403))
    );
    assert_eq!(
        probe_handshake(port, "test-token", Some("null")).await,
        Err(Some(403))
    );
    // 許可 origin (大文字小文字・末尾 / は無視) → 成功
    assert_eq!(
        probe_handshake(port, "test-token", Some("app://ui")).await,
        Ok(())
    );
    assert_eq!(
        probe_handshake(port, "test-token", Some("APP://UI/")).await,
        Ok(())
    );
    // Origin なし (CLI / シェル main) → トークンだけで成功
    assert_eq!(probe_handshake(port, "test-token", None).await, Ok(()));
    // トークン不一致は Origin が合っていても 401
    assert_eq!(
        probe_handshake(port, "wrong", Some("app://ui")).await,
        Err(Some(401))
    );
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

    // 壁時計は「最初のフレーム受信 → 5 枚目の受信」で測る (購読からの計測だと、最初のフレームが
    // 購読前に進んでいた区間を含み得て揺れる)。4 区間 = 仮想 200ms が、実時間なら 150ms〜1s に収まるはず
    let mut frames = 0;
    let mut first_at: Option<std::time::Instant> = None;
    let mut last_at: Option<std::time::Instant> = None;
    let mut last_state: Option<StateFrame> = None;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while frames < 5 {
        match tokio::time::timeout_at(deadline, c.incoming.recv()).await {
            Ok(Some(Incoming::Scope(f))) => {
                assert_eq!(f.subscription_id, sub.subscription_id);
                assert_eq!(f.channels.len(), 2);
                assert_eq!(f.channels[0].len(), 50, "window 50ms @1kHz");
                assert_eq!(f.period_ns, 1_000_000);
                let now = std::time::Instant::now();
                first_at.get_or_insert(now);
                last_at = Some(now);
                frames += 1;
            }
            Ok(Some(Incoming::State(s))) => last_state = Some(s),
            Ok(Some(_)) => {}
            _ => panic!("scope フレームが届かない"),
        }
    }
    let span = last_at.unwrap().duration_since(first_at.unwrap());
    assert!(
        span >= Duration::from_millis(150),
        "realtime なのに速すぎる (4 区間 = 仮想 200ms が {span:?})"
    );
    assert!(
        span <= Duration::from_secs(1),
        "realtime なのに遅すぎる (4 区間 = 仮想 200ms が {span:?})"
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

/// 同意の結合 (M2 ⇄ M5): 未同意の capability があると `consent.required` が流れ、`consent.answer` で起動が進む / 止まる。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn consent_required_flows_over_ipc_and_answers_gate_launch() {
    use ipemu_engine::consent_ipc::ConsentBroker;
    let tcp_manifest = root().join("dev/cartpole-tcp.manifest.json");
    if !artifacts_ready() || !root().join("build/components/tcp-reporter.wasm").exists() {
        eprintln!("SKIP: tcp-reporter.wasm がありません");
        return;
    }
    let data_dir = std::env::temp_dir().join(format!("ipemu-ipc-consent-{}", std::process::id()));
    let (out_tx, out_rx) = tokio::sync::mpsc::unbounded_channel();
    let broker = std::sync::Arc::new(ConsentBroker::new(out_tx.clone(), Duration::from_secs(30)));
    let spawned = control::spawn_with_outbound(
        ControllerConfig {
            data_dir: Some(data_dir.clone()),
            consent: Some(broker.clone()),
            no_consent_store: true,
            ..Default::default()
        },
        out_tx,
    )
    .unwrap();
    let server = ipc_server::start(
        ServerConfig {
            port: 0,
            token: "t".into(),
            state_hz: 20.0,
            allowed_origins: vec!["app://ui".into()],
        },
        spawned.handle.clone(),
        spawned.state,
        out_rx,
        Some(broker.clone()),
    )
    .await
    .unwrap();
    // 2 本の接続: 1 本は launch を待ち、もう 1 本 (シェルの同意ウィンドウ相当) が回答する
    let launcher = Client::connect(server.addr.port(), "t").await.unwrap();
    let mut answerer = Client::connect(server.addr.port(), "t").await.unwrap();
    let launch = LaunchParams {
        source: LaunchSource::Local {
            manifest_path: tcp_manifest.display().to_string(),
            allow_unhashed: true,
        },
        firmware: BTreeMap::new(),
        params: Vec::new(),
        pacing: Some(PacingConfig {
            mode: PacingMode::Step,
            speed: 1.0,
        }),
        auto_start: false,
    };

    // 問い合わせ中でない id への回答は NOT_FOUND
    let e = answerer
        .call(RequestBody::ConsentAnswer(ConsentAnswerParams {
            request_id: "nope".into(),
            grant: true,
            remember: false,
        }))
        .await
        .unwrap_err();
    assert_eq!(e.code, codes::NOT_FOUND);

    async fn collect_requests(c: &mut Client, n_min: usize) -> Vec<ConsentRequest> {
        let mut reqs = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        loop {
            match tokio::time::timeout_at(deadline, c.incoming.recv()).await {
                Ok(Some(Incoming::Notification(Notification {
                    body: NotificationBody::ConsentRequired(r),
                    ..
                }))) => {
                    reqs.push(r);
                    // 全件そろったかは broker の pending 数で判断する代わりに、少し待って追加がなければ打ち切る
                    if reqs.len() >= n_min {
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        while let Ok(Some(m)) =
                            tokio::time::timeout(Duration::from_millis(50), c.incoming.recv()).await
                        {
                            if let Incoming::Notification(Notification {
                                body: NotificationBody::ConsentRequired(r),
                                ..
                            }) = m
                            {
                                reqs.push(r);
                            }
                        }
                        return reqs;
                    }
                }
                Ok(Some(_)) => {}
                _ => panic!("consent.required が届かない (届いた {} 件)", reqs.len()),
            }
        }
    }

    // 1 回目: 1 件を拒否 → 起動しない
    let l = launcher.call(RequestBody::SessionLaunch(launch.clone()));
    let (reqs, res) = tokio::join!(
        async {
            let reqs = collect_requests(&mut answerer, 1).await;
            assert!(!reqs.is_empty());
            assert!(reqs
                .iter()
                .all(|r| !r.instances.is_empty() && r.artifact_sha256.len() == 64));
            for (i, r) in reqs.iter().enumerate() {
                answerer
                    .call(RequestBody::ConsentAnswer(ConsentAnswerParams {
                        request_id: r.request_id.clone(),
                        grant: i != 0,
                        remember: false,
                    }))
                    .await
                    .unwrap();
            }
            reqs
        },
        l
    );
    let e = res.unwrap_err();
    assert_eq!(e.code, codes::ENGINE_ERROR);
    assert!(e.message.contains("拒否"), "{}", e.message);
    let d: DescribeResult = launcher
        .call_as(RequestBody::SessionDescribe(Empty {}))
        .await
        .unwrap();
    assert!(d.session.is_none(), "拒否があれば部分起動しない");

    // 2 回目: 全件許可 → 起動し、describe に granted として載る
    let l = launcher.call(RequestBody::SessionLaunch(launch.clone()));
    let (_, res) = tokio::join!(
        async {
            let again = collect_requests(&mut answerer, reqs.len()).await;
            assert_eq!(
                again.len(),
                reqs.len(),
                "記憶しない回答なので同じ件数を再度聞く"
            );
            for r in &again {
                answerer
                    .call(RequestBody::ConsentAnswer(ConsentAnswerParams {
                        request_id: r.request_id.clone(),
                        grant: true,
                        remember: false,
                    }))
                    .await
                    .unwrap();
            }
        },
        l
    );
    let desc: SessionDescription = serde_json::from_value(res.unwrap()).unwrap();
    assert!(!desc.capabilities.is_empty());
    assert!(desc
        .capabilities
        .iter()
        .all(|c| c.consent == ConsentState::Granted));
    assert!(desc
        .capabilities
        .iter()
        .any(|c| c.capability.starts_with("net.tcpConnect")));

    server.abort();
    spawned.handle.shutdown();
    let _ = std::fs::remove_dir_all(&data_dir);
}

/// ホット差替 (firmware.load) は差し替え先の sha256 で同意を取り直す (別のコードに許可を引き継がない)。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hot_swap_asks_consent_for_the_new_build() {
    use ipemu_engine::consent_ipc::ConsentBroker;
    if !artifacts_ready() {
        return;
    }
    // dev manifest の mcu に `clock` を宣言した一時 manifest (path は絶対に)
    let text = std::fs::read_to_string(root().join("dev/cartpole.manifest.json")).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    for (_, a) in v["artifacts"].as_object_mut().unwrap() {
        let p = a["path"].as_str().unwrap().to_owned();
        let joined = root().join("dev").join(p);
        a["path"] = serde_json::json!(joined.canonicalize().unwrap_or(joined).to_string_lossy());
    }
    v["components"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["instance"] == "mcu")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert("capabilities".into(), serde_json::json!({ "clock": true }));
    let tmp_dir = std::env::temp_dir().join(format!("ipemu-ipc-swap-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let manifest_path = tmp_dir.join("clock.manifest.json");
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&v).unwrap()).unwrap();

    let (out_tx, out_rx) = tokio::sync::mpsc::unbounded_channel();
    let broker = std::sync::Arc::new(ConsentBroker::new(out_tx.clone(), Duration::from_secs(30)));
    let spawned = control::spawn_with_outbound(
        ControllerConfig {
            data_dir: Some(tmp_dir.join("data")),
            consent: Some(broker.clone()),
            no_consent_store: true,
            ..Default::default()
        },
        out_tx,
    )
    .unwrap();
    let server = ipc_server::start(
        ServerConfig {
            port: 0,
            token: "t".into(),
            state_hz: 20.0,
            allowed_origins: vec!["app://ui".into()],
        },
        spawned.handle.clone(),
        spawned.state,
        out_rx,
        Some(broker.clone()),
    )
    .await
    .unwrap();
    let launcher = Client::connect(server.addr.port(), "t").await.unwrap();
    let mut answerer = Client::connect(server.addr.port(), "t").await.unwrap();

    /// consent.required を 1 件受けて答える。返すのは requestId
    async fn answer_one(c: &mut Client, grant: bool) -> String {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        loop {
            match tokio::time::timeout_at(deadline, c.incoming.recv()).await {
                Ok(Some(Incoming::Notification(Notification {
                    body: NotificationBody::ConsentRequired(r),
                    ..
                }))) => {
                    assert_eq!(r.capability, "clock");
                    c.call(RequestBody::ConsentAnswer(ConsentAnswerParams {
                        request_id: r.request_id.clone(),
                        grant,
                        remember: false,
                    }))
                    .await
                    .unwrap();
                    return r.request_id;
                }
                Ok(Some(_)) => {}
                _ => panic!("consent.required が届かない"),
            }
        }
    }

    // launch: C 版 FW (既定) の sha256 で 1 件 → 許可
    let launch = LaunchParams {
        source: LaunchSource::Local {
            manifest_path: manifest_path.display().to_string(),
            allow_unhashed: true,
        },
        firmware: BTreeMap::new(),
        params: Vec::new(),
        pacing: Some(PacingConfig {
            mode: PacingMode::Step,
            speed: 1.0,
        }),
        auto_start: false,
    };
    let (key_c, res) = tokio::join!(
        answer_one(&mut answerer, true),
        launcher.call(RequestBody::SessionLaunch(launch))
    );
    let desc: SessionDescription = serde_json::from_value(res.unwrap()).unwrap();
    assert_eq!(desc.capabilities.len(), 1);
    let _: RunStatus = launcher
        .call_as(RequestBody::RunStep(StepParams { n: 500 }))
        .await
        .unwrap();

    // 差し替え (Rust 版 = 別の sha256): 新しい鍵で聞かれ、拒否すると古い FW のまま
    let rs = format!(
        "wasm:{}",
        root().join("build/firmware/fw-balance-rs.wasm").display()
    );
    let load = || {
        launcher.call(RequestBody::FirmwareLoad(FirmwareLoadParams {
            slot: "balance".into(),
            source: rs.clone(),
        }))
    };
    let (key_rs, res) = tokio::join!(answer_one(&mut answerer, false), load());
    assert_ne!(key_rs, key_c, "鍵は sha256 ごと");
    let e = res.unwrap_err();
    assert_eq!(e.code, codes::ENGINE_ERROR);
    assert!(e.message.contains("拒否"), "{}", e.message);
    let d: DescribeResult = launcher
        .call_as(RequestBody::SessionDescribe(Empty {}))
        .await
        .unwrap();
    let d = d.session.unwrap();
    let fw = d
        .parts
        .iter()
        .find(|p| p.instance == "mcu")
        .unwrap()
        .firmware
        .as_ref()
        .unwrap();
    assert_eq!(fw.source, "default", "拒否されたら差し替えない");
    assert_eq!(d.run.t_ns, 500_000_000, "状態は保持");

    // 許可すると差し替わる。もう一度同じビルドなら聞かれない
    let (key_again, res) = tokio::join!(answer_one(&mut answerer, true), load());
    assert_eq!(key_again, key_rs);
    let r: FirmwareLoadResult = serde_json::from_value(res.unwrap()).unwrap();
    assert!(r.hot);
    let r: FirmwareLoadResult = launcher
        .call_as(RequestBody::FirmwareLoad(FirmwareLoadParams {
            slot: "balance".into(),
            source: rs.clone(),
        }))
        .await
        .unwrap();
    assert!(r.hot, "同じ sha256 なら同意を聞き直さずに差し替わる");
    assert_eq!(broker.pending_count(), 0);

    server.abort();
    spawned.handle.shutdown();
    let _ = std::fs::remove_dir_all(&tmp_dir);
}
