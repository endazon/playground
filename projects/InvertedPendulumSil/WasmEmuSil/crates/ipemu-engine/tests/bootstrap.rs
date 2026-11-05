//! サーバ配信 → bootstrap → キャッシュ → Session::load → 倒立維持、の結合テスト (M3 の完了条件)。
//! サーバを in-process で立て、dev/cartpole.manifest.json の部品と FW を登録して配信し、
//! エンジンが `--server` 相当の経路でダウンロードして動くこと、サーバを落としてもキャッシュから起動できることを示す。
//!
//! 前提: build/components と build/firmware に wasm があること。無ければ skip。

use ipemu_core::hal::LogLevel;
use ipemu_engine::{bootstrap, BootstrapOptions, Cache, Credential, Session, SessionOptions};
use ipemu_server::config::Config;
use reqwest::blocking::Client;
use serde_json::{json, Value};
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

/// 別スレッドの tokio ランタイムで動くサーバ。テスト本体は同期のまま reqwest::blocking を使える。
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

struct Admin<'a> {
    http: Client,
    base: &'a str,
}

impl Admin<'_> {
    fn login(&self, user: &str, pass: &str) -> String {
        let v: Value = self
            .http
            .post(format!("{}/api/auth/login", self.base))
            .json(&json!({"username": user, "password": pass}))
            .send()
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .unwrap();
        v["token"].as_str().unwrap().to_owned()
    }
    fn post(&self, tok: &str, path: &str, body: Value) -> Value {
        let r = self
            .http
            .post(format!("{}{path}", self.base))
            .bearer_auth(tok)
            .json(&body)
            .send()
            .unwrap();
        let st = r.status();
        let v: Value = r.json().unwrap_or(Value::Null);
        assert!(st.is_success(), "POST {path}: {st} {v}");
        v
    }
    fn upload(&self, tok: &str, name: &str, version: &str, kind: &str, path: &Path) {
        let form = reqwest::blocking::multipart::Form::new()
            .text("name", name.to_owned())
            .text("version", version.to_owned())
            .text("kind", kind.to_owned())
            .file("file", path)
            .unwrap();
        let r = self
            .http
            .post(format!("{}/api/admin/artifacts", self.base))
            .bearer_auth(tok)
            .multipart(form)
            .send()
            .unwrap();
        assert!(
            r.status().is_success(),
            "upload {name}: {}",
            r.text().unwrap_or_default()
        );
    }
}

/// 部品と FW を登録し、dev/cartpole.manifest.json を `cartpole` として公開する。API キーを返す。
fn publish_cartpole(base: &str) -> String {
    let a = Admin {
        http: Client::new(),
        base,
    };
    let admin = a.login("admin", "admin-password");
    a.post(
        &admin,
        "/api/admin/users",
        json!({"username": "approver", "password": "password-1234", "roles": ["approver"]}),
    );
    let ap = a.login("approver", "password-1234");
    for c in ["plant-cartpole", "motor-driver", "encoder", "comm-link"] {
        a.upload(
            &admin,
            c,
            "0.1.0",
            "part",
            &root().join(format!("build/components/{c}.wasm")),
        );
    }
    for f in ["fw-balance-c", "fw-balance-rs"] {
        a.upload(
            &admin,
            f,
            "0.1.0",
            "firmware",
            &root().join(format!("build/firmware/{f}.wasm")),
        );
    }
    let mut body: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("dev/cartpole.manifest.json")).unwrap(),
    )
    .unwrap();
    body["id"] = json!("cartpole");
    // UI バンドル (M2 が dev manifest に足したもの) は pnpm build が要るので、このテストでは外す
    body.as_object_mut().unwrap().remove("ui");
    body["artifacts"]
        .as_object_mut()
        .unwrap()
        .remove("ui-cartpole@0.1.0");
    for (_, art) in body["artifacts"].as_object_mut().unwrap().iter_mut() {
        art.as_object_mut().unwrap().remove("path");
        art["url"] = json!("placeholder");
    }
    let m = a.post(
        &admin,
        "/api/admin/manifests",
        json!({"id": "cartpole", "body": body}),
    );
    assert_eq!(m["revisions"][0]["state"], json!("draft"));
    a.post(
        &ap,
        "/api/admin/manifests/cartpole/revisions/1/approve",
        json!({"reason": "test"}),
    );
    let r = a.post(
        &ap,
        "/api/admin/manifests/cartpole/revisions/1/publish",
        json!({}),
    );
    assert_eq!(r["state"], json!("published"));
    let k = a.post(
        &admin,
        "/api/admin/api-keys",
        json!({"name": "engine-test", "allowedManifests": ["cartpole"]}),
    );
    k["key"].as_str().unwrap().to_owned()
}

