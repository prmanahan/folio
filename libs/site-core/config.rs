use std::env;

/// Required prefix on `EDGE_AUTH_TOKEN`, ahead of 64 hex characters (32
/// bytes / 256 bits). The prefix gives the value a distinctive, greppable
/// shape distinct from any other hex-looking config value in the
/// environment or in logs.
pub const EDGE_AUTH_TOKEN_PREFIX: &str = "folio_edge_";

/// Number of hex characters required after [`EDGE_AUTH_TOKEN_PREFIX`].
const EDGE_AUTH_TOKEN_HEX_LEN: usize = 64;

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
    /// Shared secret Cloudflare injects into every request it forwards to
    /// the origin (task #3558). Required at startup, no default and no
    /// disable path — same fail-loud posture as `admin_password`. Only
    /// its SHA-256 digest is ever kept beyond startup (see
    /// `middleware::origin_lock::OriginLockState`).
    pub edge_auth_token: String,
}

/// Validate `EDGE_AUTH_TOKEN`'s shape: [`EDGE_AUTH_TOKEN_PREFIX`] followed
/// by exactly [`EDGE_AUTH_TOKEN_HEX_LEN`] hex characters. Returns the
/// human-readable reason on failure; never panics itself, so callers
/// control the failure mode (`Config::from_env` panics, a test can
/// assert on the message).
fn validate_edge_auth_token(token: &str) -> Result<(), String> {
    let Some(hex_part) = token.strip_prefix(EDGE_AUTH_TOKEN_PREFIX) else {
        return Err(format!("must start with '{EDGE_AUTH_TOKEN_PREFIX}'"));
    };
    if hex_part.len() != EDGE_AUTH_TOKEN_HEX_LEN || !hex_part.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err(format!(
            "must be followed by exactly {EDGE_AUTH_TOKEN_HEX_LEN} hex characters \
             ({} bytes)",
            EDGE_AUTH_TOKEN_HEX_LEN / 2
        ));
    }
    Ok(())
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let page_hit_salt = std::env::var("PAGE_HIT_SALT").unwrap_or_else(|_| {
            tracing::warn!("PAGE_HIT_SALT not set — using default value; set this in production");
            "folio-default-salt".to_string()
        });

        let edge_auth_token =
            env::var("EDGE_AUTH_TOKEN").expect("EDGE_AUTH_TOKEN env var is required");
        if let Err(reason) = validate_edge_auth_token(&edge_auth_token) {
            panic!("EDGE_AUTH_TOKEN is invalid: {reason}");
        }

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
            edge_auth_token,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_token() -> String {
        format!(
            "{EDGE_AUTH_TOKEN_PREFIX}{}",
            "a".repeat(EDGE_AUTH_TOKEN_HEX_LEN)
        )
    }

    #[test]
    fn valid_token_passes() {
        assert!(validate_edge_auth_token(&valid_token()).is_ok());
    }

    #[test]
    fn missing_prefix_is_rejected() {
        let token = "a".repeat(EDGE_AUTH_TOKEN_HEX_LEN);
        assert!(validate_edge_auth_token(&token).is_err());
    }

    #[test]
    fn empty_token_is_rejected() {
        assert!(validate_edge_auth_token("").is_err());
    }

    #[test]
    fn short_hex_part_is_rejected() {
        let token = format!("{EDGE_AUTH_TOKEN_PREFIX}abcd");
        assert!(validate_edge_auth_token(&token).is_err());
    }

    #[test]
    fn non_hex_characters_are_rejected() {
        let token = format!(
            "{EDGE_AUTH_TOKEN_PREFIX}{}",
            "z".repeat(EDGE_AUTH_TOKEN_HEX_LEN)
        );
        assert!(validate_edge_auth_token(&token).is_err());
    }

    #[test]
    fn one_hex_char_short_is_rejected() {
        let token = format!(
            "{EDGE_AUTH_TOKEN_PREFIX}{}",
            "a".repeat(EDGE_AUTH_TOKEN_HEX_LEN - 1)
        );
        assert!(validate_edge_auth_token(&token).is_err());
    }

    #[test]
    fn one_hex_char_long_is_rejected() {
        let token = format!(
            "{EDGE_AUTH_TOKEN_PREFIX}{}",
            "a".repeat(EDGE_AUTH_TOKEN_HEX_LEN + 1)
        );
        assert!(validate_edge_auth_token(&token).is_err());
    }
}
