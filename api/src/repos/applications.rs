use crate::{error::Result, models::Application};
use sqlx::PgPool;
use uuid::Uuid;

pub struct NewApplication {
    pub owner_id: Uuid,
    pub name: String,
    pub git_repo: String,
    pub git_branch: String,
    pub build_type: String,
    pub dockerfile_path: String,
    pub container_port: i32,
    pub cpu_limit: String,
    pub memory_limit: String,
    pub webhook_secret: String,
}

pub async fn create(db: &PgPool, new: NewApplication) -> Result<Application> {
    let app = sqlx::query_as!(
        Application,
        r#"INSERT INTO applications
             (owner_id, name, git_repo, git_branch, build_type, dockerfile_path,
              container_port, cpu_limit, memory_limit, webhook_secret)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
           RETURNING id, owner_id, name, git_repo, git_branch, build_type, dockerfile_path,
                     container_port, cpu_limit, memory_limit, replicas, git_credentials_set,
                     webhook_secret, created_at"#,
        new.owner_id,
        new.name,
        new.git_repo,
        new.git_branch,
        new.build_type,
        new.dockerfile_path,
        new.container_port,
        new.cpu_limit,
        new.memory_limit,
        new.webhook_secret,
    )
    .fetch_one(db)
    .await?;
    Ok(app)
}

pub async fn list_by_owner(db: &PgPool, owner_id: Uuid) -> Result<Vec<Application>> {
    let apps = sqlx::query_as!(
        Application,
        r#"SELECT id, owner_id, name, git_repo, git_branch, build_type, dockerfile_path,
                  container_port, cpu_limit, memory_limit, replicas, git_credentials_set,
                  webhook_secret, created_at
           FROM applications WHERE owner_id = $1 ORDER BY created_at DESC"#,
        owner_id,
    )
    .fetch_all(db)
    .await?;
    Ok(apps)
}

/// Ownership is part of the lookup, not a separate check, so there is no path
/// that loads another user's application at all.
pub async fn find_owned(db: &PgPool, id: Uuid, owner_id: Uuid) -> Result<Option<Application>> {
    let app = sqlx::query_as!(
        Application,
        r#"SELECT id, owner_id, name, git_repo, git_branch, build_type, dockerfile_path,
                  container_port, cpu_limit, memory_limit, replicas, git_credentials_set,
                  webhook_secret, created_at
           FROM applications WHERE id = $1 AND owner_id = $2"#,
        id,
        owner_id,
    )
    .fetch_optional(db)
    .await?;
    Ok(app)
}

/// Only the fields a user may change after creation. `NULL` leaves a column as
/// it is, which keeps this to one statement instead of a query builder.
pub struct ApplicationUpdate {
    pub git_branch: Option<String>,
    pub dockerfile_path: Option<String>,
    pub container_port: Option<i32>,
    pub cpu_limit: Option<String>,
    pub memory_limit: Option<String>,
    pub replicas: Option<i32>,
}

pub async fn update(
    db: &PgPool,
    id: Uuid,
    owner_id: Uuid,
    patch: ApplicationUpdate,
) -> Result<Option<Application>> {
    let app = sqlx::query_as!(
        Application,
        r#"UPDATE applications SET
             git_branch      = COALESCE($3, git_branch),
             dockerfile_path = COALESCE($4, dockerfile_path),
             container_port  = COALESCE($5, container_port),
             cpu_limit       = COALESCE($6, cpu_limit),
             memory_limit    = COALESCE($7, memory_limit),
             replicas        = COALESCE($8, replicas)
           WHERE id = $1 AND owner_id = $2
           RETURNING id, owner_id, name, git_repo, git_branch, build_type, dockerfile_path,
                     container_port, cpu_limit, memory_limit, replicas, git_credentials_set,
                     webhook_secret, created_at"#,
        id,
        owner_id,
        patch.git_branch,
        patch.dockerfile_path,
        patch.container_port,
        patch.cpu_limit,
        patch.memory_limit,
        patch.replicas,
    )
    .fetch_optional(db)
    .await?;
    Ok(app)
}

pub async fn delete(db: &PgPool, id: Uuid, owner_id: Uuid) -> Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM applications WHERE id = $1 AND owner_id = $2",
        id,
        owner_id,
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Unscoped lookup for background work, which acts on behalf of the queue
/// rather than a request and has already established ownership upstream.
pub async fn find_by_id(db: &PgPool, id: Uuid) -> Result<Option<Application>> {
    let app = sqlx::query_as!(
        Application,
        r#"SELECT id, owner_id, name, git_repo, git_branch, build_type, dockerfile_path,
                  container_port, cpu_limit, memory_limit, replicas, git_credentials_set,
                  webhook_secret, created_at
           FROM applications WHERE id = $1"#,
        id,
    )
    .fetch_optional(db)
    .await?;
    Ok(app)
}

/// Mirrors whether a git credential exists in the application's Secret. The
/// credential itself stays in Kubernetes.
pub async fn set_git_credentials_flag(db: &PgPool, id: Uuid, present: bool) -> Result<()> {
    sqlx::query!(
        "UPDATE applications SET git_credentials_set = $2 WHERE id = $1",
        id,
        present,
    )
    .execute(db)
    .await?;
    Ok(())
}
