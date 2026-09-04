//! The deploy job: build the image, then roll it out.

use crate::{
    error::{Error, Result},
    k8s::{
        manifests::{self, AppSpec, BuildSpec, APP_RESOURCE},
        names, JobOutcome,
    },
    models::{Application, Deployment},
    repos::{applications, deployments, domains},
    state::SharedState,
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct DeployPayload {
    pub deployment_id: Uuid,
}

/// How long a single build may take before the job is abandoned.
const BUILD_TIMEOUT: Duration = Duration::from_secs(20 * 60);
/// A container that has not passed its readiness probe by now is not going to.
const ROLLOUT_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const POLL: Duration = Duration::from_secs(2);

pub async fn run(state: &SharedState, payload: &serde_json::Value) -> Result<()> {
    let payload: DeployPayload = serde_json::from_value(payload.clone())
        .map_err(|e| Error::Internal(anyhow!("malformed deploy payload: {e}")))?;

    let deployment = deployments::get(&state.db, payload.deployment_id)
        .await?
        .ok_or(Error::NotFound("deployment"))?;
    let app = applications::find_by_id(&state.db, deployment.app_id)
        .await?
        .ok_or(Error::NotFound("application"))?;

    let namespace = names::namespace(app.id);
    // Idempotent, and covers the case where the namespace was removed out of
    // band between app creation and this deploy.
    state.cluster.ensure_namespace(&namespace).await?;

    // A rollback already has an image; rebuilding it would defeat the point of
    // rolling back to a known-good artefact.
    let image = match &deployment.image_ref {
        Some(existing) if deployment.rolled_back_from.is_some() => {
            log(
                state,
                deployment.id,
                &format!("[spark] rolling back to {existing}"),
            )
            .await;
            existing.clone()
        }
        _ => {
            let image = manifests::image_ref(&state.config.registry_url, app.id, deployment.id);
            build(state, &app, &deployment, &namespace, &image).await?;
            image
        }
    };

    deployments::set_status(&state.db, deployment.id, "deploying").await?;
    rollout(state, &app, &namespace, &image).await?;

    deployments::finish(&state.db, deployment.id, "deployed", Some(&image)).await?;
    log(
        state,
        deployment.id,
        &format!("[spark] deployed to http://{}", host(state, &app)),
    )
    .await;
    Ok(())
}

async fn build(
    state: &SharedState,
    app: &Application,
    deployment: &Deployment,
    namespace: &str,
    image: &str,
) -> Result<()> {
    deployments::mark_building(&state.db, deployment.id).await?;

    let job_name = manifests::build_job_name(deployment.id);
    // A Job spec is immutable, so a retry must start from a clean one.
    state.cluster.delete_job(namespace, &job_name).await?;

    let job = manifests::build_job(&BuildSpec {
        deployment_id: deployment.id,
        app_id: app.id,
        namespace: namespace.to_string(),
        git_repo: app.git_repo.clone(),
        git_ref: deployment.commit_sha.clone(),
        dockerfile_path: app.dockerfile_path.clone(),
        image_ref: image.to_string(),
        registry_insecure: state.config.registry_insecure,
        git_token: app.git_credentials_set,
        cache_ref: state
            .config
            .build_cache
            .then(|| manifests::cache_ref(&state.config.registry_url, app.id)),
    });

    // Deleting is not instant; apply until the old Job is really gone.
    let deadline = tokio::time::Instant::now() + BUILD_TIMEOUT;
    loop {
        match state.cluster.apply(namespace, &job).await {
            Ok(()) => break,
            Err(error) if tokio::time::Instant::now() < deadline => {
                tracing::debug!(%error, "waiting for previous build job to clear");
                tokio::time::sleep(POLL).await;
            }
            Err(error) => return Err(error),
        }
    }

    stream_build_logs(state, deployment.id, namespace, &job_name).await;

    // The log ending does not mean the Job has been marked complete yet.
    loop {
        match state.cluster.job_outcome(namespace, &job_name).await? {
            JobOutcome::Succeeded => return Ok(()),
            JobOutcome::Failed => {
                return Err(Error::Invalid(
                    "build failed; see the deployment log".to_string(),
                ))
            }
            JobOutcome::Running if tokio::time::Instant::now() < deadline => {
                tokio::time::sleep(POLL).await;
            }
            JobOutcome::Running => {
                return Err(Error::Internal(anyhow!("build timed out")));
            }
        }
    }
}

fn host(state: &SharedState, app: &Application) -> String {
    names::default_host(&app.name, &state.config.app_base_domain)
}

/// Applies the runtime objects and waits for the new pod to become ready, so a
/// deployment is only reported as deployed once it can actually serve traffic.
async fn rollout(
    state: &SharedState,
    app: &Application,
    namespace: &str,
    image: &str,
) -> Result<()> {
    // The generated host first, then any custom domains.
    let mut hosts = vec![host(state, app)];
    hosts.extend(
        domains::list_by_app(&state.db, app.id)
            .await?
            .into_iter()
            .map(|d| d.domain_name),
    );

    let spec = AppSpec {
        app_id: app.id,
        namespace: namespace.to_string(),
        image: image.to_string(),
        container_port: app.container_port,
        cpu_limit: app.cpu_limit.clone(),
        memory_limit: app.memory_limit.clone(),
        replicas: app.replicas,
        hosts,
    };

    state
        .cluster
        .apply(namespace, &manifests::app_deployment(&spec))
        .await?;
    state
        .cluster
        .apply(namespace, &manifests::app_service(&spec))
        .await?;
    state
        .cluster
        .apply(
            namespace,
            &manifests::app_ingress(
                app.id,
                namespace,
                &spec.hosts,
                state.config.cluster_issuer.as_deref(),
            ),
        )
        .await?;
    state
        .cluster
        .apply(
            namespace,
            &manifests::app_network_policy(&spec, &state.config.cluster_cidrs),
        )
        .await?;

    // Scaled to zero: there is nothing to wait for.
    if app.replicas == 0 {
        return Ok(());
    }

    let deadline = tokio::time::Instant::now() + ROLLOUT_TIMEOUT;
    loop {
        if state
            .cluster
            .ready_replicas(namespace, APP_RESOURCE)
            .await?
            >= app.replicas
        {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(Error::Invalid(
                "the application did not become ready; check that it listens on the \
                 configured port"
                    .to_string(),
            ));
        }
        tokio::time::sleep(POLL).await;
    }
}

async fn log(state: &SharedState, deployment_id: Uuid, line: &str) {
    let _ = deployments::append_log(&state.db, deployment_id, line).await;
}

/// Copies the builder's output into the deployment log. Best effort: losing the
/// log stream should not fail an otherwise good build, so problems here are
/// recorded in the log itself rather than raised.
async fn stream_build_logs(
    state: &SharedState,
    deployment_id: Uuid,
    namespace: &str,
    job_name: &str,
) {
    let deadline = tokio::time::Instant::now() + BUILD_TIMEOUT;

    let pod = loop {
        match state.cluster.job_pod(namespace, job_name).await {
            Ok(Some(pod)) => break pod,
            Ok(None) if tokio::time::Instant::now() < deadline => {
                tokio::time::sleep(POLL).await;
            }
            Ok(None) => return,
            Err(error) => {
                tracing::warn!(%error, "could not find build pod");
                return;
            }
        }
    };

    // Logs are unavailable while the pod is still pulling its image.
    while tokio::time::Instant::now() < deadline {
        match state.cluster.pod_phase(namespace, &pod).await {
            Ok(phase) if phase != "Pending" => break,
            Ok(_) => tokio::time::sleep(POLL).await,
            Err(error) => {
                tracing::warn!(%error, "could not read build pod phase");
                return;
            }
        }
    }

    let db = state.db.clone();
    let result = state
        .cluster
        .follow_logs(namespace, &pod, |line| {
            let db = db.clone();
            async move { deployments::append_log(&db, deployment_id, &line).await }
        })
        .await;

    if let Err(error) = result {
        tracing::warn!(%error, "build log stream ended early");
        let _ = deployments::append_log(
            &state.db,
            deployment_id,
            &format!("[spark] log stream ended early: {error}"),
        )
        .await;
    }
}

/// Records a terminal failure on the deployment once the queue stops retrying.
pub async fn mark_failed(state: &SharedState, payload: &serde_json::Value, message: &str) {
    let Ok(payload) = serde_json::from_value::<DeployPayload>(payload.clone()) else {
        return;
    };
    let _ = deployments::append_log(
        &state.db,
        payload.deployment_id,
        &format!("[spark] deployment failed: {message}"),
    )
    .await;
    let _ = deployments::finish(&state.db, payload.deployment_id, "failed", None).await;
}
