# Laporan Mingguan PBM — Polda Bali

Aplikasi lokal untuk mengubah export Excel **Quick Wins / Qins Polri** menjadi laporan mingguan PDF resmi. Aplikasi digunakan oleh operator dan administrator Polda Bali untuk impor data, pemeriksaan kualitas, penyuntingan narasi, dan pengarsipan dokumen.

## Fitur

- Impor workbook `.xlsx` sheet `Data` dengan pemetaan kolom berdasarkan nama header. Kolom wajib: `Tanggal`, `Waktu kirim (WIB)`, `Satuan`, `Tingkat`, `Polres`, `Polsek`, dan `Kode`; kolom lain opsional.
- Import berdiri sendiri di menu **Import data**, tidak lagi menempel pada satu periode. Satu berkas dapat dipakai oleh banyak periode.
- Validasi berkas, tanggal, duplikasi dalam berkas, dan pratinjau sebelum penyimpanan; duplikasi lintas berkas dibuang otomatis saat periode dibaca (kunci: tanggal, waktu kirim, satuan, tingkat, polres, polsek, dan kode).
- Periode harian (pilih tanggal), mingguan/rentang (pilih rentang tanggal), atau bulanan (pilih bulan dan tahun). Hierarki pelaksana diturunkan otomatis: Polres/Polsek kosong berarti Polda/satker Polda, Polsek kosong berarti Polres/satker Polres.
- Analisis otomatis dari `Isian laporan`: jumlah personel, bibit, luas area, jemaah, titik koordinat, dan tautan peta; tersedia sebagai kartu ringkas, tren harian, dan sebaran kelompok/kode/tingkat/unit.
- Drill-down pelaksana: klik satker Polda atau polres pada tabel breakdown untuk melihat ringkasan metrik, sebaran Quick Wins dan sub-kode, rincian polsek, serta kegiatan. Kode sub-kegiatan ditampilkan bersama deskripsinya. Klik QW1/QW1.2 untuk melihat cakupan polres/satker dan kegiatan sampai tingkat unit.
- Tab **Cakupan**: ringkasan pelaksanaan 8 kelompok Quick Wins dan seluruh sub-kode dengan jumlah satker Polda/polres pelaksana serta daftar unit yang belum melaksanakan. Klik kelompok QW untuk melihat jumlah kegiatan per polres dan satker Polda; klik sub-kode untuk membuka rincian berisi jumlah kegiatan per polres/satker, total personel, unit yang belum melaksanakan, serta daftar kegiatan beserta tautan peta. Menampilkan kelengkapan data per program (Green Policing, Jumat Keliling, Subuh Keliling, Minggu Kasih, Berantas Narkoba, Amplifikasi Media) per satker/polres: total metrik, jumlah baris/unit yang mengisi, dan daftar unit yang belum mengisi luas, bibit, personel, kerja bakti, jemaah, lokasi/masjid, narkoba, atau tautan media.
- Bukti dukung diunduh best-effort dari tautan gambar/PDF langsung, berkas Google Drive, ekspor Google Docs, dan folder Google Drive publik. Folder dibaca pada akar dan satu tingkat subfolder, maksimal 100 gambar/PDF per folder; berkas lain ditandai `skipped`. Media sosial dan tautan peta ditandai tidak dapat diunduh; pengguna dapat menjalankan ulang unduhan dari halaman Import data. Bukti yang sudah terunduh dapat dipratinjau langsung di aplikasi, termasuk galeri beberapa gambar/PDF dari folder Drive, tanpa harus mengunduh berkasnya terlebih dahulu. Antrean `pending` dilanjutkan otomatis saat backend start, dengan lock per import agar tidak ada worker ganda; permintaan baru saat worker berjalan dijadwalkan sebagai putaran ulang.
- Peta lokasi kegiatan dari koordinat baris sumber memakai Leaflet + OpenStreetMap, dimuat hanya saat detail dibuka. Tersedia marker, popup, tombol **Google Maps**, dan **Rute** ke tab baru. Tile peta diambil melalui backend dengan User-Agent aplikasi dan cache di `data/map-tiles/`; koneksi internet tetap diperlukan.
- Perhitungan metrik otomatis dari kolom `Isian laporan`:
  - Berantas narkoba (orang ditangkap, kawasan, rehabilitasi, penyuluhan)
  - Green Policing (bibit, luas area, kerja bakti, penanaman)
  - Jumat Keliling, Subuh Keliling, Minggu Kasih (jemaah, lokasi ibadah, polres/polsek)
  - Aspirasi masyarakat dan bentuk tindak lanjut
  - Amplifikasi media (publikasi, dokumentasi, kanal, kegiatan)
