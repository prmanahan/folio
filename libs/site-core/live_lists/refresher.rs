//! The refresher (R-15, R-17, R-30): one background task that fetches each
//! enabled list from its injected source at start and then every 6 hours,
//! validates, persists, then swaps in the held copy.
//!
//! The sources are the seam. Production registers the GitHub and feed
//! sources through [`Sources`]; tests register fakes. No request handler
//! holds a handle that can start a refresh: the only owner of a
//! [`Refresher`] is the task [`Refresher::spawn`] starts.

use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use time::OffsetDateTime;

use super::ReasonCode;
use super::clock::WallClock;
use super::held::LiveLists;
use super::store::ListStore;
use super::switch::DisabledLists;

/// R-17: the 6-hour tick. Not a parameter; tests advance the tokio test clock.
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// R-4: a list whose fetch has not finished when this passes is a failed
/// fetch for that refresh.
pub const REFRESH_BOUND: Duration = Duration::from_secs(120);

/// A post a source rejected, by its item index and reason code only (R-30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkippedPost {
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
#[derive(Debug, PartialEq, Eq)]
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

/// What started a refresh (R-30's summary line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Start,
    Tick,
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

    /// Start the refresher without awaiting it.
    pub fn spawn(self) {}
}
