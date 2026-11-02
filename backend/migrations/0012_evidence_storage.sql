-- Pembersihan bukti dukung mencari baris pemenang deduplikasi per kunci dan
-- memeriksa apakah baris tersebut sudah punya berkas siap. Media library juga
-- menyaring baris tersisih, sehingga index berikut mempercepat pencarian.
CREATE INDEX IF NOT EXISTS import_rows_import_dedupe
    ON import_rows(import_id, dedupe_key);

CREATE INDEX IF NOT EXISTS import_evidence_row_status
    ON import_evidence(row_id, status);

-- Dipakai saat memeriksa apakah sebuah berkas masih dirujuk baris lain sebelum
-- dihapus dari disk.
CREATE INDEX IF NOT EXISTS import_evidence_path
    ON import_evidence(stored_path);
