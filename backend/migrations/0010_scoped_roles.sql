-- Peran baru `polres` dan `polda` bersifat baca-saja dan terikat ke unit.
-- SQLite tidak dapat mengubah CHECK, jadi tabel users dibangun ulang.
PRAGMA foreign_keys = OFF;

CREATE TABLE users_new (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('admin', 'operator', 'polres', 'polda')),
    polres TEXT NOT NULL DEFAULT '',
    satker TEXT NOT NULL DEFAULT '',
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO users_new(id, username, password_hash, role, active, created_at)
SELECT id, username, password_hash, role, active, created_at FROM users;

DROP TABLE users;
ALTER TABLE users_new RENAME TO users;

PRAGMA foreign_keys = ON;
