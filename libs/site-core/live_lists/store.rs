//! The `live_lists` table: delete (R-19), load and re-validate (R-16) and
//! persist (R-15).
//!
//! Load and delete run once at start on the request-path connection, before
//! the first request. Persist runs on its own connection, owned by the
//! refresher: a panic in the refresher can then never poison the mutex the
//! request path and the health check share (R-17).

use rusqlite::Connection;
use time::OffsetDateTime;

use crate::projects_list::MAX_AGE_AHEAD;

use super::ListName;
use super::held::{Held, ListDocument};

/// What deleting a disabled list's row did (R-19).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteOutcome {
    Deleted,
    Absent,
    Failed,
}

impl DeleteOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            DeleteOutcome::Deleted => "deleted",
            DeleteOutcome::Absent => "absent",
            DeleteOutcome::Failed => "failed",
        }
    }
}

/// Delete `list`'s row. The cause of a failure is dropped: the caller logs
/// the `delete_failed` reason code.
pub fn delete_row(conn: &Connection, list: ListName) -> DeleteOutcome {
    match conn.execute("DELETE FROM live_lists WHERE name = ?1", [list.as_str()]) {
        Ok(0) => DeleteOutcome::Absent,
        Ok(_) => DeleteOutcome::Deleted,
        Err(_) => DeleteOutcome::Failed,
    }
}

/// Why a list loads as no data yet, with its time missing (R-16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoData {
    /// No row.
    Absent,
    /// The row could not be read.
    Unreadable,
    /// The row failed its list's validation.
    Invalid,
    /// `fetched_at` is more than the skew allowance ahead of the clock.
    AheadOfClock,
}

impl NoData {
    pub const fn as_str(self) -> &'static str {
        match self {
            NoData::Absent => "absent",
            NoData::Unreadable => "unreadable",
            NoData::Invalid => "invalid",
            NoData::AheadOfClock => "ahead_of_clock",
        }
    }
}

/// Read `L`'s stored row and re-validate it through the validator a fetch
/// ends in. Only the ahead-of-now bound is applied, never the behind bound:
/// a restart after a long stop must serve the copy held before it.
pub fn load<L: ListDocument>(conn: &Connection, now: OffsetDateTime) -> Result<Held<L>, NoData> {
    let row: Result<String, rusqlite::Error> = conn.query_row(
        "SELECT document FROM live_lists WHERE name = ?1",
        [L::NAME.as_str()],
        |r| r.get(0),
    );
    let document = match row {
        Ok(document) => document,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Err(NoData::Absent),
        Err(_) => return Err(NoData::Unreadable),
    };
    let held = Held::<L>::new(document).map_err(|_| NoData::Invalid)?;
    if held.list().fetched_at_instant() - now > MAX_AGE_AHEAD {
        return Err(NoData::AheadOfClock);
    }
    Ok(held)
}

/// R-15's write: one transaction, one row per list, overwritten.
pub trait ListStore: Send {
    /// # Errors
    /// [`PersistError`] when the write did not commit; nothing was stored.
    fn persist(&mut self, list: ListName, document: &str) -> Result<(), PersistError>;
}

/// A persist that did not commit. The cause is kept for the owner of the
/// store and never logged: R-30 logs the `persist_failed` reason code.
#[derive(Debug)]
pub struct PersistError(pub rusqlite::Error);

/// The production store: its own connection to the database file.
pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    /// Open a second connection to `database_url`, with the same pragmas as
    /// the request path's.
    ///
    /// # Errors
    /// The error from opening or configuring the connection.
    pub fn open(database_url: &str) -> Result<Self, rusqlite::Error> {
        let conn = crate::db::connect(database_url)?;
        Ok(Self { conn })
    }
}

