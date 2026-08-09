//! `SQLite` persistence for articles.

use sqlx::{FromRow, SqlitePool};

use crate::domain::article::{slugify, Article, ArticleStatus};
use crate::error::{AppError, Result};

#[derive(FromRow)]
struct ArticleRow {
    id: i64,
    title: String,
    slug: String,
    markdown: String,
    status: String,
    created_at: String,
    updated_at: String,
    published_at: Option<String>,
}

impl TryFrom<ArticleRow> for Article {
    type Error = AppError;

    fn try_from(row: ArticleRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            title: row.title,
            slug: row.slug,
            markdown: row.markdown,
            status: ArticleStatus::parse(&row.status)?,
            created_at: row.created_at,
            updated_at: row.updated_at,
            published_at: row.published_at,
        })
    }
}

const COLS: &str = "id, title, slug, markdown, status, created_at, updated_at, published_at";

/// Article repository.
#[derive(Clone)]
pub struct ArticleRepo {
    pool: SqlitePool,
}

impl ArticleRepo {
    /// Create the repository.
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a draft article with a unique slug derived from the title.
    ///
    /// # Errors
    /// Returns `AppError::Db` on database failure.
    pub async fn create(&self, title: &str, markdown: &str) -> Result<Article> {
        let base = slugify(title);
        let base = if base.is_empty() {
            "post".to_owned()
        } else {
            base
        };
        for attempt in 0_u32..100 {
            let slug = if attempt == 0 {
                base.clone()
            } else {
                format!("{base}-{}", attempt + 1)
            };
            let result = sqlx::query_as::<_, ArticleRow>(&format!(
                "INSERT INTO articles (title, slug, markdown) VALUES (?, ?, ?) RETURNING {COLS}"
            ))
            .bind(title)
            .bind(&slug)
            .bind(markdown)
            .fetch_one(&self.pool)
            .await;
            match result {
                Ok(row) => return row.try_into(),
                Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {}
                Err(e) => return Err(AppError::Db(e)),
            }
        }
        Err(AppError::Internal(
            "could not allocate a unique slug".to_owned(),
        ))
    }

