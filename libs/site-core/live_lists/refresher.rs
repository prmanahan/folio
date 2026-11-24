//! The refresher (R-15, R-17, R-30): one background task that fetches each
//! enabled list from its injected source at start and then every 6 hours,
//! validates, persists, then swaps in the held copy.
//!
//! The sources are the seam. Production registers the GitHub and feed
//! sources through [`Sources`]; tests register fakes. No request handler
//! holds a handle that can start a refresh: the only owner of a
//! [`Refresher`] is the task [`Refresher::spawn`] starts.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use time::OffsetDateTime;

use super::articles::ArticlesList;
use super::clock::WallClock;
use super::held::{Held, ListDocument, LiveLists, Rejected};
use super::store::ListStore;
use super::switch::DisabledLists;
use super::{ListName, ReasonCode};
use crate::projects_list::ProjectsList;

/// R-17: the 6-hour tick. Not a parameter; tests advance the tokio test clock.
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// R-4: a list whose fetch has not finished when this passes is a failed
/// fetch for that refresh.
pub const REFRESH_BOUND: Duration = Duration::from_secs(120);

#[cfg(test)]
thread_local! {
    static BOUND_FOR_TEST: std::cell::Cell<Option<Duration>> = const { std::cell::Cell::new(None) };
}

/// Make this thread's refreshes use `bound` as the refresh bound. Test-only,
/// per thread, so no test affects another; the refresher task of a
/// current-thread test runtime runs on the test's thread.
#[cfg(test)]
pub(crate) fn set_bound_for_test(bound: Duration) {
    BOUND_FOR_TEST.with(|b| b.set(Some(bound)));
}

/// A post a source rejected, by its item index and reason code only (R-30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkippedPost {
    /// Zero-based: the position among the feed's direct `<item>` children, in
    /// document order.
    pub index: usize,
    pub reason: ReasonCode,
}

/// One outbound request's outcome, for the summary line (R-30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestNote {
    pub status: u16,
    /// GitHub's `x-ratelimit-remaining`, only when it parsed as an integer.
    pub ratelimit_remaining: Option<i64>,
}

/// What one fetch of one list produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchReport {
    /// The list's complete document as JSON text, `fetched_at` included, or
    /// why there is none. The refresher validates it; a source need not.
    pub document: Result<String, ReasonCode>,
    pub skipped: Vec<SkippedPost>,
    pub requests: Vec<RequestNote>,
}

impl FetchReport {
    pub fn document(text: String) -> Self {
        Self {
            document: Ok(text),
            skipped: Vec::new(),
            requests: Vec::new(),
        }
    }

    pub fn failed(reason: ReasonCode) -> Self {
        Self {
            document: Err(reason),
            skipped: Vec::new(),
            requests: Vec::new(),
        }
    }
}

/// A list's source: GitHub for projects, the feed for articles.
pub trait ListSource: Send + Sync {
    /// The requests a fetch had completed when the refresh bound cancelled
    /// it, so the summary line still carries them (R-30). A source that makes
    /// more than one request overrides this; one request cannot outlast the
    /// bound, so the default is none.
    fn cancelled_requests(&self) -> Vec<RequestNote> {
        Vec::new()
    }

    /// Fetch the list once. `now` is the wall clock's instant, for the
    /// document's `fetched_at`. There is no retry inside a refresh (R-4).
    fn fetch(&self, now: OffsetDateTime) -> BoxFuture<'_, FetchReport>;
}

/// The sources registered for a start. A list with none is never fetched.
#[derive(Clone, Default)]
pub struct Sources {
    pub projects: Option<Arc<dyn ListSource>>,
    pub articles: Option<Arc<dyn ListSource>>,
}

impl Sources {
    fn get(&self, list: ListName) -> Option<&Arc<dyn ListSource>> {
        match list {
            ListName::Projects => self.projects.as_ref(),
            ListName::Articles => self.articles.as_ref(),
        }
    }
}

/// What started a refresh (R-30's summary line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Start,
    Tick,
}

impl Trigger {
    pub const fn as_str(self) -> &'static str {
        match self {
            Trigger::Start => "start",
            Trigger::Tick => "tick",
        }
    }
}

pub struct Refresher {
    lists: LiveLists,
    clock: Arc<dyn WallClock>,
    sources: Sources,
    disabled: DisabledLists,
    store: Box<dyn ListStore>,
}

impl Refresher {
    pub fn new(
        lists: LiveLists,
        clock: Arc<dyn WallClock>,
        sources: Sources,
        disabled: DisabledLists,
        store: Box<dyn ListStore>,
    ) -> Self {
        Self {
            lists,
            clock,
            sources,
            disabled,
            store,
        }
    }

