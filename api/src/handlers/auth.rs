use crate::{
    auth::session::{self, CurrentUser},
    error::{Error, Result},
    models::User,
    services::auth as svc,
    state::SharedState,
};
use axum::{extract::State, http::StatusCode, Json};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

/// Open only while the instance has no users, so the first account can be
/// created; afterwards it requires an existing session.
pub async fn signup(
    State(state): State<SharedState>,
    jar: CookieJar,
    Json(body): Json<Credentials>,
) -> Result<(StatusCode, CookieJar, Json<User>)> {
    if !svc::signup_is_open(&state.db).await? {
        let token = jar.get(session::COOKIE_NAME).ok_or(Error::Forbidden)?;
        crate::repos::sessions::find_valid_user(&state.db, &session::hash_token(token.value()))
            .await?
            .ok_or(Error::Forbidden)?;
    }

    let user = svc::register(&state.db, &body.email, &body.password).await?;
    let token = svc::start_session(&state.db, &user).await?;
    Ok((
        StatusCode::CREATED,
        jar.add(session::cookie(token, state.config.cookie_secure)),
        Json(user),
    ))
}

pub async fn login(
    State(state): State<SharedState>,
    jar: CookieJar,
    Json(body): Json<Credentials>,
) -> Result<(CookieJar, Json<User>)> {
    let user = svc::authenticate(&state.db, &body.email, &body.password).await?;
    let token = svc::start_session(&state.db, &user).await?;
    Ok((
        jar.add(session::cookie(token, state.config.cookie_secure)),
        Json(user),
    ))
}

pub async fn logout(State(state): State<SharedState>, jar: CookieJar) -> Result<CookieJar> {
    if let Some(token) = jar.get(session::COOKIE_NAME) {
        svc::end_session(&state.db, token.value()).await?;
    }
    Ok(jar.add(session::removal_cookie()))
}

pub async fn me(CurrentUser(user): CurrentUser) -> Json<User> {
    Json(user)
}
