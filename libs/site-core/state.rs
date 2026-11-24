use crate::live_lists::LiveLists;
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
    /// The held copies of both lists (R-20). The AI handlers take one snapshot
    /// per request, so a prompt never mixes two moments. The same handle is
    /// layered as an `Extension` for the `/api/lists/` routes by `start`.
    pub lists: LiveLists,
}

pub type DbState = Arc<AppState>;
