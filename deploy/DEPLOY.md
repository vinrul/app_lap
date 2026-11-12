# Deploy ke VPS (Ubuntu/Debian)

Arsitektur: browser → Nginx (TLS) → backend Rust `127.0.0.1:4317` → SQLite + Chrome headless.
Backend secara default hanya listen di loopback, jadi konfigurasi yang direkomendasikan adalah
memakai reverse proxy di mesin yang sama.

Panduan ini diasumsikan untuk Ubuntu 22.04/24.04 x86_64. Untuk distro lain, sesuaikan nama paket.
Untuk menjalankan instance kedua (Polda Jateng, port 4318) di VPS yang sama, lihat bagian 14.

## Prasyarat

- VPS minimal 2 GB RAM (Chrome headless butuh memori; build Rust juga). Jika hanya 1 GB, tambahkan swap 2 GB.
- Domain/subdomain dengan A record ke IP VPS.
- Port 22, 80, 443 terbuka. Port 4317 **tidak** perlu dibuka.
- Akses `sudo`.

## 1. Paket dasar

```bash
sudo apt update
sudo apt install -y build-essential pkg-config nginx sqlite3 curl wget unzip \
  fonts-liberation fonts-dejavu-core

# Rust (edition 2024 butuh Rust 1.85+; rustup lebih baru dari paket apt)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# Bun (untuk build frontend)
curl -fsSL https://bun.sh/install | bash
export PATH="$HOME/.bun/bin:$PATH"
bun --version

# Chrome stable (dipakai untuk HTML -> PDF)
wget -qO /tmp/chrome.deb https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb
sudo apt install -y /tmp/chrome.deb
command -v google-chrome-stable
```

## 2. User dan direktori

```bash
sudo useradd --system --user-group --create-home --home-dir /var/lib/laporan \
  --shell /usr/sbin/nologin laporan
sudo mkdir -p /opt/laporan/frontend /etc/laporan
sudo chown -R laporan:laporan /var/lib/laporan
```

Hasil akhir:

- `/opt/laporan/laporan_mingguan` — binary backend
- `/opt/laporan/frontend/dist` — hasil build SPA
- `/var/lib/laporan` — database, `imports/`, `reports/`
- `/etc/laporan/laporan.env` — konfigurasi

## 3. Salin dan build

Dari Windows, kirim source **tanpa** `target`, `node_modules`, `data`, dan artefak `.tmp-*`.
Build di direktori home agar tidak perlu memberi user biasa akses tulis ke `/opt`:

```bash
rsync -av --exclude target --exclude node_modules --exclude data \
  --exclude '.tmp-*' ./ user@VPS:~/app_lap/
```

Jika `rsync` tidak tersedia, gunakan WinSCP/SCP atau clone repository. Jangan menyalin binary Windows.

Di VPS, build:

```bash
cd ~/app_lap/frontend
bun install --frozen-lockfile
bun run build
sudo mkdir -p /opt/laporan/frontend
sudo rm -rf /opt/laporan/frontend/dist
sudo cp -r dist /opt/laporan/frontend/dist

cd ~/app_lap/backend
cargo build --release --locked
sudo rm -rf /opt/laporan/laporan_mingguan
sudo cp target/release/laporan_mingguan /opt/laporan/laporan_mingguan
sudo chmod 755 /opt/laporan/laporan_mingguan
sudo systemctl restart laporan
```

Frontend memakai hash routing dan disajikan langsung oleh backend, jadi tidak perlu aturan SPA fallback di Nginx.

## 4. Konfigurasi environment

```bash
sudo cp ~/app_lap/deploy/laporan.env.example /etc/laporan/laporan.env
sudo nano /etc/laporan/laporan.env
sudo chmod 600 /etc/laporan/laporan.env
```

Nilai penting:

