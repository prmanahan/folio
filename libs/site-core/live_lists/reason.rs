//! R-30's closed enum of reason codes. A logged failure carries one of these
//! and never a formatted wrapped error, so no byte of a response body can
//! reach a log line through an error's `Display`.
//!
//! Each task adds the members for the failures it introduces (plan D11): this
//! one holds the store and refresher roles. The transport, projects and feed
//! roles are added by the tasks that introduce those failures.

/// Why a list or a post failed. Codes are lower snake case, stable, and safe
/// to log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasonCode {
    /// R-15: the validated document could not be written to the store.
    PersistFailed,
    /// R-19: a disabled list's row could not be deleted at start.
    DeleteFailed,
    /// R-4: one list's fetch had not finished when the refresh bound passed.
    RefreshBoundPassed,
    /// R-6: the source's projects document failed the projects validator.
    ProjectsDocumentRejected,
    /// R-7 to R-11: the source's articles document failed the articles
    /// validator.
    ArticlesDocumentRejected,
}

impl ReasonCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            ReasonCode::PersistFailed => "persist_failed",
            ReasonCode::DeleteFailed => "delete_failed",
            ReasonCode::RefreshBoundPassed => "refresh_bound_passed",
            ReasonCode::ProjectsDocumentRejected => "projects_document_rejected",
            ReasonCode::ArticlesDocumentRejected => "articles_document_rejected",
        }
    }
}
