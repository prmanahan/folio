//! Shared fakes and builders for the live-lists unit tests.
//!
//! Nothing here is reachable from a non-test build. The start function takes
//! its clock, its sources and its store through `StartInputs`, the same
//! inputs production fills, so these are doubles for that seam and not a
//! second way to construct the product.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use time::{Duration, OffsetDateTime};

use super::clock::{WallClock, format_fetched_at};

/// The instant most tests start the wall clock at.
pub(crate) const T0: &str = "2026-11-19T12:00:00.000Z";

/// `s` (the `fetched_at` grammar) as an instant.
pub(crate) fn instant(s: &str) -> OffsetDateTime {
    crate::projects_list::parse_fetched_at(s).expect("a fetched_at literal")
}

pub(crate) fn format(t: OffsetDateTime) -> String {
    format_fetched_at(t).expect("a representable instant")
}

/// A valid projects document: two owners, three repos.
pub(crate) fn projects_doc(fetched_at: &str) -> String {
    projects_doc_with(fetched_at, "first description")
}

/// A valid projects document whose `alpha` repo carries `description`.
pub(crate) fn projects_doc_with(fetched_at: &str, description: &str) -> String {
    serde_json::json!({
        "fetched_at": fetched_at,
        "groups": [
            {"owner": "mnemra", "repos": [{"name": "alpha", "description": description}]},
            {"owner": "prmanahan", "repos": [
                {"name": "beta", "description": ""},
                {"name": "gamma", "description": "g"},
            ]},
        ],
    })
    .to_string()
}

/// A valid articles document of one post.
pub(crate) fn articles_doc(fetched_at: &str) -> String {
    serde_json::json!({
        "fetched_at": fetched_at,
        "posts": [{
            "title": "A post",
            "summary": "Its summary.",
            "link": "https://mnemra.dev/blog/a-post",
            "published": "2026-11-18T08:30:00.000Z",
        }],
    })
    .to_string()
}

/// A wall clock the test moves by hand.
pub(crate) struct FakeClock(Mutex<OffsetDateTime>);

impl FakeClock {
    pub(crate) fn at(fetched_at: &str) -> Arc<Self> {
        Arc::new(Self(Mutex::new(instant(fetched_at))))
    }

    pub(crate) fn advance(&self, by: Duration) {
        *self.0.lock().expect("clock lock") += by;
    }
}

impl WallClock for FakeClock {
    fn now(&self) -> OffsetDateTime {
        *self.0.lock().expect("clock lock")
    }
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A directory under the system temp dir, removed on drop.
pub(crate) struct TempDir(PathBuf);

impl TempDir {
    pub(crate) fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after the epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "folio-live-lists-{}-{n}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        Self(dir)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Best effort: a leftover temp dir must not fail a test.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ---- fakes for the refresher's seams ----

use std::sync::atomic::AtomicUsize;

use futures::future::BoxFuture;

use super::ListName;
use super::ReasonCode;
use super::refresher::{FetchReport, ListSource};
use super::store::{ListStore, PersistError};

/// What a [`FakeSource`] does when asked to fetch.
#[derive(Clone)]
pub(crate) enum Behaviour {
    /// Answer with this report.
    Report(FetchReport),
    /// Never answer.
    Never,
    /// Panic while fetching.
    Panic,
}

/// A source the test steers: it counts fetches, runs an optional probe at
/// entry, and answers as told.
pub(crate) struct FakeSource {
    calls: AtomicUsize,
    behaviour: Mutex<Behaviour>,
    probe: Option<Box<dyn Fn() + Send + Sync>>,
    /// Slept on tokio's clock before answering.
    delay: std::time::Duration,
}

impl FakeSource {
    /// Answers with `document` after sleeping `delay` on tokio's clock.
    pub(crate) fn answering_after(document: &str, delay: std::time::Duration) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            behaviour: Mutex::new(Behaviour::Report(FetchReport::document(
                document.to_string(),
            ))),
            probe: None,
            delay,
        })
    }

    pub(crate) fn answering(document: &str) -> Arc<Self> {
        Self::with(
            Behaviour::Report(FetchReport::document(document.to_string())),
            None,
        )
    }

    pub(crate) fn failing(reason: ReasonCode) -> Arc<Self> {
        Self::with(Behaviour::Report(FetchReport::failed(reason)), None)
    }

    pub(crate) fn panicking() -> Arc<Self> {
        Self::with(Behaviour::Panic, None)
    }

    pub(crate) fn never_answering() -> Arc<Self> {
        Self::with(Behaviour::Never, None)
    }

    pub(crate) fn reporting(report: FetchReport) -> Arc<Self> {
        Self::with(Behaviour::Report(report), None)
    }

    /// Answers with `document`, after running `probe` as the fetch begins.
    pub(crate) fn probed(document: &str, probe: impl Fn() + Send + Sync + 'static) -> Arc<Self> {
        Self::with(
            Behaviour::Report(FetchReport::document(document.to_string())),
            Some(Box::new(probe)),
        )
    }

    fn with(behaviour: Behaviour, probe: Option<Box<dyn Fn() + Send + Sync>>) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            behaviour: Mutex::new(behaviour),
            probe,
            delay: std::time::Duration::ZERO,
        })
    }

    pub(crate) fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    /// Change what the next fetch answers (the description changed at the source).
    pub(crate) fn answer_with(&self, document: &str) {
        *self.behaviour.lock().expect("behaviour lock") =
            Behaviour::Report(FetchReport::document(document.to_string()));
    }
}

impl ListSource for FakeSource {
    fn fetch(&self, _now: OffsetDateTime) -> BoxFuture<'_, FetchReport> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(probe) = &self.probe {
            probe();
        }
        let behaviour = self.behaviour.lock().expect("behaviour lock").clone();
        let delay = self.delay;
        Box::pin(async move {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            match behaviour {
                Behaviour::Report(report) => report,
                Behaviour::Never => std::future::pending().await,
                Behaviour::Panic => panic!("a source panicked"),
            }
        })
    }
}

/// A store whose persist panics: the forced panic in R-17's persist step,
/// carried by the injected store.
pub(crate) struct PanickingStore;

impl ListStore for PanickingStore {
    fn persist(&mut self, _list: ListName, _document: &str) -> Result<(), PersistError> {
        panic!("forced panic in the persist step");
    }
}

/// A store whose persist panics with a payload the test chose.
pub(crate) struct SentinelStore(pub(crate) String);

impl ListStore for SentinelStore {
    fn persist(&mut self, _list: ListName, _document: &str) -> Result<(), PersistError> {
        panic!("{}", self.0);
    }
}
