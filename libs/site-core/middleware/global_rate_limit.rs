use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::auth::{extract_token, validate_session};
use crate::state::DbState;

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
    /// Call periodically to bound memory usage (optional, not wired up by default).
    #[allow(dead_code)]
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

/// Extract IP from headers using the configured trusted header → X-Forwarded-For → fallback chain.
///
/// `trusted_header`: the header name configured via `TRUSTED_IP_HEADER` (e.g. `fly-client-ip`).
/// Pass `None` to skip the trusted header step and fall straight to `x-forwarded-for`.
pub fn extract_ip_for_rate_limit(headers: &HeaderMap, trusted_header: Option<&str>) -> String {
    if let Some(header_name) = trusted_header
        && let Some(val) = headers.get(header_name)
        && let Ok(val_str) = val.to_str()
    {
        let trimmed = val_str.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Some(forwarded) = headers.get("x-forwarded-for")
        && let Ok(val) = forwarded.to_str()
        && let Some(first_ip) = val.split(',').next()
    {
        let trimmed = first_ip.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    "unknown".to_string()
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

    let ip = extract_ip_for_rate_limit(request.headers(), db_state.trusted_ip_header.as_deref());

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

    #[test]
    fn ip_extraction_priority() {
        let mut headers = HeaderMap::new();

        // Only XFF present, no trusted header configured — should use XFF
        headers.insert("x-forwarded-for", "1.1.1.1, 2.2.2.2".parse().unwrap());
        assert_eq!(extract_ip_for_rate_limit(&headers, None), "1.1.1.1");

        // fly-client-ip present and configured as trusted header — takes priority over XFF
        headers.insert("fly-client-ip", "3.3.3.3".parse().unwrap());
        assert_eq!(
            extract_ip_for_rate_limit(&headers, Some("fly-client-ip")),
            "3.3.3.3"
        );

        // Trusted header configured but absent — falls back to XFF
        let mut xff_only = HeaderMap::new();
        xff_only.insert("x-forwarded-for", "4.4.4.4".parse().unwrap());
        assert_eq!(
            extract_ip_for_rate_limit(&xff_only, Some("fly-client-ip")),
            "4.4.4.4"
        );

        // Neither header — should fall back to "unknown"
        let empty = HeaderMap::new();
        assert_eq!(extract_ip_for_rate_limit(&empty, None), "unknown");
    }
}
