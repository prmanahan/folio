use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
    Json,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tracing;

use crate::error::AppError;
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
    use argon2::{Argon2, PasswordHasher};
    use argon2::password_hash::SaltString;
    use argon2::password_hash::rand_core::OsRng;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?;
    Ok(hash.to_string())
}

/// Verify a password against an Argon2id hash. Constant-time comparison.
pub fn verify_password(hash: &str, password: &str) -> Result<bool, AppError> {
    use argon2::{Argon2, PasswordVerifier};
    use argon2::password_hash::PasswordHash;

    let parsed = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("Invalid password hash: {}", e)))?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
}

pub fn create_session(conn: &Connection, token: &str) -> Result<String, AppError> {
    let now_secs = unix_now();
    let now = format_unix_secs(now_secs);
    let expires_at = chrono_add_hours(now_secs, 24);

    conn.execute(
        "INSERT INTO admin_sessions (token, created_at, expires_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![token, now, expires_at],
    )?;

    Ok(expires_at)
}

pub fn validate_session(conn: &Connection, token: &str) -> Result<bool, AppError> {
    let now = chrono_now();
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
    let now = chrono_now();
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

/// Extract client IP from request headers.
///
/// Priority: `trusted_header` (configured via `TRUSTED_IP_HEADER` env var, e.g.
/// `fly-client-ip` on Fly.io) → `x-forwarded-for` (fallback for local dev) → "unknown".
fn extract_client_ip(headers: &HeaderMap, trusted_header: Option<&str>) -> String {
    // Prefer the configured trusted header (proxy-injected, not spoofable by clients).
    if let Some(header_name) = trusted_header
        && let Some(val) = headers.get(header_name)
        && let Ok(val_str) = val.to_str()
    {
        let trimmed = val_str.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    // Fall back to X-Forwarded-For for local dev (no proxy).
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

#[tracing::instrument(skip(state, headers, payload))]
pub async fn login(
    State(state): State<DbState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), AppError> {
    // Rate limit: 5 failed attempts per IP per hour
    // Check is before password verification so brute force is blocked,
    // but only failed attempts increment the counter (below).
    let ip = extract_client_ip(&headers, state.trusted_ip_header.as_deref());
    {
        let conn = state.db.lock().map_err(|_| {
            AppError::Internal("Failed to acquire database lock".to_string())
        })?;
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
        let conn = state.db.lock().map_err(|_| {
            AppError::Internal("Failed to acquire database lock".to_string())
        })?;
        crate::ai::rate_limit::check_rate_limit(&conn, &ip, "login_fail", 5)?;
        return Err(AppError::Unauthorized("Invalid password".to_string()));
    }

    let token = generate_token();

    let conn = state.db.lock().map_err(|_| {
        AppError::Internal("Failed to acquire database lock".to_string())
    })?;

    let expires_at = create_session(&conn, &token)?;
    drop(conn);

    Ok((
        StatusCode::OK,
        Json(LoginResponse { token, expires_at }),
    ))
}

pub async fn logout(
    State(state): State<DbState>,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    let token = extract_token(&headers)
        .ok_or_else(|| AppError::Unauthorized("Missing authorization token".to_string()))?;

    let conn = state.db.lock().map_err(|_| {
        AppError::Internal("Failed to acquire database lock".to_string())
    })?;

    delete_session(&conn, &token)?;
    drop(conn);

    Ok(StatusCode::NO_CONTENT)
}

// Simple UTC datetime helpers using std (no chrono dependency needed)
fn unix_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn chrono_now() -> String {
    format_unix_secs(unix_now())
}

fn chrono_add_hours(base_secs: u64, hours: u64) -> String {
    format_unix_secs(base_secs + hours * 3600)
}

fn format_unix_secs(secs: u64) -> String {
    // Manual ISO 8601 UTC formatting without external deps
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;

    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;

    // Convert days since epoch (1970-01-01) to year/month/day
    let (year, month, day) = days_to_ymd(days_since_epoch);

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, h, m, s)
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    // Algorithm to convert days since 1970-01-01 to (year, month, day)
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days: [u64; 12] = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}
