use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub git_repo: String,
    pub git_branch: String,
    pub build_type: String,
    pub dockerfile_path: String,
    pub container_port: i32,
    pub cpu_limit: String,
    pub memory_limit: String,
    pub replicas: i32,
    /// Whether a git credential exists in the application's Kubernetes Secret.
    pub git_credentials_set: bool,
    /// Never serialised: it is the shared secret a Git provider signs with.
    #[serde(skip)]
    pub webhook_secret: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Deployment {
    pub id: Uuid,
    pub app_id: Uuid,
    pub commit_sha: String,
    pub status: String,
    pub image_ref: Option<String>,
    /// Set when this deployment reused an earlier deployment's image.
    pub rolled_back_from: Option<Uuid>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Deployment {
    /// Terminal states are the ones a log stream should stop following.
    pub fn is_finished(&self) -> bool {
        matches!(self.status.as_str(), "deployed" | "failed")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Domain {
    pub id: Uuid,
    pub app_id: Uuid,
    pub domain_name: String,
    pub ssl_status: String,
    pub created_at: DateTime<Utc>,
}
