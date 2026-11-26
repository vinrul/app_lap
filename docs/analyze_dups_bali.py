"""Analisis duplikasi isian laporan Polda Bali 11-24 November 2026.

Kategori:
  A  - QW sama, satker+pelapor+tanggal sama, jeda <=10 menit, narasi identik/kuat.
  B1 - QW berbeda, narasi kejadian sama (mengandung tanggal/tempat), jeda <=6 jam.
  B2 - QW berbeda, narasi templat/generik sama, jeda <=6 jam.
  C  - narasi identik diulang pada hari berbeda (>=3 hari) untuk QW yang sama.

Sumber: data/tmp_dup_bali/reports.json (hasil extract_dups_bali.py).
Keluaran: data/tmp_dup_bali/analysis.json
"""

import json
import re
from collections import Counter, defaultdict
from difflib import SequenceMatcher

SRC = "data/tmp_dup_bali/reports.json"
OUT = "data/tmp_dup_bali/analysis.json"

URL_RE = re.compile(r"https?://\S+|www\.\S+", re.I)
COORD_RE = re.compile(r"\(\s*-?\d+\.\d+\s*,\s*-?\d+\.\d+\s*\)")
QW_RE = re.compile(r"\bqw\s*\d+\.\d+\b", re.I)
FIELD_RE = re.compile(r"(?:(?<=^)|(?<=\s))([a-j])\.\s")

SKIP_LABELS = (
    "pelaksana",
    "kegiatan yang dilaksanakan",
    "lokasi",
    "jam pelaksanaan",
    "foto",
    "apakah",
    "jumlah",
    "status",
    "titik gatur",
    "ada gatur",
    "tautan",
    "link",
    "diterima",
)
GENERIC = {"ya", "tidak", "tidak ada", "nihil", "0", "-", "ada", "tidak tahu"}

EVENT_RE = re.compile(
    r"kecelakaan|lakalantas|laka|tabrakan|pencurian|curanmor|curat|curas|penipuan|"
    r"penggelapan|penganiayaan|kekerasan|pelecehan|pemerkosaan|narkoba|narkotika|"
    r"judi|sabung ayam|balap liar|keributan|perkelahian|kebakaran|banjir|longsor|"
    r"pohon tumbang|bencana|orang hilang|tenggelam|mabuk|miras|senjata tajam|sajam|"
    r"pengeroyokan|perampasan|penodongan|pembunuhan|bunuh diri|tindak pidana|"
    r"kejahatan|terlantar|kehilangan|hilang|karhutla",
    re.I,
)
DATE_RE = re.compile(r"pada hari|tanggal|pukul|wita|sekira|sekitar jam", re.I)
PLACE_RE = re.compile(
    r"desa|kelurahan|kecamatan|kabupaten|kota|jalan|jl\.?|tempat|tkp|bertempat|"
    r"wilayah|banjar",
    re.I,
)


def narrative(text: str) -> str:
    text = URL_RE.sub(" ", str(text or ""))
    text = COORD_RE.sub(" ", text)
    text = QW_RE.sub(" ", text)
    chunks = FIELD_RE.split(text)
    has_fields = len(chunks) >= 3 and chunks[1].isalpha() and len(chunks[1]) == 1
    iterator = list(zip(chunks[1::2], chunks[2::2])) if has_fields else [(None, text)]
    kept = []
    for label, body in iterator:
        body = body.strip()
        label_text = ""
        if ":" in body:
            label_text, body = body.split(":", 1)
        if label and any(token in label_text.strip().lower() for token in SKIP_LABELS):
            continue
        value = body.strip(" .;-")
        if len(value) < 45 or value.lower() in GENERIC:
            continue
        kept.append(value)
    if has_fields and not kept:
        return ""
    joined = " ".join(kept) if kept else text
    joined = re.sub(r"[^a-z0-9]+", " ", joined.lower())
    return re.sub(r"\s+", " ", joined).strip()


def similarity(a: str, b: str):
    matcher = SequenceMatcher(None, a, b)
    blocks = [block for block in matcher.get_matching_blocks() if block.size >= 30]
    matched = sum(block.size for block in blocks)
    base = min(len(a), len(b))
    coverage = matched / base if base else 0.0
    longest = max(matcher.get_matching_blocks(), key=lambda block: block.size)
    shared = a[longest.a : longest.a + longest.size]
    return coverage, matched, matcher.ratio(), shared, blocks


def report_payload(report):
    return {
        "uuid": report["uuid"],
        "wib": report["wib"],
        "jam": report["wib"][11:23],
        "tanggal_lapor": report["tanggal_lapor"],
        "kode": report["kode"],
        "isian": report["isian"],
        "nar": report["nar"],
        "row": report["excel_row"],
        "ms": report["ms"],
    }


