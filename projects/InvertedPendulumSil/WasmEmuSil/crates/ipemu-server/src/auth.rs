//! 認証と認可。
//!
//! - パスワードは argon2id。
//! - 認証はすべて `Authorization: Bearer <token>` (D29: Cookie を使わないので CSRF 対策が不要。
//!   トークンはポータルの SPA がメモリ / sessionStorage に持ち、シェル経由でエンジンに渡す)。
//! - トークンの種別: ログインセッション (`sess_`)、チケット交換で得た短命トークン (`lt_`)、
//!   クライアント API キー (`ipk_`)。DB にはハッシュだけを置く。

use crate::error::ApiError;
use crate::state::AppState;
use crate::util;
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

pub const ALL_ROLES: [&str; 5] = ["user", "viewer", "editor", "approver", "admin"];

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut rand::rngs::OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("argon2: {e}"))?
        .to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(h) => Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok(),
        Err(_) => false,
    }
}

/// 認証済みの主体。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Principal {
    pub kind: PrincipalKind,
    pub id: String,
    pub name: String,
    /// user のとき: 持っているロール。api_key のときは空
    pub roles: Vec<String>,
    /// user のとき: 所属グループ
    pub groups: Vec<String>,
    /// api_key のとき: 取得を許す manifest id ("*" で全部)
    pub allowed_manifests: Vec<String>,
    pub credential: CredentialKind,
    pub must_change_password: bool,
    /// セッションの発行時刻 (ログイン時刻)。破壊的な管理操作は発行から 1 時間以内のトークンにだけ許す (D36)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issued_at: Option<String>,
}

/// 破壊的な管理操作 (承認・公開・公開範囲・鍵・ユーザ) に要求する「ログインからの経過時間」の上限。
pub const FRESH_LOGIN_MAX_SECS: i64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalKind {
    User,
    ApiKey,
}

impl PrincipalKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PrincipalKind::User => "user",
            PrincipalKind::ApiKey => "api_key",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialKind {
    Session,
    Ticket,
    ApiKey,
}

impl CredentialKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            CredentialKind::Session => "session",
            CredentialKind::Ticket => "ticket",
            CredentialKind::ApiKey => "api_key",
        }
    }
}

impl Principal {
    pub fn is_user(&self) -> bool {
        self.kind == PrincipalKind::User
    }
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
    pub fn has_any(&self, roles: &[&str]) -> bool {
        roles.iter().any(|r| self.has_role(r))
    }
    /// 管理画面を見られるか
    pub fn can_view_admin(&self) -> bool {
        self.has_any(&["viewer", "editor", "approver", "admin"])
    }
    pub fn can_edit(&self) -> bool {
        self.has_any(&["editor", "admin"])
    }
    pub fn can_approve(&self) -> bool {
        self.has_any(&["approver", "admin"])
    }
    pub fn is_admin(&self) -> bool {
        self.has_role("admin")
    }
    pub fn require(&self, ok: bool, what: &str) -> Result<(), ApiError> {
        if ok {
            Ok(())
        } else {
            Err(ApiError::forbidden(format!("{what} の権限がありません")))
        }
    }
    /// ログインから `FRESH_LOGIN_MAX_SECS` 以内か。Bearer を sessionStorage に置く構成の緩和策で、
    /// 盗まれた古いトークンで承認・公開・鍵・ユーザ操作ができないようにする (D36)
    pub fn require_fresh_login(&self) -> Result<(), ApiError> {
        let fresh = self
            .issued_at
            .as_deref()
            .and_then(util::parse)
            .is_some_and(|t| (util::now() - t).whole_seconds() <= FRESH_LOGIN_MAX_SECS);
        if fresh {
            Ok(())
        } else {
            Err(ApiError::new(
                axum::http::StatusCode::FORBIDDEN,
                "reauth_required",
                "この操作にはログインから 1 時間以内のセッションが必要です。再ログインしてください",
            ))
        }
    }
    /// API キーがその manifest を取れるか (user は公開範囲で別途判定する)
    pub fn api_key_allows(&self, manifest_id: &str) -> bool {
        self.allowed_manifests
            .iter()
            .any(|m| m == "*" || m == manifest_id)
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    username: String,
    roles: String,
    groups_json: String,
    must_change_password: i64,
    disabled: i64,
}

/// Bearer トークンから主体を引く。
pub async fn authenticate(pool: &SqlitePool, token: &str) -> Result<Principal, ApiError> {
    let hash = util::token_hash(token);
    if token.starts_with("ipk_") {
        let row: Option<(String, String, String, Option<String>)> = sqlx::query_as(
            "SELECT id, name, allowed_manifests, revoked_at FROM api_keys WHERE key_hash = ?",
        )
        .bind(&hash)
        .fetch_optional(pool)
        .await?;
        let Some((id, name, allowed, revoked)) = row else {
            return Err(ApiError::unauthorized("API キーが無効です"));
        };
        if revoked.is_some() {
            return Err(ApiError::unauthorized("API キーは失効しています"));
        }
        sqlx::query("UPDATE api_keys SET last_used_at = ? WHERE id = ?")
            .bind(util::now_str())
            .bind(&id)
            .execute(pool)
            .await?;
        return Ok(Principal {
            kind: PrincipalKind::ApiKey,
            id,
            name,
            roles: vec![],
            groups: vec![],
            allowed_manifests: util::json_vec(&allowed),
            credential: CredentialKind::ApiKey,
            must_change_password: false,
            issued_at: None,
        });
    }

    let row: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT user_id, kind, expires_at, created_at FROM sessions WHERE token_hash = ?",
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await?;
    let Some((user_id, kind, expires_at, created_at)) = row else {
        return Err(ApiError::unauthorized("トークンが無効です"));
    };
    if util::is_expired(&expires_at) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(&hash)
            .execute(pool)
            .await?;
        return Err(ApiError::unauthorized("トークンの期限が切れています"));
    }
    let user: Option<UserRow> = sqlx::query_as(
        "SELECT id, username, roles, groups_json, must_change_password, disabled FROM users WHERE id = ?",
    )
    .bind(&user_id)
    .fetch_optional(pool)
    .await?;
    let Some(user) = user else {
        return Err(ApiError::unauthorized("ユーザが存在しません"));
    };
    if user.disabled != 0 {
        return Err(ApiError::unauthorized("ユーザは無効化されています"));
    }
    sqlx::query("UPDATE sessions SET last_used_at = ? WHERE token_hash = ?")
        .bind(util::now_str())
        .bind(&hash)
        .execute(pool)
        .await?;
    Ok(Principal {
        kind: PrincipalKind::User,
        id: user.id,
        name: user.username,
        roles: util::json_vec(&user.roles),
        groups: util::json_vec(&user.groups_json),
        allowed_manifests: vec![],
        credential: if kind == "launch" {
            CredentialKind::Ticket
        } else {
            CredentialKind::Session
        },
        must_change_password: user.must_change_password != 0,
        issued_at: Some(created_at),
    })
}

