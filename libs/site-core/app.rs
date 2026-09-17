//! Builds the fully-assembled Axum [`Router`] — every route, every
//! middleware layer, and the static-file fallback, in the exact order the
//! running server uses.
//!
//! Pulled out of `cmd/server` (a binary-only crate, so its own tests
//! can't import from it) so both `main.rs` and this crate's integration
//! tests build the app through the identical path (task #3558 C2 /
//! Warden M2) — a test that builds its own router can drift from
//! production wiring with no test ever noticing.

use axum::extract::DefaultBodyLimit;
use axum::http::{StatusCode, header};
use axum::{Router, routing::get};
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;

use crate::middleware::global_rate_limit::{GlobalRateLimitState, global_rate_limit_middleware};
use crate::middleware::origin_lock::{OriginLockState, origin_lock_middleware};
use crate::middleware::page_hits::page_hits_middleware;
use crate::routes;
use crate::state::DbState;
use crate::static_files::{self, serve_avatar};

/// LLM-audit L2 / R5: explicit global request-body byte cap (64 KiB).
const GLOBAL_BODY_LIMIT_BYTES: usize = 64 * 1024;

/// LLM-audit M1 / R1: outer-backstop request timeout.
const REQUEST_TIMEOUT_SECS: u64 = 60;

async fn security_headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "content-security-policy",
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'"
            .parse().unwrap(),
    );
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    headers.insert(
        "referrer-policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );
    headers.insert(
        "permissions-policy",
        "camera=(), microphone=(), geolocation=()".parse().unwrap(),
    );
    headers.insert(
        "strict-transport-security",
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );
    response
}

/// Build the complete application `Router`.
///
/// Layer order matters and is deliberate (task #3558 C1): a `.layer()`
/// call only wraps whatever fallback service already exists at the
/// moment it's called, so `origin_lock` is added AFTER
/// `.fallback_service(...)` rather than alongside the other layers
/// above it — that ordering is what makes `origin_lock` wrap the
/// fallback as well as every routed path, making it the true outermost
/// layer: nothing else in this router is reachable around it.
///
/// `global_rate_limit` is a caller-supplied instance rather than
/// constructed here: the caller (`main.rs::run_server`) also owns
/// spawning its periodic `purge_stale` task, and that task must be tied
/// to the one instance actually wired into the router's `Extension`
/// layer, not a second one this function would otherwise create. Keeps
/// `build_app` itself free of background-task side effects, which
/// matters for the test callers that build a fresh router per test.
pub fn build_app(
    db_state: DbState,
    origin_lock_state: OriginLockState,
    global_rate_limit: GlobalRateLimitState,
    static_dir: &str,
    cors_origin: &str,
) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(
            cors_origin
                .parse::<axum::http::HeaderValue>()
                .expect("Invalid CORS_ORIGIN value"),
        )
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    Router::new()
        .route("/api/health", get(routes::health_check))
        .merge(routes::public_router())
        .merge(routes::ai::routes_with_connect_info())
        .merge(routes::admin::admin_router(db_state.clone()))
        .route("/api/avatars/{filename}", get(serve_avatar))
        .with_state(db_state.clone())
        .layer(axum::middleware::from_fn_with_state(
            db_state.clone(),
            page_hits_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            db_state.clone(),
            global_rate_limit_middleware,
        ))
        .layer(axum::Extension(global_rate_limit))
        .layer(DefaultBodyLimit::max(GLOBAL_BODY_LIMIT_BYTES))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS),
        ))
        .layer(cors)
        .layer(axum::middleware::from_fn(security_headers))
        .fallback_service(static_files::static_file_service(static_dir))
        // task #3558 C1: added AFTER `.fallback_service(...)` so this
        // layer — and only this layer — wraps the fallback as well as
        // every routed path above.
        .layer(axum::middleware::from_fn_with_state(
            db_state,
            origin_lock_middleware,
        ))
        .layer(axum::Extension(origin_lock_state))
}
