//! A held projects list for in-crate `#[cfg(test)]` modules that build an
//! `AppState` or a prompt and do not care what is in it.
//!
//! The integration-test crate has its own copy under `tests/common/test_projects.rs`,
//! for the same compilation-boundary reason as `test_password.rs`.

use std::sync::Arc;

use crate::live_lists::LiveLists;
use crate::live_lists::held::{Held, Snapshot};
use crate::projects_list::ProjectsList;

pub(crate) const MINIMAL: &str = r#"{"fetched_at":"2026-11-13T12:00:00.000Z","groups":[{"owner":"mnemra","repos":[{"name":"alpha","description":""}]},{"owner":"prmanahan","repos":[{"name":"beta","description":""}]}]}"#;

/// A handle holding `doc` as its projects copy and no articles.
pub(crate) fn lists_with(doc: &str) -> LiveLists {
    let lists = LiveLists::new("2026-11-13T11:00:00.000Z".to_string());
    lists.set_projects(Arc::new(
        Held::<ProjectsList>::new(doc.to_string()).expect("test list is valid"),
    ));
    lists
}

/// A handle holding [`MINIMAL`] as its projects copy and no articles.
pub(crate) fn lists() -> LiveLists {
    lists_with(MINIMAL)
}

pub(crate) fn snapshot() -> Snapshot {
    lists().snapshot()
}
