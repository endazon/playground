//! 配信 / 管理 API の結合テスト。in-process でサーバを立て、reqwest で叩く。
//! 登録 → 検証 → 承認 → 公開 → カタログ → 取得 → 署名検証、公開範囲外の 404、チケットの単回性と失効、ETag。
//!
//! 前提: build/components と build/firmware に wasm があること (scripts/build-*.sh)。無ければ skip。

use ipemu_server::config::Config;
use ipemu_server::App;
use reqwest::{Client, StatusCode};
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
    ];
    let missing: Vec<_> = need.iter().filter(|p| !root().join(p).exists()).collect();
    if !missing.is_empty() {
        eprintln!("SKIP: ビルド成果物がありません ({missing:?})");
        return false;
    }
    true
}

struct Ctx {
    app: App,
    http: Client,
    _dir: tempfile::TempDir,
}

impl Ctx {
    async fn new() -> Ctx {
        Ctx::with_config(|_| {}).await
    }
    async fn with_config(f: impl FnOnce(&mut Config)) -> Ctx {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = Config::for_test(dir.path());
        f(&mut cfg);
        let app = App::start(cfg).await.unwrap();
        Ctx {
            app,
            http: Client::new(),
            _dir: dir,
        }
    }
    fn url(&self, p: &str) -> String {
        format!("{}{p}", self.app.base_url)
    }
    async fn login(&self, user: &str, pass: &str) -> String {
        let r: Value = self
            .http
            .post(self.url("/api/auth/login"))
            .json(&json!({"username": user, "password": pass}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        r["token"].as_str().unwrap().to_owned()
    }
    async fn post(&self, tok: &str, p: &str, body: Value) -> (StatusCode, Value) {
        let r = self
            .http
            .post(self.url(p))
            .bearer_auth(tok)
            .json(&body)
            .send()
            .await
            .unwrap();
        let s = r.status();
        (s, r.json().await.unwrap_or(Value::Null))
    }
    async fn put(&self, tok: &str, p: &str, body: Value) -> (StatusCode, Value) {
        let r = self
            .http
            .put(self.url(p))
            .bearer_auth(tok)
            .json(&body)
            .send()
            .await
            .unwrap();
        let s = r.status();
        (s, r.json().await.unwrap_or(Value::Null))
    }
    async fn get(&self, tok: &str, p: &str) -> (StatusCode, Value) {
        let r = self
            .http
            .get(self.url(p))
            .bearer_auth(tok)
            .send()
            .await
            .unwrap();
        let s = r.status();
        (s, r.json().await.unwrap_or(Value::Null))
    }
    async fn create_user(&self, admin: &str, name: &str, roles: &[&str], groups: &[&str]) {
        let (s, b) = self
            .post(admin, "/api/admin/users", json!({"username": name, "password": "password-1234", "roles": roles, "groups": groups}))
            .await;
        assert_eq!(s, StatusCode::OK, "{b}");
    }
    async fn upload(
        &self,
        tok: &str,
        name: &str,
        version: &str,
        kind: &str,
        path: &Path,
    ) -> (StatusCode, Value) {
        let bytes = std::fs::read(path).unwrap();
        self.upload_bytes(tok, name, version, kind, bytes).await
    }
    async fn upload_bytes(
        &self,
        tok: &str,
        name: &str,
        version: &str,
        kind: &str,
        bytes: Vec<u8>,
    ) -> (StatusCode, Value) {
        let form = reqwest::multipart::Form::new()
            .text("name", name.to_owned())
            .text("version", version.to_owned())
            .text("kind", kind.to_owned())
            .part(
                "file",
                reqwest::multipart::Part::bytes(bytes).file_name("x.wasm"),
            );
        let r = self
            .http
            .post(self.url("/api/admin/artifacts"))
            .bearer_auth(tok)
            .multipart(form)
            .send()
            .await
            .unwrap();
        let s = r.status();
        (s, r.json().await.unwrap_or(Value::Null))
    }
}

/// dev/cartpole.manifest.json から配信用の本文を作る (path を落とす。sha256 / url はサーバが補完する)。
fn cartpole_body(id: &str) -> Value {
    let mut v: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("dev/cartpole.manifest.json")).unwrap(),
    )
    .unwrap();
    v["id"] = json!(id);
    v["scheduler"]["pacing"] = json!("fast");
    for (_, a) in v["artifacts"].as_object_mut().unwrap().iter_mut() {
        a.as_object_mut().unwrap().remove("path");
        a["url"] = json!("placeholder");
    }
    v
}

