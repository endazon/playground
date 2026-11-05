//! 管理 API `/api/admin`。viewer は閲覧、editor は下書き、approver は承認・公開・公開範囲、admin はユーザと鍵。

use crate::api::v1::LaunchRow;
use crate::auth::{self, AdminAuth, Principal};
use crate::error::{ApiError, ApiResult};
use crate::inspect;
use crate::manifests::{self, ArtifactRow, ManifestRow, RevisionRow};
use crate::state::AppState;
use crate::util;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router(max_upload: usize) -> Router<AppState> {
    Router::new()
        .route("/artifacts", get(list_artifacts).post(upload_artifact))
        .route("/artifacts/{id}", get(get_artifact))
        .route("/manifests", get(list_manifests).post(create_manifest))
        .route("/manifests/{id}", get(get_manifest))
        .route("/manifests/{id}/visibility", put(set_visibility))
        .route(
            "/manifests/{id}/default-revision",
            put(set_default_revision),
        )
        .route("/manifests/{id}/revisions", post(create_revision))
        .route(
            "/manifests/{id}/revisions/{rev}",
            get(get_revision)
                .put(update_revision)
                .delete(delete_revision),
        )
        .route(
            "/manifests/{id}/revisions/{rev}/validate",
            post(validate_revision),
        )
        .route(
            "/manifests/{id}/revisions/{rev}/approve",
            post(approve_revision),
        )
        .route(
            "/manifests/{id}/revisions/{rev}/publish",
            post(publish_revision),
        )
        .route("/validate", post(validate_adhoc))
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", put(update_user))
        .route("/users/{id}/password", post(reset_password))
        .route("/groups", get(list_groups))
        .route("/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api-keys/{id}", delete(revoke_api_key))
        .route("/audit", get(list_audit))
        .route("/launches", get(list_launches))
        .route("/clients", get(list_clients))
        // multipart の上限 = アーティファクト上限 + 余裕
        .layer(DefaultBodyLimit::max(max_upload + 1024 * 1024))
}

fn artifact_json(a: &ArtifactRow) -> Value {
    json!({
        "id": a.id,
        "name": a.name,
        "version": a.version,
        "ref": format!("{}@{}", a.name, a.version),
        "kind": a.kind,
        "sha256": a.sha256,
        "size": a.size,
        "contentType": a.content_type,
        "world": a.world,
        "describe": a.describe_json.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
        "wasiImports": util::json_vec(&a.wasi_imports),
        "imports": serde_json::from_str::<Value>(&a.imports).unwrap_or(Value::Array(vec![])),
        "uploadedBy": a.uploaded_by,
        "createdAt": a.created_at,
        "url": format!("/api/v1/artifacts/{}/{}", a.name, a.version),
    })
}

// ---- 部品 ----------------------------------------------------------------

async fn list_artifacts(
    State(st): State<AppState>,
    AdminAuth(_p): AdminAuth,
) -> ApiResult<Json<Value>> {
    let rows =
        sqlx::query_as::<_, ArtifactRow>("SELECT * FROM artifacts ORDER BY name, created_at")
            .fetch_all(&st.pool)
            .await?;
    Ok(Json(
        json!({ "items": rows.iter().map(artifact_json).collect::<Vec<_>>() }),
    ))
}

async fn get_artifact(
    State(st): State<AppState>,
    AdminAuth(_p): AdminAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let row = sqlx::query_as::<_, ArtifactRow>("SELECT * FROM artifacts WHERE id = ?")
        .bind(&id)
        .fetch_optional(&st.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("アーティファクトがありません"))?;
    Ok(Json(artifact_json(&row)))
}

fn valid_artifact_ref(name: &str, version: &str) -> bool {
    let name_ok = !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    let mut parts = version.splitn(3, '.');
    let ver_ok = parts.next().is_some_and(|s| s.parse::<u64>().is_ok())
        && parts.next().is_some_and(|s| s.parse::<u64>().is_ok())
        && parts.next().is_some_and(|s| {
            let core: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
            !core.is_empty()
                && s[core.len()..]
                    .chars()
                    .next()
                    .is_none_or(|c| c == '-' || c == '+')
        });
    name_ok && ver_ok
}

