//! Article domain model.

use crate::error::{AppError, Result};

/// Publication status of an [`Article`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ArticleStatus {
    /// Work in progress, invisible to Visitors.
    Draft,
    /// Visible to Visitors.
    Published,
}

impl ArticleStatus {
    /// Storage representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
        }
    }

    /// Parse from the storage representation.
    ///
    /// # Errors
    /// Returns `AppError::Internal` for unknown values.
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "draft" => Ok(Self::Draft),
            "published" => Ok(Self::Published),
            other => Err(AppError::Internal(format!(
                "unknown article status: {other}"
            ))),
        }
    }
}

/// The single content entity of the blog (see CONTEXT.md).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Article {
    /// Database id.
    pub id: i64,
    /// Display title.
    pub title: String,
    /// Unique URL slug; public URL is `/posts/{slug}`.
    pub slug: String,
    /// Markdown source (rendering happens in Astro).
    pub markdown: String,
    /// Publication status.
    pub status: ArticleStatus,
    /// UTC creation timestamp `YYYY-MM-DD HH:MM:SS`.
    pub created_at: String,
    /// UTC last-update timestamp.
    pub updated_at: String,
    /// UTC first-publish timestamp; `None` while never published.
    pub published_at: Option<String>,
}

/// Derive a URL slug from a title: lowercase ASCII alphanumeric runs joined by
/// single dashes. Empty when the title has no ASCII alphanumeric characters.
#[must_use]
pub fn slugify(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    let mut pending_dash = false;
    for c in title.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(c);
        } else {
            pending_dash = true;
        }
    }
    slug
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic_ascii() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn slugify_strips_punctuation_and_collapses_dashes() {
        assert_eq!(slugify("Rust 1.0:  新特性!"), "rust-1-0");
    }

    #[test]
    fn slugify_chinese_only_is_empty() {
        assert_eq!(slugify("中文标题"), "");
    }

    #[test]
    fn status_roundtrip() {
        assert_eq!(ArticleStatus::parse("draft").unwrap(), ArticleStatus::Draft);
        assert_eq!(
            ArticleStatus::parse("published").unwrap().as_str(),
            "published"
        );
        assert!(ArticleStatus::parse("archived").is_err());
    }
}
