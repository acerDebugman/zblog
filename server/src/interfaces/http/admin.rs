//! Admin endpoints (Author-facing, behind the session cookie).

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar, SameSite};

use crate::domain::article::{Article, ArticleStatus};
use crate::error::{AppError, Result};
use crate::interfaces::http::dto::{CreateArticleRequest, LoginRequest, UpdateArticleRequest};
use crate::interfaces::http::middleware::AUTH_COOKIE;
use crate::interfaces::http::AppState;

/// `POST /api/admin/login` — verify password, issue the session cookie.
///
/// # Errors
/// Returns `AppError::Unauthorized` on wrong password, `AppError::Internal`
/// when the configured password hash is not a valid PHC string.
pub async fn login(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Json(body): Json<LoginRequest>,
) -> Result<(PrivateCookieJar, Json<serde_json::Value>)> {
    let parsed = PasswordHash::new(&state.config.password_hash)
        .map_err(|e| AppError::Internal(format!("invalid ZBLOG_PASSWORD_HASH: {e}")))?;
    let ok = Argon2::default()
        .verify_password(body.password.as_bytes(), &parsed)
        .is_ok();
    if !ok {
        return Err(AppError::Unauthorized);
    }
    let cookie = Cookie::build((AUTH_COOKIE, "authenticated"))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::days(7))
        .build();
    Ok((jar.add(cookie), Json(serde_json::json!({ "ok": true }))))
}

/// `POST /api/admin/logout` — drop the session cookie.
pub async fn logout(jar: PrivateCookieJar) -> (PrivateCookieJar, Json<serde_json::Value>) {
    let removal = Cookie::build((AUTH_COOKIE, "")).path("/").build();
    (jar.remove(removal), Json(serde_json::json!({ "ok": true })))
}

/// `GET /api/admin/me` — session probe used by the Astro middleware.
pub async fn me() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}

/// `GET /api/admin/articles` — all articles regardless of status.
///
/// # Errors
/// Returns `AppError::Db` on database failure.
pub async fn list_articles(State(state): State<AppState>) -> Result<Json<Vec<Article>>> {
    Ok(Json(state.articles.list_all().await?))
}

/// `POST /api/admin/articles` — create a draft.
///
/// # Errors
/// Returns `AppError::BadRequest` when the title is empty, `AppError::Db` on
/// database failure.
pub async fn create_article(
    State(state): State<AppState>,
    Json(body): Json<CreateArticleRequest>,
) -> Result<(StatusCode, Json<Article>)> {
    if body.title.trim().is_empty() {
        return Err(AppError::BadRequest("title must not be empty".to_owned()));
    }
    let article = state.articles.create(&body.title, &body.markdown).await?;
    Ok((StatusCode::CREATED, Json(article)))
}

/// `GET /api/admin/articles/{id}` — one article for editing.
///
/// # Errors
/// Returns `AppError::NotFound` for unknown ids, `AppError::Db` on database
/// failure.
pub async fn get_article(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Article>> {
    state
        .articles
        .find_by_id(id)
        .await?
        .map_or(Err(AppError::NotFound), |a| Ok(Json(a)))
}

/// `GET /api/admin/articles/by-slug/{slug}` — draft preview lookup.
///
/// # Errors
/// Returns `AppError::NotFound` for unknown slugs, `AppError::Db` on database
/// failure.
pub async fn get_article_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Article>> {
    state
        .articles
        .find_by_slug(&slug)
        .await?
        .map_or(Err(AppError::NotFound), |a| Ok(Json(a)))
}

/// `PUT /api/admin/articles/{id}` — update title/slug/markdown.
///
/// # Errors
/// Returns `AppError::BadRequest` when the title is empty or the slug is
/// invalid/taken, `AppError::NotFound` for unknown ids, `AppError::Db` on
/// database failure.
pub async fn update_article(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateArticleRequest>,
) -> Result<Json<Article>> {
    if body.title.trim().is_empty() {
        return Err(AppError::BadRequest("title must not be empty".to_owned()));
    }
    Ok(Json(
        state
            .articles
            .update(id, &body.title, &body.slug, &body.markdown)
            .await?,
    ))
}

/// `POST /api/admin/articles/{id}/publish`.
///
/// # Errors
/// Returns `AppError::NotFound` for unknown ids, `AppError::Db` on database
/// failure.
pub async fn publish_article(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Article>> {
    Ok(Json(
        state
            .articles
            .set_status(id, ArticleStatus::Published)
            .await?,
    ))
}

/// `POST /api/admin/articles/{id}/unpublish`.
///
/// # Errors
/// Returns `AppError::NotFound` for unknown ids, `AppError::Db` on database
/// failure.
pub async fn unpublish_article(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Article>> {
    Ok(Json(
        state.articles.set_status(id, ArticleStatus::Draft).await?,
    ))
}

/// `DELETE /api/admin/articles/{id}`.
///
/// # Errors
/// Returns `AppError::NotFound` for unknown ids, `AppError::Db` on database
/// failure.
pub async fn delete_article(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode> {
    match state.articles.delete(id).await? {
        0 => Err(AppError::NotFound),
        _ => Ok(StatusCode::NO_CONTENT),
    }
}
