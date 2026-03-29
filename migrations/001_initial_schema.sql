-- 001_initial_schema.sql
-- Initial schema for folio

CREATE TABLE IF NOT EXISTS _migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS candidate_profile (
    id INTEGER PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    name TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT '',
    title TEXT NOT NULL DEFAULT '',
    location TEXT NOT NULL DEFAULT '',
    phone TEXT NOT NULL DEFAULT '',
    linkedin_url TEXT NOT NULL DEFAULT '',
    github_url TEXT NOT NULL DEFAULT '',
    twitter_url TEXT NOT NULL DEFAULT '',
    elevator_pitch TEXT NOT NULL DEFAULT '',
    availability_status TEXT NOT NULL DEFAULT '',
    availability_date TEXT NOT NULL DEFAULT '',
    remote_preference TEXT NOT NULL DEFAULT '',
    target_titles TEXT NOT NULL DEFAULT '[]',
    target_company_stages TEXT NOT NULL DEFAULT '[]',
    career_narrative TEXT NOT NULL DEFAULT '',
    looking_for TEXT NOT NULL DEFAULT '',
    not_looking_for TEXT NOT NULL DEFAULT '',
    management_style TEXT NOT NULL DEFAULT '',
    work_style TEXT NOT NULL DEFAULT '',
    salary_min INTEGER,
    salary_max INTEGER
);

INSERT OR IGNORE INTO candidate_profile (id) VALUES (1);

CREATE TABLE IF NOT EXISTS experiences (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    company_name TEXT NOT NULL DEFAULT '',
    title TEXT NOT NULL DEFAULT '',
    location TEXT NOT NULL DEFAULT '',
    start_date TEXT NOT NULL DEFAULT '',
    end_date TEXT,
    is_current INTEGER NOT NULL DEFAULT 0,
    summary TEXT NOT NULL DEFAULT '',
    bullet_points TEXT NOT NULL DEFAULT '[]',
    display_order INTEGER NOT NULL DEFAULT 0,
    title_progression TEXT NOT NULL DEFAULT '',
    quantified_impact TEXT NOT NULL DEFAULT '{}',
    why_joined TEXT NOT NULL DEFAULT '',
    why_left TEXT NOT NULL DEFAULT '',
    actual_contributions TEXT NOT NULL DEFAULT '',
    proudest_achievement TEXT NOT NULL DEFAULT '',
    would_do_differently TEXT NOT NULL DEFAULT '',
    challenges_faced TEXT NOT NULL DEFAULT '',
    lessons_learned TEXT NOT NULL DEFAULT '',
    manager_would_say TEXT NOT NULL DEFAULT '',
    reports_would_say TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS skills (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    skill_name TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT 'moderate',
    years_experience INTEGER NOT NULL DEFAULT 0,
    last_used TEXT NOT NULL DEFAULT '',
    self_rating INTEGER NOT NULL DEFAULT 3,
    evidence TEXT NOT NULL DEFAULT '',
    honest_notes TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS education (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    degree TEXT NOT NULL DEFAULT '',
    institution TEXT NOT NULL DEFAULT '',
    location TEXT NOT NULL DEFAULT '',
    start_year TEXT NOT NULL DEFAULT '',
    end_year TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS gaps_weaknesses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    gap_type TEXT NOT NULL DEFAULT 'skill',
    description TEXT NOT NULL DEFAULT '',
    why_its_a_gap TEXT NOT NULL DEFAULT '',
    interest_in_learning INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS values_culture (
    id INTEGER PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    must_haves TEXT NOT NULL DEFAULT '',
    dealbreakers TEXT NOT NULL DEFAULT '',
    management_style_preferences TEXT NOT NULL DEFAULT '',
    team_size_preferences TEXT NOT NULL DEFAULT '',
    how_handle_conflict TEXT NOT NULL DEFAULT '',
    how_handle_ambiguity TEXT NOT NULL DEFAULT '',
    how_handle_failure TEXT NOT NULL DEFAULT ''
);

INSERT OR IGNORE INTO values_culture (id) VALUES (1);

CREATE TABLE IF NOT EXISTS faq_responses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    question TEXT NOT NULL DEFAULT '',
    answer TEXT NOT NULL DEFAULT '',
    is_common_question INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS ai_instructions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    instruction_type TEXT NOT NULL DEFAULT 'honesty',
    instruction TEXT NOT NULL DEFAULT '',
    priority INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL DEFAULT '',
    slug TEXT NOT NULL UNIQUE,
    summary TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    tech_stack TEXT NOT NULL DEFAULT '[]',
    url TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    published INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS articles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL DEFAULT '',
    slug TEXT NOT NULL UNIQUE,
    summary TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    tags TEXT NOT NULL DEFAULT '[]',
    published_at TEXT,
    published INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    label TEXT NOT NULL DEFAULT '',
    url TEXT NOT NULL DEFAULT '',
    icon TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS admin_sessions (
    token TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS rate_limits (
    ip TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    request_count INTEGER NOT NULL DEFAULT 0,
    window_start TEXT NOT NULL,
    PRIMARY KEY (ip, endpoint)
);
