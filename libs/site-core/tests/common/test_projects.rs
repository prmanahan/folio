//! A valid list for integration tests that build an `AppState` and do not
//! care what is in it. The in-crate copy lives in `libs/site-core/test_projects.rs`.

use std::sync::Arc;

use site_core::projects_list::{self, ProjectsList};

const MINIMAL: &str = r#"{"fetched_at":"2026-11-13T12:00:00.000Z","groups":[{"owner":"mnemra","repos":[{"name":"alpha","description":""}]},{"owner":"prmanahan","repos":[{"name":"beta","description":""}]}]}"#;

pub fn projects() -> Arc<ProjectsList> {
    Arc::new(projects_list::parse(MINIMAL).expect("minimal test list is valid"))
}
