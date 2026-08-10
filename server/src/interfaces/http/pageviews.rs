//! Pageview ingestion endpoint (public; called by the visitor's browser).

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::domain::pageview::NewPageview;
use crate::error::{AppError, Result};
use crate::interfaces::http::dto::RecordPageviewRequest;
use crate::interfaces::http::AppState;

/// `POST /api/pageviews` — record one pageview.
///
/// The IP is the peer address; user agent and referer come from headers.
///
/// # Errors
///
/// Returns `AppError::BadRequest` when `path` is empty, and `AppError::Db`
/// when the insert fails.
pub async fn record(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<RecordPageviewRequest>,
) -> Result<StatusCode> {
    if body.path.is_empty() {
        return Err(AppError::BadRequest("path must not be empty".to_owned()));
    }
    let header_value = |name: &str| -> String {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned()
    };
    state
        .pageviews
        .record(&NewPageview {
            path: body.path,
            article_id: body.article_id,
            ip: addr.ip().to_string(),
            user_agent: header_value("user-agent"),
            referer: header_value("referer"),
        })
        .await?;
    Ok(StatusCode::CREATED)
}
