//! The held copies: the last validated document of each list, in memory.
//!
//! A [`Held`] can only be built from text its list's validator accepts, so
//! whatever the readers serve was validated, and the stored row, the held
//! text and a 200 body are one string (R-15, R-21).
//!
//! The writers are the start-up load and the refresher, both inside this
//! crate (the setters are `pub(crate)`); no request handler can write. A write
//! replaces one `Arc` under a lock held for that assignment alone, so no lock
//! is held across a path that can panic (R-17).

use std::sync::{Arc, PoisonError, RwLock};

use time::OffsetDateTime;

use crate::projects_list::ProjectsList;

use super::ListName;
use super::articles::ArticlesList;

/// A document refused by its list's validator. Carries no text: the document
/// came from a response body, which R-30 keeps out of logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rejected;

/// What the refresher, the loader and the readers need from a validated list.
pub trait ListDocument: Sized + Send + Sync + 'static {
    const NAME: ListName;

    /// The list's own validator, over document text.
    fn validate(text: &str) -> Result<Self, Rejected>;
    fn fetched_at(&self) -> &str;
    fn fetched_at_instant(&self) -> OffsetDateTime;
    /// Repos for projects, posts for articles; for the summary line.
    fn count(&self) -> usize;
}

impl ListDocument for ProjectsList {
    const NAME: ListName = ListName::Projects;

    fn validate(text: &str) -> Result<Self, Rejected> {
        crate::projects_list::parse(text).map_err(|_| Rejected)
    }

    fn fetched_at(&self) -> &str {
        ProjectsList::fetched_at(self)
    }

    fn fetched_at_instant(&self) -> OffsetDateTime {
        ProjectsList::fetched_at_instant(self)
    }

    fn count(&self) -> usize {
        self.counts().map(|(_, n)| n).sum()
    }
}

impl ListDocument for ArticlesList {
    const NAME: ListName = ListName::Articles;

    fn validate(text: &str) -> Result<Self, Rejected> {
        super::articles::parse(text).map_err(|_| Rejected)
    }

    fn fetched_at(&self) -> &str {
        ArticlesList::fetched_at(self)
    }

    fn fetched_at_instant(&self) -> OffsetDateTime {
        ArticlesList::fetched_at_instant(self)
    }

    fn count(&self) -> usize {
        self.posts().len()
    }
}

/// A validated document and the exact text it was validated from.
#[derive(Debug)]
pub struct Held<L> {
    list: L,
    json: String,
}

impl<L: ListDocument> Held<L> {
    /// Validate `json` and hold it.
    ///
    /// # Errors
    /// [`Rejected`] when the list's validator refuses the text.
    pub fn new(json: String) -> Result<Self, Rejected> {
        let list = L::validate(&json)?;
        Ok(Self { list, json })
    }

    pub fn list(&self) -> &L {
        &self.list
    }

    /// The document text: what is stored and what the route serves.
    pub fn json(&self) -> &str {
        &self.json
    }

    pub fn fetched_at(&self) -> &str {
        self.list.fetched_at()
    }
}

/// Both lists at one instant, so a reader that needs both (the prompts, R-20)
/// sees one moment, not two reads.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub projects: Option<Arc<Held<ProjectsList>>>,
    pub articles: Option<Arc<Held<ArticlesList>>>,
}

/// The handle readers and the refresher share. Cloning is cheap.
#[derive(Debug, Clone)]
pub struct LiveLists(Arc<Inner>);

#[derive(Debug)]
struct Inner {
    /// Fixed once per process before the first request (R-22), in the
    /// `fetched_at` grammar.
    started_at: String,
    snapshot: RwLock<Snapshot>,
}

impl LiveLists {
    /// Both lists start as no data yet.
    pub fn new(started_at: String) -> Self {
        Self(Arc::new(Inner {
            started_at,
            snapshot: RwLock::new(Snapshot::default()),
        }))
    }

    pub fn started_at(&self) -> &str {
        &self.0.started_at
    }

