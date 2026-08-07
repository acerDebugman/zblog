//! `SQLite` connection pool and migrations.

use std::str::FromStr;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

use crate::error::Result;

/// Create a connection pool and apply all pending migrations.
///
/// # Errors
/// Returns `AppError::Db` when the database cannot be opened or migrations fail.
pub async fn create_pool(database_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(sqlx::Error::from)?;
    Ok(pool)
}