    /// Start the refresher without awaiting it (R-5 step 6), and watch it.
    pub fn spawn(self) {
        let task = tokio::spawn(self.run());
        // Detached on purpose: this watcher lives exactly as long as the
        // refresher and has nothing to return. It exists so the refresher's
        // end is never silent (R-17). The panic payload is not logged: it
        // could hold text from a response body (R-30).
        tokio::spawn(async move {
            match task.await {
                Ok(never) => match never {},
                Err(ended) => tracing::error!(
                    panicked = ended.is_panic(),
                    "live lists refresher ended; the held copies stay as they were and nothing will refresh them"
                ),
            }
        });
    }

    /// One refresh at start, then one every [`REFRESH_INTERVAL`]. A single
    /// task running one refresh at a time is the single flight (R-17): a tick
    /// that falls during a refresh is skipped, never run beside it.
    async fn run(mut self) -> Infallible {
        let mut interval = tokio::time::interval(REFRESH_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut trigger = Trigger::Start;
        loop {
            interval.tick().await;
            self.refresh(trigger).await;
            trigger = Trigger::Tick;
        }
    }

    /// Fetch every enabled list at once, each bounded by [`REFRESH_BOUND`],
    /// then commit each list on its own: one list failing never moves the
    /// other's time (R-17).
    async fn refresh(&mut self, trigger: Trigger) {
        let now = self.clock.now();
        // Owned inputs, not `&self`: the store is `Send` but not `Sync`, so
        // a borrow of the whole refresher cannot cross an await in a task.
        let fetch_of = |list: ListName| {
            fetch(
                self.disabled.contains(list),
                self.sources.get(list).cloned(),
                now,
            )
        };
        let (projects, articles) =
            futures::join!(fetch_of(ListName::Projects), fetch_of(ListName::Articles));
        let outcomes = [
            self.commit(ListName::Projects, projects),
            self.commit(ListName::Articles, articles),
        ];
        log_summary(trigger, &outcomes);
    }

    /// Validate, persist, then swap, in that order (R-15). Returns what to
    /// log. No lock is held here: the held-copy lock is taken only inside the
    /// swap, and the store is this task's own.
    ///
    /// The list's elapsed time is its own fetch plus its own commit (R-30),
    /// not the whole refresh's.
    fn commit(&mut self, list: ListName, (fetch, fetch_elapsed): (Fetch, Duration)) -> ListOutcome {
        let commit_began = tokio::time::Instant::now();
        let (document, skipped, requests) = match fetch {
            Fetch::Disabled => return ListOutcome::idle(list, Outcome::Disabled),
            Fetch::NoSource => return ListOutcome::idle(list, Outcome::NoSource),
            Fetch::BoundPassed(notes) => (Err(ReasonCode::RefreshBoundPassed), vec![], notes),
            Fetch::Reported(r) => (r.document, r.skipped, r.requests),
        };
        let result = document.and_then(|text| match list {
            ListName::Projects => self.swap_in::<ProjectsList>(
                text,
                ReasonCode::ProjectsDocumentRejected,
                LiveLists::set_projects,
            ),
            ListName::Articles => self.swap_in::<ArticlesList>(
                text,
                ReasonCode::ArticlesDocumentRejected,
                LiveLists::set_articles,
            ),
        });
        ListOutcome {
            list,
            outcome: match result {
                Ok(count) => Outcome::Advanced { count },
                Err(reason) => Outcome::Failed(reason),
            },
            skipped,
            requests,
            elapsed: fetch_elapsed + commit_began.elapsed(),
        }
    }

    fn swap_in<L: ListDocument>(
        &mut self,
        text: String,
        rejected: ReasonCode,
        swap: fn(&LiveLists, Arc<Held<L>>),
    ) -> Result<usize, ReasonCode> {
        let held = Held::<L>::new(text).map_err(|Rejected| rejected)?;
        // The cause is dropped on purpose: the log carries the reason code
        // only (R-30).
        self.store
            .persist(L::NAME, held.json())
            .map_err(|_cause| ReasonCode::PersistFailed)?;
        let count = held.list().count();
        swap(&self.lists, Arc::new(held));
        Ok(count)
    }
}

/// One list's fetch, bounded by [`REFRESH_BOUND`]. A disabled list is never
/// fetched (R-19).
async fn fetch(
    disabled: bool,
    source: Option<Arc<dyn ListSource>>,
    now: OffsetDateTime,
) -> (Fetch, Duration) {
    if disabled {
        return (Fetch::Disabled, Duration::ZERO);
    }
    let Some(source) = source else {
        return (Fetch::NoSource, Duration::ZERO);
    };
    let began = tokio::time::Instant::now();
    let fetched = match tokio::time::timeout(refresh_bound(), source.fetch(now)).await {
        Ok(report) => Fetch::Reported(report),
        Err(_elapsed) => Fetch::BoundPassed(source.cancelled_requests()),
    };
    (fetched, began.elapsed())
}

/// The bound a refresh uses: [`REFRESH_BOUND`], except in a test that set one.
fn refresh_bound() -> Duration {
    #[cfg(test)]
    if let Some(bound) = BOUND_FOR_TEST.with(std::cell::Cell::get) {
        return bound;
    }
    REFRESH_BOUND
}

/// What a list's fetch produced, before commit.
enum Fetch {
    Disabled,
    NoSource,
    /// The bound passed; the requests the cancelled fetch had completed.
    BoundPassed(Vec<RequestNote>),
    Reported(FetchReport),
}

enum Outcome {
    Advanced { count: usize },
    Failed(ReasonCode),
    Disabled,
    NoSource,
}

struct ListOutcome {
    list: ListName,
    outcome: Outcome,
    skipped: Vec<SkippedPost>,
    requests: Vec<RequestNote>,
    elapsed: Duration,
}

impl ListOutcome {
    fn idle(list: ListName, outcome: Outcome) -> Self {
        Self {
            list,
            outcome,
            skipped: Vec::new(),
            requests: Vec::new(),
            elapsed: Duration::ZERO,
        }
    }

