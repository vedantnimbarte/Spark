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
    if !state.login_limiter.check(&body.email) {
        return Err(Error::Invalid(
            "too many sign-in attempts; wait a few minutes and try again".into(),
        ));
    }

    let user = svc::authenticate(&state.db, &body.email, &body.password).await?;
    state.login_limiter.reset(&body.email);
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

#[derive(Debug, Deserialize)]
pub struct PasswordChange {
    pub current_password: String,
    pub new_password: String,
}

#[derive(serde::Serialize)]
pub struct PasswordChanged {
    /// How many other sessions the change signed out, so the dashboard can say
    /// what it did rather than leaving it implied.
    pub sessions_revoked: u64,
}

pub async fn change_password(
    State(state): State<SharedState>,
    jar: CookieJar,
    CurrentUser(user): CurrentUser,
    Json(body): Json<PasswordChange>,
) -> Result<Json<PasswordChanged>> {
    // The extractor already proved this cookie is valid; it is read again here
    // because the change needs to know which session to spare.
    let token = jar.get(session::COOKIE_NAME).ok_or(Error::Unauthorized)?;
    let sessions_revoked = svc::change_password(
        &state.db,
        &user,
        &body.current_password,
        &body.new_password,
        token.value(),
    )
    .await?;
    Ok(Json(PasswordChanged { sessions_revoked }))
}

pub async fn list_sessions(
    State(state): State<SharedState>,
    jar: CookieJar,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<crate::repos::sessions::SessionInfo>>> {
    let token = jar.get(session::COOKIE_NAME).ok_or(Error::Unauthorized)?;
    let sessions = crate::repos::sessions::list_for_user(
        &state.db,
        user.id,
        &session::hash_token(token.value()),
    )
    .await?;
    Ok(Json(sessions))
}

pub async fn revoke_session(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> Result<StatusCode> {
    // A revoke that matched nothing is reported as such: silently returning
    // success would tell the owner a session is gone when it is not.
    if crate::repos::sessions::delete_owned(&state.db, id, user.id).await? == 0 {
        return Err(Error::NotFound("session"));
    }
    Ok(StatusCode::NO_CONTENT)
}
