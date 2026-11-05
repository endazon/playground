//! 認証と認可。
//!
//! - パスワードは argon2id。
//! - 認証はすべて `Authorization: Bearer <token>` (D29: Cookie を使わないので CSRF 対策が不要。
//!   トークンはポータルの SPA がメモリ / sessionStorage に持ち、シェル経由でエンジンに渡す)。
//! - トークンの種別: ログインセッション (`sess_`)、チケット交換で得た短命トークン (`lt_`)、
//!   クライアント API キー (`ipk_`)。DB にはハッシュだけを置く。
//! - チケット由来の `lt_` は `ipemu://` の URL を経由して外部ブラウザ・OS・ログを通るため、
//!   **配信 API 限定** (catalog / manifests / artifacts / telemetry / hello / launches/mine) とし、
//!   `UserAuth` / `AdminAuth` では拒否し、「新鮮なログイン」とも見なさない (D51)。

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
    /// チケット交換で得た短命トークン (`lt_`) の主体か。配信 API にしか使えない (D51)
    pub fn is_delivery_only(&self) -> bool {
        self.credential == CredentialKind::Ticket
    }
    pub fn require(&self, ok: bool, what: &str) -> Result<(), ApiError> {
        if ok {
            Ok(())
        } else {
            Err(ApiError::forbidden(format!("{what} の権限がありません")))
        }
    }
    /// ログインから `FRESH_LOGIN_MAX_SECS` 以内か。Bearer を sessionStorage に置く構成の緩和策で、
    /// 盗まれた古いトークンで承認・公開・鍵・ユーザ操作ができないようにする (D36)。
    /// 成立するのはログインセッション (`sess_`) だけ。チケット交換トークンの `issued_at` は交換時刻で
    /// ログイン時刻ではないので、新しくても「新鮮なログイン」とは見なさない (D51)
    pub fn require_fresh_login(&self) -> Result<(), ApiError> {
        let fresh = self.credential == CredentialKind::Session
            && self
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

/// 利用者 (ユーザ) のログインセッション必須。API キーとチケット交換トークンは拒否する (D51)。
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
        if p.is_delivery_only() {
            return Err(ApiError::forbidden(
                "起動用の短命トークンは配信 API にしか使えません。ポータルからログインしてください",
            ));
        }
        Ok(UserAuth(p))
    }
}

/// 管理画面 (閲覧以上) 必須。`UserAuth` を経由するのでチケット交換トークンは入れない (D51)。
pub struct AdminAuth(pub Principal);

impl FromRequestParts<AppState> for AdminAuth {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let UserAuth(p) = UserAuth::from_request_parts(parts, state).await?;
        // UserAuth で弾いているが、管理 API の入口なので明示的にも確かめる
        if p.is_delivery_only() {
            return Err(ApiError::forbidden(
                "起動用の短命トークンでは管理 API を使えません",
            ));
        }
        if !p.can_view_admin() {
            return Err(ApiError::forbidden("管理画面の権限がありません"));
        }
        Ok(AdminAuth(p))
    }
}

/// 送信元 IP の extractor (ログイン抑止の鍵に使う)。失敗しない。
///
/// 既定は TCP の接続元 (`ConnectInfo<SocketAddr>`)。`IPEMU_TRUST_PROXY=1` (`Config::trust_proxy`) のときだけ
/// リバースプロキシが付ける `X-Forwarded-For` の先頭を信用する。どちらも取れなければ `None`。
pub struct ClientIp(pub Option<std::net::IpAddr>);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = std::convert::Infallible;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if state.config.trust_proxy {
            let fwd = parts
                .headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(',').next())
                .and_then(|v| v.trim().parse::<std::net::IpAddr>().ok());
            if fwd.is_some() {
                return Ok(ClientIp(fwd));
            }
        }
        Ok(ClientIp(
            parts
                .extensions
                .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                .map(|c| c.0.ip()),
        ))
    }
}

/// (ユーザ名, 送信元 IP) ごとの連続失敗の上限。超えると `THROTTLE_WINDOW` の間ロックする
pub const THROTTLE_PER_PAIR: u32 = 5;
/// ユーザ名単独 (送信元を問わない) の連続失敗の上限。分散した総当たりへの備えで、
/// 第三者が正規ユーザを簡単にロックできないよう緩くしてある (D53)
pub const THROTTLE_PER_USER: u32 = 50;
pub const THROTTLE_WINDOW: std::time::Duration = std::time::Duration::from_secs(30);

