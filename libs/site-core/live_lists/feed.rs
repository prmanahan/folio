//! The RSS feed parser behind the articles list (spec R-7 to R-12, R-14).
//!
//! Input is untrusted bytes from `https://mnemra.dev/rss.xml`; output is the
//! articles document as JSON text, which the refresher then validates again
//! with [`super::articles::parse`]. Every rule here fails closed: an
//! unrecognised shape is a rejection, never a pass-through.
//!
//! Errors are closed enums with no payload, so no byte of the body can reach a
//! log line through them (R-30).

use quick_xml::Reader;
use quick_xml::events::{BytesRef, BytesStart, BytesText, Event};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc2822;

use super::articles::{self, ArticlesError, LinkRule, MAX_POSTS, TextRule};
use super::clock::format_fetched_at;
// R-4's cap, enforced here too so the parser never trusts its caller.
use super::outbound::MAX_BODY_BYTES;

/// R-7: element nesting deeper than this fails the feed, `<rss>` being 1.
pub const MAX_DEPTH: usize = 16;
/// R-11: the longest `pubDate` accepted.
const MAX_PUB_DATE_BYTES: usize = 64;

/// A feed that produced a document.
#[derive(Clone, PartialEq, Eq)]
pub struct FeedOutcome {
    /// The articles document as JSON text, `fetched_at` included.
    pub document: String,
    /// Posts rejected among the first 20 items, in document order.
    pub skipped: Vec<SkippedItem>,
}

/// `document` is response-body text, so Debug prints its length only (R-30).
impl std::fmt::Debug for FeedOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FeedOutcome")
            .field("document_len", &self.document.len())
            .field("skipped", &self.skipped)
            .finish()
    }
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
    /// The `fetched_at` could not be written, or the assembled document failed
    /// the articles validator: an internal invariant, not a verdict on the feed.
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
    /// Decoded field text holds `<` (R-10), whichever field it is.
    AngleBracketInField,
    NumericReference,
    UnknownEntity,
    Title(TextRule),
    Summary(TextRule),
    Link(LinkRule),
    PubDate,
    /// The articles validator refused for a reason no field rule names.
    Validator,
}

/// Parse a feed body into the articles document.
///
/// `now` is the wall clock's instant; it becomes the document's `fetched_at`.
/// Whole-feed rules (R-7) are enforced for the
/// entire body, including items past the 20th. Only the first 20 `<item>`s
/// are collected and validated (R-12).
///
/// # Errors
/// The [`FeedRule`] that failed the whole feed.
pub fn parse_feed(body: &[u8], now: OffsetDateTime) -> Result<FeedOutcome, FeedRule> {
    if body.len() > MAX_BODY_BYTES {
        return Err(FeedRule::TooLarge);
    }
    let text = std::str::from_utf8(body).map_err(|_| FeedRule::NotUtf8)?;
    // A byte-order mark is not whitespace, so it is a prolog violation. It is
    // refused here so the outcome never depends on whether the reader skips it.
    if text.starts_with('\u{FEFF}') {
        return Err(FeedRule::Prolog);
    }
    let scan = Scanner::new(text).scan()?;
    // The wall clock is the caller's; a year the grammar cannot hold is not a
    // document we can write, so the feed fails rather than panics.
    let fetched_at = format_fetched_at(now).map_err(|_| FeedRule::DocumentInvalid)?;
    assemble(scan, &fetched_at)
}

/// The items among the first 20, in document order, each accepted or
/// rejected, and how many direct `<item>` children the channel held.
struct Scan {
    first_20: Vec<Result<Post, PostRule>>,
    items_seen: usize,
}

/// A post whose fields passed R-9 to R-11, ready for the document.
struct Post {
    title: String,
    summary: String,
    link: String,
    published: String,
}