- `LAPORAN_SECURE_COOKIE=true` — wajib karena diakses lewat HTTPS.
- `LAPORAN_CHROME=/usr/bin/google-chrome-stable` — sesuaikan hasil `command -v`.
- `LAPORAN_DATA_DIR` dan `LAPORAN_FRONTEND_DIR` harus persis seperti pada contoh.
- `LAPORAN_LOGO_PATH` — opsional, path logo instansi untuk web dan PDF. Bila kosong,
aplikasi memakai `<LAPORAN_DATA_DIR>/logo.png` jika ada, lalu logo bawaan.
- `LAPORAN_MAP_REGION` — `bali` atau `jateng`, menentukan profil peta pada template desain.
- `LAPORAN_GOOGLE_DRIVE_API_KEY` — opsional, wajib jika workbook memakai tautan folder Google Drive publik. Isi API key asli di `/etc/laporan/laporan.env`; jangan menyimpannya di file contoh atau repository.
- `LAPORAN_EVIDENCE_RETENTION_DAYS` — default `0`, bukti dukung baris yang kalah deduplikasi langsung dibersihkan. Isi jumlah hari bila ingin menyimpannya lebih lama.
- `LAPORAN_EVIDENCE_PURGE_SUPERSEDED` — default `true`. Set `false` bila ingin mematikan pembersihan otomatis.

Migrasi database berjalan otomatis saat server mulai (`sqlx::migrate!()` di `db.rs`). Tidak ada langkah migrasi manual.

## 5. Buat admin pertama

Jalankan sekali, sebelum service dinyalakan:

```bash
sudo -u laporan env \
  LAPORAN_DATA_DIR=/var/lib/laporan \
  LAPORAN_ADMIN_USER=admin \
  LAPORAN_ADMIN_PASSWORD='ganti-dengan-sandi-kuat' \
  /opt/laporan/laporan_mingguan bootstrap-admin
```

Sandi minimal 12 karakter. Perintah ini menolak membuat admin jika sudah ada.

## 6. systemd

```bash
sudo cp ~/app_lap/deploy/laporan.service /etc/systemd/system/laporan.service
sudo systemctl daemon-reload
sudo systemctl enable --now laporan
sudo systemctl status laporan --no-pager
journalctl -u laporan -f
```

### PaddleOCR (opsional)

Tesseract tetap dijalankan lebih dulu karena jauh lebih cepat. Sidecar PaddleOCR
hanya dipanggil untuk gambar yang belum lengkap setelah Tesseract, atau yang
tanggal pada watermark-nya berbeda dari tanggal laporan. Bila sidecar mati atau
belum siap, analisis tetap berjalan dengan Tesseract.

Prasyarat (Ubuntu/Debian):

```bash
sudo apt install -y python3-venv python3-pip libgomp1 libgl1 libglib2.0-0
```

Pasang berkas dan virtualenv terpisah:

```bash
sudo mkdir -p /opt/laporan/paddle-ocr
sudo cp ~/app_lap/paddle-ocr/app.py ~/app_lap/paddle-ocr/requirements.txt /opt/laporan/paddle-ocr/
sudo chown -R laporan:laporan /opt/laporan/paddle-ocr
sudo -u laporan python3 -m venv /opt/laporan/paddle-ocr/.venv
sudo -u laporan /opt/laporan/paddle-ocr/.venv/bin/pip install --upgrade pip
sudo -u laporan /opt/laporan/paddle-ocr/.venv/bin/pip install -r /opt/laporan/paddle-ocr/requirements.txt
```

Bila Python bawaan VPS terlalu baru dan wheel `paddlepaddle` belum tersedia,
pakai Python 3.12: `sudo apt install -y python3.12-venv` lalu
`sudo -u laporan python3.12 -m venv /opt/laporan/paddle-ocr/.venv`.

Unduh model sekali. Ukurannya sekitar 140 MB dan disimpan di
`$HOME/.paddlex/official_models`:

```bash
sudo -u laporan env \
  HOME=/home/laporan-data \
  PADDLE_PDX_ENABLE_MKLDNN_BYDEFAULT=False \
  /opt/laporan/paddle-ocr/.venv/bin/python -c \
  "from paddleocr import PaddleOCR; PaddleOCR(lang='id', use_doc_orientation_classify=False, use_doc_unwarping=False, use_textline_orientation=True)"
```

