use crate::{
    error::{Error, Result},
    services::webhooks::{self as svc, Outcome, PushEvent},
    state::SharedState,
};
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

/// Parses the body only after the signature is verified, so an unauthenticated
/// caller cannot reach the deserialiser.
fn parse(body: &Bytes) -> Result<PushEvent> {
    serde_json::from_slice(body).map_err(|e| Error::Invalid(format!("unreadable payload: {e}")))
}

fn response(outcome: Outcome) -> Json<Value> {
    match outcome {
        Outcome::Deployed(id) => Json(json!({ "status": "deploying", "deployment_id": id })),
        Outcome::Ignored(reason) => Json(json!({ "status": "ignored", "reason": reason })),
    }
}

pub async fn github(
    State(state): State<SharedState>,
    Path(app_id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>> {
    let app = svc::find_app(&state, app_id).await?;

    let signature = headers
        .get("X-Hub-Signature-256")
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::Forbidden)?;
    svc::verify_github(&app.webhook_secret, signature, &body)?;

    let outcome = svc::handle(&state, &app, &parse(&body)?).await?;
    Ok(response(outcome))
}

pub async fn gitlab(
    State(state): State<SharedState>,
    Path(app_id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>> {
    let app = svc::find_app(&state, app_id).await?;

    let token = headers
        .get("X-Gitlab-Token")
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::Forbidden)?;
    svc::verify_gitlab(&app.webhook_secret, token)?;

    let outcome = svc::handle(&state, &app, &parse(&body)?).await?;
    Ok(response(outcome))
}
