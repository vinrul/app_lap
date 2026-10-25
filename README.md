# Laporan Mingguan PBM — Polda Bali

Aplikasi lokal untuk mengubah export Excel **Quick Wins / Qins Polri** menjadi laporan mingguan PDF resmi. Aplikasi digunakan oleh operator dan administrator Polda Bali untuk impor data, pemeriksaan kualitas, penyuntingan narasi, dan pengarsipan dokumen.

## Fitur

- Impor workbook `.xlsx` sheet `Data` dengan pemetaan 18 kolom berdasarkan nama header.
- Import berdiri sendiri di menu **Import data**, tidak lagi menempel pada satu periode. Satu berkas dapat dipakai oleh banyak periode.
- Validasi berkas, tanggal, duplikasi dalam berkas, dan pratinjau sebelum penyimpanan; duplikasi lintas berkas dibuang otomatis saat periode dibaca (kunci: tanggal, waktu kirim, satuan, tingkat, polres, polsek, dan kode).
- Periode harian (pilih tanggal), mingguan/rentang (pilih rentang tanggal), atau bulanan (pilih bulan dan tahun). Hierarki pelaksana diturunkan otomatis: Polres/Polsek kosong berarti Polda/satker Polda, Polsek kosong berarti Polres/satker Polres.
- Analisis otomatis dari `Isian laporan`: jumlah personel, bibit, luas area, jemaah, titik koordinat, dan tautan peta; tersedia sebagai kartu ringkas, tren harian, dan sebaran kelompok/kode/tingkat/unit.
- Bukti dukung diunduh best-effort dari tautan gambar/PDF langsung, berkas Google Drive, dan ekspor Google Docs. Folder Drive, media sosial, dan tautan peta ditandai tidak dapat diunduh; pengguna dapat menjalankan ulang unduhan dari halaman Import data.
- Perhitungan metrik otomatis dari kolom `Isian laporan`:
  - Berantas narkoba (orang ditangkap, kawasan, rehabilitasi, penyuluhan)
  - Green Policing (bibit, luas area, kerja bakti, penanaman)
  - Jumat Keliling, Subuh Keliling, Minggu Kasih (jemaah, lokasi ibadah, polres/polsek)
  - Aspirasi masyarakat dan bentuk tindak lanjut
  - Amplifikasi media (publikasi, dokumentasi, kanal, kegiatan)
- Penyuntingan narasi laporan per bagian serta blok penandatangan.
- Editor desain visual per laporan: tambah teks (perataan horizontal dan vertikal, pilihan font sistem dan Google Fonts yang dibundel offline), kotak dan bentuk lain (lingkaran/elips, segitiga, segitiga siku, belah ketupat, jajar genjang, trapesium, segi lima, segi enam, bintang, panah, plus, hati) dengan isi warna solid atau gradien linear/radial hingga 8 warna serta opsi tanpa isi dan tanpa garis, gambar (unggah atau URL), ikon Lucide (warna dan ketebalan garis dapat diubah), kartu, tabel manual, dan grafik bar, bar horizontal, line, pie, atau donut; tabel/grafik dapat diikat ke sumber data metrik; kartu KPI dapat diedit langsung atau di-detach menjadi kartu mandiri; geser/resize/style blok laporan; editor menampilkan satu halaman dalam satu waktu dengan navigasi Sebelumnya/Berikutnya serta zoom −/+ (Ctrl +/−); edit teks blok langsung di kanvas; detach tabel/grafik lama menjadi objek manual dengan tombol Reset ke data; panel layer untuk sembunyikan dan urutkan objek; gabungkan objek agar bergerak bersama; mode dokumen blank dengan orientasi portrait/landscape dan jumlah halaman yang bisa ditambah atau dikurangi; simpan dan terapkan template desain lintas periode (mode, orientasi, jumlah halaman, dan gambar ikut disimpan); lalu terapkan layout ke pratinjau dan PDF.
- Pratinjau HTML sebelum penerbitan.
- Pembuatan PDF laporan resmi 9 halaman dengan penomoran halaman, tabel sebaran wilayah, dan grafik batang, atau dokumen blank sesuai orientasi dan jumlah halaman yang disimpan.
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

Secara default aplikasi hanya mendengarkan koneksi loopback. Frontend dibangun oleh `start.ps1` dan disajikan oleh backend pada origin yang sama.

## Alur laporan

1. Masuk dengan akun admin/operator.
2. Buka menu **Import data**, unggah workbook `.xlsx`, periksa pratinjau (baris valid, duplikat, tautan bukti, analisis), lalu simpan.
3. Buat periode pada menu **Periode** dan pilih jenisnya: harian, mingguan/rentang, atau bulanan. Periode otomatis mengambil baris dari seluruh import yang mencakup rentang tanggalnya.
4. Buka tab **Ringkasan** untuk memeriksa metrik, berkas impor, duplikat, dan sebaran wilayah.
5. Buka tab **Analisis** untuk personel, bibit, luas, jemaah, koordinat, tren harian, serta sebaran kode/unit. Tab **Data** memuat daftar baris beserta bukti dukung.
6. Buka tab **Editorial** untuk melengkapi ringkasan, narasi per bagian, serta penandatangan.
7. Gunakan tab **Desain** untuk menyusun layout, lalu **Pratinjau** dan **Buat PDF**.
8. Unduh atau tinjau PDF pada tab **Dokumen**.

Metrik dihitung dari baris sumber dan isian terstruktur. Angka yang tidak tersedia di workbook (misalnya narasi kebijakan) tetap perlu diisi operator. Tinjau kembali angka agregat dan narasi sebelum menerbitkan PDF.

## Struktur proyek

```text
app_lap/
├── backend/                     # Rust + Axum + SQLx + SQLite
│   ├── migrations/
│   │   ├── 0001_initial.sql
│   │   ├── 0002_reporting.sql
│   │   ├── 0003_report_design.sql
│   │   ├── 0004_design_templates.sql
│   │   └── 0005_standalone_imports.sql
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
│           ├── imports/         # Import mandiri, parser Excel, dedupe, analisis, bukti dukung
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
        │           └── period/      # Panel ringkasan, analisis, data, editorial, desain, dokumen
        │   ├── stores/          # Auth, router, toast, meta
        │   ├── types/           # Tipe API
        │   └── utils/           # Format tanggal/angka & konstanta
        └── routes/
            ├── login/
            ├── dashboard/
            ├── imports/
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

Pengujian backend mencakup parser workbook contoh (termasuk dedupe dan hierarki unit), analisis isian laporan, normalisasi tautan bukti dukung, normalisasi tanggal/angka, klasifikasi tautan media, render 9 halaman/halaman blank, dan agregasi metrik.

## Data dan cadangan

Default data berada di `data/`:

- `laporan.sqlite3` — akun, sesi, periode, impor, baris data, draft, audit, metadata PDF.
- `imports/` — salinan workbook sumber per import.
- `evidence/` — gambar/PDF bukti dukung hasil unduhan, dikelompokkan per import.
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
