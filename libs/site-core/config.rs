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
    pub static_dir: String,
    pub page_hit_salt: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let page_hit_salt = std::env::var("PAGE_HIT_SALT").unwrap_or_else(|_| {
            tracing::warn!("PAGE_HIT_SALT not set — using default value; set this in production");
            "folio-default-salt".to_string()
        });

        Self {
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a number"),
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "data/site.db".to_string()),
            admin_password: env::var("ADMIN_PASSWORD").expect("ADMIN_PASSWORD env var is required"),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok(),
            trusted_ip_header: env::var("TRUSTED_IP_HEADER").ok(),
            static_dir: env::var("STATIC_DIR").unwrap_or_else(|_| "frontend/build".to_string()),
            page_hit_salt,
        }
    }
}