    /// `outcome=advanced count=3 skipped=2:reason elapsed_ms=12`, one list's
    /// part of the summary line.
    fn describe(&self) -> String {
        let skipped = if self.skipped.is_empty() {
            "none".to_string()
        } else {
            self.skipped
                .iter()
                .map(|s| format!("{}:{}", s.index, s.reason.as_str()))
                .collect::<Vec<_>>()
                .join(",")
        };
        let elapsed_ms = self.elapsed.as_millis();
        match &self.outcome {
            Outcome::Advanced { count } => {
                format!("outcome=advanced count={count} skipped={skipped} elapsed_ms={elapsed_ms}")
            }
            Outcome::Failed(reason) => format!(
                "outcome=failed reason={} count=- skipped={skipped} elapsed_ms={elapsed_ms}",
                reason.as_str()
            ),
            Outcome::Disabled => "outcome=disabled".to_string(),
            Outcome::NoSource => "outcome=no_source".to_string(),
        }
    }
}

/// The per-list lines and the one summary line of a refresh (R-30). The
/// summary is `info` when every enabled list succeeded and `warn` otherwise
/// (an enabled list with no source did not succeed; a disabled list does not
/// count). It is
/// written on every refresh and never rate-limited.
fn log_summary(trigger: Trigger, outcomes: &[ListOutcome; 2]) {
    for o in outcomes {
        if let Outcome::Failed(reason) = &o.outcome {
            tracing::warn!(
                list = o.list.as_str(),
                reason = reason.as_str(),
                "live list refresh failed"
            );
        }
        for skip in &o.skipped {
            tracing::warn!(
                list = o.list.as_str(),
                index = skip.index,
                reason = skip.reason.as_str(),
                "post skipped"
            );
        }
    }
    let describe = |list: ListName| {
        outcomes
            .iter()
            .find(|o| o.list == list)
            .map(ListOutcome::describe)
            .unwrap_or_default()
    };
    let (projects, articles) = (describe(ListName::Projects), describe(ListName::Articles));
    let requests = join_notes(outcomes, |n| Some(n.status.to_string()));
    let remaining = join_notes(outcomes, |n| n.ratelimit_remaining.map(|r| r.to_string()));
    // A disabled list was not attempted and does not count. An enabled list
    // with no source did not succeed, so it makes the line `warn` (R-30).
    let all_ok = outcomes
        .iter()
        .all(|o| !matches!(o.outcome, Outcome::Failed(_) | Outcome::NoSource));
    let trigger = tracing::field::display(trigger.as_str());
    if all_ok {
        tracing::info!(
            trigger,
            projects,
            articles,
            requests,
            ratelimit_remaining = remaining,
            "live lists refresh"
        );
    } else {
        tracing::warn!(
            trigger,
            projects,
            articles,
            requests,
            ratelimit_remaining = remaining,
            "live lists refresh"
        );
    }
}

/// `projects:200,projects:200,articles:200` for the notes `pick` keeps, or
/// `None` when it keeps none, so the field is absent rather than empty.
fn join_notes(
    outcomes: &[ListOutcome; 2],
    pick: impl Fn(&RequestNote) -> Option<String>,
) -> Option<String> {
    let parts: Vec<String> = outcomes
        .iter()
        .flat_map(|o| {
            o.requests
                .iter()
                .filter_map(&pick)
                .map(|v| format!("{}:{v}", o.list.as_str()))
                .collect::<Vec<_>>()
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(","))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use axum::http::StatusCode;
    use serde_json::{Value, json};

    use super::*;
    use crate::live_lists::ListName;
    use crate::live_lists::harness::{Env, LogCapture, Running, launch, settle};
    use crate::live_lists::test_support::{
        self as ts, FakeSource, PanickingStore, T0, articles_doc, projects_doc, projects_doc_with,
    };

    const PROJECTS: &str = "/api/lists/projects";
    const FRESHNESS: &str = "/api/lists/freshness";
    const T1: &str = "2026-11-19T18:00:00.000Z";
    const T2: &str = "2026-11-20T00:00:00.000Z";

    fn sources(p: Option<&Arc<FakeSource>>, a: Option<&Arc<FakeSource>>) -> Sources {
        Sources {
            projects: p.map(|s| s.clone() as Arc<dyn ListSource>),
            articles: a.map(|s| s.clone() as Arc<dyn ListSource>),
        }
    }

    async fn freshness(running: &Running, env: &Env) -> Value {
        running.get_json(env, FRESHNESS, StatusCode::OK).await
    }

    // ---- R-5 / R-16: the refresh runs off the startup path ----

    #[tokio::test(start_paused = true)]
    async fn r5_against_a_source_that_never_answers_the_first_request_is_served_before_the_first_refresh_completes()
     {
        // Given a projects source that never answers
        let env = Env::new();
        let silent = FakeSource::never_answering();
        // When the server starts
        let running = launch(env.inputs(None, sources(Some(&silent), None))).await;
        // Then serving began without the paused clock moving: an awaited
        // refresh would have sat on the 120 s bound
        assert!(
            running.tokio_time_before_serving < Duration::from_secs(1),
            "{:?}",
            running.tokio_time_before_serving
        );
        // and the first request is answered (no data yet) while the fetch is in flight
        running
            .get(&env, PROJECTS)
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
        settle().await;
        assert_eq!(silent.calls(), 1, "the refresher did start its fetch");
    }

    #[tokio::test(start_paused = true)]
    async fn r4_a_list_whose_fetch_outlasts_the_refresh_bound_fails_and_keeps_its_held_copy() {
        // Given a held copy and a source that never answers
        let env = Env::new();
        env.seed(ListName::Projects, &projects_doc(T0));
        let silent = FakeSource::never_answering();
        let (log, _guard) = LogCapture::install();
        let running = launch(env.inputs(None, sources(Some(&silent), None))).await;
        settle().await;
        // When the refresh bound passes
        tokio::time::advance(REFRESH_BOUND).await;
        settle().await;
        // Then the held copy and its time are unchanged, and the failure is
        // logged with its reason code
        assert_eq!(running.get(&env, PROJECTS).await.text(), projects_doc(T0));
        assert_eq!(freshness(&running, &env).await["projects"], json!(T0));
        let line = log.one_line("live list refresh failed");
        assert!(line.contains("WARN"), "{line}");
        assert!(line.contains(r#"list="projects""#), "{line}");
        assert!(line.contains(r#"reason="refresh_bound_passed""#), "{line}");
    }

    // ---- R-15: validate, persist in one transaction, then swap ----

    #[tokio::test(start_paused = true)]
    async fn r15_a_successful_fetch_is_persisted_and_swapped_and_the_stored_document_equals_the_held_one()
     {
        let env = Env::new();
        env.seed(ListName::Projects, &projects_doc(T0));
        let fresh = projects_doc(T1);
        let source = FakeSource::answering(&fresh);
        let running = launch(env.inputs(None, sources(Some(&source), None))).await;
        settle().await;
        // The held copy is the fetched document, and so is the stored row
        assert_eq!(running.get(&env, PROJECTS).await.text(), fresh);
        assert_eq!(env.stored(ListName::Projects), Some(fresh));
        // and the refresh time advanced to that document's time
        assert_eq!(freshness(&running, &env).await["projects"], json!(T1));
    }

    #[tokio::test(start_paused = true)]
    async fn r15_a_forced_persist_failure_leaves_the_held_copy_and_its_time_and_logs_a_failed_fetch()
     {
        // Given a held copy and a database that refuses every write to live_lists
        let env = Env::new();
        let held = projects_doc(T0);
        env.seed(ListName::Projects, &held);
        env.db()
            .execute_batch(
                "CREATE TRIGGER refuse_update BEFORE UPDATE ON live_lists \
                 BEGIN SELECT RAISE(ABORT, 'forced'); END; \
                 CREATE TRIGGER refuse_insert BEFORE INSERT ON live_lists \
                 BEGIN SELECT RAISE(ABORT, 'forced'); END;",
            )
            .unwrap();
        let source = FakeSource::answering(&projects_doc(T1));
        let (log, _guard) = LogCapture::install();
        let running = launch(env.inputs(None, sources(Some(&source), None))).await;
        settle().await;
        // Then the fetch was attempted, yet the held copy, its time and the
        // stored row are all unchanged
        assert_eq!(source.calls(), 1);
        assert_eq!(running.get(&env, PROJECTS).await.text(), held);
        assert_eq!(freshness(&running, &env).await["projects"], json!(T0));
        assert_eq!(env.stored(ListName::Projects), Some(held));
        // and a failed fetch is logged with the persist reason code
        let line = log.one_line("live list refresh failed");
        assert!(line.contains(r#"list="projects""#), "{line}");
        assert!(line.contains(r#"reason="persist_failed""#), "{line}");
    }

    #[tokio::test(start_paused = true)]
    async fn r15_a_fetched_document_the_validator_refuses_is_neither_persisted_nor_swapped() {
        // Scenario: a fetch returns one repo whose description contains U+202E
        let env = Env::new();
        let held = projects_doc(T0);
        env.seed(ListName::Projects, &held);
        let source = FakeSource::answering(&projects_doc_with(T1, "bad \u{202e} text"));
        let (log, _guard) = LogCapture::install();
        let running = launch(env.inputs(None, sources(Some(&source), None))).await;
        settle().await;
        assert_eq!(running.get(&env, PROJECTS).await.text(), held);
        assert_eq!(freshness(&running, &env).await["projects"], json!(T0));
        assert_eq!(env.stored(ListName::Projects), Some(held));
        let line = log.one_line("live list refresh failed");
        assert!(
            line.contains("WARN") && line.contains(r#"list="projects""#),
            "{line}"
        );
        assert!(
            line.contains(r#"reason="projects_document_rejected""#),
            "{line}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r15_a_source_that_fails_leaves_the_held_copy_and_its_time() {
        let env = Env::new();
        env.seed(ListName::Articles, &articles_doc(T0));
        let source = FakeSource::failing(ReasonCode::RefreshBoundPassed);
        let running = launch(env.inputs(None, sources(None, Some(&source)))).await;
        settle().await;
        assert_eq!(source.calls(), 1);
        assert_eq!(freshness(&running, &env).await["articles"], json!(T0));
        assert_eq!(env.stored(ListName::Articles), Some(articles_doc(T0)));
    }

    // ---- R-17: cadence, independence, no handler fetch, panic isolation ----

    #[tokio::test(start_paused = true)]
    async fn r17_the_interval_fires_once_at_exactly_six_hours_and_serves_the_changed_content() {
        // Given a server whose start refresh has run
        let env = Env::new();
        let source = FakeSource::answering(&projects_doc_with(T1, "first"));
        let running = launch(env.inputs(None, sources(Some(&source), None))).await;
        settle().await;
        assert_eq!(source.calls(), 1, "the start refresh");
        // When the clock advances 6 h minus 1 s, no second fetch happens
        tokio::time::advance(REFRESH_INTERVAL - Duration::from_secs(1)).await;
        settle().await;
        assert_eq!(source.calls(), 1);
        // and a description changed at the source
        let changed = projects_doc_with(T2, "changed at the source");
        source.answer_with(&changed);
        // When the last second passes, exactly one more fetch happens
        tokio::time::advance(Duration::from_secs(1)).await;
        settle().await;
        assert_eq!(source.calls(), 2);
        // Then the server serves the new content without a restart
        assert_eq!(running.get(&env, PROJECTS).await.text(), changed);
    }

    #[tokio::test(start_paused = true)]
    async fn r17_a_failing_projects_source_beside_a_good_articles_source_advances_only_articles() {
        let env = Env::new();
        env.seed(ListName::Projects, &projects_doc(T0));
        env.seed(ListName::Articles, &articles_doc(T0));
        let projects = FakeSource::failing(ReasonCode::RefreshBoundPassed);
        let articles = FakeSource::answering(&articles_doc(T1));
        let running = launch(env.inputs(None, sources(Some(&projects), Some(&articles)))).await;
        settle().await;
        let fresh = freshness(&running, &env).await;
        assert_eq!(fresh["projects"], json!(T0));
        assert_eq!(fresh["articles"], json!(T1));
    }

    #[tokio::test(start_paused = true)]
    async fn r17_requests_to_health_the_lists_routes_the_redirect_and_a_fallback_cause_no_fetch() {
        let env = Env::new();
        let projects = FakeSource::answering(&projects_doc(T1));
        let articles = FakeSource::answering(&articles_doc(T1));
        let running = launch(env.inputs(None, sources(Some(&projects), Some(&articles)))).await;
        settle().await;
        let before = (projects.calls(), articles.calls());
        assert_eq!(before, (1, 1), "the start refresh");
        // When each kind of request is made
        running
            .server
            .get("/api/health")
            .await
            .assert_status(StatusCode::OK);
        running
            .get(&env, PROJECTS)
            .await
            .assert_status(StatusCode::OK);
        running
            .get(&env, FRESHNESS)
            .await
            .assert_status(StatusCode::OK);
        running
            .get(&env, "/articles")
            .await
            .assert_status(StatusCode::MOVED_PERMANENTLY);
        running.get(&env, "/no-such-page").await;
        settle().await;
        // Then no source was asked again
        assert_eq!((projects.calls(), articles.calls()), before);
    }

    #[tokio::test(start_paused = true)]
    async fn r17_after_a_forced_panic_in_the_persist_step_health_answers_200_and_the_readers_serve_the_held_copy()
     {
        // Given a held copy and a store whose persist panics
        let env = Env::new();
        let held = projects_doc(T0);
        env.seed(ListName::Projects, &held);
        let source = FakeSource::answering(&projects_doc(T1));
        let (log, _guard) = LogCapture::install();
        let mut inputs = env.inputs(None, sources(Some(&source), None));
        inputs.store = Some(Box::new(PanickingStore));
        let running = launch(inputs).await;
        // When the refresher reaches the persist step
        settle().await;
        assert_eq!(source.calls(), 1);
        // Then health answers 200, the readers serve the held copy,
        let health = running.server.get("/api/health").await;
        health.assert_status(StatusCode::OK);
        assert_eq!(running.get(&env, PROJECTS).await.text(), held);
        assert_eq!(freshness(&running, &env).await["projects"], json!(T0));
        // and an `error` line records the refresher's end
        let line = log.one_line("live lists refresher ended");
        assert!(line.contains("ERROR"), "{line}");
    }

    // ---- R-19: a disabled list is never fetched, and its row goes first ----

    #[tokio::test(start_paused = true)]
    async fn r19_a_disabled_list_is_never_fetched_at_start_or_on_a_tick() {
        let env = Env::new();
        let projects = FakeSource::answering(&projects_doc(T1));
        let articles = FakeSource::answering(&articles_doc(T1));
        let running =
            launch(env.inputs(Some("projects"), sources(Some(&projects), Some(&articles)))).await;
        settle().await;
        tokio::time::advance(REFRESH_INTERVAL).await;
        settle().await;
        assert_eq!(projects.calls(), 0);
        assert_eq!(articles.calls(), 2, "start and one tick");
        assert_eq!(env.stored(ListName::Projects), None);
        running
            .get(&env, PROJECTS)
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test(start_paused = true)]
    async fn r19_a_disabled_lists_row_is_gone_before_the_refreshers_first_write() {
        // Given stored rows for both lists and articles disabled
        let env = Env::new();
        env.seed(ListName::Projects, &projects_doc(T0));
        env.seed(ListName::Articles, &articles_doc(T0));
        // and a projects source that looks at the database as its fetch begins,
        // which is before the refresher can have written anything
        let seen: Arc<Mutex<Vec<i64>>> = Arc::default();
        let (db_path, record) = (env.db_path(), seen.clone());
        let source = FakeSource::probed(&projects_doc(T1), move || {
            let conn = rusqlite::Connection::open(&db_path).expect("open db");
            let rows: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM live_lists WHERE name = 'articles'",
                    [],
                    |r| r.get(0),
                )
                .expect("count rows");
            record.lock().expect("probe lock").push(rows);
        });
        // When the server starts
        launch(env.inputs(Some("articles"), sources(Some(&source), None))).await;
        settle().await;
        // Then the probe ran once and saw no articles row
        assert_eq!(*seen.lock().unwrap(), vec![0]);
    }

    #[tokio::test(start_paused = true)]
    async fn r19_unsetting_the_variable_and_restarting_lets_the_start_refresh_fill_the_list() {
        // Given articles were disabled and the server started once
        let env = Env::new();
        env.seed(ListName::Articles, &articles_doc(T0));
        let source = FakeSource::answering(&articles_doc(T1));
        let first = launch(env.inputs(Some("articles"), sources(None, Some(&source)))).await;
        settle().await;
        assert_eq!(source.calls(), 0);
        assert_eq!(freshness(&first, &env).await["articles"], Value::Null);
        // When the variable is unset and the server restarts over the same file
        let second = launch(env.inputs(None, sources(None, Some(&source)))).await;
        settle().await;
        // Then the start refresh filled articles, later than started_at
        let fresh = freshness(&second, &env).await;
        assert_eq!(fresh["articles"], json!(T1));
        assert!(
            ts::instant(fresh["articles"].as_str().unwrap())
                > ts::instant(fresh["started_at"].as_str().unwrap())
        );
    }

    // ---- R-16: a restart serves what was held, whatever the sources do ----

    #[tokio::test(start_paused = true)]
    async fn r16_after_a_restart_with_both_sources_failing_the_first_request_gets_the_held_copies_and_times()
     {
        // Given a first run that fetched and persisted both lists
        let env = Env::new();
        // (stamped at the wall clock, as a real fetch is: a copy dated hours
        // ahead of the clock would be refused at load)
        let (p, a) = (projects_doc_with(T0, "held"), articles_doc(T0));
        let first_p = FakeSource::answering(&p);
        let first_a = FakeSource::answering(&a);
        let first = launch(env.inputs(None, sources(Some(&first_p), Some(&first_a)))).await;
        settle().await;
        assert_eq!(freshness(&first, &env).await["articles"], json!(T0));
        drop(first);
        // When the server restarts and both sources fail
        let down_p = FakeSource::failing(ReasonCode::RefreshBoundPassed);
        let down_a = FakeSource::never_answering();
        let second = launch(env.inputs(None, sources(Some(&down_p), Some(&down_a)))).await;
        // Then the first request, before any refresh settles, returns the copies and times
        assert_eq!(second.get(&env, PROJECTS).await.text(), p);
        let fresh = freshness(&second, &env).await;
        assert_eq!(
            (&fresh["projects"], &fresh["articles"]),
            (&json!(T0), &json!(T0))
        );
    }

    // ---- R-30: the summary line ----

    #[tokio::test(start_paused = true)]
    async fn r30_a_refresh_where_every_enabled_list_succeeded_logs_one_info_summary_with_its_fields()
     {
        let env = Env::new();
        let projects = FakeSource::reporting(FetchReport {
            document: Ok(projects_doc(T1)),
            skipped: vec![],
            requests: vec![
                RequestNote {
                    status: 200,
                    ratelimit_remaining: Some(58),
                },
                RequestNote {
                    status: 200,
                    ratelimit_remaining: Some(57),
                },
            ],
        });
        let articles = FakeSource::reporting(FetchReport {
            document: Ok(articles_doc(T1)),
            skipped: vec![SkippedPost {
                index: 2,
                reason: ReasonCode::ArticlesDocumentRejected,
            }],
            requests: vec![RequestNote {
                status: 200,
                ratelimit_remaining: None,
            }],
        });
        let (log, _guard) = LogCapture::install();
        launch(env.inputs(None, sources(Some(&projects), Some(&articles)))).await;
        settle().await;
        let line = log.one_line("live lists refresh");
        assert!(line.contains("INFO"), "{line}");
        assert!(line.contains("trigger=start"), "{line}");
        assert!(
            line.contains(r#"projects="outcome=advanced count=3 skipped=none elapsed_ms=0""#),
            "{line}"
        );
        assert!(
            line.contains(
                r#"articles="outcome=advanced count=1 skipped=2:articles_document_rejected elapsed_ms=0""#
            ),
            "{line}"
        );
        assert!(
            line.contains(r#"requests="projects:200,projects:200,articles:200""#),
            "{line}"
        );
        // a remaining count only where the header parsed as an integer
        assert!(
            line.contains(r#"ratelimit_remaining="projects:58,projects:57""#),
            "{line}"
        );
        // and each skipped post logs its index and reason code only
        let skip = log.one_line("post skipped");
        assert!(skip.contains("WARN"), "{skip}");
        assert!(
            skip.contains(r#"list="articles""#) && skip.contains("index=2"),
            "{skip}"
        );
        assert!(
            skip.contains(r#"reason="articles_document_rejected""#),
            "{skip}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r30_a_refresh_where_a_list_failed_logs_the_summary_at_warn_with_the_reason() {
        let env = Env::new();
        let projects = FakeSource::failing(ReasonCode::RefreshBoundPassed);
        let articles = FakeSource::answering(&articles_doc(T1));
        let (log, _guard) = LogCapture::install();
        launch(env.inputs(None, sources(Some(&projects), Some(&articles)))).await;
        settle().await;
        let line = log.one_line("live lists refresh");
        assert!(line.contains("WARN"), "{line}");
        assert!(
            line.contains(r#"projects="outcome=failed reason=refresh_bound_passed count=- skipped=none elapsed_ms=0""#),
            "{line}"
        );
        assert!(line.contains("outcome=advanced count=1"), "{line}");
    }

    #[tokio::test(start_paused = true)]
    async fn r30_a_persist_failure_is_named_in_the_summary_by_its_reason_code() {
        let env = Env::new();
        env.db()
            .execute_batch(
                "CREATE TRIGGER refuse_insert BEFORE INSERT ON live_lists \
                 BEGIN SELECT RAISE(ABORT, 'forced'); END;",
            )
            .unwrap();
        let projects = FakeSource::answering(&projects_doc(T1));
        let (log, _guard) = LogCapture::install();
        launch(env.inputs(None, sources(Some(&projects), None))).await;
        settle().await;
        let line = log.one_line("live lists refresh");
        assert!(line.contains("WARN"), "{line}");
        assert!(
            line.contains("outcome=failed reason=persist_failed"),
            "{line}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r30_every_refresh_writes_its_own_summary_line_and_a_tick_says_tick() {
        let env = Env::new();
        let projects = FakeSource::answering(&projects_doc(T1));
        let (log, _guard) = LogCapture::install();
        launch(env.inputs(None, sources(Some(&projects), None))).await;
        settle().await;
        tokio::time::advance(REFRESH_INTERVAL).await;
        settle().await;
        let lines = log.lines_containing("live lists refresh");
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].contains("trigger=start"), "{}", lines[0]);
        assert!(lines[1].contains("trigger=tick"), "{}", lines[1]);
    }

    #[tokio::test(start_paused = true)]
    async fn r30_a_disabled_list_does_not_make_the_summary_warn_but_an_enabled_list_with_no_source_does()
     {
        let env = Env::new();
        let articles = FakeSource::answering(&articles_doc(T1));
        let (log, _guard) = LogCapture::install();
        // projects disabled by the switch; no source registered would be the
        // same for articles, so register only articles
        launch(env.inputs(Some("projects"), sources(None, Some(&articles)))).await;
        settle().await;
        let line = log.one_line("live lists refresh");
        assert!(line.contains("INFO"), "{line}");
        assert!(line.contains(r#"projects="outcome=disabled"#), "{line}");
        // and with nothing disabled but no projects source registered
        let env = Env::new();
        let (log, _guard2) = LogCapture::install();
        launch(env.inputs(None, sources(None, Some(&articles)))).await;
        settle().await;
        let line = log.one_line("live lists refresh");
        // an enabled list with no source did not succeed: R-30 says warn
        assert!(line.contains("WARN"), "{line}");
        assert!(line.contains(r#"projects="outcome=no_source"#), "{line}");
    }

    #[tokio::test(start_paused = true)]
    async fn r16_with_both_sources_hanging_health_answers_200_within_fifteen_seconds_through_start()
    {
        // Given both sources never answer
        let env = Env::new();
        let (p, a) = (FakeSource::never_answering(), FakeSource::never_answering());
        let before = tokio::time::Instant::now();
        // When the server starts and /api/health is requested
        let running = launch(env.inputs(None, sources(Some(&p), Some(&a)))).await;
        settle().await;
        let health = running.server.get("/api/health").await;
        // Then health is 200 and tokio's clock moved under 15 s
        health.assert_status(StatusCode::OK);
        assert!(
            before.elapsed() < Duration::from_secs(15),
            "{:?}",
            before.elapsed()
        );
        assert_eq!((p.calls(), a.calls()), (1, 1), "both fetches are in flight");
    }

    #[tokio::test(start_paused = true)]
    async fn r30_each_list_reports_its_own_elapsed_time_not_the_whole_refreshs() {
        // Given a projects source that takes 10 s and an articles source 30 s
        let env = Env::new();
        let projects = FakeSource::answering_after(&projects_doc(T1), Duration::from_secs(10));
        let articles = FakeSource::answering_after(&articles_doc(T1), Duration::from_secs(30));
        let (log, _guard) = LogCapture::install();
        launch(env.inputs(None, sources(Some(&projects), Some(&articles)))).await;
        settle().await;
        // (in two steps: one jump of 30 s would wake both sleeps at once)
        tokio::time::advance(Duration::from_secs(10)).await;
        settle().await;
        tokio::time::advance(Duration::from_secs(20)).await;
        settle().await;
        // Then each list's elapsed_ms is its own
        let line = log.one_line("live lists refresh");
        assert!(
            line.contains("count=3 skipped=none elapsed_ms=10000"),
            "{line}"
        );
        assert!(
            line.contains("count=1 skipped=none elapsed_ms=30000"),
            "{line}"
        );
    }

    // ---- one source's panic fails only its own list ----

    #[tokio::test(start_paused = true)]
    async fn r13_a_panicking_source_fails_its_own_list_and_the_other_still_advances_and_the_next_tick_runs()
     {
        // Given a projects source that panics and a good articles source
        let env = Env::new();
        env.seed(ListName::Projects, &projects_doc(T0));
        let projects = FakeSource::panicking();
        let articles = FakeSource::answering(&articles_doc(T1));
        let (log, _guard) = LogCapture::install();
        let running = launch(env.inputs(None, sources(Some(&projects), Some(&articles)))).await;
        settle().await;
        // Then articles advanced, projects kept its held copy
        let fresh = freshness(&running, &env).await;
        assert_eq!(fresh["articles"], json!(T1));
        assert_eq!(fresh["projects"], json!(T0));
        // and the failure is logged with its reason code
        let line = log.one_line("live list refresh failed");
        assert!(line.contains(r#"list="projects""#), "{line}");
        assert!(line.contains(r#"reason="source_panicked""#), "{line}");
        // and the refresher is alive: the next tick fetches both again
        tokio::time::advance(REFRESH_INTERVAL).await;
        settle().await;
        assert_eq!((projects.calls(), articles.calls()), (2, 2));
        assert!(
            log.lines_containing("refresher ended").is_empty(),
            "{}",
            log.text()
        );
    }

    // ---- Debug never prints document text ----

    #[test]
    fn a_fetch_report_debug_prints_the_document_length_never_its_text() {
        let sentinel = "sentinel-text-in-a-document";
        let report = FetchReport::document(format!("{{\"x\":\"{sentinel}\"}}"));
        let shown = format!("{report:?}");
        assert!(!shown.contains(sentinel), "{shown}");
        assert!(
            shown.contains(&format!("{}", report.document.as_ref().unwrap().len())),
            "{shown}"
        );
    }
}
