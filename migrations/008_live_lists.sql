-- 008_live_lists.sql
-- The last validated copy of each live list (spec R-15, Data Model): one row
-- per list, overwritten on each successful refresh, no history and no raw
-- response bodies. `document` is the list's validated document as JSON; the
-- server re-validates it at start (R-16), so the schema pins only the shape
-- of the row, not the document.
--
-- Forward-only. Creates one table and alters no existing one (R-15).
CREATE TABLE live_lists (
    name TEXT PRIMARY KEY CHECK (name IN ('projects', 'articles')),
    document TEXT NOT NULL
) STRICT;
