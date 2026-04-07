CREATE TABLE page_hits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL,
    ip_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    UNIQUE(path, ip_hash)
) STRICT;

CREATE INDEX idx_page_hits_path ON page_hits(path);
CREATE INDEX idx_page_hits_created_at ON page_hits(created_at);
