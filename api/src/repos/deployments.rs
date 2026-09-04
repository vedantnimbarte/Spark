use crate::{error::Result, models::Deployment};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create(db: &PgPool, app_id: Uuid, commit_sha: &str) -> Result<Deployment> {
    let deployment = sqlx::query_as!(
        Deployment,
        r#"INSERT INTO deployments (app_id, commit_sha) VALUES ($1, $2)
           RETURNING id, app_id, commit_sha, status, image_ref, started_at, finished_at, created_at"#,
        app_id,
        commit_sha,
    )
    .fetch_one(db)
    .await?;
    Ok(deployment)
}

pub async fn get(db: &PgPool, id: Uuid) -> Result<Option<Deployment>> {
    let deployment = sqlx::query_as!(
        Deployment,
        r#"SELECT id, app_id, commit_sha, status, image_ref, started_at, finished_at, created_at
           FROM deployments WHERE id = $1"#,
        id,
    )
    .fetch_optional(db)
    .await?;
    Ok(deployment)
}

/// Scoped through the owning application so one user cannot read another's
/// deployment by guessing an id.
pub async fn get_owned(db: &PgPool, id: Uuid, owner_id: Uuid) -> Result<Option<Deployment>> {
    let deployment = sqlx::query_as!(
        Deployment,
        r#"SELECT d.id, d.app_id, d.commit_sha, d.status, d.image_ref,
                  d.started_at, d.finished_at, d.created_at
           FROM deployments d
           JOIN applications a ON a.id = d.app_id
           WHERE d.id = $1 AND a.owner_id = $2"#,
        id,
        owner_id,
    )
    .fetch_optional(db)
    .await?;
    Ok(deployment)
}

pub async fn list_by_app(db: &PgPool, app_id: Uuid, limit: i64) -> Result<Vec<Deployment>> {
    let deployments = sqlx::query_as!(
        Deployment,
        r#"SELECT id, app_id, commit_sha, status, image_ref, started_at, finished_at, created_at
           FROM deployments WHERE app_id = $1 ORDER BY created_at DESC LIMIT $2"#,
        app_id,
        limit,
    )
    .fetch_all(db)
    .await?;
    Ok(deployments)
}

pub async fn mark_building(db: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE deployments SET status = 'building', started_at = now() WHERE id = $1",
        id,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_status(db: &PgPool, id: Uuid, status: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE deployments SET status = $2 WHERE id = $1",
        id,
        status
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn finish(db: &PgPool, id: Uuid, status: &str, image_ref: Option<&str>) -> Result<()> {
    sqlx::query!(
        r#"UPDATE deployments
           SET status = $2, image_ref = COALESCE($3, image_ref), finished_at = now()
           WHERE id = $1"#,
        id,
        status,
        image_ref,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Appends to the stored log. Build output arrives in chunks, and the SSE
/// endpoint reads the same column, so there is one source of truth for logs.
pub async fn append_logs(db: &PgPool, id: Uuid, chunk: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE deployments SET logs = logs || $2 WHERE id = $1",
        id,
        chunk,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn logs_from(db: &PgPool, id: Uuid, offset: i32) -> Result<Option<String>> {
    let logs = sqlx::query_scalar!(
        "SELECT substring(logs FROM $2::int) FROM deployments WHERE id = $1",
        id,
        offset + 1,
    )
    .fetch_optional(db)
    .await?;
    Ok(logs.flatten())
}
