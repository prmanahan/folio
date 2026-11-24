//! R-5: the one start function.
//!
//! It runs the whole startup sequence in order: (1) parse the switch, (2)
//! open the database and migrate, (3) delete the disabled lists' rows, (4)
//! load and re-validate the rest, (5) fix `started_at`, (6) spawn the
//! refresher without awaiting it, (7) serve. The work `main.rs` did before
//! (static-directory check, `projects.json` load and refusal, admin password
//! hash, AI client, CORS and origin-lock state, rate-limit purge task) runs
//! inside it in its old relative order, before step 6.
//!
//! The serve step is a parameter: production passes [`serve_tcp`]; tests pass
//! a step that hands them the router. `main.rs` only reads configuration and
//! calls this; that is review-checked.

use std::ffi::OsString;
use std::future::Future;
use std::io;
use std::sync::Arc;

use axum::Router;

use super::clock::{FormatError, SystemClock, WallClock};
use super::refresher::Sources;
use super::store::ListStore;
use crate::config::Config;

/// Everything `start` is given. Production fills it with
/// [`StartInputs::production`]; tests fill the same fields with fakes.
pub struct StartInputs {
    pub config: Config,
    /// `CORS_ORIGIN`, as read; `None` when unset.
    pub cors_origin: Option<String>,
    /// `LIVE_LISTS_DISABLED` as the raw OS value (R-5, R-18), `None` when
    /// unset. A value that is not Unicode must reach the parser as such.
    pub switch: Option<OsString>,
    pub clock: Arc<dyn WallClock>,
    pub sources: Sources,
    /// The refresher's store. `None` opens the production store on
    /// `config.database_url`.
    pub store: Option<Box<dyn ListStore>>,
}

impl StartInputs {
    /// The production inputs: the system clock, no sources registered yet
    /// (the projects and articles sources plug in behind this seam), and the
    /// production store.
    pub fn production(
        config: Config,
        cors_origin: Option<String>,
        switch: Option<OsString>,
    ) -> Self {
        Self {
            config,
            cors_origin,
            switch,
            clock: Arc::new(SystemClock),
            sources: Sources::default(),
            store: None,
        }
    }
}

/// Why `start` ended before serving, or why serving ended.
#[derive(Debug)]
pub enum StartError {
    /// R-10 (projects-from-github): `projects.json` is missing or invalid.
    /// The reason is logged naming the path; the process exits 1.
    ProjectsListRefused,
    Database(rusqlite::Error),
    PasswordHash(crate::error::AppError),
    StartedAt(FormatError),
    Store(rusqlite::Error),
    Serve(io::Error),
}

/// Run the startup sequence, then `serve` the app.
///
/// # Errors
/// [`StartError`] for the step that failed; panics only where `main.rs`
/// panicked before (an invalid static directory, a bad `CORS_ORIGIN`).
pub async fn start<S, Fut>(inputs: StartInputs, serve: S) -> Result<(), StartError>
where
    S: FnOnce(Router) -> Fut + Send,
    Fut: Future<Output = io::Result<()>> + Send,
{
    let _ = inputs;
    serve(Router::new()).await.map_err(StartError::Serve)
}

