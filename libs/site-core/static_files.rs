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
