//! Validator for the public list file `projects.json` (spec R-8, R-10, R-18,
//! Data Model).
//!
//! Untrusted input: the file is written by the deploy job from GitHub data and
//! read by the server (R-10) and by `folio check-projects` (R-18). Both reach
//! it through [`load`] / [`parse`], so they cannot disagree. This is a
//! fail-closed control: [`ProjectsList`] can only be built by a successful
//! validation, every unenumerated shape is an error, and no error path
//! substitutes a default. The root, every group and every repo must be a JSON
//! object: serde's derived `Deserialize` also accepts a positional array for a
//! struct, and `deny_unknown_fields` does not cover that, so [`parse`] checks
//! the shape separately.
//!
//! Freshness is a separate question ([`check_fetched_at_age`]): the startup
//! load must not apply it (a Fly restart must not depend on deploy age), so
//! [`parse`] never does.

use std::fmt;
use std::path::Path;

use serde::Deserialize;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use crate::ai::types::is_invisible;

/// Owners in file order. `groups` is exactly these two, in this order.
pub const OWNERS: [&str; 2] = ["mnemra", "prmanahan"];

/// A list file at most this far behind the check's clock passes (inclusive).
pub const MAX_AGE_BEHIND: Duration = Duration::minutes(30);
/// A list file at most this far ahead of the check's clock passes (inclusive).
pub const MAX_AGE_AHEAD: Duration = Duration::minutes(5);

const MAX_NAME_LEN: usize = 100;
const MAX_DESCRIPTION_CODE_POINTS: usize = 350;
/// `YYYY-MM-DDTHH:MM:SS.sssZ`
const FETCHED_AT_LEN: usize = 24;
/// U+2028 and U+2029, by number: the rejected class is never typed as a
/// literal in source.
const LINE_SEPARATORS: std::ops::RangeInclusive<u32> = 0x2028..=0x2029;

// ---- wire shape: every level rejects unknown fields ----

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawList {
    fetched_at: String,
    groups: Vec<RawGroup>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGroup {
    owner: String,
    repos: Vec<RawRepo>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRepo {
    name: String,
    description: String,
}

// ---- validated types ----

/// A list file that passed every Data Model rule. Fields are private: the
/// only constructors are [`parse`] and [`load`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectsList {
    fetched_at: String,
    fetched_at_instant: OffsetDateTime,
    groups: Vec<Group>,
}

/// One owner's repos, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    owner: &'static str,
    repos: Vec<Repo>,
}

/// One public repo: a name and GitHub's description. The link is composed by
/// the reader, not stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    name: String,
    description: String,
}

impl ProjectsList {
    /// The `fetched_at` literal, exactly as written in the file.
    pub fn fetched_at(&self) -> &str {
        &self.fetched_at
    }

    /// `fetched_at` parsed to an instant (UTC, millisecond precision).
    pub fn fetched_at_instant(&self) -> OffsetDateTime {
        self.fetched_at_instant
    }

    /// Exactly [`OWNERS`], in that order.
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }

    /// `(owner, repo count)` per group, in file order.
    pub fn counts(&self) -> impl Iterator<Item = (&'static str, usize)> + '_ {
        self.groups.iter().map(|g| (g.owner, g.repos.len()))
    }
}

impl Group {
    /// One of [`OWNERS`].
    pub fn owner(&self) -> &'static str {
        self.owner
    }

    /// Non-empty, strictly increasing by ASCII-lowercased name.
    pub fn repos(&self) -> &[Repo] {
        &self.repos
    }
}

impl Repo {
    /// Matches `^[A-Za-z0-9._-]{1,100}$`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `""` when GitHub's description was null.
    pub fn description(&self) -> &str {
        &self.description
    }
}

// ---- errors ----

/// Why a list file was refused. Every `Display` arm that interpolates text
/// taken from the file (a repo name, an owner found in the file, the serde
/// message) does so through `{:?}`, whatever order the checks ran in, so a
/// newline or escape sequence in the file cannot forge a log line. The
/// `&'static str` owners and numbers come from this module, not the file.
#[derive(Debug)]
pub enum ListError {
    Read(std::io::Error),
    Json(serde_json::Error),
    NotAnObject {
        at: &'static str,
    },
    FetchedAt(FetchedAtError),
    GroupCount {
        found: usize,
    },
    GroupOwner {
        index: usize,
        found: String,
    },
    EmptyOwner {
        owner: &'static str,
    },
    RepoName {
        owner: &'static str,
        name: String,
    },
    RepoOrder {
        owner: &'static str,
        name: String,
    },
    DescriptionTooLong {
        owner: &'static str,
        name: String,
        code_points: usize,
    },
    DescriptionRejectedChar {
        owner: &'static str,
        name: String,
        code_point: u32,
    },
}

