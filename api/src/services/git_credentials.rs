//! Git credentials for private repositories.
//!
//! Stored in a Secret of their own, not in `app-env`: that Secret is injected
//! into the running container, and a deploy token has no business being
//! readable by the application it deploys.
//!
//! HTTPS tokens only. SSH keys would need an agent in the build pod and a
//! known_hosts policy; a personal access token covers GitHub and GitLab, which
//! is what the webhook side already supports.

use crate::{
    error::{Error, Result},
    k8s::names,
    models::Application,
    repos::applications,
    services::applications as app_svc,
    state::AppState,
};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Secret holding the deploy token, separate from the application's env.
pub const GIT_SECRET: &str = "app-git";
pub const TOKEN_KEY: &str = "token";

pub async fn set(state: &AppState, app_id: Uuid, owner_id: Uuid, token: &str) -> Result<()> {
    app_svc::get(&state.db, app_id, owner_id).await?;

    let token = token.trim();
    if token.is_empty() {
        return Err(Error::Invalid("token must not be empty".into()));
    }
    // Tokens are opaque, but a pasted URL or a whole `Authorization:` header is
    // a common mistake worth catching before a build fails.
    if token.contains(char::is_whitespace) {
        return Err(Error::Invalid(
            "token must not contain spaces; paste only the token itself".into(),
        ));
    }

    state
        .cluster
        .apply_secret(
            &names::namespace(app_id),
            GIT_SECRET,
            BTreeMap::from([(TOKEN_KEY.to_string(), token.to_string())]),
        )
        .await?;

    applications::set_git_credentials_flag(&state.db, app_id, true).await
}

pub async fn clear(state: &AppState, app_id: Uuid, owner_id: Uuid) -> Result<()> {
    app_svc::get(&state.db, app_id, owner_id).await?;

    state
        .cluster
        .delete_secret(&names::namespace(app_id), GIT_SECRET)
        .await?;

    applications::set_git_credentials_flag(&state.db, app_id, false).await
}

/// Reads the token back for use by the control plane. Returns `None` when the
/// repository is public, which keeps the caller free of special cases.
pub async fn token_for(state: &AppState, app: &Application) -> Result<Option<String>> {
    if !app.git_credentials_set {
        return Ok(None);
    }

    let secret = state
        .cluster
        .get_secret(&names::namespace(app.id), GIT_SECRET)
        .await?;

    Ok(secret.get(TOKEN_KEY).filter(|t| !t.is_empty()).cloned())
}
