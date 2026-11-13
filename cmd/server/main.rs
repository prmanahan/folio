use clap::{Parser, Subcommand};
use site_core::app::build_app;
use site_core::auth;
use site_core::config::{Config, DEFAULT_STATIC_DIR};
use site_core::db;
use site_core::middleware::global_rate_limit::GlobalRateLimitState;
use site_core::middleware::origin_lock::OriginLockState;
use site_core::projects_list;
use site_core::state::{AppState, DbState};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing_subscriber::EnvFilter;

/// Sliding window used by `global_rate_limit_middleware` (60 req/min) and
/// the interval this purges stale buckets on. Same value on purpose: an
/// entry can't go stale faster than the window it's measured against.
const GLOBAL_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

#[derive(Parser)]
#[command(name = "folio")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the web server (default)
    Serve,
    /// Validate `$STATIC_DIR/projects.json` and its freshness, then exit
    /// (Fly release command). Reads no secrets and opens no database.
    CheckProjects,
}

/// R-18. Deliberately independent of `Config`: it must run on Fly's release
/// machine, which has no volume, and must not require or read the app's
/// secrets. Returns whether the list passed.
fn check_projects() -> bool {
    let static_dir = std::env::var_os("STATIC_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(DEFAULT_STATIC_DIR));
    let path = static_dir.join("projects.json");

    // `%e` is safe for untrusted text: every `Display` arm that interpolates
    // text from the file does so through `{:?}`, which escapes control
    // characters, independent of the order the checks ran in. Pinned per arm
    // in `libs/site-core/tests/test_projects_list_validator.rs`.
    let list = match projects_list::load(&path) {
        Ok(list) => list,
        Err(e) => {
            tracing::error!(path = %path.display(), error = %e, "check-projects failed");
            return false;
        }
    };
    let age = match projects_list::check_fetched_at_age_now(list.fetched_at()) {
        Ok(age) => age,
        Err(e) => {
            tracing::error!(path = %path.display(), error = %e, "check-projects failed");
            return false;
        }
    };
    let counts: Vec<String> = list.counts().map(|(o, n)| format!("{o}={n}")).collect();
    tracing::info!(
        path = %path.display(),
        fetched_at = list.fetched_at(),
        age_secs = age.0.whole_seconds(),
        bound = format!(
            "{} minutes behind, {} minutes ahead",
            projects_list::MAX_AGE_BEHIND.whole_minutes(),
            projects_list::MAX_AGE_AHEAD.whole_minutes()
        ),
        counts = counts.join(" "),
        "check-projects passed"
    );
    true
}

async fn run_server() {
    let config = Config::from_env();
    site_core::static_files::validate_static_dir(&config.static_dir);
    tracing::info!(port = config.port, "starting server");
    let conn = db::connect(&config.database_url).expect("Failed to connect to database");

    // Hash the admin password with Argon2id at startup
    let password_hash =
        auth::hash_password(&config.admin_password).expect("Failed to hash admin password");

    let rig_client = config
        .anthropic_api_key
        .as_ref()
        .and_then(|key| rig_core::providers::anthropic::Client::new(key).ok());

    let db_state: DbState = Arc::new(AppState {
        db: Arc::new(Mutex::new(conn)),
        admin_password_hash: password_hash,
        rig_client,
        trusted_ip_header: config.trusted_ip_header.clone(),
        page_hit_salt: config.page_hit_salt.clone(),
    });

    // LLM-audit Nit / R6: CORS_ORIGIN fail-loud. The old silent
    // `unwrap_or_else(|_| "http://localhost:3000")` made a missing prod
    // env var yield a wrong-origin policy that breaks the site without any
    // operator signal. WARN form (not require-in-prod) is chosen to match
    // the existing `PAGE_HIT_SALT` pattern and keep local/dev ergonomics
    // (dev has no CORS_ORIGIN; only `ADMIN_PASSWORD` is fail-loud). A
    // missing CORS_ORIGIN now emits a WARN naming the var before falling
    // back, so the misconfiguration is loud in the logs.
    let cors_origin = match std::env::var("CORS_ORIGIN") {
        Ok(origin) => origin,
        Err(_) => {
            tracing::warn!(
                "CORS_ORIGIN is unset; falling back to the localhost dev \
                 default (http://localhost:3000). In production set \
                 CORS_ORIGIN explicitly — a missing value silently yields a \
                 wrong-origin CORS policy."
            );
            "http://localhost:3000".to_string()
        }
    };
    let origin_lock_state = OriginLockState::new(&config.edge_auth_token);

    let global_rate_limit = GlobalRateLimitState::new();
    // Task #3558: Phase 2 keys rate-limit buckets per visitor rather than
    // per Cloudflare edge, widening this map's key space for the life of
    // the process. Purge on the same cadence as the limiter's own
    // window so an entry can't go stale faster than it's purged.
    let purge_state = global_rate_limit.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(GLOBAL_RATE_LIMIT_WINDOW);
        loop {
            interval.tick().await;
            purge_state.purge_stale(GLOBAL_RATE_LIMIT_WINDOW);
        }
    });

    let app = build_app(
        db_state,
        origin_lock_state,
        global_rate_limit,
        &config.static_dir,
        &cors_origin,
    );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .expect("Failed to bind to port");

    tracing::info!(port = config.port, "listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("Server error");
}

#[tokio::main]
async fn main() {
    // Initialize tracing — respects RUST_LOG env var, defaults to info
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Serve) {
        Commands::Serve => {
            run_server().await;
        }
        Commands::CheckProjects => {
            if !check_projects() {
                std::process::exit(1);
            }
        }
    }
}
