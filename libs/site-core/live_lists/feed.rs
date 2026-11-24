//! The RSS feed parser behind the articles list (spec R-7 to R-12, R-14).
//!
//! Input is untrusted bytes from `https://mnemra.dev/rss.xml`; output is the
//! articles document as JSON text, which the refresher then validates again
//! with [`super::articles::parse`]. Every rule here fails closed: an
//! unrecognised shape is a rejection, never a pass-through.
//!
//! Errors are closed enums with no payload, so no byte of the body can reach a
//! log line through them (R-30).

use super::articles::{LinkRule, TextRule};

/// R-4's body cap, enforced here too so the parser never trusts its caller.
pub const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
/// R-7: element nesting deeper than this fails the feed, `<rss>` being 1.
pub const MAX_DEPTH: usize = 16;

/// A feed that produced a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedOutcome {
    /// The articles document as JSON text, `fetched_at` included.
    pub document: String,
    /// Posts rejected among the first 20 items, in document order.
    pub skipped: Vec<SkippedItem>,
}

/// An item rejected by index and rule only (R-30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkippedItem {
    /// Zero-based position among the channel's direct `<item>` children.
    pub index: usize,
    pub rule: PostRule,
}

/// One of an item's four required fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Title,
    Link,
    Description,
    PubDate,
}

/// Why the whole feed failed (R-7, R-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedRule {
    TooLarge,
    NotUtf8,
    /// Any XML parse error, with no resuming.
    Xml,
    /// A prolog other than one declaration at byte 0, then whitespace.
    Prolog,
    Doctype,
    ProcessingInstruction,
    XmlDeclaration,
    Root,
    /// `<channel>` is not the sole element child of `<rss>`.
    Channel,
    /// Not closed by `</channel>` then `</rss>`, or content after `</rss>`.
    Close,
    /// Text, CDATA or a comment directly under `<rss>` or `<channel>`.
    StrayContent,
    TooDeep,
    NoItems,
    NoValidPost,
    /// The assembled document failed the articles validator.
    DocumentInvalid,
}

/// Why one post was rejected (R-8 to R-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostRule {
    UnknownChild,
    SelfClosingChild,
    DuplicateField,
    EmptyField,
    MissingField(Field),
    /// A field's start tag carries anything after its name.
    StartTagNotBare,
    /// Text, CDATA or a comment directly in `<item>`.
    StrayContent,
    ElementInField,
    CDataInField,
    CommentInField,
    NumericReference,
    UnknownEntity,
    Title(TextRule),
    Summary(TextRule),
    Link(LinkRule),
    PubDate,
}

