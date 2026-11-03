-- Hasil analisis bukti dukung dan status pemindaian per periode.

CREATE TABLE IF NOT EXISTS evidence_analysis (
    evidence_id TEXT PRIMARY KEY NOT NULL,
    row_id INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'manual',
    reason TEXT NOT NULL DEFAULT '',
    ocr_text TEXT NOT NULL DEFAULT '',
    has_datetime INTEGER NOT NULL DEFAULT 0,
    has_address INTEGER NOT NULL DEFAULT 0,
    has_coords INTEGER NOT NULL DEFAULT 0,
    exif_datetime TEXT NOT NULL DEFAULT '',
    exif_lat REAL,
    exif_lon REAL,
    analyzed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS evidence_analysis_row
    ON evidence_analysis(row_id);

CREATE TABLE IF NOT EXISTS documentation_runs (
    period_id TEXT PRIMARY KEY NOT NULL,
    status TEXT NOT NULL DEFAULT 'idle',
    total INTEGER NOT NULL DEFAULT 0,
    done INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
