import json
import re
import statistics
import uuid
from collections import Counter, defaultdict
from datetime import datetime, timedelta, timezone

META = r"F:\app_lap\data\tmp_bot_nov\all_meta.json"
RAW = r"F:\app_lap\data\tmp_bot_nov\rows6.json"
OUT = r"F:\app_lap\data\tmp_bot_nov\uuid_results.json"
FOCUS = [
    "Polres Temanggung",
    "Polres Rembang",
    "Polres Karanganyar",
    "Polres Purbalingga",
    "Polresta Surakarta",
    "Polresta Batang",
]
WIB = timezone(timedelta(hours=7))


def norm_name(raw):
    text = (raw or "").split(",")[0]
    text = re.sub(r"[^A-Za-z ]+", " ", text)
    return " ".join(text.upper().split())


def uuid_ms(value):
    try:
        parsed = uuid.UUID(value)
    except (ValueError, AttributeError):
        return None
    if parsed.version != 7:
        return None
    return parsed.int >> 80


def stamp(ms):
    return datetime.fromtimestamp(ms / 1000, WIB).strftime("%d %b %Y %H:%M:%S.%f")[:-3] + " WIB"


def short_time(ms):
    return datetime.fromtimestamp(ms / 1000, WIB).strftime("%d %b %H:%M:%S.%f")[:-3]


raw_rows = json.load(open(RAW, encoding="utf-8"))
text_by_row = {item["source_row"]: item for item in raw_rows}
rows = json.load(open(META, encoding="utf-8"))

reports = {}
for item in rows:
    rid = (item.get("report_id") or "").strip()
    if not rid:
        continue
    ms = uuid_ms(rid)
    if ms is None:
        continue
    record = reports.get(rid)
    if record is None:
        record = {
            "id": rid,
            "ms": ms,
            "polres": item.get("polres") or "",
            "satker": item.get("satuan") or "",
            "pelapor_raw": item.get("pelapor") or "",
            "rows": [],
            "kodes": set(),
            "time": datetime.fromtimestamp(ms / 1000, WIB),
            "date": datetime.fromtimestamp(ms / 1000, WIB).strftime("%Y-%m-%d"),
        }
        reports[rid] = record
    record["rows"].append(item)
    record["kodes"].add(item.get("kode") or "")

for record in reports.values():
    raw_names = [row.get("pelapor") or "" for row in record["rows"] if (row.get("pelapor") or "").strip()]
    record["pelapor_raw"] = Counter(raw_names).most_common(1)[0][0] if raw_names else ""
    record["reporter"] = norm_name(record["pelapor_raw"])
    record["key"] = (record["reporter"], record["polres"])
    satkers = [row.get("satuan") or "" for row in record["rows"] if (row.get("satuan") or "").strip()]
    record["satker"] = Counter(satkers).most_common(1)[0][0] if satkers else record["satker"]

# Duplicate detection: satker + kode + isian on the same day, or same isian across days (>=30 chars).
seen_same = set()
first_date = {}
for record in sorted(reports.values(), key=lambda item: (item["ms"], item["id"])):
    flags = []
    for row in record["rows"]:
        digest = row.get("isian_hash") or ""
        satker = row.get("satuan") or ""
        kode = row.get("kode") or ""
        length = row.get("isian_len") or 0
        date = record["date"]
        if not digest:
            flags.append(False)
            continue
        same_key = (satker, kode, digest, date)
        cross_key = (satker, digest)
        same_dup = same_key in seen_same
        cross_dup = length >= 30 and cross_key in first_date and first_date[cross_key] != date
        flags.append(same_dup or cross_dup)
        seen_same.add(same_key)
        first_date.setdefault(cross_key, date)
    record["dup_rows"] = sum(1 for flag in flags if flag)
    record["all_dup"] = bool(flags) and all(flags)
    record["any_dup"] = any(flags)

by_reporter = defaultdict(list)
for record in reports.values():
    by_reporter[record["key"]].append(record)
for items in by_reporter.values():
    items.sort(key=lambda item: (item["ms"], item["id"]))

