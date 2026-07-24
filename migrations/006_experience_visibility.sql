-- 006_experience_visibility.sql
-- Page-composition control for `experiences` rows.
--
-- Spec #2715: adds `visible` so the maintainer can remove an experience from
-- the rendered public page without deleting it or hiding it from the AI
-- context. DEFAULT 1 makes this deploy behaviour-preserving: every existing
-- row stays visible until the maintainer applies the D2 cut later, as data,
-- via the admin API (separate content track — out of scope here).
--
-- `visible` is a page-composition control, not a confidentiality boundary:
-- hidden rows remain reachable through the public AI chat surface by design
-- (R-0003). See docs/specs/2026-07-24-experiences-visible-flag.md.

ALTER TABLE experiences ADD COLUMN visible INTEGER NOT NULL DEFAULT 1;
