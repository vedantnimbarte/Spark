use crate::{
    auth::session::CurrentUser, error::Result, models::Deployment, repos::deployments as repo,
    services::deployments as svc, state::SharedState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures::Stream;
use serde::Deserialize;
use std::{convert::Infallible, time::Duration};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct DeployRequest {
    /// Supplied by a webhook; absent for a manual deploy, which resolves the
    /// branch itself.
    #[serde(default)]
    pub commit_sha: Option<String>,
}

pub async fn deploy(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(app_id): Path<Uuid>,
    body: Option<Json<DeployRequest>>,
) -> Result<(StatusCode, Json<Deployment>)> {
    let commit_sha = body.and_then(|Json(b)| b.commit_sha);
    let deployment = svc::trigger(&state, app_id, user.id, commit_sha).await?;
    Ok((StatusCode::ACCEPTED, Json(deployment)))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

pub async fn list(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(app_id): Path<Uuid>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Deployment>>> {
    Ok(Json(svc::list(&state, app_id, user.id, query.limit).await?))
}

pub async fn get(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Deployment>> {
    Ok(Json(svc::get(&state, id, user.id).await?))
}

/// Streams the build log as server-sent events.
///
/// The log column is the single source of truth and is polled here rather than
/// wired through a pub/sub: it reconnects for free, survives a control plane
/// restart mid-build, and works the same for a finished deployment as a
/// running one.
pub async fn logs(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Sse<impl Stream<Item = std::result::Result<Event, Infallible>>>> {
    // Establishes ownership before opening the stream.
    svc::get(&state, id, user.id).await?;

    let stream = async_stream::stream! {
        let mut offset = 0_i32;

        loop {
            match repo::logs_from(&state.db, id, offset).await {
                Ok(Some(chunk)) if !chunk.is_empty() => {
                    offset += i32::try_from(chunk.len()).unwrap_or(i32::MAX);
                    for line in chunk.lines() {
                        yield Ok(Event::default().data(line));
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(%error, "log stream query failed");
                    break;
                }
            }

            match repo::get(&state.db, id).await {
                Ok(Some(deployment)) if deployment.is_finished() => {
                    yield Ok(Event::default().event("end").data(deployment.status));
                    break;
                }
                Ok(Some(_)) => {}
                // Deleted or unreadable: nothing further will arrive.
                _ => break,
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
