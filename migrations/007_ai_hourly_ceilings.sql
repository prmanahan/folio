-- 007_ai_hourly_ceilings.sql
-- Site-wide hourly request ceilings for the chat and fit AI endpoints,
-- shared across all visitors regardless of any per-visitor limit.
--
-- Seeded values are a placeholder set by Puck (task #3558), not by
-- Peter — update ai.chat_hourly_ceiling / ai.fit_hourly_ceiling once real
-- Anthropic usage data is available. INSERT OR IGNORE matches migration
-- 005's belt-and-braces posture: a forced re-run is a no-op against a
-- hand-edited value.

INSERT OR IGNORE INTO site_config (key, value) VALUES ('ai.chat_hourly_ceiling', '100');
INSERT OR IGNORE INTO site_config (key, value) VALUES ('ai.fit_hourly_ceiling', '50');