def pair_payload(category, coverage, matched, ratio, gap, a, b, shared):
    return {
        "kategori": category,
        "satuan": a["satuan"],
        "polres": a["polres"],
        "polsek": a["polsek"],
        "pelapor": a["pelapor"],
        "tanggal": a["tanggal_kegiatan"],
        "gap_s": round(gap, 3),
        "coverage": round(coverage, 4),
        "matched": matched,
        "ratio": round(ratio, 4),
        "shared": shared,
        "shared_len": len(shared),
        "a": report_payload(a),
        "b": report_payload(b),
    }


with open(SRC, encoding="utf-8") as handle:
    reports = json.load(handle)
for report in reports:
    report["nar"] = narrative(report["isian"])

# ---------------------------------------------------------------------------
# A. QW sama, <=10 menit.
# ---------------------------------------------------------------------------
by_same_qw = defaultdict(list)
for report in reports:
    if len(report["nar"]) >= 40:
        key = (
            report["satuan"].lower(),
            report["pelapor"].lower(),
            report["kode"],
            report["tanggal_kegiatan"],
        )
        by_same_qw[key].append(report)

pairs_a = []
for key, items in by_same_qw.items():
    items.sort(key=lambda item: item["ms"])
    for index in range(1, len(items)):
        gap = (items[index]["ms"] - items[index - 1]["ms"]) / 1000
        if gap > 600:
            continue
        a, b = items[index - 1], items[index]
        coverage, matched, ratio, shared, _ = similarity(a["nar"], b["nar"])
        if ratio >= 0.85 or matched >= 100:
            pairs_a.append(pair_payload("A", coverage, matched, ratio, gap, a, b, shared))

pairs_a.sort(key=lambda item: (-item["ratio"], -item["matched"]))

# ---------------------------------------------------------------------------
# B. QW berbeda, <=6 jam, narasi kejadian/templat sama.
# ---------------------------------------------------------------------------
by_unit_date = defaultdict(list)
for report in reports:
    if len(report["nar"]) >= 100 and EVENT_RE.search(report["nar"]):
        key = (
            report["satuan"].lower(),
            report["pelapor"].lower(),
            report["tanggal_kegiatan"],
        )
        by_unit_date[key].append(report)

pairs_b_incident = []
pairs_b_template = []
for key, items in by_unit_date.items():
    items.sort(key=lambda item: item["ms"])
    for left in range(len(items)):
        for right in range(left + 1, len(items)):
            a, b = items[left], items[right]
            if a["kode"] == b["kode"]:
                continue
            gap = (b["ms"] - a["ms"]) / 1000
            if gap > 6 * 3600:
                continue
            coverage, matched, ratio, shared, blocks = similarity(a["nar"], b["nar"])
            if matched < 300 or coverage < 0.5:
                continue
            block_text = " ".join(
                a["nar"][block.a : block.a + block.size] for block in blocks
            )
            if not EVENT_RE.search(block_text):
                continue
            has_date = bool(DATE_RE.search(block_text))
            has_place = bool(PLACE_RE.search(block_text))
            payload = pair_payload("B", coverage, matched, ratio, gap, a, b, shared)
            if has_date and has_place:
                pairs_b_incident.append(payload)
            else:
                pairs_b_template.append(payload)

pairs_b_incident.sort(key=lambda item: (-item["matched"], -item["coverage"]))
pairs_b_template.sort(key=lambda item: (-item["matched"], -item["coverage"]))

# ---------------------------------------------------------------------------
# C. Narasi identik diulang lintas hari.
# ---------------------------------------------------------------------------
by_reporter_qw = defaultdict(list)
for report in reports:
    if len(report["nar"]) >= 80:
        key = (report["satuan"].lower(), report["pelapor"].lower(), report["kode"])
        by_reporter_qw[key].append(report)

groups_c = []
for key, items in by_reporter_qw.items():
    variants = Counter(item["nar"] for item in items)
    top_nar, top_count = variants.most_common(1)[0]
    if top_count < 3:
        continue
    sample_items = [item for item in items if item["nar"] == top_nar]
    dates = sorted({item["tanggal_kegiatan"] for item in sample_items})
    if len(dates) < 3:
        continue
    sample_items.sort(key=lambda item: item["ms"])
    groups_c.append(
        {
            "kategori": "C",
            "satuan": sample_items[0]["satuan"],
            "polres": sample_items[0]["polres"],
            "polsek": sample_items[0]["polsek"],
            "pelapor": sample_items[0]["pelapor"],
            "kode": sample_items[0]["kode"],
            "count": top_count,
            "days": len(dates),
            "dates": dates,
            "nar": top_nar,
            "isian": sample_items[0]["isian"],
            "uuids": [item["uuid"] for item in sample_items],
            "rows": [item["excel_row"] for item in sample_items],
            "first_wib": sample_items[0]["wib"],
            "last_wib": sample_items[-1]["wib"],
        }
    )

