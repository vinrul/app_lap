PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS report_shares (
    id TEXT PRIMARY KEY NOT NULL,
    report_id TEXT NOT NULL UNIQUE REFERENCES report_runs(id) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at TEXT
);

CREATE INDEX IF NOT EXISTS report_shares_token ON report_shares(token);
