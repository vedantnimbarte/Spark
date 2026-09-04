//! Pure manifest construction.
//!
//! Everything here is a plain function from a spec to a Kubernetes object, so
//! the part that is easy to get quietly wrong is testable without a cluster.

use k8s_openapi::{
    api::{
        apps::v1::{Deployment, DeploymentSpec},
        batch::v1::{Job, JobSpec},
        core::v1::{
            AppArmorProfile, Container, ContainerPort, EnvFromSource, EnvVar, PodSpec,
            PodTemplateSpec, Probe, ResourceRequirements, SeccompProfile, SecretEnvSource,
            SecurityContext, Service, ServicePort, ServiceSpec, TCPSocketAction,
        },
        networking::v1::{
            HTTPIngressPath, HTTPIngressRuleValue, IPBlock, Ingress, IngressBackend, IngressRule,
            IngressServiceBackend, IngressSpec, IngressTLS, NetworkPolicy, NetworkPolicyEgressRule,
            NetworkPolicyPeer, NetworkPolicyPort, NetworkPolicySpec, ServiceBackendPort,
        },
    },
    apimachinery::pkg::{
        api::resource::Quantity, apis::meta::v1::LabelSelector, util::intstr::IntOrString,
    },
};
use kube::api::ObjectMeta;
use std::collections::BTreeMap;
use uuid::Uuid;

/// Rootless BuildKit, replacing the archived Kaniko.
const BUILDKIT_IMAGE: &str = "moby/buildkit:v0.19.0-rootless";

pub struct BuildSpec {
    pub deployment_id: Uuid,
    pub app_id: Uuid,
    pub namespace: String,
    pub git_repo: String,
    /// A commit SHA when it could be resolved, otherwise a branch name.
    pub git_ref: String,
    pub dockerfile_path: String,
    pub image_ref: String,
    pub registry_insecure: bool,
}

pub fn build_job_name(deployment_id: Uuid) -> String {
    format!("build-{deployment_id}")
}

pub fn labels(app_id: Uuid) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("spark.io/app".to_string(), app_id.to_string()),
        (
            "app.kubernetes.io/managed-by".to_string(),
            "spark".to_string(),
        ),
    ])
}