- Editor desain visual per laporan: saat desain periode belum pernah disimpan, tab Desain menampilkan pilihan blank template dan seluruh template desain; setelah desain disimpan, kanvas terakhir langsung ditampilkan. Editor mendukung teks (penambahan cepat judul periode, tanggal/rentang/bulan laporan, atau teks biasa; perataan horizontal dan vertikal; pilihan font sistem dan Google Fonts yang dibundel offline), kotak dan bentuk lain (lingkaran/elips, segitiga, segitiga siku, belah ketupat, jajar genjang, trapesium, segi lima, segi enam, bintang, panah, plus, hati) dengan isi warna solid atau gradien linear/radial hingga 8 warna serta opsi tanpa isi dan tanpa garis, gambar (media library atau URL), ikon Lucide (warna dan ketebalan garis dapat diubah), kartu, tabel manual, dan grafik bar, bar horizontal, line, pie, atau donut; tabel/grafik dapat diikat ke sumber data metrik (sumber Quick Wins dan Kode kegiatan menampilkan kode beserta deskripsi, dengan kolom Kode/Deskripsi/Jumlah pada tabel) maupun data Cakupan (kelompok Quick Wins, sub-kode, indikator per program, rincian per polres/satker, dan daftar unit yang belum mengisi); kartu KPI dapat diikat ke indikator Ringkasan/Analisis (narkoba, green policing, personel, bibit, luas, jemaah, koordinat, bukti dukung, jumlah polres/satker, periode), memakai enam template visual (klasik, ikon kiri, ikon kanan, tengah, panel ikon, minimal) dengan ikon Lucide yang bisa diganti warna dan latarnya, diedit langsung, atau di-detach menjadi kartu mandiri, termasuk kartu Cakupan (polres/satker aktif, kelompok/sub-kode terlaksana, kegiatan tercakup, serta kartu per kelompok, per program, dan per indikator program); geser/resize/style blok laporan; editor menampilkan satu halaman dalam satu waktu dengan navigasi Sebelumnya/Berikutnya serta zoom −/+ (Ctrl +/−); edit teks blok langsung di kanvas; detach tabel/grafik lama menjadi objek manual dengan tombol Reset ke data; panel layer untuk sembunyikan, kunci posisi, urutkan, atau gabungkan objek; menu klik kanan pada kanvas untuk menaikkan/menurunkan layer, menyembunyikan, mengunci, menduplikasi, menyalin, menempel (termasuk saat tidak ada objek terpilih), dan menghapus objek; pintasan Ctrl+C/Ctrl+V; gabungkan objek agar bergerak bersama; mode dokumen blank dengan orientasi portrait/landscape, jumlah halaman yang bisa ditambah atau dikurangi, serta duplikasi halaman aktif lengkap dengan objek, binding data, dan grup independen; simpan dan terapkan template desain lintas periode (mode, orientasi, jumlah halaman, dan gambar ikut disimpan); lalu terapkan layout ke pratinjau dan PDF.
- Latar halaman desain: setiap halaman dapat diberi warna solid, gradien linear/radial hingga 8 warna, atau **gambar dari media library** yang menutupi seluruh halaman (cover) dan tidak bisa digeser. Latar diterapkan ke halaman aktif atau semua halaman, ikut tersimpan di layout dan template (gambar disalin saat template diterapkan), serta tampil di kanvas, pratinjau, dan PDF. Duplikat halaman menyalin latar halaman aktif.
- Panel **Cakupan** di editor desain: pohon QW1–QW8 dengan sub-kode, polres, dan satker Polda pelaksana beserta jumlah kegiatannya, ditambah **program prioritas** (Green Policing, Jumat Keliling, Subuh Keliling, Minggu Kasih, Berantas Narkoba, Amplifikasi Media) dengan indikator (luas, bibit, personel, kerja bakti, jemaah, lokasi/masjid, narkoba, media), polres, dan satker Polda. Tarik kelompok, sub-kode, program, indikator, polres, atau satker ke kanvas (atau klik tombol +) lalu pilih grafik batang, grafik bar samping, tabel, atau kartu. Untuk kartu tersedia pilihan satu kartu ringkasan total, semua kartu per data, atau memilih sebagian kartu lewat daftar centang. Kartu disusun dalam grid dan tidak digabung secara default sehingga tiap kartu bisa digeser sendiri; opsi Gabungkan tersedia bila ingin bergerak bersama. Data panel mengikuti cakupan periode aktif dan bisa dimuat ulang dengan tombol Muat ulang data.
- Auto-save editor desain: perubahan disimpan otomatis 2 detik setelah edit terakhir dan maksimal setiap 30 detik saat pengeditan berlangsung terus-menerus. Status penyimpanan tampil di header; perubahan juga di-flush sebelum Pratinjau/PDF dan saat tab ditutup. Tombol Simpan manual tetap tersedia.
- Draft lokal editor desain: setiap percobaan simpan menulis salinan per pengguna dan periode ke `localStorage` (maksimal ±2 MB) sebagai jaring pengaman bila server gagal. Saat editor dibuka kembali, draft dibandingkan dengan versi server; pengguna dapat memulihkan draft lewat dialog, dengan peringatan bila server sudah berubah. Header menampilkan waktu draft, tombol coba lagi, dan buang draft.
- Media library desain: gambar yang pernah diunggah dari periode mana pun dan bukti dukung gambar pada periode aktif tersedia dalam satu panel; gambar dipilih dulu sebelum masuk kanvas, dan unggahan baru disimpan ke library tanpa langsung ditempatkan.
- Pembuatan PDF laporan resmi 9 halaman dengan penomoran halaman, tabel sebaran wilayah, dan grafik batang, atau dokumen blank sesuai orientasi dan jumlah halaman yang disimpan.
- Arsip PDF dengan snapshot metrik dan desain.
- Bagikan & presentasi publik: setiap PDF dapat menghasilkan tautan token acak yang dapat dibuka tanpa login. Tautan presentasi menampilkan satu halaman penuh seperti PowerPoint dengan navigasi klik/ketuk halaman, keyboard, tombol, dan geser layar, mode layar penuh, pilihan transisi (buka buku, geser, pudar, zoom), dan opsi tanpa efek; tautan PDF juga dapat disalin, dibuka, atau dibagikan melalui WhatsApp/email. Tombol **Bagikan & presentasi** pada editor desain membuat snapshot PDF terlebih dahulu agar tautan selalu sesuai desain terakhir.
- Hapus dokumen PDF oleh admin/operator: dokumen, file PDF/HTML, dan seluruh tautan publiknya dihapus permanen setelah konfirmasi, dengan audit log.
- Pratinjau HTML sebelum penerbitan.
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
6. Buka tab **Desain**. Jika desain periode belum pernah disimpan, pilih blank template atau salah satu template desain; jika sudah ada, kanvas terakhir langsung tampil. Susun layout, lalu **Pratinjau** dan **Buat PDF**.
7. Unduh, tinjau, atau buka **Bagikan & presentasi** pada tab **Dokumen**. Dialog menyediakan tautan presentasi publik, tautan PDF, salin tautan, WhatsApp, dan email.
8. Buka tautan presentasi tanpa login. Gunakan tombol navigasi, panah/spasi, geser layar, layar penuh, dan pilihan transisi halaman.

