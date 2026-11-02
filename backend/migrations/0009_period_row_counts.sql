-- Jumlah baris unik per periode dihitung dari `dedupe_key`, yang sudah
-- dinormalisasi (trim + lowercase) saat impor. Index komposit membuat hitungan
-- hanya memindai index pada rentang tanggal, bukan seluruh tabel.
CREATE INDEX IF NOT EXISTS import_rows_tanggal_dedupe
    ON import_rows(tanggal, dedupe_key);

ANALYZE;