runs = []
all_gaps = []
sub30_records = []
for key, items in by_reporter.items():
    previous = None
    for record in items:
        record["gap"] = None
        record["bot"] = False
        record["boundary"] = False
        record["run_flag"] = False
        if previous is not None:
            gap = (record["ms"] - previous["ms"]) / 1000
            record["gap"] = gap
            record["boundary"] = 30 <= gap <= 33
            record["bot"] = gap < 3 and not record["all_dup"]
            all_gaps.append(gap)
            if gap < 30:
                sub30_records.append(record)
        previous = record

    index = 1
    while index < len(items):
        if not items[index]["boundary"]:
            index += 1
            continue
        start = index
        while index < len(items) and items[index]["boundary"]:
            index += 1
        end = index - 1
        length = end - start + 1
        if length >= 3:
            chain = items[start - 1 : end + 1]
            for record in chain:
                record["run_flag"] = True
            runs.append(
                {
                    "polres": key[1],
                    "reporter": key[0],
                    "satker": chain[0]["satker"],
                    "length": length,
                    "start_ms": chain[0]["ms"],
                    "end_ms": chain[-1]["ms"],
                    "gaps": [round(record["gap"], 3) for record in chain[1:]],
                    "nondup": not any(record["all_dup"] for record in chain),
                    "reports": chain,
                }
            )

stats = {}
for polres in sorted({record["polres"] for record in reports.values() if record["polres"].strip()}):
    items = [record for record in reports.values() if record["polres"] == polres]
    gaps = [record["gap"] for record in items if record["gap"] is not None]
    polres_runs = [run for run in runs if run["polres"] == polres]
    stats[polres] = {
        "reports": len(items),
        "reporters": len({record["key"] for record in items}),
        "rows": sum(len(record["rows"]) for record in items),
        "gaps": len(gaps),
        "duplicate_reports": sum(1 for record in items if record["all_dup"]),
        "duplicate_any": sum(1 for record in items if record["any_dup"]),
        "duplicate_rows": sum(record["dup_rows"] for record in items),
        "sub30": sum(1 for gap in gaps if gap < 30),
        "sub3": sum(1 for gap in gaps if gap < 3),
        "bot": sum(1 for record in items if record["bot"]),
        "boundary": sum(1 for gap in gaps if 30 <= gap <= 33),
        "runs": len(polres_runs),
        "run_reports": sum(1 for record in items if record["run_flag"]),
        "nondup_runs": sum(1 for run in polres_runs if run["nondup"]),
        "median_gap": statistics.median(gaps) if gaps else 0,
        "min_gap": min(gaps) if gaps else 0,
        "boundary_rate": sum(1 for gap in gaps if 30 <= gap <= 33) / len(gaps) * 100 if gaps else 0,
        "run_rate": len(polres_runs) / len(items) * 1000 if items else 0,
        "duplicate_rate": sum(1 for record in items if record["all_dup"]) / len(items) * 100 if items else 0,
    }

stats_period = {}
for polres in sorted({record["polres"] for record in reports.values() if record["polres"].strip()}):
    items = [record for record in reports.values() if record["polres"] == polres]
    gaps = [
        record["gap"]
        for record in items
        if record["gap"] is not None and record["date"] >= "2026-11-11"
    ]
    period_runs = [
        run
        for run in runs
        if run["polres"] == polres
        and datetime.fromtimestamp(run["start_ms"] / 1000, WIB).strftime("%Y-%m-%d") >= "2026-11-11"
    ]
    stats_period[polres] = {
        "reports": len(items),
        "gaps": len(gaps),
        "sub30": sum(1 for gap in gaps if gap < 30),
        "boundary": sum(1 for gap in gaps if 30 <= gap <= 33),
        "boundary_rate": sum(1 for gap in gaps if 30 <= gap <= 33) / len(gaps) * 100 if gaps else 0,
        "runs": len(period_runs),
        "nondup_runs": sum(1 for run in period_runs if run["nondup"]),
        "duplicate_reports": sum(1 for record in items if record["all_dup"]),
        "duplicate_rate": sum(1 for record in items if record["all_dup"]) / len(items) * 100 if items else 0,
    }


def report_payload(record):
    full_rows = []
    for row in record["rows"]:
        source = text_by_row.get(row["row"], {})
        full_rows.append(
            {
                "row": row["row"],
                "kode": row.get("kode") or "",
                "kegiatan": row.get("kegiatan") or "",
                "isian": source.get("isian") or "",
                "bukti": source.get("bukti") or "",
                "tanggal": row.get("tanggal") or "",
                "waktu": row.get("waktu") or "",
                "satker": row.get("satuan") or "",
            }
        )
    return {
        "id": record["id"],
        "ms": record["ms"],
        "time": stamp(record["ms"]),
        "short_time": short_time(record["ms"]),
        "date": record["date"],
        "polres": record["polres"],
        "satker": record["satker"],
        "reporter": record["reporter"],
        "pelapor_raw": record["pelapor_raw"],
        "gap": round(record["gap"], 3) if record["gap"] is not None else None,
        "bot": record["bot"],
        "boundary": record["boundary"],
        "run_flag": record["run_flag"],
        "all_dup": record["all_dup"],
        "any_dup": record["any_dup"],
        "kodes": sorted(record["kodes"]),
        "rows": full_rows,
    }


