use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct SkillPublic {
    pub id: i64,
    pub skill_name: String,
    pub category: String,
    pub years_experience: i64,
    pub last_used: String,
}

impl SkillPublic {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            skill_name: row.get("skill_name")?,
            category: row.get("category")?,
            years_experience: row.get("years_experience")?,
            last_used: row.get("last_used")?,
        })
    }
}

pub fn list_public(conn: &Connection) -> Result<Vec<SkillPublic>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, skill_name, category, years_experience, last_used
         FROM skills ORDER BY category ASC, years_experience DESC",
    )?;
    let rows = stmt.query_map([], SkillPublic::from_row)?;
    rows.collect()
}

#[derive(Debug, Serialize)]
pub struct SkillFull {
    pub id: i64,
    pub created_at: String,
    pub skill_name: String,
    pub category: String,
    pub years_experience: i64,
    pub last_used: String,
    pub self_rating: i64,
    pub evidence: String,
    pub honest_notes: String,
}

impl SkillFull {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            created_at: row.get("created_at")?,
            skill_name: row.get("skill_name")?,
            category: row.get("category")?,
            years_experience: row.get("years_experience")?,
            last_used: row.get("last_used")?,
            self_rating: row.get("self_rating")?,
            evidence: row.get("evidence")?,
            honest_notes: row.get("honest_notes")?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct SkillInput {
    pub skill_name: String,
    pub category: String,
    pub years_experience: i64,
    pub last_used: String,
    pub self_rating: i64,
    pub evidence: String,
    pub honest_notes: String,
}

pub fn list_all(conn: &Connection) -> Result<Vec<SkillFull>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, skill_name, category, years_experience, last_used,
                self_rating, evidence, honest_notes
         FROM skills ORDER BY category ASC, years_experience DESC",
    )?;
    let rows = stmt.query_map([], SkillFull::from_row)?;
    rows.collect()
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<SkillFull, rusqlite::Error> {
    conn.query_row(
        "SELECT id, created_at, skill_name, category, years_experience, last_used,
                self_rating, evidence, honest_notes
         FROM skills WHERE id = ?1",
        rusqlite::params![id],
        SkillFull::from_row,
    )
}

pub fn create(conn: &Connection, input: &SkillInput) -> Result<SkillFull, rusqlite::Error> {
    conn.execute(
        "INSERT INTO skills (skill_name, category, years_experience, last_used,
                             self_rating, evidence, honest_notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            input.skill_name,
            input.category,
            input.years_experience,
            input.last_used,
            input.self_rating,
            input.evidence,
            input.honest_notes,
        ],
    )?;
    let id = conn.last_insert_rowid();
    get_by_id(conn, id)
}

pub fn update(
    conn: &Connection,
    id: i64,
    input: &SkillInput,
) -> Result<SkillFull, rusqlite::Error> {
    conn.execute(
        "UPDATE skills SET skill_name = ?1, category = ?2, years_experience = ?3,
                last_used = ?4, self_rating = ?5, evidence = ?6, honest_notes = ?7
         WHERE id = ?8",
        rusqlite::params![
            input.skill_name,
            input.category,
            input.years_experience,
            input.last_used,
            input.self_rating,
            input.evidence,
            input.honest_notes,
            id,
        ],
    )?;
    get_by_id(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM skills WHERE id = ?1", rusqlite::params![id])?;
    if conn.changes() == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}
