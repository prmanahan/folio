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
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{Extension, Router};
use rusqlite::Connection;
use time::OffsetDateTime;

use super::articles::ArticlesList;
use super::clock::{FormatError, SystemClock, WallClock, format_fetched_at};
use super::github::GithubProjectsSource;
use super::held::{Held, ListDocument, LiveLists};
use super::outbound::OutboundClient;
use super::refresher::{ListSource, Refresher, Sources};
use super::store::{DeleteOutcome, ListStore, NoData, SqliteStore, delete_row, load};
use super::switch::{DisabledLists, ParsedSwitch, parse_switch};
use super::{ListName, ReasonCode};
use crate::app::build_app;
use crate::auth;
use crate::config::Config;
use crate::db;
use crate::middleware::global_rate_limit::GlobalRateLimitState;
use crate::middleware::origin_lock::OriginLockState;
use crate::projects_list::{self, ProjectsList};
use crate::state::{AppState, DbState};

/// Sliding window used by `global_rate_limit_middleware` (60 req/min) and
/// the interval its stale buckets are purged on. Same value on purpose: an
/// entry can't go stale faster than the window it's measured against.
const GLOBAL_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

/// Everything `start` is given. Production fills it with
/// [`StartInputs::production`]. The sources and the store are not fields a
/// caller outside this crate can set: `start` builds the production ones
/// itself (R-2: no production override of a source), and only this crate's
/// tests inject fakes.
///
/// ```
/// # use site_core::config::Config;
/// # use site_core::live_lists::start::StartInputs;
/// # fn config() -> Config { Config { port: 0, database_url: String::new(), admin_password: String::new(), anthropic_api_key: None, trusted_ip_header: None, static_dir: String::new(), page_hit_salt: String::new(), edge_auth_token: String::new() } }
/// let inputs = StartInputs::production(config(), None, None);
/// let _ = inputs.cors_origin; // a public field is readable
/// ```
///
/// Reaching the sources from outside the crate does not compile (the control
/// above is the same program reading a public field):
///
/// ```compile_fail
/// # use site_core::config::Config;
/// # use site_core::live_lists::start::StartInputs;
/// # fn config() -> Config { Config { port: 0, database_url: String::new(), admin_password: String::new(), anthropic_api_key: None, trusted_ip_header: None, static_dir: String::new(), page_hit_salt: String::new(), edge_auth_token: String::new() } }
/// let inputs = StartInputs::production(config(), None, None);
/// let _ = &inputs.sources; // the field is not reachable, so it cannot be set
/// ```
pub struct StartInputs {
    pub config: Config,
    /// `CORS_ORIGIN`, as read; `None` when unset.
    pub cors_origin: Option<String>,
    /// `LIVE_LISTS_DISABLED` as the raw OS value (R-5, R-18), `None` when
    /// unset. A value that is not Unicode must reach the parser as such.
    pub switch: Option<OsString>,
    pub clock: Arc<dyn WallClock>,
    /// `None` builds the production sources inside `start` (R-2). Only this
    /// crate's tests set it.
    pub(crate) sources: Option<Sources>,
    /// The refresher's store. `None` opens the production store on
    /// `config.database_url`. Only this crate's tests set it.
    pub(crate) store: Option<Box<dyn ListStore>>,
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
            sources: None,
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
    /// `DATABASE_URL` names an in-memory database. The refresher's store is a
    /// second connection, which would see an empty database of its own.
    InMemoryDatabase,
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
    let StartInputs {
        config,
        cors_origin,
        switch,
        clock,
        sources,
        store,
    } = inputs;
    let sources = sources.unwrap_or_else(production_sources);

    // R-5 and R-17: a panic must never print its payload (it could hold
    // response text, R-30). A test build keeps libtest's hook.
    #[cfg(not(test))]
    super::panic_hook::install();

    // The refresher's store is a second connection, so an in-memory database
    // would give it an empty database of its own. Refuse loudly.
    if is_in_memory(&config.database_url) {
        tracing::error!("DATABASE_URL names an in-memory database; live lists need a file");
        return Err(StartError::InMemoryDatabase);
    }

    // Step 1 (R-18). The parser never fails: a bad value disables both lists
    // and says so here, at `error`. The value itself is never logged.
    let ParsedSwitch { disabled, problem } = parse_switch(switch.as_deref());
    if let Some(problem) = problem {
        tracing::error!(
            problem = problem.as_str(),
            "LIVE_LISTS_DISABLED is not a clean allowlist: both live lists are disabled"
        );
    }

