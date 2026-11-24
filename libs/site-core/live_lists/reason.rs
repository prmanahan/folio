//! R-30's closed enum of reason codes. A logged failure carries one of these
//! and never a formatted wrapped error, so no byte of a response body can
//! reach a log line through an error's `Display`.
//!
//! Each task adds the members for the failures it introduces (plan D11): the
//! store and refresher roles, the transport roles (R-2 to R-4) and the
//! projects roles (R-6) and the feed roles (R-7 to R-12, one member per rule).

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
    /// R-7: the body is not valid UTF-8.
    FeedNotUtf8,
    /// R-7: an XML parse error.
    FeedXmlParse,
    /// R-7: a prolog other than one declaration at byte 0, then whitespace.
    FeedProlog,
    /// R-7: a DOCTYPE anywhere.
    FeedDoctype,
    /// R-7: a processing instruction anywhere.
    FeedProcessingInstruction,
    /// R-7: a second XML declaration anywhere.
    FeedXmlDeclaration,
    /// R-7: a root other than `<rss>`.
    FeedRoot,
    /// R-7: `<channel>` is not the sole element child of `<rss>`.
    FeedChannel,
    /// R-7: not closed by `</channel>` then `</rss>`, or content after `</rss>`.
    FeedClose,
    /// R-7: text, CDATA or a comment directly under `<rss>` or `<channel>`.
    FeedStrayContent,
    /// R-7: element nesting deeper than 16.
    FeedTooDeep,
    /// R-12: the feed has no `<item>`.
    FeedNoItems,
    /// R-12: no valid post among the first 20 items.
    FeedNoValidPost,
    /// The parser's own document failed the articles validator, or the clock could not be written: an internal invariant.
    FeedDocumentInvalid,
    /// R-9: an item child that is not a field or `guid`.
    PostUnknownChild,
    /// R-9: a self-closing field or `guid`.
    PostSelfClosingChild,
    /// R-9: a repeated field or `guid`.
    PostDuplicateField,
    /// R-9: an empty field or `guid`.
    PostEmptyField,
    /// R-9: one of the four fields is missing.
    PostMissingField,
    /// R-9: a field's start tag carries more than its name.
    PostStartTagNotBare,
    /// R-9: text, CDATA or a comment directly in `<item>`.
    PostStrayContent,
    /// R-10: an element inside a field.
    PostElementInField,
    /// R-10: CDATA inside a field.
    PostCDataInField,
    /// R-10: a comment inside a field.
    PostCommentInField,
    /// R-10: decoded field text holds `<`.
    PostAngleBracket,
    /// R-10: a numeric character reference in a field.
    PostNumericReference,
    /// R-10: a named entity other than the five.
    PostUnknownEntity,
    /// R-11: a title over 200 code points.
    PostTitleTooLong,
    /// R-11: a rejected-class code point in the title.
    PostTitleRejectedChar,
    /// R-11: a summary over 600 code points.
    PostSummaryTooLong,
    /// R-11: a rejected-class code point in the summary.
    PostSummaryRejectedChar,
    /// R-11: a link over 300 bytes.
    PostLinkTooLong,
    /// R-11: a rejected-class code point in the link.
    PostLinkRejectedChar,
    /// R-11: the link does not parse as a URL.
    PostLinkNotUrl,
    /// R-11: the link's scheme is not `https`.
    PostLinkScheme,
    /// R-11: the link's host is not `mnemra.dev`.
    PostLinkHost,
    /// R-11: the link has a port.
    PostLinkPort,
    /// R-11: the link has userinfo.
    PostLinkUserinfo,
    /// R-11: the link has a query.
    PostLinkQuery,
    /// R-11: the link has a fragment.
    PostLinkFragment,
    /// R-11: the link's path is not `/blog/` plus a remainder.
    PostLinkPath,
    /// R-11: the parsed link's serialized form differs from the raw link.
    PostLinkNotCanonical,
    /// R-11: `pubDate` is over 64 bytes, not RFC 2822, or not representable.
    PostPubDate,
    /// R-11: the articles validator refused a post for a reason no field rule names.
    PostValidatorRefused,
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
            ReasonCode::FeedNotUtf8 => "feed_not_utf8",
            ReasonCode::FeedXmlParse => "feed_xml_parse",
            ReasonCode::FeedProlog => "feed_prolog",
            ReasonCode::FeedDoctype => "feed_doctype",
            ReasonCode::FeedProcessingInstruction => "feed_processing_instruction",
            ReasonCode::FeedXmlDeclaration => "feed_xml_declaration",
            ReasonCode::FeedRoot => "feed_root",
            ReasonCode::FeedChannel => "feed_channel",
            ReasonCode::FeedClose => "feed_close",
            ReasonCode::FeedStrayContent => "feed_stray_content",
            ReasonCode::FeedTooDeep => "feed_too_deep",
            ReasonCode::FeedNoItems => "feed_no_items",
            ReasonCode::FeedNoValidPost => "feed_no_valid_post",
            ReasonCode::FeedDocumentInvalid => "feed_document_invalid",
            ReasonCode::PostUnknownChild => "post_unknown_child",
            ReasonCode::PostSelfClosingChild => "post_self_closing_child",
            ReasonCode::PostDuplicateField => "post_duplicate_field",
            ReasonCode::PostEmptyField => "post_empty_field",
            ReasonCode::PostMissingField => "post_missing_field",
            ReasonCode::PostStartTagNotBare => "post_start_tag_not_bare",
            ReasonCode::PostStrayContent => "post_stray_content",
            ReasonCode::PostElementInField => "post_element_in_field",
            ReasonCode::PostCDataInField => "post_cdata_in_field",
            ReasonCode::PostCommentInField => "post_comment_in_field",
            ReasonCode::PostAngleBracket => "post_angle_bracket",
            ReasonCode::PostNumericReference => "post_numeric_reference",
            ReasonCode::PostUnknownEntity => "post_unknown_entity",
            ReasonCode::PostTitleTooLong => "post_title_too_long",
            ReasonCode::PostTitleRejectedChar => "post_title_rejected_char",
            ReasonCode::PostSummaryTooLong => "post_summary_too_long",
            ReasonCode::PostSummaryRejectedChar => "post_summary_rejected_char",
            ReasonCode::PostLinkTooLong => "post_link_too_long",
            ReasonCode::PostLinkRejectedChar => "post_link_rejected_char",
            ReasonCode::PostLinkNotUrl => "post_link_not_url",
            ReasonCode::PostLinkScheme => "post_link_scheme",
            ReasonCode::PostLinkHost => "post_link_host",
            ReasonCode::PostLinkPort => "post_link_port",
            ReasonCode::PostLinkUserinfo => "post_link_userinfo",
            ReasonCode::PostLinkQuery => "post_link_query",
            ReasonCode::PostLinkFragment => "post_link_fragment",
            ReasonCode::PostLinkPath => "post_link_path",
            ReasonCode::PostLinkNotCanonical => "post_link_not_canonical",
            ReasonCode::PostPubDate => "post_pub_date",
            ReasonCode::PostValidatorRefused => "post_validator_refused",
            ReasonCode::ArticlesDocumentRejected => "articles_document_rejected",
        }
    }
}
