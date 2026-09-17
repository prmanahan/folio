use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::auth::{extract_token, validate_session};
use crate::net::truncate_to_prefix;
use crate::state::DbState;

/// IPv6 addresses are normalised to this prefix before use as a
/// rate-limit bucket key. A typical IPv6 allocation is a /64 or larger,
/// so this keys the allocation rather than the individual host address.
const RATE_LIMIT_IPV6_KEY_PREFIX: u32 = 64;

/// Parse `candidate` as an IP address and, if it's IPv6, normalise it to
/// [`RATE_LIMIT_IPV6_KEY_PREFIX`]. IPv4 addresses and anything that
/// doesn't parse as an IP (e.g. the literal `"unknown"`, or a malformed
/// header value) pass through unchanged — this only narrows an
/// over-wide IPv6 key, it never invents or rejects one.
///
/// `pub(crate)` so `auth::extract_client_ip` — a second, older IP
/// extractor that keys the login-failure limiter — can share the same
/// normalisation rather than drift from it (task #3558 F2).
pub(crate) fn normalize_rate_limit_key(candidate: String) -> String {
    match candidate.parse::<IpAddr>() {
        Ok(addr @ IpAddr::V6(_)) => {
            truncate_to_prefix(addr, RATE_LIMIT_IPV6_KEY_PREFIX).to_string()
        }
        _ => candidate,
    }
}

/// Per-IP sliding window entry: timestamps of recent requests within the window.
#[derive(Default)]
struct IpRecord {
    timestamps: Vec<Instant>,
}

/// Shared in-memory store for global rate limiting.
#[derive(Clone, Default)]
pub struct GlobalRateLimitState(Arc<Mutex<HashMap<String, IpRecord>>>);

impl GlobalRateLimitState {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }

    /// Check if `ip` is within the limit. Returns `true` if the request is allowed.
    ///
    /// Uses a sliding window of 60 seconds. Cleans up timestamps older than the window
    /// on each call for the queried IP (no background task needed at this scale).
    pub fn check(&self, ip: &str, limit: usize, window: Duration) -> bool {
        let mut store = self.0.lock().expect("global rate limit lock poisoned");
        let now = Instant::now();
        let record = store.entry(ip.to_string()).or_default();

        // Remove timestamps outside the sliding window.
        record
            .timestamps
            .retain(|&t| now.duration_since(t) < window);

        if record.timestamps.len() < limit {
            record.timestamps.push(now);
            true
        } else {
            false
        }
    }

    /// Purge all IPs that have no timestamps within the window.
    ///
    /// Called on an interval from `main.rs::run_server` (task #3558):
    /// Phase 2 keys buckets per visitor rather than per Cloudflare edge,
    /// which widens this map's key space for the life of the process. An
    /// unpurged entry never affects correctness (a stale entry's own
    /// timestamps are filtered on each `check()`), only memory — this
    /// bounds that growth rather than leaving it to a manual call.
    pub fn purge_stale(&self, window: Duration) {
        let mut store = self.0.lock().expect("global rate limit lock poisoned");
        let now = Instant::now();
        store.retain(|_, record| {
            record
                .timestamps
                .retain(|&t| now.duration_since(t) < window);
            !record.timestamps.is_empty()
        });
    }
}