/// Parse a feed body.
///
/// # Errors
/// The [`FeedRule`] that failed the whole feed.
pub fn parse_feed(_body: &[u8], _fetched_at: &str) -> Result<FeedOutcome, FeedRule> {
    Ok(FeedOutcome {
        document: String::from("{\"posts\":[]}"),
        skipped: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live_lists::articles;
    use serde_json::Value;

    const FETCHED_AT: &str = "2026-11-24T12:00:00.000Z";
    const CAPTURE: &str = include_str!("fixtures/mnemra-rss-2026-11-24.xml");

    // ---- builders ----

    fn link(n: usize) -> String {
        format!("https://mnemra.dev/blog/post-{n}/")
    }

    /// An item whose four fields are valid; `n` makes each distinct.
    fn good(n: usize) -> String {
        item_with(
            &format!("<title>Title {n}</title>"),
            &format!("<link>{}</link>", link(n)),
            &format!("<description>Summary {n}</description>"),
            "<pubDate>Tue, 28 Apr 2026 00:00:00 GMT</pubDate>",
            "",
        )
    }

    fn item_with(title: &str, link: &str, desc: &str, date: &str, extra: &str) -> String {
        format!("<item>{title}{link}{desc}{date}{extra}</item>")
    }

    /// A valid item with one field's whole element replaced.
    fn good_with(field: Field, element: &str) -> String {
        let t = "<title>T</title>";
        let l = "<link>https://mnemra.dev/blog/x/</link>";
        let d = "<description>D</description>";
        let p = "<pubDate>Tue, 28 Apr 2026 00:00:00 GMT</pubDate>";
        match field {
            Field::Title => item_with(element, l, d, p, ""),
            Field::Link => item_with(t, element, d, p, ""),
            Field::Description => item_with(t, l, element, p, ""),
            Field::PubDate => item_with(t, l, d, element, ""),
        }
    }

    /// A valid item with `extra` appended inside it.
    fn good_plus(extra: &str) -> String {
        item_with(
            "<title>T</title>",
            "<link>https://mnemra.dev/blog/x/</link>",
            "<description>D</description>",
            "<pubDate>Tue, 28 Apr 2026 00:00:00 GMT</pubDate>",
            extra,
        )
    }

    /// A whole feed around `channel_body`.
    fn feed(channel_body: &str) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss version=\"2.0\"><channel><title>Blog</title>{channel_body}</channel></rss>"
        )
    }

    fn feed_plain(channel_body: &str) -> String {
        format!("<rss><channel>{channel_body}</channel></rss>")
    }

    fn run(body: &str) -> Result<FeedOutcome, FeedRule> {
        parse_feed(body.as_bytes(), FETCHED_AT)
    }

    fn fails(body: &str, rule: FeedRule) {
        assert_eq!(run(body), Err(rule), "body: {body}");
    }

    fn doc(outcome: &FeedOutcome) -> Value {
        serde_json::from_str(&outcome.document).expect("document is JSON")
    }

    fn titles(outcome: &FeedOutcome) -> Vec<String> {
        doc(outcome)["posts"]
            .as_array()
            .expect("posts array")
            .iter()
            .map(|p| p["title"].as_str().expect("title").to_owned())
            .collect()
    }

    /// Feed accepted with exactly one post and no skips.
    fn accepted_one(body: &str) {
        let out = run(body).unwrap_or_else(|e| panic!("rejected {e:?}: {body}"));
        assert_eq!(titles(&out).len(), 1, "body: {body}");
        assert!(out.skipped.is_empty(), "body: {body}");
    }

    /// `bad` as item 0 and a good item 1: only the bad one is rejected, by
    /// exactly `rule`, and the good one survives.
    fn rejects_post(bad: &str, rule: PostRule) {
        let body = feed(&format!("{bad}{}", good(1)));
        let out = run(&body).unwrap_or_else(|e| panic!("feed failed {e:?}: {body}"));
        assert_eq!(
            out.skipped,
            vec![SkippedItem { index: 0, rule }],
            "body: {body}"
        );
        assert_eq!(titles(&out), vec!["Title 1".to_owned()], "body: {body}");
    }

    // ---- positive control (R-14) ----

    #[test]
    fn r14_capture_of_the_live_feed_parses_to_its_expected_post() {
        // Given the body of https://mnemra.dev/rss.xml captured 2026-11-24
        // When parsed
        let out = parse_feed(CAPTURE.as_bytes(), FETCHED_AT).expect("capture parses");
        // Then it holds the one post the feed served that day, in the stored
        // grammar, and the articles validator accepts the document
        let d = doc(&out);
        assert_eq!(d["fetched_at"], FETCHED_AT);
        let posts = d["posts"].as_array().expect("posts");
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0]["title"], "Hello, Mnemra");
        assert_eq!(posts[0]["link"], "https://mnemra.dev/blog/hello-mnemra/");
        assert_eq!(posts[0]["published"], "2026-04-28T00:00:00.000Z");
        assert_eq!(
            posts[0]["summary"],
            "Mnemra is a context layer for MCP \u{2014} a memory server for agents. This post introduces the project, the architecture, and what comes next."
        );
        assert!(out.skipped.is_empty());
        articles::parse(&out.document).expect("validator accepts the output");
    }

    #[test]
    fn r9_guid_with_ispermalink_attribute_is_accepted() {
        // Given the guid form the live feed serves
        let body = feed(&good_plus(
            "<guid isPermaLink=\"true\">https://mnemra.dev/blog/hello-mnemra/</guid>",
        ));
        // Then the post is accepted
        accepted_one(&body);
    }

    #[test]
    fn r9_whitespace_between_elements_is_accepted() {
        // Given a pretty-printed feed
        let body = "<?xml version=\"1.0\"?>\n<rss>\n <channel>\n  <title>B</title>\n  <item>\n   <title>T</title>\n   <link>https://mnemra.dev/blog/x/</link>\n   <description>D</description>\n   <pubDate>Tue, 28 Apr 2026 00:00:00 GMT</pubDate>\n  </item>\n </channel>\n</rss>\n";
        accepted_one(body);
    }

    // ---- R-7: one negative fixture per bullet ----

    #[test]
    fn r7_xml_parse_error_fails_the_feed() {
        // mismatched end tag, unclosed comment, a bare ampersand, and `--`
        // inside a comment
        fails("<rss><channel><item></channel></rss>", FeedRule::Xml);
        fails(&feed(&format!("{}<!-- x", good(1))), FeedRule::Xml);
        fails(
            &feed(&good_with(Field::Title, "<title>A & B</title>")),
            FeedRule::Xml,
        );
        fails(&feed(&format!("{}<!-- a -- b -->", good(1))), FeedRule::Xml);
    }

    #[test]
    fn r7_invalid_utf8_anywhere_fails_the_feed() {
        // Given 0xFF inside a discarded child, inside a field, and after </rss>
        for (before, after) in [
            ("<language>", "</language>"),
            ("<item><title>T", "</title></item>"),
        ] {
            let mut body = b"<rss><channel>".to_vec();
            body.extend_from_slice(before.as_bytes());
            body.push(0xFF);
            body.extend_from_slice(after.as_bytes());
            body.extend_from_slice(b"</channel></rss>");
            assert_eq!(parse_feed(&body, FETCHED_AT), Err(FeedRule::NotUtf8));
        }
        let mut body = feed(&good(1)).into_bytes();
        body.extend_from_slice(b"\n\xFF");
        assert_eq!(parse_feed(&body, FETCHED_AT), Err(FeedRule::NotUtf8));
    }

    #[test]
    fn r7_prolog_other_than_one_declaration_at_byte_zero_fails() {
        let rss = "<rss><channel><item/></channel></rss>";
        // text, a comment, leading whitespace before the declaration, and a
        // byte-order mark (not whitespace)
        fails(&format!("junk{rss}"), FeedRule::Prolog);
        fails(&format!("<!-- c -->{rss}"), FeedRule::Prolog);
        fails(&format!(" <?xml version=\"1.0\"?>{rss}"), FeedRule::Prolog);
        fails(&format!("\u{FEFF}{rss}"), FeedRule::Prolog);
    }

    #[test]
    fn r7_declaration_then_whitespace_then_rss_is_accepted() {
        let rss = feed_plain(&good(1));
        accepted_one(&format!("<?xml version=\"1.0\"?>\r\n\t {rss}"));
        // and no declaration at all
        accepted_one(&rss);
    }

    #[test]
    fn r7_doctype_fails_the_feed_at_each_named_position() {
        let dt = "<!DOCTYPE rss>";
        // prolog, before and after the declaration
        fails(&format!("{dt}{}", feed(&good(1))), FeedRule::Doctype);
        fails(
            &format!(
                "<?xml version=\"1.0\"?>{dt}<rss><channel>{}</channel></rss>",
                good(1)
            ),
            FeedRule::Doctype,
        );
        // inside a field
        fails(
            &feed(&good_with(Field::Title, &format!("<title>T{dt}</title>"))),
            FeedRule::Doctype,
        );
        // inside discarded content
        fails(
            &feed(&format!("<language>en{dt}</language>{}", good(1))),
            FeedRule::Doctype,
        );
        // after the 20th item: inside the 21st, and between the 21st and 22nd
        let twenty: String = (0..20).map(good).collect();
        fails(
            &feed(&format!("{twenty}<item><title>T{dt}</title></item>")),
            FeedRule::Doctype,
        );
        fails(
            &feed(&format!("{twenty}{}{dt}{}", good(20), good(21))),
            FeedRule::Doctype,
        );
        // inside a rejected post: still the whole feed
        fails(
            &feed(&format!("<item><bogus>{dt}</bogus></item>{}", good(1))),
            FeedRule::Doctype,
        );
    }

    #[test]
    fn r7_processing_instruction_fails_the_feed_anywhere() {
        let pi = "<?xml-stylesheet href=\"a.css\"?>";
        fails(
            &format!("{pi}{}", feed(&good(1))),
            FeedRule::ProcessingInstruction,
        );
        fails(
            &feed(&good_with(Field::Title, &format!("<title>T{pi}</title>"))),
            FeedRule::ProcessingInstruction,
        );
        fails(
            &feed(&format!("<language>{pi}</language>{}", good(1))),
            FeedRule::ProcessingInstruction,
        );
        let twenty: String = (0..20).map(good).collect();
        fails(
            &feed(&format!("{twenty}<item>{pi}</item>")),
            FeedRule::ProcessingInstruction,
        );
    }

    #[test]
    fn r7_second_xml_declaration_fails_the_feed_anywhere() {
        let decl = "<?xml version=\"1.0\"?>";
        fails(
            &format!("{decl}{}", feed(&good(1))),
            FeedRule::XmlDeclaration,
        );
        fails(
            &feed(&good_with(Field::Title, &format!("<title>T{decl}</title>"))),
            FeedRule::XmlDeclaration,
        );
        fails(
            &feed(&format!("<language>{decl}</language>{}", good(1))),
            FeedRule::XmlDeclaration,
        );
    }

    #[test]
    fn r7_root_other_than_rss_fails() {
        fails("<feed><channel/></feed>", FeedRule::Root);
        fails("<RSS><channel/></RSS>", FeedRule::Root);
        fails("<atom:rss><channel/></atom:rss>", FeedRule::Root);
    }

    #[test]
    fn r7_channel_must_be_the_sole_element_child_of_rss() {
        let ch = format!("<channel>{}</channel>", good(1));
        fails(&format!("<rss>{ch}{ch}</rss>"), FeedRule::Channel);
        fails(&format!("<rss><extra/>{ch}</rss>"), FeedRule::Channel);
        fails(
            &format!("<rss>{ch}<extra>x</extra></rss>"),
            FeedRule::Channel,
        );
        fails("<rss></rss>", FeedRule::Channel);
        fails("<rss/>", FeedRule::Channel);
    }

    #[test]
    fn r7_document_must_close_channel_then_rss_with_only_whitespace_after() {
        let ch = format!("<channel>{}</channel>", good(1));
        fails(&format!("<rss>{ch}</rss>x"), FeedRule::Close);
        fails(&format!("<rss>{ch}</rss><!-- c -->"), FeedRule::Close);
        fails(&format!("<rss>{ch}</rss><rss/>"), FeedRule::Close);
        // truncated before either end tag
        fails(&format!("<rss><channel>{}", good(1)), FeedRule::Close);
        fails(&format!("<rss>{ch}"), FeedRule::Close);
        // trailing whitespace is accepted
        accepted_one(&format!("<rss>{ch}</rss> \r\n\t"));
    }

    #[test]
    fn r7_text_cdata_or_comment_directly_under_rss_or_channel_fails() {
        let item = good(1);
        fails(
            &format!("<rss>x<channel>{item}</channel></rss>"),
            FeedRule::StrayContent,
        );
        fails(
            &format!("<rss><![CDATA[x]]><channel>{item}</channel></rss>"),
            FeedRule::StrayContent,
        );
        fails(
            &format!("<rss><!-- c --><channel>{item}</channel></rss>"),
            FeedRule::StrayContent,
        );
        fails(&feed(&format!("x{item}")), FeedRule::StrayContent);
        fails(
            &feed(&format!("{item}<![CDATA[x]]>")),
            FeedRule::StrayContent,
        );
        fails(&feed(&format!("<!-- c -->{item}")), FeedRule::StrayContent);
        fails(
            &format!("<rss><channel>{item}</channel>x</rss>"),
            FeedRule::StrayContent,
        );
    }

    /// An `<x>` chain that puts its innermost element at `depth`, with
    /// `<rss>` as 1 and `<channel>` as 2.
    fn nested_to(depth: usize) -> String {
        let n = depth - 2;
        format!("{}t{}", "<x>".repeat(n), "</x>".repeat(n))
    }

    #[test]
    fn r7_nesting_of_exactly_16_is_accepted_and_17_fails() {
        accepted_one(&feed_plain(&format!("{}{}", nested_to(16), good(1))));
        fails(
            &feed_plain(&format!("{}{}", nested_to(17), good(1))),
            FeedRule::TooDeep,
        );
        // a self-closing element is an element: `<y/>` under 14 `<x>` is 17
        let at17 = format!("{}<y/>{}", "<x>".repeat(14), "</x>".repeat(14));
        fails(
            &feed_plain(&format!("{at17}{}", good(1))),
            FeedRule::TooDeep,
        );
        let at16 = format!("{}<y/>{}", "<x>".repeat(13), "</x>".repeat(13));
        accepted_one(&feed_plain(&format!("{at16}{}", good(1))));
    }

    #[test]
    fn r7_body_over_the_r4_cap_fails_and_the_cap_itself_is_accepted() {
        let base = feed(&good(1));
        let pad = |n: usize| format!("{base}{}", " ".repeat(n));
        let at_cap = pad(MAX_BODY_BYTES - base.len());
        assert_eq!(at_cap.len(), MAX_BODY_BYTES);
        accepted_one(&at_cap);
        fails(&pad(MAX_BODY_BYTES - base.len() + 1), FeedRule::TooLarge);
    }

    #[test]
    fn r7_prefixed_element_is_never_read_as_its_local_name() {
        // Given an item whose title is <atom:title>
        let atom = item_with(
            "<atom:title>T</atom:title>",
            "<link>https://mnemra.dev/blog/x/</link>",
            "<description>D</description>",
            "<pubDate>Tue, 28 Apr 2026 00:00:00 GMT</pubDate>",
            "",
        );
        let out = run(&feed(&format!("{atom}{}", good(1)))).expect("feed accepted");
        // Then it is rejected as missing title, and the feed still updates
        assert_eq!(
            out.skipped,
            vec![SkippedItem {
                index: 0,
                rule: PostRule::MissingField(Field::Title)
            }]
        );
        assert_eq!(titles(&out), vec!["Title 1".to_owned()]);
    }

    #[test]
    fn r7_namespace_declarations_are_not_processed() {
        // xmlns on the root changes nothing: `atom:link` stays unknown
        let body = format!(
            "<rss xmlns:atom=\"http://www.w3.org/2005/Atom\"><channel><atom:link href=\"x\"/>{}</channel></rss>",
            good(1)
        );
        accepted_one(&body);
    }

    // ---- R-8 ----

    #[test]
    fn r8_item_nested_in_an_unknown_channel_child_yields_no_post() {
        let out = run(&feed(&format!("<image>{}</image>{}", good(7), good(1)))).expect("accepted");
        assert_eq!(titles(&out), vec!["Title 1".to_owned()]);
        assert!(out.skipped.is_empty());
    }

    #[test]
    fn r8_cdata_or_comment_inside_a_discarded_child_is_ignored() {
        accepted_one(&feed(&format!(
            "<language><![CDATA[en]]><!-- c --></language>{}",
            good(1)
        )));
    }

    #[test]
    fn r8_item_inside_a_rejected_item_yields_no_post() {
        // Given item 0 with an unknown child that holds a whole valid item
        let outer = good_plus(&format!("<extra>{}</extra>", good(9)));
        let out = run(&feed(&format!("{outer}{}", good(1)))).expect("accepted");
        // Then item 0 is rejected, the nested item adds nothing, and parsing
        // resumed at item 0's end tag
        assert_eq!(
            out.skipped,
            vec![SkippedItem {
                index: 0,
                rule: PostRule::UnknownChild
            }]
        );
        assert_eq!(titles(&out), vec!["Title 1".to_owned()]);
    }

    const BAD_ATTRS: [&str; 3] = [" a=\"1\" a=\"2\"", " a=b", " a=\"&bogus;\""];

    #[test]
    fn r8_malformed_attributes_on_rss_never_fail_the_feed() {
        for a in BAD_ATTRS {
            accepted_one(&format!("<rss{a}><channel>{}</channel></rss>", good(1)));
        }
    }

    #[test]
    fn r8_malformed_attributes_on_channel_never_fail_the_feed() {
        for a in BAD_ATTRS {
            accepted_one(&format!("<rss><channel{a}>{}</channel></rss>", good(1)));
        }
    }

    #[test]
    fn r8_malformed_attributes_on_item_never_fail_the_feed() {
        for a in BAD_ATTRS {
            let item = good(1).replacen("<item>", &format!("<item{a}>"), 1);
            accepted_one(&feed_plain(&item));
        }
    }

    #[test]
    fn r8_malformed_attributes_on_guid_never_fail_the_feed() {
        for a in BAD_ATTRS {
            accepted_one(&feed(&good_plus(&format!(
                "<guid{a}>https://mnemra.dev/blog/x/</guid>"
            ))));
        }
    }

    #[test]
    fn r8_malformed_attributes_on_a_discarded_child_never_fail_the_feed() {
        for a in BAD_ATTRS {
            accepted_one(&feed(&format!("<language{a}>en</language>{}", good(1))));
        }
    }

    #[test]
    fn r8_unknown_entity_inside_a_discarded_child_leaves_the_feed_accepted() {
        accepted_one(&feed(&format!(
            "<language>&bogus; &#65;</language>{}",
            good(1)
        )));
    }

    // ---- R-9: one negative fixture per bullet ----

    #[test]
    fn r9_any_other_child_element_rejects_the_post() {
        rejects_post(&good_plus("<extra>x</extra>"), PostRule::UnknownChild);
        rejects_post(&good_plus("<Title>x</Title>"), PostRule::UnknownChild);
    }

    #[test]
    fn r9_atom_link_child_is_an_unknown_child_not_link() {
        // self-closing, as an Atom feed writes it, and with content
        rejects_post(
            &good_plus("<atom:link href=\"https://mnemra.dev/rss.xml\" rel=\"self\"/>"),
            PostRule::UnknownChild,
        );
        rejects_post(
            &good_plus("<atom:link>x</atom:link>"),
            PostRule::UnknownChild,
        );
    }

    #[test]
    fn r9_any_self_closing_child_rejects_the_post() {
        rejects_post(&good_plus("<guid/>"), PostRule::SelfClosingChild);
        rejects_post(
            &good_with(Field::Title, "<title/>"),
            PostRule::SelfClosingChild,
        );
        rejects_post(
            &good_with(Field::Link, "<link />"),
            PostRule::SelfClosingChild,
        );
        rejects_post(
            &good_with(Field::Description, "<description/>"),
            PostRule::SelfClosingChild,
        );
        rejects_post(
            &good_with(Field::PubDate, "<pubDate/>"),
            PostRule::SelfClosingChild,
        );
    }

    #[test]
    fn r9_duplicate_field_or_guid_rejects_the_post() {
        rejects_post(&good_plus("<title>again</title>"), PostRule::DuplicateField);
        rejects_post(
            &good_plus("<link>https://mnemra.dev/blog/y/</link>"),
            PostRule::DuplicateField,
        );
        rejects_post(
            &good_plus("<description>again</description>"),
            PostRule::DuplicateField,
        );
        rejects_post(
            &good_plus("<pubDate>Tue, 28 Apr 2026 00:00:00 GMT</pubDate>"),
            PostRule::DuplicateField,
        );
        rejects_post(
            &good_plus("<guid>a</guid><guid>b</guid>"),
            PostRule::DuplicateField,
        );
    }

    #[test]
    fn r9_empty_field_or_guid_rejects_the_post() {
        rejects_post(
            &good_with(Field::Title, "<title></title>"),
            PostRule::EmptyField,
        );
        rejects_post(
            &good_with(Field::Link, "<link></link>"),
            PostRule::EmptyField,
        );
        rejects_post(
            &good_with(Field::Description, "<description></description>"),
            PostRule::EmptyField,
        );
        rejects_post(
            &good_with(Field::PubDate, "<pubDate></pubDate>"),
            PostRule::EmptyField,
        );
        rejects_post(&good_plus("<guid></guid>"), PostRule::EmptyField);
    }

    #[test]
    fn r9_missing_field_rejects_the_post() {
        let t = "<title>T</title>";
        let l = "<link>https://mnemra.dev/blog/x/</link>";
        let d = "<description>D</description>";
        let p = "<pubDate>Tue, 28 Apr 2026 00:00:00 GMT</pubDate>";
        let miss = |f| PostRule::MissingField(f);
        rejects_post(&format!("<item>{l}{d}{p}</item>"), miss(Field::Title));
        rejects_post(&format!("<item>{t}{d}{p}</item>"), miss(Field::Link));
        rejects_post(&format!("<item>{t}{l}{p}</item>"), miss(Field::Description));
        rejects_post(&format!("<item>{t}{l}{d}</item>"), miss(Field::PubDate));
        rejects_post("<item></item>", miss(Field::Title));
    }

    #[test]
    fn r9_a_field_start_tag_with_anything_after_its_name_rejects_the_post() {
        for (field, tag, text) in FIELDS {
            for after in [" ", " lang=\"en\"", "\n", "\t"] {
                rejects_post(
                    &good_with(field, &format!("<{tag}{after}>{text}</{tag}>")),
                    PostRule::StartTagNotBare,
                );
            }
        }
    }

    #[test]
    fn r9_text_cdata_or_comment_directly_in_item_rejects_the_post() {
        rejects_post(&good_plus("stray"), PostRule::StrayContent);
        rejects_post(&good_plus("<![CDATA[stray]]>"), PostRule::StrayContent);
        rejects_post(&good_plus("<!-- stray -->"), PostRule::StrayContent);
    }

    #[test]
    fn r9_cdata_inside_guid_rejects_the_post() {
        rejects_post(
            &good_plus("<guid><![CDATA[https://mnemra.dev/blog/x/]]></guid>"),
            PostRule::CDataInField,
        );
    }

    #[test]
    fn r9_guid_text_is_read_under_r10_rules() {
        rejects_post(&good_plus("<guid>a&#65;</guid>"), PostRule::NumericReference);
        rejects_post(&good_plus("<guid>a&nbsp;</guid>"), PostRule::UnknownEntity);
        rejects_post(&good_plus("<guid>a<b/></guid>"), PostRule::ElementInField);
    }

    #[test]
    fn r9_guid_content_is_discarded_never_stored() {
        let out = run(&feed(&good_plus("<guid>SENTINEL-GUID-TEXT</guid>"))).expect("accepted");
        assert!(!out.document.contains("SENTINEL-GUID-TEXT"));
    }

    // ---- R-10: one negative fixture per bullet ----

    const FIELDS: [(Field, &str, &str); 4] = [
        (Field::Title, "title", "T"),
        (Field::Link, "link", "https://mnemra.dev/blog/x/"),
        (Field::Description, "description", "D"),
        (Field::PubDate, "pubDate", "Tue, 28 Apr 2026 00:00:00 GMT"),
    ];

    #[test]
    fn r10_an_element_inside_a_field_rejects_the_post() {
        for (f, tag, _) in FIELDS {
            for inner in ["<b>c</b>", "<b/>"] {
                rejects_post(
                    &good_with(f, &format!("<{tag}>a{inner}d</{tag}>")),
                    PostRule::ElementInField,
                );
            }
        }
    }

    #[test]
    fn r10_cdata_inside_a_field_rejects_the_post() {
        for (f, tag, _) in FIELDS {
            rejects_post(
                &good_with(f, &format!("<{tag}><![CDATA[x]]></{tag}>")),
                PostRule::CDataInField,
            );
        }
    }

    #[test]
    fn r10_a_comment_inside_a_field_rejects_the_post() {
        for (f, tag, _) in FIELDS {
            rejects_post(
                &good_with(f, &format!("<{tag}>a<!-- c -->b</{tag}>")),
                PostRule::CommentInField,
            );
        }
    }

    #[test]
    fn r10_a_numeric_character_reference_rejects_the_post() {
        for (f, tag, _) in FIELDS {
            for r in ["&#65;", "&#x41;", "&#x3C;"] {
                rejects_post(
                    &good_with(f, &format!("<{tag}>a{r}</{tag}>")),
                    PostRule::NumericReference,
                );
            }
        }
    }

    #[test]
    fn r10_any_other_named_entity_rejects_the_post() {
        for (f, tag, _) in FIELDS {
            for r in ["&nbsp;", "&bogus;", "&AMP;", "&Lt;"] {
                rejects_post(
                    &good_with(f, &format!("<{tag}>a{r}</{tag}>")),
                    PostRule::UnknownEntity,
                );
            }
        }
    }

    #[test]
    fn r10_decoded_text_containing_less_than_rejects_the_post() {
        rejects_post(
            &good_with(Field::Title, "<title>a &lt; b</title>"),
            PostRule::Title(TextRule::AngleBracket),
        );
        rejects_post(
            &good_with(Field::Description, "<description>&lt;b&gt;</description>"),
            PostRule::Summary(TextRule::AngleBracket),
        );
    }

    #[test]
    fn r10_greater_than_and_ampersand_in_decoded_text_are_allowed() {
        // Given a title holding &amp; and &gt; (and the other two named ones)
        let body = feed(&good_with(
            Field::Title,
            "<title>A &amp; B &gt; C &quot;D&quot; &apos;E&apos;</title>",
        ));
        // Then it is accepted and stored decoded
        let out = run(&body).expect("accepted");
        assert_eq!(titles(&out), vec!["A & B > C \"D\" 'E'".to_owned()]);
        assert!(out.skipped.is_empty());
    }

    // ---- R-11: one negative fixture per rule ----

    #[test]
    fn r11_title_is_at_most_200_code_points_counted_in_code_points() {
        let ok = "\u{e9}".repeat(200);
        let body = feed(&good_with(Field::Title, &format!("<title>{ok}</title>")));
        assert_eq!(titles(&run(&body).expect("200 accepted")), vec![ok]);
        rejects_post(
            &good_with(Field::Title, &format!("<title>{}</title>", "t".repeat(201))),
            PostRule::Title(TextRule::TooLong),
        );
    }

    #[test]
    fn r11_summary_is_at_most_600_code_points() {
        let ok = "\u{e9}".repeat(600);
        let body = feed(&good_with(
            Field::Description,
            &format!("<description>{ok}</description>"),
        ));
        let out = run(&body).expect("600 accepted");
        assert_eq!(doc(&out)["posts"][0]["summary"], ok.as_str());
        rejects_post(
            &good_with(
                Field::Description,
                &format!("<description>{}</description>", "d".repeat(601)),
            ),
            PostRule::Summary(TextRule::TooLong),
        );
    }

    #[test]
    fn r11_a_rejected_class_code_point_in_title_or_summary_rejects_the_post() {
        for c in ["\u{2028}", "\u{85}", "\u{200B}"] {
            rejects_post(
                &good_with(Field::Title, &format!("<title>a{c}b</title>")),
                PostRule::Title(TextRule::RejectedChar),
            );
            rejects_post(
                &good_with(
                    Field::Description,
                    &format!("<description>a{c}b</description>"),
                ),
                PostRule::Summary(TextRule::RejectedChar),
            );
        }
    }

    fn link_item(l: &str) -> String {
        good_with(Field::Link, &format!("<link>{l}</link>"))
    }

    #[test]
    fn r11_link_rules_each_reject_the_post() {
        let long = format!("https://mnemra.dev/blog/{}", "a".repeat(300));
        rejects_post(&link_item(&long), PostRule::Link(LinkRule::TooLong));
        rejects_post(
            &link_item("https://mnemra.dev/blog/a\u{2028}b"),
            PostRule::Link(LinkRule::RejectedChar),
        );
        rejects_post(&link_item("not a url"), PostRule::Link(LinkRule::NotAUrl));
        rejects_post(
            &link_item("http://mnemra.dev/blog/x/"),
            PostRule::Link(LinkRule::Scheme),
        );
        for host in ["www.mnemra.dev", "mnemra.dev.evil.example", "example.com"] {
            rejects_post(
                &link_item(&format!("https://{host}/blog/x/")),
                PostRule::Link(LinkRule::Host),
            );
        }
        rejects_post(
            &link_item("https://mnemra.dev:8443/blog/x/"),
            PostRule::Link(LinkRule::Port),
        );
        rejects_post(
            &link_item("https://user@mnemra.dev/blog/x/"),
            PostRule::Link(LinkRule::Userinfo),
        );
        rejects_post(
            &link_item("https://mnemra.dev/blog/x/?a=1"),
            PostRule::Link(LinkRule::Query),
        );
        rejects_post(
            &link_item("https://mnemra.dev/blog/x/#top"),
            PostRule::Link(LinkRule::Fragment),
        );
        rejects_post(
            &link_item("https://mnemra.dev/blog/"),
            PostRule::Link(LinkRule::Path),
        );
        rejects_post(
            &link_item("https://mnemra.dev/about/"),
            PostRule::Link(LinkRule::Path),
        );
        // the parsed form differs from the raw link: host case
        rejects_post(
            &link_item("https://MNEMRA.dev/blog/x/"),
            PostRule::Link(LinkRule::NotCanonical),
        );
    }

    #[test]
    fn r11_a_300_byte_link_is_accepted_and_stored_as_written() {
        let l = format!("https://mnemra.dev/blog/{}", "a".repeat(300 - 24));
        assert_eq!(l.len(), 300);
        let out = run(&feed(&link_item(&l))).expect("accepted");
        assert_eq!(doc(&out)["posts"][0]["link"], l.as_str());
    }

    fn date_item(s: &str) -> String {
        good_with(Field::PubDate, &format!("<pubDate>{s}</pubDate>"))
    }

    #[test]
    fn r11_pubdate_is_at_most_64_bytes_and_rfc_2822() {
        let long = format!("Tue, 28 Apr 2026 00:00:00 GMT{}", " ".repeat(40));
        assert!(long.len() > 64);
        rejects_post(&date_item(&long), PostRule::PubDate);
        rejects_post(&date_item("yesterday"), PostRule::PubDate);
        rejects_post(&date_item("2026-04-28T00:00:00Z"), PostRule::PubDate);
        rejects_post(&date_item("Tue, 31 Feb 2026 00:00:00 GMT"), PostRule::PubDate);
    }

    #[test]
    fn r11_pubdate_is_stored_in_utc_in_the_fetched_at_grammar() {
        for (raw, stored) in [
            ("Tue, 28 Apr 2026 00:00:00 GMT", "2026-04-28T00:00:00.000Z"),
            ("Tue, 28 Apr 2026 02:00:00 +0200", "2026-04-28T00:00:00.000Z"),
            ("Mon, 27 Apr 2026 19:30:15 -0430", "2026-04-28T00:00:15.000Z"),
        ] {
            let out = run(&feed(&date_item(raw))).expect("accepted");
            assert_eq!(doc(&out)["posts"][0]["published"], stored, "raw: {raw}");
        }
    }

    // ---- R-12 ----

    #[test]
    fn r12_the_first_20_items_are_validated_and_later_ones_are_not_collected() {
        // Given 22 items whose 3rd is bad
        let items: String = (0..22)
            .map(|n| {
                if n == 2 {
                    "<item></item>".to_owned()
                } else {
                    good(n)
                }
            })
            .collect();
        let out = run(&feed(&items)).expect("accepted");
        // Then 19 posts are held, in order: neither the 21st nor the 22nd
        let want: Vec<String> = (0..20)
            .filter(|n| *n != 2)
            .map(|n| format!("Title {n}"))
            .collect();
        assert_eq!(want.len(), 19);
        assert_eq!(titles(&out), want);
        assert_eq!(
            out.skipped,
            vec![SkippedItem {
                index: 2,
                rule: PostRule::MissingField(Field::Title)
            }]
        );
    }

    #[test]
    fn r12_items_after_the_20th_are_not_validated_even_when_malformed() {
        let twenty: String = (0..20).map(good).collect();
        let body = feed(&format!(
            "{twenty}<item></item>{}<item><bogus/></item>",
            good(21)
        ));
        let out = run(&body).expect("accepted");
        assert_eq!(titles(&out).len(), 20);
        assert!(out.skipped.is_empty());
    }

    #[test]
    fn r12_a_doctype_in_the_21st_item_fails_the_feed() {
        let twenty: String = (0..20).map(good).collect();
        fails(
            &feed(&format!("{twenty}<item><title>T<!DOCTYPE x></title></item>")),
            FeedRule::Doctype,
        );
    }

    #[test]
    fn r12_a_feed_with_no_item_fails() {
        fails(&feed(""), FeedRule::NoItems);
        fails(&feed("<language>en</language>"), FeedRule::NoItems);
        // an item nested in an unknown child is not an item
        fails(
            &feed(&format!("<image>{}</image>", good(1))),
            FeedRule::NoItems,
        );
        fails("<rss><channel/></rss>", FeedRule::NoItems);
    }

    #[test]
    fn r12_a_feed_whose_posts_are_all_bad_fails_so_the_held_copy_is_kept() {
        let bad: String = (0..3).map(|_| "<item></item>").collect();
        fails(&feed(&bad), FeedRule::NoValidPost);
        // 20 bad items and a valid 21st: the 21st is not collected
        let twenty_bad: String = (0..20).map(|_| "<item></item>").collect();
        fails(
            &feed(&format!("{twenty_bad}{}", good(21))),
            FeedRule::NoValidPost,
        );
    }

    #[test]
    fn r12_three_posts_and_one_has_a_201_code_point_title() {
        // Scenario "three posts and one has a title of 201 code points"
        let long = format!("<title>{}</title>", "t".repeat(201));
        let body = feed(&format!(
            "{}{}{}",
            good(0),
            good_with(Field::Title, &long),
            good(2)
        ));
        let out = run(&body).expect("accepted");
        assert_eq!(
            titles(&out),
            vec!["Title 0".to_owned(), "Title 2".to_owned()]
        );
        assert_eq!(
            out.skipped,
            vec![SkippedItem {
                index: 1,
                rule: PostRule::Title(TextRule::TooLong)
            }]
        );
    }

    #[test]
    fn r12_item_index_counts_direct_channel_items_only() {
        // an <item> inside <image> does not advance the index
        let body = feed(&format!(
            "<image>{}</image><item></item>{}",
            good(9),
            good(1)
        ));
        let out = run(&body).expect("accepted");
        assert_eq!(
            out.skipped,
            vec![SkippedItem {
                index: 0,
                rule: PostRule::MissingField(Field::Title)
            }]
        );
    }

    // ---- properties ----

    /// xorshift64*: a seeded generator in plain std, so a failure reproduces
    /// from its seed (the repository carries no property-test crate).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    const TOKENS: [&str; 24] = [
        "<",
        ">",
        "&",
        ";",
        "&amp;",
        "&#65;",
        "<!DOCTYPE x>",
        "<?xml version=\"1.0\"?>",
        "<?pi?>",
        "<![CDATA[",
        "]]>",
        "<!--",
        "-->",
        "</item>",
        "<item>",
        "<channel>",
        "</channel>",
        "</rss>",
        "<rss>",
        "=\"",
        "\"",
        "<guid/>",
        "\u{2028}",
        "\u{FFFD}",
    ];

    fn mutate(seed: &[u8], rng: &mut Rng) -> Vec<u8> {
        let mut b = seed.to_vec();
        for _ in 0..1 + rng.below(6) {
            if b.is_empty() {
                b.push(b'<');
            }
            let at = rng.below(b.len());
            match rng.below(6) {
                0 => b[at] = rng.next() as u8,
                1 => {
                    b.remove(at);
                }
                2 => {
                    let t = TOKENS[rng.below(TOKENS.len())];
                    b.splice(at..at, t.bytes());
                }
                3 => {
                    let end = (at + rng.below(40)).min(b.len());
                    b.drain(at..end);
                }
                4 => {
                    let end = (at + rng.below(40)).min(b.len());
                    let chunk = b[at..end].to_vec();
                    b.splice(at..at, chunk);
                }
                _ => b.truncate(at),
            }
        }
        b
    }

    #[test]
    fn property_no_input_panics_and_every_ok_output_is_a_valid_document() {
        // Given mutations of valid and hostile seeds
        let twenty_two: String = (0..22).map(good).collect();
        let seeds: Vec<Vec<u8>> = vec![
            CAPTURE.as_bytes().to_vec(),
            feed(&twenty_two).into_bytes(),
            feed(&format!(
                "<language>en</language>{}<image>{}</image>",
                good(0),
                good(1)
            ))
            .into_bytes(),
            feed(&good_plus(
                "<guid isPermaLink=\"true\">https://mnemra.dev/blog/x/</guid>",
            ))
            .into_bytes(),
        ];
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let mut accepted = 0usize;
        for i in 0..20_000 {
            let input = mutate(&seeds[i % seeds.len()], &mut rng);
            // When parsed (a panic fails the test)
            if let Ok(out) = parse_feed(&input, FETCHED_AT) {
                accepted += 1;
                // Then an Ok output passes the articles validator, holds at
                // most 20 posts, and every skip index is within the first 20
                let list = articles::parse(&out.document).expect("validator accepts Ok output");
                assert!((1..=20).contains(&list.posts().len()));
                assert!(out.skipped.iter().all(|s| s.index < 20));
            }
        }
        // the generator reaches both outcomes (a control on the instrument)
        assert!(accepted > 0 && accepted < 20_000, "accepted {accepted}");
    }

    #[test]
    fn property_arbitrary_bytes_never_panic() {
        let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
        for _ in 0..5_000 {
            let n = rng.below(200);
            let bytes: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
            let _ = parse_feed(&bytes, FETCHED_AT);
        }
    }

    #[test]
    fn property_text_in_discarded_places_never_reaches_the_output() {
        // Given a marker placed in every place R-8 and R-10 discard or reject
        let mut rng = Rng(0x1234_5678_9ABC_DEF1);
        for case in 0..300u64 {
            let m = format!("ZQX{:016x}", rng.next());
            // the rejection that sends the post's description away varies
            let reject = [
                "<title>a &lt; b</title>",
                "<title>a&#65;</title>",
                "<title>a&bogus;</title>",
                "<title>a<b/></title>",
                "<extra/>",
                "<guid/>",
            ][rng.below(6)];
            let rejected = format!(
                "<item k=\"{m}-attr\">{reject}<link>https://mnemra.dev/blog/x/</link><description>{m}-desc</description><pubDate>Tue, 28 Apr 2026 00:00:00 GMT</pubDate></item>"
            );
            let discarded = format!(
                "<language k=\"{m}-lang\">{m}-text<![CDATA[{m}-cdata]]><!--{m}-comment--><x>{m}-deep &bogus;</x></language><image><item><title>{m}-img</title></item></image>"
            );
            let valid = format!(
                "<item k=\"{m}-iattr\"><title>Kept</title><link>https://mnemra.dev/blog/x/</link><description>Kept summary</description><pubDate>Tue, 28 Apr 2026 00:00:00 GMT</pubDate><guid k=\"{m}-gattr\">{m}-guid</guid></item>"
            );
            let (first, second) = if rng.below(2) == 0 {
                (&rejected, &valid)
            } else {
                (&valid, &rejected)
            };
            let body = format!(
                "<rss k=\"{m}-rss\"><channel k=\"{m}-chan\">{discarded}{first}{second}</channel></rss>"
            );
            // When parsed
            let out = parse_feed(body.as_bytes(), FETCHED_AT)
                .unwrap_or_else(|e| panic!("case {case} rejected {e:?}: {body}"));
            // Then no marker appears in the document or in the skip record
            assert_eq!(titles(&out), vec!["Kept".to_owned()], "case {case}");
            assert_eq!(out.skipped.len(), 1, "case {case}");
            assert!(
                !out.document.contains("ZQX"),
                "case {case}: {}",
                out.document
            );
            assert!(!format!("{out:?}").contains("ZQX"), "case {case}");
        }
    }

    #[test]
    fn property_control_text_of_a_valid_field_does_appear_in_the_output() {
        // The absence checks above mean something only if the instrument sees
        // a marker that is meant to be there.
        let body = feed(&good_with(
            Field::Description,
            "<description>ZQXcontrol</description>",
        ));
        let out = run(&body).expect("accepted");
        assert!(out.document.contains("ZQXcontrol"));
    }
}
