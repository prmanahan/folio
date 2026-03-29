use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AiInstruction {
    pub id: i64,
    pub created_at: String,
    pub instruction_type: String,
    pub instruction: String,
    pub priority: i64,
}

impl AiInstruction {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            created_at: row.get("created_at")?,
            instruction_type: row.get("instruction_type")?,
            instruction: row.get("instruction")?,
            priority: row.get("priority")?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct AiInstructionInput {
    pub instruction_type: String,
    pub instruction: String,
    pub priority: i64,
}

pub fn list_all(conn: &Connection) -> Result<Vec<AiInstruction>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, instruction_type, instruction, priority
         FROM ai_instructions ORDER BY priority DESC, id ASC",
    )?;
    let rows = stmt.query_map([], AiInstruction::from_row)?;
    rows.collect()
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<AiInstruction, rusqlite::Error> {
    conn.query_row(
        "SELECT id, created_at, instruction_type, instruction, priority
         FROM ai_instructions WHERE id = ?1",
        rusqlite::params![id],
        AiInstruction::from_row,
    )
}

pub fn create(conn: &Connection, input: &AiInstructionInput) -> Result<AiInstruction, rusqlite::Error> {
    conn.execute(
        "INSERT INTO ai_instructions (instruction_type, instruction, priority)
         VALUES (?1, ?2, ?3)",
        rusqlite::params![input.instruction_type, input.instruction, input.priority],
    )?;
    let id = conn.last_insert_rowid();
    get_by_id(conn, id)
}

pub fn update(conn: &Connection, id: i64, input: &AiInstructionInput) -> Result<AiInstruction, rusqlite::Error> {
    conn.execute(
        "UPDATE ai_instructions SET instruction_type = ?1, instruction = ?2, priority = ?3
         WHERE id = ?4",
        rusqlite::params![input.instruction_type, input.instruction, input.priority, id],
    )?;
    get_by_id(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM ai_instructions WHERE id = ?1", rusqlite::params![id])?;
    if conn.changes() == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}
