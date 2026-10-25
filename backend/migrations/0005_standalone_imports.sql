PRAGMA foreign_keys = OFF;

ALTER TABLE report_periods ADD COLUMN period_type TEXT NOT NULL DEFAULT 'range';

UPDATE report_periods
SET period_type = CASE
    WHEN start_date = end_date THEN 'daily'
    WHEN start_date = date(start_date, 'start of month')
         AND end_date = date(start_date, 'start of month', '+1 month', '-1 day') THEN 'monthly'
    ELSE 'range'
END;

CREATE TABLE imports_new (
    id TEXT PRIMARY KEY NOT NULL,
    filename TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    stored_path TEXT NOT NULL,
    parser_version TEXT NOT NULL,
    row_count INTEGER NOT NULL,
    skipped_rows INTEGER NOT NULL DEFAULT 0,
    duplicate_rows INTEGER NOT NULL DEFAULT 0,
    date_start TEXT NOT NULL DEFAULT '',
    date_end TEXT NOT NULL DEFAULT '',
    warnings_json TEXT NOT NULL DEFAULT '[]',
    imported_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO imports_new(
    id, filename, file_hash, stored_path, parser_version, row_count, skipped_rows,
    duplicate_rows, date_start, date_end, warnings_json, imported_by, created_at
)
SELECT
    i.id, i.filename, i.file_hash, i.stored_path, i.parser_version, i.row_count,
    i.skipped_rows, 0,
    COALESCE((SELECT MIN(r.tanggal) FROM import_rows r WHERE r.import_id = i.id), ''),
    COALESCE((SELECT MAX(r.tanggal) FROM import_rows r WHERE r.import_id = i.id), ''),
    i.warnings_json, i.imported_by, i.created_at
FROM imports i;

DROP TABLE imports;
ALTER TABLE imports_new RENAME TO imports;
CREATE INDEX IF NOT EXISTS imports_created ON imports(created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS imports_dates ON imports(date_start, date_end);

ALTER TABLE import_rows ADD COLUMN level TEXT NOT NULL DEFAULT '';
ALTER TABLE import_rows ADD COLUMN unit TEXT NOT NULL DEFAULT '';
ALTER TABLE import_rows ADD COLUMN dedupe_key TEXT NOT NULL DEFAULT '';
ALTER TABLE import_rows ADD COLUMN qw_title TEXT NOT NULL DEFAULT '';
ALTER TABLE import_rows ADD COLUMN personel_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE import_rows ADD COLUMN bibit_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE import_rows ADD COLUMN area_m2 REAL NOT NULL DEFAULT 0;
ALTER TABLE import_rows ADD COLUMN jamaah_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE import_rows ADD COLUMN latitude REAL;
ALTER TABLE import_rows ADD COLUMN longitude REAL;
ALTER TABLE import_rows ADD COLUMN map_url TEXT NOT NULL DEFAULT '';

UPDATE import_rows
SET level = CASE
        WHEN trim(polsek) <> '' THEN 'Polsek'
        WHEN trim(polres) <> '' THEN 'Polres'
        ELSE 'Polda'
    END,
    unit = CASE
        WHEN trim(polsek) <> '' THEN trim(polsek)
        WHEN trim(polres) <> '' THEN trim(polres)
        ELSE trim(satuan)
    END,
    dedupe_key = lower(trim(tanggal)) || '|' ||
        lower(trim(waktu_kirim_wib)) || '|' ||
        lower(trim(satuan)) || '|' ||
        lower(trim(tingkat)) || '|' ||
        lower(trim(polres)) || '|' ||
        lower(trim(polsek)) || '|' ||
        upper(trim(kode));

CREATE INDEX IF NOT EXISTS import_rows_dedupe ON import_rows(dedupe_key);
CREATE INDEX IF NOT EXISTS import_rows_level ON import_rows(level);

CREATE TABLE report_drafts_new (
    id TEXT PRIMARY KEY NOT NULL,
    period_id TEXT NOT NULL REFERENCES report_periods(id) ON DELETE CASCADE,
    content_json TEXT NOT NULL DEFAULT '{}',
    updated_by TEXT NOT NULL REFERENCES users(id),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(period_id)
);

INSERT INTO report_drafts_new(id, period_id, content_json, updated_by, updated_at)
SELECT id, period_id, content_json, updated_by, updated_at
FROM (
    SELECT d.*, ROW_NUMBER() OVER (
        PARTITION BY d.period_id ORDER BY d.updated_at DESC, d.rowid DESC
    ) AS row_number
    FROM report_drafts d
)
WHERE row_number = 1;

DROP TABLE report_drafts;
ALTER TABLE report_drafts_new RENAME TO report_drafts;

CREATE TABLE report_layouts_new (
    period_id TEXT PRIMARY KEY NOT NULL,
    layout_json TEXT NOT NULL DEFAULT '{}',
    updated_by TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (period_id) REFERENCES report_periods(id) ON DELETE CASCADE
);

INSERT INTO report_layouts_new(period_id, layout_json, updated_by, updated_at)
SELECT period_id, layout_json, updated_by, updated_at
FROM (
    SELECT l.*, ROW_NUMBER() OVER (
        PARTITION BY l.period_id ORDER BY l.updated_at DESC, l.rowid DESC
    ) AS row_number
    FROM report_layouts l
)
WHERE row_number = 1;

DROP TABLE report_layouts;
ALTER TABLE report_layouts_new RENAME TO report_layouts;

CREATE TABLE report_runs_new (
    id TEXT PRIMARY KEY NOT NULL,
    period_id TEXT NOT NULL REFERENCES report_periods(id) ON DELETE CASCADE,
    content_json TEXT NOT NULL,
    metrics_json TEXT NOT NULL,
    layout_json TEXT NOT NULL DEFAULT '{}',
    pdf_path TEXT NOT NULL,
    html_path TEXT NOT NULL DEFAULT '',
    polda_name TEXT NOT NULL DEFAULT '',
    source_filename TEXT NOT NULL DEFAULT '',
    period_label TEXT NOT NULL DEFAULT '',
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO report_runs_new(
    id, period_id, content_json, metrics_json, layout_json, pdf_path, html_path,
    polda_name, source_filename, period_label, created_by, created_at
)
SELECT
    id, period_id, content_json, metrics_json, layout_json, pdf_path, html_path,
    polda_name, source_filename, period_label, created_by, created_at
FROM report_runs;

DROP TABLE report_runs;
ALTER TABLE report_runs_new RENAME TO report_runs;
CREATE INDEX IF NOT EXISTS report_runs_period_created ON report_runs(period_id, created_at DESC);
CREATE INDEX IF NOT EXISTS report_runs_created ON report_runs(created_at DESC);

CREATE TABLE IF NOT EXISTS import_evidence (
    id TEXT PRIMARY KEY NOT NULL,
    import_id TEXT NOT NULL REFERENCES imports(id) ON DELETE CASCADE,
    row_id INTEGER NOT NULL REFERENCES import_rows(id) ON DELETE CASCADE,
    source_url TEXT NOT NULL,
    filename TEXT NOT NULL DEFAULT '',
    content_type TEXT NOT NULL DEFAULT '',
    stored_path TEXT NOT NULL DEFAULT '',
    size_bytes INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    error TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS import_evidence_row ON import_evidence(row_id);
CREATE INDEX IF NOT EXISTS import_evidence_import ON import_evidence(import_id);

PRAGMA foreign_keys = ON;
