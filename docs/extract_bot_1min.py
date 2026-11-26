import json
import os
import re
from datetime import datetime, timedelta, timezone

import openpyxl

SRC = "report-2026-11-11-to-2026-11-24.xlsx"
OUT = "data/tmp_bot_1min"
os.makedirs(OUT, exist_ok=True)

WIB = timezone(timedelta(hours=7))
GAP_MS = 60_000
MIN_RUN = 10


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
    # UUID v7: versi ada pada digit ke-13.
    if hexed[12] != "7":
        return None
    return ms


def wib_text(ms: int) -> str:
    stamp = datetime.fromtimestamp(ms / 1000, WIB)
    return stamp.strftime("%Y-%m-%d %H:%M:%S.") + f"{ms % 1000:03d}"


def seconds(ms: int) -> str:
    return f"{ms / 1000:.2f}".replace(".", ",")


wb = openpyxl.load_workbook(SRC, read_only=True, data_only=True)
ws = wb["Data"]

reports = []
by_uuid = {}
invalid_uuid = 0
duplicate_uuid = 0

for excel_row, row in enumerate(ws.iter_rows(min_row=2, values_only=True), start=2):
    if not row:
        continue
    uuid = norm(row[14] if len(row) > 14 else "")
    if not uuid:
        continue
    ms = uuid_ms(uuid)
    if ms is None:
        invalid_uuid += 1
        continue
    key = uuid.lower()
    kode = norm(row[7])
    kegiatan = norm(row[8])
    if key in by_uuid:
        duplicate_uuid += 1
        existing = by_uuid[key]
        if kode and kode not in existing["codes"]:
            existing["codes"].append(kode)
        if kegiatan and kegiatan not in existing["activities"]:
            existing["activities"].append(kegiatan)
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
        "activities": [kegiatan] if kegiatan else [],
        "quick_wins": norm(row[9]),
        "pelaksana": norm(row[10]),
        "pelapor": norm(row[11]),
        "isian": norm(row[16])[:400] if len(row) > 16 else "",
    }
    by_uuid[key] = report
    reports.append(report)

wb.close()

groups = {}
for report in reports:
    reporter = report["pelapor"].upper()
    polres = report["polres"].upper()
    groups.setdefault((reporter, polres), []).append(report)

runs = []
for (reporter_key, polres_key), items in groups.items():
    items.sort(key=lambda item: item["ms"])
    start = 0
    for index in range(1, len(items) + 1):
        if index < len(items) and items[index]["ms"] - items[index - 1]["ms"] <= GAP_MS:
            continue
        chunk = items[start:index]
        if len(chunk) >= MIN_RUN:
            gaps = [
                chunk[i]["ms"] - chunk[i - 1]["ms"] for i in range(1, len(chunk))
            ]
            units = sorted({item["satuan"] for item in chunk if item["satuan"]})
            polseks = sorted({item["polsek"] for item in chunk if item["polsek"]})
            codes = sorted({code for item in chunk for code in item["codes"] if code})
            runs.append(
                {
                    "reporter": chunk[0]["pelapor"],
                    "reporter_key": reporter_key,
                    "polres": chunk[0]["polres"],
                    "polres_key": polres_key,
                    "tingkat": chunk[0]["tingkat"],
                    "units": units,
                    "polseks": polseks,
                    "codes": codes,
                    "count": len(chunk),
                    "start_ms": chunk[0]["ms"],
                    "end_ms": chunk[-1]["ms"],
                    "start_wib": chunk[0]["wib"],
                    "end_wib": chunk[-1]["wib"],
                    "duration_s": (chunk[-1]["ms"] - chunk[0]["ms"]) / 1000,
                    "gap_min_s": min(gaps) / 1000 if gaps else 0,
                    "gap_median_s": sorted(gaps)[len(gaps) // 2] / 1000 if gaps else 0,
                    "gap_max_s": max(gaps) / 1000 if gaps else 0,
                    "reports": [
                        {
                            "seq": pos + 1,
                            "gap_s": (
                                (item["ms"] - chunk[pos - 1]["ms"]) / 1000
                                if pos
                                else 0
                            ),
                            "uuid": item["uuid"],
                            "wib": item["wib"],
                            "excel_row": item["excel_row"],
                            "tanggal_kegiatan": item["tanggal_kegiatan"],
                            "tanggal_lapor": item["tanggal_lapor"],
                            "satuan": item["satuan"],
                            "polres": item["polres"],
                            "polsek": item["polsek"],
                            "kode": item["kode"],
                            "codes": item["codes"],
                            "kegiatan": item["kegiatan"],
                            "activities": item["activities"],
                            "quick_wins": item["quick_wins"],
                            "pelaksana": item["pelaksana"],
                            "isian": item["isian"],
                        }
                        for pos, item in enumerate(chunk)
                    ],
                }
            )
        start = index

runs.sort(key=lambda run: (run["start_ms"], run["reporter_key"]))

with open(os.path.join(OUT, "reports.json"), "w", encoding="utf-8") as handle:
    json.dump(reports, handle, ensure_ascii=False, separators=(",", ":"))
with open(os.path.join(OUT, "runs.json"), "w", encoding="utf-8") as handle:
    json.dump(runs, handle, ensure_ascii=False, separators=(",", ":"))

polres_runs = {}
for run in runs:
    polres_runs[run["polres"]] = polres_runs.get(run["polres"], 0) + 1

print("laporan unik v7 :", len(reports))
print("uuid tidak valid:", invalid_uuid)
print("uuid duplikat   :", duplicate_uuid)
print("grup pelapor    :", len(groups))
print("rangkaian >=10  :", len(runs))
print("laporan dalam rangkaian:", sum(run["count"] for run in runs))
print("pelapor terindikasi:", len({run["reporter_key"] for run in runs}))
print("polres terindikasi :", len(polres_runs))
print("polsek terlibat    :", len({p for run in runs for p in run["polseks"]}))
print("\nTop polres:")
for label, count in sorted(polres_runs.items(), key=lambda item: -item[1])[:12]:
    reports_count = sum(
        run["count"] for run in runs if run["polres"] == label
    )
    print(f"  {label}: {count} rangkaian, {reports_count} laporan")
print("\nContoh rangkaian terbesar:")
for run in sorted(runs, key=lambda run: -run["count"])[:8]:
    print(
        f"  {run['reporter']} | {', '.join(run['units'])} | {run['count']} laporan | "
        f"{run['start_wib']} -> {run['end_wib']} | jeda maks {run['gap_max_s']:.2f}s | "
        f"{','.join(run['codes'])}"
    )
