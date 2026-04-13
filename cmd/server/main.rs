use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Router, routing::get};
use clap::{Parser, Subcommand};
use site_core::auth;
use site_core::config::Config;
use site_core::db;
use site_core::middleware::global_rate_limit::{
    GlobalRateLimitState, global_rate_limit_middleware,
};
use site_core::middleware::page_hits::page_hits_middleware;
use site_core::routes;
use site_core::state::{AppState, DbState};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

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
}

async fn security_headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "content-security-policy",
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'"
            .parse().unwrap(),
    );
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    headers.insert(
        "referrer-policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );
    headers.insert(
        "permissions-policy",
        "camera=(), microphone=(), geolocation=()".parse().unwrap(),
    );
    headers.insert(
        "strict-transport-security",
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );
    response
}

async fn serve_avatar(Path(filename): Path<String>) -> Response {
    // Sanitize: only allow alphanumeric, dash, underscore, dot
    // Also reject dotfiles and path traversal attempts
    if !filename
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        || filename.starts_with('.')
        || filename.contains("..")
    {
        return (StatusCode::BAD_REQUEST, "Invalid filename").into_response();
    }

    let avatar_dir = std::env::var("AVATAR_DIR").unwrap_or_else(|_| "data/avatars".to_string());
    let path = std::path::Path::new(&avatar_dir).join(&filename);

    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let mime = mime_guess::from_path(&filename)
                .first_or_octet_stream()
                .to_string();
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, mime),
                    (header::CACHE_CONTROL, "public, max-age=86400".to_string()),
                ],
                bytes,
            )
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Avatar not found").into_response(),
    }
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
        .and_then(|key| rig::providers::anthropic::Client::new(key).ok());

    let db_state: DbState = Arc::new(AppState {
        db: Arc::new(Mutex::new(conn)),
        admin_password_hash: password_hash,
        rig_client,
        trusted_ip_header: config.trusted_ip_header.clone(),
        page_hit_salt: config.page_hit_salt.clone(),
    });

    let cors = CorsLayer::new()
        .allow_origin(
            std::env::var("CORS_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".to_string())
                .parse::<axum::http::HeaderValue>()
                .expect("Invalid CORS_ORIGIN value"),
        )
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    let global_rate_limit = GlobalRateLimitState::new();

    let app = Router::new()
        .route("/api/health", get(routes::health_check))
        .merge(routes::public_router())
        .merge(routes::ai::routes_with_connect_info())
        .merge(routes::admin::admin_router(db_state.clone()))
        .route("/api/avatars/{filename}", get(serve_avatar))
        .with_state(db_state.clone())
        .layer(axum::middleware::from_fn_with_state(
            db_state.clone(),
            page_hits_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            db_state.clone(),
            global_rate_limit_middleware,
        ))
        .layer(axum::Extension(global_rate_limit))
        .layer(cors)
        .layer(axum::middleware::from_fn(security_headers))
        .fallback_service(site_core::static_files::static_file_service(
            &config.static_dir,
        ));

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
    }
}