fn opts(server: &TestServer, cache: &Cache, key: &str) -> BootstrapOptions {
    let mut bo = BootstrapOptions::new(&server.base_url, "cartpole", cache.clone()).unwrap();
    bo.credential = Some(Credential::ApiKey(key.to_owned()));
    bo.trusted_keys
        .insert_hex(&server.key_id, &server.public_key)
        .unwrap();
    bo
}

/// 組み立てて `seconds` 回し、最後の 3 秒で倒立を維持していることを確かめる。
fn run_and_assert_balanced(session: &mut Session, seconds: f64) -> String {
    let columns = session.snapshot_ports.clone();
    let th = columns.iter().position(|c| c == "plant.theta").unwrap();
    let en = columns
        .iter()
        .position(|c| c == "driver.drive-enabled")
        .unwrap();
    let mut hash = ipemu_engine::TraceHash::default();
    let mut worst: f64 = 0.0;
    session
        .run_for(seconds, |t_ns, v, logs| {
            hash.push_row(t_ns, v);
            for (inst, lvl, msg) in logs.drain(..) {
                if matches!(lvl, LogLevel::Warn | LogLevel::Error) {
                    eprintln!("{inst}: {msg}");
                }
            }
            let t = t_ns as f64 * 1e-9;
            if t >= seconds - 3.0 {
                worst = worst.max(ip_model::math::wrap_angle(v[th]).abs());
                assert_eq!(v[en], 1.0, "t={t}: ドライブが無効");
            }
        })
        .unwrap();
    assert!(worst < 0.05, "倒立を維持できていません (|θ|max = {worst})");
    hash.finish()
}