/// The production serve step: bind `0.0.0.0:port` and serve with connect info.
///
/// # Panics
/// When the port cannot be bound, as `main.rs` did.
pub async fn serve_tcp(port: u16, app: Router) -> io::Result<()> {
    let _ = (port, app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::http::StatusCode;
    use serde_json::{Value, json};
    use time::Duration;

    use crate::live_lists::ListName;
    use crate::live_lists::harness::{Env, LogCapture, launch, try_launch};
    use crate::live_lists::test_support::{self as ts, T0, articles_doc, projects_doc};
    use crate::projects_list::MAX_AGE_AHEAD;

    const PROJECTS: &str = "/api/lists/projects";
    const FRESHNESS: &str = "/api/lists/freshness";
    const START_LINE: &str = "live lists start";

    fn seed_both(env: &Env) -> (String, String) {
        let (p, a) = (projects_doc(T0), articles_doc(T0));
        env.seed(ListName::Projects, &p);
        env.seed(ListName::Articles, &a);
        (p, a)
    }

    // ---- R-16 through the start function ----

    #[tokio::test]
    async fn r16_against_a_pre_seeded_store_the_first_request_receives_the_seeded_copies() {
        // Given a store holding both lists
        let env = Env::new();
        let (p, a) = seed_both(&env);
        // When the server starts and the first request is a read
        let running = launch(env.inputs(None, Sources::default())).await;
        let response = running.get(&env, PROJECTS).await;
        // Then it gets the seeded projects document, byte for byte
        response.assert_status(StatusCode::OK);
        assert_eq!(response.text(), p);
        // and freshness reports the seeded times
        let fresh = running.get_json(&env, FRESHNESS, StatusCode::OK).await;
        assert_eq!(fresh["projects"], json!(T0));
        assert_eq!(fresh["articles"], json!(T0));
        // and the stored articles row is the one that was seeded
        assert_eq!(env.stored(ListName::Articles), Some(a));
    }

    #[tokio::test]
    async fn r16_a_row_edited_to_break_validation_loads_as_no_data_yet() {
        let env = Env::new();
        seed_both(&env);
        env.seed(
            ListName::Projects,
            &projects_doc(T0).replace("alpha", "al pha"),
        );
        let running = launch(env.inputs(None, Sources::default())).await;
        running
            .get(&env, PROJECTS)
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
        let fresh = running.get_json(&env, FRESHNESS, StatusCode::OK).await;
        assert_eq!(fresh["projects"], Value::Null);
        // the other list is unaffected
        assert_eq!(fresh["articles"], json!(T0));
    }

    #[tokio::test]
    async fn r16_a_row_dated_exactly_the_skew_allowance_ahead_loads() {
        let env = Env::new();
        let at_bound = ts::format(env.clock.now() + MAX_AGE_AHEAD);
        env.seed(ListName::Projects, &projects_doc(&at_bound));
        let running = launch(env.inputs(None, Sources::default())).await;
        running
            .get(&env, PROJECTS)
            .await
            .assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn r16_a_row_dated_one_millisecond_beyond_the_skew_allowance_is_no_data_yet() {
        let env = Env::new();
        let beyond = ts::format(env.clock.now() + MAX_AGE_AHEAD + Duration::milliseconds(1));
        env.seed(ListName::Projects, &projects_doc(&beyond));
        let running = launch(env.inputs(None, Sources::default())).await;
        running
            .get(&env, PROJECTS)
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn r16_health_answers_200_whatever_the_rows_hold() {
        // Given rows that are not valid documents
        let env = Env::new();
        env.seed(ListName::Projects, "not json");
        env.seed(ListName::Articles, "{}");
        // When the server starts
        let running = launch(env.inputs(None, Sources::default())).await;
        // Then health answers 200 and both lists read as no data yet
        let health = running.server.get("/api/health").await;
        health.assert_status(StatusCode::OK);
        assert_eq!(health.text(), "ok");
        let fresh = running.get_json(&env, FRESHNESS, StatusCode::OK).await;
        assert_eq!(
            (&fresh["projects"], &fresh["articles"]),
            (&Value::Null, &Value::Null)
        );
    }

    // ---- R-18, R-19: the switch scenarios through the start function ----

    #[tokio::test]
    async fn r18_articles_disabled_deletes_that_row_and_projects_behave_normally() {
        // Given LIVE_LISTS_DISABLED=articles and stored rows for both lists
        let env = Env::new();
        let (p, _) = seed_both(&env);
        let (log, _guard) = LogCapture::install();
        // When the server starts
        let running = launch(env.inputs(Some("articles"), Sources::default())).await;
        // Then the articles row is deleted and the start line records it
        assert_eq!(env.stored(ListName::Articles), None);
        let line = log.one_line(START_LINE);
        assert!(line.contains("INFO"), "{line}");
        assert!(line.contains("disabled={articles}"), "{line}");
        assert!(line.contains("articles_delete=deleted"), "{line}");
        assert!(!line.contains("projects_delete"), "{line}");
        // the freshness read reports articles as null, projects as held
        let fresh = running.get_json(&env, FRESHNESS, StatusCode::OK).await;
        assert_eq!(fresh["articles"], Value::Null);
        assert_eq!(fresh["projects"], json!(T0));
        assert_eq!(running.get(&env, PROJECTS).await.text(), p);
        assert_eq!(env.stored(ListName::Projects), Some(p));
    }

    #[tokio::test]
    async fn r18_projects_disabled_deletes_that_row_and_the_route_answers_503() {
        let env = Env::new();
        let (_, a) = seed_both(&env);
        let (log, _guard) = LogCapture::install();
        let running = launch(env.inputs(Some("projects"), Sources::default())).await;
        assert_eq!(env.stored(ListName::Projects), None);
        let line = log.one_line(START_LINE);
        assert!(line.contains("disabled={projects}"), "{line}");
        assert!(line.contains("projects_delete=deleted"), "{line}");
        running
            .get(&env, PROJECTS)
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
        let fresh = running.get_json(&env, FRESHNESS, StatusCode::OK).await;
        assert_eq!(fresh["projects"], Value::Null);
        assert_eq!(fresh["articles"], json!(T0));
        assert_eq!(env.stored(ListName::Articles), Some(a));
    }

    #[tokio::test]
    async fn r18_both_disabled_deletes_both_rows_and_serves_neither() {
        let env = Env::new();
        seed_both(&env);
        let (log, _guard) = LogCapture::install();
        let running = launch(env.inputs(Some("projects,articles"), Sources::default())).await;
        assert_eq!(env.stored(ListName::Projects), None);
        assert_eq!(env.stored(ListName::Articles), None);
        let line = log.one_line(START_LINE);
        assert!(line.contains("disabled={projects,articles}"), "{line}");
        assert!(line.contains("projects_delete=deleted"), "{line}");
        assert!(line.contains("articles_delete=deleted"), "{line}");
        running
            .get(&env, PROJECTS)
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
        let fresh = running.get_json(&env, FRESHNESS, StatusCode::OK).await;
        assert_eq!(
            (&fresh["projects"], &fresh["articles"]),
            (&Value::Null, &Value::Null)
        );
    }

    /// An unparseable switch: both rows deleted, neither list served, an
    /// `error` line, and the server still starts.
    async fn assert_both_disabled_with_an_error_line(switch: Option<OsString>) {
        let env = Env::new();
        seed_both(&env);
        let (log, _guard) = LogCapture::install();
        let running = launch(env.inputs_raw(switch, Sources::default())).await;
        assert_eq!(env.stored(ListName::Projects), None);
        assert_eq!(env.stored(ListName::Articles), None);
        let errors: Vec<String> = log
            .lines_containing("LIVE_LISTS_DISABLED")
            .into_iter()
            .filter(|l| l.contains("ERROR"))
            .collect();
        assert_eq!(errors.len(), 1, "{}", log.text());
        assert!(
            log.one_line(START_LINE)
                .contains("disabled={projects,articles}")
        );
        running
            .get(&env, PROJECTS)
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
        let fresh = running.get_json(&env, FRESHNESS, StatusCode::OK).await;
        assert_eq!(
            (&fresh["projects"], &fresh["articles"]),
            (&Value::Null, &Value::Null)
        );
        running
            .server
            .get("/api/health")
            .await
            .assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn r18_a_valid_token_beside_an_unknown_one_disables_both() {
        assert_both_disabled_with_an_error_line(Some("articles,projcts".into())).await;
    }

    #[tokio::test]
    async fn r18_a_miscased_token_disables_both() {
        assert_both_disabled_with_an_error_line(Some("Articles".into())).await;
    }

    #[tokio::test]
    async fn r18_a_misspelled_token_disables_both() {
        assert_both_disabled_with_an_error_line(Some("projcts".into())).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn r18_a_value_that_is_not_unicode_disables_both() {
        use std::os::unix::ffi::OsStringExt;
        assert_both_disabled_with_an_error_line(Some(OsString::from_vec(b"articles\xff".to_vec())))
            .await;
    }

    #[tokio::test]
    async fn r18_a_padded_token_with_a_trailing_comma_disables_only_that_list() {
        let env = Env::new();
        seed_both(&env);
        let (log, _guard) = LogCapture::install();
        launch(env.inputs(Some(" articles ,"), Sources::default())).await;
        assert_eq!(env.stored(ListName::Articles), None);
        assert!(env.stored(ListName::Projects).is_some());
        assert!(log.one_line(START_LINE).contains("disabled={articles}"));
        assert!(log.lines_containing("ERROR").is_empty(), "{}", log.text());
    }

    async fn assert_nothing_disabled(switch: Option<&str>) {
        let env = Env::new();
        seed_both(&env);
        let (log, _guard) = LogCapture::install();
        let running = launch(env.inputs(switch, Sources::default())).await;
        assert!(env.stored(ListName::Projects).is_some());
        assert!(env.stored(ListName::Articles).is_some());
        let line = log.one_line(START_LINE);
        assert!(line.contains("disabled={}"), "{line}");
        assert!(!line.contains("_delete"), "{line}");
        running
            .get(&env, PROJECTS)
            .await
            .assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn r18_an_absent_variable_disables_nothing() {
        assert_nothing_disabled(None).await;
    }

    #[tokio::test]
    async fn r18_an_empty_value_disables_nothing() {
        assert_nothing_disabled(Some("")).await;
    }

    #[tokio::test]
    async fn r18_a_whitespace_only_value_disables_nothing() {
        assert_nothing_disabled(Some("  \t ")).await;
    }

    #[tokio::test]
    async fn r19_a_disabled_list_with_no_row_records_absent() {
        let env = Env::new();
        let (log, _guard) = LogCapture::install();
        launch(env.inputs(Some("projects"), Sources::default())).await;
        assert!(log.one_line(START_LINE).contains("projects_delete=absent"));
    }

    #[tokio::test]
    async fn r19_with_the_delete_forced_to_fail_the_list_is_neither_loaded_nor_served() {
        // Given a valid stored projects row whose delete the database refuses
        let env = Env::new();
        let (p, _) = seed_both(&env);
        env.db()
            .execute_batch(
                "CREATE TRIGGER refuse_delete BEFORE DELETE ON live_lists \
                 BEGIN SELECT RAISE(ABORT, 'forced'); END;",
            )
            .unwrap();
        let (log, _guard) = LogCapture::install();
        // When the server starts with projects disabled
        let running = launch(env.inputs(Some("projects"), Sources::default())).await;
        // Then the row is still stored, but the list is not loaded or served
        assert_eq!(env.stored(ListName::Projects), Some(p));
        running
            .get(&env, PROJECTS)
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
        let fresh = running.get_json(&env, FRESHNESS, StatusCode::OK).await;
        assert_eq!(fresh["projects"], Value::Null);
        // and the start line records `failed` at error
        let line = log.one_line(START_LINE);
        assert!(line.contains("ERROR"), "{line}");
        assert!(line.contains("projects_delete=failed"), "{line}");
        assert!(line.contains("delete_failed"), "{line}");
    }

    // ---- R-22: started_at is fixed at step 5, before serving ----

    #[tokio::test]
    async fn r22_started_at_lies_between_the_instants_before_the_call_and_when_serving_began() {
        // Given two reads of the injected wall clock: one before the call
        let env = Env::new();
        let before = env.clock.now();
        let running = launch(env.inputs(None, Sources::default())).await;
        // and one when serving began, then the clock moves on
        let began = running.serving_began_at;
        env.clock.advance(Duration::hours(1));
        // When the first request reads freshness
        let fresh = running.get_json(&env, FRESHNESS, StatusCode::OK).await;
        // Then started_at lies between the two instants, so a value set
        // lazily on the first request (an hour later) would fail
        let started = ts::instant(fresh["started_at"].as_str().expect("a string"));
        assert!(
            before <= started && started <= began,
            "{before} {started} {began}"
        );
    }

    #[tokio::test]
    async fn r22_started_at_is_the_same_on_every_request() {
        let env = Env::new();
        let running = launch(env.inputs(None, Sources::default())).await;
        let first = running.get_json(&env, FRESHNESS, StatusCode::OK).await["started_at"].clone();
        env.clock.advance(Duration::minutes(7));
        let second = running.get_json(&env, FRESHNESS, StatusCode::OK).await["started_at"].clone();
        assert_eq!(first, second);
    }

    // ---- "today's other startup work" stays, in its old place ----

    #[tokio::test]
    async fn an_invalid_projects_json_refuses_the_start_naming_the_path_before_serving() {
        let env = Env::new();
        std::fs::write(env.projects_json_path(), "{}").unwrap();
        let (log, _guard) = LogCapture::install();
        let result = try_launch(env.inputs(None, Sources::default())).await;
        assert!(matches!(result, Err(StartError::ProjectsListRefused)));
        let line = log.one_line("refusing to start: projects list invalid");
        assert!(line.contains("ERROR"), "{line}");
        assert!(
            line.contains(env.projects_json_path().to_str().unwrap()),
            "{line}"
        );
        // refused before the database was opened or a list deleted
        assert!(!env.db_path().exists());
    }

    #[tokio::test]
    async fn the_start_line_is_logged_once_per_start() {
        let env = Env::new();
        let (log, _guard) = LogCapture::install();
        let running = launch(env.inputs(None, Sources::default())).await;
        assert!(running.is_serving());
        assert_eq!(log.lines_containing(START_LINE).len(), 1);
    }

    #[tokio::test]
    async fn a_start_over_a_populated_database_keeps_unrelated_tables() {
        // migration 008 runs against a database the earlier migrations filled
        let env = Env::new();
        env.db()
            .execute(
                "INSERT INTO site_config (key, value) VALUES ('probe.key', 'probe-value')",
                [],
            )
            .unwrap();
        launch(env.inputs(None, Sources::default())).await;
        let value: String = env
            .db()
            .query_row(
                "SELECT value FROM site_config WHERE key = 'probe.key'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(value, "probe-value");
    }
}
