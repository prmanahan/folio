use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AgentPublic {
    pub id: i64,
    pub name: String,
    pub role: String,
    pub short_role: String,
    pub model: String,
    pub personality_blurb: String,
    pub responsibilities: Vec<String>,
    pub avatar_filename: String,
    pub display_order: i64,
    pub is_featured: bool,
    pub is_review_gate: bool,
}

impl AgentPublic {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let resp_str: String = row.get("responsibilities")?;
        let responsibilities: Vec<String> = serde_json::from_str(&resp_str).unwrap_or_default();
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            role: row.get("role")?,
            short_role: row.get("short_role")?,
            model: row.get("model")?,
            personality_blurb: row.get("personality_blurb")?,
            responsibilities,
            avatar_filename: row.get("avatar_filename")?,
            display_order: row.get("display_order")?,
            is_featured: row.get("is_featured")?,
            is_review_gate: row.get("is_review_gate")?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct AgentFull {
    pub id: i64,
    pub created_at: i64,
    pub name: String,
    pub role: String,
    pub short_role: String,
    pub model: String,
    pub personality_blurb: String,
    pub responsibilities: Vec<String>,
    pub avatar_filename: String,
    pub display_order: i64,
    pub is_featured: bool,
    pub is_review_gate: bool,
    pub published: bool,
}

impl AgentFull {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let resp_str: String = row.get("responsibilities")?;
        let responsibilities: Vec<String> = serde_json::from_str(&resp_str).unwrap_or_default();
        Ok(Self {
            id: row.get("id")?,
            created_at: row.get("created_at")?,
            name: row.get("name")?,
            role: row.get("role")?,
            short_role: row.get("short_role")?,
            model: row.get("model")?,
            personality_blurb: row.get("personality_blurb")?,
            responsibilities,
            avatar_filename: row.get("avatar_filename")?,
            display_order: row.get("display_order")?,
            is_featured: row.get("is_featured")?,
            is_review_gate: row.get("is_review_gate")?,
            published: row.get("published")?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentInput {
    pub name: String,
    pub role: String,
    pub short_role: String,
    pub model: String,
    pub personality_blurb: String,
    pub responsibilities: Vec<String>,
    pub avatar_filename: String,
    pub display_order: i64,
    pub is_featured: bool,
    pub is_review_gate: bool,
    pub published: bool,
}

pub fn list_published(conn: &Connection) -> Result<Vec<AgentPublic>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, name, role, short_role, model, personality_blurb,
                responsibilities, avatar_filename, display_order, is_featured, is_review_gate
         FROM agents WHERE published = 1 ORDER BY display_order ASC",
    )?;
    let rows = stmt.query_map([], AgentPublic::from_row)?;
    rows.collect()
}

pub fn list_all(conn: &Connection) -> Result<Vec<AgentFull>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, name, role, short_role, model, personality_blurb,
                responsibilities, avatar_filename, display_order, is_featured,
                is_review_gate, published
         FROM agents ORDER BY display_order ASC",
    )?;
    let rows = stmt.query_map([], AgentFull::from_row)?;
    rows.collect()
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<AgentFull, rusqlite::Error> {
    conn.query_row(
        "SELECT id, created_at, name, role, short_role, model, personality_blurb,
                responsibilities, avatar_filename, display_order, is_featured,
                is_review_gate, published
         FROM agents WHERE id = ?1",
        rusqlite::params![id],
        AgentFull::from_row,
    )
}

pub fn create(conn: &Connection, input: &AgentInput) -> Result<AgentFull, rusqlite::Error> {
    let resp_str =
        serde_json::to_string(&input.responsibilities).unwrap_or_else(|_| "[]".to_string());
    if input.is_featured {
        conn.execute(
            "UPDATE agents SET is_featured = 0 WHERE is_featured = 1",
            [],
        )?;
    }
    if input.is_review_gate {
        conn.execute(
            "UPDATE agents SET is_review_gate = 0 WHERE is_review_gate = 1",
            [],
        )?;
    }
    conn.execute(
        "INSERT INTO agents (name, role, short_role, model, personality_blurb,
                             responsibilities, avatar_filename, display_order,
                             is_featured, is_review_gate, published)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![
            input.name,
            input.role,
            input.short_role,
            input.model,
            input.personality_blurb,
            resp_str,
            input.avatar_filename,
            input.display_order,
            input.is_featured,
            input.is_review_gate,
            input.published,
        ],
    )?;
    let id = conn.last_insert_rowid();
    get_by_id(conn, id)
}

