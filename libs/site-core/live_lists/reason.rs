//! R-30's closed enum of reason codes. A logged failure carries one of these
//! and never a formatted wrapped error, so no byte of a response body can
//! reach a log line through an error's `Display`.
//!
//! Each task adds the members for the failures it introduces (plan D11): the
//! store and refresher roles, the transport roles (R-2 to R-4) and the
//! projects roles (R-6). The feed roles are added with the feed source.

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
    /// R-6: the projects validator refused a document the projects source
    /// built. The source checks each R-6 condition itself, so this is the
    /// backstop for the two checks disagreeing.
    ProjectsDocumentRejected,
    /// R-2: the URL's scheme is not `https`.
    SchemeRefused,
    /// R-2: the server certificate did not chain to a trusted root, or did not
    /// name the requested host.
    CertificateRefused,
    /// R-2: the response was a 3xx.
    RedirectRefused,
    /// R-3: the response carried a `Content-Encoding` other than `identity`.
    ContentEncodingRefused,
    /// R-5: the host name did not resolve.
    NameResolutionFailed,
    /// R-4: the connection was not made within 5 s.
    ConnectTimeout,
    /// R-4: the request, body read included, took more than 20 s.
    TotalTimeout,
    /// R-4: the body held more than 2 MiB.
    BodyOverCap,
    /// R-4: an owner's 10th page still carried `rel="next"`.
    PageCapReached,
    /// R-4: the status was not 2xx (a 3xx has its own member).
    NonSuccessStatus,
    /// Any other transport failure: a refused or reset connection, a TLS
    /// failure that is not a certificate error, a malformed response.
    TransportFailed,
    /// R-6: a page body was not a JSON array.
    ProjectsBodyNotArray,
    /// R-6, R-4 of the adopted projects spec: an element failed its checks.
    ProjectsElementRejected,
    /// R-6: an owner had no repo left after R-3's exclusions.
    ProjectsOwnerEmpty,
    /// R-6: an owner's repos held a repeated name or a case-only twin.
    ProjectsDuplicateName,
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
            ReasonCode::SchemeRefused => "scheme_refused",
            ReasonCode::CertificateRefused => "certificate_refused",
            ReasonCode::RedirectRefused => "redirect_refused",
            ReasonCode::ContentEncodingRefused => "content_encoding_refused",
            ReasonCode::NameResolutionFailed => "name_resolution_failed",
            ReasonCode::ConnectTimeout => "connect_timeout",
            ReasonCode::TotalTimeout => "total_timeout",
            ReasonCode::BodyOverCap => "body_over_cap",
            ReasonCode::PageCapReached => "page_cap_reached",
            ReasonCode::NonSuccessStatus => "non_success_status",
            ReasonCode::TransportFailed => "transport_failed",
            ReasonCode::ProjectsBodyNotArray => "projects_body_not_array",
            ReasonCode::ProjectsElementRejected => "projects_element_rejected",
            ReasonCode::ProjectsOwnerEmpty => "projects_owner_empty",
            ReasonCode::ProjectsDuplicateName => "projects_duplicate_name",
            ReasonCode::ArticlesDocumentRejected => "articles_document_rejected",
        }
    }
}