async fn register_cartpole_artifacts(cx: &Ctx, tok: &str) {
    for c in ["plant-cartpole", "motor-driver", "encoder", "comm-link"] {
        let (s, b) = cx
            .upload(
                tok,
                c,
                "0.1.0",
                "part",
                &root().join(format!("build/components/{c}.wasm")),
            )
            .await;
        assert_eq!(s, StatusCode::OK, "{c}: {b}");
        assert_eq!(b["describe"]["name"], json!(c));
        assert!(b["wasiImports"].as_array().unwrap().is_empty());
        assert!(b["world"]
            .as_str()
            .unwrap()
            .starts_with("ip:emu/part-world@"));
    }
    for f in ["fw-balance-c", "fw-balance-rs"] {
        let (s, b) = cx
            .upload(
                tok,
                f,
                "0.1.0",
                "firmware",
                &root().join(format!("build/firmware/{f}.wasm")),
            )
            .await;
        assert_eq!(s, StatusCode::OK, "{f}: {b}");
        assert!(b["world"].as_str().unwrap().starts_with("ip:emu/firmware@"));
    }
}

#[tokio::test]
async fn full_publish_flow_and_delivery() {
    if !artifacts_ready() {
        return;
    }
    let cx = Ctx::new().await;
    let admin = cx.login("admin", "admin-password").await;
    cx.create_user(&admin, "ed", &["user", "editor"], &[]).await;
    cx.create_user(&admin, "ap", &["user", "approver"], &[])
        .await;
    cx.create_user(&admin, "alice", &["user"], &["lab"]).await;
    cx.create_user(&admin, "bob", &["user"], &[]).await;
    let ed = cx.login("ed", "password-1234").await;
    let ap = cx.login("ap", "password-1234").await;
    let alice = cx.login("alice", "password-1234").await;
    let bob = cx.login("bob", "password-1234").await;

    // 部品登録。同名同版の再登録は拒否、plain user は登録不可
    register_cartpole_artifacts(&cx, &ed).await;
    let (s, _) = cx
        .upload(
            &ed,
            "encoder",
            "0.1.0",
            "part",
            &root().join("build/components/encoder.wasm"),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (s, _) = cx
        .upload(
            &alice,
            "encoder",
            "0.2.0",
            "part",
            &root().join("build/components/encoder.wasm"),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    // part として firmware を登録しようとすると弾く
    let (s, b) = cx
        .upload(
            &ed,
            "bad",
            "0.1.0",
            "part",
            &root().join("build/firmware/fw-balance-c.wasm"),
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{b}");

    // 下書き → 保存 (検証付き)
    let (s, b) = cx
        .post(&ed, "/api/admin/manifests", json!({"id": "cartpole"}))
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let (s, b) = cx
        .put(
            &ed,
            "/api/admin/manifests/cartpole/revisions/1",
            json!({"body": cartpole_body("cartpole")}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["validation"]["ok"], json!(true), "{}", b["validation"]);
    assert!(
        b["body"]["artifacts"]["plant-cartpole@0.1.0"]["sha256"]
            .as_str()
            .unwrap()
            .len()
            == 64
    );
    assert_eq!(
        b["body"]["artifacts"]["plant-cartpole@0.1.0"]["url"],
        json!("/api/v1/artifacts/plant-cartpole/0.1.0")
    );

    // 未公開なので一般利用者のカタログには出ない
    let (_, cat) = cx.get(&alice, "/api/v1/catalog").await;
    assert!(cat["items"].as_array().unwrap().is_empty());

    // 編集者本人は承認できない。承認者は公開できるが未承認の公開は拒否
    let (s, _) = cx
        .post(
            &ed,
            "/api/admin/manifests/cartpole/revisions/1/approve",
            json!({"reason": "x"}),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _) = cx
        .post(
            &ap,
            "/api/admin/manifests/cartpole/revisions/1/publish",
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (s, b) = cx
        .post(
            &ap,
            "/api/admin/manifests/cartpole/revisions/1/approve",
            json!({"reason": "ok"}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["state"], json!("approved"));
    assert_eq!(b["updatedBy"], json!("ed"));
    assert_eq!(b["approvedBy"], json!("ap"));
    // 承認済みは編集不可
    let (s, _) = cx
        .put(
            &ed,
            "/api/admin/manifests/cartpole/revisions/1",
            json!({"body": cartpole_body("cartpole")}),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (s, b) = cx
        .post(
            &ap,
            "/api/admin/manifests/cartpole/revisions/1/publish",
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["state"], json!("published"));
    assert_eq!(b["signedBody"]["signature"]["alg"], json!("ed25519"));

    // カタログと取得、署名検証
    let (_, cat) = cx.get(&alice, "/api/v1/catalog").await;
    assert_eq!(cat["items"].as_array().unwrap().len(), 1);
    assert_eq!(cat["items"][0]["defaultRevision"], json!(1));
    let r = cx
        .http
        .get(cx.url("/api/v1/manifests/cartpole?rev=default"))
        .bearer_auth(&alice)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["x-ipemu-signed"], "true");
    let launch_id = r.headers()["x-ipemu-launch-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let etag = r.headers()["etag"].to_str().unwrap().to_owned();
    let signed: Value = r.json().await.unwrap();
    let (_, keys) = cx.get(&alice, "/api/v1/keys").await;
    let trusted = ipemu_sign::TrustedKeys::from_json(&keys.to_string()).unwrap();
    let v = ipemu_sign::verify_manifest(&signed, &trusted).unwrap();
    assert_eq!(v.key_id, cx.app.state.key_id);
    // ETag → 304
    let r = cx
        .http
        .get(cx.url("/api/v1/manifests/cartpole"))
        .bearer_auth(&alice)
        .header("if-none-match", &etag)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_MODIFIED);

    // アーティファクト: ETag = sha256、immutable、If-None-Match、by-hash
    let sha = signed["artifacts"]["plant-cartpole@0.1.0"]["sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    let r = cx
        .http
        .get(cx.url("/api/v1/artifacts/plant-cartpole/0.1.0"))
        .bearer_auth(&alice)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["etag"].to_str().unwrap(), format!("\"{sha}\""));
    assert!(r.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("immutable"));
    assert_eq!(r.headers()["content-type"], "application/wasm");
    let bytes = r.bytes().await.unwrap();
    assert_eq!(ipemu_server::util::sha256_hex(&bytes), sha);
    let r = cx
        .http
        .get(cx.url("/api/v1/artifacts/plant-cartpole/0.1.0"))
        .bearer_auth(&alice)
        .header("if-none-match", format!("\"{sha}\""))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_MODIFIED);
    let r = cx
        .http
        .get(cx.url(&format!("/api/v1/artifacts/by-hash/{sha}")))
        .bearer_auth(&alice)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let r = cx
        .http
        .get(cx.url("/api/v1/artifacts/plant-cartpole/0.1.0"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);

    // telemetry → 起動履歴に反映
    let (s, _) = cx.post(&alice, "/api/v1/telemetry", json!({"launchId": launch_id, "manifestId": "cartpole", "revision": 1, "event": "started", "detail": {}})).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = cx.post(&alice, "/api/v1/telemetry", json!({"launchId": launch_id, "manifestId": "cartpole", "revision": 1, "event": "stopped", "detail": {"overruns": 0}})).await;
    assert_eq!(s, StatusCode::OK);
    let (_, mine) = cx.get(&alice, "/api/v1/launches/mine").await;
    let l = mine["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["id"] == json!(launch_id))
        .unwrap();
    assert_eq!(l["outcome"], json!("stopped"));
    assert!(l["endedAt"].is_string());
    let (_, launches) = cx.get(&admin, "/api/admin/launches").await;
    assert!(launches["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["id"] == json!(launch_id) && l["credentialKind"] == json!("session")));

    // hello → クライアント一覧
    let (s, _) = cx.post(&alice, "/api/v1/clients/hello", json!({"engineVersion": "0.1.0", "witVersions": ["ip:emu@0.1.0"], "os": "linux", "arch": "x86_64"})).await;
    assert_eq!(s, StatusCode::OK);
    let (_, clients) = cx.get(&admin, "/api/admin/clients").await;
    assert_eq!(clients["items"][0]["principalName"], json!("alice"));

    // 公開範囲: lab グループだけ。bob には 404、alice は取れる
    let (s, _) = cx
        .put(
            &ap,
            "/api/admin/manifests/cartpole/visibility",
            json!({"visibility": "groups", "groups": ["lab"]}),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let (_, cat) = cx.get(&bob, "/api/v1/catalog").await;
    assert!(cat["items"].as_array().unwrap().is_empty());
    let (s, _) = cx.get(&bob, "/api/v1/manifests/cartpole").await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = cx.get(&alice, "/api/v1/manifests/cartpole").await;
    assert_eq!(s, StatusCode::OK);
    // 公開範囲の変更は editor にはできない
    let (s, _) = cx
        .put(
            &ed,
            "/api/admin/manifests/cartpole/visibility",
            json!({"visibility": "all"}),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    // 2 つ目の revision: 下書き → 承認 → 公開しても既定は 1 のまま。既定の切替 (巻き戻し) は approver
    let (s, b) = cx
        .post(&ed, "/api/admin/manifests/cartpole/revisions", json!({}))
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["revision"], json!(2));
    assert_eq!(b["body"]["revision"], json!(2));
    let mut body2 = cartpole_body("cartpole");
    body2["name"] = json!("rev2");
    let (s, b) = cx
        .put(
            &ed,
            "/api/admin/manifests/cartpole/revisions/2",
            json!({"body": body2}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    cx.post(
        &ap,
        "/api/admin/manifests/cartpole/revisions/2/approve",
        json!({}),
    )
    .await;
    let (s, _) = cx
        .post(
            &ap,
            "/api/admin/manifests/cartpole/revisions/2/publish",
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let (_, m) = cx.get(&admin, "/api/admin/manifests/cartpole").await;
    assert_eq!(m["defaultRevision"], json!(1));
    let (s, _) = cx
        .put(
            &ap,
            "/api/admin/manifests/cartpole/default-revision",
            json!({"revision": 2}),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let r = cx
        .http
        .get(cx.url("/api/v1/manifests/cartpole"))
        .bearer_auth(&alice)
        .send()
        .await
        .unwrap();
    assert_eq!(r.headers()["x-ipemu-revision"], "2");
    let (s, _) = cx
        .put(
            &ap,
            "/api/admin/manifests/cartpole/default-revision",
            json!({"revision": 9}),
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // 監査ログに一連の操作が残る
    let (_, audit) = cx.get(&admin, "/api/admin/audit").await;
    let actions: Vec<&str> = audit["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|a| a["action"].as_str())
        .collect();
    for a in [
        "artifact.register",
        "manifest.create",
        "manifest.revision.update",
        "manifest.approve",
        "manifest.publish",
        "manifest.visibility",
        "manifest.default_revision",
        "user.create",
    ] {
        assert!(
            actions.contains(&a),
            "audit に {a} がありません: {actions:?}"
        );
    }
    // viewer でない利用者は管理 API に入れない
    let (s, _) = cx.get(&alice, "/api/admin/audit").await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    cx.app.stop().await;
}

#[tokio::test]
async fn validation_rejects_broken_manifests() {
    if !artifacts_ready() {
        return;
    }
    let cx = Ctx::new().await;
    let admin = cx.login("admin", "admin-password").await;
    register_cartpole_artifacts(&cx, &admin).await;
    // 未登録のアーティファクト
    let mut body = cartpole_body("broken");
    body["artifacts"]["ghost@1.0.0"] = json!({"kind": "part", "url": "x"});
    body["components"][0]["artifact"] = json!("ghost@1.0.0");
    let (s, b) = cx.post(&admin, "/api/admin/validate", body).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["report"]["ok"], json!(false));
    assert!(
        b["report"]["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["rule"] == json!("V2")),
        "{}",
        b["report"]
    );
    // 存在しないポートへの配線
    let mut body = cartpole_body("broken");
    body["wires"][0]["to"] = json!("enc-cart.nope");
    let (_, b) = cx.post(&admin, "/api/admin/validate", body).await;
    assert!(
        b["report"]["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["rule"] == json!("V4")),
        "{}",
        b["report"]
    );
    // スキーマ違反
    let mut body = cartpole_body("broken");
    body["scheduler"]["baseStepNs"] = json!(1);
    let (_, b) = cx.post(&admin, "/api/admin/validate", body).await;
    assert!(
        b["report"]["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["rule"] == json!("V1")),
        "{}",
        b["report"]
    );
    // 承認は検証エラーがあると拒否される
    cx.create_user(&admin, "ap", &["approver"], &[]).await;
    let ap = cx.login("ap", "password-1234").await;
    let mut bad = cartpole_body("broken");
    bad["wires"][0]["to"] = json!("enc-cart.nope");
    cx.post(
        &admin,
        "/api/admin/manifests",
        json!({"id": "broken", "body": bad}),
    )
    .await;
    let (s, _) = cx
        .post(
            &ap,
            "/api/admin/manifests/broken/revisions/1/approve",
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    cx.app.stop().await;
}

#[tokio::test]
async fn launch_tickets_are_single_use_and_expire() {
    let cx = Ctx::new().await;
    let admin = cx.login("admin", "admin-password").await;
    cx.create_user(&admin, "user1", &["user"], &[]).await;
    let u = cx.login("user1", "password-1234").await;
    // 構成 (公開は不要。チケットは公開範囲だけ見る)
    cx.post(&admin, "/api/admin/manifests", json!({"id": "demo"}))
        .await;

    let (s, t) = cx
        .post(
            &u,
            "/api/v1/launch-tickets",
            json!({"manifestId": "demo", "revision": 1}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{t}");
    let ticket = t["ticket"].as_str().unwrap().to_owned();
    assert!(t["launchUrl"]
        .as_str()
        .unwrap()
        .starts_with("ipemu://launch?server="));
    assert!(t["launchUrl"]
        .as_str()
        .unwrap()
        .contains("&manifest=demo&rev=1&ticket=tk_"));

    // 交換は認証なしで 1 回だけ
    let r = cx
        .http
        .post(cx.url(&format!("/api/v1/launch-tickets/{ticket}/exchange")))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let b: Value = r.json().await.unwrap();
    let tok = b["token"].as_str().unwrap().to_owned();
    assert!(tok.starts_with("lt_"));
    let r = cx
        .http
        .post(cx.url(&format!("/api/v1/launch-tickets/{ticket}/exchange")))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::GONE);
    // 短命トークンで利用者として振る舞える (credential は ticket)
    let (s, me) = cx.get(&tok, "/api/auth/me").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(me["principal"]["credential"], json!("ticket"));
    assert_eq!(me["principal"]["name"], json!("user1"));

    // 失効: expires_at を過去にする
    let (_, t2) = cx
        .post(&u, "/api/v1/launch-tickets", json!({"manifestId": "demo"}))
        .await;
    let ticket2 = t2["ticket"].as_str().unwrap().to_owned();
    sqlx::query(
        "UPDATE launch_tickets SET expires_at = '2000-01-01T00:00:00Z' WHERE ticket_hash = ?",
    )
    .bind(ipemu_server::util::token_hash(&ticket2))
    .execute(&cx.app.state.pool)
    .await
    .unwrap();
    let r = cx
        .http
        .post(cx.url(&format!("/api/v1/launch-tickets/{ticket2}/exchange")))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::GONE);
    // でたらめなチケット
    let r = cx
        .http
        .post(cx.url("/api/v1/launch-tickets/tk_nope/exchange"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::GONE);
    // API キーではチケットを発行できない
    let (_, k) = cx
        .post(&admin, "/api/admin/api-keys", json!({"name": "ci"}))
        .await;
    let key = k["key"].as_str().unwrap().to_owned();
    let (s, _) = cx
        .post(
            &key,
            "/api/v1/launch-tickets",
            json!({"manifestId": "demo"}),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    cx.app.stop().await;
}

#[tokio::test]
async fn api_keys_scope_and_revocation() {
    if !artifacts_ready() {
        return;
    }
    let cx = Ctx::new().await;
    let admin = cx.login("admin", "admin-password").await;
    cx.create_user(&admin, "ap", &["approver"], &[]).await;
    let ap = cx.login("ap", "password-1234").await;
    register_cartpole_artifacts(&cx, &admin).await;
    for id in ["one", "two"] {
        cx.post(
            &admin,
            "/api/admin/manifests",
            json!({"id": id, "body": cartpole_body(id)}),
        )
        .await;
        let (s, b) = cx
            .post(
                &ap,
                &format!("/api/admin/manifests/{id}/revisions/1/approve"),
                json!({}),
            )
            .await;
        assert_eq!(s, StatusCode::OK, "{b}");
        let (s, _) = cx
            .post(
                &ap,
                &format!("/api/admin/manifests/{id}/revisions/1/publish"),
                json!({}),
            )
            .await;
        assert_eq!(s, StatusCode::OK);
    }
    let (s, k) = cx
        .post(
            &admin,
            "/api/admin/api-keys",
            json!({"name": "ci", "allowedManifests": ["one"]}),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let key = k["key"].as_str().unwrap().to_owned();
    let (_, cat) = cx.get(&key, "/api/v1/catalog").await;
    let ids: Vec<&str> = cat["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i["id"].as_str())
        .collect();
    assert_eq!(ids, vec!["one"]);
    let (s, _) = cx.get(&key, "/api/v1/manifests/two").await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let r = cx
        .http
        .get(cx.url("/api/v1/manifests/one"))
        .bearer_auth(&key)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    // API キーは管理 API に入れない
    let (s, _) = cx.get(&key, "/api/admin/manifests").await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    // 失効
    let r = cx
        .http
        .delete(cx.url(&format!(
            "/api/admin/api-keys/{}",
            k["id"].as_str().unwrap()
        )))
        .bearer_auth(&admin)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let (s, _) = cx.get(&key, "/api/v1/catalog").await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    // ログイン失敗の抑止
    for _ in 0..5 {
        let r = cx
            .http
            .post(cx.url("/api/auth/login"))
            .json(&json!({"username": "ap", "password": "wrong"}))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    }
    let r = cx
        .http
        .post(cx.url("/api/auth/login"))
        .json(&json!({"username": "ap", "password": "password-1234"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    cx.app.stop().await;
}

/// capability を宣言する部品 (M5 の tcp-reporter): 登録時に deny-all 形で describe を取れ、
/// 宣言のない manifest は V7 で落ち、宣言があれば通る。
#[tokio::test]
async fn capability_parts_are_inspected_and_checked_against_declarations() {
    if !artifacts_ready() || !root().join("build/components/tcp-reporter.wasm").exists() {
        eprintln!("SKIP: build/components/tcp-reporter.wasm がありません");
        return;
    }
    let cx = Ctx::new().await;
    let admin = cx.login("admin", "admin-password").await;
    register_cartpole_artifacts(&cx, &admin).await;
    let (s, b) = cx
        .upload(
            &admin,
            "tcp-reporter",
            "0.1.0",
            "part",
            &root().join("build/components/tcp-reporter.wasm"),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(
        b["describe"]["name"],
        json!("tcp-reporter"),
        "deny-all 形で describe が取れる"
    );
    let wasi: Vec<&str> = b["wasiImports"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(
        wasi.iter().any(|i| i.starts_with("wasi:sockets/")),
        "{wasi:?}"
    );
    assert!(b["imports"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["name"].as_str().is_some_and(|n| n.starts_with("wasi:"))));

    let mut body: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("dev/cartpole-tcp.manifest.json")).unwrap(),
    )
    .unwrap();
    body["id"] = json!("cartpole-tcp");
    for (_, a) in body["artifacts"].as_object_mut().unwrap().iter_mut() {
        a.as_object_mut().unwrap().remove("path");
        a["url"] = json!("placeholder");
    }
    // 宣言どおり → OK (V8 の警告はあってもよい)
    let (s, r) = cx.post(&admin, "/api/admin/validate", body.clone()).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(r["report"]["ok"], json!(true), "{}", r["report"]);
    // 宣言を外す → V7
    let mut undeclared = body.clone();
    for c in undeclared["components"].as_array_mut().unwrap() {
        c.as_object_mut().unwrap().remove("capabilities");
    }
    let (_, r) = cx.post(&admin, "/api/admin/validate", undeclared).await;
    assert_eq!(r["report"]["ok"], json!(false));
    assert!(
        r["report"]["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["rule"] == json!("V7")),
        "{}",
        r["report"]
    );
    // 公開まで通り、capabilitiesHash は ipemu-core の定義と一致する
    cx.create_user(&admin, "ap", &["approver"], &[]).await;
    let ap = cx.login("ap", "password-1234").await;
    let (s, _) = cx
        .post(
            &admin,
            "/api/admin/manifests",
            json!({"id": "cartpole-tcp", "body": body}),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let (s, r) = cx
        .post(
            &ap,
            "/api/admin/manifests/cartpole-tcp/revisions/1/approve",
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let (s, r) = cx
        .post(
            &ap,
            "/api/admin/manifests/cartpole-tcp/revisions/1/publish",
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let signed: ipemu_core::Manifest = serde_json::from_value(r["signedBody"].clone()).unwrap();
    assert_eq!(
        signed.approval.as_ref().unwrap().capabilities_hash,
        ipemu_core::capabilities_hash(&signed)
    );
    cx.app.stop().await;
}

/// 有効な cartpole 構成を公開まで進める (admin が下書きを作り、ap が承認・公開する)。
async fn publish_cartpole(cx: &Ctx, admin: &str, ap: &str, id: &str) {
    let (s, b) = cx
        .post(
            admin,
            "/api/admin/manifests",
            json!({"id": id, "body": cartpole_body(id)}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let (s, b) = cx
        .post(
            ap,
            &format!("/api/admin/manifests/{id}/revisions/1/approve"),
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let (s, b) = cx
        .post(
            ap,
            &format!("/api/admin/manifests/{id}/revisions/1/publish"),
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
}

/// 監査 A1 (D41): editor と approver を兼ねるユーザが自分で編集した下書きは、id で照合して承認を拒否する。
/// 別のユーザなら承認できる。id 列の無い古い行はユーザ名で照合する。
#[tokio::test]
async fn editor_cannot_approve_own_revision_even_with_both_roles() {
    if !artifacts_ready() {
        return;
    }
    let cx = Ctx::new().await;
    let admin = cx.login("admin", "admin-password").await;
    cx.create_user(&admin, "both", &["user", "editor", "approver"], &[])
        .await;
    cx.create_user(&admin, "ap", &["user", "approver"], &[])
        .await;
    let both = cx.login("both", "password-1234").await;
    let ap = cx.login("ap", "password-1234").await;
    register_cartpole_artifacts(&cx, &both).await;

    // 作成 (insert) した本人の承認は拒否
    let (s, b) = cx
        .post(
            &both,
            "/api/admin/manifests",
            json!({"id": "selfapp", "body": cartpole_body("selfapp")}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let (s, b) = cx
        .post(
            &both,
            "/api/admin/manifests/selfapp/revisions/1/approve",
            json!({"reason": "self"}),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "{b}");

    // 別人 (ap) が編集すると、今度は ap 本人が承認できず、both が承認できる
    let (s, b) = cx
        .put(
            &admin,
            "/api/admin/manifests/selfapp/revisions/1",
            json!({"body": cartpole_body("selfapp")}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let (s, _) = cx
        .post(
            &admin,
            "/api/admin/manifests/selfapp/revisions/1/approve",
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "admin でも最終編集者なら拒否");
    let (s, b) = cx
        .post(
            &both,
            "/api/admin/manifests/selfapp/revisions/1/approve",
            json!({"reason": "other"}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["approvedBy"], json!("both"));
    assert_eq!(b["updatedBy"], json!("admin"));

    // 保存 (update) した本人の承認も拒否。別人なら 200
    let (s, b) = cx
        .post(&admin, "/api/admin/manifests", json!({"id": "selfapp2"}))
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let (s, b) = cx
        .put(
            &both,
            "/api/admin/manifests/selfapp2/revisions/1",
            json!({"body": cartpole_body("selfapp2")}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let (s, _) = cx
        .post(
            &both,
            "/api/admin/manifests/selfapp2/revisions/1/approve",
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    // id 列の無い古い行 (migration 0002 より前) はユーザ名で照合する
    sqlx::query(
        "UPDATE manifest_revisions SET updated_by_id = NULL, created_by_id = NULL WHERE manifest_id = 'selfapp2'",
    )
    .execute(&cx.app.state.pool)
    .await
    .unwrap();
    let (s, _) = cx
        .post(
            &both,
            "/api/admin/manifests/selfapp2/revisions/1/approve",
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "古い行は名前で照合");
    let (s, b) = cx
        .post(
            &ap,
            "/api/admin/manifests/selfapp2/revisions/1/approve",
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    cx.app.stop().await;
}

/// 監査 A2 (D42): チケット交換トークン (`lt_`) は配信 API 限定。
/// admin のトークンでも管理 API・チケット発行・パスワード変更には使えず、新鮮なログインとも見なさない。
#[tokio::test]
async fn launch_token_is_limited_to_delivery_api() {
    if !artifacts_ready() {
        return;
    }
    let cx = Ctx::new().await;
    let admin = cx.login("admin", "admin-password").await;
    cx.create_user(&admin, "ap", &["user", "approver"], &[])
        .await;
    let ap = cx.login("ap", "password-1234").await;
    register_cartpole_artifacts(&cx, &admin).await;
    publish_cartpole(&cx, &admin, &ap, "cartpole").await;
    // 承認待ちの下書き (admin が作成)
    let (s, b) = cx
        .post(&admin, "/api/admin/manifests/cartpole/revisions", json!({}))
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");

    // ap (承認者) のチケット → 短命トークン
    let (s, t) = cx
        .post(
            &ap,
            "/api/v1/launch-tickets",
            json!({"manifestId": "cartpole"}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{t}");
    let r = cx
        .http
        .post(cx.url(&format!(
            "/api/v1/launch-tickets/{}/exchange",
            t["ticket"].as_str().unwrap()
        )))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let lt = r.json::<Value>().await.unwrap()["token"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(lt.starts_with("lt_"));

    // 配信 API は使える
    let (s, cat) = cx.get(&lt, "/api/v1/catalog").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(cat["items"].as_array().unwrap().len(), 1);
    let r = cx
        .http
        .get(cx.url("/api/v1/manifests/cartpole"))
        .bearer_auth(&lt)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let launch_id = r.headers()["x-ipemu-launch-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let r = cx
        .http
        .get(cx.url("/api/v1/artifacts/plant-cartpole/0.1.0"))
        .bearer_auth(&lt)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let (s, _) = cx.post(&lt, "/api/v1/telemetry", json!({"launchId": launch_id, "manifestId": "cartpole", "revision": 1, "event": "started", "detail": {}})).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = cx
        .post(
            &lt,
            "/api/v1/clients/hello",
            json!({"engineVersion": "0.1.0", "os": "linux", "arch": "x86_64"}),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = cx.get(&lt, "/api/v1/launches/mine").await;
    assert_eq!(s, StatusCode::OK);

    // 承認 (管理 API) は 403。比較のためログインセッションなら通ることも確かめる
    let (s, b) = cx
        .post(
            &lt,
            "/api/admin/manifests/cartpole/revisions/2/approve",
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "{b}");
    let (s, _) = cx.get(&lt, "/api/admin/manifests").await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    // チケットの再発行 (UserAuth) とパスワード変更も 403
    let (s, _) = cx
        .post(
            &lt,
            "/api/v1/launch-tickets",
            json!({"manifestId": "cartpole"}),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _) = cx
        .post(
            &lt,
            "/api/auth/password",
            json!({"currentPassword": "password-1234", "newPassword": "password-5678"}),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, b) = cx
        .post(
            &ap,
            "/api/admin/manifests/cartpole/revisions/2/approve",
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");

    // admin のチケットでも同じ (ユーザ作成・API キー発行は不可)
    let (_, t) = cx
        .post(
            &admin,
            "/api/v1/launch-tickets",
            json!({"manifestId": "cartpole"}),
        )
        .await;
    let r = cx
        .http
        .post(cx.url(&format!(
            "/api/v1/launch-tickets/{}/exchange",
            t["ticket"].as_str().unwrap()
        )))
        .send()
        .await
        .unwrap();
    let lt_admin = r.json::<Value>().await.unwrap()["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let (s, _) = cx
        .post(&lt_admin, "/api/admin/api-keys", json!({"name": "x"}))
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _) = cx
        .post(
            &lt_admin,
            "/api/admin/users",
            json!({"username": "evil", "password": "password-1234", "roles": ["admin"]}),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _) = cx.get(&lt_admin, "/api/v1/catalog").await;
    assert_eq!(s, StatusCode::OK);
    cx.app.stop().await;
}

/// 監査 A5 (D43): 部品アップロードは axum 既定の 2MB ではなく `max_artifact_bytes` + 余裕まで受ける。
/// 上限を超えると 413。成果物のビルドは不要 (中身の検査で落ちてよい)。
#[tokio::test]
async fn artifact_upload_accepts_bodies_beyond_axum_default() {
    let cx = Ctx::with_config(|c| c.max_artifact_bytes = 4 * 1024 * 1024).await;
    let admin = cx.login("admin", "admin-password").await;
    // 3MB (axum 既定の 2MB 超): 413 にならず、中身の検査の結果 (422) が返る
    let junk: Vec<u8> = (0..3 * 1024 * 1024u32)
        .map(|i| (i.wrapping_mul(2654435761) >> 24) as u8)
        .collect();
    let (s, b) = cx
        .upload_bytes(&admin, "big-ui", "0.1.0", "ui", junk.clone())
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{b}");
    let (s, b) = cx
        .upload_bytes(&admin, "big-part", "0.1.0", "part", junk)
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{b}");
    // 3MB の tar.gz (gzip の magic だけ合わせる) は登録まで通る
    let mut gz = vec![0u8; 3 * 1024 * 1024];
    gz[0] = 0x1f;
    gz[1] = 0x8b;
    let (s, b) = cx.upload_bytes(&admin, "big-ui", "0.1.0", "ui", gz).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["size"], json!(3 * 1024 * 1024));
    // max_artifact_bytes 超過 (本文上限の余裕の内側) は 413
    let (s, b) = cx
        .upload_bytes(
            &admin,
            "too-big",
            "0.1.0",
            "ui",
            vec![0u8; 4 * 1024 * 1024 + 512 * 1024],
        )
        .await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE, "{b}");
    // 他の管理 API は既定の上限 (2MB) のまま
    let big = "x".repeat(3 * 1024 * 1024);
    let (s, _) = cx
        .post(&admin, "/api/admin/validate", json!({"pad": big}))
        .await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);
    cx.app.stop().await;
}

/// 監査 A9: telemetry の `event` は 64 文字以内の `[a-z_]`、`detail` は 16KB 以内。
#[tokio::test]
async fn telemetry_rejects_oversized_or_malformed_fields() {
    let cx = Ctx::new().await;
    let admin = cx.login("admin", "admin-password").await;
    let base = |event: &str, detail: Value| json!({"manifestId": "demo", "revision": 1, "event": event, "detail": detail});
    let (s, _) = cx
        .post(
            &admin,
            "/api/v1/telemetry",
            base("started", json!({"n": 1})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = cx
        .post(
            &admin,
            "/api/v1/telemetry",
            base(&"a".repeat(64), json!({})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    for bad in ["", "Started", "run-summary", "x y", &"a".repeat(65)] {
        let (s, b) = cx
            .post(&admin, "/api/v1/telemetry", base(bad, json!({})))
            .await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "event={bad:?}: {b}");
    }
    let (s, _) = cx
        .post(
            &admin,
            "/api/v1/telemetry",
            base("stopped", json!({"log": "x".repeat(16 * 1024)})),
        )
        .await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM telemetry")
        .fetch_one(&cx.app.state.pool)
        .await
        .unwrap();
    assert_eq!(n, 2, "拒否したものは保存しない");
    cx.app.stop().await;
}
