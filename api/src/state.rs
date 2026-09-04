use crate::{auth::rate_limit::RateLimiter, config::Config, k8s::Cluster};
use sqlx::PgPool;
use std::sync::Arc;

pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    pub cluster: Cluster,
    pub login_limiter: RateLimiter,
}

pub type SharedState = Arc<AppState>;
