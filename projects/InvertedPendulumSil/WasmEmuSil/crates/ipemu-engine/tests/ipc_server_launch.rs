//! IPC の `session.launch {source: {kind: "server"}}` が M3 の bootstrap を通って動くことの結合テスト。
//! サーバを in-process で立て (tests/bootstrap.rs と同じ段取り)、API キーで取得 → 署名検証 → キャッシュ →
//! describe に `source: server` と UI バンドル (キャッシュ内のパス) が載る → オフライン (credential なし) でも起動できる。
//!
//! 前提: build/components と build/firmware の wasm。UI バンドル (build/ui/ui-cartpole.tar.gz) は無ければ外す。

use ipemu_engine::control::{self, ControllerConfig};
use ipemu_engine::ipc_server::client::Client;
use ipemu_engine::ipc_server::{self, ServerConfig};
use ipemu_ipc::*;
use ipemu_server::config::Config;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
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
        eprintln!("SKIP: ビルド成果物がありません ({missing:?})");
        return false;
    }
    true
}

/// 別スレッドの tokio ランタイムで動くサーバ (tests/bootstrap.rs と同じ)。
struct TestServer {
    base_url: String,
    key_id: String,
    public_key: String,
    stop: Option<std::sync::mpsc::Sender<()>>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl TestServer {
    fn start(dir: &Path) -> TestServer {
        let cfg = Config::for_test(dir);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
        let join = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let app = ipemu_server::App::start(cfg).await.unwrap();
                ready_tx
                    .send((
                        app.base_url.clone(),
                        app.state.key_id.clone(),
                        app.state.public_key_entry().public_key,
                    ))
                    .unwrap();
                let _ = tokio::task::spawn_blocking(move || stop_rx.recv()).await;
                app.stop().await;
            });
        });
        let (base_url, key_id, public_key) = ready_rx.recv().unwrap();
        TestServer {
            base_url,
            key_id,
            public_key,
            stop: Some(stop_tx),
            join: Some(join),
        }
    }
    fn stop(&mut self) {
        if let Some(s) = self.stop.take() {
            let _ = s.send(());
        }
        if let Some(j) = self.join.take() {
            j.join().unwrap();
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 部品 / FW (/ UI) を登録して dev manifest を `cartpole` として公開し、API キーを返す (同期 reqwest)。
fn publish_cartpole(base: &str, with_ui: bool) -> String {
    let http = reqwest::blocking::Client::new();
    let login = |user: &str, pass: &str| -> String {
        let v: Value = http
            .post(format!("{base}/api/auth/login"))
            .json(&json!({"username": user, "password": pass}))
            .send()
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .unwrap();
        v["token"].as_str().unwrap().to_owned()
    };
    let post = |tok: &str, path: &str, body: Value| -> Value {
        let r = http
            .post(format!("{base}{path}"))
            .bearer_auth(tok)
            .json(&body)
            .send()
            .unwrap();
        let st = r.status();
        let v: Value = r.json().unwrap_or(Value::Null);
        assert!(st.is_success(), "POST {path}: {st} {v}");
        v
    };
    let upload = |tok: &str, name: &str, kind: &str, path: &Path| {
        let form = reqwest::blocking::multipart::Form::new()
            .text("name", name.to_owned())
            .text("version", "0.1.0")
            .text("kind", kind.to_owned())
            .file("file", path)
            .unwrap();
        let r = http
            .post(format!("{base}/api/admin/artifacts"))
            .bearer_auth(tok)
            .multipart(form)
            .send()
            .unwrap();
        assert!(
            r.status().is_success(),
            "upload {name}: {}",
            r.text().unwrap_or_default()
        );
    };
    let admin = login("admin", "admin-password");
    post(
        &admin,
        "/api/admin/users",
        json!({"username": "approver", "password": "password-1234", "roles": ["approver"]}),
    );
    let ap = login("approver", "password-1234");
    for c in ["plant-cartpole", "motor-driver", "encoder", "comm-link"] {
        upload(
            &admin,
            c,
            "part",
            &root().join(format!("build/components/{c}.wasm")),
        );
    }
    for f in ["fw-balance-c", "fw-balance-rs"] {
        upload(
            &admin,
            f,
            "firmware",
            &root().join(format!("build/firmware/{f}.wasm")),
        );
    }
    if with_ui {
        upload(
            &admin,
            "ui-cartpole",
            "ui",
            &root().join("build/ui/ui-cartpole.tar.gz"),
        );
    }
    let mut body: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("dev/cartpole.manifest.json")).unwrap(),
    )
    .unwrap();
    body["id"] = json!("cartpole");
    if !with_ui {
        body.as_object_mut().unwrap().remove("ui");
        body["artifacts"]
            .as_object_mut()
            .unwrap()
            .remove("ui-cartpole@0.1.0");
    }
    for (_, art) in body["artifacts"].as_object_mut().unwrap().iter_mut() {
        art.as_object_mut().unwrap().remove("path");
        art["url"] = json!("placeholder");
    }
    post(
        &admin,
        "/api/admin/manifests",
        json!({"id": "cartpole", "body": body}),
    );
    post(
        &ap,
        "/api/admin/manifests/cartpole/revisions/1/approve",
        json!({"reason": "test"}),
    );
    let r = post(
        &ap,
        "/api/admin/manifests/cartpole/revisions/1/publish",
        json!({}),
    );
    assert_eq!(r["state"], json!("published"));
    let k = post(
        &admin,
        "/api/admin/api-keys",
        json!({"name": "engine-test", "allowedManifests": ["cartpole"]}),
    );
    k["key"].as_str().unwrap().to_owned()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_launch_from_server_bootstraps_and_offline_relaunch_works() {
    if !artifacts_ready() {
        return;
    }
    let with_ui = root().join("build/ui/ui-cartpole.tar.gz").exists();
    let dir = tempfile::tempdir().unwrap();
    // サーバは同期 API (reqwest::blocking) で用意するので、別スレッドで
    let (server, key) = tokio::task::spawn_blocking({
        let d = dir.path().join("server");
        move || {
            let s = TestServer::start(&d);
            let k = publish_cartpole(&s.base_url, with_ui);
            (s, k)
        }
    })
    .await
    .unwrap();

    let mut trusted = ipemu_sign::TrustedKeys::new();
    trusted
        .insert_hex(&server.key_id, &server.public_key)
        .unwrap();
    let spawned = control::spawn(ControllerConfig {
        cache_dir: Some(dir.path().join("cache")),
        trusted_keys: trusted,
        telemetry: true,
        ..Default::default()
    })
    .unwrap();
    let ipc = ipc_server::start(
        ServerConfig {
            port: 0,
            token: "t".into(),
            state_hz: 20.0,
            allowed_origins: vec!["app://ui".into()],
        },
        spawned.handle.clone(),
        spawned.state,
        spawned.outbound,
        None,
    )
    .await
    .unwrap();
    let c = Client::connect(ipc.addr.port(), "t").await.unwrap();

    // 資格情報なし (オフラインでもない) は INVALID_PARAMS
    let e = c
        .call(RequestBody::SessionLaunch(LaunchParams {
            source: LaunchSource::Server {
                server: server.base_url.clone(),
                manifest_id: "cartpole".into(),
                revision: None,
                credential: None,
                offline: false,
            },
            firmware: BTreeMap::new(),
            params: Vec::new(),
            pacing: None,
            auto_start: false,
        }))
        .await
        .unwrap_err();
    assert_eq!(e.code, codes::INVALID_PARAMS);

    // 1. API キーでサーバから取得して組み立てる
    let launch = LaunchParams {
        source: LaunchSource::Server {
            server: server.base_url.clone(),
            manifest_id: "cartpole".into(),
            revision: None,
            credential: Some(Credential::ApiKey { key: key.clone() }),
            offline: false,
        },
        firmware: BTreeMap::new(),
        params: Vec::new(),
        pacing: Some(PacingConfig {
            mode: PacingMode::Fast,
            speed: 1.0,
        }),
        auto_start: false,
    };
    let d: SessionDescription = c.call_as(RequestBody::SessionLaunch(launch)).await.unwrap();
    assert_eq!(d.manifest.id, "cartpole");
    assert_eq!(d.manifest.revision, 1);
    assert!(matches!(d.manifest.source, LaunchSource::Server { .. }));
    assert_eq!(d.credential, CredentialKind::ApiKey);
    assert!(!d.offline);
    if with_ui {
        let ui = d.ui.expect("ui.bundle がキャッシュのパスで載る");
        assert!(ui.path.contains("sha256"), "{}", ui.path);
        assert!(Path::new(&ui.path).is_file());
    }
    // 回る (倒立領域まで)
    let st: RunStatus = c
        .call_as(RequestBody::RunStep(StepParams { n: 7000 }))
        .await
        .unwrap();
    assert_eq!(st.tick, 7000);
    let th_i = d
        .snapshot_ports
        .iter()
        .position(|p| p == "plant.theta")
        .unwrap();
    // state フレームを待たずに describe の値で確認するため、1 回だけ出力を読む
    let mut client2 = Client::connect(ipc.addr.port(), "t").await.unwrap();
    let _: RunStatus = c
        .call_as(RequestBody::RunStep(StepParams { n: 1 }))
        .await
        .unwrap();
    let f = loop {
        match tokio::time::timeout(std::time::Duration::from_secs(5), client2.incoming.recv()).await
        {
            Ok(Some(ipemu_engine::ipc_server::client::Incoming::State(f))) if f.tick >= 7001 => {
                break f
            }
            Ok(Some(_)) => {}
            _ => panic!("state が届かない"),
        }
    };
    let th = ip_model::math::wrap_angle(f.values[th_i]);
    assert!(th.abs() < 0.3, "7 s で倒立: theta={th}");

    // manifest.refresh: 変化なし
    let r: ManifestRefreshResult = c
        .call_as(RequestBody::ManifestRefresh(Empty {}))
        .await
        .unwrap();
    assert!(!r.changed);
    assert_eq!(r.revision, 1);

    // 2. catalog.local にキャッシュ済み構成が載る (offline: true の source)
    let cat: CatalogLocalResult = c
        .call_as(RequestBody::CatalogLocal(Empty {}))
        .await
        .unwrap();
    let cached = cat
        .entries
        .iter()
        .find(|e| e.id == "cartpole" && e.server.is_some())
        .expect("キャッシュ済み構成");
    assert!(matches!(
        cached.source,
        LaunchSource::Server {
            offline: true,
            credential: None,
            ..
        }
    ));

    // 3. サーバを止め、資格情報なしのオフライン起動 (キャッシュ構成) → offline: true
    let mut server = server;
    tokio::task::spawn_blocking(move || server.stop())
        .await
        .unwrap();
    let d2: SessionDescription = c
        .call_as(RequestBody::SessionLaunch(LaunchParams {
            source: cached.source.clone(),
            firmware: BTreeMap::new(),
            params: Vec::new(),
            pacing: None,
            auto_start: false,
        }))
        .await
        .unwrap();
    assert!(d2.offline, "サーバ停止中はキャッシュ構成で起動する");
    assert_eq!(d2.credential, CredentialKind::None);
    assert_eq!(d2.manifest.revision, 1);

    ipc.abort();
    spawned.handle.shutdown();
}
