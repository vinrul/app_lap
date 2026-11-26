import json
import re
from collections import Counter, defaultdict

with open("data/tmp_dup_bali/reports.json", encoding="utf-8") as handle:
    reports = json.load(handle)


def show(label, items, limit=5):
    print(f"\n=== {label}: {len(items)} ===")
    for item in items[:limit]:
        print(" ", item)


# 1. Same unit + reporter + code, gap <= 10 minutes, any text.
by_unit_code = defaultdict(list)
for report in reports:
    by_unit_code[(report["satuan"].lower(), report["pelapor"].lower(), report["kode"])].append(report)
pairs = []
for key, items in by_unit_code.items():
    items.sort(key=lambda item: item["ms"])
    for i in range(1, len(items)):
        gap = (items[i]["ms"] - items[i - 1]["ms"]) / 1000
        if gap <= 600:
            same_text = items[i]["norm"] == items[i - 1]["norm"]
            pairs.append((gap, same_text, items[i - 1], items[i]))
pairs.sort(key=lambda item: item[0])
print("same unit+pelapor+kode gaps<=10m:", len(pairs), "same text:", sum(1 for p in pairs if p[1]))
for gap, same, a, b in pairs[:15]:
    print(
        f"  {gap:.1f}s sameText={same} | {a['satuan']} | {a['pelapor']} | {a['kode']} | "
        f"{a['wib']} -> {b['wib']} | {a['kegiatan'][:50]}"
    )
    print("    A:", a["isian"][:180].replace("\n", " "))
    print("    B:", b["isian"][:180].replace("\n", " "))

# 2. Same text exact (any unit), non-generic, length >= 80.
by_text = defaultdict(list)
for report in reports:
    if len(report["norm"]) >= 80:
        by_text[report["norm"]].append(report)
dups = sorted(by_text.values(), key=lambda items: -len(items))
print("\nexact text groups with >=2:", sum(1 for items in dups if len(items) >= 2))
for items in dups[:12]:
    units = sorted({item["satuan"] for item in items})
    codes = sorted({item["kode"] for item in items})
    dates = sorted({item["tanggal_kegiatan"] for item in items})
    print(
        f"  n={len(items)} units={len(units)} codes={codes[:5]} dates={dates[:3]} "
        f"len={len(items[0]['norm'])}"
    )
    print("   ", items[0]["isian"][:220].replace("\n", " "))

# 3. Same unit+pelapor+text, different code.
by_unit_text = defaultdict(list)
for report in reports:
    if len(report["norm"]) >= 60:
        by_unit_text[(report["satuan"].lower(), report["pelapor"].lower(), report["norm"])].append(report)
cross = []
for key, items in by_unit_text.items():
    codes = sorted({item["kode"] for item in items})
    if len(codes) >= 2:
        cross.append((len(items), codes, items))
cross.sort(key=lambda item: -item[0])
print("\nsame unit+pelapor text, different code:", len(cross))
for count, codes, items in cross[:10]:
    print(f"  n={count} codes={codes} unit={items[0]['satuan']} pelapor={items[0]['pelapor']} dates={sorted({i['tanggal_kegiatan'] for i in items})}")
    print("   ", items[0]["isian"][:220].replace("\n", " "))

# 4. QW5.3 samples.
qw53 = [r for r in reports if r["kode"].upper().startswith("QW5.3")]
print("\nQW5.3 total:", len(qw53))
for report in qw53[:8]:
    print(
        f"  {report['wib']} | {report['satuan']} | {report['pelapor']} | "
        f"{report['kegiatan'][:60]}"
    )
    print("   ", report["isian"][:240].replace("\n", " "))

# 5. Count exact repeated text per (unit,pelapor,kode).
by_uct = defaultdict(list)
for report in reports:
    if len(report["norm"]) >= 60 and report["norm"] not in {"nihil", "tidak ada"}:
        by_uct[(report["satuan"].lower(), report["pelapor"].lower(), report["kode"], report["norm"])].append(report)
repeat = [items for items in by_uct.values() if len(items) >= 2]
repeat.sort(key=lambda items: -len(items))
print("\nexact same text group per unit+pelapor+kode:", len(repeat))
for items in repeat[:10]:
    print(
        f"  n={len(items)} unit={items[0]['satuan']} pelapor={items[0]['pelapor']} "
        f"kode={items[0]['kode']} dates={sorted({i['tanggal_kegiatan'] for i in items})}"
    )
    print("   ", items[0]["isian"][:220].replace("\n", " "))
