//! Runtime configuration from environment variables.

use crate::error::{AppError, Result};

/// Runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address the HTTP server binds to, e.g. `127.0.0.1:8080`.
    pub bind_addr: String,
    /// `SQLite` connection URL, e.g. `sqlite:zblog.db`.
    pub database_url: String,
    /// Argon2 PHC hash of the admin password.
    pub password_hash: String,
    /// Secret used to sign session cookies.
    pub session_secret: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    /// Returns `AppError::Internal` when a required variable is missing.
    pub fn from_env() -> Result<Self> {
        let required = |key: &str| -> Result<String> {
            std::env::var(key).map_err(|_| AppError::Internal(format!("missing env var {key}")))
        };
        Ok(Self {
            bind_addr: std::env::var("ZBLOG_BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:8080".to_owned()),
            database_url: std::env::var("ZBLOG_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:zblog.db".to_owned()),
            password_hash: required("ZBLOG_PASSWORD_HASH")?,
            session_secret: required("ZBLOG_SESSION_SECRET")?,
        })
    }
}