#[test]
fn engine_downloads_from_server_and_balances_then_runs_offline_from_cache() {
    if !artifacts_ready() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut server = TestServer::start(&dir.path().join("server"));
    let key = publish_cartpole(&server.base_url);
    let cache = Cache::new(dir.path().join("cache")).unwrap();

    // 1 回目: すべてダウンロード
    let b = bootstrap(&opts(&server, &cache, &key)).unwrap();
    assert!(!b.from_cache);
    assert_eq!(b.revision, 1);
    assert_eq!(b.downloaded, 6);
    assert_eq!(b.cache_hits, 0);
    assert_eq!(b.verified_key_id.as_deref(), Some(server.key_id.as_str()));
    assert!(b.launch_id.is_some());
    assert!(b.manifest_path.starts_with(cache.root()));
    let so = b.session_options(&cache, SessionOptions::default());
    assert!(!so.allow_unhashed, "配信 manifest では sha256 を必須にする");
    let mut session = Session::load(&b.manifest_path, &so).unwrap();
    assert_eq!(session.manifest.id, "cartpole");
    let t = b.telemetry().expect("telemetry");
    t.send("started", json!({})).unwrap();
    let hash_online = run_and_assert_balanced(&mut session, 12.0);
    t.send("stopped", json!({"traceHash": hash_online}))
        .unwrap();

    // 起動履歴に API キー経由の起動と終了が残る
    let a = Admin {
        http: Client::new(),
        base: &server.base_url,
    };
    let admin = a.login("admin", "admin-password");
    let launches: Value = a
        .http
        .get(format!("{}/api/admin/launches", server.base_url))
        .bearer_auth(&admin)
        .send()
        .unwrap()
        .json()
        .unwrap();
    let l = launches["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["id"] == json!(b.launch_id.clone().unwrap()))
        .unwrap();
    assert_eq!(l["credentialKind"], json!("api_key"));
    assert_eq!(l["outcome"], json!("stopped"));

    // 2 回目: 全部キャッシュ命中
    let b2 = bootstrap(&opts(&server, &cache, &key)).unwrap();
    assert_eq!(b2.downloaded, 0);
    assert_eq!(b2.cache_hits, 6);
    assert!(!b2.from_cache);

    // ローカルカタログに載っている
    let cat = cache.load_catalog();
    assert_eq!(cat.entries.len(), 1);
    assert_eq!(cat.entries[0].manifest_id, "cartpole");
    assert!(cat.entries[0].is_default && cat.entries[0].signed);

    // サーバを落としてもキャッシュから起動でき、同じ軌道になる
    server.stop();
    let b3 = bootstrap(&opts(&server, &cache, &key)).unwrap();
    assert!(b3.from_cache);
    assert_eq!(b3.revision, 1);
    assert_eq!(b3.verified_key_id.as_deref(), Some(server.key_id.as_str()));
    assert!(
        b3.telemetry().is_none(),
        "オフラインでは telemetry を送らない"
    );
    let mut s3 = Session::load(
        &b3.manifest_path,
        &b3.session_options(&cache, SessionOptions::default()),
    )
    .unwrap();
    let hash_offline = run_and_assert_balanced(&mut s3, 12.0);
    assert_eq!(
        hash_online, hash_offline,
        "オンラインとオフラインで軌道が一致する (決定性)"
    );

    // --offline 指定でも同じ
    let mut o = opts(&server, &cache, &key);
    o.offline = true;
    assert!(bootstrap(&o).unwrap().from_cache);

    // キャッシュの manifest を改竄すると署名検証で落ちる
    let p = cache
        .manifest_path(&server.base_url, "cartpole", 1)
        .unwrap();
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
    m["components"][0]["params"]["pendulumLength"] = json!(9.9);
    std::fs::write(&p, m.to_string()).unwrap();
    let err = bootstrap(&o).unwrap_err();
    assert!(format!("{err:#}").contains("署名検証に失敗"), "{err:#}");
}

#[test]
fn bootstrap_rejects_unknown_key_and_wrong_credentials() {
    if !artifacts_ready() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let server = TestServer::start(&dir.path().join("server"));
    let key = publish_cartpole(&server.base_url);
    let cache = Cache::new(dir.path().join("cache")).unwrap();

    // 信頼鍵なし → 拒否 (キャッシュにも落ちない)
    let mut bo = BootstrapOptions::new(&server.base_url, "cartpole", cache.clone()).unwrap();
    bo.credential = Some(Credential::ApiKey(key.clone()));
    let err = bootstrap(&bo).unwrap_err();
    assert!(format!("{err:#}").contains("信頼済み公開鍵"), "{err:#}");
    assert!(cache.load_catalog().entries.is_empty());

    // 別の鍵だけ信頼 → 拒否
    let other = ipemu_sign::SigningKeyFile::generate("other");
    bo.trusted_keys
        .insert_hex("other", &other.public_entry().unwrap().public_key)
        .unwrap();
    let err = bootstrap(&bo).unwrap_err();
    assert!(format!("{err:#}").contains("署名検証に失敗"), "{err:#}");

    // 無効な API キー → 401 (キャッシュに落ちない)
    let mut bad = opts(&server, &cache, "ipk_bogus");
    let err = bootstrap(&bad).unwrap_err();
    assert!(format!("{err:#}").contains("401"), "{err:#}");
    // 範囲外の構成 → 404
    bad = opts(&server, &cache, &key);
    bad.manifest_id = "nope".into();
    let err = bootstrap(&bad).unwrap_err();
    assert!(format!("{err:#}").contains("404"), "{err:#}");

    // 存在しない revision
    let mut bo = opts(&server, &cache, &key);
    bo.revision = Some(7);
    assert!(bootstrap(&bo).is_err());

    // チケット経由 (利用者): 発行 → 交換 → 取得
    let a = Admin {
        http: Client::new(),
        base: &server.base_url,
    };
    let admin = a.login("admin", "admin-password");
    let t = a.post(
        &admin,
        "/api/v1/launch-tickets",
        json!({"manifestId": "cartpole", "revision": 1}),
    );
    let mut bo = opts(&server, &cache, &key);
    bo.credential = Some(Credential::Ticket(t["ticket"].as_str().unwrap().to_owned()));
    let b = bootstrap(&bo).unwrap();
    assert!(!b.from_cache);
    // 同じチケットは 2 度使えない
    assert!(bootstrap(&bo).is_err());
}
