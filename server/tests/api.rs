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

#[tokio::test]
async fn migrations_create_tables() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite:{}", dir.path().join("t.db").display());
    let pool = zblog_server::infrastructure::db::create_pool(&url)
        .await
        .unwrap();
    let names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(names.contains(&"articles".to_owned()));
    assert!(names.contains(&"pageviews".to_owned()));
}
