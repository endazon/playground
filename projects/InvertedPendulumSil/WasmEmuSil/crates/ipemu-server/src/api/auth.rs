//! `/api/auth`: ログイン / ログアウト / 自分の情報 / パスワード変更。ポータルと管理画面の共通入口。

use crate::auth::{self, Auth, ClientIp, UserAuth};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use crate::util;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/password", post(change_password))
}

#[derive(Deserialize)]
struct LoginReq {
    username: String,
    password: String,
}

async fn login(
    State(st): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(req): Json<LoginReq>,
) -> ApiResult<Json<Value>> {
    let username = req.username.trim().to_owned();
    // 抑止の鍵は (ユーザ名, 送信元 IP)。ユーザ名単独は緩い閾値 (D44)
    st.throttle.check(&username, ip)?;
    let row: Option<(String, String, i64)> =
        sqlx::query_as("SELECT id, password_hash, disabled FROM users WHERE username = ?")
            .bind(&username)
            .fetch_optional(&st.pool)
            .await?;
    // ユーザの有無で応答時間が変わらないよう、無いときもダミーのハッシュを検証する
    let (ok, user_id) = match &row {
        Some((id, hash, disabled)) => (
            *disabled == 0 && auth::verify_password(&req.password, hash),
            id.clone(),
        ),
        None => {
            let _ = auth::verify_password(&req.password, DUMMY_HASH);
            (false, String::new())
        }
    };
    st.throttle.record(&username, ip, ok);
    if !ok {
        return Err(ApiError::unauthorized("ユーザ名かパスワードが違います"));
    }
    let (token, expires) =
        auth::create_session(&st.pool, &user_id, "login", st.config.session_ttl_secs).await?;
    let p = auth::authenticate(&st.pool, &token).await?;
    crate::audit::record(&st.pool, &p, "auth.login", &p.name, json!({})).await?;
    Ok(Json(
        json!({ "token": token, "expiresAt": expires, "principal": p }),
    ))
}

// argon2id のダミーハッシュ (タイミング均し用)。パスワードは "unused"
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21lc2FsdA$6jMgm4pT7C7m0N6fS1F7CaK0wQ6tE1CqHfJ8Zc0gE0E";

async fn logout(
    State(st): State<AppState>,
    headers: axum::http::HeaderMap,
    Auth(p): Auth,
) -> ApiResult<Json<Value>> {
    if let Some(tok) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(util::token_hash(tok.trim()))
            .execute(&st.pool)
            .await?;
    }
    let _ = p;
    Ok(Json(json!({ "ok": true })))
}

async fn me(Auth(p): Auth) -> Json<Value> {
    Json(json!({ "principal": p }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PasswordReq {
    current_password: String,
    new_password: String,
}

async fn change_password(
    State(st): State<AppState>,
    UserAuth(p): UserAuth,
    Json(req): Json<PasswordReq>,
) -> ApiResult<Json<Value>> {
    if req.new_password.len() < 10 {
        return Err(ApiError::bad_request("新しいパスワードは 10 文字以上"));
    }
    let (hash,): (String,) = sqlx::query_as("SELECT password_hash FROM users WHERE id = ?")
        .bind(&p.id)
        .fetch_one(&st.pool)
        .await?;
    if !auth::verify_password(&req.current_password, &hash) {
        return Err(ApiError::unauthorized("現在のパスワードが違います"));
    }
    let new_hash = auth::hash_password(&req.new_password)?;
    sqlx::query(
        "UPDATE users SET password_hash = ?, must_change_password = 0, updated_at = ? WHERE id = ?",
    )
    .bind(&new_hash)
    .bind(util::now_str())
    .bind(&p.id)
    .execute(&st.pool)
    .await?;
    crate::audit::record(&st.pool, &p, "auth.password_change", &p.name, json!({})).await?;
    Ok(Json(json!({ "ok": true })))
}
