//! 配信 API `/api/v1`。すべて読み取り専用 (telemetry / hello / チケット発行を除く)。

use crate::auth::{Auth, UserAuth};
use crate::error::{ApiError, ApiResult};
use crate::manifests::{self, ManifestRow, RevisionRow};
use crate::state::AppState;
use crate::util;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/catalog", get(catalog))
        .route("/keys", get(keys))
        .route("/launch-tickets", post(create_ticket))
        .route("/launch-tickets/{ticket}/exchange", post(exchange_ticket))
        .route("/manifests/{id}", get(get_manifest))
        .route("/artifacts/by-hash/{sha256}", get(artifact_by_hash))
        .route("/artifacts/{name}/{version}", get(artifact_by_name))
        .route("/telemetry", post(telemetry))
        .route("/clients/hello", post(hello))
        .route("/launches/mine", get(my_launches))
}

/// 公開鍵の一覧。信頼の根拠はクライアント同梱 / 設定の鍵であり、これは確認・開発用の参考情報。
async fn keys(State(st): State<AppState>) -> Json<Value> {
    Json(json!({ "keys": [st.public_key_entry()] }))
}

/// ログイン中の利用者 (または API キー) に公開された構成。
async fn catalog(State(st): State<AppState>, Auth(p): Auth) -> ApiResult<Json<Value>> {
    let rows = sqlx::query_as::<_, ManifestRow>("SELECT * FROM manifests ORDER BY id")
        .fetch_all(&st.pool)
        .await?;
    let mut items = Vec::new();
    for m in rows.into_iter().filter(|m| m.visible_to(&p)) {
        let revs = sqlx::query_as::<_, RevisionRow>(
            "SELECT * FROM manifest_revisions WHERE manifest_id = ? ORDER BY revision",
        )
        .bind(&m.id)
        .fetch_all(&st.pool)
        .await?;
        // editor 以上は未公開 revision も「検証用に起動」できる (署名なし)
        let show_unpublished = p.can_edit();
        let visible: Vec<&RevisionRow> = revs
            .iter()
            .filter(|r| r.state == "published" || show_unpublished)
            .collect();
        if visible.iter().all(|r| r.state != "published") && !show_unpublished {
            continue;
        }
        let default_rev = m.default_revision;
        let default_body = default_rev
            .and_then(|d| revs.iter().find(|r| r.revision == d))
            .and_then(|r| serde_json::from_str::<Value>(&r.body).ok());
        items.push(json!({
            "id": m.id,
            "name": m.name,
            "description": m.description,
            "defaultRevision": default_rev,
            "revisions": visible.iter().map(|r| json!({
                "revision": r.revision,
                "state": r.state,
                "name": r.name(),
                "publishedAt": r.published_at,
                "approvedBy": r.approved_by,
                "signed": r.signed_body.is_some(),
            })).collect::<Vec<_>>(),
            "requiresCapabilities": default_body.as_ref().map(manifests::capability_summary).unwrap_or_default(),
            "updatedAt": m.updated_at,
        }));
    }
    Ok(Json(json!({ "items": items })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TicketReq {
    manifest_id: String,
    #[serde(default)]
    revision: Option<i64>,
}

/// 外部ブラウザからの `ipemu://` 起動用チケット (60 秒・単回)。
async fn create_ticket(
    State(st): State<AppState>,
    UserAuth(p): UserAuth,
    Json(req): Json<TicketReq>,
) -> ApiResult<Json<Value>> {
    let m = manifests::load_manifest(&st.pool, &req.manifest_id).await?;
    if !m.visible_to(&p) {
        return Err(ApiError::not_found("構成がありません"));
    }
    let ticket = format!("tk_{}", util::random_token());
    let expires = util::after_secs(st.config.ticket_ttl_secs);
    sqlx::query(
        "INSERT INTO launch_tickets (ticket_hash, user_id, manifest_id, revision, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(util::token_hash(&ticket))
    .bind(&p.id)
    .bind(&req.manifest_id)
    .bind(req.revision)
    .bind(util::now_str())
    .bind(&expires)
    .execute(&st.pool)
    .await?;
    let rev = req
        .revision
        .map(|r| r.to_string())
        .unwrap_or_else(|| "default".into());
    let launch_url = format!(
        "ipemu://launch?server={}&manifest={}&rev={}&ticket={}",
        urlencode(&st.config.public_url),
        urlencode(&req.manifest_id),
        rev,
        ticket
    );
    Ok(Json(
        json!({ "ticket": ticket, "expiresAt": expires, "launchUrl": launch_url }),
    ))
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// チケット → 短命トークン。1 回しか成功しない。
async fn exchange_ticket(
    State(st): State<AppState>,
    Path(ticket): Path<String>,
) -> ApiResult<Json<Value>> {
    let hash = util::token_hash(&ticket);
    let now = util::now_str();
    // used_at を条件に含めた UPDATE で単回性を DB の原子性に任せる
    let res = sqlx::query(
        "UPDATE launch_tickets SET used_at = ? WHERE ticket_hash = ? AND used_at IS NULL AND expires_at > ?",
    )
    .bind(&now)
    .bind(&hash)
    .bind(&now)
    .execute(&st.pool)
    .await?;
    if res.rows_affected() != 1 {
        return Err(ApiError::new(
            StatusCode::GONE,
            "ticket_invalid",
            "チケットは無効か、失効済みか、使用済みです",
        ));
    }
    let (user_id, manifest_id, revision): (String, String, Option<i64>) = sqlx::query_as(
        "SELECT user_id, manifest_id, revision FROM launch_tickets WHERE ticket_hash = ?",
    )
    .bind(&hash)
    .fetch_one(&st.pool)
    .await?;
    let (token, expires) = crate::auth::create_session(
        &st.pool,
        &user_id,
        "launch",
        st.config.launch_token_ttl_secs,
    )
    .await?;
    Ok(Json(
        json!({ "token": token, "expiresAt": expires, "manifestId": manifest_id, "revision": revision }),
    ))
}

#[derive(Deserialize)]
struct RevQuery {
    #[serde(default)]
    rev: Option<String>,
}

/// 署名付き manifest。公開範囲外は 404 (存在を漏らさない)。取得は起動として記録する。
async fn get_manifest(
    State(st): State<AppState>,
    Auth(p): Auth,
    Path(id): Path<String>,
    Query(q): Query<RevQuery>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let m = match manifests::load_manifest(&st.pool, &id).await {
        Ok(m) if m.visible_to(&p) => m,
        _ => return Err(ApiError::not_found("構成がありません")),
    };
    let rev = match q.rev.as_deref() {
        None | Some("default") | Some("") => m
            .default_revision
            .ok_or_else(|| ApiError::not_found("公開済みの revision がありません"))?,
        Some(s) => s
            .parse::<i64>()
            .map_err(|_| ApiError::bad_request("rev は整数か default"))?,
    };
    let row = manifests::load_revision(&st.pool, &id, rev)
        .await
        .map_err(|_| ApiError::not_found("構成がありません"))?;
    let (body_text, signed) = match (&row.signed_body, row.state.as_str()) {
        (Some(s), "published") => (s.clone(), true),
        // 未公開 revision は editor 以上にだけ、署名なしで返す (検証用の起動)
        _ if p.can_edit() => (row.body.clone(), false),
        _ => return Err(ApiError::not_found("構成がありません")),
    };
    let etag = format!("\"{}\"", util::sha256_hex(body_text.as_bytes()));
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|e| e.trim() == etag))
    {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }

    // 起動履歴
    let launch_id = util::new_id();
    let client_info = headers
        .get("x-ipemu-client")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| serde_json::from_str::<Value>(s).ok());
    sqlx::query(
        "INSERT INTO launches (id, manifest_id, revision, principal_kind, principal_id, principal_name, credential_kind, client_info, started_at, outcome)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'fetched')",
    )
    .bind(&launch_id)
    .bind(&id)
    .bind(rev)
    .bind(p.kind.as_str())
    .bind(&p.id)
    .bind(&p.name)
    .bind(p.credential.as_str())
    .bind(client_info.map(|v| v.to_string()))
    .bind(util::now_str())
    .execute(&st.pool)
    .await?;

    let mut resp = Response::new(Body::from(body_text));
    let h = resp.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    h.insert(header::ETAG, HeaderValue::from_str(&etag).unwrap());
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-cache"),
    );
    h.insert(
        "x-ipemu-launch-id",
        HeaderValue::from_str(&launch_id).unwrap(),
    );
    h.insert(
        "x-ipemu-signed",
        HeaderValue::from_static(if signed { "true" } else { "false" }),
    );
    h.insert(
        "x-ipemu-revision",
        HeaderValue::from_str(&rev.to_string()).unwrap(),
    );
    Ok(resp)
}

async fn artifact_by_name(
    State(st): State<AppState>,
    Auth(_p): Auth,
    Path((name, version)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let Some(row) = manifests::load_artifact(&st.pool, &format!("{name}@{version}")).await? else {
        return Err(ApiError::not_found("アーティファクトがありません"));
    };
    serve_artifact(&st, &row.sha256, &row.content_type, &headers)
}

async fn artifact_by_hash(
    State(st): State<AppState>,
    Auth(_p): Auth,
    Path(sha256): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    if sha256.len() != 64 || !sha256.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::bad_request("sha256 は 64 桁 hex"));
    }
    let ct: Option<(String,)> =
        sqlx::query_as("SELECT content_type FROM artifacts WHERE sha256 = ? LIMIT 1")
            .bind(&sha256)
            .fetch_optional(&st.pool)
            .await?;
    let Some((ct,)) = ct else {
        return Err(ApiError::not_found("アーティファクトがありません"));
    };
    serve_artifact(&st, &sha256, &ct, &headers)
}