/// セッション行を作り、平文トークンを返す。
pub async fn create_session(
    pool: &SqlitePool,
    user_id: &str,
    kind: &str,
    ttl_secs: i64,
) -> Result<(String, String), ApiError> {
    let token = format!(
        "{}{}",
        if kind == "launch" { "lt_" } else { "sess_" },
        util::random_token()
    );
    let expires = util::after_secs(ttl_secs);
    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, kind, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(util::token_hash(&token))
    .bind(user_id)
    .bind(kind)
    .bind(util::now_str())
    .bind(&expires)
    .execute(pool)
    .await?;
    Ok((token, expires))
}

fn bearer(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.strip_prefix("Bearer ")
                .or_else(|| v.strip_prefix("bearer "))
        })
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

/// 認証必須の extractor。
pub struct Auth(pub Principal);

impl FromRequestParts<AppState> for Auth {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let token = bearer(parts)
            .ok_or_else(|| ApiError::unauthorized("Authorization: Bearer が必要です"))?;
        Ok(Auth(authenticate(&state.pool, &token).await?))
    }
}

/// 利用者 (ユーザ) 必須。API キーは拒否する。
pub struct UserAuth(pub Principal);

impl FromRequestParts<AppState> for UserAuth {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let Auth(p) = Auth::from_request_parts(parts, state).await?;
        if !p.is_user() {
            return Err(ApiError::forbidden(
                "この操作は利用者のセッションが必要です",
            ));
        }
        Ok(UserAuth(p))
    }
}

/// 管理画面 (閲覧以上) 必須。
pub struct AdminAuth(pub Principal);

impl FromRequestParts<AppState> for AdminAuth {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let UserAuth(p) = UserAuth::from_request_parts(parts, state).await?;
        if !p.can_view_admin() {
            return Err(ApiError::forbidden("管理画面の権限がありません"));
        }
        Ok(AdminAuth(p))
    }
}

/// ログイン失敗の抑止 (ユーザ名ごとに連続失敗 5 回で 30 秒ロック)。メモリ上なのでプロセス再起動で消える。
#[derive(Default)]
pub struct LoginThrottle {
    inner: std::sync::Mutex<std::collections::HashMap<String, (u32, std::time::Instant)>>,
}

impl LoginThrottle {
    pub fn check(&self, username: &str) -> Result<(), ApiError> {
        let map = self.inner.lock().unwrap();
        if let Some((n, since)) = map.get(username) {
            if *n >= 5 && since.elapsed() < std::time::Duration::from_secs(30) {
                return Err(ApiError::new(
                    axum::http::StatusCode::TOO_MANY_REQUESTS,
                    "throttled",
                    "ログイン失敗が続いたため一時的にロックしています",
                ));
            }
        }
        Ok(())
    }
    pub fn record(&self, username: &str, ok: bool) {
        let mut map = self.inner.lock().unwrap();
        if ok {
            map.remove(username);
        } else {
            let e = map
                .entry(username.to_owned())
                .or_insert((0, std::time::Instant::now()));
            if e.1.elapsed() >= std::time::Duration::from_secs(30) {
                *e = (0, std::time::Instant::now());
            }
            e.0 += 1;
            e.1 = std::time::Instant::now();
        }
    }
}
