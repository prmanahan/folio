use axum::Router;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use tower_http::services::{ServeDir, ServeFile};

use crate::error::AppError;

/// Fallback header CSP for the static fallback, served only when
/// `csp-header.txt` is unusable (see [`load_csp_header`]).
///
/// Deliberately has no script/style/default directive: the HTML's own CSP is a
/// `<meta http-equiv>` tag with per-build script hashes, and a header CSP is
/// enforced IN ADDITION (the intersection wins), so any script directive here
/// could block the hashed inline scripts. Only the directives a `<meta>` CSP
/// cannot carry (`frame-ancestors`) or that are safe to repeat live here.
pub(crate) const STATIC_CSP: &str = "frame-ancestors 'none'; object-src 'none'; base-uri 'none'";

/// File the frontend build writes next to `index.html`: the exact header value
/// (the page's meta policy plus `; frame-ancestors 'none'`). The build-time
/// guard owns its validation; the server only transports it.
const CSP_HEADER_FILE: &str = "csp-header.txt";

/// Response-extension marker: "this response came from the static service",
/// carrying the `Content-Security-Policy` value to send. Crate-private, so
/// only code in this crate can claim the static policy.
/// `app::security_headers` is the only CSP writer and reads it.
#[derive(Clone)]
pub(crate) struct FromStaticService(pub(crate) HeaderValue);

/// Marks every response from the static service with the policy to send, and
/// gives `text/html` its charset. Keyed on WHERE the response came from, not
/// its `Content-Type`: a 304 carries no `Content-Type` and its headers update
/// the cached response, so a 304 with a weaker CSP would downgrade the cached
/// page; non-SvelteKit HTML would otherwise miss the policy too. Sets no CSP
/// header itself.
async fn mark_static_response(
    State(policy): State<HeaderValue>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let mut response = next.run(request).await;
    // Exact match only: other content types, and values that already name a
    // charset, are left alone.
    if response
        .headers()
        .get(header::CONTENT_TYPE)
        .is_some_and(|v| v.as_bytes() == b"text/html")
    {
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
    }
    response.extensions_mut().insert(FromStaticService(policy));
    response
}

/// Why `csp-header.txt` was not usable.
#[derive(Debug)]
enum CspHeaderError {
    Unreadable(std::io::Error),
    Empty,
    InvalidHeaderValue(axum::http::header::InvalidHeaderValue),
}

impl std::fmt::Display for CspHeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable(e) => write!(f, "unreadable: {e}"),
            Self::Empty => write!(f, "empty"),
            Self::InvalidHeaderValue(e) => write!(f, "not a valid header value: {e}"),
        }
    }
}

/// Reads `<static_dir>/csp-header.txt` verbatim: no trim, no policy parsing.
fn read_csp_header(static_dir: &std::path::Path) -> Result<HeaderValue, CspHeaderError> {
    let bytes =
        std::fs::read(static_dir.join(CSP_HEADER_FILE)).map_err(CspHeaderError::Unreadable)?;
    if bytes.is_empty() {
        return Err(CspHeaderError::Empty);
    }
    HeaderValue::from_bytes(&bytes).map_err(CspHeaderError::InvalidHeaderValue)
}

/// The header CSP for static responses: the build-validated file, or
/// [`STATIC_CSP`] after one error log naming the path and the reason (log and
/// stay up, like [`check_csp_meta`]; the post-deploy check runs after traffic has
/// moved and turns the deploy job red if the fallback is served).
fn load_csp_header(static_dir: &str) -> HeaderValue {
    let path = std::path::Path::new(static_dir);
    match read_csp_header(path) {
        Ok(value) => value,
        Err(e) => {
            tracing::error!(
                path = %path.join(CSP_HEADER_FILE).display(),
                reason = %e,
                "csp-header.txt unusable: static responses fall back to the minimal \
                 header CSP, so the page is not protected by the build-validated header policy. \
                 A plain rebuild does not write the file: run `npm run check:csp` in \
                 frontend/ (or `just frontend-build`)."
            );
            HeaderValue::from_static(STATIC_CSP)
        }
    }
}

/// Unknown `/api` paths are API misses, not SPA routes: a JSON 404, never the
/// index page.
async fn api_not_found() -> AppError {
    AppError::NotFound("Not found".to_string())
}

