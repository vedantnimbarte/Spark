use crate::{
    error::{Error, Result},
    git,
    models::{Application, Deployment},
    queue::{deploy::DeployPayload, KIND_DEPLOY},
    repos::{deployments, jobs},
    services::{applications, git_credentials},
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
        None => {
            let token = git_credentials::token_for(state, &app).await?;
            git::resolve_branch(&app.git_repo, &app.git_branch, token.as_deref())
                .await?
                .unwrap_or_else(|| app.git_branch.clone())
        }
    };

    enqueue(state, &app, &git_ref).await
}

/// Records a deployment and queues it. Callers have already established that
/// the request is allowed: a session for a manual deploy, a valid signature for
/// a webhook.
pub async fn enqueue(state: &AppState, app: &Application, git_ref: &str) -> Result<Deployment> {
    let deployment = deployments::create(&state.db, app.id, git_ref)
        .await
        .map_err(in_flight_conflict)?;

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

/// Redeploys the image an earlier deployment produced.
///
/// No rebuild: the point of a rollback is to return to the exact artefact that
/// was known to work, and rebuilding the same commit could produce a different
/// image if a base image or dependency has moved.
pub async fn rollback(state: &AppState, deployment_id: Uuid, owner_id: Uuid) -> Result<Deployment> {
    let source = get(state, deployment_id, owner_id).await?;

    let Some(_) = source.image_ref.as_deref() else {
        return Err(Error::Invalid(
            "that deployment never produced an image, so there is nothing to roll back to".into(),
        ));
    };
    if source.status != "deployed" {
        return Err(Error::Invalid(
            "only a deployment that reached 'deployed' can be rolled back to".into(),
        ));
    }

    let app = applications::get(&state.db, source.app_id, owner_id).await?;
    let deployment = deployments::create_rollback(&state.db, app.id, &source)
        .await
        .map_err(in_flight_conflict)?;

    let payload = serde_json::to_value(DeployPayload {
        deployment_id: deployment.id,
    })
    .map_err(|e| Error::Internal(anyhow!("could not encode deploy payload: {e}")))?;
    jobs::enqueue(&state.db, KIND_DEPLOY, payload).await?;

    Ok(deployment)
}

/// A partial unique index allows one in-flight deployment per application, so
/// two concurrent deploys cannot race and let the older commit win.
fn in_flight_conflict(error: Error) -> Error {
    match error {
        Error::Database(sqlx::Error::Database(ref db_err)) if db_err.is_unique_violation() => {
            Error::Conflict("a deployment is already in progress for this application".into())
        }
        other => other,
    }
}
