# Laporan Mingguan PBM — Polda Bali

Aplikasi lokal untuk mengubah export Excel **Quick Wins / Qins Polri** menjadi laporan mingguan PDF resmi. Aplikasi digunakan oleh operator dan administrator Polda Bali untuk impor data, pemeriksaan kualitas, penyuntingan narasi, dan pengarsipan dokumen.

## Fitur

- Impor workbook `.xlsx` sheet `Data` dengan pemetaan 18 kolom berdasarkan nama header.
- Validasi berkas, tanggal, duplikasi, baris di luar periode, dan pratinjau sebelum penyimpanan.
- Riwayat versi impor per periode; unggahan baru tidak menghapus versi lama.
- Perhitungan metrik otomatis dari kolom `Isian laporan`:
  - Berantas narkoba (orang ditangkap, kawasan, rehabilitasi, penyuluhan)
  - Green Policing (bibit, luas area, kerja bakti, penanaman)
  - Jumat Keliling, Subuh Keliling, Minggu Kasih (jemaah, lokasi ibadah, polres/polsek)
  - Aspirasi masyarakat dan bentuk tindak lanjut
  - Amplifikasi media (publikasi, dokumentasi, kanal, kegiatan)
- Penyuntingan narasi laporan per bagian serta blok penandatangan.
- Editor desain visual per laporan: tambah teks (perataan horizontal dan vertikal), kotak, gambar (unggah atau URL), ikon Lucide (warna dan ketebalan garis dapat diubah), kartu, tabel manual, dan grafik bar, bar horizontal, line, pie, atau donut; tabel/grafik dapat diikat ke sumber data metrik; kartu KPI dapat diedit langsung atau di-detach menjadi kartu mandiri; geser/resize/style blok laporan; editor menampilkan satu halaman dalam satu waktu dengan navigasi Sebelumnya/Berikutnya serta zoom −/+ (Ctrl +/−); edit teks blok langsung di kanvas; detach tabel/grafik lama menjadi objek manual dengan tombol Reset ke data; panel layer untuk sembunyikan dan urutkan objek; gabungkan objek agar bergerak bersama; simpan dan terapkan template desain lintas periode (gambar ikut disalin); lalu terapkan layout ke pratinjau dan PDF.
- Pratinjau HTML sebelum penerbitan.
- Pembuatan PDF 9 halaman dengan format laporan resmi, penomoran halaman, tabel sebaran wilayah, dan grafik batang.
- Arsip PDF dengan snapshot metrik dan konten editorial.
- Autentikasi sesi, peran `admin`/`operator`, audit event, dan pembatasan koneksi loopback.
- Antarmuka Svelte 5 responsif, modular, dan menggunakan Tailwind CSS v4.

## Kebutuhan

- Windows 10/11
- Bun 1.4+
- Rust stable 1.99+
- Google Chrome atau Microsoft Edge terpasang (untuk konversi HTML → PDF)

## Persiapan pertama

1. Buat akun administrator:

   ```powershell
   .\bootstrap-admin.ps1
   ```

   Nama pengguna wajib diisi dan kata sandi minimal 12 karakter. Tidak ada kredensial bawaan.

2. Jalankan aplikasi:

   ```powershell
   .\start.ps1
   ```

3. Buka `http://127.0.0.1:4317`.

Aplikasi hanya mendengarkan koneksi loopback. Frontend dibangun oleh `start.ps1` dan disajikan oleh backend pada origin yang sama.

## Alur laporan

1. Masuk dengan akun admin/operator.
2. Buat periode dengan rentang tanggal yang jelas.
3. Unggah workbook `.xlsx` pada tab **Impor**. Periksa jumlah baris, peringatan, duplikasi, dan pratinjau.
4. Simpan sebagai versi impor.
5. Buka tab **Ringkasan** untuk memeriksa metrik dan sebaran wilayah.
6. Buka tab **Editorial** untuk melengkapi ringkasan, narasi per bagian, serta penandatangan.
7. Gunakan **Pratinjau** untuk memeriksa HTML, lalu **Buat PDF**.
8. Unduh atau tinjau PDF pada tab **Dokumen**.

Metrik dihitung dari baris sumber dan isian terstruktur. Angka yang tidak tersedia di workbook (misalnya narasi kebijakan) tetap perlu diisi operator. Tinjau kembali angka agregat dan narasi sebelum menerbitkan PDF.

## Struktur proyek