/// Create a static file service with SPA fallback.
///
/// All requests that don't match a route fall through to this service. If the
/// requested file doesn't exist, it serves `index.html` with 200 so the SPA
/// router can handle client-side routing (#4718); `.fallback` keeps the
/// inner service's status, where `.not_found_service` forced 404. Under `/api`
/// the answer is a JSON 404 instead. Static responses carry the policy from
/// `csp-header.txt` (read once, here), or [`STATIC_CSP`] if it is unusable; the
/// JSON 404 does not (it gets the full policy).
pub fn static_file_service(static_dir: &str) -> Router {
    let index = format!("{}/index.html", static_dir);
    let files = Router::new()
        .fallback_service(ServeDir::new(static_dir).fallback(ServeFile::new(index)))
        .layer(axum::middleware::from_fn_with_state(
            load_csp_header(static_dir),
            mark_static_response,
        ));
    Router::new()
        .route("/api", any(api_not_found))
        .route("/api/", any(api_not_found))
        .route("/api/{*rest}", any(api_not_found))
        .fallback_service(files)
}

/// True when `html` contains a `<meta http-equiv="content-security-policy">`.
fn has_csp_meta(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    ["\"", "'"]
        .iter()
        .any(|q| lower.contains(&format!("http-equiv={q}content-security-policy{q}")))
}

/// Log at error level (no panic: report and stay up) when the built
/// `index.html` lacks the CSP meta tag, since HTML relies on it alone for
/// script restrictions. Returns whether the tag was found.
fn check_csp_meta(index: &std::path::Path) -> bool {
    match std::fs::read_to_string(index) {
        Ok(html) if has_csp_meta(&html) => true,
        Ok(_) => {
            tracing::error!(
                index = %index.display(),
                "index.html has NO content-security-policy meta tag: HTML pages are \
                 served without a script CSP. Rebuild the frontend (svelte.config.js kit.csp)."
            );
            false
        }
        Err(e) => {
            tracing::error!(index = %index.display(), error = %e,
                "could not read index.html to check for its CSP meta tag");
            false
        }
    }
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
    check_csp_meta(&index);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[test]
    fn csp_meta_is_found_in_either_quote_style_and_any_case() {
        for html in [
            r#"<meta http-equiv="content-security-policy" content="script-src 'sha256-x'">"#,
            r#"<META HTTP-EQUIV="Content-Security-Policy" content="x">"#,
            "<meta http-equiv='content-security-policy' content='x'>",
        ] {
            assert!(has_csp_meta(html), "{html}");
        }
    }

    #[test]
    fn csp_meta_is_not_found_in_plain_html_or_unrelated_meta() {
        for html in [
            "",
            "<html><body>no meta</body></html>",
            r#"<meta http-equiv="refresh" content="0">"#,
            r#"<meta name="content-security-policy" content="x">"#,
        ] {
            assert!(!has_csp_meta(html), "{html}");
        }
    }

    #[derive(Clone, Default)]
    struct LogBuf(Arc<Mutex<Vec<u8>>>);
    impl io::Write for LogBuf {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0
                .lock()
                .map_err(|_| io::Error::other("poisoned"))?
                .extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl<'a> MakeWriter<'a> for LogBuf {
        type Writer = LogBuf;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    #[test]
    fn missing_meta_logs_one_error_and_does_not_panic() {
        let dir = std::env::temp_dir().join(format!("folio-csp-meta-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.html"), "<html></html>").unwrap();

        let buf = LogBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_ansi(false)
            .finish();
        let found = tracing::subscriber::with_default(subscriber, || {
            validate_static_dir(dir.to_str().unwrap());
            check_csp_meta(&dir.join("index.html"))
        });
        assert!(!found);
        let log = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
        // validate_static_dir logged once, the direct call logged once more.
        assert_eq!(log.matches("ERROR").count(), 2, "{log}");
        assert!(log.contains("NO content-security-policy meta tag"), "{log}");

        std::fs::write(
            dir.join("index.html"),
            r#"<meta http-equiv="content-security-policy" content="x">"#,
        )
        .unwrap();
        assert!(check_csp_meta(&dir.join("index.html")));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