/// multipart: `name`, `version`, `kind` (part/firmware/ui), `file`。
/// サーバが describe() と import を取り、同名同版の再登録は拒否する。
async fn upload_artifact(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    mut mp: Multipart,
) -> ApiResult<Json<Value>> {
    p.require(p.can_edit(), "部品登録")?;
    let (mut name, mut version, mut kind, mut file): (String, String, String, Option<Vec<u8>>) =
        (String::new(), String::new(), "part".into(), None);
    while let Some(field) = mp
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?
    {
        let fname = field.name().unwrap_or("").to_owned();
        match fname.as_str() {
            "name" => {
                name = field
                    .text()
                    .await
                    .map_err(|e| ApiError::bad_request(e.to_string()))?
                    .trim()
                    .to_owned()
            }
            "version" => {
                version = field
                    .text()
                    .await
                    .map_err(|e| ApiError::bad_request(e.to_string()))?
                    .trim()
                    .to_owned()
            }
            "kind" => {
                kind = field
                    .text()
                    .await
                    .map_err(|e| ApiError::bad_request(e.to_string()))?
                    .trim()
                    .to_owned()
            }
            "file" => {
                let b = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::too_large(format!("ファイルを読めません: {e}")))?;
                if b.len() > st.config.max_artifact_bytes {
                    return Err(ApiError::too_large(format!(
                        "アーティファクトは {} バイトまで",
                        st.config.max_artifact_bytes
                    )));
                }
                file = Some(b.to_vec());
            }
            _ => {}
        }
    }
    let Some(bytes) = file else {
        return Err(ApiError::bad_request("file がありません"));
    };
    if !valid_artifact_ref(&name, &version) {
        return Err(ApiError::bad_request(
            "name は [a-z0-9-]、version は semver (x.y.z) で指定してください",
        ));
    }
    if manifests::load_artifact(&st.pool, &format!("{name}@{version}"))
        .await?
        .is_some()
    {
        return Err(ApiError::conflict(format!("{name}@{version} は登録済みです (アーティファクトはイミュータブル。版を上げてください)")));
    }
    // 検査は CPU を使うので blocking スレッドで
    let kind_c = kind.clone();
    let bytes_arc = std::sync::Arc::new(bytes);
    let b2 = bytes_arc.clone();
    let insp = tokio::task::spawn_blocking(move || inspect::inspect(&kind_c, &b2))
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .map_err(|e| ApiError::unprocessable(format!("wasm の検査に失敗: {e:#}")))?;
    if let Some(d) = &insp.describe {
        if d.name != name || d.version != version {
            tracing::warn!(%name, %version, desc_name = %d.name, desc_version = %d.version, "describe() の名前 / 版が登録名と異なります");
        }
    }
    let sha = st.store.put(&bytes_arc)?;
    let content_type = if kind == "ui" {
        "application/gzip"
    } else {
        "application/wasm"
    };
    let id = util::new_id();
    sqlx::query(
        "INSERT INTO artifacts (id, name, version, kind, sha256, size, content_type, world, describe_json, wasi_imports, imports, uploaded_by, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&version)
    .bind(&kind)
    .bind(&sha)
    .bind(bytes_arc.len() as i64)
    .bind(content_type)
    .bind(&insp.world)
    .bind(insp.describe.as_ref().map(|d| serde_json::to_string(d).unwrap()))
    .bind(serde_json::to_string(&insp.wasi_imports)?)
    .bind(serde_json::to_string(&insp.imports)?)
    .bind(&p.name)
    .bind(util::now_str())
    .execute(&st.pool)
    .await?;
    crate::audit::record(
        &st.pool,
        &p,
        "artifact.register",
        &format!("{name}@{version}"),
        json!({"sha256": sha, "kind": kind, "wasiImports": insp.wasi_imports}),
    )
    .await?;
    let row = sqlx::query_as::<_, ArtifactRow>("SELECT * FROM artifacts WHERE id = ?")
        .bind(&id)
        .fetch_one(&st.pool)
        .await?;
    Ok(Json(artifact_json(&row)))
}

// ---- 構成 ----------------------------------------------------------------

async fn revisions_of(st: &AppState, id: &str) -> ApiResult<Vec<RevisionRow>> {
    Ok(sqlx::query_as::<_, RevisionRow>(
        "SELECT * FROM manifest_revisions WHERE manifest_id = ? ORDER BY revision",
    )
    .bind(id)
    .fetch_all(&st.pool)
    .await?)
}

fn manifest_with_revisions(m: &ManifestRow, revs: &[RevisionRow]) -> Value {
    let mut v = m.to_json();
    v["revisions"] = Value::Array(revs.iter().map(|r| r.to_json(false)).collect());
    v
}

