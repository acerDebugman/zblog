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