    // Today's work, in today's order: it needs no database, so it runs
    // before step 2. R-10 (projects-from-github) fails closed before anything
    // is opened. No age check: a Fly restart must not depend on how long ago
    // the last deploy was (the release command checks freshness). Retired by
    // the task that moves the prompts onto the held copies.
    crate::static_files::validate_static_dir(&config.static_dir);
    let projects_path = std::path::Path::new(&config.static_dir).join("projects.json");
    let projects = match projects_list::load(&projects_path) {
        Ok(list) => Arc::new(list),
        Err(e) => {
            tracing::error!(path = %projects_path.display(), error = %e, remedy = "run `GITHUB_TOKEN=<token> just fetch-projects`, then rebuild the frontend", "refusing to start: projects list invalid");
            return Err(StartError::ProjectsListRefused);
        }
    };
    let counts: Vec<String> = projects.counts().map(|(o, n)| format!("{o}={n}")).collect();
    tracing::info!(
        path = %projects_path.display(),
        fetched_at = projects.fetched_at(),
        counts = counts.join(" "),
        "projects list loaded"
    );
    tracing::info!(port = config.port, "starting server");

    // Step 2.
    let conn = db::connect(&config.database_url).map_err(StartError::Database)?;

    // Step 3 (R-19): before any refresher write, and before the load, so a
    // disabled list's row cannot be read back.
    let deleted: Vec<(ListName, DeleteOutcome)> = ListName::ALL
        .into_iter()
        .filter(|list| disabled.contains(*list))
        .map(|list| (list, delete_row(&conn, list)))
        .collect();
    log_start_line(disabled, &deleted);

    // Step 4 (R-16): a disabled list is never loaded, whatever its delete did.
    let now = clock.now();
    let held_projects = load_enabled::<ProjectsList>(&conn, disabled, now);
    let held_articles = load_enabled::<ArticlesList>(&conn, disabled, now);

    // Step 5 (R-22): fixed here, before the first request can be served.
    let started_at = format_fetched_at(clock.now()).map_err(StartError::StartedAt)?;
    let live_lists = LiveLists::new(started_at);
    if let Some(held) = held_projects {
        live_lists.set_projects(held);
    }
    if let Some(held) = held_articles {
        live_lists.set_articles(held);
    }

    // Today's work, continued. Hash the admin password with Argon2id.
    let password_hash =
        auth::hash_password(&config.admin_password).map_err(StartError::PasswordHash)?;
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
        projects,
    });

    // LLM-audit Nit / R6: CORS_ORIGIN fail-loud. The old silent
    // `unwrap_or_else(|_| "http://localhost:3000")` made a missing prod
    // env var yield a wrong-origin policy that breaks the site without any
    // operator signal. WARN form (not require-in-prod) is chosen to match
    // the existing `PAGE_HIT_SALT` pattern and keep local/dev ergonomics
    // (dev has no CORS_ORIGIN; only `ADMIN_PASSWORD` is fail-loud). A
    // missing CORS_ORIGIN now emits a WARN naming the var before falling
    // back, so the misconfiguration is loud in the logs.
    let cors_origin = match cors_origin {
        Some(origin) => origin,
        None => {
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
    // window so an entry can't go stale faster than it's purged. Detached on
    // purpose: it lives as long as the process.
    let purge_state = global_rate_limit.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(GLOBAL_RATE_LIMIT_WINDOW);
        loop {
            interval.tick().await;
            purge_state.purge_stale(GLOBAL_RATE_LIMIT_WINDOW);
        }
    });

    // The handle the two `/api/lists/` routes read. Added outside the app so
    // `build_app`'s signature, and every caller of it, is unchanged.
    let app = build_app(
        db_state,
        origin_lock_state,
        global_rate_limit,
        &config.static_dir,
        &cors_origin,
    )
    .layer(Extension(live_lists.clone()));

    // Step 6 (R-17): the refresher runs off the startup path. Its store is a
    // connection of its own.
    let store = match store {
        Some(store) => store,
        None => Box::new(SqliteStore::open(&config.database_url).map_err(StartError::Store)?),
    };
    Refresher::new(live_lists, clock, sources, disabled, store).spawn();

    // Step 7.
    serve(app).await.map_err(StartError::Serve)
}

/// The sources a production start registers: the GitHub projects source. The
/// articles source is registered when it exists. They are built here, never
/// injected from outside the crate.
///
/// A client that cannot be built registers no projects source, and says so at
/// `error`. The enabled list then has no source, which makes every refresh's
/// summary a `warn` (R-30), so the gap does not pass as healthy.
fn production_sources() -> Sources {
    let projects = match OutboundClient::new() {
        Ok(client) => Some(Arc::new(GithubProjectsSource::new(client)) as Arc<dyn ListSource>),
        Err(error) => {
            tracing::error!(%error, "the outbound client could not be built; projects will not refresh");
            None
        }
    };
    Sources {
        projects,
        ..Sources::default()
    }
}

/// SQLite's in-memory forms: the empty name, `:memory:`, and any URI naming
/// `mode=memory` or `:memory:`.
fn is_in_memory(database_url: &str) -> bool {
    database_url.is_empty()
        || database_url.contains(":memory:")
        || database_url.contains("mode=memory")
}