async fn list_manifests(
    State(st): State<AppState>,
    AdminAuth(_p): AdminAuth,
) -> ApiResult<Json<Value>> {
    let rows = sqlx::query_as::<_, ManifestRow>("SELECT * FROM manifests ORDER BY id")
        .fetch_all(&st.pool)
        .await?;
    let mut items = Vec::new();
    for m in &rows {
        let revs = revisions_of(&st, &m.id).await?;
        items.push(manifest_with_revisions(m, &revs));
    }
    Ok(Json(json!({ "items": items })))
}

#[derive(Deserialize)]
struct CreateManifestReq {
    id: String,
    #[serde(default)]
    body: Option<Value>,
}

fn valid_manifest_id(id: &str) -> bool {
    id.len() >= 3
        && id.len() <= 64
        && id
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// 下書きの初期本文 (id を埋めた骨組み)。
fn skeleton(id: &str) -> Value {
    json!({
        "manifestVersion": 1,
        "id": id,
        "revision": 1,
        "name": id,
        "scheduler": { "baseStepNs": 1000000, "pacing": "realtime", "snapshotDivider": 20 },
        "artifacts": {},
        "components": [],
        "wires": []
    })
}

/// 本文を保存できる形に整える: id / revision を揃え、approval / signature を落とし、登録済みアーティファクトで補完する。
async fn normalize_body(st: &AppState, id: &str, rev: i64, mut body: Value) -> ApiResult<Value> {
    let obj = body
        .as_object_mut()
        .ok_or_else(|| ApiError::bad_request("body はオブジェクト"))?;
    obj.insert("id".into(), json!(id));
    obj.insert("revision".into(), json!(rev));
    obj.remove("approval");
    obj.remove("signature");
    manifests::enrich_artifacts(&st.pool, &mut body).await?;
    Ok(body)
}

async fn insert_revision(
    st: &AppState,
    p: &Principal,
    id: &str,
    rev: i64,
    body: &Value,
) -> ApiResult<()> {
    let text = body.to_string();
    let now = util::now_str();
    sqlx::query(
        "INSERT INTO manifest_revisions (manifest_id, revision, state, body, body_sha256, created_by, updated_by, created_at, updated_at)
         VALUES (?, ?, 'draft', ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(rev)
    .bind(&text)
    .bind(util::sha256_hex(text.as_bytes()))
    .bind(&p.name)
    .bind(&p.name)
    .bind(&now)
    .bind(&now)
    .execute(&st.pool)
    .await?;
    Ok(())
}

async fn create_manifest(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    Json(req): Json<CreateManifestReq>,
) -> ApiResult<Json<Value>> {
    p.require(p.can_edit(), "構成の作成")?;
    if !valid_manifest_id(&req.id) {
        return Err(ApiError::bad_request(
            "id は小文字英数字とハイフン、3〜64 文字",
        ));
    }
    if manifests::load_manifest(&st.pool, &req.id).await.is_ok() {
        return Err(ApiError::conflict(format!(
            "構成 {} は既にあります",
            req.id
        )));
    }
    let body = normalize_body(
        &st,
        &req.id,
        1,
        req.body.unwrap_or_else(|| skeleton(&req.id)),
    )
    .await?;
    let now = util::now_str();
    sqlx::query("INSERT INTO manifests (id, name, description, created_by, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&req.id)
        .bind(body.get("name").and_then(|v| v.as_str()).unwrap_or(&req.id))
        .bind(body.get("description").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(&p.name)
        .bind(&now)
        .bind(&now)
        .execute(&st.pool)
        .await?;
    insert_revision(&st, &p, &req.id, 1, &body).await?;
    crate::audit::record(&st.pool, &p, "manifest.create", &req.id, json!({})).await?;
    let m = manifests::load_manifest(&st.pool, &req.id).await?;
    let revs = revisions_of(&st, &req.id).await?;
    Ok(Json(manifest_with_revisions(&m, &revs)))
}

async fn get_manifest(
    State(st): State<AppState>,
    AdminAuth(_p): AdminAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let m = manifests::load_manifest(&st.pool, &id).await?;
    let revs = revisions_of(&st, &id).await?;
    Ok(Json(manifest_with_revisions(&m, &revs)))
}

#[derive(Deserialize)]
struct VisibilityReq {
    visibility: String,
    #[serde(default)]
    groups: Vec<String>,
}

async fn set_visibility(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    Path(id): Path<String>,
    Json(req): Json<VisibilityReq>,
) -> ApiResult<Json<Value>> {
    p.require(p.can_approve(), "公開範囲の変更")?;
    if req.visibility != "all" && req.visibility != "groups" {
        return Err(ApiError::bad_request("visibility は all か groups"));
    }
    manifests::load_manifest(&st.pool, &id).await?;
    sqlx::query(
        "UPDATE manifests SET visibility = ?, visibility_groups = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&req.visibility)
    .bind(serde_json::to_string(&req.groups)?)
    .bind(util::now_str())
    .bind(&id)
    .execute(&st.pool)
    .await?;
    crate::audit::record(
        &st.pool,
        &p,
        "manifest.visibility",
        &id,
        json!({"visibility": req.visibility, "groups": req.groups}),
    )
    .await?;
    let m = manifests::load_manifest(&st.pool, &id).await?;
    Ok(Json(m.to_json()))
}

#[derive(Deserialize)]
struct DefaultRevReq {
    revision: i64,
}

/// カタログの既定 revision (巻き戻しもこれ)。公開済みの revision にだけ設定できる。
async fn set_default_revision(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    Path(id): Path<String>,
    Json(req): Json<DefaultRevReq>,
) -> ApiResult<Json<Value>> {
    p.require(p.can_approve(), "既定 revision の変更")?;
    let row = manifests::load_revision(&st.pool, &id, req.revision).await?;
    if row.state != "published" {
        return Err(ApiError::conflict(
            "公開済みの revision にだけ既定を設定できます",
        ));
    }
    sqlx::query("UPDATE manifests SET default_revision = ?, updated_at = ? WHERE id = ?")
        .bind(req.revision)
        .bind(util::now_str())
        .bind(&id)
        .execute(&st.pool)
        .await?;
    crate::audit::record(
        &st.pool,
        &p,
        "manifest.default_revision",
        &id,
        json!({"revision": req.revision}),
    )
    .await?;
    let m = manifests::load_manifest(&st.pool, &id).await?;
    Ok(Json(m.to_json()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRevisionReq {
    /// コピー元 (省略時は最新)
    #[serde(default)]
    from_revision: Option<i64>,
    /// 本文を直接与える (from_revision より優先)
    #[serde(default)]
    body: Option<Value>,
}

async fn create_revision(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    Path(id): Path<String>,
    Json(req): Json<CreateRevisionReq>,
) -> ApiResult<Json<Value>> {
    p.require(p.can_edit(), "下書きの作成")?;
    manifests::load_manifest(&st.pool, &id).await?;
    let revs = revisions_of(&st, &id).await?;
    let next = revs.iter().map(|r| r.revision).max().unwrap_or(0) + 1;
    let base = match req.body {
        Some(b) => b,
        None => {
            let from = match req.from_revision {
                Some(r) => revs
                    .iter()
                    .find(|x| x.revision == r)
                    .ok_or_else(|| ApiError::not_found("コピー元の revision がありません"))?,
                None => revs
                    .last()
                    .ok_or_else(|| ApiError::not_found("revision がありません"))?,
            };
            serde_json::from_str(&from.body)?
        }
    };
    let body = normalize_body(&st, &id, next, base).await?;
    insert_revision(&st, &p, &id, next, &body).await?;
    crate::audit::record(
        &st.pool,
        &p,
        "manifest.revision.create",
        &format!("{id}@{next}"),
        json!({"fromRevision": req.from_revision}),
    )
    .await?;
    Ok(Json(
        manifests::load_revision(&st.pool, &id, next)
            .await?
            .to_json(true),
    ))
}

async fn get_revision(
    State(st): State<AppState>,
    AdminAuth(_p): AdminAuth,
    Path((id, rev)): Path<(String, i64)>,
) -> ApiResult<Json<Value>> {
    Ok(Json(
        manifests::load_revision(&st.pool, &id, rev)
            .await?
            .to_json(true),
    ))
}

#[derive(Deserialize)]
struct UpdateRevisionReq {
    body: Value,
}

/// 下書きの保存。承認済み / 公開済みは変更不可 (新しい下書きを作る)。保存のたびに検証結果も更新する。
async fn update_revision(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    Path((id, rev)): Path<(String, i64)>,
    Json(req): Json<UpdateRevisionReq>,
) -> ApiResult<Json<Value>> {
    p.require(p.can_edit(), "下書きの編集")?;
    let row = manifests::load_revision(&st.pool, &id, rev).await?;
    if row.state != "draft" {
        return Err(ApiError::conflict(format!(
            "revision {rev} は {} のため編集できません。新しい下書きを作ってください",
            row.state
        )));
    }
    let body = normalize_body(&st, &id, rev, req.body).await?;
    let report = manifests::validate_body(&st.pool, &body).await?;
    let text = body.to_string();
    let now = util::now_str();
    sqlx::query(
        "UPDATE manifest_revisions SET body = ?, body_sha256 = ?, validation = ?, updated_by = ?, updated_at = ? WHERE manifest_id = ? AND revision = ?",
    )
    .bind(&text)
    .bind(util::sha256_hex(text.as_bytes()))
    .bind(serde_json::to_string(&report)?)
    .bind(&p.name)
    .bind(&now)
    .bind(&id)
    .bind(rev)
    .execute(&st.pool)
    .await?;
    sqlx::query("UPDATE manifests SET name = COALESCE(?, name), description = COALESCE(?, description), updated_at = ? WHERE id = ? AND default_revision IS NULL")
        .bind(body.get("name").and_then(|v| v.as_str()))
        .bind(body.get("description").and_then(|v| v.as_str()))
        .bind(&now)
        .bind(&id)
        .execute(&st.pool)
        .await?;
    crate::audit::record(
        &st.pool,
        &p,
        "manifest.revision.update",
        &format!("{id}@{rev}"),
        json!({"ok": report.ok, "errors": report.errors.len()}),
    )
    .await?;
    Ok(Json(
        manifests::load_revision(&st.pool, &id, rev)
            .await?
            .to_json(true),
    ))
}

async fn delete_revision(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    Path((id, rev)): Path<(String, i64)>,
) -> ApiResult<Json<Value>> {
    p.require(p.can_edit(), "下書きの削除")?;
    let row = manifests::load_revision(&st.pool, &id, rev).await?;
    if row.state == "published" {
        return Err(ApiError::conflict(
            "公開済みの revision は削除できません (巻き戻しは既定 revision の変更で)",
        ));
    }
    sqlx::query("DELETE FROM manifest_revisions WHERE manifest_id = ? AND revision = ?")
        .bind(&id)
        .bind(rev)
        .execute(&st.pool)
        .await?;
    crate::audit::record(
        &st.pool,
        &p,
        "manifest.revision.delete",
        &format!("{id}@{rev}"),
        json!({}),
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

async fn validate_revision(
    State(st): State<AppState>,
    AdminAuth(_p): AdminAuth,
    Path((id, rev)): Path<(String, i64)>,
) -> ApiResult<Json<Value>> {
    let row = manifests::load_revision(&st.pool, &id, rev).await?;
    let body: Value = serde_json::from_str(&row.body)?;
    let report = manifests::validate_body(&st.pool, &body).await?;
    sqlx::query(
        "UPDATE manifest_revisions SET validation = ? WHERE manifest_id = ? AND revision = ?",
    )
    .bind(serde_json::to_string(&report)?)
    .bind(&id)
    .bind(rev)
    .execute(&st.pool)
    .await?;
    Ok(Json(serde_json::to_value(report)?))
}

/// 保存せずに本文だけ検証する (エディタの「検証」ボタン)。
async fn validate_adhoc(
    State(st): State<AppState>,
    AdminAuth(_p): AdminAuth,
    Json(mut body): Json<Value>,
) -> ApiResult<Json<Value>> {
    manifests::enrich_artifacts(&st.pool, &mut body).await?;
    if let Some(o) = body.as_object_mut() {
        o.remove("approval");
        o.remove("signature");
    }
    let report = manifests::validate_body(&st.pool, &body).await?;
    Ok(Json(json!({ "report": report, "body": body })))
}

#[derive(Deserialize, Default)]
struct ApproveReq {
    #[serde(default)]
    reason: String,
}

async fn approve_revision(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    Path((id, rev)): Path<(String, i64)>,
    body: Option<Json<ApproveReq>>,
) -> ApiResult<Json<Value>> {
    let req = body.map(|b| b.0).unwrap_or_default();
    Ok(Json(
        manifests::approve(&st, &p, &id, rev, &req.reason)
            .await?
            .to_json(true),
    ))
}

async fn publish_revision(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    Path((id, rev)): Path<(String, i64)>,
) -> ApiResult<Json<Value>> {
    Ok(Json(
        manifests::publish(&st, &p, &id, rev).await?.to_json(true),
    ))
}

// ---- ユーザ ----------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    username: String,
    display_name: String,
    roles: String,
    groups_json: String,
    must_change_password: i64,
    disabled: i64,
    created_at: String,
    updated_at: String,
}

fn user_json(u: &UserRow) -> Value {
    json!({
        "id": u.id,
        "username": u.username,
        "displayName": u.display_name,
        "roles": util::json_vec(&u.roles),
        "groups": util::json_vec(&u.groups_json),
        "mustChangePassword": u.must_change_password != 0,
        "disabled": u.disabled != 0,
        "createdAt": u.created_at,
        "updatedAt": u.updated_at,
    })
}

fn check_roles(roles: &[String]) -> ApiResult<()> {
    for r in roles {
        if !auth::ALL_ROLES.contains(&r.as_str()) {
            return Err(ApiError::bad_request(format!("未知のロール `{r}`")));
        }
    }
    Ok(())
}

async fn list_users(State(st): State<AppState>, AdminAuth(p): AdminAuth) -> ApiResult<Json<Value>> {
    p.require(p.is_admin(), "ユーザ一覧")?;
    let rows = sqlx::query_as::<_, UserRow>("SELECT id, username, display_name, roles, groups_json, must_change_password, disabled, created_at, updated_at FROM users ORDER BY username")
        .fetch_all(&st.pool)
        .await?;
    Ok(Json(
        json!({ "items": rows.iter().map(user_json).collect::<Vec<_>>() }),
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateUserReq {
    username: String,
    password: String,
    #[serde(default)]
    display_name: String,
    #[serde(default = "default_roles")]
    roles: Vec<String>,
    #[serde(default)]
    groups: Vec<String>,
}

fn default_roles() -> Vec<String> {
    vec!["user".into()]
}

async fn create_user(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    Json(req): Json<CreateUserReq>,
) -> ApiResult<Json<Value>> {
    p.require(p.is_admin(), "ユーザ作成")?;
    let username = req.username.trim().to_owned();
    if username.len() < 2
        || username.len() > 64
        || !username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-@".contains(c))
    {
        return Err(ApiError::bad_request("ユーザ名は英数字と ._-@、2〜64 文字"));
    }
    if req.password.len() < 10 {
        return Err(ApiError::bad_request("パスワードは 10 文字以上"));
    }
    check_roles(&req.roles)?;
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE username = ?")
        .bind(&username)
        .fetch_optional(&st.pool)
        .await?;
    if exists.is_some() {
        return Err(ApiError::conflict("同名のユーザがいます"));
    }
    let id = util::new_id();
    let now = util::now_str();
    sqlx::query(
        "INSERT INTO users (id, username, display_name, password_hash, roles, groups_json, must_change_password, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?)",
    )
    .bind(&id)
    .bind(&username)
    .bind(&req.display_name)
    .bind(auth::hash_password(&req.password)?)
    .bind(serde_json::to_string(&req.roles)?)
    .bind(serde_json::to_string(&req.groups)?)
    .bind(&now)
    .bind(&now)
    .execute(&st.pool)
    .await?;
    crate::audit::record(
        &st.pool,
        &p,
        "user.create",
        &username,
        json!({"roles": req.roles, "groups": req.groups}),
    )
    .await?;
    let row = sqlx::query_as::<_, UserRow>("SELECT id, username, display_name, roles, groups_json, must_change_password, disabled, created_at, updated_at FROM users WHERE id = ?")
        .bind(&id)
        .fetch_one(&st.pool)
        .await?;
    Ok(Json(user_json(&row)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateUserReq {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    roles: Option<Vec<String>>,
    #[serde(default)]
    groups: Option<Vec<String>>,
    #[serde(default)]
    disabled: Option<bool>,
}

async fn update_user(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserReq>,
) -> ApiResult<Json<Value>> {
    p.require(p.is_admin(), "ユーザ変更")?;
    if let Some(r) = &req.roles {
        check_roles(r)?;
        // 自分から admin を外して管理者が 0 人になる事故を防ぐ
        if id == p.id && !r.iter().any(|x| x == "admin") {
            return Err(ApiError::bad_request("自分の admin ロールは外せません"));
        }
    }
    if req.disabled == Some(true) && id == p.id {
        return Err(ApiError::bad_request("自分を無効化することはできません"));
    }
    let now = util::now_str();
    sqlx::query(
        "UPDATE users SET display_name = COALESCE(?, display_name), roles = COALESCE(?, roles), groups_json = COALESCE(?, groups_json), disabled = COALESCE(?, disabled), updated_at = ? WHERE id = ?",
    )
    .bind(&req.display_name)
    .bind(req.roles.as_ref().map(|r| serde_json::to_string(r).unwrap()))
    .bind(req.groups.as_ref().map(|g| serde_json::to_string(g).unwrap()))
    .bind(req.disabled.map(|d| d as i64))
    .bind(&now)
    .bind(&id)
    .execute(&st.pool)
    .await?;
    if req.disabled == Some(true) {
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(&id)
            .execute(&st.pool)
            .await?;
    }
    let row = sqlx::query_as::<_, UserRow>("SELECT id, username, display_name, roles, groups_json, must_change_password, disabled, created_at, updated_at FROM users WHERE id = ?")
        .bind(&id)
        .fetch_optional(&st.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("ユーザがいません"))?;
    crate::audit::record(
        &st.pool,
        &p,
        "user.update",
        &row.username,
        json!({"roles": req.roles, "groups": req.groups, "disabled": req.disabled}),
    )
    .await?;
    Ok(Json(user_json(&row)))
}

#[derive(Deserialize)]
struct ResetPasswordReq {
    password: String,
}

async fn reset_password(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    Path(id): Path<String>,
    Json(req): Json<ResetPasswordReq>,
) -> ApiResult<Json<Value>> {
    p.require(p.is_admin(), "パスワード再設定")?;
    if req.password.len() < 10 {
        return Err(ApiError::bad_request("パスワードは 10 文字以上"));
    }
    let res = sqlx::query(
        "UPDATE users SET password_hash = ?, must_change_password = 1, updated_at = ? WHERE id = ?",
    )
    .bind(auth::hash_password(&req.password)?)
    .bind(util::now_str())
    .bind(&id)
    .execute(&st.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("ユーザがいません"));
    }
    sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(&id)
        .execute(&st.pool)
        .await?;
    crate::audit::record(&st.pool, &p, "user.reset_password", &id, json!({})).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn list_groups(
    State(st): State<AppState>,
    AdminAuth(_p): AdminAuth,
) -> ApiResult<Json<Value>> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT groups_json FROM users")
        .fetch_all(&st.pool)
        .await?;
    let mrows: Vec<(String,)> = sqlx::query_as("SELECT visibility_groups FROM manifests")
        .fetch_all(&st.pool)
        .await?;
    let mut set = std::collections::BTreeSet::new();
    for (g,) in rows.iter().chain(mrows.iter()) {
        set.extend(util::json_vec(g));
    }
    Ok(Json(json!({ "items": set })))
}

// ---- API キー ----------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: String,
    name: String,
    prefix: String,
    allowed_manifests: String,
    created_by: String,
    created_at: String,
    revoked_at: Option<String>,
    last_used_at: Option<String>,
}

fn api_key_json(k: &ApiKeyRow) -> Value {
    json!({
        "id": k.id, "name": k.name, "prefix": k.prefix,
        "allowedManifests": util::json_vec(&k.allowed_manifests),
        "createdBy": k.created_by, "createdAt": k.created_at,
        "revokedAt": k.revoked_at, "lastUsedAt": k.last_used_at,
    })
}

async fn list_api_keys(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
) -> ApiResult<Json<Value>> {
    p.require(p.is_admin(), "API キー一覧")?;
    let rows = sqlx::query_as::<_, ApiKeyRow>("SELECT id, name, prefix, allowed_manifests, created_by, created_at, revoked_at, last_used_at FROM api_keys ORDER BY created_at DESC")
        .fetch_all(&st.pool)
        .await?;
    Ok(Json(
        json!({ "items": rows.iter().map(api_key_json).collect::<Vec<_>>() }),
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateApiKeyReq {
    name: String,
    #[serde(default = "default_allowed")]
    allowed_manifests: Vec<String>,
}

fn default_allowed() -> Vec<String> {
    vec!["*".into()]
}

/// 平文のキーは作成時の応答にだけ含める。
async fn create_api_key(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    Json(req): Json<CreateApiKeyReq>,
) -> ApiResult<Json<Value>> {
    p.require(p.is_admin(), "API キー発行")?;
    if req.name.trim().is_empty() {
        return Err(ApiError::bad_request("name が必要です"));
    }
    let key = format!("ipk_{}", util::random_token());
    let id = util::new_id();
    let prefix = key[..12].to_owned();
    sqlx::query("INSERT INTO api_keys (id, name, key_hash, prefix, allowed_manifests, created_by, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(req.name.trim())
        .bind(util::token_hash(&key))
        .bind(&prefix)
        .bind(serde_json::to_string(&req.allowed_manifests)?)
        .bind(&p.name)
        .bind(util::now_str())
        .execute(&st.pool)
        .await?;
    crate::audit::record(
        &st.pool,
        &p,
        "api_key.create",
        &id,
        json!({"name": req.name, "allowedManifests": req.allowed_manifests}),
    )
    .await?;
    Ok(Json(
        json!({ "id": id, "name": req.name.trim(), "prefix": prefix, "key": key, "allowedManifests": req.allowed_manifests }),
    ))
}

async fn revoke_api_key(
    State(st): State<AppState>,
    AdminAuth(p): AdminAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    p.require(p.is_admin(), "API キー失効")?;
    let res = sqlx::query("UPDATE api_keys SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
        .bind(util::now_str())
        .bind(&id)
        .execute(&st.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("API キーがないか、既に失効しています"));
    }
    crate::audit::record(&st.pool, &p, "api_key.revoke", &id, json!({})).await?;
    Ok(Json(json!({ "ok": true })))
}

// ---- 監査 / 起動履歴 / クライアント ------------------------------------------

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: String,
    at: String,
    actor_id: String,
    actor_name: String,
    action: String,
    target: String,
    detail: Option<String>,
}

async fn list_audit(
    State(st): State<AppState>,
    AdminAuth(_p): AdminAuth,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Value>> {
    let rows = sqlx::query_as::<_, AuditRow>("SELECT id, at, actor_id, actor_name, action, target, detail FROM audit ORDER BY at DESC LIMIT ?")
        .bind(q.limit.unwrap_or(200).clamp(1, 1000))
        .fetch_all(&st.pool)
        .await?;
    Ok(Json(json!({ "items": rows.iter().map(|r| json!({
        "id": r.id, "at": r.at, "actorId": r.actor_id, "actorName": r.actor_name, "action": r.action, "target": r.target,
        "detail": r.detail.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
    })).collect::<Vec<_>>() })))
}

async fn list_launches(
    State(st): State<AppState>,
    AdminAuth(_p): AdminAuth,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Value>> {
    let rows =
        sqlx::query_as::<_, LaunchRow>("SELECT * FROM launches ORDER BY started_at DESC LIMIT ?")
            .bind(q.limit.unwrap_or(200).clamp(1, 1000))
            .fetch_all(&st.pool)
            .await?;
    Ok(Json(
        json!({ "items": rows.iter().map(LaunchRow::to_json).collect::<Vec<_>>() }),
    ))
}

#[derive(sqlx::FromRow)]
struct ClientRow {
    id: String,
    client_id: String,
    principal_kind: String,
    principal_name: String,
    engine_version: String,
    wit_versions: String,
    os: String,
    arch: String,
    first_seen: String,
    last_seen: String,
    hello_count: i64,
}

async fn list_clients(
    State(st): State<AppState>,
    AdminAuth(_p): AdminAuth,
) -> ApiResult<Json<Value>> {
    let rows = sqlx::query_as::<_, ClientRow>("SELECT id, client_id, principal_kind, principal_name, engine_version, wit_versions, os, arch, first_seen, last_seen, hello_count FROM clients ORDER BY last_seen DESC LIMIT 500")
        .fetch_all(&st.pool)
        .await?;
    // 起動中 (telemetry で終了が来ていない) の構成を principal ごとに添える
    let running = sqlx::query_as::<_, LaunchRow>("SELECT * FROM launches WHERE ended_at IS NULL AND outcome IN ('started', 'running') ORDER BY started_at DESC LIMIT 500")
        .fetch_all(&st.pool)
        .await?;
    Ok(Json(json!({ "items": rows.iter().map(|c| json!({
        "id": c.id, "clientId": c.client_id, "principalKind": c.principal_kind, "principalName": c.principal_name,
        "engineVersion": c.engine_version, "witVersions": util::json_vec(&c.wit_versions), "os": c.os, "arch": c.arch,
        "firstSeen": c.first_seen, "lastSeen": c.last_seen, "helloCount": c.hello_count,
        "running": running.iter().filter(|l| l.principal_name == c.principal_name).map(|l| json!({"manifestId": l.manifest_id, "revision": l.revision, "startedAt": l.started_at})).collect::<Vec<_>>(),
    })).collect::<Vec<_>>() })))
}
