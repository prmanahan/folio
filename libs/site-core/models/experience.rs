use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ExperiencePublic {
    pub id: i64,
    pub company_name: String,
    pub title: String,
    pub location: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub is_current: bool,
    pub summary: String,
    pub bullet_points: serde_json::Value,
    pub display_order: i64,
}

impl ExperiencePublic {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let bp_str: String = row.get("bullet_points")?;
        let bullet_points: serde_json::Value =
            serde_json::from_str(&bp_str).unwrap_or(serde_json::json!([]));
        Ok(Self {
            id: row.get("id")?,
            company_name: row.get("company_name")?,
            title: row.get("title")?,
            location: row.get("location")?,
            start_date: row.get("start_date")?,
            end_date: row.get("end_date")?,
            is_current: row.get("is_current")?,
            summary: row.get("summary")?,
            bullet_points,
            display_order: row.get("display_order")?,
        })
    }
}

pub fn list_public(conn: &Connection) -> Result<Vec<ExperiencePublic>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, company_name, title, location, start_date, end_date,
                is_current, summary, bullet_points, display_order
         FROM experiences ORDER BY display_order ASC",
    )?;
    let rows = stmt.query_map([], ExperiencePublic::from_row)?;
    rows.collect()
}

#[derive(Debug, Serialize)]
pub struct ExperienceFull {
    pub id: i64,
    pub created_at: String,
    pub company_name: String,
    pub title: String,
    pub location: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub is_current: bool,
    pub summary: String,
    pub bullet_points: serde_json::Value,
    pub display_order: i64,
    pub title_progression: String,
    pub quantified_impact: serde_json::Value,
    pub why_joined: String,
    pub why_left: String,
    pub actual_contributions: String,
    pub proudest_achievement: String,
    pub would_do_differently: String,
    pub challenges_faced: String,
    pub lessons_learned: String,
    pub manager_would_say: String,
    pub reports_would_say: String,
}

