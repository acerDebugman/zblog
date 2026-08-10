//! Embedded static site serving with SPA fallback (see ADR-0004).

use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
};
use rust_embed::RustEmbed;

/// The built Astro site, embedded into the binary (release) or read from
/// disk (debug builds).
#[derive(RustEmbed)]
#[folder = "../web/dist"]
pub struct SiteAssets;

/// `GET /` — the home page shell.
pub async fn index() -> Response {
    serve_path_with::<SiteAssets>("")
}

/// Fallback handler: static asset, or SPA shell, or 404.
pub async fn fallback(uri: Uri) -> Response {
    serve_path_with::<SiteAssets>(uri.path())
}

/// Resolve a request path to an embedded response.
///
/// Order: exact file → `{path}/index.html` → longest ancestor `index.html`
/// → `404.html` with status 404. Unmatched `/api/*` paths return the JSON
/// `AppError::NotFound` body instead of HTML. The empty path (`/`) maps to
/// `index.html`. Paths containing `..` segments are rejected with the 404
/// response before any lookup.
#[must_use]
pub fn serve_path_with<T: RustEmbed>(path: &str) -> Response {
    let path = path.trim_matches('/');
    if path == "api" || path.starts_with("api/") {
        return crate::error::AppError::NotFound.into_response();
    }
    if path.split('/').any(|seg| seg == "..") {
        return not_found::<T>();
    }
    let path = if path.is_empty() { "index.html" } else { path };
    if let Some(response) = embedded::<T>(path) {
        return response;
    }
    if let Some(response) = embedded::<T>(&format!("{path}/index.html")) {
        return response;
    }
    let mut current = path;
    while let Some(pos) = current.rfind('/') {
        current = &current[..pos];
        if let Some(response) = embedded::<T>(&format!("{current}/index.html")) {
            return response;
        }
    }
    not_found::<T>()
}

fn embedded<T: RustEmbed>(path: &str) -> Option<Response> {
    let asset = T::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Response::builder()
        .header(header::CONTENT_TYPE, mime.as_ref())
        .body(Body::from(asset.data.into_owned()))
        .ok()
}

fn not_found<T: RustEmbed>() -> Response {
    match T::get("404.html") {
        Some(asset) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(asset.data.into_owned()))
            .unwrap_or_else(|_| StatusCode::NOT_FOUND.into_response()),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Router fragment for the static site (kept for `mod.rs` readability).
pub fn routes<S>(router: axum::Router<S>) -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.route("/", get(index)).fallback(fallback)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::serve_path_with;
    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "tests/fixtures/site"]
    struct TestAssets;

    async fn body_string(response: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), 1 << 20).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn serves_exact_asset_with_mime() {
        let response = serve_path_with::<TestAssets>("assets/app.js");
        assert_eq!(response.status(), 200);
        assert!(response.headers()["content-type"].to_str().unwrap().contains("javascript"));
        assert_eq!(body_string(response).await, "// app");
    }

    #[tokio::test]
    async fn directory_index_for_shell_routes() {
        assert_eq!(body_string(serve_path_with::<TestAssets>("about")).await, "<h1>about</h1>");
        assert_eq!(body_string(serve_path_with::<TestAssets>("")).await, "<h1>home</h1>");
    }

    #[tokio::test]
    async fn spa_falls_back_to_ancestor_index() {
        assert_eq!(
            body_string(serve_path_with::<TestAssets>("posts/hello-world")).await,
            "<h1>post shell</h1>"
        );
        assert_eq!(
            body_string(serve_path_with::<TestAssets>("admin/edit/3")).await,
            "<h1>edit shell</h1>"
        );
        assert_eq!(body_string(serve_path_with::<TestAssets>("admin")).await, "<h1>admin home</h1>");
    }

    #[tokio::test]
    async fn unknown_path_gets_404_page_with_404_status() {
        let response = serve_path_with::<TestAssets>("no-such-page");
        assert_eq!(response.status(), 404);
        assert_eq!(body_string(response).await, "<h1>not found</h1>");
    }

    #[tokio::test]
    async fn parent_directory_segments_get_404() {
        let response = serve_path_with::<TestAssets>("../../etc/passwd");
        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn unknown_api_path_gets_json_404() {
        let response = serve_path_with::<TestAssets>("api/nope");
        assert_eq!(response.status(), 404);
        assert_eq!(body_string(response).await, "{\"error\":\"not found\"}");
    }
}
