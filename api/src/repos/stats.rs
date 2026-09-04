//! Aggregate reads for the analytics page.
//!
//! These roll up in Postgres rather than in the dashboard: the alternative is
//! shipping every deployment row to the browser and counting there, which grows
//! without bound while the answer stays four numbers wide.

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// One day of deploy activity. Days with no deploys are present with zeroes,
/// so the chart keeps an even time axis instead of collapsing gaps.
#[derive(Debug, Serialize)]
pub struct DayBucket {
    pub day: DateTime<Utc>,
    pub succeeded: i64,
    pub failed: i64,
    pub total: i64,
    /// Median build seconds for that day, or null when nothing finished.
    pub median_build_seconds: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub total: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub in_flight: i64,
    pub median_build_seconds: Option<f64>,
    pub p95_build_seconds: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ProjectRollup {
    pub app_id: Uuid,
    pub name: String,
    pub deploys: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub median_build_seconds: Option<f64>,
    pub last_deploy_at: Option<DateTime<Utc>>,
}

/// A deployment with the name of the application it belongs to, for the
/// cross-project activity feed.
#[derive(Debug, Serialize)]
pub struct RecentDeployment {
    pub id: Uuid,
    pub app_id: Uuid,
    pub app_name: String,
    pub commit_sha: String,
    pub status: String,
    pub rolled_back: bool,
    pub duration_seconds: Option<f64>,
    pub created_at: DateTime<Utc>,
}

pub async fn daily(db: &PgPool, owner_id: Uuid, days: i32) -> Result<Vec<DayBucket>> {
    // The series is generated first and left-joined onto, so a quiet day still
    // produces a row. Filtering the deployments inside the ON clause rather
    // than a WHERE is what keeps those empty days from being dropped.
    let rows = sqlx::query_as!(
        DayBucket,
        r#"SELECT
               s.day AS "day!",
               count(d.id) FILTER (WHERE d.status = 'deployed') AS "succeeded!",
               count(d.id) FILTER (WHERE d.status = 'failed') AS "failed!",
               count(d.id) AS "total!",
               percentile_cont(0.5) WITHIN GROUP (
                   ORDER BY extract(epoch FROM d.finished_at - d.started_at)::float8
               ) FILTER (
                   WHERE d.started_at IS NOT NULL AND d.finished_at IS NOT NULL
               ) AS "median_build_seconds"
           FROM generate_series(
                    date_trunc('day', now()) - make_interval(days => $2 - 1),
                    date_trunc('day', now()),
                    '1 day'
                ) AS s(day)
           LEFT JOIN deployments d
             ON d.created_at >= s.day
            AND d.created_at < s.day + interval '1 day'
            AND d.app_id IN (SELECT id FROM applications WHERE owner_id = $1)
           GROUP BY s.day
           ORDER BY s.day"#,
        owner_id,
        days,
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

pub async fn summary(db: &PgPool, owner_id: Uuid, days: i32) -> Result<Summary> {
    let row = sqlx::query_as!(
        Summary,
        r#"SELECT
               count(*) AS "total!",
               count(*) FILTER (WHERE status = 'deployed') AS "succeeded!",
               count(*) FILTER (WHERE status = 'failed') AS "failed!",
               count(*) FILTER (
                   WHERE status IN ('pending', 'building', 'deploying')
               ) AS "in_flight!",
               percentile_cont(0.5) WITHIN GROUP (
                   ORDER BY extract(epoch FROM finished_at - started_at)::float8
               ) FILTER (
                   WHERE started_at IS NOT NULL AND finished_at IS NOT NULL
               ) AS "median_build_seconds",
               percentile_cont(0.95) WITHIN GROUP (
                   ORDER BY extract(epoch FROM finished_at - started_at)::float8
               ) FILTER (
                   WHERE started_at IS NOT NULL AND finished_at IS NOT NULL
               ) AS "p95_build_seconds"
           FROM deployments
           WHERE app_id IN (SELECT id FROM applications WHERE owner_id = $1)
             AND created_at >= now() - make_interval(days => $2)"#,
        owner_id,
        days,
    )
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// Every application the user owns, including ones with no deploys in the
/// window: "this project has shipped nothing in 30 days" is the finding.
pub async fn by_project(db: &PgPool, owner_id: Uuid, days: i32) -> Result<Vec<ProjectRollup>> {
    let rows = sqlx::query_as!(
        ProjectRollup,
        r#"SELECT
               a.id AS "app_id!",
               a.name AS "name!",
               count(d.id) AS "deploys!",
               count(d.id) FILTER (WHERE d.status = 'deployed') AS "succeeded!",
               count(d.id) FILTER (WHERE d.status = 'failed') AS "failed!",
               percentile_cont(0.5) WITHIN GROUP (
                   ORDER BY extract(epoch FROM d.finished_at - d.started_at)::float8
               ) FILTER (
                   WHERE d.started_at IS NOT NULL AND d.finished_at IS NOT NULL
               ) AS "median_build_seconds",
               max(d.created_at) AS "last_deploy_at"
           FROM applications a
           LEFT JOIN deployments d
             ON d.app_id = a.id
            AND d.created_at >= now() - make_interval(days => $2)
           WHERE a.owner_id = $1
           GROUP BY a.id, a.name
           ORDER BY count(d.id) DESC, a.name"#,
        owner_id,
        days,
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

pub async fn recent(db: &PgPool, owner_id: Uuid, limit: i64) -> Result<Vec<RecentDeployment>> {
    let rows = sqlx::query_as!(
        RecentDeployment,
        r#"SELECT
               d.id AS "id!",
               d.app_id AS "app_id!",
               a.name AS "app_name!",
               d.commit_sha AS "commit_sha!",
               d.status AS "status!",
               (d.rolled_back_from IS NOT NULL) AS "rolled_back!",
               extract(epoch FROM d.finished_at - d.started_at)::float8 AS "duration_seconds",
               d.created_at AS "created_at!"
           FROM deployments d
           JOIN applications a ON a.id = d.app_id
           WHERE a.owner_id = $1
           ORDER BY d.created_at DESC
           LIMIT $2"#,
        owner_id,
        limit,
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}
