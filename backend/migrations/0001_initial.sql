PRAGMA foreign_keys = ON;

CREATE TABLE users (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('admin', 'operator')),
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE sessions (
    token TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE report_periods (
    id TEXT PRIMARY KEY NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    label TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (start_date <= end_date)
);

CREATE TABLE imports (
    id TEXT PRIMARY KEY NOT NULL,
    period_id TEXT NOT NULL REFERENCES report_periods(id),
    filename TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    stored_path TEXT NOT NULL,
    parser_version TEXT NOT NULL,
    row_count INTEGER NOT NULL,
    imported_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX imports_period_created ON imports(period_id, created_at DESC);

CREATE TABLE import_rows (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    import_id TEXT NOT NULL REFERENCES imports(id) ON DELETE CASCADE,
    source_row INTEGER NOT NULL,
    tanggal TEXT NOT NULL DEFAULT '',
    waktu_kirim_wib TEXT NOT NULL DEFAULT '',
    satuan TEXT NOT NULL DEFAULT '',
    tingkat TEXT NOT NULL DEFAULT '',
    polda TEXT NOT NULL DEFAULT '',
    polres TEXT NOT NULL DEFAULT '',
    polsek TEXT NOT NULL DEFAULT '',
    kode TEXT NOT NULL DEFAULT '',
    kegiatan TEXT NOT NULL DEFAULT '',
    quick_wins TEXT NOT NULL DEFAULT '',
    pejabat_pelaksana TEXT NOT NULL DEFAULT '',
    jabatan TEXT NOT NULL DEFAULT '',
    pelaksana TEXT NOT NULL DEFAULT '',
    pelapor TEXT NOT NULL DEFAULT '',
    status_penerimaan TEXT NOT NULL DEFAULT '',
    bukti_dukung TEXT NOT NULL DEFAULT '',
    keluhan_personel_polri TEXT NOT NULL DEFAULT '',
    isian_laporan TEXT NOT NULL DEFAULT '',
    raw_json TEXT NOT NULL,
    UNIQUE(import_id, source_row)
);

CREATE INDEX import_rows_date ON import_rows(tanggal);
CREATE INDEX import_rows_topic ON import_rows(quick_wins, kode);
CREATE INDEX import_rows_period_import ON import_rows(import_id, tanggal);

CREATE TABLE report_drafts (
    id TEXT PRIMARY KEY NOT NULL,
    period_id TEXT NOT NULL REFERENCES report_periods(id),
    import_id TEXT NOT NULL REFERENCES imports(id),
    content_json TEXT NOT NULL DEFAULT '{}',
    updated_by TEXT NOT NULL REFERENCES users(id),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(period_id, import_id)
);

CREATE TABLE report_runs (
    id TEXT PRIMARY KEY NOT NULL,
    period_id TEXT NOT NULL REFERENCES report_periods(id),
    import_id TEXT NOT NULL REFERENCES imports(id),
    content_json TEXT NOT NULL,
    metrics_json TEXT NOT NULL,
    pdf_path TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT REFERENCES users(id),
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    details_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX audit_events_created ON audit_events(created_at DESC);
