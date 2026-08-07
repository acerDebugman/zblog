//! Public article endpoints (Visitor-facing).

use axum::{
    extract::{Path, State},
    Json,
};

use crate::domain::article::{Article, ArticleStatus};
use crate::error::{AppError, Result};
use crate::interfaces::http::AppState;

/// `GET /api/articles` — published articles, newest first.
///
/// # Errors
/// Returns `AppError::Db` on database failure.
pub async fn list_published(State(state): State<AppState>) -> Result<Json<Vec<Article>>> {
    Ok(Json(state.articles.list_published().await?))
}

/// `GET /api/articles/{slug}` — one published article; drafts are invisible.
///
/// # Errors
/// Returns `AppError::NotFound` for drafts and unknown slugs, `AppError::Db`
/// on database failure.
pub async fn get_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Article>> {
    match state.articles.find_by_slug(&slug).await? {
        Some(article) if article.status == ArticleStatus::Published => Ok(Json(article)),
        _ => Err(AppError::NotFound),
    }
}
