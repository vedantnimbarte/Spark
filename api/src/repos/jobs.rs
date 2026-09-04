use crate::error::Result;
use serde_json::Value;
use sqlx::PgPool;

pub struct Job {
    pub id: i64,
    pub kind: String,
    pub payload: Value,
    pub attempts: i32,
}

pub async fn enqueue(db: &PgPool, kind: &str, payload: Value) -> Result<()> {
    sqlx::query!(
        "INSERT INTO jobs (kind, payload) VALUES ($1, $2)",
        kind,
        payload,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Takes exactly one job, skipping rows another worker already holds.
///
/// A lock older than `stale_after_minutes` is treated as abandoned, which is
/// what lets work resume after the control plane is killed mid-build instead
/// of stranding the job forever.
pub async fn claim(db: &PgPool, stale_after_minutes: i32) -> Result<Option<Job>> {
    let job = sqlx::query_as!(
        Job,
        r#"UPDATE jobs SET locked_at = now(), attempts = attempts + 1
           WHERE id = (
               SELECT id FROM jobs
               WHERE run_at <= now()
                 AND (locked_at IS NULL
                      OR locked_at < now() - make_interval(mins => $1))
               ORDER BY run_at
               FOR UPDATE SKIP LOCKED
               LIMIT 1
           )
           RETURNING id, kind, payload, attempts"#,
        stale_after_minutes,
    )
    .fetch_optional(db)
    .await?;
    Ok(job)
}

pub async fn complete(db: &PgPool, id: i64) -> Result<()> {
    sqlx::query!("DELETE FROM jobs WHERE id = $1", id)
        .execute(db)
        .await?;
    Ok(())
}

/// Releases the lock and backs off before the next attempt.
pub async fn retry_later(db: &PgPool, id: i64, error: &str, delay_seconds: i32) -> Result<()> {
    sqlx::query!(
        r#"UPDATE jobs
           SET locked_at = NULL,
               last_error = $2,
               run_at = now() + make_interval(secs => $3)
           WHERE id = $1"#,
        id,
        error,
        f64::from(delay_seconds),
    )
    .execute(db)
    .await?;
    Ok(())
}
