use crate::{error::Result, models::Domain};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn list_by_app(db: &PgPool, app_id: Uuid) -> Result<Vec<Domain>> {
    let domains = sqlx::query_as!(
        Domain,
        r#"SELECT id, app_id, domain_name, ssl_status, created_at
           FROM domains WHERE app_id = $1 ORDER BY created_at"#,
        app_id,
    )
    .fetch_all(db)
    .await?;
    Ok(domains)
}

pub async fn create(db: &PgPool, app_id: Uuid, domain_name: &str) -> Result<Domain> {
    let domain = sqlx::query_as!(
        Domain,
        r#"INSERT INTO domains (app_id, domain_name) VALUES ($1, $2)
           RETURNING id, app_id, domain_name, ssl_status, created_at"#,
        app_id,
        domain_name,
    )
    .fetch_one(db)
    .await?;
    Ok(domain)
}

pub async fn delete(db: &PgPool, id: Uuid, app_id: Uuid) -> Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM domains WHERE id = $1 AND app_id = $2",
        id,
        app_id,
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn set_ssl_status(db: &PgPool, app_id: Uuid, status: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE domains SET ssl_status = $2 WHERE app_id = $1 AND ssl_status IS DISTINCT FROM $2",
        app_id,
        status,
    )
    .execute(db)
    .await?;
    Ok(())
}
