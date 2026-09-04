pub mod auth;
pub mod config;
pub mod error;
pub mod git;
pub mod handlers;
pub mod k8s;
pub mod models;
pub mod queue;
pub mod repos;
pub mod services;
pub mod state;

use axum::{
    routing::{get, post},
    Router,
};
use state::SharedState;
use tower_http::trace::TraceLayer;

pub fn router(state: SharedState) -> Router {
    let api = Router::new()
        .route("/auth/signup", post(handlers::auth::signup))
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/auth/me", get(handlers::auth::me))
        .route(
            "/apps",
            get(handlers::apps::list).post(handlers::apps::create),
        )
        .route("/apps/{id}/deploy", post(handlers::deployments::deploy))
        .route("/apps/{id}/deployments", get(handlers::deployments::list))
        .route("/deployments/{id}", get(handlers::deployments::get))
        .route("/deployments/{id}/logs", get(handlers::deployments::logs))
        .route("/apps/{id}/webhook", get(handlers::apps::webhook))
        .route("/apps/{id}/health", get(handlers::apps::health))
        .route("/webhooks/github/{id}", post(handlers::webhooks::github))
        .route("/webhooks/gitlab/{id}", post(handlers::webhooks::gitlab))
        .route(
            "/apps/{id}/env",
            get(handlers::env::list).put(handlers::env::set),
        )
        .route(
            "/apps/{id}/env/{key}",
            get(handlers::env::reveal).delete(handlers::env::remove),
        )
        .route(
            "/apps/{id}/domains",
            get(handlers::domains::list).post(handlers::domains::add),
        )
        .route(
            "/apps/{id}/domains/{domain_id}",
            axum::routing::delete(handlers::domains::remove),
        )
        .route(
            "/apps/{id}",
            get(handlers::apps::get)
                .patch(handlers::apps::update)
                .delete(handlers::apps::delete),
        );

    Router::new()
        .route("/health", get(handlers::health::health))
        .nest("/api/v1", api)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
