//! `/articles` retired: R-25 of `docs/specs/2026-11-18-live-lists-and-blog-move.md`.
//!
//! Answers a fixed 301 to the blog for `/articles` and every path whose
//! percent-decoded form starts `/articles/`, for every method. Nothing from
//! the request reaches the response: the target is a constant.
//!
//! Wired inside `origin_lock` (see `app::build_app`), so an unauthenticated
//! request gets the lock's 404, never this redirect (R-23).

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

/// The one redirect target. A constant, so no request byte can reach `Location`.
pub(crate) const BLOG_URL: &str = "https://mnemra.dev/blog/";
const CACHE_CONTROL: &str = "public, max-age=3600";

/// Strict percent-decoding of a request path. `None` when a `%` is not followed
/// by two hex digits, or the decoded bytes are not UTF-8. Stricter than
/// `static_files::percent_decode_utf8`, which keeps an invalid escape literally
/// (R-25), so that helper is deliberately not reused.
fn strict_percent_decode(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hi = hex_value(*bytes.get(i + 1)?)?;
            let lo = hex_value(*bytes.get(i + 2)?)?;
            out.push(hi << 4 | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// ASCII hex digit to its value; a leading `+` or `-` is not a digit.
fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// True for the raw request path (query excluded) that R-25 redirects: the
/// literal `/articles`, or any path whose strict decoding starts `/articles/`
/// (which includes `/articles/`). A path that cannot be decoded is not
/// redirected and keeps today's handling.
fn is_redirect_path(raw_path: &str) -> bool {
    raw_path == "/articles"
        || strict_percent_decode(raw_path).is_some_and(|decoded| decoded.starts_with("/articles/"))
}

fn redirect_response() -> Response {
    (
        StatusCode::MOVED_PERMANENTLY,
        [
            (header::LOCATION, HeaderValue::from_static(BLOG_URL)),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static(CACHE_CONTROL),
            ),
        ],
    )
        .into_response()
}

/// Redirects R-25's paths; passes every other request through untouched.
pub async fn articles_redirect_middleware(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if is_redirect_path(request.uri().path()) {
        return redirect_response();
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::http::Method;

    const SPA_BODY: &str = "spa-fallback";

    fn server() -> axum_test::TestServer {
        let app = Router::new()
            .fallback(|| async { SPA_BODY })
            .layer(axum::middleware::from_fn(articles_redirect_middleware));
        axum_test::TestServer::new(app)
    }

    fn assert_fixed_301(response: &axum_test::TestResponse, what: &str) {
        assert_eq!(
            response.status_code(),
            StatusCode::MOVED_PERMANENTLY,
            "{what}"
        );
        assert_eq!(response.header("location"), BLOG_URL, "{what}");
        assert_eq!(
            response.header("cache-control"),
            "public, max-age=3600",
            "{what}"
        );
    }

    // R-25: Given the five listed paths, When requested, Then the fixed 301.
    #[tokio::test]
    async fn the_listed_paths_answer_the_fixed_301() {
        let server = server();
        for path in [
            "/articles",
            "/articles/",
            "/articles/x",
            "/articles%2Fx",
            "/articles?next=https://example.com/",
        ] {
            assert_fixed_301(&server.get(path).await, path);
        }
    }

    // R-25: forged Host and each X-Forwarded-* change nothing, on all five
    // AC1 paths.
    #[tokio::test]
    async fn forged_host_and_forwarded_headers_do_not_change_the_response() {
        let server = server();
        for (name, value) in [
            ("host", "evil.example"),
            ("x-forwarded-host", "evil.example"),
            ("x-forwarded-proto", "http"),
            ("x-forwarded-for", "203.0.113.9"),
            ("x-forwarded-prefix", "/evil"),
        ] {
            for path in [
                "/articles",
                "/articles/",
                "/articles/x",
                "/articles%2Fx",
                "/articles?next=https://example.com/",
            ] {
                let response = server.get(path).add_header(name, value).await;
                assert_fixed_301(&response, &format!("{path} with {name}"));
            }
        }
    }

    // R-25: every method answers the same 301.
    #[tokio::test]
    async fn every_method_answers_the_same_301() {
        let server = server();
        for method in [
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::HEAD,
            Method::OPTIONS,
            Method::PATCH,
        ] {
            let response = server.method(method.clone(), "/articles/x").await;
            assert_fixed_301(&response, method.as_str());
        }
    }

    // R-25: near-misses keep today's handling.
    #[tokio::test]
    async fn near_miss_paths_do_not_redirect() {
        let server = server();
        for path in ["/articlesx", "//articles", "/article", "/Articles/x"] {
            let response = server.get(path).await;
            assert_ne!(
                response.status_code(),
                StatusCode::MOVED_PERMANENTLY,
                "{path}"
            );
            assert_eq!(response.text(), SPA_BODY, "{path} reaches the fallback");
        }
    }

    // R-25: a path that cannot be strictly decoded is neither 301 nor 5xx.
    #[tokio::test]
    async fn undecodable_paths_pass_through_without_redirect_or_5xx() {
        let server = server();
        for path in [
            "/articles/%zz",
            "/articles/%FF",
            "/articles/%",
            "/articles/%4",
        ] {
            let response = server.get(path).await;
            assert_eq!(response.status_code(), StatusCode::OK, "{path}");
            assert_eq!(response.text(), SPA_BODY, "{path} reaches the fallback");
        }
    }

    // R-25 decoder rows.
    #[test]
    fn strict_decode_rejects_a_bad_escape_and_non_utf8() {
        for bad in [
            "%zz", "%", "%4", "/a%4", "%+5", "%-1", "%FF", "%C3", "%80abc",
        ] {
            assert_eq!(strict_percent_decode(bad), None, "{bad}");
        }
    }

    #[test]
    fn strict_decode_accepts_valid_escapes_of_either_case() {
        assert_eq!(strict_percent_decode("/a%2Fb"), Some("/a/b".to_string()));
        assert_eq!(strict_percent_decode("/a%2fb"), Some("/a/b".to_string()));
        assert_eq!(strict_percent_decode("%C3%A9"), Some("\u{e9}".to_string()));
        assert_eq!(strict_percent_decode("plain"), Some("plain".to_string()));
    }

    #[test]
    fn matcher_rows() {
        for yes in [
            "/articles",
            "/articles/",
            "/articles/x",
            "/articles%2Fx",
            "/articles%2f",
            "/%61rticles/x",
            "/articles/%C3%A9",
        ] {
            assert!(is_redirect_path(yes), "{yes}");
        }
        for no in [
            "/",
            "/articlesx",
            "//articles",
            "/articles/%zz",
            "/articles/%FF",
            "/article",
            "/api/articles",
            "/%61rticles",
        ] {
            assert!(!is_redirect_path(no), "{no}");
        }
    }

    /// Seeded xorshift; plain std, reproducible.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    fn encode_all(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("%{b:02X}")).collect()
    }

    // Property (R-25): strict decode round-trips any valid-UTF-8 string
    // percent-encoded byte by byte, and a string carrying one invalid escape
    // decodes to None.
    #[test]
    fn property_decode_roundtrips_and_rejects_invalid_escapes() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let alphabet: Vec<char> = "ab/Z-_.~ \u{e9}\u{4e2d}\u{1f600}%0".chars().collect();
        for _ in 0..500 {
            let len = rng.below(12);
            let s: String = (0..len)
                .map(|_| alphabet[rng.below(alphabet.len())])
                .collect();
            assert_eq!(
                strict_percent_decode(&encode_all(s.as_bytes())),
                Some(s.clone())
            );
            for bad in ["%g0", "%0g", "%", "%0", "%+1"] {
                let broken = format!("{}{bad}", encode_all(s.as_bytes()));
                assert_eq!(strict_percent_decode(&broken), None, "{broken}");
            }
        }
    }

    /// `/articles` (each letter literal or %-encoded), a separator (literal or
    /// %2F/%2f), then the marker twice: once %-encoded per character at random,
    /// once as a literal, unencoded segment, so the raw path as sent holds the
    /// marker verbatim.
    fn random_path(rng: &mut Rng, marker: &str) -> String {
        let mut path = String::from("/");
        for c in "articles".chars() {
            if rng.below(3) == 0 {
                path.push_str(&format!("%{:02X}", c as u8));
            } else {
                path.push(c);
            }
        }
        path.push_str(["/", "%2F", "%2f"][rng.below(3)]);
        for c in marker.chars() {
            if rng.below(2) == 0 {
                path.push_str(&format!("%{:02x}", c as u8));
            } else {
                path.push(c);
            }
        }
        path.push('/');
        path.push_str(marker);
        path
    }

    // Property (R-25): a generated path whose strict decode starts /articles/
    // gets the fixed 301, and neither the marker (carried literally in the
    // path and query, and in Host and X-Forwarded-Host/-Proto/-For/-Prefix)
    // nor the raw path as sent appears in any response header value (lossy
    // UTF-8, so non-ASCII values are scanned) or in the body.
    #[tokio::test]
    async fn property_redirect_is_fixed_and_echoes_no_request_byte() {
        let server = server();
        let mut rng = Rng(0xD1B5_4A32_D192_ED03);
        for i in 0..200 {
            let marker = format!("zq{}x{}", rng.next() % 1_000_000, i);
            let path = random_path(&mut rng, &marker);
            let decoded = strict_percent_decode(&path).expect("generated path decodes");
            assert!(decoded.starts_with("/articles/"), "{path}");
            let response = server
                .get(&format!("{path}?q={marker}"))
                .add_header("host", format!("{marker}.example"))
                .add_header("x-forwarded-host", marker.clone())
                .add_header("x-forwarded-proto", marker.clone())
                .add_header("x-forwarded-for", marker.clone())
                .add_header("x-forwarded-prefix", marker.clone())
                .await;
            assert_fixed_301(&response, &path);
            let mut seen = String::new();
            for (name, value) in response.headers() {
                seen.push_str(&format!(
                    "{name}:{}\n",
                    String::from_utf8_lossy(value.as_bytes())
                ));
            }
            seen.push_str(&response.text());
            assert!(!seen.contains(&marker), "{path} echoed {marker} in {seen}");
            assert!(
                !seen.contains(&path),
                "{path} echoed the raw path in {seen}"
            );
        }
    }

    // Property (R-25): a generated decodable path whose strict decode neither
    // equals /articles nor starts with /articles/ falls through to the
    // fallback, never 301.
    #[tokio::test]
    async fn property_other_decodable_paths_fall_through() {
        let server = server();
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        let heads = [
            "/articlesx",
            "//articles",
            "/article",
            "/Articles",
            "/api/articles",
            "/x/articles",
            "/",
            "/%61rticle",
            "/%2Farticles",
            "/articles%00x",
        ];
        let mut checked = 0;
        for _ in 0..300 {
            let mut path = heads[rng.below(heads.len())].to_string();
            for _ in 0..rng.below(4) {
                path.push_str(["a", "-", "%41", "%2e", "z9", "%C3%A9", "/"][rng.below(7)]);
            }
            let Some(decoded) = strict_percent_decode(&path) else {
                continue;
            };
            // Guard the generator, not the code under test: only keep paths
            // outside R-25's set by its own definition.
            if decoded.starts_with("/articles/") || path == "/articles" {
                continue;
            }
            checked += 1;
            let response = server.get(&path).await;
            assert_ne!(
                response.status_code(),
                StatusCode::MOVED_PERMANENTLY,
                "{path}"
            );
            assert_eq!(response.text(), SPA_BODY, "{path} reaches the fallback");
        }
        assert!(
            checked > 100,
            "generator produced only {checked} usable paths"
        );
    }
}
