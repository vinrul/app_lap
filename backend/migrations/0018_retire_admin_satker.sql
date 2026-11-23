-- Peran `admin_satker` dari migrasi 0017 (versi awal) tidak dipakai lagi;
-- kemampuan desain kini melekat pada peran `polres`. Akun yang sempat dibuat
-- untuk satu polres diubah menjadi `polres` agar tetap dapat menyusun desain
-- dan PDF polresnya. Akun tanpa polres dinonaktifkan.
UPDATE users
SET role = 'polres'
WHERE role = 'admin_satker' AND TRIM(polres) <> '';

UPDATE users
SET active = 0
WHERE role = 'admin_satker';
