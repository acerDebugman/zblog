//! `SQLite` persistence for pageviews (stub — completed in the pageview task).

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
