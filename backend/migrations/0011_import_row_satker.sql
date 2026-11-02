-- Satker Polda tidak tersimpan di kolom unit (yang berisi nama Polda), melainkan
-- di pejabat pelaksana/pelaksana. Kolom turunan ini memakai aturan yang sama
-- dengan `satker_label` agar cakupan, filter, dan pilihan dropdown konsisten.
ALTER TABLE import_rows ADD COLUMN satker TEXT NOT NULL DEFAULT '';

UPDATE import_rows
SET satker = TRIM(
    CASE
        WHEN TRIM(pejabat_pelaksana) <> '' THEN
            CASE
                WHEN LOWER(TRIM(pejabat_pelaksana)) LIKE 'kabid %' THEN SUBSTR(TRIM(pejabat_pelaksana), 7)
                WHEN LOWER(TRIM(pejabat_pelaksana)) LIKE 'ka %' THEN SUBSTR(TRIM(pejabat_pelaksana), 4)
                WHEN LOWER(TRIM(pejabat_pelaksana)) LIKE 'dir %' THEN SUBSTR(TRIM(pejabat_pelaksana), 5)
                ELSE TRIM(pejabat_pelaksana)
            END
        WHEN TRIM(pelaksana) <> '' THEN
            CASE
                WHEN LOWER(TRIM(pelaksana)) LIKE 'kabid %' THEN SUBSTR(TRIM(pelaksana), 7)
                WHEN LOWER(TRIM(pelaksana)) LIKE 'ka %' THEN SUBSTR(TRIM(pelaksana), 4)
                WHEN LOWER(TRIM(pelaksana)) LIKE 'dir %' THEN SUBSTR(TRIM(pelaksana), 5)
                ELSE TRIM(pelaksana)
            END
        WHEN TRIM(unit) <> '' THEN
            CASE
                WHEN LOWER(TRIM(unit)) LIKE 'kabid %' THEN SUBSTR(TRIM(unit), 7)
                WHEN LOWER(TRIM(unit)) LIKE 'ka %' THEN SUBSTR(TRIM(unit), 4)
                WHEN LOWER(TRIM(unit)) LIKE 'dir %' THEN SUBSTR(TRIM(unit), 5)
                ELSE TRIM(unit)
            END
        ELSE
            CASE
                WHEN LOWER(TRIM(satuan)) LIKE 'kabid %' THEN SUBSTR(TRIM(satuan), 7)
                WHEN LOWER(TRIM(satuan)) LIKE 'ka %' THEN SUBSTR(TRIM(satuan), 4)
                WHEN LOWER(TRIM(satuan)) LIKE 'dir %' THEN SUBSTR(TRIM(satuan), 5)
                ELSE TRIM(satuan)
            END
    END
)
WHERE LOWER(TRIM(level)) = 'polda';

CREATE INDEX IF NOT EXISTS import_rows_satker ON import_rows(satker);
