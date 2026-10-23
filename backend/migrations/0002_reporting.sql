PRAGMA foreign_keys = ON;

ALTER TABLE imports ADD COLUMN skipped_rows INTEGER NOT NULL DEFAULT 0;
ALTER TABLE imports ADD COLUMN warnings_json TEXT NOT NULL DEFAULT '[]';

ALTER TABLE import_rows ADD COLUMN parsed_json TEXT NOT NULL DEFAULT '{}';

ALTER TABLE report_runs ADD COLUMN html_path TEXT NOT NULL DEFAULT '';
ALTER TABLE report_runs ADD COLUMN polda_name TEXT NOT NULL DEFAULT '';
ALTER TABLE report_runs ADD COLUMN source_filename TEXT NOT NULL DEFAULT '';
ALTER TABLE report_runs ADD COLUMN period_label TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS import_rows_import_kode ON import_rows(import_id, kode);
CREATE INDEX IF NOT EXISTS import_rows_import_quick_wins ON import_rows(import_id, quick_wins);
CREATE INDEX IF NOT EXISTS import_rows_import_polres ON import_rows(import_id, polres);
CREATE INDEX IF NOT EXISTS report_runs_period_created ON report_runs(period_id, created_at DESC);
