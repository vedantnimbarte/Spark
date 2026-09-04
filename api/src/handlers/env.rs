use crate::{auth::session::CurrentUser, error::Result, services::env as svc, state::SharedState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Serialize)]
pub struct EnvKeys {
    /// Names only. Values are write-only by design and must be revealed one at
    /// a time.
    pub keys: Vec<String>,
}

pub async fn list(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(app_id): Path<Uuid>,
) -> Result<Json<EnvKeys>> {
    let keys = svc::list_keys(&state, app_id, user.id).await?;
    Ok(Json(EnvKeys { keys }))
}

pub async fn set(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(app_id): Path<Uuid>,
    Json(vars): Json<BTreeMap<String, String>>,
) -> Result<Json<EnvKeys>> {
    let keys = svc::set(&state, app_id, user.id, vars).await?;
    Ok(Json(EnvKeys { keys }))
}

#[derive(Serialize)]
pub struct EnvValue {
    pub key: String,
    pub value: String,
}

pub async fn reveal(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path((app_id, key)): Path<(Uuid, String)>,
) -> Result<Json<EnvValue>> {
    let value = svc::reveal(&state, app_id, user.id, &key).await?;
    Ok(Json(EnvValue { key, value }))
}

pub async fn remove(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path((app_id, key)): Path<(Uuid, String)>,
) -> Result<StatusCode> {
    svc::remove(&state, app_id, user.id, &key).await?;
    Ok(StatusCode::NO_CONTENT)
}
