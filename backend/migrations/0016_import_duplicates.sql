-- Baris yang tersaring oleh deduplikasi operasional tetap disimpan di tabel
-- terpisah agar analisis Anev (duplikasi isian dan tautan berulang) dapat
-- memeriksa data mentah tanpa mengubah hitungan laporan utama.
CREATE TABLE IF NOT EXISTS import_duplicate_rows (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    import_id TEXT NOT NULL REFERENCES imports(id) ON DELETE CASCADE,
    source_row INTEGER NOT NULL,
    tanggal TEXT NOT NULL DEFAULT '',
    waktu_kirim_wib TEXT NOT NULL DEFAULT '',
    satuan TEXT NOT NULL DEFAULT '',
    tingkat TEXT NOT NULL DEFAULT '',
    polres TEXT NOT NULL DEFAULT '',
    polsek TEXT NOT NULL DEFAULT '',
    level TEXT NOT NULL DEFAULT '',
    unit TEXT NOT NULL DEFAULT '',
    satker TEXT NOT NULL DEFAULT '',
    kode TEXT NOT NULL DEFAULT '',
    kegiatan TEXT NOT NULL DEFAULT '',
    isian_laporan TEXT NOT NULL DEFAULT '',
    bukti_dukung TEXT NOT NULL DEFAULT '',
    dedupe_key TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS import_duplicate_rows_import
    ON import_duplicate_rows(import_id);
CREATE INDEX IF NOT EXISTS import_duplicate_rows_tanggal
    ON import_duplicate_rows(tanggal);
CREATE INDEX IF NOT EXISTS import_duplicate_rows_satuan
    ON import_duplicate_rows(satuan);

-- Import lama belum memiliki baris duplikat. Nilai 0 memicu tombol
-- "Siapkan analisis duplikat" di tab Anev; setelah diproses nilainya 1.
ALTER TABLE imports ADD COLUMN duplicates_synced INTEGER NOT NULL DEFAULT 0;
