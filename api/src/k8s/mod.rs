pub mod manifests;
pub mod names;
pub mod quantity;

use crate::error::Result;
use futures::{AsyncBufReadExt, TryStreamExt};
use k8s_openapi::api::{
    apps::v1::Deployment,
    batch::v1::Job,
    core::v1::{Namespace, Pod, Secret},
};
use kube::{
    api::{
        Api, ApiResource, DeleteParams, DynamicObject, GroupVersionKind, ListParams, LogParams,
        Patch, PatchParams, PropagationPolicy,
    },
    Client,
};
use std::collections::BTreeMap;

/// Identifies this controller as the owner of the fields it applies, so
/// server-side apply can reconcile rather than conflict.
pub const FIELD_MANAGER: &str = "spark-control-plane";

#[derive(Clone)]
pub struct Cluster {
    client: Client,
}

impl Cluster {
    /// Uses the ambient configuration: a kubeconfig during host development,
    /// the mounted ServiceAccount when running in-cluster.
    pub async fn connect() -> anyhow::Result<Self> {
        Ok(Self {
            client: Client::try_default().await?,
        })
    }

    /// Server-side apply, so calling this repeatedly is a no-op rather than a
    /// 409. Every manifest this control plane writes goes through the same
    /// pattern.
    pub async fn ensure_namespace(&self, name: &str) -> Result<()> {
        let api: Api<Namespace> = Api::all(self.client.clone());
        let namespace = Namespace {
            metadata: kube::api::ObjectMeta {
                name: Some(name.to_string()),
                labels: Some(BTreeMap::from([(
                    "spark.io/managed".to_string(),
                    "true".to_string(),
                )])),
                ..Default::default()
            },
            ..Default::default()
        };
        api.patch(
            name,
            &PatchParams::apply(FIELD_MANAGER),
            &Patch::Apply(&namespace),
        )
        .await?;
        Ok(())
    }

    /// Deleting the namespace takes every object belonging to the application
    /// with it, which is why per-app namespaces are worth the overhead.
    pub async fn delete_namespace(&self, name: &str) -> Result<()> {
        let api: Api<Namespace> = Api::all(self.client.clone());
        match api.delete(name, &DeleteParams::default()).await {
            Ok(_) => Ok(()),
            // Already gone is the outcome we wanted.
            Err(kube::Error::Api(e)) if e.code == 404 => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

/// Certificate state, mirrored into `domains.ssl_status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SslStatus {
    None,
    Pending,
    Ready,
    Failed,
}

impl SslStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SslStatus::None => "none",
            SslStatus::Pending => "pending",
            SslStatus::Ready => "ready",
            SslStatus::Failed => "failed",
        }
    }
}

/// Name the application Deployment is always given inside its own namespace.
const APP_DEPLOYMENT: &str = "app";

#[derive(Debug, serde::Serialize)]
pub struct PodStatus {
    pub name: String,
    pub phase: String,
    pub ready: bool,
    pub restarts: i32,
    pub message: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct AppStatus {
    pub ready: bool,
    pub replicas: i32,
    pub ready_replicas: i32,
    pub restarts: i32,
    /// Live usage summed across pods. `None` when metrics-server is not
    /// installed, which is different from zero usage and is shown as such.
    pub cpu_millicores: Option<i64>,
    pub memory_bytes: Option<i64>,
    pub pods: Vec<PodStatus>,
}

/// How a build finished, as far as the Job controller is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobOutcome {
    Succeeded,
    Failed,
    Running,
}

impl Cluster {
    /// Applies any namespaced resource by server-side apply.
    pub async fn apply<K>(&self, namespace: &str, resource: &K) -> Result<()>
    where
        K: kube::Resource<Scope = k8s_openapi::NamespaceResourceScope>
            + Clone
            + std::fmt::Debug
            + serde::de::DeserializeOwned
            + serde::Serialize,
        K::DynamicType: Default,
    {
        let name = resource.meta().name.clone().ok_or_else(|| {
            crate::error::Error::Internal(anyhow::anyhow!("resource has no name"))
        })?;
        let api: Api<K> = Api::namespaced(self.client.clone(), namespace);
        api.patch(
            &name,
            &PatchParams::apply(FIELD_MANAGER).force(),
            &Patch::Apply(resource),
        )
        .await?;
        Ok(())
    }

