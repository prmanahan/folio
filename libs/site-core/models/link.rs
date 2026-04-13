use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Link {
    pub id: i64,
    pub label: String,
    pub url: String,
    pub icon: String,
    pub sort_order: i64,
}

impl Link {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            label: row.get("label")?,
            url: row.get("url")?,
            icon: row.get("icon")?,
            sort_order: row.get("sort_order")?,
        })
    }
}

pub fn list(conn: &Connection) -> Result<Vec<Link>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT id, label, url, icon, sort_order FROM links ORDER BY sort_order ASC")?;
    let rows = stmt.query_map([], Link::from_row)?;
    rows.collect()
}

#[derive(Debug, Deserialize)]
pub struct LinkInput {
    pub label: String,
    pub url: String,
    pub icon: String,
    pub sort_order: i64,
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<Link, rusqlite::Error> {
    conn.query_row(
        "SELECT id, label, url, icon, sort_order FROM links WHERE id = ?1",
        rusqlite::params![id],
        Link::from_row,
    )
}

pub fn create(conn: &Connection, input: &LinkInput) -> Result<Link, rusqlite::Error> {
    conn.execute(
        "INSERT INTO links (label, url, icon, sort_order) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![input.label, input.url, input.icon, input.sort_order],
    )?;
    let id = conn.last_insert_rowid();
    get_by_id(conn, id)
}

pub fn update(conn: &Connection, id: i64, input: &LinkInput) -> Result<Link, rusqlite::Error> {
    conn.execute(
        "UPDATE links SET label = ?1, url = ?2, icon = ?3, sort_order = ?4 WHERE id = ?5",
        rusqlite::params![input.label, input.url, input.icon, input.sort_order, id],
    )?;
    get_by_id(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM links WHERE id = ?1", rusqlite::params![id])?;
    if conn.changes() == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}