`HOME` harus sama dengan nilai di `laporan-ocr.env` dan dapat ditulis user
`laporan`. Bila `LAPORAN_DATA_DIR=/var/lib/laporan`, pakai `HOME=/var/lib/laporan`.
Untuk VPS tanpa akses internet, jalankan perintah di atas di mesin lain lalu
salin folder `~/.paddlex/official_models` ke lokasi yang sama di VPS.

Pasang environment dan service:

```bash
sudo cp ~/app_lap/deploy/laporan-ocr.env.example /etc/laporan/laporan-ocr.env
sudo nano /etc/laporan/laporan-ocr.env
sudo chmod 600 /etc/laporan/laporan-ocr.env
sudo cp ~/app_lap/deploy/laporan-ocr.service /etc/systemd/system/laporan-ocr.service
sudo systemctl daemon-reload
sudo systemctl enable --now laporan-ocr
journalctl -u laporan-ocr -f
```

Tunggu sampai model termuat, lalu pastikan healthcheck hijau:

```bash
curl -s http://127.0.0.1:4320/health
# {"status":"ok","engine":"PaddleOCR","version":"3.7.0",...}
```

Aktifkan di backend, lalu restart:

```bash
# Di /etc/laporan/laporan.env, hapus tanda # pada baris ini:
# LAPORAN_PADDLE_URL=http://127.0.0.1:4320
sudo systemctl restart laporan
journalctl -u laporan -n 30 --no-pager
```

Log backend harus memuat `sidecar PaddleOCR siap`. Setelah itu buka tab
**Dokumentasi** dan klik **Pindai ulang** agar gambar berstatus cek manual
dianalisis ulang dengan kedua mesin. Pemindaian berjalan di latar belakang dan
bisa dilanjutkan setelah restart.

Bila CPU VPS terbatas, turunkan `LAPORAN_PADDLE_MAX_SIDE` ke `1600`; bila
watermark kecil sering tidak terbaca, naikkan ke `2560`. `CPUQuota=200%` pada
unit service membatasi OCR agar server web tetap responsif.

## 7. Nginx dan TLS

```bash
sudo cp ~/app_lap/deploy/nginx-laporan.conf /etc/nginx/sites-available/laporan
sudo ln -sf /etc/nginx/sites-available/laporan /etc/nginx/sites-enabled/laporan
sudo nano /etc/nginx/sites-available/laporan   # ganti server_name
sudo nginx -t
sudo systemctl reload nginx

sudo apt install -y certbot python3-certbot-nginx
sudo certbot --nginx -d laporan.example.go.id
```

Saat certbot menawarkan redirect HTTP → HTTPS, pilih **redirect**. Tanpa HTTPS, cookie `Secure` tidak akan dikirim balik oleh browser sehingga login gagal.

**Pertahankan `proxy_set_header Host 127.0.0.1;`** karena backend default hanya mengizinkan Host
loopback. Alternatifnya, set `LAPORAN_ALLOWED_HOSTS=domain` pada env dan biarkan Host asli. Tanpa
salah satu dari itu, API membalas `403 Host tidak diizinkan`.

`client_max_body_size 40m` diperlukan karena batas unggah aplikasi 32 MB; default Nginx hanya 1 MB.
`proxy_read_timeout 300s` memberi waktu pembuatan PDF.

## 8. Firewall

```bash
sudo ufw allow OpenSSH
sudo ufw allow 'Nginx Full'
sudo ufw enable
```

Tidak perlu membuka 4317; backend hanya listen di `127.0.0.1`.

### Akses langsung tanpa Nginx

Jika memang ingin backend listen di semua interface:

```text
LAPORAN_BIND_ADDR=0.0.0.0
LAPORAN_ALLOWED_HOSTS=IP_VPS,domain-atau-host-lan
```

Ini membuka aplikasi tanpa TLS dan tanpa proteksi Nginx. Batasi juga lewat firewall, dan jangan
memakai `*` di `LAPORAN_ALLOWED_HOSTS` kecuali untuk uji sementara.

## 9. Uji

1. Buka `https://laporan.example.go.id`.
2. Login dengan admin.
3. Buat periode, impor workbook `.xlsx`, cek metrik, lalu buat PDF.
4. Pastikan cookie memakai `Secure` (DevTools → Application → Cookies).

Jika pembuatan PDF gagal, cek log lalu uji Chrome langsung:

