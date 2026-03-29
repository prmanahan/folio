-- 002_agents.sql
-- Agents table for the public Agents page

CREATE TABLE IF NOT EXISTS agents (
    id INTEGER PRIMARY KEY,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    name TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT '',
    short_role TEXT NOT NULL DEFAULT '',
    model TEXT NOT NULL DEFAULT 'sonnet',
    personality_blurb TEXT NOT NULL DEFAULT '',
    responsibilities TEXT NOT NULL DEFAULT '[]',
    avatar_filename TEXT NOT NULL DEFAULT '',
    display_order INTEGER NOT NULL DEFAULT 0,
    is_featured INTEGER NOT NULL DEFAULT 0,
    is_review_gate INTEGER NOT NULL DEFAULT 0,
    published INTEGER NOT NULL DEFAULT 1
) STRICT;
