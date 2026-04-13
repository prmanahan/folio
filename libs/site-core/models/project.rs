use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Project {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub description: String,
    pub tech_stack: serde_json::Value,
    pub url: String,
    pub sort_order: i64,
}

impl Project {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let ts_str: String = row.get("tech_stack")?;
        let tech_stack: serde_json::Value =
            serde_json::from_str(&ts_str).unwrap_or(serde_json::json!([]));
        Ok(Self {
            id: row.get("id")?,
            title: row.get("title")?,
            slug: row.get("slug")?,
            summary: row.get("summary")?,
            description: row.get("description")?,
            tech_stack,
            url: row.get("url")?,
            sort_order: row.get("sort_order")?,
        })
    }
}

pub fn list_published(conn: &Connection) -> Result<Vec<Project>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, title, slug, summary, description, tech_stack, url, sort_order
         FROM projects WHERE published = 1 ORDER BY sort_order ASC",
    )?;
    let rows = stmt.query_map([], Project::from_row)?;
    rows.collect()
}

pub fn get_by_slug(conn: &Connection, slug: &str) -> Result<Option<Project>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, title, slug, summary, description, tech_stack, url, sort_order
         FROM projects WHERE slug = ?1 AND published = 1",
    )?;
    let mut rows = stmt.query_map([slug], Project::from_row)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

#[derive(Debug, Serialize)]
pub struct ProjectFull {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub description: String,
    pub tech_stack: serde_json::Value,
    pub url: String,
    pub sort_order: i64,
    pub published: bool,
}

impl ProjectFull {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let ts_str: String = row.get("tech_stack")?;
        let tech_stack: serde_json::Value =
            serde_json::from_str(&ts_str).unwrap_or(serde_json::json!([]));
        Ok(Self {
            id: row.get("id")?,
            title: row.get("title")?,
            slug: row.get("slug")?,
            summary: row.get("summary")?,
            description: row.get("description")?,
            tech_stack,
            url: row.get("url")?,
            sort_order: row.get("sort_order")?,
            published: row.get("published")?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct ProjectInput {
    pub title: String,
    pub slug: Option<String>,
    pub summary: String,
    pub description: String,
    pub tech_stack: serde_json::Value,
    pub url: String,
    pub sort_order: i64,
    pub published: bool,
}

fn slugify(title: &str) -> String {
    title.to_lowercase().replace(' ', "-")
}

pub fn list_all(conn: &Connection) -> Result<Vec<ProjectFull>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, title, slug, summary, description, tech_stack, url, sort_order, published
         FROM projects ORDER BY sort_order ASC",
    )?;
    let rows = stmt.query_map([], ProjectFull::from_row)?;
    rows.collect()
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<ProjectFull, rusqlite::Error> {
    conn.query_row(
        "SELECT id, title, slug, summary, description, tech_stack, url, sort_order, published
         FROM projects WHERE id = ?1",
        rusqlite::params![id],
        ProjectFull::from_row,
    )
}

pub fn create(conn: &Connection, input: &ProjectInput) -> Result<ProjectFull, rusqlite::Error> {
    let slug = input.slug.clone().unwrap_or_else(|| slugify(&input.title));
    let ts_str = serde_json::to_string(&input.tech_stack).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT INTO projects (title, slug, summary, description, tech_stack, url, sort_order, published)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            input.title,
            slug,
            input.summary,
            input.description,
            ts_str,
            input.url,
            input.sort_order,
            input.published as i64,
        ],
    )?;
    let id = conn.last_insert_rowid();
    get_by_id(conn, id)
}

pub fn update(
    conn: &Connection,
    id: i64,
    input: &ProjectInput,
) -> Result<ProjectFull, rusqlite::Error> {
    let slug = input.slug.clone().unwrap_or_else(|| slugify(&input.title));
    let ts_str = serde_json::to_string(&input.tech_stack).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "UPDATE projects SET title = ?1, slug = ?2, summary = ?3, description = ?4,
                tech_stack = ?5, url = ?6, sort_order = ?7, published = ?8
         WHERE id = ?9",
        rusqlite::params![
            input.title,
            slug,
            input.summary,
            input.description,
            ts_str,
            input.url,
            input.sort_order,
            input.published as i64,
            id,
        ],
    )?;
    get_by_id(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM projects WHERE id = ?1", rusqlite::params![id])?;
    if conn.changes() == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}
