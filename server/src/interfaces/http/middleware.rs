//! Authentication middleware.

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::PrivateCookieJar;

use crate::error::{AppError, Result};
use crate::interfaces::http::AppState;

/// Name of the session cookie.
pub const AUTH_COOKIE: &str = "zblog_auth";

/// Value stored in the (signed, unforgeable) session cookie.
const AUTH_VALUE: &str = "authenticated";

/// Reject requests without a valid session cookie.
///
/// # Errors
/// Returns `AppError::Unauthorized` when the cookie is missing or wrong.
pub async fn require_auth(
    State(_state): State<AppState>,
    jar: PrivateCookieJar,
    request: Request,
    next: Next,
) -> Result<Response> {
    if jar
        .get(AUTH_COOKIE)
        .is_some_and(|c| c.value() == AUTH_VALUE)
    {
        Ok(next.run(request).await)
    } else {
        Err(AppError::Unauthorized)
    }
}
