//! Admin traffic statistics endpoints.

use axum::{
    extract::{Query, State},
    Json,
};

use crate::domain::stats::{ArticleStat, DailyStat, IpStat, StatsOverview};
use crate::error::Result;
use crate::interfaces::http::dto::{DailyQuery, LimitQuery};
use crate::interfaces::http::AppState;

/// `GET /api/admin/stats/overview`.
///
/// # Errors
///
/// Returns `AppError::Db` when the aggregation query fails.
pub async fn overview(State(state): State<AppState>) -> Result<Json<StatsOverview>> {
    Ok(Json(state.pageviews.overview().await?))
}

/// `GET /api/admin/stats/daily?days=30`.
///
/// # Errors
///
/// Returns `AppError::Db` when the aggregation query fails.
pub async fn daily(
    State(state): State<AppState>,
    Query(query): Query<DailyQuery>,
) -> Result<Json<Vec<DailyStat>>> {
    Ok(Json(state.pageviews.daily(query.days.unwrap_or(30)).await?))
}

/// `GET /api/admin/stats/articles`.
///
/// # Errors
///
/// Returns `AppError::Db` when the aggregation query fails.
pub async fn articles(State(state): State<AppState>) -> Result<Json<Vec<ArticleStat>>> {
    Ok(Json(state.pageviews.by_article().await?))
}

/// `GET /api/admin/stats/ips?limit=20`.
///
/// # Errors
///
/// Returns `AppError::Db` when the aggregation query fails.
pub async fn ips(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<IpStat>>> {
    Ok(Json(
        state.pageviews.top_ips(query.limit.unwrap_or(20)).await?,
    ))
}