/// One event, classified. Whole-feed rules never reach a caller: [`Scanner::step`]
/// fails the feed on them first, so no state can forget one.
enum Tok<'a> {
    Start(BytesStart<'a>),
    Empty(BytesStart<'a>),
    End,
    Text(BytesText<'a>),
    CData,
    Comment,
    Ref(BytesRef<'a>),
    Eof,
}

struct Scanner<'a> {
    reader: Reader<&'a [u8]>,
    depth: usize,
    seen_decl: bool,
    /// Set at the first start tag: a declaration after it is a second
    /// declaration (R-7), before it a prolog violation.
    root_started: bool,
}

impl<'a> Scanner<'a> {
    /// DSP-16: every option `Config` has today is set here. `Config` is
    /// `#[non_exhaustive]`, so an option a later release adds keeps its
    /// default; the pinned-configuration test fails when that happens.
    fn new(text: &'a str) -> Self {
        let mut reader = Reader::from_str(text);
        let config = reader.config_mut();
        config.allow_dangling_amp = false;
        config.allow_unmatched_ends = false;
        config.check_comments = true;
        config.check_end_names = true;
        config.expand_empty_elements = false;
        config.trim_markup_names_in_closing_tags = true;
        config.trim_text_start = false;
        config.trim_text_end = false;
        Self {
            reader,
            depth: 0,
            seen_decl: false,
            root_started: false,
        }
    }

    /// Read one event. The match names every `Event` variant: a variant added
    /// by a library upgrade fails to compile instead of passing through.
    fn step(&mut self) -> Result<Tok<'a>, FeedRule> {
        let position = self.reader.buffer_position();
        // The cause is dropped on purpose: it can quote body text (R-30).
        let event = self.reader.read_event().map_err(|_| FeedRule::Xml)?;
        match event {
            Event::DocType(_) => Err(FeedRule::Doctype),
            Event::PI(_) => Err(FeedRule::ProcessingInstruction),
            Event::Decl(_) => {
                if self.seen_decl || self.root_started {
                    return Err(FeedRule::XmlDeclaration);
                }
                if position != 0 {
                    return Err(FeedRule::Prolog);
                }
                // The declaration's attributes are never read (Frame D-3 C3):
                // only UTF-8 validity of the whole body counts.
                self.seen_decl = true;
                self.step()
            }
            Event::Start(e) => {
                self.root_started = true;
                self.depth += 1;
                if self.depth > MAX_DEPTH {
                    return Err(FeedRule::TooDeep);
                }
                Ok(Tok::Start(e))
            }
            Event::Empty(e) => {
                self.root_started = true;
                if self.depth + 1 > MAX_DEPTH {
                    return Err(FeedRule::TooDeep);
                }
                Ok(Tok::Empty(e))
            }
            Event::End(_) => {
                self.depth = self.depth.checked_sub(1).ok_or(FeedRule::Xml)?;
                Ok(Tok::End)
            }
            Event::Text(t) => Ok(Tok::Text(t)),
            Event::CData(_) => Ok(Tok::CData),
            Event::Comment(_) => Ok(Tok::Comment),
            Event::GeneralRef(r) => Ok(Tok::Ref(r)),
            Event::Eof => Ok(Tok::Eof),
        }
    }

    fn scan(mut self) -> Result<Scan, FeedRule> {
        self.prolog()?;
        let scan = self.rss_children()?;
        self.after_rss()?;
        Ok(scan)
    }

    /// Everything before `<rss>`: at most the declaration, which `step` has
    /// already checked sits at byte 0, then whitespace.
    fn prolog(&mut self) -> Result<(), FeedRule> {
        loop {
            match self.step()? {
                Tok::Text(t) if is_xml_whitespace(&t) => {}
                Tok::Text(_) | Tok::CData | Tok::Comment | Tok::Ref(_) => {
                    return Err(FeedRule::Prolog);
                }
                Tok::Start(e) if e.name().as_ref() == "rss" => return Ok(()),
                // `<rss/>` is a root with no `<channel>`: the channel rule.
                Tok::Empty(e) if e.name().as_ref() == "rss" => return Err(FeedRule::Channel),
                Tok::Start(_) | Tok::Empty(_) | Tok::End | Tok::Eof => return Err(FeedRule::Root),
            }
        }
    }

    /// `<rss>`'s content: exactly one `<channel>`, then `</rss>`.
    fn rss_children(&mut self) -> Result<Scan, FeedRule> {
        let mut scan = None;
        loop {
            match self.step()? {
                Tok::Text(t) if is_xml_whitespace(&t) => {}
                Tok::Text(_) | Tok::CData | Tok::Comment | Tok::Ref(_) => {
                    return Err(FeedRule::StrayContent);
                }
                Tok::Start(e) if e.name().as_ref() == "channel" && scan.is_none() => {
                    scan = Some(self.channel_children()?);
                }
                Tok::Empty(e) if e.name().as_ref() == "channel" && scan.is_none() => {
                    scan = Some(Scan {
                        first_20: Vec::new(),
                        items_seen: 0,
                    });
                }
                // another `<channel>`, or any other element child
                Tok::Start(_) | Tok::Empty(_) => return Err(FeedRule::Channel),
                Tok::End => return scan.ok_or(FeedRule::Channel),
                Tok::Eof => return Err(FeedRule::Close),
            }
        }
    }

    /// After `</rss>`: whitespace, then the end of the body.
    fn after_rss(&mut self) -> Result<(), FeedRule> {
        loop {
            match self.step()? {
                Tok::Text(t) if is_xml_whitespace(&t) => {}
                Tok::Eof => return Ok(()),
                Tok::Text(_)
                | Tok::CData
                | Tok::Comment
                | Tok::Ref(_)
                | Tok::Start(_)
                | Tok::Empty(_)
                | Tok::End => return Err(FeedRule::Close),
            }
        }
    }

    /// `<channel>`'s content, `<channel>` already consumed. Only direct
    /// `<item>` children count (R-8); every other child is skipped whole.
    fn channel_children(&mut self) -> Result<Scan, FeedRule> {
        let mut scan = Scan {
            first_20: Vec::new(),
            items_seen: 0,
        };
        loop {
            match self.step()? {
                Tok::Text(t) if is_xml_whitespace(&t) => {}
                Tok::Text(_) | Tok::CData | Tok::Comment | Tok::Ref(_) => {
                    return Err(FeedRule::StrayContent);
                }
                Tok::Start(e) if e.name().as_ref() == "item" => {
                    scan.items_seen += 1;
                    if scan.items_seen <= MAX_POSTS {
                        let post = self.item()?;
                        scan.first_20.push(post);
                    } else {
                        self.skip_element()?;
                    }
                }
                Tok::Empty(e) if e.name().as_ref() == "item" => {
                    scan.items_seen += 1;
                    if scan.items_seen <= MAX_POSTS {
                        scan.first_20.push(ItemAcc::default().finish());
                    }
                }
                Tok::Start(_) => self.skip_element()?,
                Tok::Empty(_) => {}
                Tok::End => return Ok(scan),
                Tok::Eof => return Err(FeedRule::Close),
            }
        }
    }

    /// Skip to the end tag matching the start tag just consumed. Content is
    /// never decoded or kept; `step` still enforces the whole-feed rules.
    fn skip_element(&mut self) -> Result<(), FeedRule> {
        let target = self.depth - 1;
        loop {
            match self.step()? {
                Tok::End if self.depth == target => return Ok(()),
                Tok::Eof => return Err(FeedRule::Close),
                Tok::Start(_)
                | Tok::Empty(_)
                | Tok::End
                | Tok::Text(_)
                | Tok::CData
                | Tok::Comment
                | Tok::Ref(_) => {}
            }
        }
    }

    /// One `<item>`, start tag consumed; returns at its matching end tag, so a
    /// rejected post resumes there (R-8).
    fn item(&mut self) -> Result<Result<Post, PostRule>, FeedRule> {
        let mut acc = ItemAcc::default();
        loop {
            match self.step()? {
                Tok::Text(t) if is_xml_whitespace(&t) => {}
                Tok::Text(_) | Tok::CData | Tok::Comment | Tok::Ref(_) => {
                    acc.flag(PostRule::StrayContent);
                }
                Tok::Start(e) => match ChildName::of(e.name().as_ref()) {
                    ChildName::Unknown => {
                        acc.flag(PostRule::UnknownChild);
                        self.skip_element()?;
                    }
                    known => {
                        // Attributes are never read; a field's start tag must
                        // be its bare name (R-9). `guid` may carry any.
                        let bare = e.len() == e.name().as_ref().len();
                        if !bare && known != ChildName::Guid {
                            acc.mark_seen(known);
                            acc.flag(PostRule::StartTagNotBare);
                            self.skip_element()?;
                        } else {
                            let read = self.field_text()?;
                            acc.take(known, read);
                        }
                    }
                },
                Tok::Empty(e) => match ChildName::of(e.name().as_ref()) {
                    ChildName::Unknown => acc.flag(PostRule::UnknownChild),
                    known => {
                        acc.mark_seen(known);
                        acc.flag(PostRule::SelfClosingChild);
                    }
                },
                Tok::End => return Ok(acc.finish()),
                Tok::Eof => return Err(FeedRule::Close),
            }
        }
    }

    /// A field's content under R-10, start tag consumed; returns at its end
    /// tag. Only text and the five named entities are accepted.
    fn field_text(&mut self) -> Result<FieldRead, FeedRule> {
        let mut text = String::new();
        let mut bad = None;
        loop {
            match self.step()? {
                Tok::Text(t) => text.push_str(&t),
                Tok::Ref(r) => match decode_entity(&r) {
                    Ok(c) => text.push(c),
                    Err(rule) => {
                        bad.get_or_insert(rule);
                    }
                },
                Tok::CData => {
                    bad.get_or_insert(PostRule::CDataInField);
                }
                Tok::Comment => {
                    bad.get_or_insert(PostRule::CommentInField);
                }
                Tok::Start(_) => {
                    bad.get_or_insert(PostRule::ElementInField);
                    self.skip_element()?;
                }
                Tok::Empty(_) => {
                    bad.get_or_insert(PostRule::ElementInField);
                }
                Tok::End => {
                    // R-10: decoded `<` is refused in every field, guid
                    // included, before any field rule reads the text.
                    if bad.is_none() && text.contains('<') {
                        bad = Some(PostRule::AngleBracketInField);
                    }
                    return Ok(FieldRead { text, bad });
                }
                Tok::Eof => return Err(FeedRule::Close),
            }
        }
    }
}

/// XML whitespace is these four bytes, not Unicode whitespace.
fn is_xml_whitespace(s: &str) -> bool {
    s.bytes().all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
}

/// R-10: the five named entities and nothing else.
fn decode_entity(r: &BytesRef<'_>) -> Result<char, PostRule> {
    match &**r {
        "lt" => Ok('<'),
        "gt" => Ok('>'),
        "amp" => Ok('&'),
        "quot" => Ok('"'),
        "apos" => Ok('\''),
        name if name.starts_with('#') => Err(PostRule::NumericReference),
        _ => Err(PostRule::UnknownEntity),
    }
}

struct FieldRead {
    text: String,
    bad: Option<PostRule>,
}

/// An item child, by exact qualified name (R-9): `atom:link` is `Unknown`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ChildName {
    Field(Field),
    Guid,
    Unknown,
}

