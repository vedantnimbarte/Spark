use crate::{
    error::{Error, Result},
    models::User,
    repos::sessions,
    state::SharedState,
};
use argon2::password_hash::rand_core::{OsRng, RngCore};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use sha2::{Digest, Sha256};

pub const COOKIE_NAME: &str = "spark_session";
pub const SESSION_DAYS: i64 = 30;

/// 256 bits of entropy, hex encoded.
pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// The database stores only this digest. SHA-256 is right here where argon2
/// would not be: the input is already high-entropy random, so there is nothing
/// to brute force and no reason to pay a KDF's cost on every request.
pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub fn cookie(token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((COOKIE_NAME, token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        // Driven by config rather than hard-coded: a browser silently drops a
        // Secure cookie sent over plain HTTP, which is exactly what host
        // development uses.
        .secure(secure)
        .max_age(time::Duration::days(SESSION_DAYS))
        .build()
}

pub fn removal_cookie() -> Cookie<'static> {
    Cookie::build((COOKIE_NAME, ""))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::seconds(0))
        .build()
}

/// Extractor that turns a session cookie into the authenticated user. Putting
/// it in the handler signature is what makes a route protected.
pub struct CurrentUser(pub User);

impl FromRequestParts<SharedState> for CurrentUser {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &SharedState) -> Result<Self> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar.get(COOKIE_NAME).ok_or(Error::Unauthorized)?;
        let user = sessions::find_valid_user(&state.db, &hash_token(token.value()))
            .await?
            .ok_or(Error::Unauthorized)?;
        Ok(CurrentUser(user))
    }
}
