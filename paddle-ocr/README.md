# Sidecar PaddleOCR

Layanan Python kecil yang membaca teks pada gambar bukti dukung. Backend Rust
memakainya sebagai cadangan: Tesseract tetap dijalankan lebih dulu karena jauh
lebih cepat, dan PaddleOCR hanya dipanggil bila Tesseract tidak menemukan
tanggal + jam + alamat, atau tanggal yang ditemukannya berbeda dari tanggal
laporan (kemungkinan salah baca). Gambar yang terdeteksi sebagai dokumen atau
tangkapan layar dilewati karena tidak akan pernah lolos validasi.

## Menjalankan di Windows (pengembangan)

```powershell
py -3.13 -m venv paddle-ocr\.venv
paddle-ocr\.venv\Scripts\pip install -r paddle-ocr\requirements.txt
$env:PADDLE_PDX_ENABLE_MKLDNN_BYDEFAULT = "False"
paddle-ocr\.venv\Scripts\uvicorn app:app --host 127.0.0.1 --port 4320 --workers 1
```

Jalankan dari direktori `paddle-ocr`, lalu set `LAPORAN_PADDLE_URL=http://127.0.0.1:4320`
di `.env` proyek.

Di mesin pengembangan ini venv pilot `ocr/_paddle_venv` sudah berisi
PaddleOCR 3.7.0 dan modelnya, jadi bisa dipakai untuk uji cepat tanpa membuat
venv baru.

## Menjalankan di VPS Linux

Lihat bagian "PaddleOCR (opsional)" di `deploy/DEPLOY.md`.

## Endpoint

- `GET /health` — status model, versi, dan pesan kesalahan bila gagal dimuat.
- `POST /ocr` — `{"path": "..."}` atau `{"image_base64": "..."}`.

Balasan `POST /ocr`:

```json
{
  "text": "Senin, 02 November 2026 07:13\nKp. Toyapakeh, Kec. Nusa Penida",
  "lines": [{"text": "Senin, 02 November 2026 07:13", "score": 0.98}],
  "seconds": 4.21,
  "engine": "PaddleOCR",
  "version": "3.7.0"
}
```

## Variabel lingkungan

| Variabel | Default | Keterangan |
|---|---|---|
| `PADDLE_PDX_ENABLE_MKLDNN_BYDEFAULT` | `False` | Wajib `False`; Paddle 3.3.1 gagal di Windows bila MKLDNN aktif |
| `LAPORAN_PADDLE_LANG` | `id` | Bahasa model PaddleOCR |
| `LAPORAN_PADDLE_MAX_SIDE` | `1920` | Sisi terpanjang gambar setelah diperkecil; `0` berarti tanpa perkecilan |
| `LAPORAN_PADDLE_MIN_SCORE` | `0` | Buang baris dengan skor di bawah nilai ini; `0` berarti semua dipakai |
| `LAPORAN_PADDLE_READY_TIMEOUT` | `180` | Detik menunggu model selesai dimuat |
| `LAPORAN_PADDLE_BIND` | `127.0.0.1` | Alamat bind saat dijalankan lewat `python app.py` |
| `LAPORAN_PADDLE_PORT` | `4320` | Port saat dijalankan lewat `python app.py` |
