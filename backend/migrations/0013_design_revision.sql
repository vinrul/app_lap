-- Revisi desain dipakai untuk sinkronisasi realtime. Setiap penyimpanan
-- menaikkan revisi sehingga klien dapat mengabaikan perubahan yang sudah
-- dilihat dan meminta sinkronisasi ulang bila tertinggal.
ALTER TABLE report_layouts ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;
