//! SQLite (sqlx)。プールの作成、マイグレーション、初回 admin の作成。

use crate::auth;
use crate::util;
use anyhow::Context;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;

pub async fn open(path: &Path) -> anyhow::Result<SqlitePool> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("{} を作れません", parent.display()))?;
    }
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(10));
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await
        .with_context(|| format!("{} を開けません", path.display()))?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("マイグレーションに失敗")?;
    Ok(pool)
}

/// ユーザが 1 人もいなければ admin を作る。パスワードは設定か自動生成。
/// 戻り値は生成したパスワード (ログに出すため)。
pub async fn bootstrap_admin(
    pool: &SqlitePool,
    configured_password: Option<&str>,
) -> anyhow::Result<Option<String>> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if n > 0 {
        return Ok(None);
    }
    let generated = configured_password.is_none();
    let password = configured_password
        .map(str::to_owned)
        .unwrap_or_else(util::random_token);
    let hash = auth::hash_password(&password)?;
    let now = util::now_str();
    sqlx::query(
        "INSERT INTO users (id, username, display_name, password_hash, roles, groups_json, must_change_password, created_at, updated_at)
         VALUES (?, 'admin', 'Administrator', ?, ?, '[]', 1, ?, ?)",
    )
    .bind(util::new_id())
    .bind(&hash)
    .bind(serde_json::to_string(&auth::ALL_ROLES).unwrap())
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(generated.then_some(password))
}
