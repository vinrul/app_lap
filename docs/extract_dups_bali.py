import json
import os
import re
from datetime import datetime, timedelta, timezone

import openpyxl

SRC = "report-2026-11-11-to-2026-11-24-bali.xlsx"
OUT = "data/tmp_dup_bali"
os.makedirs(OUT, exist_ok=True)

WIB = timezone(timedelta(hours=7))
SAME_QW_WINDOW = 10 * 60  # detik
CROSS_QW_WINDOW = 24 * 3600  # detik
MIN_TEXT = 60


def norm(value) -> str:
    return re.sub(r"\s+", " ", str(value or "")).strip()


def uuid_ms(value: str):
    hexed = value.replace("-", "").strip().lower()
    if len(hexed) < 32:
        return None
    try:
        ms = int(hexed[:12], 16)
    except ValueError:
        return None
    if hexed[12] != "7":
        return None
    return ms


def wib_text(ms: int) -> str:
    stamp = datetime.fromtimestamp(ms / 1000, WIB)
    return stamp.strftime("%Y-%m-%d %H:%M:%S.") + f"{ms % 1000:03d}"


def normalize_text(value: str) -> str:
    text = str(value or "").lower()
    text = re.sub(r"[^a-z0-9]+", " ", text)
    return re.sub(r"\s+", " ", text).strip()


GENERIC = {
    "nihil",
    "tidak ada",
    "tidak",
    "0",
    "none",
    "tidak ada kegiatan",
    "tidak ada laporan",
    "tidak ada temuan",
}


wb = openpyxl.load_workbook(SRC, read_only=True, data_only=True)
ws = wb["Data"]

reports = []
by_uuid = {}
duplicates = 0
for excel_row, row in enumerate(ws.iter_rows(min_row=2, values_only=True), start=2):
    if not row:
        continue
    uuid = norm(row[14] if len(row) > 14 else "")
    if not uuid:
        continue
    ms = uuid_ms(uuid)
    if ms is None:
        continue
    key = uuid.lower()
    kode = norm(row[7])
    isian = norm(row[16]) if len(row) > 16 else ""
    kegiatan = norm(row[8])
    if key in by_uuid:
        duplicates += 1
        existing = by_uuid[key]
        if kode and kode not in existing["codes"]:
            existing["codes"].append(kode)
        if isian and isian not in existing["isian_variants"]:
            existing["isian_variants"].append(isian)
        continue
    report = {
        "uuid": uuid,
        "ms": ms,
        "wib": wib_text(ms),
        "excel_row": excel_row,
        "tanggal_kegiatan": norm(row[0]),
        "tanggal_lapor": norm(row[1]),
        "satuan": norm(row[2]),
        "tingkat": norm(row[3]),
        "polda": norm(row[4]),
        "polres": norm(row[5]),
        "polsek": norm(row[6]),
        "kode": kode,
        "codes": [kode] if kode else [],
        "kegiatan": kegiatan,
        "quick_wins": norm(row[9]),
        "pelaksana": norm(row[10]),
        "pelapor": norm(row[11]),
        "isian": isian,
        "isian_variants": [isian] if isian else [],
        "norm": normalize_text(isian),
    }
    by_uuid[key] = report
    reports.append(report)

wb.close()

for report in reports:
    # Pakai varian isian terpanjang agar tidak kehilangan narasi utama.
    variants = [item for item in report["isian_variants"] if item]
    if variants:
        report["isian"] = max(variants, key=len)
    report["norm"] = normalize_text(report["isian"])

