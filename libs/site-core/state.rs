use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub admin_password_hash: String,
    pub rig_client: Option<rig_core::providers::anthropic::Client>,
    /// Header name to trust for the real client IP (e.g. "cf-connecting-ip").
    /// None means key on the connection (peer) addr; `x-forwarded-for` is never trusted.
    pub trusted_ip_header: Option<String>,
    /// Salt used to hash IPs before storing page-hit records.
    pub page_hit_salt: String,
}

pub type DbState = Arc<AppState>;
