//! HTTP interface: router, state, and middleware.

pub mod dto;

use std::sync::Arc;

use axum::{routing::get, Json, Router};
use axum_extra::extract::cookie::Key;
use sqlx::SqlitePool;

use crate::config::Config;
use crate::infrastructure::article_repo::ArticleRepo;
use crate::infrastructure::pageview_repo::PageviewRepo;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Article repository.
    pub articles: ArticleRepo,
    /// Pageview repository.
    pub pageviews: PageviewRepo,
    /// Runtime configuration.
    pub config: Arc<Config>,
    /// Key used for private (signed+encrypted) cookies.
    pub cookie_key: Key,
}

impl AppState {
    /// Build state from a pool and config.
    ///
    /// The cookie key is derived deterministically from `session_secret`:
    /// the secret bytes are cycled into a 64-byte buffer (`Key::from`
    /// requires exactly 64 bytes and `Key::derive_from` needs the cookie
    /// `key-expansion` feature plus a >= 32-byte secret).
    #[must_use]
    pub fn new(pool: SqlitePool, config: Config) -> Self {
        let mut key_material = [0u8; 64];
        let secret = config.session_secret.as_bytes();
        if !secret.is_empty() {
            for (slot, byte) in key_material.iter_mut().zip(secret.iter().cycle()) {
                *slot = *byte;
            }
        }
        let cookie_key = Key::from(&key_material);
        Self {
            articles: ArticleRepo::new(pool.clone()),
            pageviews: PageviewRepo::new(pool),
            config: Arc::new(config),
            cookie_key,
        }
    }
}

impl axum::extract::FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}

/// Build the full HTTP router.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