/// ETag = sha256、immutable、If-None-Match → 304、Content-Digest。
fn serve_artifact(
    st: &AppState,
    sha256: &str,
    content_type: &str,
    headers: &HeaderMap,
) -> ApiResult<Response> {
    use base64::Engine as _;
    let etag = format!("\"{sha256}\"");
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|e| e.trim() == etag || e.trim() == "*"))
    {
        let mut r = StatusCode::NOT_MODIFIED.into_response();
        r.headers_mut()
            .insert(header::ETAG, HeaderValue::from_str(&etag).unwrap());
        return Ok(r);
    }
    let Some(bytes) = st.store.get(sha256)? else {
        return Err(ApiError::internal(format!(
            "保存されたはずのアーティファクト {sha256} が見つかりません"
        )));
    };
    let digest = base64::engine::general_purpose::STANDARD.encode(hex::decode(sha256).unwrap());
    let mut resp = Response::new(Body::from(bytes));
    let h = resp.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    h.insert(header::ETAG, HeaderValue::from_str(&etag).unwrap());
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=31536000, immutable"),
    );
    h.insert(
        "content-digest",
        HeaderValue::from_str(&format!("sha-256=:{digest}:")).unwrap(),
    );
    Ok(resp)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TelemetryReq {
    #[serde(default)]
    launch_id: Option<String>,
    manifest_id: String,
    revision: i64,
    /// started / stopped / fault / ...
    event: String,
    #[serde(default)]
    detail: Value,
}

