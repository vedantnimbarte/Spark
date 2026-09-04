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

/// A session as the owner may see it: no token material, just a handle and
/// enough context to recognise which one to revoke.
#[derive(Debug, serde::Serialize)]
pub struct SessionInfo {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// True for the session making the request, so the dashboard can label it
    /// rather than letting someone sign themselves out by accident.
    pub current: bool,
}

pub async fn list_for_user(
    db: &PgPool,
    user_id: Uuid,
    current_token_hash: &str,
) -> Result<Vec<SessionInfo>> {
    let sessions = sqlx::query_as!(
        SessionInfo,
        r#"SELECT id AS "id!",
                  created_at AS "created_at!",
                  expires_at AS "expires_at!",
                  (token_hash = $2) AS "current!"
           FROM sessions
           WHERE user_id = $1 AND expires_at > now()
           ORDER BY created_at DESC"#,
        user_id,
        current_token_hash,
    )
    .fetch_all(db)
    .await?;
    Ok(sessions)
}

/// Scoped by user id so one account cannot revoke another's session by
/// guessing an id.
pub async fn delete_owned(db: &PgPool, id: Uuid, user_id: Uuid) -> Result<u64> {
    let result = sqlx::query!(
        "DELETE FROM sessions WHERE id = $1 AND user_id = $2",
        id,
        user_id,
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

/// Everything except the caller's own session. Used after a password change:
/// the point of changing it is to lock out whoever else was signed in.
pub async fn delete_others(db: &PgPool, user_id: Uuid, keep_token_hash: &str) -> Result<u64> {
    let result = sqlx::query!(
        "DELETE FROM sessions WHERE user_id = $1 AND token_hash <> $2",
        user_id,
        keep_token_hash,
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}
