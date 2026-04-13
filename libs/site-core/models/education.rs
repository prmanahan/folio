use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Education {
    pub id: i64,
    pub degree: String,
    pub institution: String,
    pub location: String,
    pub start_year: String,
    pub end_year: String,
}

impl Education {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            degree: row.get("degree")?,
            institution: row.get("institution")?,
            location: row.get("location")?,
            start_year: row.get("start_year")?,
            end_year: row.get("end_year")?,
        })
    }
}

pub fn list(conn: &Connection) -> Result<Vec<Education>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, degree, institution, location, start_year, end_year FROM education ORDER BY start_year ASC"
    )?;
    let rows = stmt.query_map([], Education::from_row)?;
    rows.collect()
}

#[derive(Debug, Deserialize)]
pub struct EducationInput {
    pub degree: String,
    pub institution: String,
    pub location: String,
    pub start_year: String,
    pub end_year: String,
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<Education, rusqlite::Error> {
    conn.query_row(
        "SELECT id, degree, institution, location, start_year, end_year
         FROM education WHERE id = ?1",
        rusqlite::params![id],
        Education::from_row,
    )
}

pub fn create(conn: &Connection, input: &EducationInput) -> Result<Education, rusqlite::Error> {
    conn.execute(
        "INSERT INTO education (degree, institution, location, start_year, end_year)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            input.degree,
            input.institution,
            input.location,
            input.start_year,
            input.end_year,
        ],
    )?;
    let id = conn.last_insert_rowid();
    get_by_id(conn, id)
}

pub fn update(
    conn: &Connection,
    id: i64,
    input: &EducationInput,
) -> Result<Education, rusqlite::Error> {
    conn.execute(
        "UPDATE education SET degree = ?1, institution = ?2, location = ?3,
                start_year = ?4, end_year = ?5
         WHERE id = ?6",
        rusqlite::params![
            input.degree,
            input.institution,
            input.location,
            input.start_year,
            input.end_year,
            id,
        ],
    )?;
    get_by_id(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM education WHERE id = ?1", rusqlite::params![id])?;
    if conn.changes() == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}
