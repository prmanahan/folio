//! A valid list for in-crate `#[cfg(test)]` modules that build an `AppState`
//! and do not care what is in it.
//!
//! The integration-test crate has its own copy under `tests/common/test_projects.rs`,
//! for the same compilation-boundary reason as `test_password.rs`.

use std::sync::Arc;

use crate::projects_list::{self, ProjectsList};

const MINIMAL: &str = r#"{"fetched_at":"2026-11-13T12:00:00.000Z","groups":[{"owner":"mnemra","repos":[{"name":"alpha","description":""}]},{"owner":"prmanahan","repos":[{"name":"beta","description":""}]}]}"#;

pub(crate) fn projects() -> Arc<ProjectsList> {
    Arc::new(projects_list::parse(MINIMAL).expect("minimal test list is valid"))
}