/// Extract IP from headers using the configured trusted header →
/// X-Forwarded-For → ConnectInfo peer addr → `"unknown"` chain.
///
/// `trusted_header`: the header name configured via `TRUSTED_IP_HEADER` (e.g. `fly-client-ip`).
/// Pass `None` to skip the trusted header step and fall straight to `x-forwarded-for`.
///
/// `peer_addr`: the `ConnectInfo` peer socket address when available.
///
/// LLM-audit L1 / R4: when neither header resolves, prefer the peer
/// address so distinct no-proxy clients get distinct rate-limit buckets.
/// The literal `"unknown"` (a single shared bucket — one client can DoS
/// another, many attackers share one quota) is used ONLY when no peer
/// address is available. Header precedence is unchanged: the peer addr is
/// a fallback, never an override of a present trusted header / XFF.
///
/// SECURITY NOTE: The trusted header is set by the reverse proxy and cannot be spoofed
/// by clients in production. `x-forwarded-for` is used only as a local-dev fallback
/// and is client-controlled; if this service is ever exposed directly (no proxy),
/// rate limiting by XFF IP is bypassable — and, since `middleware::page_hits`
/// hashes this same value to key unique-hit analytics, a spoofed XFF also
/// poisons hit counts. The consequence tracks the call surface, not just the
/// limiter: a caller added here inherits both failure modes.
///
/// Bucket keys for the global rate-limit middleware,
/// `middleware::page_hits` and the AI handlers in `routes::ai` all come
/// from here. `routes::ai` carried a character-identical private copy
/// until #1077; a second extractor is how two callers drift into
/// disagreeing about who gets throttled.
///
/// NOT the only IP read in the service. `auth::extract_client_ip` keys
/// the login brute-force limiter and is still the pre-R4 two-arg shape:
/// no peer-addr fallback, so it collapses every no-header client into
/// one `"unknown"` bucket. Converging it changes an authentication path
/// and was outside #1077's scope — reported, not fixed here.
///
/// Task #3558: when a trusted header IS configured but absent from this
/// request, the fallback goes straight to the peer address — never
/// `x-forwarded-for`. `x-forwarded-for` stays a fallback only for the
/// no-trusted-header case (local dev without a reverse proxy). Every
/// resolved candidate is normalized via
/// `normalize_rate_limit_key` (IPv6 → /64) before it's returned, so this
/// one function is the single place that decision has to be made for
/// every caller (the global limiter, `page_hits`, and the AI routes).
pub fn extract_ip_for_rate_limit(
    headers: &HeaderMap,
    trusted_header: Option<&str>,
    peer_addr: Option<SocketAddr>,
) -> String {
    if let Some(header_name) = trusted_header {
        if let Some(val) = headers.get(header_name)
            && let Ok(val_str) = val.to_str()
        {
            let trimmed = val_str.trim();
            if !trimmed.is_empty() {
                return normalize_rate_limit_key(trimmed.to_string());
            }
        }
        // Trusted header configured but missing/empty on this request:
        // go straight to the peer addr, skipping x-forwarded-for.
        return normalize_rate_limit_key(match peer_addr {
            Some(addr) => addr.ip().to_string(),
            None => "unknown".to_string(),
        });
    }

    // No trusted header configured at all (e.g. local dev without a
    // reverse proxy) — x-forwarded-for is the historical dev-ergonomics
    // fallback.
    if let Some(forwarded) = headers.get("x-forwarded-for")
        && let Ok(val) = forwarded.to_str()
        && let Some(first_ip) = val.split(',').next()
    {
        let trimmed = first_ip.trim();
        if !trimmed.is_empty() {
            return normalize_rate_limit_key(trimmed.to_string());
        }
    }
    // R4: prefer the ConnectInfo peer addr over the collapsing "unknown"
    // bucket; "unknown" only when no peer addr is available.
    normalize_rate_limit_key(match peer_addr {
        Some(addr) => addr.ip().to_string(),
        None => "unknown".to_string(),
    })
}

