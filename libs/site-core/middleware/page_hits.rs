use axum::extract::State;
use axum::response::Response;
use sha2::{Sha256, Digest};

use crate::middleware::global_rate_limit::extract_ip_for_rate_limit;
use crate::models::page_hits;
use crate::state::DbState;

/// Routes worth tracking for unique visitor counts.
fn should_track(path: &str) -> bool {
    matches!(
        path,
        "/" | "/projects" | "/articles" | "/agents"
    ) || path.starts_with("/projects/")
        || path.starts_with("/articles/")
}

/// Hash an IP with a salt for privacy. The hash is one-way so raw IPs
/// are never stored.
fn hash_ip(ip: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}", salt, ip));
    format!("{:x}", hasher.finalize())
}

/// Middleware that records unique page hits per path.
///
/// Follows the same pattern as `global_rate_limit_middleware`. Extracts IP
/// via the shared `extract_ip_for_rate_limit` helper, hashes it, and fires
/// a best-effort INSERT OR IGNORE. The DB write does not block the response.
pub async fn page_hits_middleware(
    State(db_state): State<DbState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let path = request.uri().path().to_string();

    if should_track(&path) {
        let ip = extract_ip_for_rate_limit(
            request.headers(),
            db_state.trusted_ip_header.as_deref(),
        );
        let ip_hash = hash_ip(&ip, &db_state.page_hit_salt);
        let db = db_state.db.clone();

        // Fire-and-forget: spawn so we don't block the response.
        tokio::task::spawn_blocking(move || {
            if let Ok(conn) = db.lock()
                && let Err(e) = page_hits::record_hit(&conn, &path, &ip_hash)
            {
                tracing::warn!(error = %e, path = %path, "failed to record page hit");
            }
        });
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracked_routes() {
        assert!(should_track("/"));
        assert!(should_track("/projects"));
        assert!(should_track("/articles"));
        assert!(should_track("/agents"));
        assert!(should_track("/projects/folio"));
        assert!(should_track("/articles/some-article"));
    }

    #[test]
    fn untracked_routes() {
        assert!(!should_track("/api/health"));
        assert!(!should_track("/api/projects"));
        assert!(!should_track("/_app/immutable/something.js"));
        assert!(!should_track("/admin/dashboard"));
        assert!(!should_track("/favicon.ico"));
    }

    #[test]
    fn hash_is_deterministic() {
        let a = hash_ip("1.2.3.4", "test-salt");
        let b = hash_ip("1.2.3.4", "test-salt");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_differs_for_different_ips() {
        let a = hash_ip("1.2.3.4", "test-salt");
        let b = hash_ip("5.6.7.8", "test-salt");
        assert_ne!(a, b);
    }
}