print("laporan unik:", len(reports), "duplikat uuid:", duplicates)
lengths = sorted(len(report["norm"]) for report in reports)
print(
    "panjang teks min/median/p90/max:",
    lengths[0],
    lengths[len(lengths) // 2],
    lengths[int(len(lengths) * 0.9)],
    lengths[-1],
)

# ---------------------------------------------------------------------------
# Group A: unit + pelapor + kode + teks sama, jeda <=10 menit.
# Group B: unit + pelapor + tanggal kegiatan sama, kode berbeda, teks sama.
# Group C: teks + tanggal kegiatan sama, unit berbeda.
# ---------------------------------------------------------------------------
groups = []

by_unit_code_text = {}
by_unit_text_date = {}
by_text_date = {}
for report in reports:
    text = report["norm"]
    if len(text) < MIN_TEXT or text in GENERIC:
        continue
    satuan = report["satuan"].lower()
    pelapor = report["pelapor"].lower()
    by_unit_code_text.setdefault((satuan, pelapor, report["kode"], text), []).append(report)
    by_unit_text_date.setdefault((satuan, pelapor, text, report["tanggal_kegiatan"]), []).append(report)
    by_text_date.setdefault((text, report["tanggal_kegiatan"]), []).append(report)

for (satuan, pelapor, kode, text), items in by_unit_code_text.items():
    items.sort(key=lambda item: item["ms"])
    start = 0
    for index in range(1, len(items) + 1):
        if index < len(items) and items[index]["ms"] - items[index - 1]["ms"] <= SAME_QW_WINDOW * 1000:
            continue
        chunk = items[start:index]
        if len(chunk) >= 2:
            gaps = [chunk[i]["ms"] - chunk[i - 1]["ms"] for i in range(1, len(chunk))]
            groups.append(
                {
                    "type": "same_qw",
                    "satuan": chunk[0]["satuan"],
                    "polres": chunk[0]["polres"],
                    "pelapor": chunk[0]["pelapor"],
                    "tanggal": chunk[0]["tanggal_kegiatan"],
                    "codes": [kode],
                    "gap_s": min(gaps) / 1000,
                    "gap_max_s": max(gaps) / 1000,
                    "count": len(chunk),
                    "reports": chunk,
                }
            )
        start = index

for (satuan, pelapor, text, tanggal), items in by_unit_text_date.items():
    codes = sorted({item["kode"] for item in items if item["kode"]})
    if len(codes) < 2:
        continue
    items.sort(key=lambda item: item["ms"])
    groups.append(
        {
            "type": "cross_qw",
            "satuan": items[0]["satuan"],
            "polres": items[0]["polres"],
            "pelapor": items[0]["pelapor"],
            "tanggal": tanggal,
            "codes": codes,
            "gap_s": (items[-1]["ms"] - items[0]["ms"]) / 1000,
            "gap_max_s": max(
                (items[i]["ms"] - items[i - 1]["ms"]) / 1000 for i in range(1, len(items))
            ),
            "count": len(items),
            "reports": items,
        }
    )

for (text, tanggal), items in by_text_date.items():
    units = sorted({item["satuan"] for item in items if item["satuan"]})
    if len(units) < 2:
        continue
    items.sort(key=lambda item: item["ms"])
    groups.append(
        {
            "type": "cross_unit",
            "satuan": " / ".join(units),
            "polres": " / ".join(sorted({item["polres"] for item in items if item["polres"]})),
            "pelapor": " / ".join(sorted({item["pelapor"] for item in items if item["pelapor"]})),
            "tanggal": tanggal,
            "codes": sorted({item["kode"] for item in items if item["kode"]}),
            "gap_s": (items[-1]["ms"] - items[0]["ms"]) / 1000,
            "gap_max_s": max(
                (items[i]["ms"] - items[i - 1]["ms"]) / 1000 for i in range(1, len(items))
            ),
            "count": len(items),
            "reports": items,
        }
    )

# Serialisasi ringkas.
for group in groups:
    group["reports"] = [
        {
            "uuid": item["uuid"],
            "wib": item["wib"],
            "excel_row": item["excel_row"],
            "tanggal_kegiatan": item["tanggal_kegiatan"],
            "satuan": item["satuan"],
            "polres": item["polres"],
            "polsek": item["polsek"],
            "kode": item["kode"],
            "codes": item["codes"],
            "kegiatan": item["kegiatan"],
            "pelapor": item["pelapor"],
            "isian": item["isian"],
        }
        for item in group["reports"]
    ]

with open(os.path.join(OUT, "reports.json"), "w", encoding="utf-8") as handle:
    json.dump(
        [
            {
                "uuid": report["uuid"],
                "ms": report["ms"],
                "wib": report["wib"],
                "excel_row": report["excel_row"],
                "tanggal_kegiatan": report["tanggal_kegiatan"],
                "tanggal_lapor": report["tanggal_lapor"],
                "satuan": report["satuan"],
                "tingkat": report["tingkat"],
                "polda": report["polda"],
                "polres": report["polres"],
                "polsek": report["polsek"],
                "kode": report["kode"],
                "codes": report["codes"],
                "kegiatan": report["kegiatan"],
                "quick_wins": report["quick_wins"],
                "pelaksana": report["pelaksana"],
                "pelapor": report["pelapor"],
                "isian": report["isian"],
                "norm": report["norm"],
            }
            for report in reports
        ],
        handle,
        ensure_ascii=False,
        separators=(",", ":"),
    )
with open(os.path.join(OUT, "groups.json"), "w", encoding="utf-8") as handle:
    json.dump(groups, handle, ensure_ascii=False, separators=(",", ":"))

by_type = {}
for group in groups:
    stat = by_type.setdefault(group["type"], {"groups": 0, "reports": 0})
    stat["groups"] += 1
    stat["reports"] += group["count"]
print("grup:", by_type)

# Ringkasan per satuan/polres untuk grup A+B.
unit_reports = {}
polres_reports = {}
for group in groups:
    if group["type"] == "cross_unit":
        continue
    for report in group["reports"]:
        unit_reports[report["satuan"]] = unit_reports.get(report["satuan"], 0) + 1
        polres_reports[report["polres"]] = polres_reports.get(report["polres"], 0) + 1
print("unit terlibat:", len(unit_reports))
for label, count in sorted(unit_reports.items(), key=lambda item: -item[1])[:12]:
    print(f"  {label}: {count}")
print("polres terlibat:", len(polres_reports))
for label, count in sorted(polres_reports.items(), key=lambda item: -item[1])[:10]:
    print(f"  {label}: {count}")

print("\nContoh same_qw (3):")
for group in [g for g in groups if g["type"] == "same_qw"][:3]:
    print(
        f"  {group['satuan']} | {group['pelapor']} | {group['tanggal']} | "
        f"{group['codes']} | {group['count']} laporan | gap {group['gap_s']:.1f}-{group['gap_max_s']:.1f}s"
    )
    print("   ", group["reports"][0]["isian"][:220].replace("\n", " "))

print("\nContoh cross_qw (5):")
for group in [g for g in groups if g["type"] == "cross_qw"][:5]:
    print(
        f"  {group['satuan']} | {group['pelapor']} | {group['tanggal']} | "
        f"{','.join(group['codes'])} | {group['count']} laporan"
    )
    print("   ", group["reports"][0]["isian"][:220].replace("\n", " "))

print("\nContoh cross_unit (3):")
for group in [g for g in groups if g["type"] == "cross_unit"][:3]:
    print(
        f"  {group['satuan']} | {group['tanggal']} | {','.join(group['codes'])} | "
        f"{group['count']} laporan"
    )
    print("   ", group["reports"][0]["isian"][:220].replace("\n", " "))
