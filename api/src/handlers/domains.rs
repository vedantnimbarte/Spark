use crate::{
    auth::session::CurrentUser, error::Result, models::Domain, services::domains as svc,
    state::SharedState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

pub async fn list(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(app_id): Path<Uuid>,
) -> Result<Json<Vec<Domain>>> {
    Ok(Json(svc::list(&state, app_id, user.id).await?))
}

#[derive(Debug, Deserialize)]
pub struct AddDomain {
    pub domain_name: String,
}

pub async fn add(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(app_id): Path<Uuid>,
    Json(body): Json<AddDomain>,
) -> Result<(StatusCode, Json<Domain>)> {
    let domain = svc::add(&state, app_id, user.id, &body.domain_name).await?;
    Ok((StatusCode::CREATED, Json(domain)))
}

pub async fn remove(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path((app_id, domain_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    svc::remove(&state, app_id, user.id, domain_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