impl fmt::Display for ListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(e) => write!(f, "cannot read the list file: {e}"),
            Self::Json(e) => write!(
                f,
                "the list file breaks the Data Model: {:?}",
                e.to_string()
            ),
            Self::NotAnObject { at } => {
                write!(f, "the {at} of the list file must be a JSON object")
            }
            Self::FetchedAt(e) => write!(f, "{e}"),
            Self::GroupCount { found } => write!(
                f,
                "groups must hold exactly {} entries, found {found}",
                OWNERS.len()
            ),
            Self::GroupOwner { index, found } => write!(
                f,
                "groups[{index}].owner must be {:?}, found {found:?}",
                OWNERS[*index]
            ),
            Self::EmptyOwner { owner } => write!(f, "owner {owner} has zero repos"),
            Self::RepoName { owner, name } => write!(
                f,
                "{owner}: repo name {name:?} does not match ^[A-Za-z0-9._-]{{1,100}}$"
            ),
            Self::RepoOrder { owner, name } => write!(
                f,
                "{owner}: repo {name:?} is not strictly after its predecessor in \
                 ASCII-lowercased name order"
            ),
            Self::DescriptionTooLong {
                owner,
                name,
                code_points,
            } => write!(
                f,
                "{owner}/{name:?}: description is {code_points} code points, the limit is \
                 {MAX_DESCRIPTION_CODE_POINTS}"
            ),
            Self::DescriptionRejectedChar {
                owner,
                name,
                code_point,
            } => write!(
                f,
                "{owner}/{name:?}: description holds rejected code point U+{code_point:04X}"
            ),
        }
    }
}

impl std::error::Error for ListError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::FetchedAt(e) => Some(e),
            _ => None,
        }
    }
}

/// The `fetched_at` literal is outside `YYYY-MM-DDTHH:MM:SS.sssZ` or is not a
/// calendar instant.
#[derive(Debug, PartialEq, Eq)]
pub enum FetchedAtError {
    Grammar,
    NotAnInstant,
}

impl fmt::Display for FetchedAtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grammar => write!(f, "fetched_at is not exactly YYYY-MM-DDTHH:MM:SS.sssZ"),
            Self::NotAnInstant => write!(f, "fetched_at is not a valid calendar instant"),
        }
    }
}

impl std::error::Error for FetchedAtError {}

/// Outside the freshness bound, or `fetched_at` itself is invalid.
#[derive(Debug, PartialEq, Eq)]
pub enum AgeError {
    Invalid(FetchedAtError),
    TooOld { fetched_at: String, age: Duration },
    TooFarAhead { fetched_at: String, ahead: Duration },
}

impl fmt::Display for AgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(e) => write!(f, "{e}"),
            Self::TooOld { fetched_at, age } => write!(
                f,
                "fetched_at {fetched_at:?} is stale: {} seconds old, the bound is {} minutes",
                age.whole_seconds(),
                MAX_AGE_BEHIND.whole_minutes()
            ),
            Self::TooFarAhead { fetched_at, ahead } => write!(
                f,
                "fetched_at {fetched_at:?} is in the future: {} seconds ahead, the bound is \
                 {} minutes",
                ahead.whole_seconds(),
                MAX_AGE_AHEAD.whole_minutes()
            ),
        }
    }
}

impl std::error::Error for AgeError {}

// ---- fetched_at ----

/// Check the exact grammar, then parse as RFC 3339. The grammar check comes
/// first because Rfc3339 alone accepts offsets, lowercase `t`/`z` and other
/// fraction lengths that the fetch script never writes.
pub fn parse_fetched_at(s: &str) -> Result<OffsetDateTime, FetchedAtError> {
    if !matches_fetched_at_grammar(s) {
        return Err(FetchedAtError::Grammar);
    }
    OffsetDateTime::parse(s, &Rfc3339).map_err(|_| FetchedAtError::NotAnInstant)
}

fn matches_fetched_at_grammar(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != FETCHED_AT_LEN {
        return false;
    }
    b.iter().enumerate().all(|(i, &c)| match i {
        // Seconds 00..=59: Rfc3339 accepts a month-end `:60`, which the
        // fetch script's `toISOString` never writes.
        17 => (b'0'..=b'5').contains(&c),
        4 | 7 => c == b'-',
        10 => c == b'T',
        13 | 16 => c == b':',
        19 => c == b'.',
        23 => c == b'Z',
        _ => c.is_ascii_digit(),
    })
}

/// How far `fetched_at` is behind `now`; negative when ahead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Age(pub Duration);

/// R-18's freshness bound: `fetched_at` may be at most [`MAX_AGE_BEHIND`]
/// before `now` and at most [`MAX_AGE_AHEAD`] after it, both inclusive.
/// `now` is a parameter so the verdict never depends on the wall clock.
///
/// # Errors
/// [`AgeError::Invalid`] when `fetched_at` breaks the grammar,
/// [`AgeError::TooOld`] or [`AgeError::TooFarAhead`] outside the bound.
pub fn check_fetched_at_age(fetched_at: &str, now: OffsetDateTime) -> Result<Age, AgeError> {
    let instant = parse_fetched_at(fetched_at).map_err(AgeError::Invalid)?;
    let age = now - instant;
    if age > MAX_AGE_BEHIND {
        return Err(AgeError::TooOld {
            fetched_at: fetched_at.to_owned(),
            age,
        });
    }
    if -age > MAX_AGE_AHEAD {
        return Err(AgeError::TooFarAhead {
            fetched_at: fetched_at.to_owned(),
            ahead: -age,
        });
    }
    Ok(Age(age))
}