```bash
sudo -u laporan google-chrome-stable --headless --disable-gpu \
  --print-to-pdf=/tmp/test.pdf about:blank
```

Bila muncul error sandbox `No usable sandbox!` (umum pada kernel VPS tertentu), buat wrapper:

```bash
sudo tee /usr/local/bin/chrome-laporan >/dev/null <<'EOF'
#!/bin/sh
exec /usr/bin/google-chrome-stable --no-sandbox "$@"
EOF
sudo chmod +x /usr/local/bin/chrome-laporan
```

Lalu set `LAPORAN_CHROME=/usr/local/bin/chrome-laporan` dan restart service. `--no-sandbox`
mengurangi isolasi browser; hanya gunakan bila diperlukan. Alternatif lain adalah mengaktifkan
unprivileged user namespace di kernel (`kernel.apparmor_restrict_unprivileged_userns=0` pada
Ubuntu 24.04), tetapi itu juga melemahkan isolasi sistem secara global. Bila `/dev/shm` VPS kecil,
tambahkan juga `--disable-dev-shm-usage` pada wrapper.

## 10. Data dan backup

Seluruh data ada di `/var/lib/laporan`. Backup SQLite secara online:

```bash
sudo mkdir -p /var/backups/laporan
sudo chown laporan:laporan /var/backups/laporan
sudo -u laporan sqlite3 /var/lib/laporan/laporan.sqlite3 \
  ".backup '/var/backups/laporan/db-$(date +%F).sqlite3'"
sudo -u laporan tar czf /var/backups/laporan/files-$(date +%F).tar.gz \
  --ignore-failed-read -C /var/lib/laporan imports reports design
```

Untuk backup penuh yang konsisten, hentikan service lebih dulu:

```bash
sudo systemctl stop laporan
sudo tar czf /var/backups/laporan/full-$(date +%F).tar.gz -C /var/lib laporan
sudo systemctl start laporan
```

## 11. Update aplikasi

```bash
# ganti binary dan/atau frontend/dist
sudo systemctl restart laporan
```

`systemctl stop` dan `systemctl restart` mengirim SIGTERM. Aplikasi berhenti dengan rapi:
permintaan HTTP yang sedang berjalan diselesaikan, worker unduhan bukti dukung berhenti
mengambil tugas baru, lalu koneksi SQLite ditutup. Unduhan yang belum selesai tetap
berstatus `pending` dan dilanjutkan otomatis saat service hidup lagi.

Migrasi baru dijalankan otomatis saat startup. Cek `journalctl -u laporan -n 50` bila gagal.

Desain laporan memakai satu koneksi WebSocket per tab untuk kolaborasi realtime
(`/api/ws`). Service berjalan langsung pada port `4317` tanpa Nginx; WebSocket memakai
origin yang sama sehingga tidak ada konfigurasi tambahan.

Bila nanti dipasang Nginx di depannya, blok `location /` harus meneruskan upgrade:

```nginx
proxy_http_version 1.1;
proxy_set_header Upgrade $http_upgrade;
proxy_set_header Connection "upgrade";
proxy_read_timeout 3600s;
```

Editor menyambung ulang otomatis dengan backoff dan mengirim heartbeat setiap 20 detik.
Sebagai jaring pengaman, editor juga memeriksa revisi layout tiap 10 detik, jadi halaman
tidak perlu dimuat ulang manual meskipun kanal realtime terhambat. Bila kanal belum
siap, penyimpanan otomatis jatuh ke `PATCH /layout`.

Port `4317` tanpa TLS mengirim cookie sesi dalam teks biasa. Batasi akses lewat VPN,
terowongan SSH, atau terminator TLS; setel juga `LAPORAN_ALLOWED_HOSTS` bila bind bukan
loopback.

Sinkronisasi desain memakai koordinasi dalam satu proses backend. Jalankan satu instance
`laporan_mingguan serve` per database; jangan menyalakan beberapa worker dengan database
yang sama.

### Perawatan bukti dukung

