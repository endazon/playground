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
        let dir = tempfile::tempdir().unwrap();
        let app = App::start(Config::for_test(dir.path())).await.unwrap();
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