impl ListStore for SqliteStore {
    fn persist(&mut self, list: ListName, document: &str) -> Result<(), PersistError> {
        let write = |conn: &mut Connection| -> Result<(), rusqlite::Error> {
            let tx = conn.transaction()?;
            tx.execute(
                "INSERT INTO live_lists (name, document) VALUES (?1, ?2) \
                 ON CONFLICT(name) DO UPDATE SET document = excluded.document",
                rusqlite::params![list.as_str(), document],
            )?;
            tx.commit()
        };
        write(&mut self.conn).map_err(PersistError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live_lists::articles::ArticlesList;
    use crate::live_lists::test_support::{self as ts, TempDir};
    use crate::projects_list::ProjectsList;
    use time::Duration;

    fn migrated() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrate(&conn).unwrap();
        conn
    }

    fn seed(conn: &Connection, list: ListName, document: &str) {
        conn.execute(
            "INSERT INTO live_lists (name, document) VALUES (?1, ?2) \
             ON CONFLICT(name) DO UPDATE SET document = excluded.document",
            rusqlite::params![list.as_str(), document],
        )
        .unwrap();
    }

    fn stored(conn: &Connection, list: ListName) -> Option<String> {
        conn.query_row(
            "SELECT document FROM live_lists WHERE name = ?1",
            [list.as_str()],
            |r| r.get(0),
        )
        .ok()
    }

    // ---- delete_row (R-19) ----

    #[test]
    fn r19_delete_removes_the_row_and_reports_deleted() {
        let conn = migrated();
        seed(&conn, ListName::Articles, &ts::articles_doc(ts::T0));
        seed(&conn, ListName::Projects, &ts::projects_doc(ts::T0));
        assert_eq!(
            delete_row(&conn, ListName::Articles),
            DeleteOutcome::Deleted
        );
        assert_eq!(stored(&conn, ListName::Articles), None);
        // the other list's row is untouched
        assert!(stored(&conn, ListName::Projects).is_some());
    }

    #[test]
    fn r19_delete_of_a_missing_row_reports_absent() {
        let conn = migrated();
        assert_eq!(delete_row(&conn, ListName::Projects), DeleteOutcome::Absent);
    }

    #[test]
    fn r19_delete_that_the_database_refuses_reports_failed_and_keeps_the_row() {
        let conn = migrated();
        seed(&conn, ListName::Projects, &ts::projects_doc(ts::T0));
        conn.execute_batch(
            "CREATE TRIGGER refuse_delete BEFORE DELETE ON live_lists \
             BEGIN SELECT RAISE(ABORT, 'forced'); END;",
        )
        .unwrap();
        assert_eq!(delete_row(&conn, ListName::Projects), DeleteOutcome::Failed);
        assert!(stored(&conn, ListName::Projects).is_some());
    }

    #[test]
    fn r19_delete_against_a_database_with_no_table_reports_failed() {
        let conn = Connection::open_in_memory().unwrap();
        assert_eq!(delete_row(&conn, ListName::Projects), DeleteOutcome::Failed);
    }

    // ---- load (R-16) ----

    fn now() -> OffsetDateTime {
        ts::instant(ts::T0)
    }

    #[test]
    fn r16_a_valid_row_loads_as_the_text_that_was_stored() {
        let conn = migrated();
        let doc = ts::projects_doc(ts::T0);
        seed(&conn, ListName::Projects, &doc);
        let held = load::<ProjectsList>(&conn, now()).expect("loads");
        assert_eq!(held.json(), doc);
        assert_eq!(held.fetched_at(), ts::T0);
    }

    #[test]
    fn r16_each_list_loads_through_its_own_validator() {
        let conn = migrated();
        seed(&conn, ListName::Articles, &ts::articles_doc(ts::T0));
        let held = load::<ArticlesList>(&conn, now()).expect("loads");
        assert_eq!(held.list().posts().len(), 1);
        // an articles document under the projects name is invalid there
        seed(&conn, ListName::Projects, &ts::articles_doc(ts::T0));
        assert_eq!(
            load::<ProjectsList>(&conn, now()).unwrap_err(),
            NoData::Invalid
        );
    }

    #[test]
    fn r16_a_missing_row_is_no_data_absent() {
        let conn = migrated();
        assert_eq!(
            load::<ProjectsList>(&conn, now()).unwrap_err(),
            NoData::Absent
        );
    }

    #[test]
    fn r16_a_row_edited_to_break_validation_is_no_data_invalid() {
        let conn = migrated();
        for bad in [
            "not json".to_string(),
            "{}".to_string(),
            ts::projects_doc(ts::T0).replace("alpha", "al pha"),
            ts::projects_doc(ts::T0).replace("\"fetched_at\"", "\"fetched\""),
        ] {
            seed(&conn, ListName::Projects, &bad);
            assert_eq!(
                load::<ProjectsList>(&conn, now()).unwrap_err(),
                NoData::Invalid,
                "{bad}"
            );
        }
    }

    #[test]
    fn r16_a_row_dated_exactly_the_skew_allowance_ahead_loads() {
        let conn = migrated();
        let at_bound = ts::format(now() + MAX_AGE_AHEAD);
        seed(&conn, ListName::Projects, &ts::projects_doc(&at_bound));
        assert!(load::<ProjectsList>(&conn, now()).is_ok());
        seed(&conn, ListName::Articles, &ts::articles_doc(&at_bound));
        assert!(load::<ArticlesList>(&conn, now()).is_ok());
    }

    #[test]
    fn r16_a_row_dated_one_millisecond_beyond_the_skew_allowance_is_no_data() {
        let conn = migrated();
        let beyond = ts::format(now() + MAX_AGE_AHEAD + Duration::milliseconds(1));
        seed(&conn, ListName::Projects, &ts::projects_doc(&beyond));
        assert_eq!(
            load::<ProjectsList>(&conn, now()).unwrap_err(),
            NoData::AheadOfClock
        );
        seed(&conn, ListName::Articles, &ts::articles_doc(&beyond));
        assert_eq!(
            load::<ArticlesList>(&conn, now()).unwrap_err(),
            NoData::AheadOfClock
        );
    }

    #[test]
    fn r16_the_skew_allowance_is_the_one_projects_list_value() {
        // R-34: one value, so the store cannot drift from the validator's
        assert_eq!(MAX_AGE_AHEAD, Duration::minutes(5));
    }

    #[test]
    fn r16_a_row_older_than_the_projects_behind_bound_still_loads() {
        // A restart after a long stop serves the copy held before it
        let conn = migrated();
        let old = ts::format(now() - Duration::days(3));
        seed(&conn, ListName::Projects, &ts::projects_doc(&old));
        assert!(load::<ProjectsList>(&conn, now()).is_ok());
    }

    #[test]
    fn r16_an_unreadable_row_is_no_data_unreadable() {
        let conn = Connection::open_in_memory().unwrap(); // no live_lists table
        assert_eq!(
            load::<ProjectsList>(&conn, now()).unwrap_err(),
            NoData::Unreadable
        );
    }

    // ---- persist (R-15) ----

    fn open_store() -> (TempDir, SqliteStore, Connection) {
        let dir = TempDir::new();
        let path = dir.path().join("site.db");
        let url = path.to_str().unwrap();
        let store = SqliteStore::open(url).unwrap();
        let reader = Connection::open(url).unwrap();
        (dir, store, reader)
    }

    #[test]
    fn r15_persist_stores_exactly_the_document() {
        let (_dir, mut store, reader) = open_store();
        let doc = ts::projects_doc(ts::T0);
        store.persist(ListName::Projects, &doc).unwrap();
        assert_eq!(stored(&reader, ListName::Projects), Some(doc));
        assert_eq!(stored(&reader, ListName::Articles), None);
    }

    #[test]
    fn r15_persist_overwrites_so_each_list_has_one_row() {
        let (_dir, mut store, reader) = open_store();
        let first = ts::projects_doc(ts::T0);
        let second = ts::projects_doc("2026-11-19T18:00:00.000Z");
        store.persist(ListName::Projects, &first).unwrap();
        store.persist(ListName::Projects, &second).unwrap();
        assert_eq!(stored(&reader, ListName::Projects), Some(second));
        let rows: i64 = reader
            .query_row("SELECT COUNT(*) FROM live_lists", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn r15_a_persist_the_database_refuses_errs_and_keeps_the_stored_row() {
        let (_dir, mut store, reader) = open_store();
        let kept = ts::projects_doc(ts::T0);
        store.persist(ListName::Projects, &kept).unwrap();
        reader
            .execute_batch(
                "CREATE TRIGGER refuse_write BEFORE UPDATE ON live_lists \
                 BEGIN SELECT RAISE(ABORT, 'forced'); END;",
            )
            .unwrap();
        let result = store.persist(
            ListName::Projects,
            &ts::projects_doc("2026-11-19T18:00:00.000Z"),
        );
        assert!(result.is_err());
        assert_eq!(stored(&reader, ListName::Projects), Some(kept));
    }
}