    pub fn snapshot(&self) -> Snapshot {
        // A poisoned lock still holds a whole `Snapshot`: every write is one
        // assignment, which cannot be interrupted half-done, so the value is
        // consistent and readers keep serving it.
        self.0
            .snapshot
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn set_projects(&self, held: Arc<Held<ProjectsList>>) {
        self.0
            .snapshot
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .projects = Some(held);
    }

    pub(crate) fn set_articles(&self, held: Arc<Held<ArticlesList>>) {
        self.0
            .snapshot
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .articles = Some(held);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const PROJECTS_DOC: &str = r#"{"fetched_at":"2026-11-19T12:00:00.000Z","groups":[{"owner":"mnemra","repos":[{"name":"alpha","description":"a"}]},{"owner":"prmanahan","repos":[{"name":"beta","description":""},{"name":"gamma","description":"g"}]}]}"#;
    pub(crate) const ARTICLES_DOC: &str = r#"{"fetched_at":"2026-11-19T12:00:00.000Z","posts":[{"title":"T","summary":"S","link":"https://mnemra.dev/blog/t","published":"2026-11-18T08:30:00.000Z"}]}"#;

    #[test]
    fn held_keeps_the_exact_text_it_validated() {
        let held = Held::<ProjectsList>::new(PROJECTS_DOC.to_string()).unwrap();
        assert_eq!(held.json(), PROJECTS_DOC);
        assert_eq!(held.fetched_at(), "2026-11-19T12:00:00.000Z");
        assert_eq!(held.list().count(), 3);
    }

    #[test]
    fn held_cannot_be_built_from_text_the_validator_refuses() {
        assert_eq!(
            Held::<ProjectsList>::new("{}".to_string()).unwrap_err(),
            Rejected
        );
        assert_eq!(
            Held::<ArticlesList>::new(PROJECTS_DOC.to_string()).unwrap_err(),
            Rejected
        );
    }

    #[test]
    fn a_new_handle_holds_no_data_yet_and_its_started_at() {
        let lists = LiveLists::new("2026-11-19T11:59:00.000Z".to_string());
        assert_eq!(lists.started_at(), "2026-11-19T11:59:00.000Z");
        let s = lists.snapshot();
        assert!(s.projects.is_none() && s.articles.is_none());
    }

    #[test]
    fn setting_one_list_leaves_the_other_untouched() {
        let lists = LiveLists::new("2026-11-19T11:59:00.000Z".to_string());
        let held = Arc::new(Held::<ProjectsList>::new(PROJECTS_DOC.to_string()).unwrap());
        lists.set_projects(held);
        let s = lists.snapshot();
        assert_eq!(s.projects.unwrap().json(), PROJECTS_DOC);
        assert!(s.articles.is_none());
        let held = Arc::new(Held::<ArticlesList>::new(ARTICLES_DOC.to_string()).unwrap());
        lists.set_articles(held);
        let s = lists.snapshot();
        assert!(s.projects.is_some());
        assert_eq!(s.articles.unwrap().json(), ARTICLES_DOC);
    }

    #[test]
    fn a_snapshot_is_unaffected_by_a_later_write() {
        let lists = LiveLists::new("2026-11-19T11:59:00.000Z".to_string());
        let before = lists.snapshot();
        lists.set_projects(Arc::new(
            Held::<ProjectsList>::new(PROJECTS_DOC.to_string()).unwrap(),
        ));
        assert!(before.projects.is_none());
    }

    #[test]
    fn a_poisoned_lock_still_serves_the_held_copy() {
        let lists = LiveLists::new("2026-11-19T11:59:00.000Z".to_string());
        lists.set_projects(Arc::new(
            Held::<ProjectsList>::new(PROJECTS_DOC.to_string()).unwrap(),
        ));
        let poisoner = lists.clone();
        let joined = std::thread::spawn(move || {
            let _guard = poisoner.0.snapshot.write().unwrap();
            panic!("poison the snapshot lock");
        })
        .join();
        assert!(joined.is_err(), "setup: the thread must have panicked");
        assert!(lists.0.snapshot.is_poisoned(), "setup: lock is poisoned");
        assert_eq!(lists.snapshot().projects.unwrap().json(), PROJECTS_DOC);
    }

    #[test]
    fn held_debug_prints_the_document_length_never_its_text() {
        let held = Held::<ProjectsList>::new(PROJECTS_DOC.to_string()).unwrap();
        let shown = format!("{held:?}");
        assert!(!shown.contains("alpha"), "{shown}");
        assert!(shown.contains(&PROJECTS_DOC.len().to_string()), "{shown}");
    }
}
