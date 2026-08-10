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
    db_url: String,
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
    let db_url = config.database_url.clone();
    let pool = db::create_pool(&db_url).await.unwrap();
    let state = AppState::new(pool, config);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            build_router(state).into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });
    let client = Client::builder().cookie_store(true).build().unwrap();
    TestApp {
        base_url: format!("http://{addr}"),
        client,
        db_url,
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

async fn seed(
    pool_url: &str,
    title: &str,
    markdown: &str,
    publish: bool,
) -> zblog_server::domain::article::Article {
    let pool = db::create_pool(pool_url).await.unwrap();
    let repo = zblog_server::infrastructure::article_repo::ArticleRepo::new(pool);
    let article = repo.create(title, markdown).await.unwrap();
    if publish {
        repo.set_status(
            article.id,
            zblog_server::domain::article::ArticleStatus::Published,
        )
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

    let res = app
        .client
        .get(format!("{}/api/articles", app.base_url))
        .send()
        .await
        .unwrap();
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

async fn get(app: &TestApp, path: &str) -> reqwest::Response {
    app.client
        .get(format!("{}{path}", app.base_url))
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn auth_flow() {
    let app = spawn_app().await;

    // negative: admin endpoints reject anonymous callers
    let res = app
        .client
        .get(format!("{}/api/admin/articles", app.base_url))
        .send()
        .await
        .unwrap();
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
    let res = app
        .client
        .get(format!("{}/api/admin/me", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .client
        .post(format!("{}/api/admin/logout", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = app
        .client
        .get(format!("{}/api/admin/me", app.base_url))
        .send()
        .await
        .unwrap();
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
    let res = get(&app, "/api/admin/articles/by-slug/draft-one").await;
    assert_eq!(res.status(), StatusCode::OK);
    let res = get(&app, "/api/articles/draft-one").await;
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
    let res = get(&app, "/api/articles").await;
    let list: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(list.len(), 1);

    let res = app
        .client
        .post(format!(
            "{}/api/admin/articles/{id}/unpublish",
            app.base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = get(&app, "/api/articles").await;
    let list: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(list.len(), 0);

    // admin list still contains the draft
    let res = get(&app, "/api/admin/articles").await;
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

#[tokio::test]
async fn pageview_ingestion_and_stats() {
    let app = spawn_app().await;
    let article = seed(&app.db_url, "Watched Post", "w", true).await;

    // negative: validation
    let res = app
        .client
        .post(format!("{}/api/pageviews", app.base_url))
        .json(&serde_json::json!({ "path": "" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    for _ in 0..3 {
        let res = app
            .client
            .post(format!("{}/api/pageviews", app.base_url))
            .header("user-agent", "curl-test")
            .json(&serde_json::json!({
                "path": "/posts/watched-post",
                "article_id": article.id
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
    assert_eq!(overview["total_uv"], 1); // all from the loopback peer
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
        .get(format!("{}/api/admin/stats/ips?limit=5", app.base_url))
        .send()
        .await
        .unwrap();
    let ips: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(ips.len(), 1);
    assert_eq!(ips[0]["ip"], "127.0.0.1");
    assert_eq!(ips[0]["count"], 3);
}

#[tokio::test]
async fn static_site_and_api_404_shapes() {
    let app = spawn_app().await;

    // home page shell (placeholder or real build) is served as HTML
    let res = app.client.get(format!("{}/", app.base_url)).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let content_type = res.headers()["content-type"].to_str().unwrap().to_owned();
    assert!(content_type.contains("text/html"));

    // unknown API path → JSON 404, not HTML
    let res = app
        .client
        .get(format!("{}/api/definitely-not-a-route", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"], "not found");

    // unknown page path → 404 status
    let res = app
        .client
        .get(format!("{}/definitely/not/a/page", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
