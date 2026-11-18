use axum::{
    Extension, Json,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tracing;

use crate::error::AppError;
use crate::middleware::global_rate_limit::extract_ip_for_rate_limit;
use crate::state::DbState;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: String,
}

pub fn generate_token() -> String {
    use rand::RngExt;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    hex::encode(bytes)
}

/// Hash a password with Argon2id using a random salt. Used at startup.
pub fn hash_password(password: &str) -> Result<String, AppError> {
    use argon2::password_hash::SaltString;
    use argon2::password_hash::rand_core::OsRng;
    use argon2::{Argon2, PasswordHasher};

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?;
    Ok(hash.to_string())
}

/// Verify a password against an Argon2id hash. Constant-time comparison.
pub fn verify_password(hash: &str, password: &str) -> Result<bool, AppError> {
    use argon2::password_hash::PasswordHash;
    use argon2::{Argon2, PasswordVerifier};

    let parsed = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("Invalid password hash: {}", e)))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn create_session(conn: &Connection, token: &str) -> Result<String, AppError> {
    let now_ts = jiff::Timestamp::now();
    let now = format_utc(now_ts);
    let expires_at = utc_after_hours(now_ts, 24)?;

    conn.execute(
        "INSERT INTO admin_sessions (token, created_at, expires_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![token, now, expires_at],
    )?;

    Ok(expires_at)
}

pub fn validate_session(conn: &Connection, token: &str) -> Result<bool, AppError> {
    let now = utc_now_string();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM admin_sessions WHERE token = ?1 AND expires_at > ?2",
        rusqlite::params![token, now],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

pub fn delete_session(conn: &Connection, token: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM admin_sessions WHERE token = ?1",
        rusqlite::params![token],
    )?;
    Ok(())
}

pub fn cleanup_expired(conn: &Connection) -> Result<(), AppError> {
    let now = utc_now_string();
    conn.execute(
        "DELETE FROM admin_sessions WHERE expires_at <= ?1",
        rusqlite::params![now],
    )?;
    Ok(())
}

pub fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

pub async fn require_auth(
    State(state): State<DbState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    use axum::response::IntoResponse;

    let token = match extract_token(request.headers()) {
        Some(t) => t,
        None => {
            return AppError::Unauthorized("Missing authorization token".to_string())
                .into_response();
        }
    };

    let valid = {
        let conn = match state.db.lock() {
            Ok(c) => c,
            Err(_) => {
                return AppError::Internal("Failed to acquire database lock".to_string())
                    .into_response();
            }
        };
        match validate_session(&conn, &token) {
            Ok(v) => v,
            Err(e) => return e.into_response(),
        }
    };

    if !valid {
        return AppError::Unauthorized("Invalid or expired token".to_string()).into_response();
    }

    next.run(request).await
}

#[tracing::instrument(skip_all)]
pub async fn login(
    State(state): State<DbState>,
    // `ConnectInfo` is absent when the server is not built with
    // `into_make_service_with_connect_info` (some test harnesses); the
    // shared extractor then falls back to "unknown" only as a last resort.
    connect_info: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), AppError> {
    // Rate limit: 5 failed attempts per IP per hour
    // Check is before password verification so brute force is blocked,
    // but only failed attempts increment the counter (below).
    // Keyed by the same extractor as every other limiter (#3482): a
    // configured-but-absent trusted header falls to the peer address,
    // never the client-controlled x-forwarded-for.
    let peer_addr = connect_info.map(|Extension(ConnectInfo(addr))| addr);
    let ip = extract_ip_for_rate_limit(&headers, state.trusted_ip_header.as_deref(), peer_addr);
    {
        let conn = state
            .db
            .lock()
            .map_err(|_| AppError::Internal("Failed to acquire database lock".to_string()))?;
        // Read-only check: reject if already over limit
        let count: i64 = conn.query_row(
            "SELECT COALESCE(
                (SELECT request_count FROM rate_limits
                 WHERE ip = ?1 AND endpoint = 'login_fail'
                 AND window_start = strftime('%Y-%m-%d %H:00:00', 'now')),
                0
            )",
            rusqlite::params![&ip],
            |row| row.get(0),
        )?;
        if count >= 5 {
            return Err(AppError::RateLimited("Rate limit exceeded".to_string()));
        }
    }

    if !verify_password(&state.admin_password_hash, &payload.password)? {
        // Only increment rate limit counter on failed attempts
        let conn = state
            .db
            .lock()
            .map_err(|_| AppError::Internal("Failed to acquire database lock".to_string()))?;
        crate::ai::rate_limit::check_rate_limit(&conn, &ip, "login_fail", 5)?;
        return Err(AppError::Unauthorized("Invalid password".to_string()));
    }

    let token = generate_token();

    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("Failed to acquire database lock".to_string()))?;

    let expires_at = create_session(&conn, &token)?;
    drop(conn);

    Ok((StatusCode::OK, Json(LoginResponse { token, expires_at })))
}

pub async fn logout(
    State(state): State<DbState>,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    let token = extract_token(&headers)
        .ok_or_else(|| AppError::Unauthorized("Missing authorization token".to_string()))?;

    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("Failed to acquire database lock".to_string()))?;

    delete_session(&conn, &token)?;
    drop(conn);

    Ok(StatusCode::NO_CONTENT)
}

/// Storage format for `admin_sessions` timestamps. The columns are TEXT and compared
/// lexicographically in SQL, so it must stay fixed-width UTC at second precision.
const SESSION_TS_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

fn format_utc(ts: jiff::Timestamp) -> String {
    ts.strftime(SESSION_TS_FORMAT).to_string()
}

fn utc_now_string() -> String {
    format_utc(jiff::Timestamp::now())
}

fn utc_after_hours(base: jiff::Timestamp, hours: i64) -> Result<String, AppError> {
    let ts = base
        .checked_add(jiff::SignedDuration::from_hours(hours))
        .map_err(|e| AppError::Internal(format!("session expiry out of range: {e}")))?;
    Ok(format_utc(ts))
}

#[cfg(test)]
mod timestamp_format_tests {
    use super::*;

    // admin_sessions.created_at/expires_at are TEXT compared lexicographically in SQL
    // (`expires_at > ?`), so the format is a storage contract: fixed-width, UTC,
    // second precision, literal `Z`. Sub-second input must truncate, never round.
    fn fmt(secs: i64, nanos: i32) -> String {
        format_utc(jiff::Timestamp::new(secs, nanos).expect("in range"))
    }

    #[test]
    fn pins_format_for_fixed_instants() {
        let cases: [(i64, i32, &str); 8] = [
            (0, 0, "1970-01-01T00:00:00Z"),
            (1_709_251_199, 0, "2024-02-29T23:59:59Z"),
            (1_709_251_199, 999_999_999, "2024-02-29T23:59:59Z"),
            (1_704_067_199, 0, "2023-12-31T23:59:59Z"),
            (1_704_067_200, 0, "2024-01-01T00:00:00Z"),
            (4_107_542_399, 0, "2100-02-28T23:59:59Z"),
            (4_107_542_400, 0, "2100-03-01T00:00:00Z"),
            (1_704_067_200, 500_000_000, "2024-01-01T00:00:00Z"),
        ];
        for (secs, nanos, want) in cases {
            assert_eq!(fmt(secs, nanos), want, "secs={secs} nanos={nanos}");
        }
    }
}