Versi ini menyimpan bukti dukung berdasarkan hash isi dan membersihkan bukti
dukung baris yang kalah deduplikasi. Setelah binary baru terpasang, jalankan
perawatan sekali (dry run dulu) untuk memindahkan berkas lama dan membebaskan
ruang. Hentikan service selama proses agar tidak berbenturan dengan unduhan baru:

```bash
sudo systemctl stop laporan

sudo -u laporan env \
  LAPORAN_DATA_DIR=/var/lib/laporan \
  /opt/laporan/laporan_mingguan evidence-maintenance --dry-run

sudo -u laporan env \
  LAPORAN_DATA_DIR=/var/lib/laporan \
  /opt/laporan/laporan_mingguan evidence-maintenance

sudo systemctl start laporan
```

Perintah ini dapat diulang kapan saja; berkas yang sudah rapi dilewati. Output
menampilkan jumlah berkas dipindai, baris tersisih dibersihkan, dan ruang yang
dibebaskan. Sesuaikan `LAPORAN_DATA_DIR` bila data tidak berada di `/var/lib/laporan`.

## 12. Memindahkan data lama dari Windows

Menyalin `data/` dari Windows bisa dilakukan, tetapi `report_runs.pdf_path`/`html_path` dan
`imports.stored_path` menyimpan path absolut Windows. Akibatnya PDF lama tidak dapat diunduh di
Linux dan path arsip menjadi tidak valid sampai diperbaiki. Path bukti dukung diperbaiki otomatis
oleh `evidence-maintenance`, yang menulis ulang `import_evidence.stored_path` menjadi path relatif
terhadap `LAPORAN_DATA_DIR`. Untuk deploy baru, mulai dengan data
kosong; bila data lama diperlukan, perbaiki path di SQLite atau buat ulang laporan.
Jangan menyalin `target/`, `node_modules/`, `.tmp-*`, atau `data/` kecuali memang ingin
memindahkan data.

## 13. Catatan keamanan

Aplikasi ini dirancang untuk satu komputer internal. Saat dipublikasikan ke internet:

- Tidak ada rate limit/login lockout di aplikasi. Gunakan Tailscale/WireGuard, allowlist IP di
  Nginx, atau basic auth tambahan bila memungkinkan.
- Wajib HTTPS dan `LAPORAN_SECURE_COOKIE=true` bila aplikasi diakses dari internet.
- `LAPORAN_BIND_ADDR=0.0.0.0` tanpa `LAPORAN_ALLOWED_HOSTS` menerima semua Host header; batasi
  allowlist dan firewall.
- Gunakan sandi admin yang kuat dan unik; jangan memakai akun bersama.
- Batasi akses SSH, rutin update OS dan Chrome, dan simpan backup di luar VPS.
- Aplikasi menyimpan workbook serta PDF di server; pastikan kebijakan retensi/privasi data sesuai.

## 14. Instance kedua: Polda Jateng (port 4318)

Satu VPS dapat menjalankan dua instance selama **database dan direktori data dipisah**.
Contoh ini mempertahankan Polda Bali di port `4317` dan menambahkan Polda Jateng di `4318`.

| Bagian | Polda Bali | Polda Jateng |
| --- | --- | --- |
| Service | `laporan` | `laporan-jateng` |
| Environment | `/etc/laporan/laporan.env` | `/etc/laporan/laporan-jateng.env` |
| Data | `/var/lib/laporan` | `/var/lib/laporan-jateng` |
| Binary + frontend | `/opt/laporan` | `/opt/laporan-jateng` |
| Port loopback | `4317` | `4318` |

Jangan mengarahkan dua service ke `LAPORAN_DATA_DIR` yang sama. Database SQLite,
folder `imports/`, `reports/`, dan `design/` harus terpisah agar tidak saling menimpa.

### 14.1 Salin binary dan frontend

```bash
sudo mkdir -p /opt/laporan-jateng/frontend /var/lib/laporan-jateng
sudo chown -R laporan:laporan /var/lib/laporan-jateng

sudo cp /opt/laporan/laporan_mingguan /opt/laporan-jateng/laporan_mingguan
sudo rm -rf /opt/laporan-jateng/frontend/dist
sudo cp -r /opt/laporan/frontend/dist /opt/laporan-jateng/frontend/dist
sudo chmod 755 /opt/laporan-jateng/laporan_mingguan
```

