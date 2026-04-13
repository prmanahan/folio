use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ProfilePublic {
    pub name: String,
    pub email: String,
    pub title: String,
    pub location: String,
    pub phone: String,
    pub linkedin_url: String,
    pub github_url: String,
    pub twitter_url: String,
    pub elevator_pitch: String,
    pub availability_status: String,
    pub availability_date: String,
    pub remote_preference: String,
}

impl ProfilePublic {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            name: row.get("name")?,
            email: row.get("email")?,
            title: row.get("title")?,
            location: row.get("location")?,
            phone: row.get("phone")?,
            linkedin_url: row.get("linkedin_url")?,
            github_url: row.get("github_url")?,
            twitter_url: row.get("twitter_url")?,
            elevator_pitch: row.get("elevator_pitch")?,
            availability_status: row.get("availability_status")?,
            availability_date: row.get("availability_date")?,
            remote_preference: row.get("remote_preference")?,
        })
    }
}

pub fn get_public(conn: &Connection) -> Result<ProfilePublic, rusqlite::Error> {
    conn.query_row(
        "SELECT name, email, title, location, phone, linkedin_url, github_url,
                twitter_url, elevator_pitch, availability_status, availability_date,
                remote_preference
         FROM candidate_profile WHERE id = 1",
        [],
        ProfilePublic::from_row,
    )
}

#[derive(Debug, Serialize)]
pub struct ProfileFull {
    pub created_at: String,
    pub updated_at: String,
    pub name: String,
    pub email: String,
    pub title: String,
    pub location: String,
    pub phone: String,
    pub linkedin_url: String,
    pub github_url: String,
    pub twitter_url: String,
    pub elevator_pitch: String,
    pub availability_status: String,
    pub availability_date: String,
    pub remote_preference: String,
    pub target_titles: serde_json::Value,
    pub target_company_stages: serde_json::Value,
    pub career_narrative: String,
    pub looking_for: String,
    pub not_looking_for: String,
    pub management_style: String,
    pub work_style: String,
    pub salary_min: Option<i64>,
    pub salary_max: Option<i64>,
}

impl ProfileFull {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let tt_str: String = row.get("target_titles")?;
        let target_titles: serde_json::Value =
            serde_json::from_str(&tt_str).unwrap_or(serde_json::json!([]));
        let tcs_str: String = row.get("target_company_stages")?;
        let target_company_stages: serde_json::Value =
            serde_json::from_str(&tcs_str).unwrap_or(serde_json::json!([]));
        Ok(Self {
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            name: row.get("name")?,
            email: row.get("email")?,
            title: row.get("title")?,
            location: row.get("location")?,
            phone: row.get("phone")?,
            linkedin_url: row.get("linkedin_url")?,
            github_url: row.get("github_url")?,
            twitter_url: row.get("twitter_url")?,
            elevator_pitch: row.get("elevator_pitch")?,
            availability_status: row.get("availability_status")?,
            availability_date: row.get("availability_date")?,
            remote_preference: row.get("remote_preference")?,
            target_titles,
            target_company_stages,
            career_narrative: row.get("career_narrative")?,
            looking_for: row.get("looking_for")?,
            not_looking_for: row.get("not_looking_for")?,
            management_style: row.get("management_style")?,
            work_style: row.get("work_style")?,
            salary_min: row.get("salary_min")?,
            salary_max: row.get("salary_max")?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct ProfileInput {
    pub name: String,
    pub email: String,
    pub title: String,
    pub location: String,
    pub phone: String,
    pub linkedin_url: String,
    pub github_url: String,
    pub twitter_url: String,
    pub elevator_pitch: String,
    pub availability_status: String,
    pub availability_date: String,
    pub remote_preference: String,
    pub target_titles: serde_json::Value,
    pub target_company_stages: serde_json::Value,
    pub career_narrative: String,
    pub looking_for: String,
    pub not_looking_for: String,
    pub management_style: String,
    pub work_style: String,
    pub salary_min: Option<i64>,
    pub salary_max: Option<i64>,
}

pub fn get_full(conn: &Connection) -> Result<ProfileFull, rusqlite::Error> {
    conn.query_row(
        "SELECT created_at, updated_at, name, email, title, location, phone,
                linkedin_url, github_url, twitter_url, elevator_pitch,
                availability_status, availability_date, remote_preference,
                target_titles, target_company_stages, career_narrative,
                looking_for, not_looking_for, management_style, work_style,
                salary_min, salary_max
         FROM candidate_profile WHERE id = 1",
        [],
        ProfileFull::from_row,
    )
}

pub fn update(conn: &Connection, input: &ProfileInput) -> Result<ProfileFull, rusqlite::Error> {
    let target_titles_str =
        serde_json::to_string(&input.target_titles).unwrap_or_else(|_| "[]".to_string());
    let target_company_stages_str =
        serde_json::to_string(&input.target_company_stages).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "UPDATE candidate_profile SET
            updated_at = datetime('now'),
            name = ?1, email = ?2, title = ?3, location = ?4, phone = ?5,
            linkedin_url = ?6, github_url = ?7, twitter_url = ?8,
            elevator_pitch = ?9, availability_status = ?10, availability_date = ?11,
            remote_preference = ?12, target_titles = ?13, target_company_stages = ?14,
            career_narrative = ?15, looking_for = ?16, not_looking_for = ?17,
            management_style = ?18, work_style = ?19, salary_min = ?20, salary_max = ?21
         WHERE id = 1",
        rusqlite::params![
            input.name,
            input.email,
            input.title,
            input.location,
            input.phone,
            input.linkedin_url,
            input.github_url,
            input.twitter_url,
            input.elevator_pitch,
            input.availability_status,
            input.availability_date,
            input.remote_preference,
            target_titles_str,
            target_company_stages_str,
            input.career_narrative,
            input.looking_for,
            input.not_looking_for,
            input.management_style,
            input.work_style,
            input.salary_min,
            input.salary_max,
        ],
    )?;
    get_full(conn)
}