impl ChildName {
    fn of(name: &str) -> Self {
        match name {
            "title" => ChildName::Field(Field::Title),
            "link" => ChildName::Field(Field::Link),
            "description" => ChildName::Field(Field::Description),
            "pubDate" => ChildName::Field(Field::PubDate),
            "guid" => ChildName::Guid,
            _ => ChildName::Unknown,
        }
    }
}

/// What one `<item>` has shown so far. The first rule broken is the one kept.
#[derive(Default)]
struct ItemAcc {
    title: Slot,
    link: Slot,
    description: Slot,
    pub_date: Slot,
    guid: Slot,
    problem: Option<PostRule>,
}

#[derive(Default)]
struct Slot {
    seen: bool,
    text: String,
}

impl ItemAcc {
    fn flag(&mut self, rule: PostRule) {
        self.problem.get_or_insert(rule);
    }

    fn slot(&mut self, name: ChildName) -> Option<&mut Slot> {
        match name {
            ChildName::Field(Field::Title) => Some(&mut self.title),
            ChildName::Field(Field::Link) => Some(&mut self.link),
            ChildName::Field(Field::Description) => Some(&mut self.description),
            ChildName::Field(Field::PubDate) => Some(&mut self.pub_date),
            ChildName::Guid => Some(&mut self.guid),
            ChildName::Unknown => None,
        }
    }

