use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

// --- Length limits (per spec R6, R7, R11) -----------------------------------
//
// Enforced at the validation layer (not the DB). The admin UI mirrors these
// with maxlength + visible counters on pitch_short and pitch_long.

pub const MAX_NAME: usize = 32;
pub const MAX_TITLE: usize = 48;
pub const MAX_PITCH_SHORT: usize = 280;
pub const MAX_PITCH_LONG: usize = 1500;
pub const MAX_LOCATION: usize = 48;
pub const MAX_REMOTE_PREFERENCE: usize = 64;
pub const MAX_AVAILABILITY_STATUS: usize = 32;

/// Structured validation error returned to the API layer.
/// Maps to a 400 with body `{"error": "...", "field": "...", "limit": N}`.
#[derive(Debug)]
pub struct ProfileValidationError {
    pub field: &'static str,
    pub limit: Option<usize>,
    pub reason: &'static str,
}

impl std::fmt::Display for ProfileValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.limit {
            Some(n) => write!(
                f,
                "field `{}` {} (limit {} characters)",
                self.field, self.reason, n
            ),
            None => write!(f, "field `{}` {}", self.field, self.reason),
        }
    }
}

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
    pub pitch_short: String,
    pub pitch_long: String,
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
            pitch_short: row.get("pitch_short")?,
            pitch_long: row.get("pitch_long")?,
            availability_status: row.get("availability_status")?,
            availability_date: row.get("availability_date")?,
            remote_preference: row.get("remote_preference")?,
        })
    }
}