    /// Update title, slug, and markdown of an existing article.
    ///
    /// # Errors
    /// `NotFound` when the id does not exist; `BadRequest` when the slug is
    /// empty or already used by another article; `AppError::Db` otherwise.
    pub async fn update(
        &self,
        id: i64,
        title: &str,
        slug: &str,
        markdown: &str,
    ) -> Result<Article> {
        if slug.is_empty() {
            return Err(AppError::BadRequest("slug must not be empty".to_owned()));
        }
        let result = sqlx::query_as::<_, ArticleRow>(&format!(
            "UPDATE articles SET title = ?, slug = ?, markdown = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ? RETURNING {COLS}"
        ))
        .bind(title)
        .bind(slug)
        .bind(markdown)
        .bind(id)
        .fetch_optional(&self.pool)
        .await;
        match result {
            Ok(Some(row)) => row.try_into(),
            Ok(None) => Err(AppError::NotFound),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                Err(AppError::BadRequest("slug already exists".to_owned()))
            }
            Err(e) => Err(AppError::Db(e)),
        }
    }

    /// Change publication status. Publishing preserves the first `published_at`.
    ///
    /// # Errors
    /// `NotFound` when the id does not exist; `AppError::Db` otherwise.
    pub async fn set_status(&self, id: i64, status: ArticleStatus) -> Result<Article> {
        let row = sqlx::query_as::<_, ArticleRow>(&format!(
            "UPDATE articles SET status = ?, \
             published_at = CASE WHEN ? = 'published' \
                 THEN COALESCE(published_at, CURRENT_TIMESTAMP) ELSE published_at END, \
             updated_at = CURRENT_TIMESTAMP WHERE id = ? RETURNING {COLS}"
        ))
        .bind(status.as_str())
        .bind(status.as_str())
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map_or(Err(AppError::NotFound), TryInto::try_into)
    }

    /// Find an article by id.
    ///
    /// # Errors
    /// Returns `AppError::Db` on database failure.
    pub async fn find_by_id(&self, id: i64) -> Result<Option<Article>> {
        let row =
            sqlx::query_as::<_, ArticleRow>(&format!("SELECT {COLS} FROM articles WHERE id = ?"))
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Find an article by slug (any status).
    ///
    /// # Errors
    /// Returns `AppError::Db` on database failure.
    pub async fn find_by_slug(&self, slug: &str) -> Result<Option<Article>> {
        let row =
            sqlx::query_as::<_, ArticleRow>(&format!("SELECT {COLS} FROM articles WHERE slug = ?"))
                .bind(slug)
                .fetch_optional(&self.pool)
                .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// List published articles, newest first.
    ///
    /// # Errors
    /// Returns `AppError::Db` on database failure.
    pub async fn list_published(&self) -> Result<Vec<Article>> {
        let rows = sqlx::query_as::<_, ArticleRow>(&format!(
            "SELECT {COLS} FROM articles WHERE status = 'published' \
             ORDER BY published_at DESC, id DESC"
        ))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// List all articles regardless of status, most recently updated first.
    ///
    /// # Errors
    /// Returns `AppError::Db` on database failure.
    pub async fn list_all(&self) -> Result<Vec<Article>> {
        let rows = sqlx::query_as::<_, ArticleRow>(&format!(
            "SELECT {COLS} FROM articles ORDER BY updated_at DESC, id DESC"
        ))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Delete an article; returns the number of rows removed (0 or 1).
    ///
    /// # Errors
    /// Returns `AppError::Db` on database failure.
    pub async fn delete(&self, id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM articles WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::article::ArticleStatus;

    async fn repo() -> (ArticleRepo, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite:{}", dir.path().join("t.db").display());
        let pool = crate::infrastructure::db::create_pool(&url).await.unwrap();
        (ArticleRepo::new(pool), dir)
    }

    #[tokio::test]
    async fn create_generates_unique_slugs() {
        let (repo, _dir) = repo().await;
        let a = repo.create("Hello World", "# one").await.unwrap();
        let b = repo.create("Hello World", "# two").await.unwrap();
        let c = repo.create("中文标题", "# three").await.unwrap();
        let d = repo.create("中文标题", "# four").await.unwrap();
        assert_eq!(a.slug, "hello-world");
        assert_eq!(b.slug, "hello-world-2");
        assert_eq!(c.slug, "post");
        assert_eq!(d.slug, "post-2");
        assert_eq!(a.status, ArticleStatus::Draft);
        assert!(a.published_at.is_none());
    }

    #[tokio::test]
    async fn lists_respect_status_and_order() {
        let (repo, _dir) = repo().await;
        let draft = repo.create("Alpha Post", "a").await.unwrap();
        let published = repo.create("Beta Post", "b").await.unwrap();
        repo.set_status(published.id, ArticleStatus::Published)
            .await
            .unwrap();

        let public = repo.list_published().await.unwrap();
        assert_eq!(public.len(), 1);
        assert_eq!(public[0].title, "Beta Post"); // negative: draft excluded
        assert!(public[0].published_at.is_some());

        let all = repo.list_all().await.unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|a| a.id == draft.id));
    }

    #[tokio::test]
    async fn publish_keeps_first_published_at_after_unpublish() {
        let (repo, _dir) = repo().await;
        let a = repo.create("Gamma Post", "g").await.unwrap();
        let p1 = repo
            .set_status(a.id, ArticleStatus::Published)
            .await
            .unwrap();
        let back = repo.set_status(a.id, ArticleStatus::Draft).await.unwrap();
        assert_eq!(back.status, ArticleStatus::Draft);
        let p2 = repo
            .set_status(a.id, ArticleStatus::Published)
            .await
            .unwrap();
        assert_eq!(p1.published_at, p2.published_at);
    }

    #[tokio::test]
    async fn update_and_delete() {
        let (repo, _dir) = repo().await;
        let a = repo.create("Delta Post", "old").await.unwrap();
        let updated = repo
            .update(a.id, "Delta Renamed", "delta-renamed", "new")
            .await
            .unwrap();
        assert_eq!(updated.markdown, "new");
        assert_eq!(updated.slug, "delta-renamed");
        assert!(repo.update(a.id, "X", "", "y").await.is_err()); // empty slug rejected

        let b = repo.create("Echo Post", "e").await.unwrap();
        assert!(repo
            .update(b.id, "Echo Post", "delta-renamed", "e")
            .await
            .is_err()); // conflict

        assert_eq!(repo.delete(a.id).await.unwrap(), 1);
        assert!(repo.find_by_id(a.id).await.unwrap().is_none());
        assert_eq!(repo.delete(a.id).await.unwrap(), 0); // negative: already gone
    }
}
