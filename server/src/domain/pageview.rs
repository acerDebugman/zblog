//! Pageview domain model.

/// A pageview to record (see CONTEXT.md `PageView`).
#[derive(Debug, Clone)]
pub struct NewPageview {
    /// Request path.
    pub path: String,
    /// Article id when the page shows an article.
    pub article_id: Option<i64>,
    /// Visitor IP (stored in clear by design).
    pub ip: String,
    /// Visitor user agent; empty when unknown.
    pub user_agent: String,
    /// Visitor referer; empty when unknown.
    pub referer: String,
}