pub fn get_public(conn: &Connection) -> Result<ProfilePublic, rusqlite::Error> {
    conn.query_row(
        "SELECT name, email, title, location, phone, linkedin_url, github_url,
                twitter_url, pitch_short, pitch_long, availability_status,
                availability_date, remote_preference
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
    pub pitch_short: String,
    pub pitch_long: String,
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
            pitch_short: row.get("pitch_short")?,
            pitch_long: row.get("pitch_long")?,
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
    pub pitch_short: String,
    pub pitch_long: String,
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

impl ProfileInput {
    /// Per-field length validation. Counts Unicode scalar values
    /// (`chars().count()`) so multi-byte content is measured by user-visible
    /// length, not byte length.
    ///
    /// Rules (per spec R6, R7, R8, R11):
    ///   - `pitch_short`: required (non-empty), max 280 chars.
    ///   - `pitch_long`: optional (empty allowed), max 1500 chars.
    ///   - `name`, `title`, `location`, `remote_preference`,
    ///     `availability_status`: max as documented above.
    pub fn validate(&self) -> Result<(), ProfileValidationError> {
        check_max("name", &self.name, MAX_NAME)?;
        check_max("title", &self.title, MAX_TITLE)?;
        check_max("location", &self.location, MAX_LOCATION)?;
        check_max(
            "remote_preference",
            &self.remote_preference,
            MAX_REMOTE_PREFERENCE,
        )?;
        check_max(
            "availability_status",
            &self.availability_status,
            MAX_AVAILABILITY_STATUS,
        )?;

        // pitch_short: non-empty + ≤ 280
        if self.pitch_short.trim().is_empty() {
            return Err(ProfileValidationError {
                field: "pitch_short",
                limit: None,
                reason: "must not be empty",
            });
        }
        check_max("pitch_short", &self.pitch_short, MAX_PITCH_SHORT)?;

        // pitch_long: empty allowed (R7 caps length only)
        check_max("pitch_long", &self.pitch_long, MAX_PITCH_LONG)?;

        Ok(())
    }
}

fn check_max(field: &'static str, value: &str, limit: usize) -> Result<(), ProfileValidationError> {
    if value.chars().count() > limit {
        Err(ProfileValidationError {
            field,
            limit: Some(limit),
            reason: "exceeds maximum length",
        })
    } else {
        Ok(())
    }
}

pub fn get_full(conn: &Connection) -> Result<ProfileFull, rusqlite::Error> {
    conn.query_row(
        "SELECT created_at, updated_at, name, email, title, location, phone,
                linkedin_url, github_url, twitter_url, pitch_short, pitch_long,
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
            pitch_short = ?9, pitch_long = ?10,
            availability_status = ?11, availability_date = ?12,
            remote_preference = ?13, target_titles = ?14, target_company_stages = ?15,
            career_narrative = ?16, looking_for = ?17, not_looking_for = ?18,
            management_style = ?19, work_style = ?20, salary_min = ?21, salary_max = ?22
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
            input.pitch_short,
            input.pitch_long,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> ProfileInput {
        ProfileInput {
            name: "Alex Rivera".into(),
            email: "alex@example.com".into(),
            title: "Software Architect".into(),
            location: "Vancouver, BC".into(),
            phone: "".into(),
            linkedin_url: "".into(),
            github_url: "".into(),
            twitter_url: "".into(),
            pitch_short: "Short pitch.".into(),
            pitch_long: "Long pitch.".into(),
            availability_status: "open".into(),
            availability_date: "".into(),
            remote_preference: "remote".into(),
            target_titles: serde_json::json!([]),
            target_company_stages: serde_json::json!([]),
            career_narrative: "".into(),
            looking_for: "".into(),
            not_looking_for: "".into(),
            management_style: "".into(),
            work_style: "".into(),
            salary_min: None,
            salary_max: None,
        }
    }

    #[test]
    fn validate_baseline_ok() {
        assert!(base_input().validate().is_ok());
    }

    #[test]
    fn validate_pitch_short_at_limit_ok() {
        let mut i = base_input();
        i.pitch_short = "a".repeat(MAX_PITCH_SHORT);
        assert!(i.validate().is_ok());
    }

    #[test]
    fn validate_pitch_short_over_limit_rejected() {
        let mut i = base_input();
        i.pitch_short = "a".repeat(MAX_PITCH_SHORT + 1);
        let err = i.validate().unwrap_err();
        assert_eq!(err.field, "pitch_short");
        assert_eq!(err.limit, Some(MAX_PITCH_SHORT));
    }

    #[test]
    fn validate_pitch_short_empty_rejected() {
        let mut i = base_input();
        i.pitch_short = "".into();
        let err = i.validate().unwrap_err();
        assert_eq!(err.field, "pitch_short");
        assert!(err.limit.is_none());
    }

    #[test]
    fn validate_pitch_short_whitespace_only_rejected() {
        let mut i = base_input();
        i.pitch_short = "   \n  ".into();
        let err = i.validate().unwrap_err();
        assert_eq!(err.field, "pitch_short");
    }

    #[test]
    fn validate_pitch_long_at_limit_ok() {
        let mut i = base_input();
        i.pitch_long = "a".repeat(MAX_PITCH_LONG);
        assert!(i.validate().is_ok());
    }

    #[test]
    fn validate_pitch_long_over_limit_rejected() {
        let mut i = base_input();
        i.pitch_long = "a".repeat(MAX_PITCH_LONG + 1);
        let err = i.validate().unwrap_err();
        assert_eq!(err.field, "pitch_long");
        assert_eq!(err.limit, Some(MAX_PITCH_LONG));
    }

    #[test]
    fn validate_pitch_long_empty_ok() {
        let mut i = base_input();
        i.pitch_long = "".into();
        assert!(i.validate().is_ok());
    }

    #[test]
    fn validate_name_over_limit_rejected() {
        let mut i = base_input();
        i.name = "a".repeat(MAX_NAME + 1);
        let err = i.validate().unwrap_err();
        assert_eq!(err.field, "name");
        assert_eq!(err.limit, Some(MAX_NAME));
    }

    #[test]
    fn validate_title_over_limit_rejected() {
        let mut i = base_input();
        i.title = "a".repeat(MAX_TITLE + 1);
        let err = i.validate().unwrap_err();
        assert_eq!(err.field, "title");
    }

    #[test]
    fn validate_location_over_limit_rejected() {
        let mut i = base_input();
        i.location = "a".repeat(MAX_LOCATION + 1);
        let err = i.validate().unwrap_err();
        assert_eq!(err.field, "location");
    }

    #[test]
    fn validate_remote_preference_over_limit_rejected() {
        let mut i = base_input();
        i.remote_preference = "a".repeat(MAX_REMOTE_PREFERENCE + 1);
        let err = i.validate().unwrap_err();
        assert_eq!(err.field, "remote_preference");
    }

    #[test]
    fn validate_availability_status_over_limit_rejected() {
        let mut i = base_input();
        i.availability_status = "a".repeat(MAX_AVAILABILITY_STATUS + 1);
        let err = i.validate().unwrap_err();
        assert_eq!(err.field, "availability_status");
    }

    #[test]
    fn validate_unicode_counted_by_chars_not_bytes() {
        // 280 emoji = 280 chars but ~1120 bytes. Must pass.
        let mut i = base_input();
        i.pitch_short = "🌊".repeat(MAX_PITCH_SHORT);
        assert!(i.validate().is_ok());
        // 281 emoji should fail
        i.pitch_short = "🌊".repeat(MAX_PITCH_SHORT + 1);
        assert!(i.validate().is_err());
    }
}
