use crate::{
    error::{Error, Result},
    git,
    models::{Application, Deployment},
    queue::{deploy::DeployPayload, KIND_DEPLOY},
    repos::{deployments, jobs},
    services::applications,
    state::AppState,
};
use anyhow::anyhow;
use uuid::Uuid;

/// Records a deployment and queues the work. Returns immediately: the build
/// runs in the worker, so the request is not held open for minutes.
pub async fn trigger(
    state: &AppState,
    app_id: Uuid,
    owner_id: Uuid,
    commit_sha: Option<String>,
) -> Result<Deployment> {
    let app = applications::get(&state.db, app_id, owner_id).await?;

    // A webhook already knows the commit. A manual deploy does not, so the
    // branch is resolved now and the build pinned to that SHA, which keeps a
    // deployment reproducible and gives the dashboard something real to show.
    let git_ref = match commit_sha {
        Some(sha) => sha,
        None => git::resolve_branch(&app.git_repo, &app.git_branch)
            .await?
            .unwrap_or_else(|| app.git_branch.clone()),
    };

    enqueue(state, &app, &git_ref).await
}

/// Records a deployment and queues it. Callers have already established that
/// the request is allowed: a session for a manual deploy, a valid signature for
/// a webhook.
pub async fn enqueue(state: &AppState, app: &Application, git_ref: &str) -> Result<Deployment> {
    let deployment = deployments::create(&state.db, app.id, git_ref).await?;

    let payload = serde_json::to_value(DeployPayload {
        deployment_id: deployment.id,
    })
    .map_err(|e| Error::Internal(anyhow!("could not encode deploy payload: {e}")))?;
    jobs::enqueue(&state.db, KIND_DEPLOY, payload).await?;

    Ok(deployment)
}

pub async fn list(
    state: &AppState,
    app_id: Uuid,
    owner_id: Uuid,
    limit: i64,
) -> Result<Vec<Deployment>> {
    // Establishes ownership before listing.
    applications::get(&state.db, app_id, owner_id).await?;
    deployments::list_by_app(&state.db, app_id, limit.clamp(1, 100)).await
}

pub async fn get(state: &AppState, id: Uuid, owner_id: Uuid) -> Result<Deployment> {
    deployments::get_owned(&state.db, id, owner_id)
        .await?
        .ok_or(Error::NotFound("deployment"))
}