groups_c.sort(key=lambda item: (-item["count"], item["satuan"], item["pelapor"]))

# ---------------------------------------------------------------------------
# Rekap per satker / polres / pelapor.
# ---------------------------------------------------------------------------


def new_stat():
    return {
        "a_pairs": 0,
        "b_incident_pairs": 0,
        "b_template_pairs": 0,
        "c_groups": 0,
        "c_reports": 0,
        "uuids": set(),
        "uuids_a": set(),
        "uuids_b": set(),
        "uuids_c": set(),
        "pelapor": set(),
        "satuan": set(),
        "polres": "",
        "polsek": set(),
    }


def ingest(stat, item, kind):
    if kind == "A":
        stat["a_pairs"] += 1
        for side in ("a", "b"):
            stat["uuids"].add(item[side]["uuid"])
            stat["uuids_a"].add(item[side]["uuid"])
        stat["pelapor"].add(item["pelapor"])
        stat["satuan"].add(item["satuan"])
        stat["polsek"].add(item["polsek"] or item["satuan"])
    elif kind in ("B1", "B2"):
        stat["b_incident_pairs" if kind == "B1" else "b_template_pairs"] += 1
        for side in ("a", "b"):
            stat["uuids"].add(item[side]["uuid"])
            stat["uuids_b"].add(item[side]["uuid"])
        stat["pelapor"].add(item["pelapor"])
        stat["satuan"].add(item["satuan"])
        stat["polsek"].add(item["polsek"] or item["satuan"])
    else:
        stat["c_groups"] += 1
        stat["c_reports"] += item["count"]
        stat["uuids"].update(item["uuids"])
        stat["uuids_c"].update(item["uuids"])
        stat["pelapor"].add(item["pelapor"])
        stat["satuan"].add(item["satuan"])
        stat["polsek"].add(item["polsek"] or item["satuan"])


satker_stats = defaultdict(new_stat)
polres_stats = defaultdict(new_stat)
pelapor_stats = defaultdict(new_stat)

for pair in pairs_a:
    for table, key in (
        (satker_stats, pair["satuan"]),
        (polres_stats, pair["polres"]),
        (pelapor_stats, (pair["pelapor"], pair["satuan"])),
    ):
        stat = table[key]
        stat["polres"] = pair["polres"]
        ingest(stat, pair, "A")

for pairs, kind in ((pairs_b_incident, "B1"), (pairs_b_template, "B2")):
    for pair in pairs:
        for table, key in (
            (satker_stats, pair["satuan"]),
            (polres_stats, pair["polres"]),
            (pelapor_stats, (pair["pelapor"], pair["satuan"])),
        ):
            stat = table[key]
            stat["polres"] = pair["polres"]
            ingest(stat, pair, kind)

for group in groups_c:
    for table, key in (
        (satker_stats, group["satuan"]),
        (polres_stats, group["polres"]),
        (pelapor_stats, (group["pelapor"], group["satuan"])),
    ):
        stat = table[key]
        stat["polres"] = group["polres"]
        ingest(stat, group, "C")


def export_ranking(table):
    rows = []
    for key, stat in table.items():
        rows.append(
            {
                "key": key if isinstance(key, str) else list(key),
                "satuan": ", ".join(sorted(stat["satuan"])),
                "polres": stat["polres"],
                "polsek": ", ".join(sorted(stat["polsek"])),
                "pelapor_count": len(stat["pelapor"]),
                "a_pairs": stat["a_pairs"],
                "b_incident_pairs": stat["b_incident_pairs"],
                "b_template_pairs": stat["b_template_pairs"],
                "c_groups": stat["c_groups"],
                "c_reports": stat["c_reports"],
                "a_reports": len(stat["uuids_a"]),
                "b_reports": len(stat["uuids_b"]),
                "direct_reports": len(stat["uuids_a"] | stat["uuids_b"]),
                "c_report_count": len(stat["uuids_c"]),
                "total_reports": len(stat["uuids"]),
            }
        )
    rows.sort(
        key=lambda row: (
            -row["total_reports"],
            -row["a_pairs"],
            -row["b_incident_pairs"],
            -row["c_groups"],
        )
    )
    return rows


rank_polres = export_ranking(polres_stats)
rank_satker = export_ranking(satker_stats)
rank_pelapor = export_ranking(pelapor_stats)

