#![allow(clippy::unwrap_used)]

use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
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
    TestApp {
        base_url: format!("http://{addr}"),
        client,
        _dir: dir,
    }
}

#[tokio::test]
async fn health_returns_ok() {
    let app = spawn_app().await;
    let res = app
        .client
        .get(format!("{}/api/health", app.base_url))
        .send()
        .await
        .unwrap();
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
