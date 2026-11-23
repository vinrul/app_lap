-- Peran `admin_satker` menyusun desain dan menerbitkan PDF untuk unitnya
-- sendiri. Desain, draft, dan dokumen dipisah per cakupan (`scope`); string
-- kosong berarti cakupan global Polda. SQLite tidak dapat mengubah CHECK atau
-- primary key, jadi tabel yang terdampak dibangun ulang.
PRAGMA foreign_keys = OFF;

CREATE TABLE users_new (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (
        role IN ('admin', 'operator', 'polres', 'polda', 'admin_satker')
    ),
    polres TEXT NOT NULL DEFAULT '',
    satker TEXT NOT NULL DEFAULT '',
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO users_new(id, username, password_hash, role, polres, satker, active, created_at)
SELECT id, username, password_hash, role, polres, satker, active, created_at FROM users;

DROP TABLE users;
ALTER TABLE users_new RENAME TO users;

CREATE TABLE report_layouts_new (
    period_id TEXT NOT NULL,
    scope TEXT NOT NULL DEFAULT '',
    layout_json TEXT NOT NULL DEFAULT '{}',
    updated_by TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    revision INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (period_id, scope),
    FOREIGN KEY (period_id) REFERENCES report_periods(id) ON DELETE CASCADE
);

INSERT INTO report_layouts_new(period_id, scope, layout_json, updated_by, updated_at, revision)
SELECT period_id, '', layout_json, updated_by, updated_at, revision FROM report_layouts;

DROP TABLE report_layouts;
ALTER TABLE report_layouts_new RENAME TO report_layouts;

CREATE TABLE report_drafts_new (
    id TEXT PRIMARY KEY NOT NULL,
    period_id TEXT NOT NULL REFERENCES report_periods(id) ON DELETE CASCADE,
    scope TEXT NOT NULL DEFAULT '',
    content_json TEXT NOT NULL DEFAULT '{}',
    updated_by TEXT NOT NULL REFERENCES users(id),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(period_id, scope)
);

INSERT INTO report_drafts_new(id, period_id, scope, content_json, updated_by, updated_at)
SELECT id, period_id, '', content_json, updated_by, updated_at FROM report_drafts;

DROP TABLE report_drafts;
ALTER TABLE report_drafts_new RENAME TO report_drafts;

ALTER TABLE report_runs ADD COLUMN scope TEXT NOT NULL DEFAULT '';
ALTER TABLE report_runs ADD COLUMN scope_label TEXT NOT NULL DEFAULT '';
CREATE INDEX IF NOT EXISTS report_runs_period_scope
    ON report_runs(period_id, scope, created_at DESC);

PRAGMA foreign_keys = ON;
