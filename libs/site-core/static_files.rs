use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_status::SetStatus;

/// Create a static file service with SPA fallback.
///
/// All requests that don't match an API route fall through to this service.
/// If the requested file doesn't exist, it serves `index.html` so the SPA
/// router can handle client-side routing.
pub fn static_file_service(static_dir: &str) -> ServeDir<SetStatus<ServeFile>> {
    let index = format!("{}/index.html", static_dir);
    ServeDir::new(static_dir).not_found_service(ServeFile::new(index))
}

/// Validate that the static directory and its index.html exist at startup.
/// Panics with a clear message if either is missing.
pub fn validate_static_dir(path: &str) {
    let dir = std::path::Path::new(path);
    if !dir.is_dir() {
        panic!(
            "STATIC_DIR '{}' does not exist or is not a directory. \
             Set STATIC_DIR to the path containing your built frontend.",
            path
        );
    }
    let index = dir.join("index.html");
    if !index.is_file() {
        panic!(
            "STATIC_DIR '{}' exists but contains no index.html. \
             Run the frontend build first.",
            path
        );
    }
}

/// Serve an avatar image from `AVATAR_DIR` (default `data/avatars`).
///
/// `filename` is sanitized against path traversal: only alphanumerics,
/// `-`, `_` and `.` are allowed, and a leading dot or an embedded `..` is
/// rejected before the path is ever joined and read.
pub async fn serve_avatar(Path(filename): Path<String>) -> Response {
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
