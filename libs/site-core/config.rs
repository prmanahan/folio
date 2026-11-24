use std::env;

/// Required prefix on `EDGE_AUTH_TOKEN`, ahead of 64 hex characters (32
/// bytes / 256 bits). The prefix gives the value a distinctive, greppable
/// shape distinct from any other hex-looking config value in the
/// environment or in logs.
pub const EDGE_AUTH_TOKEN_PREFIX: &str = "folio_edge_";

/// Where the built frontend lives when `STATIC_DIR` is unset.
pub const DEFAULT_STATIC_DIR: &str = "frontend/build";

/// Number of hex characters required after [`EDGE_AUTH_TOKEN_PREFIX`].
const EDGE_AUTH_TOKEN_HEX_LEN: usize = 64;

pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub admin_password: String,
    pub anthropic_api_key: Option<String>,
    /// Header name to trust for the real client IP.
    /// Set to the header your reverse proxy injects (e.g. `cf-connecting-ip`
    /// behind Cloudflare). When unset, rate limits key on the peer address;
    /// `x-forwarded-for` is never trusted. Switching providers means changing
    /// this one env var.
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

/// A required variable, read from `result` (the outcome of `env::var(name)`).
/// Fails loud when absent. Panics carry fixed text only: `VarError::NotUnicode`
/// holds the raw value, and its `Debug` would print a secret's bytes, before
/// the payload-free panic hook is installed.
fn required_from(name: &str, result: Result<String, env::VarError>) -> String {
    match result {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => panic!("{name} env var is required: NotPresent"),
        Err(env::VarError::NotUnicode(_)) => {
            panic!("{name} env var is required and must be valid Unicode")
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let page_hit_salt = std::env::var("PAGE_HIT_SALT").unwrap_or_else(|_| {
            tracing::warn!("PAGE_HIT_SALT not set — using default value; set this in production");
            "folio-default-salt".to_string()
        });

        let trusted_ip_header = env::var("TRUSTED_IP_HEADER").ok();
        if trusted_ip_header.is_none() {
            tracing::warn!(
                "TRUSTED_IP_HEADER not set — rate limits will key on the peer address \
                 (the proxy, if one is in front)"
            );
        }

        let edge_auth_token = required_from("EDGE_AUTH_TOKEN", env::var("EDGE_AUTH_TOKEN"));
        if let Err(reason) = validate_edge_auth_token(&edge_auth_token) {
            panic!("EDGE_AUTH_TOKEN is invalid: {reason}");
        }

        Self {
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a number"),
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "data/site.db".to_string()),
            admin_password: required_from("ADMIN_PASSWORD", env::var("ADMIN_PASSWORD")),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok(),
            trusted_ip_header,
            static_dir: env::var("STATIC_DIR").unwrap_or_else(|_| DEFAULT_STATIC_DIR.to_string()),
            page_hit_salt,
            edge_auth_token,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text a panic carried, whichever payload type it used.
    fn panic_text(payload: Box<dyn std::any::Any + Send>) -> String {
        match payload.downcast::<String>() {
            Ok(s) => *s,
            Err(p) => p
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .unwrap_or_default(),
        }
    }

    /// A value that is not valid Unicode, built per run: its bytes stand in
    /// for a secret that must never reach a log.
    #[cfg(unix)]
    fn non_unicode_secret() -> (std::ffi::OsString, String) {
        use std::os::unix::ffi::OsStringExt;
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let mut bytes = nonce.clone().into_bytes();
        bytes.push(0xff); // never valid UTF-8
        (std::ffi::OsString::from_vec(bytes), nonce)
    }

    #[cfg(unix)]
    #[test]
    fn a_non_unicode_required_value_never_appears_in_the_panic_message() {
        for name in ["ADMIN_PASSWORD", "EDGE_AUTH_TOKEN"] {
            // Given a value that is not valid Unicode
            let (value, nonce) = non_unicode_secret();
            // When the required read fails on it
            let caught = std::panic::catch_unwind(|| {
                required_from(name, Err(env::VarError::NotUnicode(value)))
            });
            // Then it panics, and the message carries none of the value
            let text = panic_text(caught.expect_err("a non-Unicode value must fail loud"));
            assert!(!text.contains(&nonce), "{name}: the value leaked: {text}");
            assert!(text.contains(name), "{name}: {text}");
        }
    }

    #[test]
    fn an_absent_required_value_keeps_its_message() {
        let caught = std::panic::catch_unwind(|| {
            required_from("ADMIN_PASSWORD", Err(env::VarError::NotPresent))
        });
        assert_eq!(
            panic_text(caught.expect_err("absent must fail loud")),
            "ADMIN_PASSWORD env var is required: NotPresent"
        );
    }

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

    /// Task #3558 (F5): `.env.example`'s `EDGE_AUTH_TOKEN` placeholder is
    /// deliberately shape-invalid, so a copy-paste that skips generating
    /// a real value panics loudly at startup instead of running on a
    /// value every clone of this public repo shares. This reads the real
    /// file rather than a copy of its value, so an edit that accidentally
    /// makes the placeholder valid-shaped fails here, not silently.
    #[test]
    fn dot_env_example_placeholder_is_deliberately_invalid() {
        let env_example =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.env.example"))
                .expect(".env.example must be readable");

        let placeholder = env_example
            .lines()
            .find_map(|line| line.strip_prefix("EDGE_AUTH_TOKEN="))
            .expect(".env.example must set EDGE_AUTH_TOKEN");

        assert!(
            validate_edge_auth_token(placeholder).is_err(),
            ".env.example's EDGE_AUTH_TOKEN placeholder ({placeholder:?}) must \
             FAIL startup validation — a shape-valid placeholder would let a \
             copy-paste run on a value every clone of this public repo shares"
        );
    }
}
