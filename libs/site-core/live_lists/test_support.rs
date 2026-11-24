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
