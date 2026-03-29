use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct GapWeakness {
    pub id: i64,
    pub created_at: String,
    pub gap_type: String,
    pub description: String,
    pub why_its_a_gap: String,
    pub interest_in_learning: bool,
}

impl GapWeakness {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            created_at: row.get("created_at")?,
            gap_type: row.get("gap_type")?,
            description: row.get("description")?,
            why_its_a_gap: row.get("why_its_a_gap")?,
            interest_in_learning: row.get("interest_in_learning")?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct GapWeaknessInput {
    pub gap_type: String,
    pub description: String,
    pub why_its_a_gap: String,
    pub interest_in_learning: bool,
}

pub fn list_all(conn: &Connection) -> Result<Vec<GapWeakness>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, gap_type, description, why_its_a_gap, interest_in_learning
         FROM gaps_weaknesses ORDER BY id ASC",
    )?;
    let rows = stmt.query_map([], GapWeakness::from_row)?;
    rows.collect()
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<GapWeakness, rusqlite::Error> {
    conn.query_row(
        "SELECT id, created_at, gap_type, description, why_its_a_gap, interest_in_learning
         FROM gaps_weaknesses WHERE id = ?1",
        rusqlite::params![id],
        GapWeakness::from_row,
    )
}

pub fn create(conn: &Connection, input: &GapWeaknessInput) -> Result<GapWeakness, rusqlite::Error> {
    conn.execute(
        "INSERT INTO gaps_weaknesses (gap_type, description, why_its_a_gap, interest_in_learning)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            input.gap_type,
            input.description,
            input.why_its_a_gap,
            input.interest_in_learning as i64,
        ],
    )?;
    let id = conn.last_insert_rowid();
    get_by_id(conn, id)
}

pub fn update(conn: &Connection, id: i64, input: &GapWeaknessInput) -> Result<GapWeakness, rusqlite::Error> {
    conn.execute(
        "UPDATE gaps_weaknesses SET gap_type = ?1, description = ?2,
                why_its_a_gap = ?3, interest_in_learning = ?4
         WHERE id = ?5",
        rusqlite::params![
            input.gap_type,
            input.description,
            input.why_its_a_gap,
            input.interest_in_learning as i64,
            id,
        ],
    )?;
    get_by_id(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM gaps_weaknesses WHERE id = ?1", rusqlite::params![id])?;
    if conn.changes() == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}
