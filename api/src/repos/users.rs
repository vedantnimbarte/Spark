use crate::{error::Result, models::User};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create(db: &PgPool, email: &str, password_hash: &str) -> Result<User> {
    let user = sqlx::query_as!(
        User,
        r#"INSERT INTO users (email, password_hash) VALUES ($1, $2)
           RETURNING id, email, password_hash, created_at"#,
        email,
        password_hash,
    )
    .fetch_one(db)
    .await?;
    Ok(user)
}

pub async fn find_by_email(db: &PgPool, email: &str) -> Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"SELECT id, email, password_hash, created_at FROM users WHERE email = $1"#,
        email,
    )
    .fetch_optional(db)
    .await?;
    Ok(user)
}

pub async fn find_by_id(db: &PgPool, id: Uuid) -> Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"SELECT id, email, password_hash, created_at FROM users WHERE id = $1"#,
        id,
    )
    .fetch_optional(db)
    .await?;
    Ok(user)
}

pub async fn count(db: &PgPool) -> Result<i64> {
    let count = sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM users"#)
        .fetch_one(db)
        .await?;
    Ok(count)
}

pub async fn update_password(db: &PgPool, id: Uuid, password_hash: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE users SET password_hash = $2 WHERE id = $1",
        id,
        password_hash,
    )
    .execute(db)
    .await?;
    Ok(())
}
