//! The live lists: the projects list and the articles list, each fetched at
//! runtime, held in memory, and stored in the `live_lists` table.
//!
//! Spec `docs/specs/2026-11-18-live-lists-and-blog-move.md`, R-5 and R-15 to
//! R-23. This module is the seam every source plugs into: `start` runs the
//! startup sequence, `LiveLists` holds the copies the readers serve, and the
//! refresher fills them from injected sources.

pub mod articles;
pub mod clock;
pub mod held;
pub mod outbound;
pub mod panic_hook;
pub mod reason;
pub mod refresher;
pub mod start;
pub mod store;
pub mod switch;

#[cfg(test)]
mod harness;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tls_harness;

pub use held::LiveLists;
pub use reason::ReasonCode;

/// One of the two lists. The names are the `live_lists.name` values and the
/// tokens of `LIVE_LISTS_DISABLED`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListName {
    Projects,
    Articles,
}

impl ListName {
    pub const ALL: [ListName; 2] = [ListName::Projects, ListName::Articles];

    pub const fn as_str(self) -> &'static str {
        match self {
            ListName::Projects => "projects",
            ListName::Articles => "articles",
        }
    }
}
