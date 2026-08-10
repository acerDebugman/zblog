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
    axum::serve(
        listener,
        build_router(state).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .map_err(|e| AppError::Internal(format!("server error: {e}")))?;
    Ok(())
}
