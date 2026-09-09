//! Unified application error type.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

/// Application-wide error type.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Database layer error.
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    /// Resource not found.
    #[error("not found")]
    NotFound,
    /// Authentication missing or invalid.
    #[error("unauthorized")]
    Unauthorized,
    /// Client sent invalid data.
    #[error("bad request: {0}")]
    BadRequest(String),
    /// Login locked out after too many failed attempts; carries the
    /// remaining lockout seconds.
    #[error("too_many_attempts")]
    TooManyAttempts(u64),
    /// Unexpected internal failure.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Application-wide result alias.
pub type Result<T> = std::result::Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::TooManyAttempts(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Db(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = match &self {
            Self::TooManyAttempts(secs) => {
                serde_json::json!({ "error": self.to_string(), "retry_after": secs })
            }
            _ => serde_json::json!({ "error": self.to_string() }),
        };
        (status, Json(body)).into_response()
    }
}
