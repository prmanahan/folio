use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct FaqSuggestion {
    pub id: i64,
    pub question: String,
}

impl FaqSuggestion {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            question: row.get("question")?,
        })
    }
}

pub fn list_suggestions(conn: &Connection) -> Result<Vec<FaqSuggestion>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, question FROM faq_responses WHERE is_common_question = 1 ORDER BY id ASC",
    )?;
    let rows = stmt.query_map([], FaqSuggestion::from_row)?;
    rows.collect()
}

#[derive(Debug, Serialize)]
pub struct FaqFull {
    pub id: i64,
    pub created_at: String,
    pub question: String,
    pub answer: String,
    pub is_common_question: bool,
}

impl FaqFull {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            created_at: row.get("created_at")?,
            question: row.get("question")?,
            answer: row.get("answer")?,
            is_common_question: row.get("is_common_question")?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct FaqInput {
    pub question: String,
    pub answer: String,
    pub is_common_question: bool,
}

pub fn list_all(conn: &Connection) -> Result<Vec<FaqFull>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, question, answer, is_common_question
         FROM faq_responses ORDER BY id ASC",
    )?;
    let rows = stmt.query_map([], FaqFull::from_row)?;
    rows.collect()
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<FaqFull, rusqlite::Error> {
    conn.query_row(
        "SELECT id, created_at, question, answer, is_common_question
         FROM faq_responses WHERE id = ?1",
        rusqlite::params![id],
        FaqFull::from_row,
    )
}

pub fn create(conn: &Connection, input: &FaqInput) -> Result<FaqFull, rusqlite::Error> {
    conn.execute(
        "INSERT INTO faq_responses (question, answer, is_common_question) VALUES (?1, ?2, ?3)",
        rusqlite::params![
            input.question,
            input.answer,
            input.is_common_question as i64,
        ],
    )?;
    let id = conn.last_insert_rowid();
    get_by_id(conn, id)
}

pub fn update(conn: &Connection, id: i64, input: &FaqInput) -> Result<FaqFull, rusqlite::Error> {
    conn.execute(
        "UPDATE faq_responses SET question = ?1, answer = ?2, is_common_question = ?3
         WHERE id = ?4",
        rusqlite::params![
            input.question,
            input.answer,
            input.is_common_question as i64,
            id,
        ],
    )?;
    get_by_id(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM faq_responses WHERE id = ?1",
        rusqlite::params![id],
    )?;
    if conn.changes() == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}
