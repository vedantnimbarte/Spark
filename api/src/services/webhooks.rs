//! Git provider webhooks.
//!
//! The endpoint carries the application id because a bare `/webhooks/github`
//! gives no way to know which application a push is for or which secret to
//! verify it against. Each application therefore has its own
//! webhook URL and secret.

use crate::{
    error::{Error, Result},
    models::Application,
    repos::applications,
    services::deployments,
    state::AppState,
};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct PushEvent {
    /// e.g. `refs/heads/main`. Both GitHub and GitLab send this.
    #[serde(default)]
    pub r#ref: Option<String>,
    /// Commit the branch now points at.
    #[serde(default)]
    pub after: Option<String>,
    #[serde(default)]
    pub checkout_sha: Option<String>,
}

impl PushEvent {
    fn branch(&self) -> Option<&str> {
        self.r#ref.as_deref()?.strip_prefix("refs/heads/")
    }

    fn sha(&self) -> Option<&str> {
        // GitLab populates checkout_sha; GitHub populates after.
        self.after
            .as_deref()
            .or(self.checkout_sha.as_deref())
            .filter(|sha| !sha.chars().all(|c| c == '0'))
    }
}

/// What the endpoint decided, so the response can say why nothing happened.
pub enum Outcome {
    Deployed(Uuid),
    Ignored(&'static str),
}

/// Verifies a GitHub `X-Hub-Signature-256` header.
///
/// The comparison is done by the MAC implementation itself, which is
/// constant-time; comparing hex strings with `==` would leak the signature a
/// byte at a time.
pub fn verify_github(secret: &str, signature_header: &str, body: &[u8]) -> Result<()> {
    let hex = signature_header
        .strip_prefix("sha256=")
        .ok_or_else(|| Error::Forbidden)?;
    let expected = hex::decode(hex).map_err(|_| Error::Forbidden)?;

    let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes())
        .map_err(|e| Error::Internal(anyhow::anyhow!("invalid webhook secret: {e}")))?;
    mac.update(body);
    mac.verify_slice(&expected).map_err(|_| Error::Forbidden)
}

/// GitLab sends the secret verbatim, so compare it without short-circuiting.
pub fn verify_gitlab(secret: &str, token: &str) -> Result<()> {
    let equal = secret.len() == token.len()
        && secret
            .bytes()
            .zip(token.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0;

    if equal {
        Ok(())
    } else {
        Err(Error::Forbidden)
    }
}

pub async fn handle(state: &AppState, app: &Application, event: &PushEvent) -> Result<Outcome> {
    let Some(branch) = event.branch() else {
        // A ping or a tag push, not something to deploy.
        return Ok(Outcome::Ignored("not a branch push"));
    };
    if branch != app.git_branch {
        return Ok(Outcome::Ignored("push was to a different branch"));
    }
    let Some(sha) = event.sha() else {
        return Ok(Outcome::Ignored("branch was deleted"));
    };

    let deployment = deployments::enqueue(state, app, sha).await?;
    Ok(Outcome::Deployed(deployment.id))
}

/// Loads the application a webhook names. Ownership is established by the
/// signature, so there is no session here.
pub async fn find_app(state: &AppState, app_id: Uuid) -> Result<Application> {
    applications::find_by_id(&state.db, app_id)
        .await?
        .ok_or(Error::NotFound("application"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "a-webhook-secret";
    const BODY: &[u8] = br#"{"ref":"refs/heads/main","after":"abc"}"#;

    /// Signature produced by the same construction GitHub uses.
    fn sign(secret: &str, body: &[u8]) -> String {
        let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes()).expect("key");
        mac.update(body);
        format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
    }

    #[test]
    fn accepts_a_correct_signature() {
        assert!(verify_github(SECRET, &sign(SECRET, BODY), BODY).is_ok());
    }

    #[test]
    fn rejects_a_tampered_body() {
        let signature = sign(SECRET, BODY);
        let tampered = br#"{"ref":"refs/heads/main","after":"evil"}"#;
        assert!(verify_github(SECRET, &signature, tampered).is_err());
    }

    #[test]
    fn rejects_the_wrong_secret_and_malformed_headers() {
        assert!(verify_github("other-secret", &sign(SECRET, BODY), BODY).is_err());
        assert!(verify_github(SECRET, "not-a-signature", BODY).is_err());
        assert!(verify_github(SECRET, "sha256=zzzz", BODY).is_err());
        assert!(verify_github(SECRET, "", BODY).is_err());
    }

    #[test]
    fn gitlab_token_must_match_exactly() {
        assert!(verify_gitlab(SECRET, SECRET).is_ok());
        assert!(verify_gitlab(SECRET, "a-webhook-secre").is_err());
        assert!(verify_gitlab(SECRET, "a-webhook-secretX").is_err());
        assert!(verify_gitlab(SECRET, "").is_err());
    }

    fn event(json: &str) -> PushEvent {
        serde_json::from_str(json).expect("valid event")
    }

    #[test]
    fn extracts_branch_and_sha_from_both_providers() {
        let github = event(r#"{"ref":"refs/heads/main","after":"1234"}"#);
        assert_eq!(github.branch(), Some("main"));
        assert_eq!(github.sha(), Some("1234"));

        let gitlab = event(r#"{"ref":"refs/heads/dev","checkout_sha":"5678"}"#);
        assert_eq!(gitlab.branch(), Some("dev"));
        assert_eq!(gitlab.sha(), Some("5678"));
    }

    #[test]
    fn ignores_tags_pings_and_branch_deletions() {
        assert_eq!(event(r#"{"ref":"refs/tags/v1"}"#).branch(), None);
        assert_eq!(event(r#"{"zen":"hi"}"#).branch(), None);
        // An all-zero `after` is how a branch deletion is reported.
        let deleted = event(
            r#"{"ref":"refs/heads/main","after":"0000000000000000000000000000000000000000"}"#,
        );
        assert_eq!(deleted.branch(), Some("main"));
        assert_eq!(deleted.sha(), None, "a deletion must not trigger a build");
    }
}
