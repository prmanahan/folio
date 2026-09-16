//! Origin lock (task #3558): reject any request that doesn't carry the
//! Cloudflare-injected shared secret, before it reaches rate limiting,
//! page-hit tracking, or the static-file fallback.
//!
//! This is the OUTERMOST layer of the whole service (see `app::build_app`
//! — it is layered after `.fallback_service(...)`, which is what makes it
//! wrap the fallback too, not just the routed paths). Nothing else in the
//! service is reachable around it.
//!
//! `GET /api/health` is the one exemption: exact path, exact method,
//! answered here directly via `routes::health_check_body` without ever
//! calling `next` — so it can't reach the rate limiter or page-hit
//! tracking regardless of whether a secret is presented. Fly's own health
//! checker (`fly.toml`) hits this path with no Cloudflare headers and no
//! secret, so the exemption is load-bearing for availability, not just a
//! convenience.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::http::{Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};

use crate::middleware::client_ip_log::log_client_ip_fields;
use crate::routes::health_check_body;
use crate::state::DbState;

const EDGE_AUTH_HEADER: &str = "x-folio-edge-auth";

/// Holds only the SHA-256 digest of the shared secret — never the
/// plaintext. A memory dump or an accidental `Debug` derive on this type
/// can't recover the secret it guards.
#[derive(Clone, Copy)]
pub struct OriginLockState {
    expected_digest: [u8; 32],
}

impl OriginLockState {
    /// `token` is the plaintext secret read from `EDGE_AUTH_TOKEN` at
    /// startup (see `config::Config`, which validates its shape). Hashed
    /// once here; the caller is free to drop the plaintext afterwards.
    pub fn new(token: &str) -> Self {
        let expected_digest: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        Self { expected_digest }
    }

    fn matches(&self, presented: &str) -> bool {
        let presented_digest: [u8; 32] = Sha256::digest(presented.as_bytes()).into();
        constant_time_eq(&self.expected_digest, &presented_digest)
    }
}

/// Constant-time equality over two fixed-length 32-byte digests.
///
/// Hand-rolled XOR-accumulate rather than `subtle` (a new *direct*
/// dependency — `subtle` is transitive-only in this workspace today, and
/// adding it directly needs the standing tool-addition proposal this role
/// operates under) and rather than a bare `==` (no language guarantee
/// against a short-circuiting comparison). Every call touches all 32
/// bytes with no branch on their content, so the number of bytes examined
/// never depends on where the two digests first differ.
fn constant_time_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff: u8 = 0;
    for i in 0..32 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

fn is_health_check(method: &Method, uri: &Uri) -> bool {
    method == Method::GET && uri.path() == "/api/health"
}

/// Reject any request that doesn't carry exactly one correct
/// `X-Folio-Edge-Auth` header. Same 404 for a missing, wrong, non-UTF-8,
/// or repeated header — only the server-side log line (never the
/// presented value) distinguishes the reason.
pub async fn origin_lock_middleware(
    State(db_state): State<DbState>,
    lock_state: axum::extract::Extension<OriginLockState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let peer_addr = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0);
    log_client_ip_fields(request.headers(), peer_addr);

    if is_health_check(request.method(), request.uri()) {
        return health_check_body(&db_state).into_response();
    }

    let header_values: Vec<_> = request.headers().get_all(EDGE_AUTH_HEADER).iter().collect();

    let (authorized, reason) = match header_values.as_slice() {
        [] => (false, "missing_secret"),
        [single] => match single.to_str() {
            Ok(value) if lock_state.matches(value) => (true, ""),
            Ok(_) => (false, "wrong_secret"),
            Err(_) => (false, "unparseable_secret"),
        },
        // A repeated header is rejected outright, not resolved to its
        // first value — an intermediate proxy and this check must never
        // disagree about which value applies.
        _ => (false, "repeated_secret_header"),
    };

    if !authorized {
        tracing::warn!(reason, "origin lock: rejected request");
        return StatusCode::NOT_FOUND.into_response();
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "folio_edge_test_secret";
    const OTHER: &str = "folio_edge_a_different_secret";

    #[test]
    fn matching_token_is_authorized() {
        let state = OriginLockState::new(SECRET);
        assert!(state.matches(SECRET));
    }

    #[test]
    fn wrong_token_is_rejected() {
        let state = OriginLockState::new(SECRET);
        assert!(!state.matches(OTHER));
    }

    #[test]
    fn empty_presented_value_is_rejected() {
        let state = OriginLockState::new(SECRET);
        assert!(!state.matches(""));
    }

    #[test]
    fn constant_time_eq_is_reflexive() {
        let digest = Sha256::digest(SECRET.as_bytes()).into();
        assert!(constant_time_eq(&digest, &digest));
    }

    #[test]
    fn constant_time_eq_detects_any_single_byte_difference() {
        let a: [u8; 32] = Sha256::digest(SECRET.as_bytes()).into();
        for i in 0..32 {
            let mut b = a;
            b[i] ^= 0x01;
            assert!(!constant_time_eq(&a, &b), "byte {i} flip must be detected");
        }
    }

    #[test]
    fn health_check_path_and_method_are_exact() {
        let health_get: Uri = "/api/health".parse().unwrap();
        let health_get_query: Uri = "/api/health?x=1".parse().unwrap();
        let health_slash: Uri = "/api/health/".parse().unwrap();
        let other_path: Uri = "/api/healthz".parse().unwrap();

        assert!(is_health_check(&Method::GET, &health_get));
        assert!(is_health_check(&Method::GET, &health_get_query));
        assert!(!is_health_check(&Method::GET, &health_slash));
        assert!(!is_health_check(&Method::GET, &other_path));
        assert!(!is_health_check(&Method::HEAD, &health_get));
        assert!(!is_health_check(&Method::POST, &health_get));
    }
}
