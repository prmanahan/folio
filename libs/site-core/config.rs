use std::env;

pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub admin_password: String,
    pub anthropic_api_key: Option<String>,
    /// Header name to trust for the real client IP.
    /// Set to the header your reverse proxy injects (e.g. `fly-client-ip` on Fly.io).
    /// Defaults to `x-forwarded-for` when unset, which works for local dev.
    /// Switching providers means changing this one env var.
    pub trusted_ip_header: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        Self {
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a number"),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "data/site.db".to_string()),
            admin_password: env::var("ADMIN_PASSWORD")
                .expect("ADMIN_PASSWORD env var is required"),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok(),
            trusted_ip_header: env::var("TRUSTED_IP_HEADER").ok(),
        }
    }
}
