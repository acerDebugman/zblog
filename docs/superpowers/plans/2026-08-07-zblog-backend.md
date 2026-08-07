# zblog Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Rust (axum + sqlx/SQLite) HTTP API for zblog: article CRUD with draft/publish, password+session admin auth, pageview ingestion, and traffic statistics.

**Architecture:** Single Cargo crate `server/` with layered modules (domain → application is folded into repositories/handlers for this size → infrastructure → interfaces). Astro (separate plan) is the only public entry and calls this API over localhost; the API binds to an internal address only (ADR-0003). Markdown stays as source-of-truth text — rendering happens in Astro (ADR-0001). Drafts are a status on the single `articles` table (ADR-0002).

**Tech Stack:** Rust 1.96, axum 0.8, sqlx 0.8 (SQLite, runtime-tokio-rustls), axum-extra 0.10 (private cookies), argon2 0.5, thiserror 2, reqwest + tempfile (dev).

## Global Constraints

- Timestamps are SQLite `TEXT` in UTC `YYYY-MM-DD HH:MM:SS` (`CURRENT_TIMESTAMP` defaults). No chrono/time parsing in Rust domain code.
- All user-facing strings in Rust code (errors, logs) in **English**.
- No `unwrap()`/`expect()`/`panic!()` outside tests; tests start with `#![allow(clippy::unwrap_used)]`.
- Strict lints live in `server/Cargo.toml` `[lints]` tables (defined in Task 1) — every `cargo clippy` run must be clean.
- Commits follow Conventional Commits in Chinese, e.g. `feat(server): 添加文章发布接口`.
- Domain terms per `CONTEXT.md`: Article / Draft / Published / Slug / PageView / Author / Visitor.
- Tests must verify content (not just counts), use mutually exclusive data, strict equality, and verify negative conditions (AGENTS.md).

---

### Task 1: Crate scaffold, lint config, AppError, health endpoint

**Files:**
- Create: `server/Cargo.toml`
- Create: `server/src/main.rs`
- Create: `server/src/lib.rs`
- Create: `server/src/error.rs`
- Test: `server/tests/api.rs`

**Interfaces:**
- Produces: `zblog_server::error::{AppError, Result<T>}`; `AppError` variants `Db(sqlx::Error)`, `NotFound`, `Unauthorized`, `BadRequest(String)`, `Internal(String)`; `AppError: IntoResponse` (404/401/400/500 + JSON `{"error": "..."}`).

- [ ] **Step 1: Write the failing test**

`server/tests/api.rs`:

```rust
#![allow(clippy::unwrap_used)]

use reqwest::StatusCode;

#[tokio::test]
async fn health_returns_ok() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, zblog_server::health_router()).await.unwrap();
    });
    let res = reqwest::get(format!("http://{addr}/api/health")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path server/Cargo.toml`
Expected: FAIL — crate `zblog_server` does not exist.

- [ ] **Step 3: Write minimal implementation**

`server/Cargo.toml`:

```toml
[package]
name = "zblog-server"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.8"
axum-extra = { version = "0.10", features = ["cookie-private"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio-rustls", "sqlite"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
argon2 = "0.5"
time = "0.3"
tracing = "0.1"
tracing-subscriber = "0.3"

[dev-dependencies]
reqwest = { version = "0.12", features = ["json", "cookies"] }
tempfile = "3"

[lints.rust]
unsafe_code = "forbid"
warnings = "deny"

[lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

`server/src/lib.rs`:

```rust
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
```

`server/src/error.rs`:

```rust
//! Unified application error type.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

/// Application-wide error type.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Database layer error.
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    /// Resource not found.
    #[error("not found")]
    NotFound,
    /// Authentication missing or invalid.
    #[error("unauthorized")]
    Unauthorized,
    /// Client sent invalid data.
    #[error("bad request: {0}")]
    BadRequest(String),
    /// Unexpected internal failure.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Application-wide result alias.
pub type Result<T> = std::result::Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Db(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(serde_json::json!({ "error": self.to_string() }))).into_response()
    }
}
```

`server/src/main.rs`:

```rust
//! zblog backend binary.

fn main() {
    println!("zblog-server: library crate; HTTP server entry arrives in a later task");
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path server/Cargo.toml && cargo clippy --manifest-path server/Cargo.toml --all-targets`
Expected: test PASS, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add server/
git commit -m "feat(server): 初始化 axum 工程与健康检查接口"
```

---

### Task 2: Config, SQLite pool, initial migration

**Files:**
- Create: `server/src/config.rs`
- Create: `server/src/infrastructure/mod.rs`
- Create: `server/src/infrastructure/db.rs`
- Create: `server/migrations/0001_init.sql`
- Modify: `server/src/lib.rs`
- Test: `server/tests/api.rs` (append)

**Interfaces:**
- Consumes: `AppError`, `Result` from Task 1.
- Produces: `config::Config { bind_addr, database_url, password_hash, session_secret }`, `Config::from_env() -> Result<Config>`; `db::create_pool(&str) -> Result<SqlitePool>` (applies migrations). Tables `articles`, `pageviews` per schema below.

- [ ] **Step 1: Write the failing test**

Append to `server/tests/api.rs`:

```rust
#[tokio::test]
async fn migrations_create_tables() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite:{}", dir.path().join("t.db").display());
    let pool = zblog_server::infrastructure::db::create_pool(&url).await.unwrap();
    let names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(names.contains(&"articles".to_owned()));
    assert!(names.contains(&"pageviews".to_owned()));
}
```

(`sqlx` must be importable from the test crate — it is already a direct dependency; integration tests may use it.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path server/Cargo.toml`
Expected: FAIL — unresolved module `infrastructure`.

- [ ] **Step 3: Write minimal implementation**

`server/migrations/0001_init.sql`:

```sql
CREATE TABLE articles (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    title        TEXT NOT NULL,
    slug         TEXT NOT NULL UNIQUE,
    markdown     TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published')),
    created_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    published_at TEXT
);
CREATE INDEX idx_articles_status_published_at ON articles (status, published_at DESC);

CREATE TABLE pageviews (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    path       TEXT NOT NULL,
    article_id INTEGER REFERENCES articles (id) ON DELETE SET NULL,
    ip         TEXT NOT NULL,
    user_agent TEXT NOT NULL DEFAULT '',
    referer    TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_pageviews_created_at ON pageviews (created_at);
CREATE INDEX idx_pageviews_ip ON pageviews (ip);
CREATE INDEX idx_pageviews_article_id ON pageviews (article_id);
```

`server/src/config.rs`:

```rust
//! Runtime configuration from environment variables.

use crate::error::{AppError, Result};

/// Runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address the HTTP server binds to, e.g. `127.0.0.1:8080`.
    pub bind_addr: String,
    /// SQLite connection URL, e.g. `sqlite:zblog.db`.
    pub database_url: String,
    /// Argon2 PHC hash of the admin password.
    pub password_hash: String,
    /// Secret used to sign session cookies.
    pub session_secret: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    /// Returns `AppError::Internal` when a required variable is missing.
    pub fn from_env() -> Result<Self> {
        let required = |key: &str| -> Result<String> {
            std::env::var(key).map_err(|_| AppError::Internal(format!("missing env var {key}")))
        };
        Ok(Self {
            bind_addr: std::env::var("ZBLOG_BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:8080".to_owned()),
            database_url: std::env::var("ZBLOG_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:zblog.db".to_owned()),
            password_hash: required("ZBLOG_PASSWORD_HASH")?,
            session_secret: required("ZBLOG_SESSION_SECRET")?,
        })
    }
}
```

`server/src/infrastructure/mod.rs`:

```rust
//! Infrastructure layer: database access.

pub mod db;
```

`server/src/infrastructure/db.rs`:

```rust
//! SQLite connection pool and migrations.

use std::str::FromStr;

use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions}, SqlitePool};

