PRAGMA foreign_keys = ON;

ALTER TABLE report_runs ADD COLUMN layout_json TEXT NOT NULL DEFAULT '{}';

CREATE TABLE IF NOT EXISTS report_layouts (
    period_id TEXT NOT NULL,
    import_id TEXT NOT NULL,
    layout_json TEXT NOT NULL DEFAULT '{}',
    updated_by TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (period_id, import_id),
    FOREIGN KEY (period_id) REFERENCES report_periods(id) ON DELETE CASCADE,
    FOREIGN KEY (import_id) REFERENCES imports(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS design_images (
    id TEXT PRIMARY KEY,
    period_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL,
    stored_path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    created_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (period_id) REFERENCES report_periods(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS design_images_period ON design_images(period_id, created_at DESC);
