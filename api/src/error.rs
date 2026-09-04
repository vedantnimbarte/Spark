use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Domain errors. Handlers return these; the `IntoResponse` impl is the single
/// place that decides status codes and what the client is allowed to see.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} not found")]
    NotFound(&'static str),

    #[error("{0}")]
    Invalid(String),

    #[error("authentication required")]
    Unauthorized,

    #[error("not permitted")]
    Forbidden,

    #[error("{0}")]
    Conflict(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Kube(#[from] kube::Error),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match &self {
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::Invalid(_) => StatusCode::BAD_REQUEST,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::Forbidden => StatusCode::FORBIDDEN,
            Error::Conflict(_) => StatusCode::CONFLICT,
            Error::Database(_) | Error::Kube(_) | Error::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        // Internal detail is logged, never returned: it routinely contains
        // connection strings and SQL.
        let body = if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = ?self, "request failed");
            "internal server error".to_string()
        } else {
            self.to_string()
        };

        (status, Json(json!({ "error": body }))).into_response()
    }
}
