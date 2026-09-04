use crate::error::Result;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn list(db: &PgPool, app_id: Uuid) -> Result<Vec<String>> {
    let keys = sqlx::query_scalar!(
        r#"SELECT key FROM app_env_keys WHERE app_id = $1 ORDER BY key"#,
        app_id,
    )
    .fetch_all(db)
    .await?;
    Ok(keys)
}

pub async fn upsert(db: &PgPool, app_id: Uuid, key: &str) -> Result<()> {
    sqlx::query!(
        r#"INSERT INTO app_env_keys (app_id, key) VALUES ($1, $2)
           ON CONFLICT (app_id, key) DO NOTHING"#,
        app_id,
        key,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete(db: &PgPool, app_id: Uuid, key: &str) -> Result<()> {
    sqlx::query!(
        "DELETE FROM app_env_keys WHERE app_id = $1 AND key = $2",
        app_id,
        key,
    )
    .execute(db)
    .await?;
    Ok(())
}