use crate::error::Result;

/// Create a connection pool and apply all pending migrations.
///
/// # Errors
/// Returns `AppError::Db` when the database cannot be opened or migrations fail.
pub async fn create_pool(database_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
    let pool = SqlitePoolOptions::new().max_connections(5).connect_with(options).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
```

`server/src/lib.rs` — add below `pub mod error;`:

```rust
pub mod config;
pub mod infrastructure;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path server/Cargo.toml && cargo clippy --manifest-path server/Cargo.toml --all-targets`
Expected: both tests PASS, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add server/
git commit -m "feat(server): 添加配置加载与 SQLite 连接池及初始迁移"
```

---

### Task 3: Article domain model and slugify

**Files:**
- Create: `server/src/domain/mod.rs`
- Create: `server/src/domain/article.rs`
- Modify: `server/src/lib.rs`

**Interfaces:**
- Produces: `domain::article::{Article, ArticleStatus, slugify}`.
  - `ArticleStatus::{Draft, Published}`, `as_str(self) -> &'static str` (`"draft"`/`"published"`), `parse(&str) -> Result<ArticleStatus>`; serializes lowercase via serde.
  - `Article { id: i64, title: String, slug: String, markdown: String, status: ArticleStatus, created_at: String, updated_at: String, published_at: Option<String> }`, `Serialize`.
  - `slugify(&str) -> String`: lowercase ASCII alphanumeric runs joined by single `-`; empty when no ASCII alnum.

- [ ] **Step 1: Write the failing test**

`server/src/domain/article.rs` will contain `#[cfg(test)]` tests — first write only the test module at the bottom of an empty `article.rs` (with `mod` declared in `mod.rs`) so it fails to compile:

```rust
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
        assert_eq!(ArticleStatus::parse("published").unwrap().as_str(), "published");
        assert!(ArticleStatus::parse("archived").is_err());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path server/Cargo.toml`
Expected: FAIL — unresolved functions/types.

- [ ] **Step 3: Write minimal implementation**

`server/src/domain/mod.rs`:

```rust
//! Domain layer: core blog concepts.

pub mod article;
```

`server/src/domain/article.rs` (above the test module):

```rust
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
    pub fn as_str(self) -> &'static str {
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
            other => Err(AppError::Internal(format!("unknown article status: {other}"))),
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
```

`server/src/lib.rs` — add:

```rust
pub mod domain;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path server/Cargo.toml && cargo clippy --manifest-path server/Cargo.toml --all-targets`
Expected: PASS, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add server/
git commit -m "feat(server): 添加文章领域模型与 slug 生成"
```

---

### Task 4: Article repository

**Files:**
- Create: `server/src/infrastructure/article_repo.rs`
- Modify: `server/src/infrastructure/mod.rs`

**Interfaces:**
- Consumes: `db::create_pool`, `domain::article::*`, `AppError`, `Result`.
- Produces: `ArticleRepo` (Clone, `new(pool: SqlitePool)`):
  - `create(title, markdown) -> Result<Article>` — always a Draft; unique slug from `slugify(title)` (empty → base `"post"`), conflicts suffixed `-2`, `-3`, …
  - `update(id, title, slug, markdown) -> Result<Article>` — `NotFound`; empty slug → `BadRequest`; slug conflict → `BadRequest("slug already exists")`
  - `set_status(id, ArticleStatus) -> Result<Article>` — publishing sets `published_at` once (kept on unpublish)
  - `find_by_id(id) -> Result<Option<Article>>`, `find_by_slug(slug) -> Result<Option<Article>>`
  - `list_published() -> Result<Vec<Article>>` (published_at desc), `list_all() -> Result<Vec<Article>>` (updated_at desc)
  - `delete(id) -> Result<u64>` (rows affected)

- [ ] **Step 1: Write the failing test**

`server/src/infrastructure/article_repo.rs` bottom `#[cfg(test)]` module (file starts with only `use` items + this test so it fails):

```rust
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
        repo.set_status(published.id, ArticleStatus::Published).await.unwrap();

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
        let p1 = repo.set_status(a.id, ArticleStatus::Published).await.unwrap();
        let back = repo.set_status(a.id, ArticleStatus::Draft).await.unwrap();
        assert_eq!(back.status, ArticleStatus::Draft);
        let p2 = repo.set_status(a.id, ArticleStatus::Published).await.unwrap();
        assert_eq!(p1.published_at, p2.published_at);
    }

    #[tokio::test]
    async fn update_and_delete() {
        let (repo, _dir) = repo().await;
        let a = repo.create("Delta Post", "old").await.unwrap();
        let updated = repo.update(a.id, "Delta Renamed", "delta-renamed", "new").await.unwrap();
        assert_eq!(updated.markdown, "new");
        assert_eq!(updated.slug, "delta-renamed");
        assert!(repo.update(a.id, "X", "", "y").await.is_err()); // empty slug rejected

        let b = repo.create("Echo Post", "e").await.unwrap();
        assert!(repo.update(b.id, "Echo Post", "delta-renamed", "e").await.is_err()); // conflict

        assert_eq!(repo.delete(a.id).await.unwrap(), 1);
        assert!(repo.find_by_id(a.id).await.unwrap().is_none());
        assert_eq!(repo.delete(a.id).await.unwrap(), 0); // negative: already gone
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path server/Cargo.toml`
Expected: FAIL — `ArticleRepo` undefined.

- [ ] **Step 3: Write minimal implementation**

`server/src/infrastructure/article_repo.rs` (above the test module):

```rust
//! SQLite persistence for articles.

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
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a draft article with a unique slug derived from the title.
    ///
    /// # Errors
    /// Returns `AppError::Db` on database failure.
    pub async fn create(&self, title: &str, markdown: &str) -> Result<Article> {
        let base = slugify(title);
        let base = if base.is_empty() { "post".to_owned() } else { base };
        for attempt in 0_u32..100 {
            let slug = if attempt == 0 { base.clone() } else { format!("{base}-{}", attempt + 1) };
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
                Err(sqlx::Error::Database(e)) if e.is_unique_violation() => continue,
                Err(e) => return Err(AppError::Db(e)),
            }
        }
        Err(AppError::Internal("could not allocate a unique slug".to_owned()))
    }

    /// Update title, slug, and markdown of an existing article.
    ///
    /// # Errors
    /// `NotFound` when the id does not exist; `BadRequest` when the slug is
    /// empty or already used by another article; `AppError::Db` otherwise.
    pub async fn update(&self, id: i64, title: &str, slug: &str, markdown: &str) -> Result<Article> {
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
        let row = sqlx::query_as::<_, ArticleRow>(&format!(
            "SELECT {COLS} FROM articles WHERE id = ?"
        ))
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
        let row = sqlx::query_as::<_, ArticleRow>(&format!(
            "SELECT {COLS} FROM articles WHERE slug = ?"
        ))
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
```

`server/src/infrastructure/mod.rs` — add:

```rust
pub mod article_repo;
```

Note: `tempfile` is used by the in-file tests — it is already in `[dev-dependencies]`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path server/Cargo.toml && cargo clippy --manifest-path server/Cargo.toml --all-targets`
Expected: PASS, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add server/
git commit -m "feat(server): 添加文章仓储层"
```

---

### Task 5: HTTP shell — AppState, router, main entry, shared test harness

**Files:**
- Create: `server/src/interfaces/mod.rs`
- Create: `server/src/interfaces/http/mod.rs`
- Create: `server/src/interfaces/http/dto.rs`
- Modify: `server/src/lib.rs`, `server/src/main.rs`
- Test: `server/tests/api.rs` (rewrite — replace Task 1's `health_returns_ok` with harness-based version)

**Interfaces:**
- Consumes: everything so far.
- Produces:
  - `interfaces::http::AppState` (Clone): `articles: ArticleRepo`, `pageviews: PageviewRepo` (placeholder struct until Task 8 — see note), `config: Arc<Config>`, `cookie_key: axum_extra::extract::cookie::Key`; `AppState::new(pool, config) -> AppState`; `impl FromRef<AppState> for Key`.
  - `interfaces::http::build_router(state) -> Router` with `/api/health`.
  - Test harness `spawn_app() -> TestApp { base_url, client }` in `tests/api.rs` (cookie-store reqwest client, temp DB, password `"test-password"`).
  - Binary binds `config.bind_addr` and serves `build_router`.

Note: `AppState` references `PageviewRepo` which only exists from Task 8. To keep this task compilable, create a stub `infrastructure/pageview_repo.rs` now containing only `#[derive(Clone)] pub struct PageviewRepo;` with `pub fn new(_pool: SqlitePool) -> Self`; Tasks 8–9 replace its body. Do the same-stub trick nowhere else.

- [ ] **Step 1: Write the failing test**

Replace the whole of `server/tests/api.rs` with:

```rust
#![allow(clippy::unwrap_used)]

use argon2::{password_hash::{rand_core::OsRng, SaltString}, Argon2, PasswordHasher};
use reqwest::{Client, StatusCode};
use tempfile::TempDir;
use zblog_server::config::Config;
use zblog_server::infrastructure::db;
use zblog_server::interfaces::http::{build_router, AppState};

struct TestApp {
    base_url: String,
    client: Client,
    _dir: TempDir,
}

async fn spawn_app() -> TestApp {
    let dir = tempfile::tempdir().unwrap();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(b"test-password", &salt)
        .unwrap()
        .to_string();
    let config = Config {
        bind_addr: "127.0.0.1:0".to_owned(),
        database_url: format!("sqlite:{}", dir.path().join("test.db").display()),
        password_hash,
        session_secret: "test-secret".to_owned(),
    };
    let pool = db::create_pool(&config.database_url).await.unwrap();
    let state = AppState::new(pool, config);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, build_router(state)).await.unwrap() });
    let client = Client::builder().cookie_store(true).build().unwrap();
    TestApp { base_url: format!("http://{addr}"), client, _dir: dir }
}

#[tokio::test]
async fn health_returns_ok() {
    let app = spawn_app().await;
    let res = app.client.get(format!("{}/api/health", app.base_url)).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}
```

(`argon2` is needed by tests — move it is already a main dependency, available to tests.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path server/Cargo.toml`
Expected: FAIL — unresolved `interfaces` module.

- [ ] **Step 3: Write minimal implementation**

`server/src/interfaces/mod.rs`:

```rust
//! Interfaces layer: HTTP API.

pub mod http;
```

`server/src/interfaces/http/dto.rs`:

```rust
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
```

`server/src/interfaces/http/mod.rs`:

```rust
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
    #[must_use]
    pub fn new(pool: SqlitePool, config: Config) -> Self {
        let cookie_key = Key::derive_from(config.session_secret.as_bytes());
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
    Router::new().route("/api/health", get(health)).with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
```

`server/src/infrastructure/pageview_repo.rs` (stub for now):

```rust
//! SQLite persistence for pageviews (stub — completed in the pageview task).

use sqlx::SqlitePool;

/// Pageview repository.
#[derive(Clone)]
pub struct PageviewRepo;

impl PageviewRepo {
    /// Create the repository.
    #[must_use]
    pub fn new(_pool: SqlitePool) -> Self {
        Self
    }
}
```

`server/src/infrastructure/mod.rs` — add `pub mod pageview_repo;`.
`server/src/lib.rs` — add `pub mod interfaces;`, and remove `health_router` plus the `axum` use items (the health test now goes through `build_router`).

`server/src/main.rs`:

```rust
//! zblog backend binary.

use zblog_server::config::Config;
use zblog_server::error::{AppError, Result};
use zblog_server::infrastructure::db;
use zblog_server::interfaces::http::{build_router, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let config = Config::from_env()?;
    let bind_addr = config.bind_addr.clone();
    let pool = db::create_pool(&config.database_url).await?;
    let state = AppState::new(pool, config);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| AppError::Internal(format!("bind {bind_addr}: {e}")))?;
    tracing::info!("listening on {bind_addr}");
    axum::serve(listener, build_router(state))
        .await
        .map_err(|e| AppError::Internal(format!("server error: {e}")))?;
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path server/Cargo.toml && cargo clippy --manifest-path server/Cargo.toml --all-targets`
Expected: PASS, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add server/
git commit -m "feat(server): 搭建 HTTP 路由骨架与应用状态"
```

---

### Task 6: Public articles API

**Files:**
- Create: `server/src/interfaces/http/articles.rs`
- Modify: `server/src/interfaces/http/mod.rs`
- Test: `server/tests/api.rs` (append)

**Interfaces:**
- Consumes: `AppState`, `ArticleRepo`, DTOs.
- Produces: `GET /api/articles -> 200 [Article]` (published only, newest first); `GET /api/articles/{slug} -> 200 Article | 404` (drafts are 404 to Visitors). Admin write endpoints arrive in Task 7; tests here seed data via `state`-level SQL through a helper — see test code (insert via `sqlx` against the app's DB is not possible since the pool lives inside the app; instead, create drafts by calling the repo through a second pool on the same file — the harness below exposes `db_url`).

Modify the harness: add `db_url: String` field to `TestApp` (set from `config.database_url` before moving `config`). Concretely, in `spawn_app`:

```rust
    let config = Config {
        // ... unchanged ...
    };
    let db_url = config.database_url.clone();
    let pool = db::create_pool(&db_url).await.unwrap();
    // ... unchanged ...
    TestApp { base_url: format!("http://{addr}"), client, db_url, _dir: dir }
```

and add `db_url: String` to the `TestApp` struct definition.

- [ ] **Step 1: Write the failing test**

Append to `server/tests/api.rs` (and add `db_url` to `TestApp`/`spawn_app`):

```rust
async fn seed(pool_url: &str, title: &str, markdown: &str, publish: bool) -> zblog_server::domain::article::Article {
    let pool = db::create_pool(pool_url).await.unwrap();
    let repo = zblog_server::infrastructure::article_repo::ArticleRepo::new(pool);
    let article = repo.create(title, markdown).await.unwrap();
    if publish {
        repo.set_status(article.id, zblog_server::domain::article::ArticleStatus::Published)
            .await
            .unwrap()
    } else {
        article
    }
}

#[tokio::test]
async fn public_articles_list_and_detail() {
    let app = spawn_app().await;
    seed(&app.db_url, "Alpha Notes", "# alpha body", true).await;
    seed(&app.db_url, "Beta Draft", "# beta body", false).await;

    let res = app.client.get(format!("{}/api/articles", app.base_url)).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(list.len(), 1); // negative: draft excluded
    assert_eq!(list[0]["title"], "Alpha Notes");
    assert_eq!(list[0]["status"], "published");
    assert!(list[0]["published_at"].is_string());

    let res = app
        .client
        .get(format!("{}/api/articles/alpha-notes", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let detail: serde_json::Value = res.json().await.unwrap();
    assert_eq!(detail["markdown"], "# alpha body");

    // negative: draft slug and unknown slug are both 404 for Visitors
    let res = app
        .client
        .get(format!("{}/api/articles/beta-draft", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = app
        .client
        .get(format!("{}/api/articles/no-such-slug", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path server/Cargo.toml`
Expected: FAIL — routes return 404 (handler missing).

- [ ] **Step 3: Write minimal implementation**

`server/src/interfaces/http/articles.rs`:

```rust
//! Public article endpoints (Visitor-facing).

use axum::{extract::{Path, State}, Json};

use crate::domain::article::{Article, ArticleStatus};
use crate::error::{AppError, Result};
use crate::interfaces::http::AppState;

/// `GET /api/articles` — published articles, newest first.
pub async fn list_published(State(state): State<AppState>) -> Result<Json<Vec<Article>>> {
    Ok(Json(state.articles.list_published().await?))
}

/// `GET /api/articles/{slug}` — one published article; drafts are invisible.
pub async fn get_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Article>> {
    match state.articles.find_by_slug(&slug).await? {
        Some(article) if article.status == ArticleStatus::Published => Ok(Json(article)),
        _ => Err(AppError::NotFound),
    }
}
```

`server/src/interfaces/http/mod.rs` — add `pub mod articles;` and extend `build_router`:

```rust
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/articles", get(articles::list_published))
        .route("/api/articles/{slug}", get(articles::get_by_slug))
        .with_state(state)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path server/Cargo.toml && cargo clippy --manifest-path server/Cargo.toml --all-targets`
Expected: PASS, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add server/
git commit -m "feat(server): 添加公开文章查询接口"
```

---

### Task 7: Auth (login/logout/me) and admin articles API

**Files:**
- Create: `server/src/interfaces/http/middleware.rs`
- Create: `server/src/interfaces/http/admin.rs`
- Modify: `server/src/interfaces/http/mod.rs`
- Test: `server/tests/api.rs` (append)

**Interfaces:**
- Consumes: DTOs, `AppState`, repos.
- Produces:
  - Cookie `zblog_auth` (private/signed via `PrivateCookieJar`, httpOnly, path `/`, 7-day max age).
  - `POST /api/admin/login {password}` → 200 `{"ok":true}` + Set-Cookie; 401 on wrong password. **Public route.**
  - Protected (401 without valid cookie): `POST /api/admin/logout`, `GET /api/admin/me` → `{"ok":true}`, `GET|POST /api/admin/articles`, `GET|PUT|DELETE /api/admin/articles/{id}`, `POST /api/admin/articles/{id}/publish`, `POST /api/admin/articles/{id}/unpublish`, `GET /api/admin/articles/by-slug/{slug}` (any status — backs the Draft Preview route).
  - Create returns 201 + Article; title must be non-empty (400).

- [ ] **Step 1: Write the failing test**

Append to `server/tests/api.rs`:

```rust
async fn login(app: &TestApp) {
    let res = app
        .client
        .post(format!("{}/api/admin/login", app.base_url))
        .json(&serde_json::json!({ "password": "test-password" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn auth_flow() {
    let app = spawn_app().await;

    // negative: admin endpoints reject anonymous callers
    let res = app.client.get(format!("{}/api/admin/articles", app.base_url)).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // negative: wrong password
    let res = app
        .client
        .post(format!("{}/api/admin/login", app.base_url))
        .json(&serde_json::json!({ "password": "wrong" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    login(&app).await;
    let res = app.client.get(format!("{}/api/admin/me", app.base_url)).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app.client.post(format!("{}/api/admin/logout", app.base_url)).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = app.client.get(format!("{}/api/admin/me", app.base_url)).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_article_crud_and_preview_lookup() {
    let app = spawn_app().await;
    login(&app).await;

    // create (empty title rejected first — negative validation)
    let res = app
        .client
        .post(format!("{}/api/admin/articles", app.base_url))
        .json(&serde_json::json!({ "title": "  ", "markdown": "x" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let res = app
        .client
        .post(format!("{}/api/admin/articles", app.base_url))
        .json(&serde_json::json!({ "title": "Draft One", "markdown": "# draft body" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let created: serde_json::Value = res.json().await.unwrap();
    let id = created["id"].as_i64().unwrap();
    assert_eq!(created["status"], "draft");
    assert_eq!(created["slug"], "draft-one");

    // draft visible through admin by-slug (Draft Preview), not publicly
    let res = app
        .client
        .get(format!("{}/api/admin/articles/by-slug/draft-one", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = app
        .client
        .get(format!("{}/api/articles/draft-one", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // update
    let res = app
        .client
        .put(format!("{}/api/admin/articles/{id}", app.base_url))
        .json(&serde_json::json!({ "title": "Draft One v2", "slug": "draft-one", "markdown": "# v2" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let updated: serde_json::Value = res.json().await.unwrap();
    assert_eq!(updated["title"], "Draft One v2");
    assert_eq!(updated["markdown"], "# v2");

    // publish → public; unpublish → hidden again
    let res = app
        .client
        .post(format!("{}/api/admin/articles/{id}/publish", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let published: serde_json::Value = res.json().await.unwrap();
    assert_eq!(published["status"], "published");
    assert!(published["published_at"].is_string());
    let res = app.client.get(format!("{}/api/articles", app.base_url)).send().await.unwrap();
    let list: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(list.len(), 1);

    let res = app
        .client
        .post(format!("{}/api/admin/articles/{id}/unpublish", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = app.client.get(format!("{}/api/articles", app.base_url)).send().await.unwrap();
    let list: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(list.len(), 0);

    // admin list still contains the draft
    let res = app.client.get(format!("{}/api/admin/articles", app.base_url)).send().await.unwrap();
    let all: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(all.len(), 1);

    // delete
    let res = app
        .client
        .delete(format!("{}/api/admin/articles/{id}", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let res = app
        .client
        .get(format!("{}/api/admin/articles/{id}", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path server/Cargo.toml`
Expected: FAIL — routes missing.

- [ ] **Step 3: Write minimal implementation**

`server/src/interfaces/http/middleware.rs`:

```rust
//! Authentication middleware.

use axum::{extract::{Request, State}, middleware::Next, response::Response};
use axum_extra::extract::cookie::PrivateCookieJar;

use crate::error::{AppError, Result};
use crate::interfaces::http::AppState;

/// Name of the session cookie.
pub const AUTH_COOKIE: &str = "zblog_auth";

/// Value stored in the (signed, unforgeable) session cookie.
const AUTH_VALUE: &str = "authenticated";

/// Reject requests without a valid session cookie.
///
/// # Errors
/// Returns `AppError::Unauthorized` when the cookie is missing or wrong.
pub async fn require_auth(
    State(_state): State<AppState>,
    jar: PrivateCookieJar,
    request: Request,
    next: Next,
) -> Result<Response> {
    if jar.get(AUTH_COOKIE).is_some_and(|c| c.value() == AUTH_VALUE) {
        Ok(next.run(request).await)
    } else {
        Err(AppError::Unauthorized)
    }
}
```

`server/src/interfaces/http/admin.rs`:

```rust
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
pub async fn login(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Json(body): Json<LoginRequest>,
) -> Result<(PrivateCookieJar, Json<serde_json::Value>)> {
    let parsed = PasswordHash::new(&state.config.password_hash)
        .map_err(|e| AppError::Internal(format!("invalid ZBLOG_PASSWORD_HASH: {e}")))?;
    let ok = Argon2::default().verify_password(body.password.as_bytes(), &parsed).is_ok();
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
pub async fn list_articles(State(state): State<AppState>) -> Result<Json<Vec<Article>>> {
    Ok(Json(state.articles.list_all().await?))
}

/// `POST /api/admin/articles` — create a draft.
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
pub async fn get_article(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Article>> {
    state.articles.find_by_id(id).await?.map_or(Err(AppError::NotFound), |a| Ok(Json(a)))
}

/// `GET /api/admin/articles/by-slug/{slug}` — draft preview lookup.
pub async fn get_article_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Article>> {
    state.articles.find_by_slug(&slug).await?.map_or(Err(AppError::NotFound), |a| Ok(Json(a)))
}

/// `PUT /api/admin/articles/{id}` — update title/slug/markdown.
pub async fn update_article(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateArticleRequest>,
) -> Result<Json<Article>> {
    if body.title.trim().is_empty() {
        return Err(AppError::BadRequest("title must not be empty".to_owned()));
    }
    Ok(Json(state.articles.update(id, &body.title, &body.slug, &body.markdown).await?))
}

/// `POST /api/admin/articles/{id}/publish`.
pub async fn publish_article(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Article>> {
    Ok(Json(state.articles.set_status(id, ArticleStatus::Published).await?))
}

/// `POST /api/admin/articles/{id}/unpublish`.
pub async fn unpublish_article(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Article>> {
    Ok(Json(state.articles.set_status(id, ArticleStatus::Draft).await?))
}

/// `DELETE /api/admin/articles/{id}`.
pub async fn delete_article(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode> {
    match state.articles.delete(id).await? {
        0 => Err(AppError::NotFound),
        _ => Ok(StatusCode::NO_CONTENT),
    }
}
```

`server/src/interfaces/http/mod.rs` — add `pub mod admin;`, `pub mod middleware;`, and rebuild `build_router`:

```rust
use axum::{middleware, routing::{get, post}, Json, Router};

/// Build the full HTTP router.
pub fn build_router(state: AppState) -> Router {
    let public = Router::new()
        .route("/api/health", get(health))
        .route("/api/articles", get(articles::list_published))
        .route("/api/articles/{slug}", get(articles::get_by_slug))
        .route("/api/admin/login", post(admin::login));
    let admin = Router::new()
        .route("/api/admin/logout", post(admin::logout))
        .route("/api/admin/me", get(admin::me))
        .route("/api/admin/articles", get(admin::list_articles).post(admin::create_article))
        .route(
            "/api/admin/articles/{id}",
            get(admin::get_article).put(admin::update_article).delete(admin::delete_article),
        )
        .route("/api/admin/articles/{id}/publish", post(admin::publish_article))
        .route("/api/admin/articles/{id}/unpublish", post(admin::unpublish_article))
        .route("/api/admin/articles/by-slug/{slug}", get(admin::get_article_by_slug))
        .layer(middleware::from_fn_with_state(state.clone(), middleware::require_auth));
    Router::new().merge(public).merge(admin).with_state(state)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path server/Cargo.toml && cargo clippy --manifest-path server/Cargo.toml --all-targets`
Expected: PASS, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add server/
git commit -m "feat(server): 添加管理员认证与文章管理接口"
```

---

### Task 8: Pageview domain, stats structs, and PageviewRepo

**Files:**
- Create: `server/src/domain/pageview.rs`
- Create: `server/src/domain/stats.rs`
- Modify: `server/src/domain/mod.rs`
- Modify: `server/src/infrastructure/pageview_repo.rs` (replace stub)

**Interfaces:**
- Produces:
  - `domain::pageview::NewPageview { path: String, article_id: Option<i64>, ip: String, user_agent: String, referer: String }`
  - `domain::stats::{StatsOverview { total_pv, total_uv, today_pv, today_uv }, DailyStat { date, pv, uv }, ArticleStat { article_id, title, slug, pv }, IpStat { ip, count, last_seen }}` — all `i64`/`String`, `Serialize + FromRow`.
  - `PageviewRepo::record(&NewPageview) -> Result<()>`; `daily(days: i64) -> Result<Vec<DailyStat>>` (desc by date, window `datetime('now', '-{days} days')`); `by_article() -> Result<Vec<ArticleStat>>` (only articles with ≥1 view, pv desc); `top_ips(limit: i64) -> Result<Vec<IpStat>>` (count desc, `last_seen` = max created_at); `overview() -> Result<StatsOverview>` ("today" = `date('now')`, UTC).

- [ ] **Step 1: Write the failing test**

Replace the stub `server/src/infrastructure/pageview_repo.rs` content with only imports + this test module (so it fails):

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::pageview::NewPageview;
    use crate::infrastructure::article_repo::ArticleRepo;

    async fn setup() -> (ArticleRepo, PageviewRepo, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite:{}", dir.path().join("t.db").display());
        let pool = crate::infrastructure::db::create_pool(&url).await.unwrap();
        (ArticleRepo::new(pool.clone()), PageviewRepo::new(pool), dir)
    }

    fn pv(path: &str, article_id: Option<i64>, ip: &str) -> NewPageview {
        NewPageview {
            path: path.to_owned(),
            article_id,
            ip: ip.to_owned(),
            user_agent: "test-agent".to_owned(),
            referer: String::new(),
        }
    }

    #[tokio::test]
    async fn stats_are_exact() {
        let (articles, repo, _dir) = setup().await;
        let a = articles.create("Stats Post", "s").await.unwrap();

        repo.record(&pv("/posts/stats-post", Some(a.id), "1.1.1.1")).await.unwrap();
        repo.record(&pv("/posts/stats-post", Some(a.id), "1.1.1.1")).await.unwrap();
        repo.record(&pv("/", None, "2.2.2.2")).await.unwrap();

        let overview = repo.overview().await.unwrap();
        assert_eq!(overview.total_pv, 3);
        assert_eq!(overview.total_uv, 2);
        assert_eq!(overview.today_pv, 3);
        assert_eq!(overview.today_uv, 2);

        let daily = repo.daily(30).await.unwrap();
        assert_eq!(daily.len(), 1);
        assert_eq!(daily[0].pv, 3);
        assert_eq!(daily[0].uv, 2);

        let by_article = repo.by_article().await.unwrap();
        assert_eq!(by_article.len(), 1);
        assert_eq!(by_article[0].article_id, a.id);
        assert_eq!(by_article[0].title, "Stats Post");
        assert_eq!(by_article[0].pv, 2); // negative: homepage view not counted here

        let ips = repo.top_ips(20).await.unwrap();
        assert_eq!(ips.len(), 2);
        assert_eq!(ips[0].ip, "1.1.1.1");
        assert_eq!(ips[0].count, 2);
        assert_eq!(ips[1].ip, "2.2.2.2");
        assert_eq!(ips[1].count, 1);
        assert!(!ips[0].last_seen.is_empty());

        // negative: empty database yields zeros, not NULLs
        let (_a2, repo2, _dir2) = setup().await;
        let empty = repo2.overview().await.unwrap();
        assert_eq!(empty.total_pv, 0);
        assert_eq!(empty.total_uv, 0);
    }
}
```

(`ArticleRepo` is imported inside the test module — not at file top level — so the non-test build has no unused-import warning under `warnings = "deny"`.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path server/Cargo.toml`
Expected: FAIL — `NewPageview`, methods undefined.

- [ ] **Step 3: Write minimal implementation**

`server/src/domain/pageview.rs`:

```rust
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
```

`server/src/domain/stats.rs`:

```rust
//! Traffic statistics shapes returned by the stats API.

/// Whole-site counters.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct StatsOverview {
    /// Total pageviews.
    pub total_pv: i64,
    /// Total unique visitor IPs.
    pub total_uv: i64,
    /// Pageviews today (UTC).
    pub today_pv: i64,
    /// Unique visitor IPs today (UTC).
    pub today_uv: i64,
}

/// One row of per-day traffic.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct DailyStat {
    /// Day, `YYYY-MM-DD`.
    pub date: String,
    /// Pageviews that day.
    pub pv: i64,
    /// Unique visitor IPs that day.
    pub uv: i64,
}

/// Per-article view counts.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ArticleStat {
    /// Article id.
    pub article_id: i64,
    /// Article title.
    pub title: String,
    /// Article slug.
    pub slug: String,
    /// Pageviews of the article page.
    pub pv: i64,
}

/// Per-IP view counts.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct IpStat {
    /// Visitor IP (clear text by design).
    pub ip: String,
    /// Total requests from this IP.
    pub count: i64,
    /// Most recent visit timestamp.
    pub last_seen: String,
}
```

`server/src/domain/mod.rs` — add `pub mod pageview;` and `pub mod stats;`.

`server/src/infrastructure/pageview_repo.rs` (full replacement of stub, above the tests):

```rust
//! SQLite persistence and aggregation for pageviews.

use sqlx::SqlitePool;

use crate::domain::pageview::NewPageview;
use crate::domain::stats::{ArticleStat, DailyStat, IpStat, StatsOverview};
use crate::error::Result;

/// Pageview repository.
#[derive(Clone)]
pub struct PageviewRepo {
    pool: SqlitePool,
}

impl PageviewRepo {
    /// Create the repository.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Record one pageview.
    ///
    /// # Errors
    /// Returns `AppError::Db` on database failure.
    pub async fn record(&self, pageview: &NewPageview) -> Result<()> {
        sqlx::query(
            "INSERT INTO pageviews (path, article_id, ip, user_agent, referer) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&pageview.path)
        .bind(pageview.article_id)
        .bind(&pageview.ip)
        .bind(&pageview.user_agent)
        .bind(&pageview.referer)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Whole-site counters (today in UTC).
    ///
    /// # Errors
    /// Returns `AppError::Db` on database failure.
    pub async fn overview(&self) -> Result<StatsOverview> {
        let row = sqlx::query_as::<_, StatsOverview>(
            "SELECT COUNT(*) AS total_pv, COUNT(DISTINCT ip) AS total_uv, \
             COALESCE(SUM(CASE WHEN date(created_at) = date('now') THEN 1 ELSE 0 END), 0) AS today_pv, \
             COUNT(DISTINCT CASE WHEN date(created_at) = date('now') THEN ip END) AS today_uv \
             FROM pageviews",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Per-day traffic for the last `days` days, newest day first.
    ///
    /// # Errors
    /// Returns `AppError::Db` on database failure.
    pub async fn daily(&self, days: i64) -> Result<Vec<DailyStat>> {
        let rows = sqlx::query_as::<_, DailyStat>(
            "SELECT date(created_at) AS date, COUNT(*) AS pv, COUNT(DISTINCT ip) AS uv \
             FROM pageviews WHERE created_at >= datetime('now', ?) \
             GROUP BY date(created_at) ORDER BY date DESC",
        )
        .bind(format!("-{days} days"))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Per-article view counts, most viewed first; articles without views are omitted.
    ///
    /// # Errors
    /// Returns `AppError::Db` on database failure.
    pub async fn by_article(&self) -> Result<Vec<ArticleStat>> {
        let rows = sqlx::query_as::<_, ArticleStat>(
            "SELECT a.id AS article_id, a.title, a.slug, COUNT(p.id) AS pv \
             FROM articles a JOIN pageviews p ON p.article_id = a.id \
             GROUP BY a.id ORDER BY pv DESC, a.id ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Top visitor IPs by request count.
    ///
    /// # Errors
    /// Returns `AppError::Db` on database failure.
    pub async fn top_ips(&self, limit: i64) -> Result<Vec<IpStat>> {
        let rows = sqlx::query_as::<_, IpStat>(
            "SELECT ip, COUNT(*) AS count, MAX(created_at) AS last_seen \
             FROM pageviews GROUP BY ip ORDER BY count DESC, ip ASC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path server/Cargo.toml && cargo clippy --manifest-path server/Cargo.toml --all-targets`
Expected: PASS, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add server/
git commit -m "feat(server): 添加访问统计领域模型与聚合查询"
```

---

### Task 9: Pageview ingestion + stats API, hash-password example

**Files:**
- Create: `server/src/interfaces/http/pageviews.rs`
- Create: `server/src/interfaces/http/stats.rs`
- Create: `server/examples/hash_password.rs`
- Modify: `server/src/interfaces/http/mod.rs`
- Test: `server/tests/api.rs` (append)

**Interfaces:**
- Consumes: DTOs, repos.
- Produces:
  - `POST /api/pageviews {path, article_id?, ip, user_agent?, referer?}` → 201; 400 on empty path or ip. **Public route** (trusted because Astro is the only caller that can reach the API — ADR-0003).
  - Protected: `GET /api/admin/stats/overview`, `GET /api/admin/stats/daily?days=30`, `GET /api/admin/stats/articles`, `GET /api/admin/stats/ips?limit=20`.
  - `cargo run --example hash_password -- '<password>'` prints an Argon2 PHC string for `ZBLOG_PASSWORD_HASH`.

- [ ] **Step 1: Write the failing test**

Append to `server/tests/api.rs`:

```rust
#[tokio::test]
async fn pageview_ingestion_and_stats() {
    let app = spawn_app().await;
    let article = seed(&app.db_url, "Watched Post", "w", true).await;

    // negative: validation
    let res = app
        .client
        .post(format!("{}/api/pageviews", app.base_url))
        .json(&serde_json::json!({ "path": "", "ip": "1.1.1.1" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    for ip in ["1.1.1.1", "1.1.1.1", "2.2.2.2"] {
        let res = app
            .client
            .post(format!("{}/api/pageviews", app.base_url))
            .json(&serde_json::json!({
                "path": "/posts/watched-post",
                "article_id": article.id,
                "ip": ip,
                "user_agent": "curl"
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    // negative: stats are admin-only
    let res = app
        .client
        .get(format!("{}/api/admin/stats/overview", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    login(&app).await;
    let res = app
        .client
        .get(format!("{}/api/admin/stats/overview", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let overview: serde_json::Value = res.json().await.unwrap();
    assert_eq!(overview["total_pv"], 3);
    assert_eq!(overview["total_uv"], 2);
    assert_eq!(overview["today_pv"], 3);

    let res = app
        .client
        .get(format!("{}/api/admin/stats/daily?days=7", app.base_url))
        .send()
        .await
        .unwrap();
    let daily: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(daily.len(), 1);
    assert_eq!(daily[0]["pv"], 3);

    let res = app
        .client
        .get(format!("{}/api/admin/stats/articles", app.base_url))
        .send()
        .await
        .unwrap();
    let by_article: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(by_article.len(), 1);
    assert_eq!(by_article[0]["title"], "Watched Post");
    assert_eq!(by_article[0]["pv"], 3);

    let res = app
        .client
        .get(format!("{}/api/admin/stats/ips?limit=1", app.base_url))
        .send()
        .await
        .unwrap();
    let ips: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(ips.len(), 1);
    assert_eq!(ips[0]["ip"], "1.1.1.1");
    assert_eq!(ips[0]["count"], 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path server/Cargo.toml`
Expected: FAIL — routes missing.

- [ ] **Step 3: Write minimal implementation**

`server/src/interfaces/http/pageviews.rs`:

```rust
//! Pageview ingestion endpoint (called by the Astro SSR layer only).

use axum::{extract::State, http::StatusCode, Json};

use crate::domain::pageview::NewPageview;
use crate::error::{AppError, Result};
use crate::interfaces::http::dto::RecordPageviewRequest;
use crate::interfaces::http::AppState;

/// `POST /api/pageviews` — record one pageview.
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
```

`server/src/interfaces/http/stats.rs`:

```rust
//! Admin traffic statistics endpoints.

use axum::{extract::{Query, State}, Json};

use crate::domain::stats::{ArticleStat, DailyStat, IpStat, StatsOverview};
use crate::error::Result;
use crate::interfaces::http::dto::{DailyQuery, LimitQuery};
use crate::interfaces::http::AppState;

/// `GET /api/admin/stats/overview`.
pub async fn overview(State(state): State<AppState>) -> Result<Json<StatsOverview>> {
    Ok(Json(state.pageviews.overview().await?))
}

/// `GET /api/admin/stats/daily?days=30`.
pub async fn daily(
    State(state): State<AppState>,
    Query(query): Query<DailyQuery>,
) -> Result<Json<Vec<DailyStat>>> {
    Ok(Json(state.pageviews.daily(query.days.unwrap_or(30)).await?))
}

/// `GET /api/admin/stats/articles`.
pub async fn articles(State(state): State<AppState>) -> Result<Json<Vec<ArticleStat>>> {
    Ok(Json(state.pageviews.by_article().await?))
}

/// `GET /api/admin/stats/ips?limit=20`.
pub async fn ips(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<IpStat>>> {
    Ok(Json(state.pageviews.top_ips(query.limit.unwrap_or(20)).await?))
}
```

`server/src/interfaces/http/mod.rs` — add `pub mod pageviews;`, `pub mod stats;`; in `build_router` add `.route("/api/pageviews", post(pageviews::record))` to `public` and these to `admin`:

```rust
        .route("/api/admin/stats/overview", get(stats::overview))
        .route("/api/admin/stats/daily", get(stats::daily))
        .route("/api/admin/stats/articles", get(stats::articles))
        .route("/api/admin/stats/ips", get(stats::ips))
```

`server/examples/hash_password.rs`:

```rust
//! Print an Argon2 PHC hash for `ZBLOG_PASSWORD_HASH`.
//!
//! Usage: `cargo run --example hash_password -- 'your-password'`

use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};

fn main() {
    let Some(password) = std::env::args().nth(1) else {
        eprintln!("usage: hash_password <password>");
        std::process::exit(2);
    };
    let salt = SaltString::generate(&mut OsRng);
    match Argon2::default().hash_password(password.as_bytes(), &salt) {
        Ok(hash) => println!("{hash}"),
        Err(e) => {
            eprintln!("failed to hash password: {e}");
            std::process::exit(1);
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path server/Cargo.toml && cargo clippy --manifest-path server/Cargo.toml --all-targets && cargo run --manifest-path server/Cargo.toml --example hash_password -- 'x' | head -c 20`
Expected: tests PASS, clippy clean, example prints `$argon2id$v=19$m=…`.

- [ ] **Step 5: Commit**

```bash
git add server/
git commit -m "feat(server): 添加访问上报与统计查询接口"
```

---

### Task 10: Backend smoke run

**Files:** none (verification only)

- [ ] **Step 1: Start the server with a real config**

```bash
cd server
HASH=$(cargo run --example hash_password -- 'dev-password' | tail -n1)
ZBLOG_PASSWORD_HASH="$HASH" ZBLOG_SESSION_SECRET="$(openssl rand -hex 32)$(openssl rand -hex 32)" \
  cargo run &
sleep 3
```

- [ ] **Step 2: Exercise the API end to end**

```bash
curl -s localhost:8080/api/health
curl -s -c /tmp/zblog.jar -X POST localhost:8080/api/admin/login \
  -H 'content-type: application/json' -d '{"password":"dev-password"}'
curl -s -b /tmp/zblog.jar -X POST localhost:8080/api/admin/articles \
  -H 'content-type: application/json' -d '{"title":"Smoke Post","markdown":"# hi"}'
curl -s -b /tmp/zblog.jar -X POST localhost:8080/api/admin/articles/1/publish
curl -s localhost:8080/api/articles
curl -s -X POST localhost:8080/api/pageviews -H 'content-type: application/json' \
  -d '{"path":"/posts/smoke-post","article_id":1,"ip":"9.9.9.9"}'
curl -s -b /tmp/zblog.jar localhost:8080/api/admin/stats/overview
```

Expected: health `ok`; login `ok`; article created as draft then published; public list contains `Smoke Post`; overview shows `total_pv: 1, total_uv: 1`.

- [ ] **Step 3: Stop server and commit**

```bash
kill %1
rm -f server/zblog.db /tmp/zblog.jar
git commit --allow-empty -m "test(server): 后端冒烟验证通过"
```

(If anything fails, fix and re-run before committing.)
