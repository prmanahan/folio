use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ValuesCulture {
    pub id: i64,
    pub created_at: String,
    pub must_haves: String,
    pub dealbreakers: String,
    pub management_style_preferences: String,
    pub team_size_preferences: String,
    pub how_handle_conflict: String,
    pub how_handle_ambiguity: String,
    pub how_handle_failure: String,
}

impl ValuesCulture {
    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            created_at: row.get("created_at")?,
            must_haves: row.get("must_haves")?,
            dealbreakers: row.get("dealbreakers")?,
            management_style_preferences: row.get("management_style_preferences")?,
            team_size_preferences: row.get("team_size_preferences")?,
            how_handle_conflict: row.get("how_handle_conflict")?,
            how_handle_ambiguity: row.get("how_handle_ambiguity")?,
            how_handle_failure: row.get("how_handle_failure")?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct ValuesCultureInput {
    pub must_haves: String,
    pub dealbreakers: String,
    pub management_style_preferences: String,
    pub team_size_preferences: String,
    pub how_handle_conflict: String,
    pub how_handle_ambiguity: String,
    pub how_handle_failure: String,
}

pub fn get(conn: &Connection) -> Result<ValuesCulture, rusqlite::Error> {
    conn.query_row(
        "SELECT id, created_at, must_haves, dealbreakers, management_style_preferences,
                team_size_preferences, how_handle_conflict, how_handle_ambiguity, how_handle_failure
         FROM values_culture WHERE id = 1",
        [],
        ValuesCulture::from_row,
    )
}

pub fn update(conn: &Connection, input: &ValuesCultureInput) -> Result<ValuesCulture, rusqlite::Error> {
    conn.execute(
        "UPDATE values_culture SET
            must_haves = ?1, dealbreakers = ?2, management_style_preferences = ?3,
            team_size_preferences = ?4, how_handle_conflict = ?5,
            how_handle_ambiguity = ?6, how_handle_failure = ?7
         WHERE id = 1",
        rusqlite::params![
            input.must_haves,
            input.dealbreakers,
            input.management_style_preferences,
            input.team_size_preferences,
            input.how_handle_conflict,
            input.how_handle_ambiguity,
            input.how_handle_failure,
        ],
    )?;
    get(conn)
}
