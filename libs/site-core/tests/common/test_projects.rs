//! Lists for integration tests that build an `AppState` or a prompt and do
//! not care what is in them. The in-crate copy lives in `libs/site-core/test_projects.rs`.
//!
//! An integration test cannot write a held copy (the setters are crate-private,
//! R-17), so these hold no data yet; a test that needs held lists is an in-crate test.

use site_core::live_lists::LiveLists;
use site_core::live_lists::held::Snapshot;

pub fn lists() -> LiveLists {
    LiveLists::new("2026-11-13T11:00:00.000Z".to_string())
}

pub fn snapshot() -> Snapshot {
    Snapshot::default()
}
