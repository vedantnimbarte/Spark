//! Application environment variables.
//!
//! Values are stored only in the application's Kubernetes Secret. Postgres
//! keeps the key names so the dashboard can list variables without a cluster
//! round-trip, and so a name is still known if the cluster is unreachable.

use crate::{
    error::{Error, Result},
    k8s::{manifests, names},
    repos::env_keys,
    services::applications,
    state::AppState,
};
use std::collections::BTreeMap;
use uuid::Uuid;

pub async fn list_keys(state: &AppState, app_id: Uuid, owner_id: Uuid) -> Result<Vec<String>> {
    applications::get(&state.db, app_id, owner_id).await?;
    env_keys::list(&state.db, app_id).await
}

/// Reveals one value, which is the only operation that reads the cluster.
pub async fn reveal(state: &AppState, app_id: Uuid, owner_id: Uuid, key: &str) -> Result<String> {
    applications::get(&state.db, app_id, owner_id).await?;
    let secret = state
        .cluster
        .get_secret(&names::namespace(app_id), manifests::ENV_SECRET)
        .await?;
    secret
        .get(key)
        .cloned()
        .ok_or(Error::NotFound("environment variable"))
}

/// Merges the supplied variables into the Secret and restarts the application
/// so the new values take effect.
pub async fn set(
    state: &AppState,
    app_id: Uuid,
    owner_id: Uuid,
    vars: BTreeMap<String, String>,
) -> Result<Vec<String>> {
    applications::get(&state.db, app_id, owner_id).await?;
    for key in vars.keys() {
        validate_key(key)?;
    }

    let namespace = names::namespace(app_id);
    let mut secret = state
        .cluster
        .get_secret(&namespace, manifests::ENV_SECRET)
        .await?;
    secret.extend(vars.clone());

    state
        .cluster
        .apply_secret(&namespace, manifests::ENV_SECRET, secret)
        .await?;

    for key in vars.keys() {
        env_keys::upsert(&state.db, app_id, key).await?;
    }

    state
        .cluster
        .restart_deployment(&namespace, manifests::APP_RESOURCE)
        .await?;

    env_keys::list(&state.db, app_id).await
}

pub async fn remove(state: &AppState, app_id: Uuid, owner_id: Uuid, key: &str) -> Result<()> {
    applications::get(&state.db, app_id, owner_id).await?;

    let namespace = names::namespace(app_id);
    let mut secret = state
        .cluster
        .get_secret(&namespace, manifests::ENV_SECRET)
        .await?;

    if secret.remove(key).is_none() {
        return Err(Error::NotFound("environment variable"));
    }

    state
        .cluster
        .apply_secret(&namespace, manifests::ENV_SECRET, secret)
        .await?;
    env_keys::delete(&state.db, app_id, key).await?;
    state
        .cluster
        .restart_deployment(&namespace, manifests::APP_RESOURCE)
        .await?;
    Ok(())
}

/// Kubernetes accepts only these characters in a Secret key, and the shell in
/// the container will not export anything else usefully.
fn validate_key(key: &str) -> Result<()> {
    let valid = !key.is_empty()
        && key.len() <= 253
        && !key.starts_with(|c: char| c.is_ascii_digit())
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-');

    if valid {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "{key:?} is not a usable variable name: use letters, digits, '_', '.' or '-', \
             and do not start with a digit"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_conventional_names_and_rejects_the_rest() {
        for good in ["DATABASE_URL", "PORT", "a", "my.key", "my-key", "X1"] {
            assert!(validate_key(good).is_ok(), "should accept {good:?}");
        }
        for bad in [
            "",
            "1LEADING",
            "has space",
            "has=equals",
            "dollar$",
            &"a".repeat(254),
        ] {
            assert!(validate_key(bad).is_err(), "should reject {bad:?}");
        }
    }
}
