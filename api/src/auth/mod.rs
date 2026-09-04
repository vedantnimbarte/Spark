pub mod session;

use crate::error::{Error, Result};
use anyhow::anyhow;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

/// Argon2id with the crate's default parameters, which track the OWASP
/// recommendation.
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| Error::Internal(anyhow!("could not hash password: {e}")))
}

/// Returns false rather than an error for a wrong password; a malformed stored
/// hash is the only genuine failure.
pub fn verify_password(password: &str, stored_hash: &str) -> Result<bool> {
    let parsed = PasswordHash::new(stored_hash)
        .map_err(|e| Error::Internal(anyhow!("stored password hash is malformed: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}
