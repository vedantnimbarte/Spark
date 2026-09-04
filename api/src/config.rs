use anyhow::{anyhow, Context, Result};
use std::net::SocketAddr;

/// Runtime configuration, read once at startup from the environment.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    /// Apps are published at `{name}.{app_base_domain}`.
    pub app_base_domain: String,
    /// Registry the build job pushes to and the kubelet pulls from.
    pub registry_url: String,
    pub registry_insecure: bool,
    /// Internal ranges user workloads are kept off by the per-app
    /// NetworkPolicy. Defaults match a stock Docker Desktop cluster.
    pub cluster_cidrs: Vec<String>,
    /// cert-manager ClusterIssuer applied to application Ingresses. Empty
    /// disables TLS entirely and keeps the plain-HTTP behaviour.
    pub cluster_issuer: Option<String>,
    /// Set on the session cookie. Must be false when the dashboard is served
    /// over plain HTTP, or the browser silently discards the cookie.
    pub cookie_secure: bool,
    /// Import and export a BuildKit layer cache in the registry between builds.
    pub build_cache: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            bind_addr: optional("BIND_ADDR", "0.0.0.0:8080")
                .parse()
                .context("BIND_ADDR must be a socket address such as 0.0.0.0:8080")?,
            app_base_domain: optional("APP_BASE_DOMAIN", "localhost"),
            registry_url: optional("REGISTRY_URL", "localhost:30500"),
            registry_insecure: optional("REGISTRY_INSECURE", "true")
                .parse()
                .context("REGISTRY_INSECURE must be true or false")?,
            cluster_issuer: {
                let issuer = optional("CLUSTER_ISSUER", "spark-selfsigned");
                (!issuer.trim().is_empty()).then_some(issuer)
            },
            cookie_secure: optional("COOKIE_SECURE", "false")
                .parse()
                .context("COOKIE_SECURE must be true or false")?,
            build_cache: optional("BUILD_CACHE", "true")
                .parse()
                .context("BUILD_CACHE must be true or false")?,
            cluster_cidrs: optional("CLUSTER_CIDRS", "10.96.0.0/12,10.244.0.0/16")
                .split(',')
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
                .collect(),
        })
    }
}

fn required(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| anyhow!("required environment variable {key} is not set"))
}

fn optional(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
