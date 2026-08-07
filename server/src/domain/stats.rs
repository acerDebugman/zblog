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