/// [`check_fetched_at_age`] against the wall clock. The one place the clock is
/// read, so the binary needs no direct `time` dependency.
///
/// # Errors
/// As [`check_fetched_at_age`].
pub fn check_fetched_at_age_now(fetched_at: &str) -> Result<Age, AgeError> {
    check_fetched_at_age(fetched_at, OffsetDateTime::now_utc())
}

// ---- validation ----

/// Read and validate the file at `path`. Does not apply the age bound.
///
/// # Errors
/// [`ListError::Read`] when the file cannot be read as UTF-8, including a
/// missing file and a permission error; otherwise as [`parse`].
pub fn load(path: &Path) -> Result<ProjectsList, ListError> {
    let text = std::fs::read_to_string(path).map_err(ListError::Read)?;
    parse(&text)
}

/// Validate list-file text against every Data Model rule. Does not apply the
/// age bound.
///
/// # Errors
/// A [`ListError`] naming the first Data Model rule the text breaks.
pub fn parse(text: &str) -> Result<ProjectsList, ListError> {
    // Positional arrays satisfy a derived struct `Deserialize`; require
    // objects first. `Value` keeps the last of a duplicate key, so the strict
    // typed parse below still runs and rejects duplicates.
    let value: serde_json::Value = serde_json::from_str(text).map_err(ListError::Json)?;
    require_objects(&value)?;
    let raw: RawList = serde_json::from_str(text).map_err(ListError::Json)?;
    let fetched_at_instant = parse_fetched_at(&raw.fetched_at).map_err(ListError::FetchedAt)?;

    if raw.groups.len() != OWNERS.len() {
        return Err(ListError::GroupCount {
            found: raw.groups.len(),
        });
    }
    let mut groups = Vec::with_capacity(OWNERS.len());
    for (index, (raw_group, owner)) in raw.groups.into_iter().zip(OWNERS).enumerate() {
        if raw_group.owner != owner {
            return Err(ListError::GroupOwner {
                index,
                found: raw_group.owner,
            });
        }
        groups.push(validate_group(owner, raw_group.repos)?);
    }
    Ok(ProjectsList {
        fetched_at: raw.fetched_at,
        fetched_at_instant,
        groups,
    })
}

fn require_objects(root: &serde_json::Value) -> Result<(), ListError> {
    let Some(root) = root.as_object() else {
        return Err(ListError::NotAnObject { at: "root" });
    };
    for group in root
        .get("groups")
        .and_then(|g| g.as_array())
        .into_iter()
        .flatten()
    {
        let Some(group) = group.as_object() else {
            return Err(ListError::NotAnObject { at: "group" });
        };
        for repo in group
            .get("repos")
            .and_then(|r| r.as_array())
            .into_iter()
            .flatten()
        {
            if !repo.is_object() {
                return Err(ListError::NotAnObject { at: "repo" });
            }
        }
    }
    Ok(())
}

fn validate_group(owner: &'static str, raw_repos: Vec<RawRepo>) -> Result<Group, ListError> {
    if raw_repos.is_empty() {
        return Err(ListError::EmptyOwner { owner });
    }
    let mut repos: Vec<Repo> = Vec::with_capacity(raw_repos.len());
    for raw in raw_repos {
        if !is_valid_name(&raw.name) {
            return Err(ListError::RepoName {
                owner,
                name: raw.name,
            });
        }
        // Strictly increasing: a duplicate name is rejected too. GitHub cannot
        // produce one, so failing closed costs nothing.
        if let Some(prev) = repos.last()
            && !ascii_lowercase_lt(&prev.name, &raw.name)
        {
            return Err(ListError::RepoOrder {
                owner,
                name: raw.name,
            });
        }
        let code_points = raw.description.chars().count();
        if code_points > MAX_DESCRIPTION_CODE_POINTS {
            return Err(ListError::DescriptionTooLong {
                owner,
                name: raw.name,
                code_points,
            });
        }
        if let Some(c) = raw.description.chars().find(|c| is_rejected(*c)) {
            return Err(ListError::DescriptionRejectedChar {
                owner,
                name: raw.name,
                code_point: c as u32,
            });
        }
        repos.push(Repo {
            name: raw.name,
            description: raw.description,
        });
    }
    Ok(Group { owner, repos })
}

/// `^[A-Za-z0-9._-]{1,100}$`
fn is_valid_name(name: &str) -> bool {
    (1..=MAX_NAME_LEN).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// ASCII-lowercased, compared by code unit. Names are ASCII by this point.
fn ascii_lowercase_lt(a: &str, b: &str) -> bool {
    a.bytes()
        .map(|c| c.to_ascii_lowercase())
        .lt(b.bytes().map(|c| c.to_ascii_lowercase()))
}

/// The rejected class: `\p{Cc}` (C0, DEL, C1), U+2028, U+2029, and the
/// generated invisible table.
fn is_rejected(c: char) -> bool {
    c.is_control() || LINE_SEPARATORS.contains(&(c as u32)) || is_invisible(c)
}
