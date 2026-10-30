ALTER TABLE report_periods ADD COLUMN archived_at TEXT;

CREATE INDEX IF NOT EXISTS report_periods_archived ON report_periods(archived_at);
