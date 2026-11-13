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

/// The one static file that must never be edge-cached (spec R-11): Cloudflare
/// would otherwise apply its default `max-age=14400` to a list that is replaced
/// on each deploy.
const NO_CACHE_FILE: &str = "projects.json";

/// Percent-decodes `s` the way `ServeDir` does (`percent_decode`): a `%` not
/// followed by two hex digits is kept literally. `None` when the result is not
/// UTF-8, where `ServeDir` serves nothing.
fn percent_decode_utf8(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let decoded = (bytes[i] == b'%')
            .then(|| bytes.get(i + 1..i + 3))
            .flatten()
            // Strict hex: `from_str_radix` alone accepts a leading `+`.
            .filter(|hex| hex.iter().all(u8::is_ascii_hexdigit))
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match decoded {
            Some(b) => {
                out.push(b);
                i += 3;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// True when `ServeDir` would resolve `request_path` to the file
/// [`NO_CACHE_FILE`] at the static root. Mirrors its `build_and_validate_path`
/// (tower-http 0.6, `serve_dir/mod.rs`): trim leading `/`, percent-decode, walk
/// `Path::components()` skipping `CurDir` (and so a trailing `/`). Any
/// `RootDir`, `ParentDir` or `Prefix` is refused by `ServeDir`, so it never
/// matches here; neither does a nested path. Keyed on the resolved path, not
/// the raw one: `//projects.json`, `/%70rojects.json` and `/projects.json/`
/// reach the same file. A path that does not decode to UTF-8 returns `true`:
/// the header is harmless on a response that was never going to be the file,
/// while a missed one on a path that could be would fail open. A
/// case-insensitive dev filesystem (macOS) would also serve `/Projects.json`
/// from the file without this matching; production is Linux.
fn resolves_to_no_cache_file(request_path: &str) -> bool {
    use std::path::Component;
    let Some(decoded) = percent_decode_utf8(request_path.trim_start_matches('/')) else {
        return true;
    };
    let mut names = Vec::new();
    for component in std::path::Path::new(&decoded).components() {
        match component {
            Component::Normal(name) => names.push(name),
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => return false,
        }
    }
    names == [std::ffi::OsStr::new(NO_CACHE_FILE)]
}

/// Two independent rules, both applied to every response of the static service.
///
/// 1. Caching, keyed on the REQUEST's resolved path: a response for the file
///    `projects.json` (200, 304, HEAD, and the SPA fallback while the file is
///    absent) gets `Cache-Control: no-cache`, however the request spelled the
///    path (see [`resolves_to_no_cache_file`]). No other path's caching headers
///    change.
/// 2. CSP, keyed on WHERE the response came from, not its `Content-Type`: the
///    response is marked with the policy to send, and `text/html` gets its
///    charset. A 304 carries no `Content-Type` and its headers update the
///    cached response, so a 304 with a weaker CSP would downgrade the cached
///    page; non-SvelteKit HTML would otherwise miss the policy too. This sets
///    no CSP header itself.
async fn mark_static_response(
    State(policy): State<HeaderValue>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let no_cache = resolves_to_no_cache_file(request.uri().path());
    let mut response = next.run(request).await;
    if no_cache {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    }
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

    #[test]
    fn no_cache_file_matches_every_spelling_servedir_resolves_to_it() {
        for p in [
            "/projects.json",
            "//projects.json",
            "/./projects.json",
            "/projects.json/.",
            "/projects.json/",
            "/projects%2Ejson",
            "/%70rojects.json",
            "/projects.json%2F",
        ] {
            assert!(resolves_to_no_cache_file(p), "{p}");
        }
    }

    /// An extra `no-cache` is harmless; a missed one fails open. So a path that
    /// cannot be evaluated (not UTF-8 once decoded) gets the header, unlike the
    /// RootDir/ParentDir refusals, which ServeDir never serves.
    #[test]
    fn undecodable_path_fails_toward_no_cache() {
        assert!(resolves_to_no_cache_file("/projects.json%FF"));
    }

    #[test]
    fn percent_decode_requires_two_ascii_hex_digits() {
        // `u8::from_str_radix` would take `+5` as 0x05; ServeDir keeps it literal.
        assert_eq!(percent_decode_utf8("%+5"), Some("%+5".to_string()));
        assert_eq!(percent_decode_utf8("%-1"), Some("%-1".to_string()));
        assert_eq!(percent_decode_utf8("%4"), Some("%4".to_string()));
        assert_eq!(percent_decode_utf8("%70%2e"), Some("p.".to_string()));
    }

    #[test]
    fn no_cache_file_rejects_other_paths_and_what_servedir_refuses() {
        for p in [
            "/",
            "",
            "/projects.json.bak",
            "/x/projects.json",
            "/projects.jsonx",
            "/Projects.json",
            "/../projects.json",
            "/x/../projects.json",
            "/projects.json/..",
            // Decodes to a leading `/`: RootDir, which ServeDir refuses.
            "/%2Fprojects.json",
            "/projects%252Ejson",
            "/%",
        ] {
            assert!(!resolves_to_no_cache_file(p), "{p}");
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
