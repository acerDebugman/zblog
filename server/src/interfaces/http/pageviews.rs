//! Pageview ingestion endpoint (called by the Astro SSR layer only).

use axum::{extract::State, http::StatusCode, Json};

use crate::domain::pageview::NewPageview;
use crate::error::{AppError, Result};
use crate::interfaces::http::dto::RecordPageviewRequest;
use crate::interfaces::http::AppState;

/// `POST /api/pageviews` — record one pageview.
///
/// # Errors
///
/// Returns `AppError::BadRequest` when `path` or `ip` is empty, and
/// `AppError::Db` when the insert fails.
pub async fn record(
    State(state): State<AppState>,
    Json(body): Json<RecordPageviewRequest>,
) -> Result<StatusCode> {
    if body.path.is_empty() {
        return Err(AppError::BadRequest("path must not be empty".to_owned()));
    }
    if body.ip.is_empty() {
        return Err(AppError::BadRequest("ip must not be empty".to_owned()));
    }
    state
        .pageviews
        .record(&NewPageview {
            path: body.path,
            article_id: body.article_id,
            ip: body.ip,
            user_agent: body.user_agent.unwrap_or_default(),
            referer: body.referer.unwrap_or_default(),
        })
        .await?;
    Ok(StatusCode::CREATED)
}
