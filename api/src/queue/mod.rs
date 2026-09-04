//! Background worker.
//!
//! Work is claimed from the `jobs` table rather than held in memory, so a
//! deployment survives the control plane restarting and is retried if the
//! cluster is briefly unreachable — the reliability requirement in the PRD.

pub mod deploy;
pub mod maintenance;

use crate::{repos::jobs, state::SharedState};
use std::time::Duration;

pub const KIND_DEPLOY: &str = "deploy";

/// A lock older than this belongs to a worker that died; the job is reclaimed.
const STALE_LOCK_MINUTES: i32 = 30;
const MAX_ATTEMPTS: i32 = 3;
const IDLE_POLL: Duration = Duration::from_secs(2);
/// Housekeeping is slow-moving; running it often would be pure noise.
const MAINTENANCE_EVERY: Duration = Duration::from_secs(60 * 60);

pub fn spawn(state: SharedState) {
    spawn_maintenance(state.clone());

    tokio::spawn(async move {
        loop {
            match run_once(&state).await {
                // Something was processed; look for more work immediately.
                Ok(true) => continue,
                Ok(false) => {}
                Err(error) => {
                    tracing::error!(%error, "worker loop failed; backing off");
                }
            }
            tokio::time::sleep(IDLE_POLL).await;
        }
    });
}

async fn run_once(state: &SharedState) -> crate::error::Result<bool> {
    let Some(job) = jobs::claim(&state.db, STALE_LOCK_MINUTES).await? else {
        return Ok(false);
    };

    let result = match job.kind.as_str() {
        KIND_DEPLOY => deploy::run(state, &job.payload).await,
        other => Err(crate::error::Error::Internal(anyhow::anyhow!(
            "unknown job kind {other}"
        ))),
    };

    match result {
        Ok(()) => jobs::complete(&state.db, job.id).await?,
        Err(error) => {
            let message = error.to_string();
            if job.attempts >= MAX_ATTEMPTS {
                tracing::error!(job = job.id, attempts = job.attempts, %message, "giving up");
                deploy::mark_failed(state, &job.payload, &message).await;
                jobs::complete(&state.db, job.id).await?;
            } else {
                // Linear backoff is enough here: the failures worth retrying
                // are a briefly unreachable cluster, not a thundering herd.
                let delay = 10 * job.attempts;
                tracing::warn!(job = job.id, attempts = job.attempts, %message, "retrying");
                jobs::retry_later(&state.db, job.id, &message, delay).await?;
            }
        }
    }

    Ok(true)
}

/// Periodic cleanup, kept out of the job loop so a long prune never delays a
/// deployment.
fn spawn_maintenance(state: SharedState) {
    tokio::spawn(async move {
        loop {
            if let Err(error) = maintenance::run(&state).await {
                tracing::warn!(%error, "maintenance pass failed");
            }
            tokio::time::sleep(MAINTENANCE_EVERY).await;
        }
    });
}
