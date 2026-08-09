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
            Self::Db(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (
            status,
            Json(serde_json::json!({ "error": self.to_string() })),
        )
            .into_response()
    }
}
