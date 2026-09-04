use crate::{error::Result, models::User};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create(
    db: &PgPool,
    token_hash: &str,
    user_id: Uuid,
    expires_at: DateTime<Utc>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, $3)",
        token_hash,
        user_id,
        expires_at,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Resolves a session token hash to its user, rejecting expired sessions in the
/// same query so an expired row can never authenticate.
pub async fn find_valid_user(db: &PgPool, token_hash: &str) -> Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"SELECT u.id, u.email, u.password_hash, u.created_at
           FROM sessions s
           JOIN users u ON u.id = s.user_id
           WHERE s.token_hash = $1 AND s.expires_at > now()"#,
        token_hash,
    )
    .fetch_optional(db)
    .await?;
    Ok(user)
}

pub async fn delete(db: &PgPool, token_hash: &str) -> Result<()> {
    sqlx::query!("DELETE FROM sessions WHERE token_hash = $1", token_hash)
        .execute(db)
        .await?;
    Ok(())
}

/// Expired rows are never read, only accumulated; nothing else removes them.
pub async fn delete_expired(db: &PgPool) -> Result<u64> {
    let result = sqlx::query!("DELETE FROM sessions WHERE expires_at < now()")
        .execute(db)
        .await?;
    Ok(result.rows_affected())
}
