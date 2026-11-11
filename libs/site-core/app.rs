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
pub(crate) const REQUEST_TIMEOUT_SECS: u64 = 60;

const FULL_CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'";

/// Adds the security headers to every response that passes through it.
///
/// `Content-Security-Policy` depends on where the response came from, never on
/// its `Content-Type`:
/// - responses carrying the crate-private `FromStaticService` marker (the
///   static fallback, 304s included) get the policy that marker carries: the
///   build-validated `csp-header.txt` (which repeats the page's meta policy),
///   or [`static_files::STATIC_CSP`] when that file is unusable;
/// - everything else (routes, origin_lock's own responses, avatars, any
///   routed 304) gets the full policy.
///
/// This is the ONLY writer of the header and it always overwrites: a CSP a
/// handler set itself is replaced, so a future handler cannot weaken it.
async fn security_headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    let policy = match response
        .extensions()
        .get::<static_files::FromStaticService>()
    {
        Some(static_files::FromStaticService(value)) => value.clone(),
        None => axum::http::HeaderValue::from_static(FULL_CSP),
    };
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_SECURITY_POLICY, policy);
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
/// fallback as well as every routed path, making it the outermost
/// access layer: nothing else in this router is reachable around it.
/// `security_headers` (#4710) is added after `origin_lock`, outside it,
/// for the same wrap-the-fallback reason plus one more: `origin_lock`
/// answers `GET /api/health` and its 404 rejections itself, and those
/// responses need the headers too. The layers above the fallback (page
/// hits, rate limit, body limit, timeout, CORS) do not wrap it.
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
        .fallback_service(static_files::static_file_service(static_dir))
        // task #3558 C1: added AFTER `.fallback_service(...)` so this
        // layer wraps the fallback as well as every routed path above.
        // It is the outermost ACCESS layer: nothing is reachable around it.
        .layer(axum::middleware::from_fn_with_state(
            db_state,
            origin_lock_middleware,
        ))
        .layer(axum::Extension(origin_lock_state))
        // #4710 F2: outermost of all, so even responses `origin_lock` writes
        // itself (the exempt `GET /api/health` and its 404 rejections) carry
        // the headers. It only decorates the response on the way out; it
        // grants no route access, so `origin_lock` still gates everything
        // inside it.
        .layer(axum::middleware::from_fn(security_headers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    const MARKED_POLICY: &str = "frame-ancestors 'none'; marked-test-policy";

    fn csp_values(response: &axum_test::TestResponse) -> Vec<String> {
        response
            .headers()
            .get_all("content-security-policy")
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .collect()
    }

    fn server() -> axum_test::TestServer {
        let app = Router::new()
            // A handler that tries to set its own (weak) CSP, twice over.
            .route(
                "/weak",
                get(|| async {
                    (
                        [
                            ("content-security-policy", "default-src *"),
                            ("content-security-policy", "script-src 'unsafe-inline'"),
                        ],
                        "x",
                    )
                        .into_response()
                }),
            )
            // A bare routed 304: no static marker.
            .route("/bare304", get(|| async { StatusCode::NOT_MODIFIED }))
            // A routed response that carries the static marker.
            .route(
                "/marked",
                get(|| async {
                    let mut r = "x".into_response();
                    r.extensions_mut().insert(static_files::FromStaticService(
                        MARKED_POLICY.parse().unwrap(),
                    ));
                    r.headers_mut()
                        .insert("content-security-policy", "default-src *".parse().unwrap());
                    r
                }),
            )
            .layer(axum::middleware::from_fn(security_headers));
        axum_test::TestServer::new(app)
    }

    #[tokio::test]
    async fn a_handler_set_csp_is_overwritten_with_exactly_one_full_policy() {
        let response = server().get("/weak").await;
        assert_eq!(csp_values(&response), [FULL_CSP]);
    }

    #[tokio::test]
    async fn a_bare_routed_304_gets_the_full_policy() {
        let response = server().get("/bare304").await;
        response.assert_status(StatusCode::NOT_MODIFIED);
        assert_eq!(csp_values(&response), [FULL_CSP]);
    }

    #[tokio::test]
    async fn the_static_marker_selects_its_own_policy_and_replaces_any_handler_csp() {
        let response = server().get("/marked").await;
        assert_eq!(csp_values(&response), [MARKED_POLICY]);
    }
}