pub fn update(
    conn: &Connection,
    id: i64,
    input: &AgentInput,
) -> Result<AgentFull, rusqlite::Error> {
    let resp_str =
        serde_json::to_string(&input.responsibilities).unwrap_or_else(|_| "[]".to_string());
    if input.is_featured {
        conn.execute(
            "UPDATE agents SET is_featured = 0 WHERE is_featured = 1 AND id != ?1",
            rusqlite::params![id],
        )?;
    }
    if input.is_review_gate {
        conn.execute(
            "UPDATE agents SET is_review_gate = 0 WHERE is_review_gate = 1 AND id != ?1",
            rusqlite::params![id],
        )?;
    }
    conn.execute(
        "UPDATE agents SET name = ?1, role = ?2, short_role = ?3, model = ?4,
                personality_blurb = ?5, responsibilities = ?6, avatar_filename = ?7,
                display_order = ?8, is_featured = ?9, is_review_gate = ?10,
                published = ?11
         WHERE id = ?12",
        rusqlite::params![
            input.name,
            input.role,
            input.short_role,
            input.model,
            input.personality_blurb,
            resp_str,
            input.avatar_filename,
            input.display_order,
            input.is_featured,
            input.is_review_gate,
            input.published,
            id,
        ],
    )?;
    if conn.changes() == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    get_by_id(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM agents WHERE id = ?1", rusqlite::params![id])?;
    if conn.changes() == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_input(name: &str, order: i64) -> AgentInput {
        AgentInput {
            name: name.to_string(),
            role: "Test Role".to_string(),
            short_role: "Test".to_string(),
            model: "sonnet".to_string(),
            personality_blurb: "A test agent.".to_string(),
            responsibilities: vec!["Task 1".to_string(), "Task 2".to_string()],
            avatar_filename: "test.png".to_string(),
            display_order: order,
            is_featured: false,
            is_review_gate: false,
            published: true,
        }
    }

    #[test]
    fn test_crud_cycle() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let a = create(&conn, &make_input("Puck", 1)).unwrap();
        let b = create(&conn, &make_input("Forge", 2)).unwrap();

        assert_eq!(a.name, "Puck");
        assert_eq!(b.display_order, 2);

        let all = list_all(&conn).unwrap();
        assert!(all.iter().any(|ag| ag.id == a.id));

        let public = list_published(&conn).unwrap();
        assert!(public.iter().any(|ag| ag.id == a.id));

        let fetched = get_by_id(&conn, a.id).unwrap();
        assert_eq!(fetched.name, "Puck");

        let mut updated_input = make_input("Puck Updated", 1);
        updated_input.is_featured = true;
        let updated = update(&conn, a.id, &updated_input).unwrap();
        assert_eq!(updated.name, "Puck Updated");
        assert!(updated.is_featured);

        delete(&conn, b.id).unwrap();
        let result = get_by_id(&conn, b.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_nonexistent_returns_error() {
        let conn = db::connect(":memory:").expect("in-memory db");
        let result = update(&conn, 9999, &make_input("Ghost", 1));
        assert!(result.is_err());
    }

    #[test]
    fn test_single_featured_agent_enforced() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let mut input_a = make_input("Alpha", 1);
        input_a.is_featured = true;
        let a = create(&conn, &input_a).unwrap();
        assert!(a.is_featured);

        // Creating a second featured agent should clear the first
        let mut input_b = make_input("Beta", 2);
        input_b.is_featured = true;
        let b = create(&conn, &input_b).unwrap();
        assert!(b.is_featured);

        let a_refetched = get_by_id(&conn, a.id).unwrap();
        assert!(
            !a_refetched.is_featured,
            "Alpha should no longer be featured"
        );
    }

    #[test]
    fn test_update_single_featured_agent_enforced() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let mut input_a = make_input("Alpha", 1);
        input_a.is_featured = true;
        let a = create(&conn, &input_a).unwrap();

        let b = create(&conn, &make_input("Beta", 2)).unwrap();

        // Updating Beta to be featured should clear Alpha
        let mut input_b_update = make_input("Beta", 2);
        input_b_update.is_featured = true;
        update(&conn, b.id, &input_b_update).unwrap();

        let a_refetched = get_by_id(&conn, a.id).unwrap();
        assert!(
            !a_refetched.is_featured,
            "Alpha should no longer be featured after Beta updated"
        );
    }
}
