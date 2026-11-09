"""Sidecar PaddleOCR untuk analisis dokumentasi bukti dukung.

Backend Rust memanggil `POST /ocr` hanya ketika Tesseract tidak menemukan
tanggal, jam, dan alamat sekaligus. Model dimuat sekali saat proses mulai agar
setiap permintaan tidak membayar biaya inisialisasi.

Jalankan dengan uvicorn:

    PADDLE_PDX_ENABLE_MKLDNN_BYDEFAULT=False \
        uvicorn app:app --host 127.0.0.1 --port 4320 --workers 1
"""

import base64
import io
import logging
import os
import threading
import time
from contextlib import asynccontextmanager

# Harus diatur sebelum paddleocr diimpor. Tanpa ini PaddlePaddle 3.3.1 di
# Windows gagal dengan "ConvertPirAttribute2RuntimeAttribute not support".
os.environ.setdefault("PADDLE_PDX_ENABLE_MKLDNN_BYDEFAULT", "False")

import numpy as np
from fastapi import FastAPI, HTTPException
from PIL import Image, ImageOps
from pydantic import BaseModel

log = logging.getLogger("laporan.paddle_ocr")

MODEL_LANG = os.environ.get("LAPORAN_PADDLE_LANG", "id")
# Sisi terpanjang gambar setelah diperkecil. Watermark biasanya masih terbaca
# pada 1920 px dan biaya deteksi turun jauh dibanding resolusi asli 4000 px.
MAX_SIDE = int(os.environ.get("LAPORAN_PADDLE_MAX_SIDE", "1920"))
# Nilai 0 berarti semua baris dipakai. Naikkan bila ingin membuang baris
# dengan keyakinan sangat rendah.
MIN_SCORE = float(os.environ.get("LAPORAN_PADDLE_MIN_SCORE", "0"))
# Batas menunggu model selesai dimuat sebelum permintaan pertama dilayani.
READY_TIMEOUT_SECONDS = float(os.environ.get("LAPORAN_PADDLE_READY_TIMEOUT", "180"))

_engine = None
_engine_error = None
_engine_ready = threading.Event()
_engine_lock = threading.Lock()

try:
    import paddleocr

    PADDLEOCR_VERSION = paddleocr.__version__
except Exception:  # pragma: no cover - hanya untuk laporan /health
    PADDLEOCR_VERSION = "tidak diketahui"


class OcrRequest(BaseModel):
    path: str | None = None
    image_base64: str | None = None


class OcrLine(BaseModel):
    text: str
    score: float


class OcrResponse(BaseModel):
    text: str
    lines: list[OcrLine]
    seconds: float
    engine: str
    version: str


def load_engine() -> None:
    """Memuat model PaddleOCR sekali di latar belakang."""
    global _engine, _engine_error
    started = time.perf_counter()
    try:
        from paddleocr import PaddleOCR

        _engine = PaddleOCR(
            lang=MODEL_LANG,
            use_doc_orientation_classify=False,
            use_doc_unwarping=False,
            use_textline_orientation=True,
            text_det_limit_side_len=MAX_SIDE,
            text_det_limit_type="max",
        )
        log.info(
            "model PaddleOCR siap: lang=%s max_side=%d dalam %.1f detik",
            MODEL_LANG,
            MAX_SIDE,
            time.perf_counter() - started,
        )
    except Exception as error:  # noqa: BLE001 - alasan harus terlihat di log
        _engine_error = f"{type(error).__name__}: {error}"
        log.exception("model PaddleOCR gagal dimuat")
    finally:
        _engine_ready.set()


@asynccontextmanager
async def lifespan(_app: FastAPI):
    threading.Thread(target=load_engine, name="paddle-load", daemon=True).start()
    yield


app = FastAPI(title="Laporan Mingguan PaddleOCR", lifespan=lifespan)


def load_image(request: OcrRequest) -> np.ndarray:
    if request.image_base64:
        try:
            raw = base64.b64decode(request.image_base64, validate=True)
        except Exception as error:  # noqa: BLE001
            raise HTTPException(400, f"image_base64 tidak valid: {error}") from error
        source = io.BytesIO(raw)
    elif request.path:
        if not os.path.isfile(request.path):
            raise HTTPException(400, f"berkas tidak ditemukan: {request.path}")
        source = request.path
    else:
        raise HTTPException(400, "isi path atau image_base64")

    try:
        image = Image.open(source)
        image = ImageOps.exif_transpose(image).convert("RGB")
    except Exception as error:  # noqa: BLE001
        raise HTTPException(400, f"gambar tidak dapat dibaca: {error}") from error

    if MAX_SIDE > 0 and max(image.size) > MAX_SIDE:
        image.thumbnail((MAX_SIDE, MAX_SIDE), Image.LANCZOS)
    return np.ascontiguousarray(np.asarray(image))


@app.get("/health")
def health() -> dict:
    if _engine is not None:
        status = "ok"
    elif not _engine_ready.is_set():
        status = "loading"
    else:
        status = "error"
    return {
        "status": status,
        "engine": "PaddleOCR",
        "version": PADDLEOCR_VERSION,
        "lang": MODEL_LANG,
        "max_side": MAX_SIDE,
        "error": _engine_error,
    }


@app.post("/ocr", response_model=OcrResponse)
def ocr(request: OcrRequest) -> OcrResponse:
    if not _engine_ready.wait(timeout=READY_TIMEOUT_SECONDS):
        raise HTTPException(503, "model PaddleOCR masih dimuat")
    if _engine is None:
        raise HTTPException(503, f"model PaddleOCR tidak siap: {_engine_error}")

    image = load_image(request)
    started = time.perf_counter()
    try:
        # Satu model dipakai bergantian; Paddle tidak dijamin aman dipanggil
        # paralel dari beberapa thread.
        with _engine_lock:
            predictions = _engine.predict(image)
    except Exception as error:  # noqa: BLE001 - diteruskan ke backend
        log.exception("PaddleOCR gagal membaca gambar (%s)", request.path or "base64")
        raise HTTPException(500, f"PaddleOCR gagal: {type(error).__name__}: {error}") from error

    lines: list[OcrLine] = []
    for prediction in predictions:
        texts = prediction.get("rec_texts", []) or []
        scores = prediction.get("rec_scores", []) or []
        for index, text in enumerate(texts):
            text = str(text).strip()
            if not text:
                continue
            score = float(scores[index]) if index < len(scores) else 0.0
            if MIN_SCORE > 0 and score < MIN_SCORE:
                continue
            lines.append(OcrLine(text=text, score=score))

    seconds = time.perf_counter() - started
    log.debug(
        "OCR %s: %d baris dalam %.2f detik",
        request.path or "base64",
        len(lines),
        seconds,
    )
    return OcrResponse(
        text="\n".join(line.text for line in lines),
        lines=lines,
        seconds=round(seconds, 3),
        engine="PaddleOCR",
        version=PADDLEOCR_VERSION,
    )


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(
        app,
        host=os.environ.get("LAPORAN_PADDLE_BIND", "127.0.0.1"),
        port=int(os.environ.get("LAPORAN_PADDLE_PORT", "4320")),
    )