focus = {}
for polres in FOCUS:
    items = [record for record in reports.values() if record["polres"] == polres]
    polres_runs = [run for run in runs if run["polres"] == polres]
    sub30 = [record for record in items if record["gap"] is not None and record["gap"] < 30]
    bot = [record for record in items if record["bot"]]
    run_details = []
    for run in polres_runs:
        run_details.append(
            {
                "reporter": run["reporter"],
                "satker": run["satker"],
                "length": run["length"],
                "start": short_time(run["start_ms"]),
                "end": short_time(run["end_ms"]),
                "gaps": run["gaps"],
                "nondup": run["nondup"],
                "reports": [report_payload(record) for record in run["reports"]],
            }
        )
    focus[polres] = {
        "stats": stats[polres],
        "runs": run_details,
        "sub30": [report_payload(record) for record in sorted(sub30, key=lambda item: item["ms"])],
        "bot": [report_payload(record) for record in bot],
    }

reporter_stats = defaultdict(lambda: {"boundary": 0, "runs": 0, "gaps": 0, "polres": "", "satkers": Counter()})
for record in reports.values():
    key = (record["reporter"], record["polres"])
    target = reporter_stats[key]
    target["polres"] = record["polres"]
    target["satkers"][record["satker"]] += 1
    if record["gap"] is not None:
        target["gaps"] += 1
        if record["boundary"]:
            target["boundary"] += 1
    if record["run_flag"]:
        target["runs"] += 1

top_reporters = []
for (reporter, polres), data in reporter_stats.items():
    if data["boundary"] == 0:
        continue
    top_reporters.append(
        {
            "reporter": reporter,
            "polres": polres,
            "satker": data["satkers"].most_common(1)[0][0],
            "boundary": data["boundary"],
            "gaps": data["gaps"],
            "runs": data["runs"],
        }
    )
top_reporters.sort(key=lambda item: (-item["runs"], -item["boundary"], item["reporter"]))

histogram = Counter()
for gap in all_gaps:
    histogram[min(int(gap), 61)] += 1

result = {
    "generated_from": "report-2026-11-10-to-2026-11-19.xlsx",
    "reports": len(reports),
    "rows": len(rows),
    "reporters": len(by_reporter),
    "gaps": len(all_gaps),
    "min_gap": min(all_gaps),
    "median_gap": statistics.median(all_gaps),
    "sub30": sum(1 for gap in all_gaps if gap < 30),
    "sub3": sum(1 for gap in all_gaps if gap < 3),
    "boundary": sum(1 for gap in all_gaps if 30 <= gap <= 33),
    "stats": stats,
    "stats_period": stats_period,
    "focus": focus,
    "runs": len(runs),
    "nondup_runs": sum(1 for run in runs if run["nondup"]),
    "top_reporters": top_reporters,
    "histogram": {str(key): value for key, value in sorted(histogram.items())},
    "boundary_runs": [
        {
            "polres": run["polres"],
            "reporter": run["reporter"],
            "satker": run["satker"],
            "length": run["length"],
            "start": short_time(run["start_ms"]),
            "end": short_time(run["end_ms"]),
            "gaps": run["gaps"],
            "nondup": run["nondup"],
        }
        for run in sorted(runs, key=lambda item: (-item["length"], item["polres"]))
    ],
}

with open(OUT, "w", encoding="utf-8") as file:
    json.dump(result, file, ensure_ascii=False, indent=2)

print("laporan", result["reports"], "baris", result["rows"], "pelapor", result["reporters"], "jeda", result["gaps"])
print(
    "min", result["min_gap"], "median", result["median_gap"], "sub30", result["sub30"],
    "sub3", result["sub3"], "boundary", result["boundary"], "runs", result["runs"],
    "nondup_runs", result["nondup_runs"],
)
print("\nFOKUS")
for polres in FOCUS:
    print(polres, stats[polres])
print("\nTOP 15 BOUNDARY RATE")
for polres, item in sorted(stats.items(), key=lambda pair: -pair[1]["boundary_rate"])[:15]:
    print(
        f"  {polres:26} reports {item['reports']:5} reporters {item['reporters']:3} "
        f"gaps {item['gaps']:5} <30 {item['sub30']:4} 30-33 {item['boundary']:4} "
        f"({item['boundary_rate']:5.2f}%) runs {item['runs']:3} runrep {item['run_reports']:3} bot {item['bot']}"
    )
print("\nTOP REPORTER RUNS")
for item in top_reporters[:15]:
    print(f"  {item['polres']:26} {item['reporter']:30} runs {item['runs']:3} boundary {item['boundary']:3}/{item['gaps']:4}")