    /// The child was present, whatever its content: no `MissingField` for it.
    fn mark_seen(&mut self, name: ChildName) {
        if let Some(slot) = self.slot(name) {
            slot.seen = true;
        }
    }

    fn take(&mut self, name: ChildName, read: FieldRead) {
        let Some(slot) = self.slot(name) else {
            return;
        };
        let duplicate = std::mem::replace(&mut slot.seen, true);
        let rule = match read.bad {
            Some(bad) => Some(bad),
            None if read.text.is_empty() => Some(PostRule::EmptyField),
            None if duplicate => Some(PostRule::DuplicateField),
            None => {
                slot.text = read.text;
                None
            }
        };
        if let Some(rule) = rule {
            self.flag(rule);
        }
    }

    /// Missing fields first, so an `<atom:title>` item reads as a missing
    /// title (R-7), then the first rule broken, then R-10 and R-11 on values.
    fn finish(self) -> Result<Post, PostRule> {
        for (slot, field) in [
            (&self.title, Field::Title),
            (&self.link, Field::Link),
            (&self.description, Field::Description),
            (&self.pub_date, Field::PubDate),
        ] {
            if !slot.seen {
                return Err(PostRule::MissingField(field));
            }
        }
        if let Some(rule) = self.problem {
            return Err(rule);
        }
        validate_post(
            self.title.text,
            self.description.text,
            self.link.text,
            &self.pub_date.text,
        )
    }
}

/// R-11, through the articles validator so the two never disagree: the post is
/// wrapped in a one-post document and parsed by [`articles::parse`].
fn validate_post(
    title: String,
    summary: String,
    link: String,
    pub_date: &str,
) -> Result<Post, PostRule> {
    let published = convert_pub_date(pub_date).ok_or(PostRule::PubDate)?;
    let post = Post {
        title,
        summary,
        link,
        published,
    };
    // Any valid instant: the post's own rules do not read it.
    let probe = document_of("1970-01-01T00:00:00.000Z", std::slice::from_ref(&post));
    match articles::parse(&probe) {
        Ok(_) => Ok(post),
        Err(ArticlesError::Title { rule, .. }) => Err(PostRule::Title(rule)),
        Err(ArticlesError::Summary { rule, .. }) => Err(PostRule::Summary(rule)),
        Err(ArticlesError::Link { rule, .. }) => Err(PostRule::Link(rule)),
        Err(ArticlesError::Published { .. }) => Err(PostRule::PubDate),
        // The probe is built here: these cannot come from a post's fields.
        // Refuse rather than accept a post the validator would not.
        Err(
            ArticlesError::Json
            | ArticlesError::NotAnObject { .. }
            | ArticlesError::FetchedAt(_)
            | ArticlesError::PostCount { .. },
        ) => Err(PostRule::Validator),
    }
}

/// R-11: at most 64 bytes, RFC 2822, stored in UTC in the `fetched_at` grammar.
fn convert_pub_date(raw: &str) -> Option<String> {
    if raw.len() > MAX_PUB_DATE_BYTES {
        return None;
    }
    let instant = OffsetDateTime::parse(raw, &Rfc2822).ok()?;
    format_fetched_at(instant).ok()
}

fn document_of(fetched_at: &str, posts: &[Post]) -> String {
    let posts: Vec<serde_json::Value> = posts
        .iter()
        .map(|p| {
            serde_json::json!({
                "title": p.title,
                "summary": p.summary,
                "link": p.link,
                "published": p.published,
            })
        })
        .collect();
    serde_json::json!({ "fetched_at": fetched_at, "posts": posts }).to_string()
}

