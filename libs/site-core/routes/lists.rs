//! The two `/api/lists/` readers (spec API Contract, R-21 to R-23).
//!
//! Both answer from the held copies and never start a refresh: the handlers
//! take a [`LiveLists`] and nothing that can fetch (R-17, review-checked).
//! Both are routed inside `build_app`, so the origin lock and the global rate
//! limiter wrap them (R-23).

use axum::Router;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::live_lists::LiveLists;

pub fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/api/lists/projects", get(projects))
        .route("/api/lists/freshness", get(freshness))
}

async fn projects(Extension(lists): Extension<LiveLists>) -> Response {
    StatusCode::NOT_IMPLEMENTED.into_response()
}

async fn freshness(Extension(lists): Extension<LiveLists>) -> Response {
    StatusCode::NOT_IMPLEMENTED.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use axum::http::{HeaderName, HeaderValue};
    use serde_json::{Value, json};

    use crate::live_lists::held::Held;
    use crate::middleware::global_rate_limit::GlobalRateLimitState;
    use crate::middleware::origin_lock::OriginLockState;
    use crate::projects_list::ProjectsList;

    const STARTED_AT: &str = "2026-11-19T11:59:00.000Z";
    const PROJECTS_DOC: &str = r#"{"fetched_at":"2026-11-19T12:00:00.000Z","groups":[{"owner":"mnemra","repos":[{"name":"alpha","description":"a"}]},{"owner":"prmanahan","repos":[{"name":"beta","description":""}]}]}"#;
    const ARTICLES_DOC: &str = r#"{"fetched_at":"2026-11-19T12:05:00.000Z","posts":[{"title":"T","summary":"S","link":"https://mnemra.dev/blog/t","published":"2026-11-18T08:30:00.000Z"}]}"#;
    const EDGE_SECRET: &str = "folio_edge_test_secret_for_ci_only";
    const UNAVAILABLE_BODY: &str = r#"{"error":"Projects list unavailable"}"#;

    fn lists() -> LiveLists {
        LiveLists::new(STARTED_AT.to_string())
    }

    fn with_projects(l: &LiveLists) {
        let held = Held::<ProjectsList>::new(PROJECTS_DOC.to_string()).unwrap();
        l.set_projects(Arc::new(held));
    }

    fn with_articles(l: &LiveLists) {
        let held = Held::new(ARTICLES_DOC.to_string()).unwrap();
        l.set_articles(Arc::new(held));
    }

    /// The routes alone, with the handle supplied.
    fn bare(l: &LiveLists) -> axum_test::TestServer {
        let app: Router = routes().layer(Extension(l.clone()));
        axum_test::TestServer::new(app)
    }

    fn header(response: &axum_test::TestResponse, name: &str) -> Option<String> {
        response
            .maybe_header(name)
            .map(|v| v.to_str().unwrap().to_string())
    }

    // ---- R-21 ----

    #[tokio::test]
    async fn r21_with_a_held_copy_answers_200_no_store_with_the_held_document() {
        // Given a held projects copy
        let l = lists();
        with_projects(&l);
        // When GET /api/lists/projects
        let response = bare(&l).get("/api/lists/projects").await;
        // Then 200, no-store, JSON, and the body is the held text byte for byte
        response.assert_status(StatusCode::OK);
        assert_eq!(
            header(&response, "cache-control").as_deref(),
            Some("no-store")
        );
        assert_eq!(
            header(&response, "content-type").as_deref(),
            Some("application/json")
        );
        assert_eq!(response.text(), PROJECTS_DOC);
    }

    #[tokio::test]
    async fn r21_with_no_data_yet_answers_503_no_store_with_the_exact_error_body() {
        let response = bare(&lists()).get("/api/lists/projects").await;
        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            header(&response, "cache-control").as_deref(),
            Some("no-store")
        );
        assert_eq!(
            header(&response, "content-type").as_deref(),
            Some("application/json")
        );
        assert_eq!(response.text(), UNAVAILABLE_BODY);
    }

    #[tokio::test]
    async fn r21_a_held_articles_copy_alone_does_not_make_projects_available() {
        let l = lists();
        with_articles(&l);
        bare(&l)
            .get("/api/lists/projects")
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }

    // ---- R-22 ----

    async fn freshness_of(l: &LiveLists) -> (axum_test::TestResponse, Value) {
        let response = bare(l).get("/api/lists/freshness").await;
        response.assert_status(StatusCode::OK);
        let body: Value = serde_json::from_str(&response.text()).expect("a JSON body");
        (response, body)
    }

    #[tokio::test]
    async fn r22_with_no_data_yet_both_times_are_null_and_started_at_is_present() {
        let (response, body) = freshness_of(&lists()).await;
        response.assert_status(StatusCode::OK);
        assert_eq!(
            header(&response, "cache-control").as_deref(),
            Some("no-store")
        );
        assert_eq!(
            body,
            json!({"projects": null, "articles": null, "started_at": STARTED_AT})
        );
    }

    #[tokio::test]
    async fn r22_each_time_is_the_held_copys_fetched_at() {
        let l = lists();
        with_projects(&l);
        with_articles(&l);
        let (_, body) = freshness_of(&l).await;
        assert_eq!(
            body,
            json!({
                "projects": "2026-11-19T12:00:00.000Z",
                "articles": "2026-11-19T12:05:00.000Z",
                "started_at": STARTED_AT,
            })
        );
    }

    #[tokio::test]
    async fn r22_a_missing_time_is_null_while_the_other_is_present() {
        let l = lists();
        with_articles(&l);
        let (_, body) = freshness_of(&l).await;
        assert_eq!(body["projects"], Value::Null);
        assert_eq!(body["articles"], json!("2026-11-19T12:05:00.000Z"));
        let mut keys: Vec<&String> = body.as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(keys, ["articles", "projects", "started_at"]);
    }

    // ---- R-23: the routes sit behind the origin lock and the limiter ----

    fn built_app(l: &LiveLists, limiter: GlobalRateLimitState) -> axum_test::TestServer {
        use std::sync::Mutex;
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrate(&conn).unwrap();
        let state: crate::state::DbState = Arc::new(crate::state::AppState {
            db: Arc::new(Mutex::new(conn)),
            admin_password_hash: crate::test_password::password_hash(),
            rig_client: None,
            trusted_ip_header: None,
            page_hit_salt: "test-salt".to_string(),
            projects: crate::test_projects::projects(),
        });
        let app = crate::app::build_app(
            state,
            OriginLockState::new(EDGE_SECRET),
            limiter,
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/static_site"),
            "http://localhost:3000",
        )
        .layer(Extension(l.clone()));
        axum_test::TestServer::new(app)
    }

    fn edge_header() -> (HeaderName, HeaderValue) {
        (
            HeaderName::from_static("x-folio-edge-auth"),
            HeaderValue::from_static(EDGE_SECRET),
        )
    }

    const LIST_ROUTES: [&str; 2] = ["/api/lists/projects", "/api/lists/freshness"];

    #[tokio::test]
    async fn r23_each_route_without_the_edge_secret_gets_the_origin_locks_refusal() {
        let l = lists();
        let server = built_app(&l, GlobalRateLimitState::new());
        // The lock's refusal is what any unknown path gets without the secret
        let refusal = server.get("/api/no-such-path").await;
        refusal.assert_status(StatusCode::NOT_FOUND);
        // (path, the route's own answer with no data held)
        let routes = [
            ("/api/lists/projects", StatusCode::SERVICE_UNAVAILABLE),
            ("/api/lists/freshness", StatusCode::OK),
        ];
        for (path, own_answer) in routes {
            let response = server.get(path).await;
            assert_eq!(response.status_code(), refusal.status_code(), "{path}");
            assert_eq!(response.text(), refusal.text(), "{path}");
            // Control: with the secret the route answers its own way, so
            // the refusal above was the lock and not a missing route
            let (name, value) = edge_header();
            let answered = server.get(path).add_header(name, value).await;
            assert_eq!(answered.status_code(), own_answer, "{path}");
        }
    }

    #[tokio::test]
    async fn r23_with_the_global_limiter_exhausted_each_route_gets_the_limiters_refusal() {
        let l = lists();
        for path in LIST_ROUTES {
            // Given a limiter whose bucket for this peer is spent
            let limiter = GlobalRateLimitState::new();
            let server = built_app(&l, limiter.clone());
            let (name, value) = edge_header();
            let control = server
                .get(path)
                .add_header(name.clone(), value.clone())
                .await;
            assert_ne!(control.status_code(), StatusCode::TOO_MANY_REQUESTS);
            let window = std::time::Duration::from_secs(60);
            while limiter.check("unknown", 60, window) {}
            // When the route is requested with the secret
            let response = server.get(path).add_header(name, value).await;
            // Then it is the limiter's refusal
            response.assert_status(StatusCode::TOO_MANY_REQUESTS);
            assert_eq!(
                header(&response, "retry-after").as_deref(),
                Some("60"),
                "{path}"
            );
        }
    }
}