Metrik dihitung dari baris sumber dan isian terstruktur. Angka yang tidak tersedia di workbook (misalnya narasi kebijakan) tetap perlu diisi operator. Tinjau kembali angka agregat dan desain sebelum menerbitkan PDF.

## Struktur proyek

```text
app_lap/
├── backend/                     # Rust + Axum + SQLx + SQLite
│   ├── migrations/
│   │   ├── 0001_initial.sql
│   │   ├── 0002_reporting.sql
│   │   ├── 0003_report_design.sql
│   │   ├── 0004_design_templates.sql
│   │   ├── 0005_standalone_imports.sql
│   │   └── 0006_report_shares.sql
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
│           ├── reports/         # Metrik, render PDF, layout desain
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
        │           └── period/      # Panel ringkasan, analisis, data, desain, dokumen
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

Pengujian backend mencakup parser workbook contoh (termasuk dedupe dan hierarki unit), analisis isian laporan, normalisasi tautan bukti dukung, klasifikasi folder Google Drive dan batas 100 berkas, normalisasi tanggal/angka, klasifikasi tautan media, render 9 halaman/halaman blank, dan agregasi metrik.

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
| `LAPORAN_GOOGLE_DRIVE_API_KEY` | kosong | API key untuk membaca folder Google Drive publik |
| `LAPORAN_FRONTEND_DIR` | `<root>/frontend/dist` | Lokasi build frontend |
| `LAPORAN_SECURE_COOKIE` | `false` | Set `true` bila memakai HTTPS |

Default server mengikat ke `127.0.0.1`. Untuk akses langsung dari LAN, set
`LAPORAN_BIND_ADDR=0.0.0.0` dan batasi `LAPORAN_ALLOWED_HOSTS` (misalnya
`192.168.1.10,laporan.local`). Jika `LAPORAN_ALLOWED_HOSTS` kosong saat bind non-loopback,
semua Host header diterima dan server menulis peringatan di log. Untuk VPS, tetap gunakan
loopback + reverse proxy seperti pada `deploy/DEPLOY.md`.

### Menjalankan dengan API key Google Drive di Windows

```powershell
$env:LAPORAN_GOOGLE_DRIVE_API_KEY = '<API_KEY>'
.\start.ps1
```
