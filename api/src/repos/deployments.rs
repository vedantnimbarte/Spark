use crate::{error::Result, models::Deployment};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create(db: &PgPool, app_id: Uuid, commit_sha: &str) -> Result<Deployment> {
    let deployment = sqlx::query_as!(
        Deployment,
        r#"INSERT INTO deployments (app_id, commit_sha) VALUES ($1, $2)
           RETURNING id, app_id, commit_sha, status, image_ref, rolled_back_from,
                     started_at, finished_at, created_at"#,
        app_id,
        commit_sha,
    )
    .fetch_one(db)
    .await?;
    Ok(deployment)
}

/// A rollback reuses an image that was already built, so it starts at
/// `deploying` and never enters the build path.
pub async fn create_rollback(db: &PgPool, app_id: Uuid, source: &Deployment) -> Result<Deployment> {
    let deployment = sqlx::query_as!(
        Deployment,
        r#"INSERT INTO deployments (app_id, commit_sha, image_ref, rolled_back_from, status)
           VALUES ($1, $2, $3, $4, 'deploying')
           RETURNING id, app_id, commit_sha, status, image_ref, rolled_back_from,
                     started_at, finished_at, created_at"#,
        app_id,
        source.commit_sha,
        source.image_ref,
        source.id,
    )
    .fetch_one(db)
    .await?;
    Ok(deployment)
}

pub async fn get(db: &PgPool, id: Uuid) -> Result<Option<Deployment>> {
    let deployment = sqlx::query_as!(
        Deployment,
        r#"SELECT id, app_id, commit_sha, status, image_ref, rolled_back_from,
                  started_at, finished_at, created_at
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
        r#"SELECT d.id, d.app_id, d.commit_sha, d.status, d.image_ref, d.rolled_back_from,
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
        r#"SELECT id, app_id, commit_sha, status, image_ref, rolled_back_from,
                  started_at, finished_at, created_at
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

/// Appends one line of build output.
///
/// An append-only row rather than a growing TEXT column: concatenating onto a
/// column rewrites the whole row on every line, and reading the tail with
/// substring() detoasts all of it on every poll.
pub async fn append_log(db: &PgPool, id: Uuid, line: &str) -> Result<()> {
    sqlx::query!(
        "INSERT INTO deployment_log_lines (deployment_id, line) VALUES ($1, $2)",
        id,
        line,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub struct LogLine {
    pub seq: i64,
    pub line: String,
}

/// Lines after `after_seq`, which is how the log stream tails without re-reading
/// what it already sent.
pub async fn logs_after(db: &PgPool, id: Uuid, after_seq: i64) -> Result<Vec<LogLine>> {
    let lines = sqlx::query_as!(
        LogLine,
        r#"SELECT seq, line FROM deployment_log_lines
           WHERE deployment_id = $1 AND seq > $2
           ORDER BY seq
           LIMIT 2000"#,
        id,
        after_seq,
    )
    .fetch_all(db)
    .await?;
    Ok(lines)
}

/// Deployments beyond the newest `keep_per_app` for their application.
///
/// The currently deployed one is protected regardless of age, so pruning can
/// never delete the image an application is running.
pub async fn stale(db: &PgPool, keep_per_app: i64) -> Result<Vec<Deployment>> {
    let stale = sqlx::query_as!(
        Deployment,
        r#"WITH ranked AS (
               SELECT id,
                      row_number() OVER (PARTITION BY app_id ORDER BY created_at DESC) AS rank
               FROM deployments
           ),
           live AS (
               SELECT DISTINCT ON (app_id) id
               FROM deployments
               WHERE status = 'deployed'
               ORDER BY app_id, created_at DESC
           )
           SELECT d.id, d.app_id, d.commit_sha, d.status, d.image_ref, d.rolled_back_from,
                  d.started_at, d.finished_at, d.created_at
           FROM deployments d
           JOIN ranked r ON r.id = d.id
           WHERE r.rank > $1
             AND d.id NOT IN (SELECT id FROM live)
             AND d.status IN ('deployed', 'failed')"#,
        keep_per_app,
    )
    .fetch_all(db)
    .await?;
    Ok(stale)
}

pub async fn delete_many(db: &PgPool, ids: &[Uuid]) -> Result<u64> {
    let result = sqlx::query!("DELETE FROM deployments WHERE id = ANY($1)", ids)
        .execute(db)
        .await?;
    Ok(result.rows_affected())
}