/// R-19: one line per start, naming the parsed set and each disabled list's
/// delete outcome; `error` when any delete failed.
fn log_start_line(disabled: DisabledLists, deleted: &[(ListName, DeleteOutcome)]) {
    let outcome = |list: ListName| {
        deleted
            .iter()
            .find(|(l, _)| *l == list)
            .map(|(_, o)| tracing::field::display(o.as_str()))
    };
    let set = tracing::field::display(disabled.describe());
    let projects = outcome(ListName::Projects);
    let articles = outcome(ListName::Articles);
    if deleted.iter().any(|(_, o)| *o == DeleteOutcome::Failed) {
        tracing::error!(
            disabled = set,
            projects_delete = projects,
            articles_delete = articles,
            reason = ReasonCode::DeleteFailed.as_str(),
            "live lists start"
        );
    } else {
        tracing::info!(
            disabled = set,
            projects_delete = projects,
            articles_delete = articles,
            "live lists start"
        );
    }
}

/// Load `L` unless the switch disabled it, and say what happened. The reason
/// a list loads as no data yet is one of [`NoData`]'s fixed codes, never the
/// row's text.
fn load_enabled<L: ListDocument>(
    conn: &Connection,
    disabled: DisabledLists,
    now: OffsetDateTime,
) -> Option<Arc<Held<L>>> {
    if disabled.contains(L::NAME) {
        return None;
    }
    match load::<L>(conn, now) {
        Ok(held) => {
            tracing::info!(
                list = L::NAME.as_str(),
                fetched_at = held.fetched_at(),
                count = held.list().count(),
                "live list loaded from the store"
            );
            Some(Arc::new(held))
        }
        Err(no_data) => {
            // A missing row is ordinary (first start, or just disabled); a row
            // that exists and cannot be used is an anomaly worth a warn.
            match no_data {
                NoData::Absent => tracing::info!(
                    list = L::NAME.as_str(),
                    reason = no_data.as_str(),
                    "live list has no data yet"
                ),
                NoData::Invalid | NoData::Unreadable | NoData::AheadOfClock => tracing::warn!(
                    list = L::NAME.as_str(),
                    reason = no_data.as_str(),
                    "live list has no data yet"
                ),
            }
            None
        }
    }
}

/// The production serve step: bind `0.0.0.0:port` and serve with connect info.
///
/// # Panics
/// When the port cannot be bound, as `main.rs` did.
pub async fn serve_tcp(port: u16, app: Router) -> io::Result<()> {
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("Failed to bind to port");
    tracing::info!(port, "listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
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
        // a clean value raises no error line at all, and the start line is `info`
        assert!(log.lines_containing("ERROR").is_empty(), "{}", log.text());
        assert!(log.one_line(START_LINE).contains("INFO"));
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
    async fn a_start_logs_no_error_line_when_nothing_is_wrong() {
        let env = Env::new();
        let (log, _guard) = LogCapture::install();
        launch(env.inputs(None, Sources::default())).await;
        assert!(log.lines_containing("ERROR").is_empty(), "{}", log.text());
    }

    async fn load_line_for(row: &str) -> String {
        let env = Env::new();
        env.seed(ListName::Projects, row);
        let (log, _guard) = LogCapture::install();
        launch(env.inputs(None, Sources::default())).await;
        log.lines_containing("live list has no data yet")
            .into_iter()
            .find(|l| l.contains(r#"list="projects""#))
            .unwrap_or_else(|| panic!("no projects line in {}", log.text()))
    }

    #[tokio::test]
    async fn an_invalid_row_loading_as_no_data_logs_at_warn() {
        let line = load_line_for("not json").await;
        assert!(
            line.contains("WARN") && line.contains(r#"reason="invalid""#),
            "{line}"
        );
    }

    #[tokio::test]
    async fn a_row_ahead_of_the_clock_loading_as_no_data_logs_at_warn() {
        let env = Env::new();
        let beyond = ts::format(env.clock.now() + MAX_AGE_AHEAD + Duration::milliseconds(1));
        let line = load_line_for(&projects_doc(&beyond)).await;
        assert!(
            line.contains("WARN") && line.contains(r#"reason="ahead_of_clock""#),
            "{line}"
        );
    }

    #[tokio::test]
    async fn a_missing_row_is_ordinary_and_logs_at_info() {
        let env = Env::new();
        let (log, _guard) = LogCapture::install();
        launch(env.inputs(None, Sources::default())).await;
        let line = log
            .lines_containing("live list has no data yet")
            .into_iter()
            .find(|l| l.contains(r#"list="projects""#))
            .expect("a projects line");
        assert!(
            line.contains("INFO") && line.contains(r#"reason="absent""#),
            "{line}"
        );
    }

    #[tokio::test]
    async fn an_in_memory_database_url_refuses_the_start_before_opening_anything() {
        for url in [
            ":memory:",
            "",
            "file::memory:",
            "file:x?mode=memory&cache=shared",
        ] {
            let env = Env::new();
            let mut inputs = env.inputs(None, Sources::default());
            inputs.config.database_url = url.to_string();
            let result = try_launch(inputs).await;
            assert!(
                matches!(result, Err(StartError::InMemoryDatabase)),
                "{url:?}: {:?}",
                result.err()
            );
        }
    }
}
