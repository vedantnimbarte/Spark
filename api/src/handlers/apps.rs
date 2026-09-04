use crate::{
    auth::session::CurrentUser,
    error::Result,
    models::Application,
    services::{
        applications::{self as svc, CreateApplication, UpdateApplication},
        git_credentials,
    },
    state::SharedState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

pub async fn list(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<Application>>> {
    Ok(Json(svc::list(&state.db, user.id).await?))
}

pub async fn create(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<CreateApplication>,
) -> Result<(StatusCode, Json<Application>)> {
    let app = svc::create(&state, user.id, body).await?;
    Ok((StatusCode::CREATED, Json(app)))
}

pub async fn get(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Application>> {
    Ok(Json(svc::get(&state.db, id, user.id).await?))
}

pub async fn update(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateApplication>,
) -> Result<Json<Application>> {
    Ok(Json(svc::update(&state.db, id, user.id, body).await?))
}

pub async fn delete(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    svc::delete(&state, id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Serialize)]
pub struct WebhookConfig {
    pub github_url: String,
    pub gitlab_url: String,
    /// Shown so the owner can paste it into the provider. It is never included
    /// in the application record itself.
    pub secret: String,
}

/// The values needed to configure a push webhook at the git provider.
pub async fn webhook(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<WebhookConfig>> {
    let app = svc::get(&state.db, id, user.id).await?;
    Ok(Json(WebhookConfig {
        github_url: format!("/api/v1/webhooks/github/{}", app.id),
        gitlab_url: format!("/api/v1/webhooks/gitlab/{}", app.id),
        secret: app.webhook_secret,
    }))
}

/// Live health straight from the cluster; nothing about it is cached in
/// Postgres, so it cannot go stale.
pub async fn health(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::k8s::AppStatus>> {
    let app = svc::get(&state.db, id, user.id).await?;
    let status = state
        .cluster
        .app_status(&crate::k8s::names::namespace(app.id), &app.id.to_string())
        .await?;
    Ok(Json(status))
}

#[derive(Debug, serde::Deserialize)]
pub struct GitCredentials {
    pub token: String,
}

/// Stores a deploy token for a private repository. Write-only: the token is
/// never returned, only the fact that one is set.
pub async fn set_git_credentials(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<GitCredentials>,
) -> Result<StatusCode> {
    git_credentials::set(&state, id, user.id, &body.token).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn clear_git_credentials(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    git_credentials::clear(&state, id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
