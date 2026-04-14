-- 004_profile_pitch_split.sql
-- Split candidate_profile.elevator_pitch into pitch_short (hub) + pitch_long (resume).
--
-- Migration strategy (forward-only, no rollback):
--   1. Add pitch_short + pitch_long columns with default ''.
--   2. Copy existing elevator_pitch into pitch_long verbatim
--      (existing data is the long-form pitch — Peter supplies pitch_short
--       as content work after the migration lands).
--   3. Drop the old elevator_pitch column.
--
-- Length constraints (≤280 / ≤1500) and non-empty enforcement live at the
-- application/validation layer, NOT the DB. This lets seed data and post-migration
-- backfill operate without a second migration. See
-- libs/site-core/models/profile.rs::ProfileInput::validate for the rules.

ALTER TABLE candidate_profile ADD COLUMN pitch_short TEXT NOT NULL DEFAULT '';
ALTER TABLE candidate_profile ADD COLUMN pitch_long  TEXT NOT NULL DEFAULT '';

UPDATE candidate_profile SET pitch_long = elevator_pitch WHERE id = 1;

ALTER TABLE candidate_profile DROP COLUMN elevator_pitch;
