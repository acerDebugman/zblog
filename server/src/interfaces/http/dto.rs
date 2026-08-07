//! Request DTOs.

use serde::Deserialize;

/// Login payload.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Plaintext admin password.
    pub password: String,
}

/// Article creation payload (always creates a Draft).
#[derive(Debug, Deserialize)]
pub struct CreateArticleRequest {
    /// Article title.
    pub title: String,
    /// Markdown source.
    pub markdown: String,
}

/// Article update payload.
#[derive(Debug, Deserialize)]
pub struct UpdateArticleRequest {
    /// Article title.
    pub title: String,
    /// Desired slug.
    pub slug: String,
    /// Markdown source.
    pub markdown: String,
}

/// Pageview ingestion payload (sent by the Astro SSR layer).
#[derive(Debug, Deserialize)]
pub struct RecordPageviewRequest {
    /// Request path, e.g. `/posts/hello`.
    pub path: String,
    /// Article id when the page is an article.
    pub article_id: Option<i64>,
    /// Visitor IP as seen by the Astro server.
    pub ip: String,
    /// Visitor user agent.
    pub user_agent: Option<String>,
    /// Visitor referer.
    pub referer: Option<String>,
}

/// Query for the daily stats endpoint.
#[derive(Debug, Deserialize)]
pub struct DailyQuery {
    /// How many days back; defaults to 30.
    pub days: Option<i64>,
}

/// Query for bounded list endpoints.
#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    /// Max rows; defaults to 20.
    pub limit: Option<i64>,
}
