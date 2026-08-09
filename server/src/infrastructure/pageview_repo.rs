//! `SQLite` persistence and aggregation for pageviews.

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
    pub const fn new(pool: SqlitePool) -> Self {
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

        repo.record(&pv("/posts/stats-post", Some(a.id), "1.1.1.1"))
            .await
            .unwrap();
        repo.record(&pv("/posts/stats-post", Some(a.id), "1.1.1.1"))
            .await
            .unwrap();
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
