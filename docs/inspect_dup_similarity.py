import json
import re
from collections import defaultdict
from difflib import SequenceMatcher

with open("data/tmp_dup_bali/reports.json", encoding="utf-8") as handle:
    reports = json.load(handle)

GENERIC_VALUES = {
    "ya",
    "tidak",
    "tidak ada",
    "nihil",
    "0",
    "-",
    "ada",
    "tidak menangani penyidikan perkara",
    "tidak ada kawasan rawan narkoba",
    "tidak ada layanan call center 110",
}


def content(text: str) -> str:
    values = []
    for line in str(text or "").splitlines():
        if ":" in line:
            value = line.split(":", 1)[1].strip()
        else:
            value = line.strip()
        if len(value) < 15:
            continue
        if value.lower() in GENERIC_VALUES:
            continue
        values.append(value)
    joined = " ".join(values) if values else str(text or "")
    joined = re.sub(r"[^a-z0-9]+", " ", joined.lower())
    return re.sub(r"\s+", " ", joined).strip()


def ratio(a: str, b: str) -> float:
    return SequenceMatcher(None, a, b).ratio()


# Pairs same unit/pelapor/kode within 10 minutes
by_unit_code = defaultdict(list)
for report in reports:
    by_unit_code[(report["satuan"].lower(), report["pelapor"].lower(), report["kode"])].append(report)

pairs = []
for key, items in by_unit_code.items():
    items.sort(key=lambda item: item["ms"])
    for i in range(1, len(items)):
        gap = (items[i]["ms"] - items[i - 1]["ms"]) / 1000
        if gap <= 600:
            pairs.append((gap, items[i - 1], items[i]))
pairs.sort(key=lambda item: item[0])

print("same unit+pelapor+kode pairs <=10m:", len(pairs))
scored = []
for gap, a, b in pairs:
    scored.append((ratio(a["norm"], b["norm"]), ratio(content(a["isian"]), content(b["isian"])), gap, a, b))
scored.sort(key=lambda item: (-item[1], -item[0]))
print("top similarity same QW:")
for full, core, gap, a, b in scored[:15]:
    print(
        f"  gap={gap:.1f}s full={full:.3f} content={core:.3f} | {a['satuan']} | {a['pelapor']} | {a['kode']}"
    )
    print("    A", a["isian"][:260].replace("\n", " | "))
    print("    B", b["isian"][:260].replace("\n", " | "))

# Cross-QW pairs same unit+pelapor/date
by_unit_date = defaultdict(list)
for report in reports:
    by_unit_date[(report["satuan"].lower(), report["pelapor"].lower(), report["tanggal_kegiatan"])].append(report)

cross = []
for key, items in by_unit_date.items():
    for i in range(len(items)):
        for j in range(i + 1, len(items)):
            a, b = items[i], items[j]
            if a["kode"] == b["kode"]:
                continue
            ca, cb = content(a["isian"]), content(b["isian"])
            if len(ca) < 40 or len(cb) < 40:
                continue
            core = ratio(ca, cb)
            if core >= 0.55:
                cross.append((core, ratio(a["norm"], b["norm"]), a, b))
cross.sort(key=lambda item: -item[0])
print("\ncross-QW candidates same unit+pelapor+date (content>=0.55):", len(cross))
for core, full, a, b in cross[:20]:
    print(
        f"  content={core:.3f} full={full:.3f} | {a['satuan']} | {a['pelapor']} | "
        f"{a['tanggal_kegiatan']} | {a['kode']} vs {b['kode']}"
    )
    print("    A", a["isian"][:260].replace("\n", " | "))
    print("    B", b["isian"][:260].replace("\n", " | "))