```text
app_lap/
├── backend/                     # Rust + Axum + SQLx + SQLite
│   ├── migrations/
│   │   ├── 0001_initial.sql
│   │   └── 0002_reporting.sql
│   └── src/
│       ├── main.rs              # Entry point & CLI
│       ├── app.rs               # Perakitan router & middleware
│       ├── config.rs            # Konfigurasi environment
│       ├── db.rs                # Pool SQLite & migrasi
│       ├── error.rs             # Error API terpusat
│       ├── state.rs             # AppState
│       ├── server.rs            # Bootstrap server
│       ├── middleware/          # Loopback & security headers
│       ├── shared/              # Utilitas teks, tanggal, angka, audit
│       └── modules/
│           ├── auth/            # Login, sesi, bootstrap admin
│           ├── users/           # Pengelolaan akun & peran
│           ├── periods/         # Periode laporan
│           ├── imports/         # Parser Excel, validasi, data baris
│           ├── reports/         # Metrik, render PDF, draft editorial, layout desain
│           └── dashboard/       # Ringkasan lintas periode
└── frontend/                    # Bun + Svelte 5 SPA + Tailwind CSS v4
    └── src/
        ├── App.svelte
        ├── lib/
        │   ├── api/             # Client HTTP & endpoint per domain
        │   ├── components/
        │   │   ├── layout/      # AppShell, sidebar, topbar
        │   │   ├── ui/          # Button, modal, badge, tabel, dll.
        │   │   ├── charts/      # Bar list & grafik harian
        │   │   └── period/      # Panel impor, data, editorial, dokumen
        │   ├── stores/          # Auth, router, toast, meta
        │   ├── types/           # Tipe API
        │   └── utils/           # Format tanggal/angka & konstanta
        └── routes/
            ├── login/
            ├── dashboard/
            ├── periods/
            └── users/
```

## Perintah pengembangan

```powershell
# Frontend
cd frontend
bun run check       # svelte-check + tsc
bun run build       # build produksi ke frontend/dist

# Backend
cd backend
cargo check --all-targets
cargo test
cargo run --release
```

Pengujian backend mencakup parser workbook contoh, normalisasi tanggal/angka, klasifikasi tautan media, render 9 halaman, dan agregasi metrik dari 4.600 baris data contoh.

## Data dan cadangan

Default data berada di `data/`:

- `laporan.sqlite3` — akun, sesi, periode, impor, baris data, draft, audit, metadata PDF.
- `imports/` — salinan workbook sumber per versi.
- `reports/` — HTML dan PDF yang diarsipkan per versi laporan.
- `design/` — gambar yang diunggah untuk layout desain laporan.

Tutup aplikasi sebelum membuat cadangan konsisten. Salin seluruh direktori `data/` ke lokasi cadangan tepercaya; uji pemulihan dengan menyalin kembali direktori tersebut.

## Konfigurasi environment

| Variabel | Default | Keterangan |
|---|---|---|
| `LAPORAN_DATA_DIR` | `<root>/data` | Lokasi database dan berkas |
| `LAPORAN_BIND_ADDR` | `127.0.0.1` | Alamat bind server; isi `0.0.0.0` untuk akses LAN |
| `LAPORAN_ALLOWED_HOSTS` | kosong | Host yang diizinkan, dipisah koma; `*` untuk semua |
| `LAPORAN_PORT` | `4317` | Port server |
| `LAPORAN_CHROME` | auto-detect | Path `chrome.exe`/`msedge.exe` |
| `LAPORAN_POLDA_NAME` | `POLDA BALI` | Nama polda pada laporan |
| `LAPORAN_APP_NAME` | `Laporan Mingguan PBM` | Nama aplikasi |
| `LAPORAN_MAX_UPLOAD_MB` | `32` | Batas ukuran workbook |
| `LAPORAN_SESSION_HOURS` | `12` | Masa berlaku sesi login |
| `LAPORAN_FRONTEND_DIR` | `<root>/frontend/dist` | Lokasi build frontend |
| `LAPORAN_SECURE_COOKIE` | `false` | Set `true` bila memakai HTTPS |

Default server mengikat ke `127.0.0.1`. Untuk akses langsung dari LAN, set
`LAPORAN_BIND_ADDR=0.0.0.0` dan batasi `LAPORAN_ALLOWED_HOSTS` (misalnya
`192.168.1.10,laporan.local`). Jika `LAPORAN_ALLOWED_HOSTS` kosong saat bind non-loopback,
semua Host header diterima dan server menulis peringatan di log. Untuk VPS, tetap gunakan
loopback + reverse proxy seperti pada `deploy/DEPLOY.md`.