/// ログイン失敗の抑止 (D53)。鍵は (ユーザ名, 送信元 IP) で 5 回 / 30 秒、ユーザ名単独で 50 回 / 30 秒。
/// 最後の失敗から 30 秒経つと数え直す。メモリ上なのでプロセス再起動で消える。
#[derive(Default)]
pub struct LoginThrottle {
    inner: std::sync::Mutex<ThrottleMaps>,
}

type Counter = (u32, std::time::Instant);

#[derive(Default)]
struct ThrottleMaps {
    by_pair: std::collections::HashMap<(String, Option<std::net::IpAddr>), Counter>,
    by_user: std::collections::HashMap<String, Counter>,
}

fn locked(c: Option<&Counter>, limit: u32) -> bool {
    c.is_some_and(|(n, since)| *n >= limit && since.elapsed() < THROTTLE_WINDOW)
}

fn bump(c: &mut Counter) {
    if c.1.elapsed() >= THROTTLE_WINDOW {
        c.0 = 0;
    }
    c.0 += 1;
    c.1 = std::time::Instant::now();
}

impl LoginThrottle {
    pub fn check(&self, username: &str, ip: Option<std::net::IpAddr>) -> Result<(), ApiError> {
        let maps = self.inner.lock().unwrap();
        let pair = maps.by_pair.get(&(username.to_owned(), ip));
        if locked(pair, THROTTLE_PER_PAIR) || locked(maps.by_user.get(username), THROTTLE_PER_USER)
        {
            return Err(ApiError::new(
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                "throttled",
                "ログイン失敗が続いたため一時的にロックしています",
            ));
        }
        Ok(())
    }
    pub fn record(&self, username: &str, ip: Option<std::net::IpAddr>, ok: bool) {
        let mut maps = self.inner.lock().unwrap();
        let key = (username.to_owned(), ip);
        if ok {
            // 成功した送信元の分だけ消す。ユーザ名単独の数は他の送信元の失敗を含むので残す
            maps.by_pair.remove(&key);
        } else {
            let now = std::time::Instant::now();
            bump(maps.by_pair.entry(key).or_insert((0, now)));
            bump(maps.by_user.entry(username.to_owned()).or_insert((0, now)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn ip(n: u8) -> Option<IpAddr> {
        Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, n)))
    }

    #[test]
    fn throttle_locks_per_user_and_ip() {
        let t = LoginThrottle::default();
        for _ in 0..THROTTLE_PER_PAIR {
            assert!(t.check("alice", ip(1)).is_ok());
            t.record("alice", ip(1), false);
        }
        // 失敗させた送信元からはロック
        assert_eq!(
            t.check("alice", ip(1)).unwrap_err().status,
            axum::http::StatusCode::TOO_MANY_REQUESTS
        );
        // 別の送信元の正規ユーザはロックされない
        assert!(t.check("alice", ip(2)).is_ok());
        // 同じ送信元でも別のユーザ名はロックされない
        assert!(t.check("bob", ip(1)).is_ok());
        // 別の送信元から成功しても、失敗させた送信元のロックは残る
        t.record("alice", ip(2), true);
        assert!(t.check("alice", ip(1)).is_err());
    }

    #[test]
    fn throttle_locks_user_after_many_sources() {
        let t = LoginThrottle::default();
        // 送信元を変えながら失敗させる (各送信元は上限未満)
        for i in 0..THROTTLE_PER_USER {
            let src = ip((i % 250) as u8 + 1);
            t.record("alice", src, false);
        }
        // ユーザ名単独の上限に達したので、新しい送信元からもロック
        assert!(t.check("alice", ip(251)).is_err());
        assert!(t.check("bob", ip(251)).is_ok());
    }

    #[test]
    fn throttle_success_clears_pair() {
        let t = LoginThrottle::default();
        for _ in 0..THROTTLE_PER_PAIR - 1 {
            t.record("alice", ip(1), false);
        }
        t.record("alice", ip(1), true);
        t.record("alice", ip(1), false);
        assert!(t.check("alice", ip(1)).is_ok());
    }
}