impl ExperienceFull {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let bp_str: String = row.get("bullet_points")?;
        let bullet_points: serde_json::Value =
            serde_json::from_str(&bp_str).unwrap_or(serde_json::json!([]));
        let qi_str: String = row.get("quantified_impact")?;
        let quantified_impact: serde_json::Value =
            serde_json::from_str(&qi_str).unwrap_or(serde_json::json!({}));
        Ok(Self {
            id: row.get("id")?,
            created_at: row.get("created_at")?,
            company_name: row.get("company_name")?,
            title: row.get("title")?,
            location: row.get("location")?,
            start_date: row.get("start_date")?,
            end_date: row.get("end_date")?,
            is_current: row.get("is_current")?,
            summary: row.get("summary")?,
            bullet_points,
            display_order: row.get("display_order")?,
            title_progression: row.get("title_progression")?,
            quantified_impact,
            why_joined: row.get("why_joined")?,
            why_left: row.get("why_left")?,
            actual_contributions: row.get("actual_contributions")?,
            proudest_achievement: row.get("proudest_achievement")?,
            would_do_differently: row.get("would_do_differently")?,
            challenges_faced: row.get("challenges_faced")?,
            lessons_learned: row.get("lessons_learned")?,
            manager_would_say: row.get("manager_would_say")?,
            reports_would_say: row.get("reports_would_say")?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct ExperienceInput {
    pub company_name: String,
    pub title: String,
    pub location: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub is_current: bool,
    pub summary: String,
    pub bullet_points: serde_json::Value,
    pub display_order: i64,
    pub title_progression: String,
    pub quantified_impact: serde_json::Value,
    pub why_joined: String,
    pub why_left: String,
    pub actual_contributions: String,
    pub proudest_achievement: String,
    pub would_do_differently: String,
    pub challenges_faced: String,
    pub lessons_learned: String,
    pub manager_would_say: String,
    pub reports_would_say: String,
}

pub fn list_all(conn: &Connection) -> Result<Vec<ExperienceFull>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, company_name, title, location, start_date, end_date,
                is_current, summary, bullet_points, display_order, title_progression,
                quantified_impact, why_joined, why_left, actual_contributions,
                proudest_achievement, would_do_differently, challenges_faced,
                lessons_learned, manager_would_say, reports_would_say
         FROM experiences ORDER BY display_order ASC",
    )?;
    let rows = stmt.query_map([], ExperienceFull::from_row)?;
    rows.collect()
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<ExperienceFull, rusqlite::Error> {
    conn.query_row(
        "SELECT id, created_at, company_name, title, location, start_date, end_date,
                is_current, summary, bullet_points, display_order, title_progression,
                quantified_impact, why_joined, why_left, actual_contributions,
                proudest_achievement, would_do_differently, challenges_faced,
                lessons_learned, manager_would_say, reports_would_say
         FROM experiences WHERE id = ?1",
        rusqlite::params![id],
        ExperienceFull::from_row,
    )
}

pub fn create(
    conn: &Connection,
    input: &ExperienceInput,
) -> Result<ExperienceFull, rusqlite::Error> {
    let bp_str = serde_json::to_string(&input.bullet_points).unwrap_or_else(|_| "[]".to_string());
    let qi_str =
        serde_json::to_string(&input.quantified_impact).unwrap_or_else(|_| "{}".to_string());
    conn.execute(
        "INSERT INTO experiences (
            company_name, title, location, start_date, end_date, is_current,
            summary, bullet_points, display_order, title_progression,
            quantified_impact, why_joined, why_left, actual_contributions,
            proudest_achievement, would_do_differently, challenges_faced,
            lessons_learned, manager_would_say, reports_would_say
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        rusqlite::params![
            input.company_name,
            input.title,
            input.location,
            input.start_date,
            input.end_date,
            input.is_current as i64,
            input.summary,
            bp_str,
            input.display_order,
            input.title_progression,
            qi_str,
            input.why_joined,
            input.why_left,
            input.actual_contributions,
            input.proudest_achievement,
            input.would_do_differently,
            input.challenges_faced,
            input.lessons_learned,
            input.manager_would_say,
            input.reports_would_say,
        ],
    )?;
    let id = conn.last_insert_rowid();
    get_by_id(conn, id)
}

pub fn update(
    conn: &Connection,
    id: i64,
    input: &ExperienceInput,
) -> Result<ExperienceFull, rusqlite::Error> {
    let bp_str = serde_json::to_string(&input.bullet_points).unwrap_or_else(|_| "[]".to_string());
    let qi_str =
        serde_json::to_string(&input.quantified_impact).unwrap_or_else(|_| "{}".to_string());
    conn.execute(
        "UPDATE experiences SET
            company_name = ?1, title = ?2, location = ?3, start_date = ?4,
            end_date = ?5, is_current = ?6, summary = ?7, bullet_points = ?8,
            display_order = ?9, title_progression = ?10, quantified_impact = ?11,
            why_joined = ?12, why_left = ?13, actual_contributions = ?14,
            proudest_achievement = ?15, would_do_differently = ?16,
            challenges_faced = ?17, lessons_learned = ?18,
            manager_would_say = ?19, reports_would_say = ?20
         WHERE id = ?21",
        rusqlite::params![
            input.company_name,
            input.title,
            input.location,
            input.start_date,
            input.end_date,
            input.is_current as i64,
            input.summary,
            bp_str,
            input.display_order,
            input.title_progression,
            qi_str,
            input.why_joined,
            input.why_left,
            input.actual_contributions,
            input.proudest_achievement,
            input.would_do_differently,
            input.challenges_faced,
            input.lessons_learned,
            input.manager_would_say,
            input.reports_would_say,
            id,
        ],
    )?;
    get_by_id(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM experiences WHERE id = ?1",
        rusqlite::params![id],
    )?;
    if conn.changes() == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}
