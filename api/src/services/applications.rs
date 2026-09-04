use crate::{
    auth::session::generate_token,
    error::{Error, Result},
    k8s::names,
    models::Application,
    repos::applications::{self, ApplicationUpdate, NewApplication},
    state::AppState,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateApplication {
    pub name: String,
    pub git_repo: String,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub dockerfile_path: Option<String>,
    #[serde(default)]
    pub container_port: Option<i32>,
    #[serde(default)]
    pub cpu_limit: Option<String>,
    #[serde(default)]
    pub memory_limit: Option<String>,
}

pub async fn create(
    state: &AppState,
    owner_id: Uuid,
    input: CreateApplication,
) -> Result<Application> {
    let db = &state.db;
    let name = input.name.trim().to_lowercase();
    validate_name(&name)?;
    validate_git_repo(&input.git_repo)?;
    let port = input.container_port.unwrap_or(8080);
    validate_port(port)?;

    let app = applications::create(
        db,
        NewApplication {
            owner_id,
            name,
            git_repo: input.git_repo.trim().to_string(),
            git_branch: input.git_branch.unwrap_or_else(|| "main".into()),
            // Dockerfile is the only builder in v1; Nixpacks slots in here.
            build_type: "dockerfile".into(),
            dockerfile_path: input.dockerfile_path.unwrap_or_else(|| "Dockerfile".into()),
            container_port: port,
            cpu_limit: input.cpu_limit.unwrap_or_else(|| "500m".into()),
            memory_limit: input.memory_limit.unwrap_or_else(|| "512Mi".into()),
            webhook_secret: generate_token(),
        },
    )
    .await
    .map_err(|e| match e {
        // The (owner_id, name) unique index is the authority on collisions;
        // checking first would still race.
        Error::Database(sqlx::Error::Database(ref db_err)) if db_err.is_unique_violation() => {
            Error::Conflict("an application with that name already exists".into())
        }
        other => other,
    })?;

    // The namespace has to exist before the application does anything else,
    // because environment variables are stored only as a Kubernetes Secret and
    // may be set before the first deploy. If the cluster refuses, the row is
    // rolled back rather than left describing an application with nowhere to
    // run.
    if let Err(error) = state
        .cluster
        .ensure_namespace(&names::namespace(app.id))
        .await
    {
        tracing::error!(app_id = %app.id, %error, "namespace creation failed; rolling back");
        applications::delete(db, app.id, owner_id).await?;
        return Err(error);
    }

    Ok(app)
}

pub async fn list(db: &PgPool, owner_id: Uuid) -> Result<Vec<Application>> {
    applications::list_by_owner(db, owner_id).await
}

pub async fn get(db: &PgPool, id: Uuid, owner_id: Uuid) -> Result<Application> {
    applications::find_owned(db, id, owner_id)
        .await?
        .ok_or(Error::NotFound("application"))
}

#[derive(Debug, Deserialize)]
pub struct UpdateApplication {
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub dockerfile_path: Option<String>,
    #[serde(default)]
    pub container_port: Option<i32>,
    #[serde(default)]
    pub cpu_limit: Option<String>,
    #[serde(default)]
    pub memory_limit: Option<String>,
}

pub async fn update(
    db: &PgPool,
    id: Uuid,
    owner_id: Uuid,
    patch: UpdateApplication,
) -> Result<Application> {
    if let Some(port) = patch.container_port {
        validate_port(port)?;
    }
    applications::update(
        db,
        id,
        owner_id,
        ApplicationUpdate {
            git_branch: patch.git_branch,
            dockerfile_path: patch.dockerfile_path,
            container_port: patch.container_port,
            cpu_limit: patch.cpu_limit,
            memory_limit: patch.memory_limit,
        },
    )
    .await?
    .ok_or(Error::NotFound("application"))
}

pub async fn delete(state: &AppState, id: Uuid, owner_id: Uuid) -> Result<()> {
    if !applications::delete(&state.db, id, owner_id).await? {
        return Err(Error::NotFound("application"));
    }
    // Removing the namespace removes every object the application owns.
    state.cluster.delete_namespace(&names::namespace(id)).await
}

/// The name becomes both a DNS label in the ingress host and part of a
/// Kubernetes namespace, so it is checked here rather than left to a database
/// constraint violation surfacing as a 500.
fn validate_name(name: &str) -> Result<()> {
    let valid_shape = !name.is_empty()
        && name.len() <= 40
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-');

    if valid_shape {
        Ok(())
    } else {
        Err(Error::Invalid(
            "name must be 1-40 characters of lowercase letters, digits or hyphens, \
             and may not start or end with a hyphen"
                .into(),
        ))
    }
}

/// Rejects anything that is not an http(s) or scp-style git URL. This string is
/// handed to git inside the build job, so a `--upload-pack=...` style argument
/// must not reach it.
fn validate_git_repo(repo: &str) -> Result<()> {
    let repo = repo.trim();
    let plausible =
        (repo.starts_with("https://") || repo.starts_with("http://") || repo.starts_with("git@"))
            && !repo.contains(char::is_whitespace)
            && repo.len() <= 512;

    if plausible {
        Ok(())
    } else {
        Err(Error::Invalid(
            "git_repo must be an http(s) or git@ URL".into(),
        ))
    }
}

fn validate_port(port: i32) -> Result<()> {
    if (1..=65535).contains(&port) {
        Ok(())
    } else {
        Err(Error::Invalid("container_port must be 1-65535".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_validation_matches_dns_label_rules() {
        for good in ["a", "my-app", "app123", &"a".repeat(40)] {
            assert!(validate_name(good).is_ok(), "should accept {good:?}");
        }
        for bad in [
            "",
            "-lead",
            "trail-",
            "Upper",
            "has space",
            "under_score",
            &"a".repeat(41),
        ] {
            assert!(validate_name(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn git_repo_validation_rejects_argument_injection() {
        assert!(validate_git_repo("https://github.com/o/r.git").is_ok());
        assert!(validate_git_repo("git@github.com:o/r.git").is_ok());
        for bad in [
            "--upload-pack=touch /tmp/x",
            "file:///etc/passwd",
            "ext::sh -c id",
            "",
        ] {
            assert!(validate_git_repo(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn port_validation_covers_boundaries() {
        assert!(validate_port(1).is_ok());
        assert!(validate_port(65535).is_ok());
        assert!(validate_port(0).is_err());
        assert!(validate_port(65536).is_err());
        assert!(validate_port(-1).is_err());
    }
}
