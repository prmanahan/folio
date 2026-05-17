-- 005_site_config.sql
-- Runtime-mutable site configuration (key/value).
--
-- Spec #572: replaces the env-var path for AI model + max_tokens with a
-- DB-backed config table so operators can change the model without a redeploy.
--
-- Seed rows use INSERT OR IGNORE so a forced re-run of this migration is a
-- no-op against hand-edited values (Scenario 10 belt-and-braces). The runner
-- in libs/site-core/db/schema.rs already gates re-runs via the _migrations
-- tracking table; the OR IGNORE here is defense in depth.
--
-- Typed accessors and validation live at the application layer
-- (libs/site-core/db/config.rs, owned by Task 3). This migration is schema
-- + seed only.

CREATE TABLE IF NOT EXISTS site_config (
    key        TEXT PRIMARY KEY NOT NULL,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

INSERT OR IGNORE INTO site_config (key, value) VALUES ('ai.model_id',   'claude-sonnet-4-6');
INSERT OR IGNORE INTO site_config (key, value) VALUES ('ai.max_tokens', '5530');
