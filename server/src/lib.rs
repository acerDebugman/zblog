//! zblog backend library.

pub mod error;

use axum::{routing::get, Json, Router};

/// Router exposing only the health endpoint; extended by later tasks.
pub fn health_router() -> Router {
    Router::new().route("/api/health", get(health))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