# ---------------------------------------------------------------------------
# Contoh fokus: kecelakaan lintas QW.
# ---------------------------------------------------------------------------
accident = [
    pair
    for pair in pairs_b_incident
    if re.search(r"kecelakaan|lakalantas|laka|tabrakan", pair["shared"], re.I)
]
accident.sort(key=lambda item: (item["gap_s"], -item["matched"]))

# ---------------------------------------------------------------------------
# Cek khusus contoh pengguna: QW5.3 -> QW4.2 dalam <=10 menit.
# ---------------------------------------------------------------------------
by_unit_date_all = defaultdict(list)
for report in reports:
    by_unit_date_all[
        (report["satuan"].lower(), report["pelapor"].lower(), report["tanggal_kegiatan"])
    ].append(report)

qw53_pairs = []
qw53_accident = 0
for key, items in by_unit_date_all.items():
    a_list = [item for item in items if item["kode"] == "QW5.3"]
    b_list = [item for item in items if item["kode"] == "QW4.2"]
    for a in a_list:
        for b in b_list:
            gap = abs(b["ms"] - a["ms"]) / 1000
            if gap > 600:
                continue
            ratio = SequenceMatcher(None, a["norm"], b["norm"]).ratio()
            qw53_pairs.append(ratio)
            if re.search(r"kecelakaan|lakalantas|laka", a["isian"], re.I) and re.search(
                r"kecelakaan|lakalantas|laka", b["isian"], re.I
            ):
                qw53_accident += 1

payload = {
    "meta": {
        "source": "report-2026-11-11-to-2026-11-24-bali.xlsx",
        "total_reports": len(reports),
        "total_satker": len({report["satuan"] for report in reports}),
        "total_polres": len({report["polres"] for report in reports if report["polres"]}),
        "total_pelapor": len({report["pelapor"] for report in reports if report["pelapor"]}),
        "a_pairs": len(pairs_a),
        "a_reports": len({pair[side]["uuid"] for pair in pairs_a for side in ("a", "b")}),
        "b_incident_pairs": len(pairs_b_incident),
        "b_incident_reports": len(
            {pair[side]["uuid"] for pair in pairs_b_incident for side in ("a", "b")}
        ),
        "b_template_pairs": len(pairs_b_template),
        "b_template_reports": len(
            {pair[side]["uuid"] for pair in pairs_b_template for side in ("a", "b")}
        ),
        "c_groups": len(groups_c),
        "c_reports": sum(group["count"] for group in groups_c),
        "accident_pairs": len(accident),
        "qw53_qw42_pairs": len(qw53_pairs),
        "qw53_qw42_max_ratio": round(max(qw53_pairs), 4) if qw53_pairs else 0,
        "qw53_qw42_accident": qw53_accident,
        "date_range": ["2026-11-11", "2026-11-24"],
    },
    "pairs_a": pairs_a,
    "pairs_b_incident": pairs_b_incident,
    "pairs_b_template": pairs_b_template,
    "groups_c": groups_c,
    "accident": accident[:12],
    "ranking_polres": rank_polres,
    "ranking_satker": rank_satker,
    "ranking_pelapor": rank_pelapor,
}

with open(OUT, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, ensure_ascii=False, separators=(",", ":"))

print("A pairs:", len(pairs_a), "| reports:", payload["meta"]["a_reports"])
print("B1 incident pairs:", len(pairs_b_incident), "| reports:", payload["meta"]["b_incident_reports"])
print("B2 template pairs:", len(pairs_b_template), "| reports:", payload["meta"]["b_template_reports"])
print("C groups:", len(groups_c), "| reports:", payload["meta"]["c_reports"])
print("accident pairs:", len(accident))
print("\nTop polres:")
for row in rank_polres[:6]:
    print(
        f"  {row['key']}: total={row['total_reports']} A={row['a_pairs']} "
        f"B1={row['b_incident_pairs']} B2={row['b_template_pairs']} C={row['c_groups']}"
    )
print("\nTop satker:")
for row in rank_satker[:8]:
    print(
        f"  {row['key']}: total={row['total_reports']} A={row['a_pairs']} "
        f"B1={row['b_incident_pairs']} B2={row['b_template_pairs']} C={row['c_groups']}"
    )
print("\nTop pelapor:")
for row in rank_pelapor[:8]:
    print(
        f"  {row['key']}: total={row['total_reports']} A={row['a_pairs']} "
        f"B1={row['b_incident_pairs']} B2={row['b_template_pairs']} C={row['c_groups']}"
    )
print("\nContoh kecelakaan:")
for pair in accident[:5]:
    print(
        f"  {pair['satuan']} | {pair['pelapor']} | {pair['tanggal']} | "
        f"{pair['a']['kode']}->{pair['b']['kode']} | gap {pair['gap_s']:.0f}s"
    )