/// R-12: keep the valid posts in order, record the skips, and confirm the
/// whole document with the validator the refresher will run.
fn assemble(scan: Scan, fetched_at: &str) -> Result<FeedOutcome, FeedRule> {
    if scan.items_seen == 0 {
        return Err(FeedRule::NoItems);
    }
    let mut posts = Vec::new();
    let mut skipped = Vec::new();
    for (index, result) in scan.first_20.into_iter().enumerate() {
        match result {
            Ok(post) => posts.push(post),
            Err(rule) => skipped.push(SkippedItem { index, rule }),
        }
    }
    if posts.is_empty() {
        return Err(FeedRule::NoValidPost);
    }
    let document = document_of(fetched_at, &posts);
    articles::parse(&document).map_err(|_| FeedRule::DocumentInvalid)?;
    Ok(FeedOutcome { document, skipped })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::live_lists::articles;
    use serde_json::Value;

    const FETCHED_AT: &str = "2026-11-24T12:00:00.000Z";

    fn now() -> OffsetDateTime {
        time::macros::datetime!(2026-11-24 12:00 UTC)
    }
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
        parse_feed(body.as_bytes(), now())
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
        let out = parse_feed(CAPTURE.as_bytes(), now()).expect("capture parses");
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
            assert_eq!(parse_feed(&body, now()), Err(FeedRule::NotUtf8));
        }
        let mut body = feed(&good(1)).into_bytes();
        body.extend_from_slice(b"\n\xFF");
        assert_eq!(parse_feed(&body, now()), Err(FeedRule::NotUtf8));
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
    fn r7_namespaces_are_not_processed() {
        // Given a prefix bound to the same namespace as the default one, so a
        // namespace-aware reader would read <p:item> as <item>
        let body = format!(
            "<rss xmlns=\"urn:a\" xmlns:p=\"urn:a\"><channel>{}</channel></rss>",
            good(1).replace("item", "p:item")
        );
        // Then it is an unknown channel child, not an item
        fails(&body, FeedRule::NoItems);
        // and a prefixed field is an unknown child, not the field
        let prefixed = good(1)
            .replace("<title>", "<p:title>")
            .replace("</title>", "</p:title>");
        let body = format!(
            "<rss xmlns:p=\"urn:a\"><channel>{prefixed}{}</channel></rss>",
            good(2)
        );
        let out = run(&body).expect("accepted");
        assert_eq!(
            out.skipped,
            vec![SkippedItem {
                index: 0,
                rule: PostRule::MissingField(Field::Title)
            }]
        );
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
        rejects_post(
            &good_plus("<guid>a&#65;</guid>"),
            PostRule::NumericReference,
        );
        rejects_post(&good_plus("<guid>a&nbsp;</guid>"), PostRule::UnknownEntity);
        rejects_post(&good_plus("<guid>a<b/></guid>"), PostRule::ElementInField);
    }

    #[test]
    fn r9_guid_content_is_discarded_never_stored() {
        let out = run(&feed(&good_plus("<guid>SENTINEL-GUID-TEXT</guid>"))).expect("accepted");
        assert_eq!(titles(&out), vec!["T".to_owned()]);
        assert!(!out.document.contains("SENTINEL-GUID-TEXT"));
    }

    #[test]
    fn r9_comment_inside_guid_rejects_the_post() {
        rejects_post(
            &good_plus("<guid>a<!-- c -->b</guid>"),
            PostRule::CommentInField,
        );
    }

    #[test]
    fn r10_decoded_less_than_in_guid_rejects_the_post() {
        rejects_post(
            &good_plus("<guid>a&lt;b</guid>"),
            PostRule::AngleBracketInField,
        );
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
    fn r10_decoded_text_containing_less_than_rejects_the_post_in_every_field() {
        for (f, tag, _) in FIELDS {
            rejects_post(
                &good_with(f, &format!("<{tag}>a&lt;b</{tag}>")),
                PostRule::AngleBracketInField,
            );
        }
        rejects_post(
            &good_with(Field::Description, "<description>&lt;b&gt;</description>"),
            PostRule::AngleBracketInField,
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
        rejects_post(
            &date_item("Tue, 31 Feb 2026 00:00:00 GMT"),
            PostRule::PubDate,
        );
    }

    #[test]
    fn r11_pubdate_of_exactly_64_bytes_is_accepted_and_65_is_rejected() {
        // time accepts trailing folding whitespace, so padding isolates length
        let base = "Tue, 28 Apr 2026 00:00:00 GMT";
        let at_64 = format!("{base}{}", " ".repeat(64 - base.len()));
        assert_eq!(at_64.len(), 64);
        let out = run(&feed(&date_item(&at_64))).expect("64 bytes accepted");
        assert_eq!(
            doc(&out)["posts"][0]["published"],
            "2026-04-28T00:00:00.000Z"
        );
        rejects_post(&date_item(&format!("{at_64} ")), PostRule::PubDate);
    }

    #[test]
    fn r11_pubdate_whose_utc_year_is_past_9999_rejects_the_post_and_never_panics() {
        // 23:00 at -01:00 on 31 Dec 9999 is year 10000 in UTC (R-13, Frame C7)
        rejects_post(
            &date_item("Fri, 31 Dec 9999 23:00:00 -0100"),
            PostRule::PubDate,
        );
    }

    #[test]
    fn r11_pubdate_is_stored_in_utc_in_the_fetched_at_grammar() {
        for (raw, stored) in [
            ("Tue, 28 Apr 2026 00:00:00 GMT", "2026-04-28T00:00:00.000Z"),
            (
                "Tue, 28 Apr 2026 02:00:00 +0200",
                "2026-04-28T00:00:00.000Z",
            ),
            (
                "Mon, 27 Apr 2026 19:30:15 -0430",
                "2026-04-28T00:00:15.000Z",
            ),
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
            &feed(&format!(
                "{twenty}<item><title>T<!DOCTYPE x></title></item>"
            )),
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
    fn r12_a_feed_whose_posts_are_all_bad_fails_with_no_valid_post() {
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

    // ---- the negative fixture table (R-13 runs it through a live refresher) ----

    /// What a negative fixture must do to the feed.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Verdict {
        /// The whole feed fails with this rule.
        Feed(FeedRule),
        /// The feed is accepted and item 0 is skipped with this rule.
        Post(PostRule),
    }

    /// One negative fixture per R-7 to R-12 bullet and listed alternative
    /// (R-14), as `(name, body, verdict)`. A body here is a failed feed or a
    /// skipped post; no entry is a valid feed.
    pub(crate) fn negative_fixtures() -> Vec<(&'static str, Vec<u8>, Verdict)> {
        let mut out: Vec<(&'static str, Vec<u8>, Verdict)> = Vec::new();
        let mut feed_fail = |name: &'static str, body: String, rule: FeedRule| {
            out.push((name, body.into_bytes(), Verdict::Feed(rule)));
        };
        let dt = "<!DOCTYPE rss>";
        let pi = "<?xml-stylesheet href=\"a.css\"?>";
        let decl = "<?xml version=\"1.0\"?>";
        let twenty: String = (0..20).map(good).collect();
        let ch = format!("<channel>{}</channel>", good(1));

        feed_fail(
            "r7 parse error: mismatched end tag",
            "<rss><channel><item></channel></rss>".into(),
            FeedRule::Xml,
        );
        feed_fail(
            "r7 parse error: bare ampersand",
            feed(&good_with(Field::Title, "<title>A & B</title>")),
            FeedRule::Xml,
        );
        feed_fail(
            "r7 prolog: text",
            format!("junk{}", feed_plain(&good(1))),
            FeedRule::Prolog,
        );
        feed_fail(
            "r7 prolog: comment",
            format!("<!-- c -->{}", feed_plain(&good(1))),
            FeedRule::Prolog,
        );
        feed_fail(
            "r7 prolog: byte-order mark",
            format!("\u{FEFF}{}", feed_plain(&good(1))),
            FeedRule::Prolog,
        );
        feed_fail(
            "r7 doctype: prolog",
            format!("{dt}{}", feed(&good(1))),
            FeedRule::Doctype,
        );
        feed_fail(
            "r7 doctype: inside a field",
            feed(&good_with(Field::Title, &format!("<title>T{dt}</title>"))),
            FeedRule::Doctype,
        );
        feed_fail(
            "r7 doctype: inside discarded content",
            feed(&format!("<language>en{dt}</language>{}", good(1))),
            FeedRule::Doctype,
        );
        feed_fail(
            "r7 doctype: after the 20th item",
            feed(&format!("{twenty}<item><title>T{dt}</title></item>")),
            FeedRule::Doctype,
        );
        feed_fail(
            "r7 processing instruction: inside a field",
            feed(&good_with(Field::Title, &format!("<title>T{pi}</title>"))),
            FeedRule::ProcessingInstruction,
        );
        feed_fail(
            "r7 second declaration",
            format!("{decl}{}", feed(&good(1))),
            FeedRule::XmlDeclaration,
        );
        feed_fail(
            "r7 root other than rss",
            "<feed><channel/></feed>".into(),
            FeedRule::Root,
        );
        feed_fail(
            "r7 second channel",
            format!("<rss>{ch}{ch}</rss>"),
            FeedRule::Channel,
        );
        feed_fail(
            "r7 element beside channel",
            format!("<rss>{ch}<extra/></rss>"),
            FeedRule::Channel,
        );
        feed_fail(
            "r7 text after </rss>",
            format!("<rss>{ch}</rss>x"),
            FeedRule::Close,
        );
        feed_fail(
            "r7 truncated before </rss>",
            format!("<rss>{ch}"),
            FeedRule::Close,
        );
        feed_fail(
            "r7 text under rss",
            format!("<rss>x{ch}</rss>"),
            FeedRule::StrayContent,
        );
        feed_fail(
            "r7 cdata under channel",
            feed(&format!("{}<![CDATA[x]]>", good(1))),
            FeedRule::StrayContent,
        );
        feed_fail(
            "r7 comment under channel",
            feed(&format!("<!-- c -->{}", good(1))),
            FeedRule::StrayContent,
        );
        feed_fail(
            "r7 nesting 17 deep",
            feed_plain(&format!("{}{}", nested_to(17), good(1))),
            FeedRule::TooDeep,
        );
        feed_fail(
            "r7 body over the cap",
            format!("{}{}", feed(&good(1)), " ".repeat(MAX_BODY_BYTES)),
            FeedRule::TooLarge,
        );
        feed_fail("r12 no item", feed(""), FeedRule::NoItems);
        feed_fail(
            "r12 every post bad",
            feed("<item></item><item></item>"),
            FeedRule::NoValidPost,
        );
        feed_fail(
            "r12 doctype in the 21st item",
            feed(&format!("{twenty}<item><title>T{dt}</title></item>")),
            FeedRule::Doctype,
        );
        // not valid UTF-8, anywhere
        let mut bytes = feed(&good(1)).into_bytes();
        bytes.extend_from_slice(b"\n\xFF");
        out.push(("r7 invalid UTF-8", bytes, Verdict::Feed(FeedRule::NotUtf8)));

        let mut post_fail = |name: &'static str, item: String, rule: PostRule| {
            let body = feed(&format!("{item}{}", good(1)));
            out.push((name, body.into_bytes(), Verdict::Post(rule)));
        };
        post_fail(
            "r8 item inside a rejected item",
            good_plus(&format!("<extra>{}</extra>", good(9))),
            PostRule::UnknownChild,
        );
        post_fail(
            "r9 atom:link child",
            good_plus("<atom:link href=\"x\"/>"),
            PostRule::UnknownChild,
        );
        post_fail(
            "r9 self-closing guid",
            good_plus("<guid/>"),
            PostRule::SelfClosingChild,
        );
        post_fail(
            "r9 duplicate guid",
            good_plus("<guid>a</guid><guid>b</guid>"),
            PostRule::DuplicateField,
        );
        post_fail(
            "r9 empty guid",
            good_plus("<guid></guid>"),
            PostRule::EmptyField,
        );
        post_fail(
            "r9 cdata in guid",
            good_plus("<guid><![CDATA[x]]></guid>"),
            PostRule::CDataInField,
        );
        post_fail(
            "r9 missing link",
            "<item><title>T</title><description>D</description><pubDate>Tue, 28 Apr 2026 00:00:00 GMT</pubDate></item>".into(),
            PostRule::MissingField(Field::Link),
        );
        post_fail(
            "r9 field start tag with an attribute",
            good_with(Field::Title, "<title lang=\"en\">T</title>"),
            PostRule::StartTagNotBare,
        );
        post_fail(
            "r9 text directly in item",
            good_plus("stray"),
            PostRule::StrayContent,
        );
        post_fail(
            "r10 element in a field",
            good_with(Field::Title, "<title>a<b>c</b></title>"),
            PostRule::ElementInField,
        );
        post_fail(
            "r10 cdata in a field",
            good_with(Field::Title, "<title><![CDATA[x]]></title>"),
            PostRule::CDataInField,
        );
        post_fail(
            "r10 comment in a field",
            good_with(Field::Title, "<title>a<!-- c -->b</title>"),
            PostRule::CommentInField,
        );
        post_fail(
            "r10 numeric reference",
            good_with(Field::Title, "<title>a&#65;</title>"),
            PostRule::NumericReference,
        );
        post_fail(
            "r10 unknown entity",
            good_with(Field::Title, "<title>a&nbsp;</title>"),
            PostRule::UnknownEntity,
        );
        post_fail(
            "r10 decoded less-than",
            good_with(Field::Title, "<title>a&lt;b</title>"),
            PostRule::AngleBracketInField,
        );
        post_fail(
            "r11 title over 200",
            good_with(Field::Title, &format!("<title>{}</title>", "t".repeat(201))),
            PostRule::Title(TextRule::TooLong),
        );
        post_fail(
            "r11 summary over 600",
            good_with(
                Field::Description,
                &format!("<description>{}</description>", "d".repeat(601)),
            ),
            PostRule::Summary(TextRule::TooLong),
        );
        post_fail(
            "r11 rejected char in the title",
            good_with(Field::Title, "<title>a\u{2028}b</title>"),
            PostRule::Title(TextRule::RejectedChar),
        );
        post_fail(
            "r11 link host",
            link_item("https://www.mnemra.dev/blog/x/"),
            PostRule::Link(LinkRule::Host),
        );
        post_fail(
            "r11 link scheme",
            link_item("http://mnemra.dev/blog/x/"),
            PostRule::Link(LinkRule::Scheme),
        );
        post_fail(
            "r11 link query",
            link_item("https://mnemra.dev/blog/x/?a=1"),
            PostRule::Link(LinkRule::Query),
        );
        post_fail(
            "r11 link path",
            link_item("https://mnemra.dev/about/"),
            PostRule::Link(LinkRule::Path),
        );
        post_fail(
            "r11 pubDate not RFC 2822",
            date_item("yesterday"),
            PostRule::PubDate,
        );
        post_fail(
            "r11 pubDate in year 10000 UTC",
            date_item("Fri, 31 Dec 9999 23:00:00 -0100"),
            PostRule::PubDate,
        );
        out
    }

    #[test]
    fn every_negative_fixture_is_negative_for_its_stated_rule() {
        // Given the table R-13 runs through a live refresher
        let fixtures = negative_fixtures();
        assert!(fixtures.len() >= 45, "{} fixtures", fixtures.len());
        for (name, body, verdict) in fixtures {
            // Then each fails the feed, or skips item 0, by exactly its rule
            let got = match parse_feed(&body, now()) {
                Err(rule) => Verdict::Feed(rule),
                Ok(out) => {
                    assert_eq!(out.skipped.len(), 1, "{name}: {:?}", out.skipped);
                    assert_eq!(out.skipped[0].index, 0, "{name}");
                    Verdict::Post(out.skipped[0].rule)
                }
            };
            assert_eq!(got, verdict, "{name}");
        }
    }

    // ---- declarations and encoding ----

    #[test]
    fn r7_declaration_after_the_20th_item_or_without_a_leading_one_is_xml_declaration() {
        let decl = "<?xml version=\"1.0\"?>";
        let twenty: String = (0..20).map(good).collect();
        // with a leading declaration, a second one inside the 21st item
        fails(
            &feed(&format!("{twenty}<item>{decl}</item>")),
            FeedRule::XmlDeclaration,
        );
        // with none, a declaration after the 20th item, inside a field, or
        // after </rss> is the same rule, not a prolog violation
        fails(
            &feed_plain(&format!("{twenty}<item>{decl}</item>")),
            FeedRule::XmlDeclaration,
        );
        fails(
            &feed_plain(&good_with(Field::Title, &format!("<title>T{decl}</title>"))),
            FeedRule::XmlDeclaration,
        );
        fails(
            &format!("{}{decl}", feed_plain(&good(1))),
            FeedRule::XmlDeclaration,
        );
    }

    #[test]
    fn r7_the_declarations_encoding_attribute_is_never_read() {
        // Frame D-3 C3: "Attributes are never parsed". Only the UTF-8
        // validity of the whole body counts, so an ISO-8859-1 declaration over
        // bytes that are valid UTF-8 is accepted.
        accepted_one(&format!(
            "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>{}",
            feed_plain(&good(1))
        ));
    }

    #[test]
    fn r7_a_clock_instant_the_grammar_cannot_hold_is_an_internal_failure_not_a_panic() {
        // 23:00 at -01:00 on 31 Dec 9999 is year 10000 in UTC
        let at = time::macros::datetime!(9999-12-31 23:00 -1);
        assert_eq!(
            parse_feed(feed(&good(1)).as_bytes(), at),
            Err(FeedRule::DocumentInvalid)
        );
    }

    // ---- untrimmed text (no trim rule in R-10 or R-11) ----

    #[test]
    fn r10_a_whitespace_only_title_is_accepted_and_stored_as_written() {
        // Pinned, not endorsed: the spec has no trim rule, so a title of one
        // space is non-empty and holds no rejected-class code point.
        let out = run(&feed(&good_with(Field::Title, "<title> </title>"))).expect("accepted");
        assert_eq!(titles(&out), vec![" ".to_owned()]);
    }

    #[test]
    fn r11_a_pretty_printed_field_with_a_newline_is_rejected_as_a_rejected_char() {
        rejects_post(
            &good_with(Field::Title, "<title>\n  T\n</title>"),
            PostRule::Title(TextRule::RejectedChar),
        );
        rejects_post(
            &good_with(Field::Description, "<description>\n  D\n</description>"),
            PostRule::Summary(TextRule::RejectedChar),
        );
    }

    // ---- no body text through Debug, and the reader's configuration ----

    #[test]
    fn outcome_debug_carries_the_document_length_and_skips_never_its_text() {
        let out = run(&feed(&format!(
            "{}{}",
            good_with(Field::Title, "<title>SECRETTITLE</title>"),
            good_plus("<extra/>")
        )))
        .expect("accepted");
        // item 0 is kept, item 1 is skipped; the kept title is in `document`
        let shown = format!("{out:?}");
        assert!(!shown.contains("SECRETTITLE"), "{shown}");
        assert!(
            shown.contains(&format!("document_len: {}", out.document.len())),
            "{shown}"
        );
        assert!(shown.contains("UnknownChild"), "{shown}");
    }

    const PINNED_READER_CONFIG: &str = "Config { allow_dangling_amp: false, allow_unmatched_ends: false, check_comments: true, check_end_names: true, expand_empty_elements: false, trim_markup_names_in_closing_tags: true, trim_text_start: false, trim_text_end: false }";

    #[test]
    fn reader_configuration_is_exactly_the_pinned_one() {
        // `Config` is #[non_exhaustive]: a library upgrade can add an option
        // with a default this module has never seen. Its Debug text changes
        // when one does, which fails here and sends the reader to the changelog.
        let scanner = Scanner::new("");
        assert_eq!(
            format!("{:?}", scanner.reader.config()),
            PINNED_READER_CONFIG
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
            if let Ok(out) = parse_feed(&input, now()) {
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
            let _ = parse_feed(&bytes, now());
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
            let out = parse_feed(body.as_bytes(), now())
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
