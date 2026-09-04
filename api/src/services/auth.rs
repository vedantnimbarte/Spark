use crate::{
    auth::{hash_password, session, verify_password},
    error::{Error, Result},
    models::User,
    repos::{sessions, users},
};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::sync::LazyLock;

/// The first account is created without authentication so the instance can be
/// bootstrapped; after that, only an existing user may add accounts. Without
/// this, a control plane exposed to the internet is open to anyone.
pub async fn signup_is_open(db: &PgPool) -> Result<bool> {
    Ok(users::count(db).await? == 0)
}

pub async fn register(db: &PgPool, email: &str, password: &str) -> Result<User> {
    let email = email.trim().to_lowercase();
    validate_credentials(&email, password)?;

    if users::find_by_email(db, &email).await?.is_some() {
        return Err(Error::Conflict("that email is already registered".into()));
    }

    users::create(db, &email, &hash_password(password)?).await
}

pub async fn authenticate(db: &PgPool, email: &str, password: &str) -> Result<User> {
    let email = email.trim().to_lowercase();
    let user = users::find_by_email(db, &email).await?;

    // Same error and roughly the same work either way, so the response does not
    // reveal whether an email is registered.
    match user {
        Some(user) if verify_password(password, &user.password_hash)? => Ok(user),
        Some(_) => Err(Error::Unauthorized),
        None => {
            let _ = verify_password(password, &DUMMY_HASH);
            Err(Error::Unauthorized)
        }
    }
}

/// Issues a session and returns the raw token; only its digest is persisted.
pub async fn start_session(db: &PgPool, user: &User) -> Result<String> {
    let token = session::generate_token();
    let expires_at = Utc::now() + Duration::days(session::SESSION_DAYS);
    sessions::create(db, &session::hash_token(&token), user.id, expires_at).await?;
    Ok(token)
}

pub async fn end_session(db: &PgPool, token: &str) -> Result<()> {
    sessions::delete(db, &session::hash_token(token)).await
}

/// Changes the password and signs out everywhere else.
///
/// Verifying the current password first is what stops a borrowed browser tab
/// from becoming a permanent takeover; dropping the other sessions is what
/// makes the change actually evict whoever was already in.
pub async fn change_password(
    db: &PgPool,
    user: &User,
    current: &str,
    new: &str,
    keep_token: &str,
) -> Result<u64> {
    if !verify_password(current, &user.password_hash)? {
        return Err(Error::Invalid("current password is incorrect".into()));
    }
    validate_password(new)?;
    if current == new {
        return Err(Error::Invalid(
            "the new password must differ from the current one".into(),
        ));
    }

    users::update_password(db, user.id, &hash_password(new)?).await?;
    sessions::delete_others(db, user.id, &session::hash_token(keep_token)).await
}

fn validate_credentials(email: &str, password: &str) -> Result<()> {
    // Deliberately loose: the only email check that is not wrong in some locale
    // is whether it could plausibly be routed.
    if !email.contains('@') || email.len() < 3 || email.len() > 254 {
        return Err(Error::Invalid("a valid email address is required".into()));
    }
    validate_password(password)
}

fn validate_password(password: &str) -> Result<()> {
    if password.chars().count() < 12 {
        return Err(Error::Invalid(
            "password must be at least 12 characters".into(),
        ));
    }
    if password.len() > 1024 {
        return Err(Error::Invalid("password is too long".into()));
    }
    Ok(())
}

/// Hashed once at first use so it is guaranteed to be a well-formed argon2
/// string. Verifying against it makes a login for an unknown email cost about
/// the same as one for a known email, which keeps the response time from
/// disclosing whether an account exists.
static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| hash_password("timing equalisation placeholder").unwrap_or_default());
