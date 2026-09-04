//! Custom domains. Adding or removing one re-applies the Ingress; nothing has
//! to be rebuilt or redeployed.

use crate::{
    error::{Error, Result},
    k8s::{manifests, names, SslStatus},
    models::Domain,
    repos::domains,
    services::applications,
    state::AppState,
};
use uuid::Uuid;

pub async fn list(state: &AppState, app_id: Uuid, owner_id: Uuid) -> Result<Vec<Domain>> {
    applications::get(&state.db, app_id, owner_id).await?;

    // Read the certificate's real condition rather than trusting the stored
    // column, so a revoked or failed issuance cannot keep reporting "ready".
    // All hosts share one Certificate, so one lookup covers every row.
    if state.config.cluster_issuer.is_some() {
        let status = state
            .cluster
            .certificate_status(&names::namespace(app_id), manifests::TLS_SECRET)
            .await
            .unwrap_or(SslStatus::None);
        domains::set_ssl_status(&state.db, app_id, status.as_str()).await?;
    }

    domains::list_by_app(&state.db, app_id).await
}

pub async fn add(
    state: &AppState,
    app_id: Uuid,
    owner_id: Uuid,
    domain_name: &str,
) -> Result<Domain> {
    applications::get(&state.db, app_id, owner_id).await?;
    let domain_name = domain_name.trim().to_lowercase();
    validate_domain(&domain_name)?;

    let domain = domains::create(&state.db, app_id, &domain_name)
        .await
        .map_err(|e| match e {
            Error::Database(sqlx::Error::Database(ref db_err)) if db_err.is_unique_violation() => {
                Error::Conflict("that domain is already in use".into())
            }
            other => other,
        })?;

    sync_ingress(state, app_id, owner_id).await?;
    Ok(domain)
}

pub async fn remove(state: &AppState, app_id: Uuid, owner_id: Uuid, domain_id: Uuid) -> Result<()> {
    applications::get(&state.db, app_id, owner_id).await?;
    if !domains::delete(&state.db, domain_id, app_id).await? {
        return Err(Error::NotFound("domain"));
    }
    sync_ingress(state, app_id, owner_id).await
}

/// Rewrites the Ingress from the generated host plus whatever custom domains
/// are currently recorded.
async fn sync_ingress(state: &AppState, app_id: Uuid, owner_id: Uuid) -> Result<()> {
    let app = applications::get(&state.db, app_id, owner_id).await?;
    let namespace = names::namespace(app_id);

    let mut hosts = vec![names::default_host(
        &app.name,
        &state.config.app_base_domain,
    )];
    hosts.extend(
        domains::list_by_app(&state.db, app_id)
            .await?
            .into_iter()
            .map(|d| d.domain_name),
    );

    state
        .cluster
        .apply(
            &namespace,
            &manifests::app_ingress(
                app_id,
                &namespace,
                &hosts,
                state.config.cluster_issuer.as_deref(),
            ),
        )
        .await
}

/// A hostname, not a URL: no scheme, no path, no port.
fn validate_domain(domain: &str) -> Result<()> {
    let valid = !domain.is_empty()
        && domain.len() <= 253
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains("..")
        && domain.split('.').count() >= 2
        && domain
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
        });

    if valid {
        Ok(())
    } else {
        Err(Error::Invalid(
            "enter a hostname such as app.example.com, without a scheme or path".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_hostnames_and_rejects_urls_and_malformed_names() {
        for good in ["example.com", "app.example.com", "a-b.example.co.uk"] {
            assert!(validate_domain(good).is_ok(), "should accept {good:?}");
        }
        for bad in [
            "",
            "localhost",           // needs a dot
            "https://example.com", // scheme
            "example.com/path",    // path
            "example.com:8080",    // port
            "-lead.example.com",
            "trail-.example.com",
            "double..dot.com",
            ".leading.dot.com",
            "UPPER.example.com",
        ] {
            assert!(validate_domain(bad).is_err(), "should reject {bad:?}");
        }
    }
}
