use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub admin_password_hash: String,
    pub rig_client: Option<rig::providers::anthropic::Client>,
    /// Header name to trust for the real client IP (e.g. "fly-client-ip").
    /// None means fall through to `x-forwarded-for` then connection addr.
    pub trusted_ip_header: Option<String>,
}

pub type DbState = Arc<AppState>;
