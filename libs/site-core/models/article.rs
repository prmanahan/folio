use rusqlite::{Connection, Row};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ArticleFull {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub content: String,
    pub tags: serde_json::Value,
    pub published_at: Option<String>,
    pub published: bool,
}

impl ArticleFull {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let tags_str: String = row.get("tags")?;
        let tags: serde_json::Value =
            serde_json::from_str(&tags_str).unwrap_or(serde_json::json!([]));
        Ok(Self {
            id: row.get("id")?,
            title: row.get("title")?,
            slug: row.get("slug")?,
            summary: row.get("summary")?,
            content: row.get("content")?,
            tags,
            published_at: row.get("published_at")?,
            published: row.get("published")?,
        })
    }
}

pub fn list_all(conn: &Connection) -> Result<Vec<ArticleFull>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, title, slug, summary, content, tags, published_at, published
         FROM articles ORDER BY published_at DESC",
    )?;
    let rows = stmt.query_map([], ArticleFull::from_row)?;
    rows.collect()
}
