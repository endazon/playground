//! 監査ログ。すべての変更操作が通る。

use crate::auth::Principal;
use crate::util;
use sqlx::SqlitePool;

pub async fn record(
    pool: &SqlitePool,
    actor: &Principal,
    action: &str,
    target: &str,
    detail: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit (id, at, actor_id, actor_name, action, target, detail) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(util::new_id())
    .bind(util::now_str())
    .bind(&actor.id)
    .bind(&actor.name)
    .bind(action)
    .bind(target)
    .bind(detail.to_string())
    .execute(pool)
    .await?;
    Ok(())
}