    /// Removes a previous build Job so a redeploy is not blocked by the
    /// immutable spec of the last one.
    pub async fn delete_job(&self, namespace: &str, name: &str) -> Result<()> {
        let api: Api<Job> = Api::namespaced(self.client.clone(), namespace);
        let params = DeleteParams {
            // Without this the Job disappears but its pods linger.
            propagation_policy: Some(PropagationPolicy::Background),
            ..Default::default()
        };
        match api.delete(name, &params).await {
            Ok(_) => Ok(()),
            Err(kube::Error::Api(e)) if e.code == 404 => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn job_outcome(&self, namespace: &str, name: &str) -> Result<JobOutcome> {
        let api: Api<Job> = Api::namespaced(self.client.clone(), namespace);
        let job = api.get(name).await?;
        let status = job.status.unwrap_or_default();

        if status.succeeded.unwrap_or(0) > 0 {
            Ok(JobOutcome::Succeeded)
        } else if status.failed.unwrap_or(0) > 0 {
            Ok(JobOutcome::Failed)
        } else {
            Ok(JobOutcome::Running)
        }
    }

    /// Name of the pod a Job created, if it has been scheduled yet.
    pub async fn job_pod(&self, namespace: &str, job_name: &str) -> Result<Option<String>> {
        let api: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
        let pods = api
            .list(&ListParams::default().labels(&format!("job-name={job_name}")))
            .await?;
        Ok(pods.items.into_iter().find_map(|p| p.metadata.name))
    }

    /// Follows a pod's log, handing each line to `on_line` as it arrives so the
    /// dashboard can show a build in progress rather than only its result.
    pub async fn follow_logs<F, Fut>(
        &self,
        namespace: &str,
        pod: &str,
        mut on_line: F,
    ) -> Result<()>
    where
        F: FnMut(String) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let api: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
        let params = LogParams {
            follow: true,
            timestamps: false,
            ..Default::default()
        };

        let mut lines = api.log_stream(pod, &params).await?.lines();
        while let Some(line) = lines
            .try_next()
            .await
            .map_err(|e| crate::error::Error::Internal(anyhow::anyhow!("log stream failed: {e}")))?
        {
            on_line(line).await?;
        }
        Ok(())
    }

    /// Live CPU and memory usage summed over an application's pods.
    ///
    /// Returns `None` when metrics-server is absent, so the dashboard can say
    /// "unavailable" rather than showing a misleading zero.
    async fn pod_usage(&self, namespace: &str, selector: &str) -> Result<Option<(i64, i64)>> {
        let gvk = GroupVersionKind::gvk("metrics.k8s.io", "v1beta1", "PodMetrics");
        let resource = ApiResource::from_gvk(&gvk);
        let api: Api<DynamicObject> =
            Api::namespaced_with(self.client.clone(), namespace, &resource);

        let metrics = match api.list(&ListParams::default().labels(selector)).await {
            Ok(metrics) => metrics,
            // metrics-server not installed.
            Err(kube::Error::Api(e)) if e.code == 404 || e.code == 503 => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let mut cpu = 0;
        let mut memory = 0;
        for pod in metrics.items {
            let containers = pod
                .data
                .get("containers")
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default();

            for container in containers {
                let usage = container.get("usage");
                if let Some(value) = usage.and_then(|u| u.get("cpu")).and_then(|v| v.as_str()) {
                    cpu += quantity::cpu_millicores(value).unwrap_or(0);
                }
                if let Some(value) = usage.and_then(|u| u.get("memory")).and_then(|v| v.as_str()) {
                    memory += quantity::memory_bytes(value).unwrap_or(0);
                }
            }
        }

        Ok(Some((cpu, memory)))
    }

    /// Reads the Ready condition of the certificate cert-manager issued for an
    /// application.
    ///
    /// The Certificate is created by ingress-shim from the Ingress annotation,
    /// so it is addressed by the TLS secret name. Read through the dynamic API
    /// rather than generated types: this is the only cert-manager resource the
    /// control plane touches, and a code-generated CRD binding for one field
    /// would be more to maintain than to parse.
    pub async fn certificate_status(&self, namespace: &str, name: &str) -> Result<SslStatus> {
        let gvk = GroupVersionKind::gvk("cert-manager.io", "v1", "Certificate");
        let resource = ApiResource::from_gvk(&gvk);
        let api: Api<DynamicObject> =
            Api::namespaced_with(self.client.clone(), namespace, &resource);

        let certificate = match api.get(name).await {
            Ok(certificate) => certificate,
            // Not requested, or cert-manager is not installed.
            Err(kube::Error::Api(e)) if e.code == 404 => return Ok(SslStatus::None),
            Err(e) => return Err(e.into()),
        };

        let ready = certificate
            .data
            .get("status")
            .and_then(|status| status.get("conditions"))
            .and_then(|conditions| conditions.as_array())
            .and_then(|conditions| {
                conditions
                    .iter()
                    .find(|c| c.get("type").and_then(|t| t.as_str()) == Some("Ready"))
            });

        Ok(
            match ready.and_then(|c| c.get("status")).and_then(|s| s.as_str()) {
                Some("True") => SslStatus::Ready,
                // cert-manager reports a hard failure as Ready=False with a reason;
                // an issuance still in flight has no condition yet.
                Some("False") => SslStatus::Failed,
                _ => SslStatus::Pending,
            },
        )
    }

    /// Pod-level view of a running application: phase, readiness and restart
    /// count, which is what the dashboard shows and what tells a user their
    /// container is crash-looping.
    pub async fn app_status(&self, namespace: &str, app_id: &str) -> Result<AppStatus> {
        let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
        let (replicas, ready_replicas) = match deployments.get(APP_DEPLOYMENT).await {
            Ok(deployment) => {
                let status = deployment.status.unwrap_or_default();
                (
                    status.replicas.unwrap_or(0),
                    status.ready_replicas.unwrap_or(0),
                )
            }
            // Never deployed.
            Err(kube::Error::Api(e)) if e.code == 404 => (0, 0),
            Err(e) => return Err(e.into()),
        };

        let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
        let selector = format!("spark.io/app={app_id}");
        let pods = pods.list(&ListParams::default().labels(&selector)).await?;

        let pods: Vec<PodStatus> = pods
            .items
            .into_iter()
            .filter(|pod| {
                // Build pods carry the same app label; only runtime pods count.
                pod.metadata
                    .labels
                    .as_ref()
                    .map(|l| !l.contains_key("job-name"))
                    .unwrap_or(true)
            })
            .map(|pod| {
                let name = pod.metadata.name.unwrap_or_default();
                let status = pod.status.unwrap_or_default();
                let containers = status.container_statuses.unwrap_or_default();

                PodStatus {
                    name,
                    phase: status.phase.unwrap_or_else(|| "Unknown".to_string()),
                    ready: containers.iter().all(|c| c.ready) && !containers.is_empty(),
                    restarts: containers.iter().map(|c| c.restart_count).sum(),
                    // Surfaces the reason a container will not start, such as
                    // an image pull failure or a crash loop.
                    message: containers.iter().find_map(|c| {
                        c.state
                            .as_ref()
                            .and_then(|s| s.waiting.as_ref())
                            .and_then(|w| w.reason.clone())
                    }),
                }
            })
            .collect();

        let usage = self.pod_usage(namespace, &selector).await.unwrap_or(None);

        Ok(AppStatus {
            ready: ready_replicas > 0,
            replicas,
            ready_replicas,
            restarts: pods.iter().map(|p| p.restarts).sum(),
            cpu_millicores: usage.map(|u| u.0),
            memory_bytes: usage.map(|u| u.1),
            pods,
        })
    }

    /// Reads an application's environment Secret.
    ///
    /// Values live only here, never in Postgres, so this is the one way to see
    /// them and it costs a cluster round-trip by design.
    pub async fn get_secret(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<BTreeMap<String, String>> {
        let api: Api<Secret> = Api::namespaced(self.client.clone(), namespace);
        let secret = match api.get(name).await {
            Ok(secret) => secret,
            Err(kube::Error::Api(e)) if e.code == 404 => return Ok(BTreeMap::new()),
            Err(e) => return Err(e.into()),
        };

        Ok(secret
            .data
            .unwrap_or_default()
            .into_iter()
            .map(|(key, value)| {
                // Values are written as UTF-8; anything else is shown as empty
                // rather than failing the whole read.
                (key, String::from_utf8(value.0).unwrap_or_default())
            })
            .collect())
    }

    pub async fn apply_secret(
        &self,
        namespace: &str,
        name: &str,
        data: BTreeMap<String, String>,
    ) -> Result<()> {
        let secret = Secret {
            metadata: kube::api::ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            // string_data takes plaintext and Kubernetes encodes it.
            string_data: Some(data),
            ..Default::default()
        };
        self.apply(namespace, &secret).await
    }

    pub async fn delete_secret(&self, namespace: &str, name: &str) -> Result<()> {
        let api: Api<Secret> = Api::namespaced(self.client.clone(), namespace);
        match api.delete(name, &DeleteParams::default()).await {
            Ok(_) => Ok(()),
            Err(kube::Error::Api(e)) if e.code == 404 => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /// Rolls pods so a changed Secret is actually picked up: envFrom values are
    /// injected at container start and do not update in place.
    pub async fn restart_deployment(&self, namespace: &str, name: &str) -> Result<()> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
        let patch = serde_json::json!({
            "spec": { "template": { "metadata": { "annotations": {
                "spark.io/restarted-at": chrono::Utc::now().to_rfc3339()
            }}}}
        });
        match api
            .patch(name, &PatchParams::default(), &Patch::Merge(&patch))
            .await
        {
            Ok(_) => Ok(()),
            // Nothing deployed yet; the next deploy will pick the values up.
            Err(kube::Error::Api(e)) if e.code == 404 => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /// Number of replicas currently passing their readiness probe.
    pub async fn ready_replicas(&self, namespace: &str, name: &str) -> Result<i32> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
        let deployment = api.get(name).await?;
        Ok(deployment
            .status
            .and_then(|s| s.ready_replicas)
            .unwrap_or(0))
    }

    /// Current phase of a pod, used to tell when a build container has
    /// started and its log can be followed.
    pub async fn pod_phase(&self, namespace: &str, pod: &str) -> Result<String> {
        let api: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
        let pod = api.get(pod).await?;
        Ok(pod.status.and_then(|s| s.phase).unwrap_or_default())
    }
}
