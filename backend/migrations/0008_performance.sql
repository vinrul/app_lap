-- Katalog media desain selalu menyaring status dan tipe konten pada tabel
-- bukti dukung; tanpa index ini setiap permintaan memindai seluruh tabel.
CREATE INDEX IF NOT EXISTS import_evidence_status_type
    ON import_evidence(status, content_type);

-- Statistik planner sebelumnya belum pernah dibuat sehingga SQLite menebak
-- kardinalitas pada tabel besar. ANALYZE hanya berjalan sekali di migrasi ini.
ANALYZE;
