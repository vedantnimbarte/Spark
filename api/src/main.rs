use anyhow::{Context, Result};
use spark_api::{
    auth::rate_limit::RateLimiter, config::Config, k8s::Cluster, queue, router, state::AppState,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // A missing .env is fine in production, where the environment is supplied
    // by the container.
    let _ = dotenvy::dotenv();

    // sqlx and reqwest each pull in a rustls crypto provider, so rustls cannot
    // pick one on its own. Choosing here fails loudly at startup instead of
    // panicking on the first TLS connection.
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("could not install the rustls crypto provider"))?;

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "spark_api=debug,tower_http=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("could not connect to Postgres; is `docker compose up -d` running?")?;

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .context("database migrations failed")?;

    let cluster = Cluster::connect()
        .await
        .context("could not reach Kubernetes; check `kubectl get nodes`")?;

    let bind_addr = config.bind_addr;
    let state = Arc::new(AppState {
        db,
        config,
        cluster,
        login_limiter: RateLimiter::new(),
    });

    queue::spawn(state.clone());

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("could not bind {bind_addr}"))?;
    tracing::info!(%bind_addr, "control plane listening");

    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    Ok(())
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "could not install ctrl-c handler");
    }
    tracing::info!("shutting down");
}