/// 実行要約。launch_id があれば起動履歴の終わりを埋める。
async fn telemetry(
    State(st): State<AppState>,
    Auth(p): Auth,
    Json(req): Json<TelemetryReq>,
) -> ApiResult<Json<Value>> {
    let now = util::now_str();
    sqlx::query(
        "INSERT INTO telemetry (id, launch_id, principal_kind, principal_id, manifest_id, revision, event, detail, received_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(util::new_id())
    .bind(&req.launch_id)
    .bind(p.kind.as_str())
    .bind(&p.id)
    .bind(&req.manifest_id)
    .bind(req.revision)
    .bind(&req.event)
    .bind(req.detail.to_string())
    .bind(&now)
    .execute(&st.pool)
    .await?;
    if let Some(lid) = &req.launch_id {
        // 自分の起動だけ更新できる
        let ended = matches!(
            req.event.as_str(),
            "stopped" | "fault" | "error" | "aborted"
        );
        sqlx::query(
            "UPDATE launches SET outcome = ?, detail = ?, ended_at = CASE WHEN ? THEN ? ELSE ended_at END
             WHERE id = ? AND principal_kind = ? AND principal_id = ?",
        )
        .bind(&req.event)
        .bind(req.detail.to_string())
        .bind(ended)
        .bind(&now)
        .bind(lid)
        .bind(p.kind.as_str())
        .bind(&p.id)
        .execute(&st.pool)
        .await?;
    }
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HelloReq {
    #[serde(default)]
    client_id: Option<String>,
    engine_version: String,
    #[serde(default)]
    wit_versions: Vec<String>,
    os: String,
    arch: String,
}

async fn hello(
    State(st): State<AppState>,
    Auth(p): Auth,
    Json(req): Json<HelloReq>,
) -> ApiResult<Json<Value>> {
    let client_id = req
        .client_id
        .clone()
        .unwrap_or_else(|| format!("{}-{}-{}", p.id, req.os, req.arch));
    let now = util::now_str();
    sqlx::query(
        "INSERT INTO clients (id, client_id, principal_kind, principal_id, principal_name, engine_version, wit_versions, os, arch, first_seen, last_seen, hello_count)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1)
         ON CONFLICT(client_id, principal_kind, principal_id) DO UPDATE SET
           engine_version = excluded.engine_version, wit_versions = excluded.wit_versions, os = excluded.os, arch = excluded.arch,
           principal_name = excluded.principal_name, last_seen = excluded.last_seen, hello_count = hello_count + 1",
    )
    .bind(util::new_id())
    .bind(&client_id)
    .bind(p.kind.as_str())
    .bind(&p.id)
    .bind(&p.name)
    .bind(&req.engine_version)
    .bind(serde_json::to_string(&req.wit_versions)?)
    .bind(&req.os)
    .bind(&req.arch)
    .bind(&now)
    .bind(&now)
    .execute(&st.pool)
    .await?;
    Ok(Json(
        json!({ "ok": true, "serverTime": now, "keys": [st.public_key_entry()] }),
    ))
}

/// 自分の起動履歴 (ポータル)。
async fn my_launches(State(st): State<AppState>, Auth(p): Auth) -> ApiResult<Json<Value>> {
    let rows = sqlx::query_as::<_, LaunchRow>(
        "SELECT * FROM launches WHERE principal_kind = ? AND principal_id = ? ORDER BY started_at DESC LIMIT 100",
    )
    .bind(p.kind.as_str())
    .bind(&p.id)
    .fetch_all(&st.pool)
    .await?;
    Ok(Json(
        json!({ "items": rows.iter().map(LaunchRow::to_json).collect::<Vec<_>>() }),
    ))
}

#[derive(sqlx::FromRow)]
pub struct LaunchRow {
    pub id: String,
    pub manifest_id: String,
    pub revision: i64,
    pub principal_kind: String,
    pub principal_id: String,
    pub principal_name: String,
    pub credential_kind: String,
    pub client_info: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub outcome: Option<String>,
    pub detail: Option<String>,
}

impl LaunchRow {
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "manifestId": self.manifest_id,
            "revision": self.revision,
            "principalKind": self.principal_kind,
            "principalName": self.principal_name,
            "credentialKind": self.credential_kind,
            "clientInfo": self.client_info.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
            "startedAt": self.started_at,
            "endedAt": self.ended_at,
            "outcome": self.outcome,
            "detail": self.detail.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
        })
    }
}
