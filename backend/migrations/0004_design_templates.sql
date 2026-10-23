PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS design_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    layout_json TEXT NOT NULL DEFAULT '{}',
    created_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS design_templates_created ON design_templates(created_at DESC);