pub fn build_job(spec: &BuildSpec) -> Job {
    let mut output = format!("type=image,name={},push=true", spec.image_ref);
    if spec.registry_insecure {
        output.push_str(",registry.insecure=true");
    }

    let args = vec![
        "build".to_string(),
        "--frontend=dockerfile.v0".to_string(),
        format!("--opt=context={}#{}", spec.git_repo, spec.git_ref),
        format!("--opt=filename={}", spec.dockerfile_path),
        format!("--output={output}"),
        // Plain progress keeps the pod log readable as a build log.
        "--progress=plain".to_string(),
    ];

    let mut job_labels = labels(spec.app_id);
    job_labels.insert(
        "spark.io/deployment".to_string(),
        spec.deployment_id.to_string(),
    );

    Job {
        metadata: ObjectMeta {
            name: Some(build_job_name(spec.deployment_id)),
            namespace: Some(spec.namespace.clone()),
            labels: Some(job_labels.clone()),
            ..Default::default()
        },
        spec: Some(JobSpec {
            // Retries are the queue's job, not the Job controller's; a second
            // attempt here would produce a second, confusing log stream.
            backoff_limit: Some(0),
            ttl_seconds_after_finished: Some(3600),
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(job_labels),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    restart_policy: Some("Never".to_string()),
                    // The registry is published on a NodePort, so the builder
                    // shares the node's network to reach it at exactly the
                    // address the kubelet will later pull from.
                    host_network: Some(true),
                    containers: vec![Container {
                        name: "buildkit".to_string(),
                        image: Some(BUILDKIT_IMAGE.to_string()),
                        command: Some(vec!["buildctl-daemonless.sh".to_string()]),
                        args: Some(args),
                        env: Some(vec![EnvVar {
                            // Required for rootless BuildKit without a
                            // privileged container.
                            name: "BUILDKITD_FLAGS".to_string(),
                            value: Some("--oci-worker-no-process-sandbox".to_string()),
                            ..Default::default()
                        }]),
                        security_context: Some(SecurityContext {
                            run_as_user: Some(1000),
                            run_as_group: Some(1000),
                            seccomp_profile: Some(SeccompProfile {
                                type_: "Unconfined".to_string(),
                                ..Default::default()
                            }),
                            app_armor_profile: Some(AppArmorProfile {
                                type_: "Unconfined".to_string(),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// `<registry>/spark/<app id>:<deployment id>`.
///
/// Keyed on ids rather than the application name: names are unique per owner
/// but not globally, and two users' `blog` must not collide in one registry.
pub fn image_ref(registry: &str, app_id: Uuid, deployment_id: Uuid) -> String {
    format!("{registry}/spark/{app_id}:{deployment_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> BuildSpec {
        BuildSpec {
            deployment_id: Uuid::from_u128(2),
            app_id: Uuid::from_u128(1),
            namespace: "spark-app-x".to_string(),
            git_repo: "https://github.com/traefik/whoami.git".to_string(),
            git_ref: "abc123".to_string(),
            dockerfile_path: "Dockerfile".to_string(),
            image_ref: "localhost:30500/spark/app:dep".to_string(),
            registry_insecure: true,
        }
    }

    fn args_of(job: &Job) -> Vec<String> {
        job.spec
            .as_ref()
            .and_then(|s| s.template.spec.as_ref())
            .and_then(|s| s.containers.first())
            .and_then(|c| c.args.clone())
            .unwrap_or_default()
    }

    #[test]
    fn build_pins_the_context_to_the_resolved_ref() {
        let args = args_of(&build_job(&spec()));
        assert!(
            args.contains(
                &"--opt=context=https://github.com/traefik/whoami.git#abc123".to_string()
            ),
            "context must carry the ref so the build is reproducible: {args:?}"
        );
    }

    #[test]
    fn insecure_flag_follows_the_registry_setting() {
        let args = args_of(&build_job(&spec()));
        assert!(args.iter().any(|a| a.contains("registry.insecure=true")));

        let secure = BuildSpec {
            registry_insecure: false,
            ..spec()
        };
        let args = args_of(&build_job(&secure));
        assert!(
            !args.iter().any(|a| a.contains("registry.insecure")),
            "an external registry must not be pushed to insecurely: {args:?}"
        );
    }

    #[test]
    fn builder_shares_the_node_network_to_reach_the_nodeport_registry() {
        let job = build_job(&spec());
        let pod = job.spec.as_ref().and_then(|s| s.template.spec.as_ref());
        assert_eq!(pod.and_then(|p| p.host_network), Some(true));
    }

    #[test]
    fn retries_are_left_to_the_queue() {
        let job = build_job(&spec());
        assert_eq!(job.spec.and_then(|s| s.backoff_limit), Some(0));
    }

    #[test]
    fn image_refs_are_keyed_on_ids_not_names() {
        let a = Uuid::from_u128(1);
        let d = Uuid::from_u128(2);
        assert_eq!(
            image_ref("localhost:30500", a, d),
            format!("localhost:30500/spark/{a}:{d}")
        );
    }
}

// ---------------------------------------------------------------------------
// Application runtime objects
// ---------------------------------------------------------------------------

pub struct AppSpec {
    pub app_id: Uuid,
    pub namespace: String,
    pub image: String,
    pub container_port: i32,
    pub cpu_limit: String,
    pub memory_limit: String,
    /// Default host first, then any custom domains.
    pub hosts: Vec<String>,
}

/// Name shared by the Deployment, Service and Ingress inside the application's
/// own namespace, so there is nothing to disambiguate.
pub const APP_RESOURCE: &str = "app";
/// Secret holding the application's environment variables.
pub const ENV_SECRET: &str = "app-env";

fn selector(app_id: Uuid) -> BTreeMap<String, String> {
    BTreeMap::from([("spark.io/app".to_string(), app_id.to_string())])
}

pub fn app_deployment(spec: &AppSpec) -> Deployment {
    let pod_labels = {
        let mut l = labels(spec.app_id);
        l.extend(selector(spec.app_id));
        l
    };

    Deployment {
        metadata: ObjectMeta {
            name: Some(APP_RESOURCE.to_string()),
            namespace: Some(spec.namespace.clone()),
            labels: Some(labels(spec.app_id)),
            ..Default::default()
        },
        spec: Some(DeploymentSpec {
            replicas: Some(1),
            selector: LabelSelector {
                match_labels: Some(selector(spec.app_id)),
                ..Default::default()
            },
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(pod_labels),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: APP_RESOURCE.to_string(),
                        image: Some(spec.image.clone()),
                        ports: Some(vec![ContainerPort {
                            container_port: spec.container_port,
                            name: Some("http".to_string()),
                            ..Default::default()
                        }]),
                        // Optional: environment variables may not have been set
                        // yet, and a missing Secret must not wedge the rollout.
                        env_from: Some(vec![EnvFromSource {
                            secret_ref: Some(SecretEnvSource {
                                name: ENV_SECRET.to_string(),
                                optional: Some(true),
                            }),
                            ..Default::default()
                        }]),
                        resources: Some(ResourceRequirements {
                            limits: Some(BTreeMap::from([
                                ("cpu".to_string(), Quantity(spec.cpu_limit.clone())),
                                ("memory".to_string(), Quantity(spec.memory_limit.clone())),
                            ])),
                            // Deliberately below the limits: reserving the full
                            // limit would let a handful of idle applications
                            // exhaust a small cluster's schedulable capacity.
                            requests: Some(BTreeMap::from([
                                ("cpu".to_string(), Quantity("50m".to_string())),
                                ("memory".to_string(), Quantity("64Mi".to_string())),
                            ])),
                            ..Default::default()
                        }),
                        // A TCP check is the only readiness signal that works
                        // for an arbitrary user application.
                        readiness_probe: Some(Probe {
                            tcp_socket: Some(TCPSocketAction {
                                port: IntOrString::Int(spec.container_port),
                                ..Default::default()
                            }),
                            initial_delay_seconds: Some(2),
                            period_seconds: Some(5),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub fn app_service(spec: &AppSpec) -> Service {
    Service {
        metadata: ObjectMeta {
            name: Some(APP_RESOURCE.to_string()),
            namespace: Some(spec.namespace.clone()),
            labels: Some(labels(spec.app_id)),
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            selector: Some(selector(spec.app_id)),
            ports: Some(vec![ServicePort {
                port: 80,
                target_port: Some(IntOrString::Int(spec.container_port)),
                name: Some("http".to_string()),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Secret cert-manager writes the issued certificate into.
pub const TLS_SECRET: &str = "app-tls";

/// Builds the application Ingress.
///
/// When `cluster_issuer` is set, a `tls` block and the cert-manager annotation
/// are added and ingress-shim creates the Certificate; there is no separate
/// Certificate object to keep in step. When it is `None`, the Ingress is plain
/// HTTP exactly as before.
pub fn app_ingress(
    app_id: Uuid,
    namespace: &str,
    hosts: &[String],
    cluster_issuer: Option<&str>,
) -> Ingress {
    let rules = hosts
        .iter()
        .map(|host| IngressRule {
            host: Some(host.clone()),
            http: Some(HTTPIngressRuleValue {
                paths: vec![HTTPIngressPath {
                    path: Some("/".to_string()),
                    path_type: "Prefix".to_string(),
                    backend: IngressBackend {
                        service: Some(IngressServiceBackend {
                            name: APP_RESOURCE.to_string(),
                            port: Some(ServiceBackendPort {
                                number: Some(80),
                                ..Default::default()
                            }),
                        }),
                        ..Default::default()
                    },
                }],
            }),
        })
        .collect();

    // One certificate covering every host on the Ingress.
    let tls = cluster_issuer.map(|_| {
        vec![IngressTLS {
            hosts: Some(hosts.to_vec()),
            secret_name: Some(TLS_SECRET.to_string()),
        }]
    });

    let annotations = cluster_issuer.map(|issuer| {
        BTreeMap::from([(
            "cert-manager.io/cluster-issuer".to_string(),
            issuer.to_string(),
        )])
    });

    Ingress {
        metadata: ObjectMeta {
            name: Some(APP_RESOURCE.to_string()),
            namespace: Some(namespace.to_string()),
            labels: Some(labels(app_id)),
            annotations,
            ..Default::default()
        },
        spec: Some(IngressSpec {
            ingress_class_name: Some("traefik".to_string()),
            rules: Some(rules),
            tls,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Keeps user workloads off the cluster's internal networks while leaving the
/// internet reachable, implementing the control plane / data plane boundary.
///
/// ponytail: expressed as "everywhere except the private ranges" rather than a
/// precise peer selector, because that is one rule instead of a maintained
/// inventory of what the control plane happens to run. Narrow it if user
/// applications ever need to reach a specific in-cluster service.
pub fn app_network_policy(spec: &AppSpec, cluster_cidrs: &[String]) -> NetworkPolicy {
    NetworkPolicy {
        metadata: ObjectMeta {
            name: Some("isolate-control-plane".to_string()),
            namespace: Some(spec.namespace.clone()),
            labels: Some(labels(spec.app_id)),
            ..Default::default()
        },
        spec: Some(NetworkPolicySpec {
            // An empty selector applies to every pod in the namespace.
            pod_selector: Some(LabelSelector::default()),
            policy_types: Some(vec!["Egress".to_string()]),
            egress: Some(vec![
                // DNS has to stay reachable or nothing resolves at all.
                NetworkPolicyEgressRule {
                    to: Some(vec![NetworkPolicyPeer {
                        namespace_selector: Some(LabelSelector {
                            match_labels: Some(BTreeMap::from([(
                                "kubernetes.io/metadata.name".to_string(),
                                "kube-system".to_string(),
                            )])),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }]),
                    ports: Some(vec![
                        NetworkPolicyPort {
                            port: Some(IntOrString::Int(53)),
                            protocol: Some("UDP".to_string()),
                            ..Default::default()
                        },
                        NetworkPolicyPort {
                            port: Some(IntOrString::Int(53)),
                            protocol: Some("TCP".to_string()),
                            ..Default::default()
                        },
                    ]),
                },
                NetworkPolicyEgressRule {
                    to: Some(vec![NetworkPolicyPeer {
                        ip_block: Some(IPBlock {
                            cidr: "0.0.0.0/0".to_string(),
                            except: Some(cluster_cidrs.to_vec()),
                        }),
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        }),
    }
}

#[cfg(test)]
mod app_tests {
    use super::*;

    fn app_spec() -> AppSpec {
        AppSpec {
            app_id: Uuid::from_u128(7),
            namespace: "spark-app-7".to_string(),
            image: "localhost:30500/spark/a:b".to_string(),
            container_port: 3000,
            cpu_limit: "250m".to_string(),
            memory_limit: "256Mi".to_string(),
            hosts: vec!["blog.localhost".to_string(), "example.com".to_string()],
        }
    }

    #[test]
    fn pods_carry_the_label_the_service_selects_on() {
        let spec = app_spec();
        let deployment = app_deployment(&spec);
        let service = app_service(&spec);

        let pod_labels = deployment
            .spec
            .and_then(|s| s.template.metadata)
            .and_then(|m| m.labels)
            .unwrap_or_default();
        let service_selector = service.spec.and_then(|s| s.selector).unwrap_or_default();

        // A mismatch here produces a Service that silently routes nowhere.
        for (key, value) in &service_selector {
            assert_eq!(
                pod_labels.get(key),
                Some(value),
                "pod is missing the selector label {key}"
            );
        }
        assert!(!service_selector.is_empty(), "selector must not be empty");
    }

    #[test]
    fn resource_limits_come_from_the_application_record() {
        let deployment = app_deployment(&app_spec());
        let resources = deployment
            .spec
            .and_then(|s| s.template.spec)
            .and_then(|s| s.containers.into_iter().next())
            .and_then(|c| c.resources)
            .unwrap_or_default();

        let limits = resources.limits.unwrap_or_default();
        assert_eq!(limits.get("cpu"), Some(&Quantity("250m".to_string())));
        assert_eq!(limits.get("memory"), Some(&Quantity("256Mi".to_string())));

        // Requests must stay at or below limits or the pod is unschedulable.
        let requests = resources.requests.unwrap_or_default();
        assert!(requests.contains_key("cpu") && requests.contains_key("memory"));
    }

    #[test]
    fn service_targets_the_container_port_but_publishes_80() {
        let service = app_service(&app_spec());
        let port = service
            .spec
            .and_then(|s| s.ports)
            .and_then(|p| p.into_iter().next())
            .expect("service must expose a port");

        assert_eq!(port.port, 80, "ingress always talks to port 80");
        assert_eq!(port.target_port, Some(IntOrString::Int(3000)));
    }

    #[test]
    fn ingress_routes_every_host_to_the_app_service() {
        let spec = app_spec();
        let ingress = app_ingress(spec.app_id, &spec.namespace, &spec.hosts, None);
        let rules = ingress
            .spec
            .as_ref()
            .and_then(|s| s.rules.clone())
            .unwrap_or_default();

        let hosts: Vec<_> = rules.iter().filter_map(|r| r.host.clone()).collect();
        assert_eq!(hosts, vec!["blog.localhost", "example.com"]);

        assert_eq!(
            ingress.spec.and_then(|s| s.ingress_class_name).as_deref(),
            Some("traefik")
        );
    }

    #[test]
    fn no_issuer_configured_means_no_tls_block() {
        let spec = app_spec();
        let ingress = app_ingress(spec.app_id, &spec.namespace, &spec.hosts, None);
        assert!(
            ingress.spec.and_then(|s| s.tls).is_none(),
            "an empty tls block would make Traefik serve a certificate it does not have"
        );
        assert!(ingress.metadata.annotations.is_none());
    }

    #[test]
    fn issuer_adds_the_annotation_and_covers_every_host() {
        let spec = app_spec();
        let ingress = app_ingress(
            spec.app_id,
            &spec.namespace,
            &spec.hosts,
            Some("spark-selfsigned"),
        );

        // ingress-shim reads this annotation to create the Certificate.
        assert_eq!(
            ingress
                .metadata
                .annotations
                .as_ref()
                .and_then(|a| a.get("cert-manager.io/cluster-issuer"))
                .map(String::as_str),
            Some("spark-selfsigned")
        );

        let tls = ingress.spec.and_then(|s| s.tls).unwrap_or_default();
        let entry = tls.first().expect("a tls entry");
        // A host missing here is a host served with the wrong certificate.
        assert_eq!(entry.hosts.as_deref(), Some(spec.hosts.as_slice()));
        assert_eq!(entry.secret_name.as_deref(), Some(TLS_SECRET));
    }

    #[test]
    fn egress_policy_permits_dns_and_excludes_cluster_ranges() {
        let cidrs = vec!["10.96.0.0/12".to_string(), "10.244.0.0/16".to_string()];
        let policy = app_network_policy(&app_spec(), &cidrs);
        let egress = policy
            .spec
            .and_then(|s| s.egress)
            .expect("policy must have egress rules");

        let dns = egress.iter().any(|r| {
            r.ports
                .iter()
                .flatten()
                .any(|p| p.port == Some(IntOrString::Int(53)))
        });
        assert!(dns, "blocking DNS would break every application");

        let excepted = egress
            .iter()
            .flat_map(|r| r.to.iter().flatten())
            .filter_map(|peer| peer.ip_block.as_ref())
            .filter_map(|block| block.except.clone())
            .flatten()
            .collect::<Vec<_>>();
        assert_eq!(
            excepted, cidrs,
            "cluster ranges must be excluded from egress"
        );
    }
}
