use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Article {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub content: String,
    pub tags: serde_json::Value,
    pub published_at: Option<String>,
}

impl Article {
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
        })
    }
}

pub fn list_published(conn: &Connection) -> Result<Vec<Article>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, title, slug, summary, content, tags, published_at
         FROM articles WHERE published = 1 ORDER BY published_at DESC"
    )?;
    let rows = stmt.query_map([], Article::from_row)?;
    rows.collect()
}

pub fn get_by_slug(conn: &Connection, slug: &str) -> Result<Option<Article>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, title, slug, summary, content, tags, published_at
         FROM articles WHERE slug = ?1 AND published = 1"
    )?;
    let mut rows = stmt.query_map([slug], Article::from_row)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

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

#[derive(Debug, Deserialize)]
pub struct ArticleInput {
    pub title: String,
    pub slug: Option<String>,
    pub summary: String,
    pub content: String,
    pub tags: serde_json::Value,
    pub published_at: Option<String>,
    pub published: bool,
}

fn slugify(title: &str) -> String {
    title.to_lowercase().replace(' ', "-")
}

pub fn list_all(conn: &Connection) -> Result<Vec<ArticleFull>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, title, slug, summary, content, tags, published_at, published
         FROM articles ORDER BY published_at DESC",
    )?;
    let rows = stmt.query_map([], ArticleFull::from_row)?;
    rows.collect()
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<ArticleFull, rusqlite::Error> {
    conn.query_row(
        "SELECT id, title, slug, summary, content, tags, published_at, published
         FROM articles WHERE id = ?1",
        rusqlite::params![id],
        ArticleFull::from_row,
    )
}

pub fn create(conn: &Connection, input: &ArticleInput) -> Result<ArticleFull, rusqlite::Error> {
    let slug = input.slug.clone().unwrap_or_else(|| slugify(&input.title));
    let tags_str = serde_json::to_string(&input.tags)
        .unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT INTO articles (title, slug, summary, content, tags, published_at, published)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            input.title,
            slug,
            input.summary,
            input.content,
            tags_str,
            input.published_at,
            input.published as i64,
        ],
    )?;
    let id = conn.last_insert_rowid();
    get_by_id(conn, id)
}

pub fn update(conn: &Connection, id: i64, input: &ArticleInput) -> Result<ArticleFull, rusqlite::Error> {
    let slug = input.slug.clone().unwrap_or_else(|| slugify(&input.title));
    let tags_str = serde_json::to_string(&input.tags)
        .unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "UPDATE articles SET title = ?1, slug = ?2, summary = ?3, content = ?4,
                tags = ?5, published_at = ?6, published = ?7
         WHERE id = ?8",
        rusqlite::params![
            input.title,
            slug,
            input.summary,
            input.content,
            tags_str,
            input.published_at,
            input.published as i64,
            id,
        ],
    )?;
    get_by_id(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM articles WHERE id = ?1", rusqlite::params![id])?;
    if conn.changes() == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}
