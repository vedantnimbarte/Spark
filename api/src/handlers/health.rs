use crate::{error::Result, state::SharedState};
use axum::{extract::State, Json};
use serde_json::{json, Value};

/// Liveness plus a real database round-trip, so an unreachable Postgres shows
/// up here rather than as a surprise on the first request that matters.
pub async fn health(State(state): State<SharedState>) -> Result<Json<Value>> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    Ok(Json(json!({ "status": "ok" })))
}