Jika `/opt/laporan` belum berisi hasil build, build dari repo seperti pada bagian 3 lalu
salin binary dan `frontend/dist` ke `/opt/laporan-jateng`.

### 14.2 Environment Polda Jateng

```bash
sudo cp ~/app_lap/deploy/laporan-jateng.env.example /etc/laporan/laporan-jateng.env
sudo nano /etc/laporan/laporan-jateng.env
sudo chmod 600 /etc/laporan/laporan-jateng.env
```

```bash
cd ~/app_lap_jateng/frontend
bun install --frozen-lockfile
bun run build
sudo mkdir -p /opt/laporan-jateng/frontend
sudo rm -rf /opt/laporan-jateng/frontend/dist
sudo cp -r dist /opt/laporan-jateng/frontend/dist

cd ~/app_lap_jateng/backend
cargo build --release --locked
sudo rm -rf /opt/laporan-jateng/laporan_mingguan
sudo cp target/release/laporan_mingguan /opt/laporan-jateng/laporan_mingguan
sudo chmod 755 /opt/laporan-jateng/laporan_mingguan
sudo systemctl restart laporan-jateng
```

Nilai yang wajib dicek:

- `LAPORAN_PORT=4318`
- `LAPORAN_DATA_DIR=/var/lib/laporan-jateng`
- `LAPORAN_FRONTEND_DIR=/opt/laporan-jateng/frontend/dist`
- `LAPORAN_POLDA_NAME=POLDA JATENG`
- `LAPORAN_TIMEZONE_LABEL=WIB` — jangan memakai `WITA` seperti instance Bali.
- `LAPORAN_MAP_REGION=jateng` — peta template memakai profil Jawa Tengah, bukan Bali.
- `LAPORAN_SECURE_COOKIE=true` bila diakses lewat HTTPS.
- `HOME=/var/lib/laporan-jateng` agar Chrome headless dapat menulis profil sementara.
- `LAPORAN_LOGO_PATH` — opsional, misalnya `/var/lib/laporan-jateng/logo.png`
atau `/home/laporan-data-jateng/logo.png`.

Jika instance Bali memakai data di `/home/laporan-data`, Anda dapat memakai pola yang sama
untuk Jateng, misalnya `/home/laporan-data-jateng`:

```bash
sudo install -d -o laporan -g laporan -m 750 /home/laporan-data-jateng
sudo -u laporan test -w /home/laporan-data-jateng && echo "direktori dapat ditulis"
```

Namun unit `laporan-jateng.service` memakai `ProtectHome=true`, sehingga direktori di
`/home` tidak dapat ditulis. Untuk pola itu, ubah pada unit service:

```ini
ProtectHome=false
ReadWritePaths=/home/laporan-data-jateng
```

lalu sesuaikan `LAPORAN_DATA_DIR=/home/laporan-data-jateng` dan
`HOME=/home/laporan-data-jateng` pada env. Cara paling aman tetap memakai
`/var/lib/laporan-jateng`.

Sidecar PaddleOCR dapat dipakai bersama. Aktifkan baris yang sama seperti instance Bali:

```text
LAPORAN_PADDLE_URL=http://127.0.0.1:4320
```

### 14.3 Admin pertama

Jalankan sekali sebelum service dinyalakan:

```bash
sudo -u laporan env \
  LAPORAN_DATA_DIR=/var/lib/laporan-jateng \
  LAPORAN_ADMIN_USER=admin \
  LAPORAN_ADMIN_PASSWORD='ganti-dengan-sandi-kuat' \
  /opt/laporan-jateng/laporan_mingguan bootstrap-admin
```

### 14.4 systemd

```bash
sudo cp ~/app_lap/deploy/laporan-jateng.service /etc/systemd/system/laporan-jateng.service
sudo systemctl daemon-reload
sudo systemctl enable --now laporan-jateng
sudo systemctl status laporan-jateng --no-pager
journalctl -u laporan-jateng -f
```

### 14.5 Nginx dan TLS

Buat DNS/subdomain terpisah untuk Polda Jateng, misalnya
`laporan-jateng.example.go.id`, lalu:

