use crate::{auth::session::CurrentUser, error::Result, repos::stats, state::SharedState};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

/// How many deployments the activity feed carries. Enough to fill the panel
/// without turning the analytics request into a full history download.
const RECENT_LIMIT: i64 = 12;

#[derive(Debug, Deserialize)]
pub struct Window {
    days: Option<i32>,
}

impl Window {
    /// Clamped rather than rejected: a nonsense value in a query string should
    /// still render a page, and an unbounded one would scan the whole table.
    fn days(&self) -> i32 {
        self.days.unwrap_or(30).clamp(1, 365)
    }
}

#[derive(Serialize)]
pub struct Stats {
    days: i32,
    summary: stats::Summary,
    daily: Vec<stats::DayBucket>,
    projects: Vec<stats::ProjectRollup>,
    recent: Vec<stats::RecentDeployment>,
}

/// Everything the analytics page draws, in one round trip: the four rollups
/// are independent, so issuing them as four requests would only add latency.
pub async fn overview(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Query(window): Query<Window>,
) -> Result<Json<Stats>> {
    let days = window.days();
    let (summary, daily, projects, recent) = tokio::try_join!(
        stats::summary(&state.db, user.id, days),
        stats::daily(&state.db, user.id, days),
        stats::by_project(&state.db, user.id, days),
        stats::recent(&state.db, user.id, RECENT_LIMIT),
    )?;

    Ok(Json(Stats {
        days,
        summary,
        daily,
        projects,
        recent,
    }))
}
