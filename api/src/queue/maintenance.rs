//! Periodic housekeeping.
//!
//! Everything here is unbounded growth that nothing else cleans up: expired
//! sessions, deployment history, and the images those deployments pushed.

use crate::{
    error::Result,
    k8s::manifests,
    repos::{deployments, sessions},
    state::SharedState,
};

/// Deployments kept per application. Enough to roll back through a bad run,
/// small enough that log lines and images do not accumulate forever.
pub const KEEP_PER_APP: i64 = 20;

pub async fn run(state: &SharedState) -> Result<()> {
    let sessions = sessions::delete_expired(&state.db).await?;
    if sessions > 0 {
        tracing::info!(sessions, "removed expired sessions");
    }

    prune_deployments(state).await?;
    Ok(())
}

/// Deletes old deployment rows and the images they pushed.
///
/// The row goes first only after the image delete is attempted: a registry that
/// refuses the delete leaves a reclaimable blob, whereas losing the row first
/// would leave an image nothing points at.
async fn prune_deployments(state: &SharedState) -> Result<()> {
    let stale = deployments::stale(&state.db, KEEP_PER_APP).await?;
    if stale.is_empty() {
        return Ok(());
    }

    for deployment in &stale {
        let Some(image) = &deployment.image_ref else {
            continue;
        };
        if let Err(error) = delete_image(state, image).await {
            // Not fatal: the row is still worth removing, and the registry's
            // own garbage collection can reclaim the blob later.
            tracing::warn!(%image, %error, "could not delete image");
        }
    }

    let ids: Vec<_> = stale.iter().map(|d| d.id).collect();
    let removed = deployments::delete_many(&state.db, &ids).await?;
    tracing::info!(removed, "pruned old deployments");
    Ok(())
}

/// Removes a manifest from the registry over the v2 API.
///
/// Deleting a manifest only unlinks it; the registry reclaims disk on its own
/// garbage-collection run, which is a separate operation an operator triggers.
async fn delete_image(state: &SharedState, image: &str) -> Result<()> {
    let Some((repository, reference)) = manifests::split_image_ref(image) else {
        return Ok(());
    };

    let scheme = if state.config.registry_insecure {
        "http"
    } else {
        "https"
    };
    let base = format!("{scheme}://{}/v2/{repository}", state.config.registry_url);
    let client = reqwest::Client::new();

    // A manifest can only be deleted by digest, so resolve the tag first.
    let head = client
        .head(format!("{base}/manifests/{reference}"))
        .header(
            "Accept",
            "application/vnd.oci.image.index.v1+json, \
             application/vnd.docker.distribution.manifest.list.v2+json, \
             application/vnd.oci.image.manifest.v1+json, \
             application/vnd.docker.distribution.manifest.v2+json",
        )
        .send()
        .await
        .map_err(|e| crate::error::Error::Internal(anyhow::anyhow!("registry HEAD failed: {e}")))?;

    if !head.status().is_success() {
        // Already gone.
        return Ok(());
    }

    let Some(digest) = head
        .headers()
        .get("Docker-Content-Digest")
        .and_then(|v| v.to_str().ok())
    else {
        return Ok(());
    };

    client
        .delete(format!("{base}/manifests/{digest}"))
        .send()
        .await
        .map_err(|e| {
            crate::error::Error::Internal(anyhow::anyhow!("registry DELETE failed: {e}"))
        })?;

    Ok(())
}
