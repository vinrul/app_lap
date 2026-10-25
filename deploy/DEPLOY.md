# Deploy ke VPS (Ubuntu/Debian)

Arsitektur: browser → Nginx (TLS) → backend Rust `127.0.0.1:4317` → SQLite + Chrome headless.
Backend secara default hanya listen di loopback, jadi konfigurasi yang direkomendasikan adalah
memakai reverse proxy di mesin yang sama.

Panduan ini diasumsikan untuk Ubuntu 22.04/24.04 x86_64. Untuk distro lain, sesuaikan nama paket.

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
- `LAPORAN_GOOGLE_DRIVE_API_KEY` — opsional, wajib jika workbook memakai tautan folder Google Drive publik. Isi API key asli di `/etc/laporan/laporan.env`; jangan menyimpannya di file contoh atau repository.

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

Migrasi baru dijalankan otomatis saat startup. Cek `journalctl -u laporan -n 50` bila gagal.

## 12. Memindahkan data lama dari Windows

Menyalin `data/` dari Windows bisa dilakukan, tetapi `report_runs.pdf_path`/`html_path` dan
`imports.stored_path` menyimpan path absolut Windows. Akibatnya PDF lama tidak dapat diunduh di
Linux dan path arsip menjadi tidak valid sampai diperbaiki. Untuk deploy baru, mulai dengan data
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