/// Axum middleware: 60 requests per IP per minute, globally.
///
/// Authenticated admin sessions (valid Bearer token) are exempt — admin workflows
/// involve rapid saves and bulk edits that should not be throttled.
pub async fn global_rate_limit_middleware(
    State(db_state): State<DbState>,
    rate_limit_state: axum::extract::Extension<GlobalRateLimitState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    // Check for a valid admin Bearer token; exempt if present and valid.
    if let Some(token) = extract_token(request.headers()) {
        let is_valid_admin = {
            match db_state.db.lock() {
                Ok(conn) => validate_session(&conn, &token).unwrap_or(false),
                Err(_) => false,
            }
        };
        if is_valid_admin {
            return next.run(request).await;
        }
    }

    // R4: thread the ConnectInfo peer addr (present in production via
    // `into_make_service_with_connect_info`; absent in tests / non-
    // ConnectInfo setups → `None` → "unknown" only then).
    let peer_addr = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0);
    let ip = extract_ip_for_rate_limit(
        request.headers(),
        db_state.trusted_ip_header.as_deref(),
        peer_addr,
    );

    let window = Duration::from_secs(60);
    let limit = 60_usize;

    if !rate_limit_state.check(&ip, limit, window) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [("Retry-After", "60")],
            axum::Json(
                serde_json::json!({ "error": "Too many requests. Please wait before retrying." }),
            ),
        )
            .into_response();
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn within_limit_succeeds() {
        let state = GlobalRateLimitState::new();
        let window = Duration::from_secs(60);
        let limit = 5;

        for i in 0..limit {
            assert!(
                state.check("1.2.3.4", limit, window),
                "request {i} should be allowed"
            );
        }
    }

    #[test]
    fn over_limit_denied() {
        let state = GlobalRateLimitState::new();
        let window = Duration::from_secs(60);
        let limit = 3;

        for _ in 0..limit {
            assert!(state.check("10.0.0.1", limit, window));
        }
        assert!(
            !state.check("10.0.0.1", limit, window),
            "request beyond limit should be denied"
        );
    }

    #[test]
    fn different_ips_are_independent() {
        let state = GlobalRateLimitState::new();
        let window = Duration::from_secs(60);
        let limit = 2;

        state.check("192.168.1.1", limit, window);
        state.check("192.168.1.1", limit, window);
        // 192.168.1.1 is now at limit; 192.168.1.2 should still pass
        assert!(
            state.check("192.168.1.2", limit, window),
            "a different IP should have its own quota"
        );
    }

    #[test]
    fn expired_timestamps_are_evicted() {
        let state = GlobalRateLimitState::new();
        // Use a very short window so we can exhaust it, then show it resets.
        let window = Duration::from_millis(10);
        let limit = 2;

        state.check("5.5.5.5", limit, window);
        state.check("5.5.5.5", limit, window);
        assert!(
            !state.check("5.5.5.5", limit, window),
            "third request should be denied"
        );

        // Sleep past the window
        std::thread::sleep(Duration::from_millis(20));

        // Now the timestamps have expired; the next request should be allowed.
        assert!(
            state.check("5.5.5.5", limit, window),
            "after window expires, requests should be allowed again"
        );
    }

    /// Task #3558 (F6): `purge_stale` is now wired on an interval in
    /// `main.rs` — Phase 2 keys buckets per visitor, widening this map's
    /// key space for the life of the process, so an entry whose
    /// timestamps have all fallen outside `window` must actually be
    /// removed, not merely have its own timestamp `Vec` emptied in place.
    #[test]
    fn purge_stale_removes_an_entry_with_no_timestamps_in_window() {
        let state = GlobalRateLimitState::new();
        let window = Duration::from_millis(10);

        state.check("6.6.6.6", 5, window);
        assert_eq!(
            state.0.lock().unwrap().len(),
            1,
            "a checked IP must have a map entry"
        );

        std::thread::sleep(Duration::from_millis(20));
        state.purge_stale(window);

        assert_eq!(
            state.0.lock().unwrap().len(),
            0,
            "an entry with no timestamps inside the window must be purged, \
             not just have its timestamp list emptied"
        );
    }

    #[test]
    fn purge_stale_keeps_an_entry_with_a_recent_timestamp() {
        let state = GlobalRateLimitState::new();
        let window = Duration::from_secs(60);

        state.check("7.7.7.7", 5, window);
        state.purge_stale(window);

        assert_eq!(
            state.0.lock().unwrap().len(),
            1,
            "an entry with a timestamp inside the window must survive a purge"
        );
    }

    #[test]
    fn ip_extraction_priority() {
        let mut headers = HeaderMap::new();

        // Only XFF present, no trusted header configured — should use XFF
        // (peer addr present but headers win — R4 precedence preserved).
        headers.insert("x-forwarded-for", "1.1.1.1, 2.2.2.2".parse().unwrap());
        let peer: SocketAddr = "9.9.9.9:443".parse().unwrap();
        assert_eq!(
            extract_ip_for_rate_limit(&headers, None, Some(peer)),
            "1.1.1.1"
        );

        // fly-client-ip present and configured as trusted header — takes priority over XFF
        headers.insert("fly-client-ip", "3.3.3.3".parse().unwrap());
        assert_eq!(
            extract_ip_for_rate_limit(&headers, Some("fly-client-ip"), Some(peer)),
            "3.3.3.3"
        );

        // Task #3558: trusted header configured but absent on this
        // request — falls back to the peer addr, never x-forwarded-for.
        // x-forwarded-for remains a fallback only when no trusted header
        // is configured at all (local dev without a reverse proxy).
        let mut xff_only = HeaderMap::new();
        xff_only.insert("x-forwarded-for", "4.4.4.4".parse().unwrap());
        assert_eq!(
            extract_ip_for_rate_limit(&xff_only, Some("fly-client-ip"), Some(peer)),
            "9.9.9.9",
            "a configured-but-absent trusted header must fall to the peer \
             addr, not the client-controlled x-forwarded-for"
        );
        // ...and with no peer addr available either, "unknown" — still
        // never XFF.
        assert_eq!(
            extract_ip_for_rate_limit(&xff_only, Some("fly-client-ip"), None),
            "unknown"
        );

        // Neither header, but a peer addr → R4: use the peer addr, NOT the
        // collapsing "unknown" bucket.
        let empty = HeaderMap::new();
        assert_eq!(
            extract_ip_for_rate_limit(&empty, None, Some(peer)),
            "9.9.9.9"
        );

        // Neither header AND no peer addr — only then fall back to "unknown".
        assert_eq!(extract_ip_for_rate_limit(&empty, None, None), "unknown");
    }

    /// Task #3558 (H2): an IPv6 trusted-header value is normalized to its
    /// /64 — two addresses in the same /64 must share one bucket key.
    #[test]
    fn ipv6_trusted_header_keys_are_normalized_to_64() {
        let mut headers_a = HeaderMap::new();
        headers_a.insert(
            "cf-connecting-ip",
            "2001:db8:1234:5678:aaaa:bbbb:cccc:0001".parse().unwrap(),
        );
        let mut headers_b = HeaderMap::new();
        headers_b.insert(
            "cf-connecting-ip",
            "2001:db8:1234:5678:ffff:ffff:ffff:ffff".parse().unwrap(),
        );

        let key_a = extract_ip_for_rate_limit(&headers_a, Some("cf-connecting-ip"), None);
        let key_b = extract_ip_for_rate_limit(&headers_b, Some("cf-connecting-ip"), None);

        assert_eq!(
            key_a, key_b,
            "two addresses in the same /64 must resolve to the same bucket key"
        );
        assert_eq!(key_a, "2001:db8:1234:5678::");
    }

    /// A different /64 must NOT collapse into the same bucket.
    #[test]
    fn ipv6_addresses_in_different_64s_are_independent() {
        let mut headers_a = HeaderMap::new();
        headers_a.insert("cf-connecting-ip", "2001:db8:0:0::1".parse().unwrap());
        let mut headers_b = HeaderMap::new();
        headers_b.insert("cf-connecting-ip", "2001:db8:0:1::1".parse().unwrap());

        let key_a = extract_ip_for_rate_limit(&headers_a, Some("cf-connecting-ip"), None);
        let key_b = extract_ip_for_rate_limit(&headers_b, Some("cf-connecting-ip"), None);

        assert_ne!(key_a, key_b);
    }

    /// IPv4 keys are untouched by the IPv6 normalization step.
    #[test]
    fn ipv4_keys_are_not_truncated() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", "203.0.113.77".parse().unwrap());
        assert_eq!(
            extract_ip_for_rate_limit(&headers, Some("cf-connecting-ip"), None),
            "203.0.113.77"
        );
    }
}