```bash
sudo cp ~/app_lap/deploy/nginx-laporan-jateng.conf /etc/nginx/sites-available/laporan-jateng
sudo ln -sf /etc/nginx/sites-available/laporan-jateng /etc/nginx/sites-enabled/laporan-jateng
sudo nano /etc/nginx/sites-available/laporan-jateng   # ganti server_name
sudo nginx -t
sudo systemctl reload nginx

sudo certbot --nginx -d laporan-jateng.example.go.id
```

Blok Nginx instance kedua sudah meneruskan header WebSocket (`Upgrade`/`Connection`)
untuk kolaborasi desain. Port `4318` tetap hanya listen di loopback dan tidak perlu
dibuka di firewall.

### 14.6 Verifikasi

```bash
ss -ltnp | grep -E '4317|4318'
curl -s http://127.0.0.1:4317/api/health
curl -s http://127.0.0.1:4318/api/health
curl -s http://127.0.0.1:4318/api/meta
```

Balasan `/api/meta` harus menampilkan `"polda_name":"POLDA JATENG"` dan
`"timezone_label":"WIB"`. Setelah itu buka domain Jateng, login, lalu buat periode
pertama dan impor workbook milik Polda Jateng.

### 14.7 Logo Polda Jateng

Logo web dan PDF sekarang dibaca saat runtime dari `LAPORAN_LOGO_PATH`. Letakkan logo
Jateng di data dir instance tersebut:

```bash
sudo cp logo-polda-jateng.png /var/lib/laporan-jateng/logo.png
sudo chown laporan:laporan /var/lib/laporan-jateng/logo.png
sudo systemctl restart laporan-jateng
```

Cukup letakkan berkas bernama `logo.png` di `LAPORAN_DATA_DIR`; tanpa env tambahan aplikasi
otomatis memakainya. Bila ingin path lain, isi:

```text
LAPORAN_LOGO_PATH=/home/laporan-data-jateng/logo-polda-jateng.png
```

Logo yang sama dipakai di halaman login, sidebar, favicon, pratinjau/PDF, dan template desain
baru. Format yang didukung: PNG, JPG, WebP, GIF, dan SVG. Setelah mengganti logo, terapkan
ulang template mingguan pada desain yang sudah ada agar elemen lama ikut diperbarui.

### 14.8 Backup instance Jateng

```bash
sudo mkdir -p /var/backups/laporan-jateng
sudo chown laporan:laporan /var/backups/laporan-jateng
sudo -u laporan sqlite3 /var/lib/laporan-jateng/laporan.sqlite3 \
  ".backup '/var/backups/laporan-jateng/db-$(date +%F).sqlite3'"
sudo -u laporan tar czf /var/backups/laporan-jateng/files-$(date +%F).tar.gz \
  --ignore-failed-read -C /var/lib/laporan-jateng imports reports design evidence
```

### 14.9 Akses langsung tanpa Nginx

Bila memang ingin membuka `http://IP_VPS:4318` tanpa reverse proxy, set:

```text
LAPORAN_BIND_ADDR=0.0.0.0
LAPORAN_ALLOWED_HOSTS=IP_VPS
LAPORAN_SECURE_COOKIE=false
```

lalu buka port `4318` di firewall. Cara ini tanpa TLS sehingga cookie sesi dikirim
terbuka; gunakan hanya di jaringan internal/VPN. Untuk akses internet tetap gunakan
Nginx + HTTPS seperti bagian 14.5.

### 14.10 Peta wilayah pada template

Peta pada template mingguan mengikuti `LAPORAN_MAP_REGION`:

```text
LAPORAN_MAP_REGION=bali    # siluet, label, dan titik tengah Pulau Bali
LAPORAN_MAP_REGION=jateng  # siluet, label, dan titik tengah Jawa Tengah
```

Setelah mengubah nilai ini, restart service lalu **terapkan ulang template mingguan**.
Gambar peta di desain lama tidak berubah otomatis karena gambar peta diunggah saat
template diterapkan. Tile OSM tetap diambil lewat proxy backend; bila tile gagal,
siluet wilayah yang dipilih akan digambar sebagai cadangan.
