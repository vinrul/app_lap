import html
import json
import os
import re
import subprocess
from collections import Counter, OrderedDict
from difflib import SequenceMatcher

ANALYSIS = "data/tmp_dup_bali/analysis.json"
OUT_HTML = "data/tmp_dup_bali/laporan_duplikat_bali.html"
OUT_PDF = "Laporan-Analisis-Duplikasi-Laporan-Polda-Bali-11-24-November-2026.pdf"
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"

TITLE = "Analisis Duplikasi Isian Laporan"
SUBTITLE = "Polda Bali · 11–24 November 2026"

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
LONG_DIGITS = re.compile(r"\b\d{10,}\b")
EMAIL_RE = re.compile(r"\b[\w.+-]+@[\w-]+\.[\w.-]+\b")


def mask_sensitive(text: str) -> str:
    text = EMAIL_RE.sub("[email disamarkan]", text)
    return LONG_DIGITS.sub(lambda match: match.group(0)[:4] + "•" * (len(match.group(0)) - 4), text)


def esc(value) -> str:
    return html.escape(str(value or ""), quote=True)


def truncate(value: str, limit: int) -> str:
    value = re.sub(r"\s+", " ", str(value or "")).strip()
    if len(value) <= limit:
        return value
    return value[: limit - 1].rstrip() + "…"


def num(value) -> str:
    return f"{int(value):,}".replace(",", ".")


def dec(value, digits=1) -> str:
    return f"{float(value):,.{digits}f}".replace(",", "X").replace(".", ",").replace("X", ".")


def pct(value) -> str:
    return dec(float(value) * 100, 1) + "%"


def gap_text(seconds: float) -> str:
    seconds = float(seconds)
    if seconds < 60:
        return dec(seconds, 1) + " dtk"
    minutes = int(seconds // 60)
    rest = seconds - minutes * 60
    if minutes < 60:
        return f"{minutes} mnt {dec(rest, 0)} dtk"
    hours = minutes // 60
    minutes = minutes % 60
    return f"{hours} jam {minutes} mnt"


def date_id(value: str) -> str:
    # 2026-11-18 -> 18/11/2026
    if len(value) >= 10 and value[4] == "-":
        return value[8:10] + "/" + value[5:7] + "/" + value[0:4]
    return value


def date_short(value: str) -> str:
    if len(value) >= 10 and value[4] == "-":
        return value[8:10] + "/" + value[5:7]
    return value


def date_range(dates) -> str:
    dates = sorted(set(dates))
    if not dates:
        return "—"
    if len(dates) == 1:
        return date_id(dates[0])
    return date_id(dates[0]) + " – " + date_id(dates[-1])


def date_count_text(dates) -> str:
    dates = sorted(set(dates))
    return f"{len(dates)} hari · {date_short(dates[0])}–{date_short(dates[-1])}" if dates else "—"


def format_isian_full(text: str) -> str:
    text = mask_sensitive(str(text or ""))
    lines = []
    for raw_line in text.replace("\r", "\n").split("\n"):
        line = re.sub(r"[ \t]+", " ", raw_line).strip()
        if not line:
            continue
        line = re.sub(r"\s+(?=[a-j]\.\s)", "\n", line)
        line = re.sub(r"\s+(?=\d{1,2}\)\s)", "\n", line)
        lines.append(line)
    return "\n".join(lines).strip()


def display_narrative(text: str) -> str:
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
    joined = " · ".join(kept) if kept else text
    joined = re.sub(r"\s+", " ", joined).strip()
    return mask_sensitive(joined)


def shared_display(pair: dict, limit: int = 260) -> str:
    a = display_narrative(pair["a"]["isian"])
    b = display_narrative(pair["b"]["isian"])
    if a and b:
        matcher = SequenceMatcher(None, a, b)
        longest = max(matcher.get_matching_blocks(), key=lambda block: block.size)
        shared = a[longest.a : longest.a + longest.size].strip()
        if len(shared) >= 80:
            return truncate(shared, limit)
    return truncate(pair["shared"], limit)


def is_identical(pair: dict) -> bool:
    return pair["ratio"] >= 0.999


with open(ANALYSIS, encoding="utf-8") as handle:
    data = json.load(handle)

META = data["meta"]
PAIRS_A = data["pairs_a"]
PAIRS_B1 = data["pairs_b_incident"]
PAIRS_B2 = data["pairs_b_template"]
GROUPS_C = data["groups_c"]
ACCIDENT = data["accident"]
RANK_POLRES = data["ranking_polres"]
RANK_SATKER = data["ranking_satker"]
RANK_PELAPOR = data["ranking_pelapor"]

TOTAL_REPORTS = META["total_reports"]


def uuid_set(pairs):
    return {pair[side]["uuid"] for pair in pairs for side in ("a", "b")}


DIRECT_REPORTS = len(uuid_set(PAIRS_A) | uuid_set(PAIRS_B1) | uuid_set(PAIRS_B2))
C_REPORTS = META["c_reports"]
ALL_REPORTS = len(uuid_set(PAIRS_A) | uuid_set(PAIRS_B1) | uuid_set(PAIRS_B2) | {u for g in GROUPS_C for u in g["uuids"]})
TOTAL_PAIRS = len(PAIRS_A) + len(PAIRS_B1) + len(PAIRS_B2)
FAST_B1 = sum(1 for pair in PAIRS_B1 if pair["gap_s"] <= 600)
FAST_B2 = sum(1 for pair in PAIRS_B2 if pair["gap_s"] <= 600)
QW53_PAIRS = META.get("qw53_qw42_pairs", 0)
QW53_MAX = META.get("qw53_qw42_max_ratio", 0)
QW53_ACCIDENT = META.get("qw53_qw42_accident", 0)

# ---------------------------------------------------------------------------
# Peringkat.
# ---------------------------------------------------------------------------
DIRECT_POLRES = sorted(
    [row for row in RANK_POLRES if row["direct_reports"] > 0],
    key=lambda row: (-row["direct_reports"], -row["a_pairs"], -row["b_incident_pairs"]),
)
DIRECT_SATKER = sorted(
    [row for row in RANK_SATKER if row["direct_reports"] > 0],
    key=lambda row: (-row["direct_reports"], -row["a_pairs"], -row["b_incident_pairs"]),
)
DIRECT_PELAPOR = sorted(
    [row for row in RANK_PELAPOR if row["direct_reports"] > 0],
    key=lambda row: (-row["direct_reports"], -row["a_pairs"], -row["b_incident_pairs"]),
)
C_POLRES = sorted(
    [row for row in RANK_POLRES if row["c_report_count"] > 0],
    key=lambda row: (-row["c_report_count"], -row["c_groups"]),
)
C_SATKER = sorted(
    [row for row in RANK_SATKER if row["c_report_count"] > 0],
    key=lambda row: (-row["c_report_count"], -row["c_groups"]),
)
C_PELAPOR = sorted(
    [row for row in RANK_PELAPOR if row["c_report_count"] > 0],
    key=lambda row: (-row["c_report_count"], -row["c_groups"]),
)


def polres_name(row) -> str:
    if isinstance(row["key"], str) and row["key"]:
        return row["key"]
    return row["satuan"] or "Polda Bali"


def pelapor_name(row) -> str:
    return row["key"][0] if isinstance(row["key"], list) else row["key"]


def pelapor_satker(row) -> str:
    return row["key"][1] if isinstance(row["key"], list) else row["satuan"]


def group_by(items, keyfunc):
    groups = OrderedDict()
    for item in items:
        groups.setdefault(keyfunc(item), []).append(item)
    return groups


# ---------------------------------------------------------------------------
# HTML helpers.
# ---------------------------------------------------------------------------
def cards_html() -> str:
    cards = [
        ("A · QW sama ≤10 mnt", num(len(PAIRS_A)), f"{num(META['a_reports'])} laporan terlibat"),
        ("B1 · Narasi kejadian", num(len(PAIRS_B1)), f"{num(META['b_incident_reports'])} laporan · {num(FAST_B1)} pasangan ≤10 mnt"),
        ("B2 · Narasi templat", num(len(PAIRS_B2)), f"{num(META['b_template_reports'])} laporan · {num(FAST_B2)} pasangan ≤10 mnt"),
        ("C · Identik lintas hari", num(len(GROUPS_C)), f"{num(C_REPORTS)} laporan · ≥3 hari"),
    ]
    blocks = []
    for label, value, note in cards:
        blocks.append(
            '<div class="card">'
            f'<span class="label">{esc(label)}</span>'
            f"<strong>{value}</strong>"
            f"<small>{esc(note)}</small>"
            "</div>"
        )
    return '<div class="cards">' + "".join(blocks) + "</div>"


def top_findings_rows() -> str:
    rows = []

    def add(rank, satker, polres, pelapor, tanggal, qw, gap, score, note):
        rows.append(
            "<tr>"
            f'<td class="num">{rank}</td>'
            f"<td><strong>{esc(satker)}</strong><br><span class='sub'>{esc(polres)}</span></td>"
            f"<td>{esc(pelapor)}</td>"
            f'<td class="nowrap">{esc(date_id(tanggal))}</td>'
            f'<td class="kode">{esc(qw)}</td>'
            f'<td class="num nowrap">{esc(gap)}</td>'
            f'<td class="num">{esc(score)}</td>'
            f"<td>{esc(note)}</td>"
            "</tr>"
        )

    # A paling ekstrem
    a_sorted = sorted(PAIRS_A, key=lambda pair: (pair["gap_s"], -pair["matched"]))
    for pair in a_sorted[:3]:
        add(
            len(rows) + 1,
            pair["satuan"],
            "Lingkup Polda" if not pair["polres"] else pair["polres"],
            pair["pelapor"],
            pair["tanggal"],
            pair["a"]["kode"],
            gap_text(pair["gap_s"]),
            pct(pair["ratio"]),
            "Isian identik pada QW yang sama",
        )
    # B1 tercepat
    for pair in sorted(PAIRS_B1, key=lambda item: item["gap_s"])[:3]:
        note = "Narasi kejadian sama"
        if re.search(r"kecelakaan|lakalantas|laka", pair["shared"], re.I):
            note = "Narasi kecelakaan sama"
        add(
            len(rows) + 1,
            pair["satuan"],
            "Lingkup Polda" if not pair["polres"] else pair["polres"],
            pair["pelapor"],
            pair["tanggal"],
            pair["a"]["kode"] + " → " + pair["b"]["kode"],
            gap_text(pair["gap_s"]),
            num(pair["matched"]) + " kar.",
            note,
        )
    # B2 terbesar
    b2_by_unit = Counter(pair["satuan"] for pair in PAIRS_B2)
    b2_pelapor = {}
    for pair in PAIRS_B2:
        b2_pelapor.setdefault(pair["satuan"], Counter())[pair["pelapor"]] += 1
    for satker, count in b2_by_unit.most_common(1):
        pelapor, pcount = b2_pelapor[satker].most_common(1)[0]
        unit_pairs = [pair for pair in PAIRS_B2 if pair["satuan"] == satker]
        combo = Counter((pair["a"]["kode"], pair["b"]["kode"]) for pair in unit_pairs).most_common(1)[0][0]
        same_combo = [pair for pair in unit_pairs if (pair["a"]["kode"], pair["b"]["kode"]) == combo]
        representative = min(same_combo, key=lambda pair: pair["gap_s"])
        add(
            len(rows) + 1,
            satker,
            representative["polres"] or "Lingkup Polda",
            pelapor,
            representative["tanggal"],
            combo[0] + " → " + combo[1],
            gap_text(representative["gap_s"]),
            num(count) + " psg",
            "Narasi templat yang sama disalin ke beberapa QW",
        )
    # C terbesar
    for group in GROUPS_C[:2]:
        add(
            len(rows) + 1,
            group["satuan"],
            "Lingkup Polda" if not group["polres"] else group["polres"],
            group["pelapor"],
            group["dates"][0],
            group["kode"],
            "lintas hari",
            num(group["count"]) + "×",
            f"Isian identik {group['days']} hari",
        )
    return "".join(rows)


def ranking_polres_rows(mode: str) -> str:
    rows = []
    source = DIRECT_POLRES if mode == "direct" else C_POLRES
    for rank, row in enumerate(source, start=1):
        rows.append(
            f'<tr class="{"rank-1" if rank == 1 else ""}">'
            f'<td class="num">{rank}</td>'
            f"<td><strong>{esc(polres_name(row))}</strong></td>"
            f'<td class="num">{num(row["a_pairs"])}</td>'
            f'<td class="num">{num(row["b_incident_pairs"])}</td>'
            f'<td class="num">{num(row["b_template_pairs"])}</td>'
            f'<td class="num"><strong>{num(row["direct_reports"])}</strong></td>'
            f'<td class="num">{num(row["c_groups"])}</td>'
            f'<td class="num"><strong>{num(row["c_report_count"])}</strong></td>'
            "</tr>"
        )
    return "".join(rows)


def ranking_satker_rows(mode: str) -> str:
    rows = []
    source = DIRECT_SATKER if mode == "direct" else C_SATKER
    for rank, row in enumerate(source, start=1):
        rows.append(
            f'<tr class="{"rank-1" if rank == 1 else ""}">'
            f'<td class="num">{rank}</td>'
            f"<td><strong>{esc(row['key'])}</strong></td>"
            f"<td>{esc(row['polres'] or '—')}</td>"
            f'<td class="num">{num(row["a_pairs"])}</td>'
            f'<td class="num">{num(row["b_incident_pairs"])}</td>'
            f'<td class="num">{num(row["b_template_pairs"])}</td>'
            f'<td class="num"><strong>{num(row["direct_reports"])}</strong></td>'
            f'<td class="num">{num(row["c_groups"])}</td>'
            f'<td class="num"><strong>{num(row["c_report_count"])}</strong></td>'
            "</tr>"
        )
    return "".join(rows)


def ranking_pelapor_rows(mode: str) -> str:
    rows = []
    source = (DIRECT_PELAPOR if mode == "direct" else C_PELAPOR)[:25]
    for rank, row in enumerate(source, start=1):
        rows.append(
            f'<tr class="{"rank-1" if rank == 1 else ""}">'
            f'<td class="num">{rank}</td>'
            f"<td><strong>{esc(pelapor_name(row))}</strong></td>"
            f"<td>{esc(pelapor_satker(row))}</td>"
            f"<td>{esc(row['polres'] or '—')}</td>"
            f'<td class="num">{num(row["a_pairs"])}</td>'
            f'<td class="num">{num(row["b_incident_pairs"])}</td>'
            f'<td class="num">{num(row["b_template_pairs"])}</td>'
            f'<td class="num"><strong>{num(row["direct_reports"])}</strong></td>'
            f'<td class="num">{num(row["c_groups"])}</td>'
            f'<td class="num"><strong>{num(row["c_report_count"])}</strong></td>'
            "</tr>"
        )
    return "".join(rows)


def pair_block(pair: dict, show_shared: bool = True, full: bool = False) -> str:
    satker = pair["satuan"]
    polres_raw = pair["polres"]
    polres = polres_raw or "Polda Bali"
    polsek = pair["polsek"]
    pelapor = pair["pelapor"]
    tanggal = pair["tanggal"]
    a, b = pair["a"], pair["b"]
    shared = shared_display(pair, 240)
    identical = is_identical(pair)
    badge_class = "badge-red" if pair["gap_s"] <= 600 else "badge-gold"
    if a["kode"] == b["kode"]:
        badge = f"{a['kode']} · {gap_text(pair['gap_s'])}"
    else:
        badge = f"{a['kode']} → {b['kode']} · {gap_text(pair['gap_s'])}"
    if identical:
        badge += " · 100%"

    if not polres_raw:
        unit_line = "Lingkup Polda"
    elif polres_raw == satker:
        unit_line = "Tingkat Polres"
    else:
        unit_line = polres if polres.startswith("Polres") else f"Polres {polres}"
    if polsek and polsek != satker:
        unit_line += f" · {polsek}"

    def col(side_label, report):
        excerpt = display_narrative(report["isian"])
        if not excerpt:
            excerpt = report["isian"]
        return (
            '<div class="pair-col">'
            f'<div class="col-head"><span class="side">{side_label}</span>'
            f'<span class="kode">{esc(report["kode"])}</span>'
            f'<span class="jam">{esc(date_short(report["wib"][:10]))} {esc(report["jam"])} WIB</span></div>'
            f'<div class="uuid">UUID {esc(report["uuid"])} · baris {num(report["row"])}</div>'
            f'<div class="excerpt">{esc(truncate(excerpt, 190))}</div>'
            "</div>"
        )

    shared_html = ""
    if show_shared and shared and not identical:
        shared_html = (
            '<div class="shared-box">'
            f"<b>Bagian yang sama</b> · {num(pair['shared_len'])} karakter: "
            f"<i>{esc(truncate(shared, 200))}</i>"
            "</div>"
        )

    def full_text(report):
        text = format_isian_full(report["isian"])
        if not text:
            text = "(isian kosong)"
        return "".join(f'<div class="fl">{esc(part)}</div>' for part in text.split("\n"))

    def full_head(side_label, report):
        return (
            '<div class="full-report-head">'
            f'<span class="side">{side_label}</span>'
            f'<span class="kode">{esc(report["kode"])}</span>'
            f'<span class="jam">{esc(date_short(report["wib"][:10]))} {esc(report["jam"])} WIB</span>'
            f'<span class="uuid">UUID {esc(report["uuid"])} · baris {num(report["row"])}</span>'
            "</div>"
        )

    if full:
        body = (
            '<div class="full-report">'
            + full_head("A", a)
            + f'<div class="full-text">{full_text(a)}</div>'
            + "</div>"
            + '<div class="full-report">'
            + full_head("B", b)
            + f'<div class="full-text">{full_text(b)}</div>'
            + "</div>"
        )
        return f"""
<div class="full-pair">
  <div class="pair-head">
    <div class="who">
      <strong>{esc(satker)}</strong>
      <span>{esc(unit_line)} · {esc(pelapor)}</span>
    </div>
    <div class="badge {badge_class}">{esc(badge)}</div>
  </div>
  <div class="pair-meta">
    <span>Tanggal kegiatan <b>{esc(date_id(tanggal))}</b></span>
    <span>Kemiripan <b>{pct(pair['ratio'])}</b></span>
    <span>Teks sama <b>{num(pair['matched'])} karakter</b></span>
    <span>Cakupan <b>{pct(pair['coverage'])}</b></span>
    {"<span><b>Isian identik</b></span>" if identical else ""}
  </div>
  {shared_html}
  {body}
</div>
"""

    return f"""
<div class="pair">
  <div class="pair-head">
    <div class="who">
      <strong>{esc(satker)}</strong>
      <span>{esc(unit_line)} · {esc(pelapor)}</span>
    </div>
    <div class="badge {badge_class}">{esc(badge)}</div>
  </div>
  <div class="pair-meta">
    <span>Tanggal kegiatan <b>{esc(date_id(tanggal))}</b></span>
    <span>Kemiripan <b>{pct(pair['ratio'])}</b></span>
    <span>Teks sama <b>{num(pair['matched'])} karakter</b></span>
    <span>Cakupan <b>{pct(pair['coverage'])}</b></span>
    {"<span><b>Isian identik</b></span>" if identical else ""}
  </div>
  <div class="pair-cols">{col('A', a)}{col('B', b)}</div>
  {shared_html}
</div>
"""


def section_a_html() -> str:
    blocks = []
    groups = group_by(PAIRS_A, lambda pair: (pair["polres"], pair["satuan"]))
    for (polres, satker), items in groups.items():
        items.sort(key=lambda pair: (pair["tanggal"], pair["a"]["ms"]))
        blocks.append(
            f'<div class="unit-head"><h3>{esc(satker)}</h3>'
            f"<span>{esc(polres or 'Polda Bali')} · {num(len(items))} pasangan · "
            f"{num(len({pair['pelapor'] for pair in items}))} pelapor · "
            f"{num(len(uuid_set(items)))} laporan</span></div>"
        )
        for pair in items:
            blocks.append(pair_block(pair, full=True))
    return "".join(blocks)


def section_b1_html() -> str:
    blocks = []
    groups = group_by(PAIRS_B1, lambda pair: (pair["polres"], pair["satuan"]))
    for (polres, satker), items in groups.items():
        items.sort(key=lambda pair: (pair["gap_s"], pair["tanggal"]))
        blocks.append(
            f'<div class="unit-head"><h3>{esc(satker)}</h3>'
            f"<span>{esc(polres or 'Polda Bali')} · {num(len(items))} pasangan · "
            f"{num(len({pair['pelapor'] for pair in items}))} pelapor · "
            f"{num(len(uuid_set(items)))} laporan</span></div>"
        )
        for pair in items:
            blocks.append(pair_block(pair, full=True))
    return "".join(blocks)


def b2_summary_rows() -> str:
    rows = []
    groups = Counter(pair["satuan"] for pair in PAIRS_B2)
    pelapor_by_unit = {}
    qw_by_unit = {}
    for pair in PAIRS_B2:
        pelapor_by_unit.setdefault(pair["satuan"], Counter())[pair["pelapor"]] += 1
        qw_by_unit.setdefault(pair["satuan"], Counter())[pair["a"]["kode"] + "→" + pair["b"]["kode"]] += 1
    for rank, (satker, count) in enumerate(groups.most_common(), start=1):
        pelapor, pcount = pelapor_by_unit[satker].most_common(1)[0]
        qw, qcount = qw_by_unit[satker].most_common(1)[0]
        polres = next(pair["polres"] for pair in PAIRS_B2 if pair["satuan"] == satker)
        rows.append(
            f'<tr class="{"rank-1" if rank == 1 else ""}">'
            f'<td class="num">{rank}</td>'
            f"<td><strong>{esc(satker)}</strong></td>"
            f"<td>{esc(polres or 'Polda Bali')}</td>"
            f'<td class="num">{num(count)}</td>'
            f'<td>{esc(pelapor)}<br><span class="sub">{num(pcount)} pasangan</span></td>'
            f'<td class="kode">{esc(qw)}</td>'
            f'<td class="num">{num(qcount)}</td>'
            "</tr>"
        )
    return "".join(rows)


def c_rows(limit=None, excerpt_limit: int = 70) -> str:
    rows = []
    source = GROUPS_C if limit is None else GROUPS_C[:limit]
    for rank, group in enumerate(source, start=1):
        rows.append(
            f'<tr class="{"rank-1" if rank == 1 else ""}">'
            f'<td class="num">{rank}</td>'
            f"<td><strong>{esc(group['satuan'])}</strong></td>"
            f"<td>{esc(group['polres'] or 'Polda Bali')}</td>"
            f"<td>{esc(group['pelapor'])}</td>"
            f'<td class="kode">{esc(group["kode"])}</td>'
            f'<td class="num"><strong>{num(group["count"])}×</strong></td>'
            f'<td class="num">{num(group["days"])}</td>'
            f'<td class="nowrap">{esc(date_range(group["dates"]))}</td>'
            f"<td>{esc(truncate(mask_sensitive(group['nar']), excerpt_limit))}</td>"
            "</tr>"
        )
    return "".join(rows)


def qw_patterns_html() -> str:
    combos = Counter()
    units = {}
    for pair in PAIRS_B1:
        key = (pair["a"]["kode"], pair["b"]["kode"])
        combos[key] += 1
        units.setdefault(key, set()).add(pair["satuan"])
    rows = []
    for rank, (combo, count) in enumerate(combos.most_common(15), start=1):
        labels = sorted(units[combo])
        shown = ", ".join(labels[:3]) + (f" +{len(labels) - 3}" if len(labels) > 3 else "")
        rows.append(
            "<tr>"
            f'<td class="num">{rank}</td>'
            f'<td class="kode">{esc(combo[0] + " → " + combo[1])}</td>'
            f'<td class="num"><strong>{num(count)}</strong></td>'
            f"<td>{esc(shown)}</td>"
            "</tr>"
        )
    return "".join(rows)


def qw_same_rows() -> str:
    counter = Counter(pair["a"]["kode"] for pair in PAIRS_A)
    rows = []
    for rank, (code, count) in enumerate(counter.most_common(), start=1):
        units = sorted({pair["satuan"] for pair in PAIRS_A if pair["a"]["kode"] == code})
        rows.append(
            "<tr>"
            f'<td class="num">{rank}</td>'
            f'<td class="kode">{esc(code)}</td>'
            f'<td class="num"><strong>{num(count)}</strong></td>'
            f"<td>{esc(', '.join(units))}</td>"
            "</tr>"
        )
    return "".join(rows)


def priority_rows() -> str:
    rows = []
    singaraja_b1 = [pair for pair in PAIRS_B1 if pair["satuan"] == "Polsek Singaraja"]
    singaraja_fast = sum(1 for pair in singaraja_b1 if pair["gap_s"] <= 600)
    priority = [
        (
            "Polsek Singaraja",
            "Polres Buleleng",
            f"B1: {num(len(singaraja_b1))} pasangan narasi kejadian sama; "
            f"{num(singaraja_fast)} pasangan ≤10 menit; kecelakaan, balap liar, judi, KDRT",
        ),
        ("Polsek Klungkung", "Polres Klungkung", "B2: 67 pasangan; satu pelapor menyalin narasi yang sama ke QW3.2 dan QW4.1–QW4.4"),
        ("Polda Bali", "Lingkup Polda", "A: 6 pasangan QW1.10 dalam 31–533 detik; isian identik"),
        ("Polsek Busungbiu", "Polres Buleleng", "A: 7 pasangan QW1.10 dalam 210–385 detik; isian identik"),
        ("Polsek Seririt", "Polres Buleleng", "A: 3 pasangan QW1.10 dalam 316–352 detik; isian identik"),
        ("Polsek Payangan", "Polres Gianyar", "B2: 16 pasangan narasi templat lintas QW"),
        ("Polsek Abang", "Polres Karangasem", "B2: 12 pasangan narasi templat lintas QW"),
        ("Polsek Banjarangkan", "Polres Klungkung", "B2: 8 pasangan narasi templat lintas QW"),
        ("Polsek Banjar", "Polres Buleleng", "C: 18 laporan QW1.10 identik dalam 14 hari"),
        ("Polres Tabanan", "Polres Tabanan", "C: 34 grup / 192 laporan isian berulang"),
    ]
    for rank, (satker, polres, note) in enumerate(priority, start=1):
        rows.append(
            "<tr>"
            f'<td class="num">{rank}</td>'
            f"<td><strong>{esc(satker)}</strong></td>"
            f"<td>{esc(polres)}</td>"
            f"<td>{esc(note)}</td>"
            "</tr>"
        )
    return "".join(rows)


def accident_html() -> str:
    if not ACCIDENT:
        return '<p class="small">Tidak ada pasangan kecelakaan lintas QW yang memenuhi ambang.</p>'
    blocks = []
    for pair in ACCIDENT:
        blocks.append(pair_block(pair, full=True))
    return "".join(blocks)


# ---------------------------------------------------------------------------
# CSS.
# ---------------------------------------------------------------------------
CSS = r"""
:root { --ink:#1f2a24; --forest:#143d30; --gold:#b8894b; --red:#a33a3a; --soft:#5c6a63; --muted:#7a857f; --line:#cfd8cf; --panel:#f5f8f3; --pale:#eef3ee; }
@page { size: A4 landscape; margin: 9mm 9mm 14mm; }
* { box-sizing: border-box; }
html, body { margin:0; padding:0; }
body { font-family:"Segoe UI", Arial, sans-serif; font-size:8.2pt; line-height:1.36; color:var(--ink); -webkit-print-color-adjust:exact; print-color-adjust:exact; }
h1,h2,h3,h4,p { margin:0; }
h1 { font-family:"Segoe UI Semibold","Segoe UI",Arial,sans-serif; font-size:24pt; line-height:1.07; color:#fff; }
h2 { font-size:14pt; color:var(--forest); margin:0 0 2mm; }
h3 { font-size:10.5pt; color:var(--forest); margin:0 0 1mm; }
h4 { font-size:9pt; color:var(--forest); margin:0 0 1mm; }
h2, h3, h4 { break-after:avoid; page-break-after:avoid; }
.mono { font-family:Consolas,"Courier New",monospace; }
.footer { display:none; }
.cover { page-break-after:always; background:linear-gradient(135deg,#0d2a21 0%,#143d30 58%,#1d4a3a 100%); color:#fff; padding:16mm 16mm 12mm; height:174mm; position:relative; overflow:hidden; }
.cover::after { content:""; position:absolute; right:-30mm; top:-30mm; width:110mm; height:110mm; border-radius:50%; background:rgba(184,137,75,.12); }
.cover .logo { width:17mm; height:auto; margin-bottom:6mm; }
.cover .kicker { font-size:8pt; font-weight:700; letter-spacing:.22em; color:#e6d3b3; text-transform:uppercase; }
.cover .sub { margin-top:4mm; font-size:11pt; color:#cfe0d6; max-width:190mm; }
.cover .criteria { margin-top:7mm; border:1pt solid rgba(255,255,255,.25); border-radius:2mm; padding:3.4mm 4mm; background:rgba(255,255,255,.06); font-size:8pt; color:#e3ece6; max-width:225mm; }
.cover .criteria strong { color:#e6d3b3; }
.cover .meta { position:absolute; left:16mm; bottom:13mm; right:16mm; display:flex; gap:5mm; font-size:7pt; color:#cfe0d6; }
.cover .meta div { border-top:1.4pt solid var(--gold); padding-top:2mm; min-width:36mm; }
.cover .meta strong { display:block; color:#fff; font-size:7.8pt; margin-bottom:.8mm; }
.cards { display:flex; gap:3mm; margin:3mm 0 4mm; }
.card { flex:1; border:.5pt solid var(--line); border-radius:2mm; background:var(--panel); padding:2.6mm 3mm; }
.card .label { display:block; font-size:6.4pt; font-weight:700; letter-spacing:.05em; text-transform:uppercase; color:var(--muted); }
.card strong { display:block; margin-top:.8mm; font-size:16pt; line-height:1; color:var(--forest); }
.card small { display:block; margin-top:1mm; font-size:6.5pt; color:var(--soft); }
.note { border-left:1.5mm solid var(--gold); background:#fbf5e6; padding:2.5mm 3mm; font-size:7.6pt; color:#5b4a35; margin:3mm 0; }
.note strong { color:#7b5a2f; }
.red-note { border-left:1.5mm solid var(--red); background:#fbf0ee; color:#6b2f2b; }
.section { page-break-before:always; }
table { width:100%; border-collapse:collapse; table-layout:fixed; }
th,td { border:.4pt solid var(--line); padding:1.05mm 1.25mm; vertical-align:top; }
th { background:var(--forest); color:#fff; font-size:6.6pt; font-weight:700; text-align:left; letter-spacing:.03em; text-transform:uppercase; }
tbody tr:nth-child(even) td { background:var(--panel); }
thead { display:table-header-group; }
tr { page-break-inside:avoid; }
.num { text-align:right; white-space:nowrap; }
.nowrap { white-space:nowrap; }
.rank-table { font-size:7.2pt; }
.rank-table td { padding:1.15mm 1.3mm; }
.rank-1 td { background:#fbf3e3 !important; }
.rank-1 strong { color:#8a5a1f; }
.sub { color:var(--muted); font-size:6.3pt; }
.kode { font-family:Consolas,monospace; font-weight:700; font-size:6.5pt; color:var(--forest); }
.small { font-size:7pt; color:var(--soft); }
ul.list { margin:1mm 0 0 4mm; padding:0; }
ul.list li { margin-bottom:1.2mm; }
.unit-head { margin:4mm 0 2mm; padding:2.1mm 3mm; background:var(--forest); color:#fff; border-radius:1.6mm; display:flex; justify-content:space-between; align-items:center; break-after:avoid; page-break-after:avoid; }
.unit-head h3 { color:#fff; margin:0; font-size:11.5pt; }
.unit-head span { font-size:7.2pt; color:#cfe0d6; }
.pair { border:.5pt solid var(--line); border-radius:2mm; margin:0 0 3.2mm; break-inside:avoid; page-break-inside:avoid; overflow:hidden; }
.full-pair { border:.5pt solid var(--line); border-radius:2mm; margin:0 0 3.2mm; }
.full-report { padding:1.6mm 3mm 2mm; border-top:.4pt solid var(--line); }
.full-report:first-of-type { border-top:none; }
.full-report-head { display:flex; flex-wrap:wrap; gap:1mm 4mm; align-items:center; font-size:6.8pt; color:var(--soft); break-after:avoid; page-break-after:avoid; }
.full-report-head .side { display:inline-block; width:3.4mm; height:3.4mm; line-height:3.4mm; text-align:center; border-radius:50%; background:var(--forest); color:#fff; font-weight:700; font-size:5.8pt; }
.full-report-head .jam { color:var(--ink); font-weight:700; }
.full-report-head .uuid { font-family:Consolas,monospace; font-size:5.6pt; color:var(--muted); }
.full-text { font-size:6.9pt; line-height:1.42; color:#35413b; margin-top:.9mm; overflow-wrap:anywhere; word-break:break-word; column-count:2; column-gap:6mm; }
.full-text .fl { margin:0 0 .5mm; break-inside:avoid; }
.pair-head { background:var(--pale); padding:2mm 3mm; border-bottom:.5pt solid var(--line); display:flex; justify-content:space-between; gap:4mm; break-after:avoid; page-break-after:avoid; }
.pair-head .who { min-width:0; }
.pair-head .who strong { font-size:9.4pt; color:var(--forest); }
.pair-head .who span { display:block; font-size:7pt; color:var(--soft); margin-top:.3mm; }
.badge { display:inline-block; border-radius:1.4mm; padding:.6mm 1.8mm; font-size:6.7pt; font-weight:700; }
.badge-red { background:#f6e2df; color:#8f2f28; }
.badge-gold { background:#f8eedb; color:#7b5a2f; }
.pair-meta { display:flex; flex-wrap:wrap; gap:1mm 5mm; padding:1.4mm 3mm; font-size:6.7pt; color:var(--soft); border-bottom:.4pt solid var(--line); }
.pair-meta b { color:var(--ink); }
.pair-cols { display:flex; gap:3mm; padding:2mm 3mm 1.4mm; }
.pair-col { flex:1; min-width:0; border-left:1.2mm solid var(--line); padding-left:2.2mm; }
.pair-col .col-head { font-size:6.8pt; color:var(--soft); margin-bottom:.5mm; }
.pair-col .side { display:inline-block; width:3.4mm; height:3.4mm; line-height:3.4mm; text-align:center; border-radius:50%; background:var(--forest); color:#fff; font-weight:700; font-size:5.8pt; margin-right:1mm; }
.pair-col .jam { color:var(--ink); font-weight:700; margin-left:2mm; }
.pair-col .uuid { font-family:Consolas,monospace; font-size:5.7pt; color:var(--muted); }
.pair-col .excerpt { font-size:6.7pt; color:#3d4943; margin-top:.8mm; }
.shared-box { margin:0 3mm 2.2mm; padding:1.6mm 2.4mm; background:#fbf5e6; border-left:1.2mm solid var(--gold); font-size:6.5pt; color:#5b4a35; }
.shared-box b { color:#7b5a2f; }
.bars { margin:2mm 0 4mm; }
.bar-row { display:flex; align-items:center; gap:2.5mm; margin-bottom:1.3mm; font-size:7pt; }
.bar-label { width:40mm; color:var(--soft); }
.bar-track { flex:1; display:block; height:3.2mm; background:#edf1ed; border-radius:1mm; overflow:hidden; }
.bar-fill { display:block; height:100%; background:var(--forest); }
.bar-fill.gold { background:var(--gold); }
.bar-value { width:20mm; text-align:right; color:var(--ink); font-weight:700; }
.two-col { display:flex; gap:5mm; }
.two-col > div { flex:1; }
.method td:first-child { width:24%; font-weight:700; color:var(--forest); }
.method td { font-size:7.2pt; }
.appendix { font-size:6.2pt; }
.appendix th { font-size:6pt; padding:.75mm .8mm; }
.appendix td { padding:.55mm .75mm; }
.compact { font-size:6.5pt; }
.compact th { font-size:6.2pt; padding:.7mm .9mm; }
.compact td { padding:.7mm .9mm; }
"""


def bar_rows(counter: Counter, total: int, gold: bool = False) -> str:
    rows = []
    for label, value in counter.most_common():
        width = value / total * 100 if total else 0
        rows.append(
            f'<div class="bar-row"><span class="bar-label">{esc(label)}</span>'
            f'<span class="bar-track"><span class="bar-fill{" gold" if gold else ""}" '
            f'style="width:{width:.1f}%"></span></span>'
            f'<span class="bar-value">{num(value)}</span></div>'
        )
    return "".join(rows)


# ---------------------------------------------------------------------------
# Narasi ringkas.
# ---------------------------------------------------------------------------
top_a = sorted(PAIRS_A, key=lambda pair: (pair["gap_s"], -pair["matched"]))[0]
top_b1_fast = sorted(PAIRS_B1, key=lambda pair: pair["gap_s"])[0]
accident_pair = ACCIDENT[0] if ACCIDENT else None
b2_top = Counter(pair["satuan"] for pair in PAIRS_B2).most_common(1)[0]
c_top = GROUPS_C[0]

SUMMARY_HTML = f"""
<p class="small">Polda Bali · 11–24 November 2026 · Sumber <strong>{esc(META['source'])}</strong></p>
<h1>Analisis Duplikasi<br>Isian Laporan</h1>
<p class="sub">Pemeriksaan {num(TOTAL_REPORTS)} laporan unik berbasis UUID v7 untuk menemukan pengiriman ganda,
narasi kejadian yang disalin ke beberapa Quick Wins, dan isian yang tidak berubah lintas hari.</p>
<div class="criteria">
  <strong>Empat pola yang diperiksa</strong><br>
  <b>A.</b> QW sama, satker + pelapor + tanggal sama, jeda ≤10 menit, isian identik atau sangat mirip.<br>
  <b>B1.</b> QW berbeda, narasi kejadian sama (memuat tanggal/tempat), jeda ≤6 jam.<br>
  <b>B2.</b> QW berbeda, narasi templat/generik sama, jeda ≤6 jam.<br>
  <b>C.</b> Isian identik diulang pada QW yang sama selama ≥3 hari berbeda.
</div>
"""


# ---------------------------------------------------------------------------
# Bangun HTML.
# ---------------------------------------------------------------------------
polres_direct_bars = bar_rows(Counter({polres_name(row): row["direct_reports"] for row in DIRECT_POLRES}), max(row["direct_reports"] for row in DIRECT_POLRES))
satker_direct_bars = bar_rows(Counter({row["key"]: row["direct_reports"] for row in DIRECT_SATKER[:12]}), DIRECT_SATKER[0]["direct_reports"])
polres_c_bars = bar_rows(Counter({polres_name(row): row["c_report_count"] for row in C_POLRES}), max(row["c_report_count"] for row in C_POLRES), gold=True)

def appendix_b2_rows() -> str:
    rows = []
    for rank, pair in enumerate(PAIRS_B2, start=1):
        rows.append(
            "<tr>"
            f'<td class="num">{rank}</td>'
            f"<td>{esc(pair['satuan'])}</td>"
            f"<td>{esc(pair['pelapor'])}</td>"
            f'<td class="nowrap">{esc(date_id(pair["tanggal"]))}</td>'
            f'<td class="kode">{esc(pair["a"]["kode"] + "→" + pair["b"]["kode"])}</td>'
            f'<td class="num nowrap">{esc(gap_text(pair["gap_s"]))}</td>'
            f'<td class="num">{num(pair["matched"])}</td>'
            f'<td class="num">{pct(pair["ratio"])}</td>'
            f"<td>{esc(shared_display(pair, 70))}</td>"
            "</tr>"
        )
    return "".join(rows)


LOGO_URI = "file:///" + os.path.abspath("logo_polda_bali.png").replace("\\", "/")

HTML = f"""<!DOCTYPE html>
<html lang="id">
<head>
<meta charset="utf-8">
<title>{esc(TITLE)} — {esc(SUBTITLE)}</title>
<style>{CSS}</style>
</head>
<body>

<div class="cover">
  <img class="logo" src="{esc(LOGO_URI)}" alt="Polda Bali">
  <div class="kicker">Pemeriksaan Integritas Data Laporan</div>
  <h1>Analisis Duplikasi<br>Isian Laporan</h1>
  <p class="sub">Polda Bali · 11–24 November 2026<br>
  {num(TOTAL_REPORTS)} laporan unik · 10 polres · 72 satker · 1.001 pelapor</p>
  <div class="criteria">
    <strong>Dasar pemeriksaan</strong><br>
    Waktu laporan diambil dari 48 bit pertama UUID v7 (epoch milidetik UTC) lalu dikonversi ke WIB.
    Isian dinormalisasi dengan membuang label formulir, jawaban generik, URL, koordinat, dan kode QW.
    Kemiripan diukur dengan <i>SequenceMatcher</i> dan jumlah karakter yang benar-benar sama.
  </div>
  <div class="meta">
    <div><strong>Sumber data</strong>{esc(META['source'])}</div>
    <div><strong>Periode</strong>11–24 November 2026</div>
    <div><strong>Basis waktu</strong>UUID v7 → WIB (UTC+7)</div>
    <div><strong>Disusun</strong>26 November 2026</div>
  </div>
</div>

<h2>1. Ringkasan Eksekutif</h2>
<div class="small">Cakupan: {num(TOTAL_REPORTS)} laporan unik · {num(TOTAL_PAIRS)} pasangan duplikasi langsung · {num(len(GROUPS_C))} grup isian berulang</div>
{cards_html()}

<p>Pemeriksaan menemukan <strong>{num(TOTAL_PAIRS)} pasangan laporan</strong> yang memuat isian sama atau sangat mirip dalam rentang waktu dekat,
serta <strong>{num(len(GROUPS_C))} grup isian identik</strong> yang diulang pada hari berbeda. Secara keseluruhan,
<strong>{num(ALL_REPORTS)} laporan</strong> ({pct(ALL_REPORTS / TOTAL_REPORTS)}) terlibat dalam sedikitnya satu pola duplikasi.
Dari jumlah itu, <strong>{num(DIRECT_REPORTS)} laporan</strong> berada pada pasangan yang berdekatan (Kategori A/B), sedangkan
<strong>{num(C_REPORTS)} laporan</strong> merupakan isian yang diulang lintas hari (Kategori C).</p>

<div class="note">
  <strong>Temuan terkuat.</strong>
  {num(len(PAIRS_A))} pasangan dikirim pada QW yang sama dalam ≤10 menit; {num(sum(1 for pair in PAIRS_A if is_identical(pair)))} di antaranya berisi isian 100% identik.
  Seluruhnya berada pada QW1.10 dengan dua sumber utama: Polda Bali (6 pasangan) dan Polsek Busungbiu (7 pasangan).
  Pada Kategori B1, Polsek Singaraja mendominasi dengan 24 dari 32 pasangan, termasuk narasi kecelakaan yang sama
  pada QW4.2 dan QW4.3 dengan jeda {esc(gap_text(accident_pair['gap_s']) if accident_pair else '—')}.
</div>

<h3>Temuan paling menonjol</h3>
<table class="rank-table">
  <colgroup>
    <col style="width:4%"><col style="width:18%"><col style="width:18%"><col style="width:9%">
    <col style="width:11%"><col style="width:9%"><col style="width:8%"><col style="width:23%">
  </colgroup>
  <thead><tr>
    <th class="num">No</th><th>Satker</th><th>Pelapor</th><th>Tgl. kegiatan</th>
    <th>QW</th><th class="num">Jeda</th><th class="num">Skor</th><th>Catatan</th>
  </tr></thead>
  <tbody>{top_findings_rows()}</tbody>
</table>

<div class="note red-note">
  <strong>Batas tafsir.</strong> Temuan ini adalah <strong>indikasi duplikasi isian</strong>, bukan bukti bot atau bukti perangkat tertentu.
  Data tidak memuat IP, <i>user agent</i>, atau identitas perangkat. Isian yang sama dapat berasal dari:
  pengiriman ganda karena gangguan jaringan/aplikasi, salin-tempel antar-QW, kegiatan rutin yang memang berulang,
  atau isian yang tidak diperbarui. Setiap temuan perlu diverifikasi dengan log aplikasi dan klarifikasi satker.
</div>

<h2 class="section">2. Metode dan Batasan</h2>
<table class="method">
  <colgroup><col style="width:22%"><col></colgroup>
  <tbody>
    <tr><td>Sumber</td><td><strong>{esc(META['source'])}</strong>, sheet <i>Data</i>. Setelah duplikasi baris dengan UUID yang sama digabung,
      terdapat <strong>{num(TOTAL_REPORTS)} laporan unik</strong> dengan UUID v7 valid.</td></tr>
    <tr><td>Waktu laporan</td><td>48 bit pertama UUID v7 adalah epoch milidetik UTC. Nilai tersebut dikonversi ke <strong>WIB (UTC+7)</strong>.
      Contoh: <span class="mono">019a...&nbsp;→&nbsp;2026-11-18 11:00:44.441 WIB</span>. Kolom “Tanggal lapor (WIB)” tidak dipakai sebagai dasar jeda.</td></tr>
    <tr><td>Normalisasi isian</td><td>Label formulir (“a. Pelaksana”, “b. Kegiatan yang dilaksanakan”, “c. Lokasi”),
      jawaban generik (“Ya”, “Tidak”, “Nihil”, “0”), URL, koordinat, dan kode QW dibuang.
      Yang dibandingkan adalah <strong>inti narasi</strong>: uraian kejadian, rute, kasus, saran, dan tindak lanjut.</td></tr>
    <tr><td>Kemiripan</td><td><i>SequenceMatcher</i> menghasilkan rasio kemiripan, jumlah karakter yang sama,
      dan cakupan (karakter sama ÷ teks terpendek). Blok yang lebih pendek dari 30 karakter diabaikan.</td></tr>
    <tr><td>Kategori A</td><td>Satker + pelapor + tanggal + <strong>kode QW sama</strong>, jeda ≤600 detik,
      rasio ≥0,85 <i>atau</i> karakter sama ≥100.</td></tr>
    <tr><td>Kategori B</td><td>Satker + pelapor + tanggal sama, <strong>kode QW berbeda</strong>, jeda ≤6 jam,
      karakter sama ≥300 dan cakupan ≥0,50. <strong>B1</strong> jika bagian yang sama memuat unsur tanggal dan tempat;
      <strong>B2</strong> jika tidak (narasi templat/generik).</td></tr>
    <tr><td>Kategori C</td><td>Satker + pelapor + kode QW sama, narasi <strong>identik</strong> (≥80 karakter),
      muncul pada <strong>≥3 hari berbeda</strong> dalam periode.</td></tr>
    <tr><td>Tanpa duplikasi baris</td><td>Satu UUID = satu laporan. Baris ganda dengan UUID sama tidak dihitung sebagai dua laporan.</td></tr>
  </tbody>
</table>

<div class="two-col" style="margin-top:4mm">
  <div>
    <h3>Batasan yang perlu dibaca</h3>
    <ul class="list">
      <li>UUID v7 merekam waktu <i>record dibuat</i>, bukan waktu klik tombol kirim. Jeda antarlaporan karena itu adalah jeda pembuatan record.</li>
      <li>Narasi yang sama belum tentu laporan ganda: rute patroli harian, jawaban “nihil”, dan kegiatan rutin lain dapat memang sama setiap hari.</li>
      <li>Kategori C sengaja dipisahkan dari A/B karena sifatnya lintas hari; kelompok ini menunjukkan isian yang tidak diperbarui, bukan pengiriman beruntun.</li>
      <li>Kategori B hanya menandai <strong>bagian narasi yang sama</strong>. Pada laporan yang memuat beberapa kasus, satu kasus dapat sama sementara kasus lain berbeda.</li>
      <li>Tidak ada klaim tentang niat, pelanggaran, atau penggunaan bot. Diperlukan verifikasi lanjutan oleh Itwasda/Propam dan pengelola aplikasi.</li>
    </ul>
  </div>
  <div>
    <h3>Ambang yang dipakai</h3>
    <table class="rank-table">
      <thead><tr><th>Pola</th><th class="num">Jeda</th><th class="num">Ambang</th><th class="num">Temuan</th></tr></thead>
      <tbody>
        <tr><td>A · QW sama</td><td class="num">≤10 mnt</td><td class="num">rasio ≥0,85</td><td class="num"><strong>{num(len(PAIRS_A))} psg</strong></td></tr>
        <tr><td>B1 · Kejadian</td><td class="num">≤6 jam</td><td class="num">≥300 kar.</td><td class="num"><strong>{num(len(PAIRS_B1))} psg</strong></td></tr>
        <tr><td>B2 · Templat</td><td class="num">≤6 jam</td><td class="num">≥300 kar.</td><td class="num"><strong>{num(len(PAIRS_B2))} psg</strong></td></tr>
        <tr><td>C · Lintas hari</td><td class="num">≥3 hari</td><td class="num">identik</td><td class="num"><strong>{num(len(GROUPS_C))} grup</strong></td></tr>
      </tbody>
    </table>
    <p class="small" style="margin-top:2mm">Dari {num(len(PAIRS_B1))} pasangan B1, {num(FAST_B1)} terjadi ≤10 menit.
    Dari {num(len(PAIRS_B2))} pasangan B2, {num(FAST_B2)} terjadi ≤10 menit.</p>
  </div>
</div>

<h2 class="section">3. Peringkat per Polres, Satker, dan Pelapor</h2>
<p class="small">Peringkat langsung dihitung dari laporan unik yang terlibat pada Kategori A, B1, dan B2.
Peringkat isian berulang dihitung terpisah dari Kategori C karena sifatnya lintas hari.</p>

<h3>3.1 Polres — duplikasi langsung (A + B1 + B2)</h3>
<table class="rank-table">
  <colgroup>
    <col style="width:4%"><col style="width:22%"><col style="width:8%"><col style="width:8%"><col style="width:8%">
    <col style="width:10%"><col style="width:10%"><col style="width:12%">
  </colgroup>
  <thead><tr>
    <th class="num">Rank</th><th>Polres</th><th class="num">A</th><th class="num">B1</th><th class="num">B2</th>
    <th class="num">Laporan A+B</th><th class="num">Grup C</th><th class="num">Laporan C</th>
  </tr></thead>
  <tbody>{ranking_polres_rows('direct')}</tbody>
</table>

<h3 style="margin-top:4mm">3.2 Polres — isian berulang lintas hari (C)</h3>
<div class="bars">{polres_c_bars}</div>
<table class="rank-table">
  <colgroup>
    <col style="width:4%"><col style="width:26%"><col style="width:8%"><col style="width:8%"><col style="width:8%">
    <col style="width:10%"><col style="width:10%"><col style="width:12%">
  </colgroup>
  <thead><tr>
    <th class="num">Rank</th><th>Polres</th><th class="num">A</th><th class="num">B1</th><th class="num">B2</th>
    <th class="num">Laporan A+B</th><th class="num">Grup C</th><th class="num">Laporan C</th>
  </tr></thead>
  <tbody>{ranking_polres_rows('c')}</tbody>
</table>

<h3 style="margin-top:4mm">3.3 Satker/Polsek — duplikasi langsung</h3>
<div class="bars">{satker_direct_bars}</div>
<table class="rank-table">
  <colgroup>
    <col style="width:4%"><col style="width:17%"><col style="width:14%"><col style="width:6%"><col style="width:6%">
    <col style="width:6%"><col style="width:9%"><col style="width:8%"><col style="width:10%">
  </colgroup>
  <thead><tr>
    <th class="num">Rank</th><th>Satker</th><th>Polres</th><th class="num">A</th><th class="num">B1</th><th class="num">B2</th>
    <th class="num">Laporan A+B</th><th class="num">Grup C</th><th class="num">Laporan C</th>
  </tr></thead>
  <tbody>{ranking_satker_rows('direct')}</tbody>
</table>

<h3 style="margin-top:4mm">3.4 Pelapor — duplikasi langsung (25 teratas)</h3>
<table class="rank-table">
  <colgroup>
    <col style="width:4%"><col style="width:17%"><col style="width:15%"><col style="width:12%"><col style="width:5%">
    <col style="width:5%"><col style="width:5%"><col style="width:9%"><col style="width:7%"><col style="width:9%">
  </colgroup>
  <thead><tr>
    <th class="num">Rank</th><th>Pelapor</th><th>Satker</th><th>Polres</th><th class="num">A</th><th class="num">B1</th>
    <th class="num">B2</th><th class="num">Laporan A+B</th><th class="num">Grup C</th><th class="num">Laporan C</th>
  </tr></thead>
  <tbody>{ranking_pelapor_rows('direct')}</tbody>
</table>

<h2 class="section">4. Kategori A — Pengiriman Ganda pada QW yang Sama (≤10 Menit)</h2>
<p>Kategori ini paling mendekati pola yang Anda contohkan: satker dan pelapor yang sama mengirim isian yang sama
pada QW yang sama dalam hitungan menit. Dari {num(TOTAL_REPORTS)} laporan, ditemukan <strong>{num(len(PAIRS_A))} pasangan</strong>
yang memenuhi ambang, seluruhnya pada <strong>QW1.10</strong>. Isian yang berulang adalah rute patroli dan alasan pemilihan rute.
Isian kedua laporan ditampilkan <strong>lengkap</strong> di bawah ini.</p>
<div class="note">
  <strong>Contoh tercepat.</strong> {esc(top_a['satuan'])} — {esc(top_a['pelapor'])}, {esc(date_id(top_a['tanggal']))},
  QW1.10 pukul {esc(top_a['a']['jam'])} WIB lalu {esc(top_a['b']['jam'])} WIB: jeda {esc(gap_text(top_a['gap_s']))},
  isian identik {num(top_a['matched'])} karakter.
</div>
{section_a_html()}

<h2 class="section">5. Kategori B1 — Narasi Kejadian yang Sama pada QW Berbeda</h2>
<p>Kategori ini menjawab pola kedua: isian tentang suatu kejadian dilaporkan pada satu QW, lalu bagian yang sama
muncul pada QW lain. Ditemukan <strong>{num(len(PAIRS_B1))} pasangan</strong> pada {num(len({pair['satuan'] for pair in PAIRS_B1}))} satker;
<strong>{num(FAST_B1)} pasangan terjadi ≤10 menit</strong>. Isian lengkap ditampilkan pada contoh 5.1 dan seluruh pasangan 5.2.</p>
<div class="note">
  <strong>Catatan tentang contoh QW5.3 → QW4.2.</strong> Pola persis “kecelakaan pada QW5.3 lalu diulang pada QW4.2”
  tidak ditemukan pada data Bali. Dari {num(QW53_PAIRS)} pasangan QW5.3–QW4.2 yang berjarak ≤10 menit,
  kemiripan tertinggi hanya {pct(QW53_MAX)} dan {num(QW53_ACCIDENT)} pasangan yang sama-sama memuat kata kecelakaan.
  Yang ditemukan adalah pola setara: <strong>kecelakaan pada QW4.2 diulang pada QW4.3</strong>
  di Polsek Singaraja, 18 November 2026, jeda <strong>{esc(gap_text(accident_pair['gap_s']) if accident_pair else '—')}</strong>,
  dengan narasi “kecelakaan lalu lintas di depan hotel Melka, Desa Kalibukbuk” yang sama.
</div>

<h3>5.1 Contoh narasi kecelakaan lintas QW</h3>
{accident_html()}

<h3 style="margin-top:4mm">5.2 Seluruh pasangan B1</h3>
{section_b1_html()}

<h2 class="section">6. Kategori B2 — Narasi Templat yang Sama pada QW Berbeda</h2>
<p>Kategori ini memuat <strong>{num(len(PAIRS_B2))} pasangan</strong> pada {num(len({pair['satuan'] for pair in PAIRS_B2}))} satker.
Sebagian besar merupakan jawaban formulir yang sama (misalnya “belum ada saran atau keluhan”) yang disalin ke beberapa QW.
Pola ini tidak selalu berarti laporan ganda, tetapi menunjukkan isian yang tidak spesifik dan berpotensi mengaburkan penilaian.
Kategori B2 dan C ditampilkan dalam bentuk ringkas/cuplikan karena narasinya bersifat templat atau kelompok;
isian lengkap per laporan ada pada pasangan A dan B1 di bagian 4–5.</p>
<div class="note">
  <strong>Konsentrasi terbesar.</strong> {esc(b2_top[0])} menyumbang {num(b2_top[1])} pasangan.
  Satu pelapor menyalin narasi yang sama ke QW3.2 dan QW4.1–QW4.4 pada hari yang sama.
</div>
<table class="rank-table">
  <colgroup>
    <col style="width:4%"><col style="width:18%"><col style="width:15%"><col style="width:7%"><col style="width:25%"><col style="width:10%"><col style="width:6%">
  </colgroup>
  <thead><tr>
    <th class="num">Rank</th><th>Satker</th><th>Polres</th><th class="num">Pasangan</th>
    <th>Pelapor dominan</th><th>Pasangan QW</th><th class="num">Jml</th>
  </tr></thead>
  <tbody>{b2_summary_rows()}</tbody>
</table>
<p class="small" style="margin-top:2mm">Daftar lengkap 120 pasangan B2 ada pada Lampiran B.</p>

<h2 class="section">7. Kategori C — Isian Identik yang Diulang Lintas Hari</h2>
<p>Kategori ini berbeda dari A/B: bukan pengiriman dalam hitungan menit, melainkan <strong>narasi yang sama persis</strong>
digunakan pada hari berbeda. Ditemukan <strong>{num(len(GROUPS_C))} grup</strong> dengan ≥3 hari dan <strong>{num(C_REPORTS)} laporan</strong>
({pct(C_REPORTS / TOTAL_REPORTS)} dari seluruh laporan).</p>
<div class="note">
  <strong>Interpretasi.</strong> Untuk QW1.10 (patroli), rute harian yang sama dapat wajar. Namun untuk QW lain,
  teks yang identik selama 14 hari menunjukkan isian tidak diperbarui dan kegiatan yang dilaporkan tidak dapat diverifikasi.
  Kelompok terbesar: {esc(c_top['satuan'])} — {esc(c_top['pelapor'])}, {esc(c_top['kode'])}, {num(c_top['count'])} laporan
  identik dalam {num(c_top['days'])} hari.
</div>
<table class="rank-table compact">
  <colgroup>
    <col style="width:4%"><col style="width:17%"><col style="width:13%"><col style="width:19%"><col style="width:7%">
    <col style="width:7%"><col style="width:6%"><col style="width:13%"><col style="width:14%">
  </colgroup>
  <thead><tr>
    <th class="num">Rank</th><th>Satker</th><th>Polres</th><th>Pelapor</th><th>QW</th>
    <th class="num">Jml</th><th class="num">Hari</th><th>Rentang</th><th>Cuplikan isian</th>
  </tr></thead>
  <tbody>{c_rows(40, excerpt_limit=55)}</tbody>
</table>
<p class="small" style="margin-top:2mm">40 grup teratas ditampilkan. Daftar lengkap {num(len(GROUPS_C))} grup ada pada Lampiran A.</p>

<h2 class="section">8. Pola QW yang Berulang</h2>
<div class="two-col">
  <div>
    <h3>8.1 Pasangan QW pada Kategori A</h3>
    <table class="rank-table">
      <thead><tr><th class="num">Rank</th><th>QW</th><th class="num">Pasangan</th><th>Satker</th></tr></thead>
      <tbody>{qw_same_rows()}</tbody>
    </table>
    <p class="small">Seluruh 18 pasangan Kategori A berada pada QW1.10. Tidak ada QW lain yang memenuhi ambang
    “QW sama + ≤10 menit + isian hampir identik”.</p>
  </div>
  <div>
    <h3>8.2 Pasangan QW pada Kategori B1</h3>
    <table class="rank-table">
      <thead><tr><th class="num">Rank</th><th>Arah QW</th><th class="num">Pasangan</th><th>Satker</th></tr></thead>
      <tbody>{qw_patterns_html()}</tbody>
    </table>
    <p class="small">Arah QW4.2 ↔ QW4.3 dan QW7.3 → QW4.x mendominasi. Pola ini konsisten dengan satu kejadian
    yang dicatat pada beberapa kegiatan: penerimaan laporan SPKT (QW4.2), respons 110 (QW4.3), dan Polsek siaga (QW7.3).</p>
  </div>
</div>

<h2 class="section">9. Kesimpulan dan Tindak Lanjut</h2>
<h3>9.1 Kesimpulan</h3>
<ol style="margin:1mm 0 0 4mm; padding:0; font-size:8pt">
  <li><strong>Duplikasi langsung terkonsentrasi pada tiga pola.</strong> Kategori A seluruhnya QW1.10;
      Kategori B1 didominasi QW4.2 ↔ QW4.3 dan QW7.3 → QW4.x; Kategori B2 didominasi QW4.1–QW4.4.</li>
  <li><strong>Polsek Singaraja menjadi titik terbesar B1.</strong> {num(sum(1 for pair in PAIRS_B1 if pair['satuan'] == 'Polsek Singaraja'))} dari
      {num(len(PAIRS_B1))} pasangan B1 berasal dari satu pelapor, dengan pola berulang 11–23 November 2026
      (kecelakaan, balap liar, judi, KDRT, pembakaran sampah).</li>
  <li><strong>Polsek Klungkung menjadi titik terbesar B2.</strong> {num(b2_top[1])} pasangan berasal dari satu pelapor yang menyalin
      narasi yang sama ke beberapa QW pada hari yang sama.</li>
  <li><strong>Polda Bali dan Polsek Busungbiu menunjukkan pengiriman ganda A.</strong> Enam pasangan Polda Bali dan tujuh pasangan
      Busungbiu memiliki isian 100% identik dengan jeda {num(min(pair["gap_s"] for pair in PAIRS_A if pair["satuan"] in ("Polda Bali", "Polsek Busungbiu")))}–{num(max(pair["gap_s"] for pair in PAIRS_A if pair["satuan"] in ("Polda Bali", "Polsek Busungbiu")))} detik.</li>
  <li><strong>Isian berulang lintas hari lebih luas.</strong> {num(len(GROUPS_C))} grup / {num(C_REPORTS)} laporan
      ({pct(C_REPORTS / TOTAL_REPORTS)}) menggunakan narasi yang sama pada ≥3 hari; ini masalah kualitas isian,
      bukan sekadar pengiriman ganda.</li>
</ol>

<h3 style="margin-top:4mm">9.2 Prioritas verifikasi</h3>
<table class="rank-table">
  <colgroup><col style="width:5%"><col style="width:20%"><col style="width:17%"><col></colgroup>
  <thead><tr><th class="num">No</th><th>Satker</th><th>Polres</th><th>Temuan dan langkah yang disarankan</th></tr></thead>
  <tbody>{priority_rows()}</tbody>
</table>

<h3 style="margin-top:4mm">9.3 Langkah tindak lanjut</h3>
<div class="two-col">
  <div>
    <ul class="list">
      <li><strong>Verifikasi log aplikasi.</strong> Cocokkan UUID v7 yang berdekatan dengan log request, sesi, dan perangkat.
        Jeda 31–63 detik pada isian identik perlu dipastikan apakah pengiriman ganda atau input ulang.</li>
      <li><strong>Klarifikasi satker.</strong> Minta penjelasan tertulis untuk pasangan dengan jeda ≤10 menit dan isian identik,
        khususnya Polsek Singaraja, Polda Bali, Polsek Busungbiu, dan Polsek Seririt.</li>
      <li><strong>Perbaiki validasi aplikasi.</strong> Tolak atau beri peringatan bila dalam 10 menit ada laporan dengan
        satker + pelapor + QW + tanggal yang sama dan isian sangat mirip.</li>
    </ul>
  </div>
  <div>
    <ul class="list">
      <li><strong>Wajibkan narasi spesifik.</strong> Untuk QW4.x, QW5.x, dan QW7.x, jawaban generik seperti
        “nihil”, “tidak ada saran”, atau “belum ada” sebaiknya tidak dapat dikirim tanpa uraian.</li>
      <li><strong>Beri tanda pada isian berulang.</strong> Tampilkan peringatan bila narasi yang diketik sama dengan
        laporan sebelumnya pada QW yang sama.</li>
      <li><strong>Audit berkala.</strong> Jalankan pemeriksaan ini setiap minggu; kelompok C dapat dipakai sebagai
        indikator kualitas isian per satker.</li>
    </ul>
  </div>
</div>

<div class="note" style="margin-top:5mm">
  <strong>Kesimpulan satu paragraf.</strong> Data Bali 11–24 November 2026 menunjukkan {num(TOTAL_PAIRS)} pasangan duplikasi langsung
  dan {num(len(GROUPS_C))} grup isian berulang. Yang paling kuat adalah QW1.10 di Polda Bali dan Polsek Busungbiu
  (isian 100% identik dalam hitungan menit), serta Polsek Singaraja pada QW4.2 ↔ QW4.3 (narasi kecelakaan/balap liar
  yang sama). Namun tidak ada bukti perangkat atau bot dalam data ini; seluruh temuan perlu diverifikasi melalui log aplikasi
  dan klarifikasi satker sebelum diambil kesimpulan disiplin.
</div>

<h2 class="section">Lampiran A — Daftar Lengkap Kategori C ({num(len(GROUPS_C))} Grup)</h2>
<p class="small">Narasi identik ≥80 karakter, diulang pada ≥3 hari berbeda. Diurutkan berdasarkan jumlah pengulangan.</p>
<table class="rank-table appendix">
  <colgroup>
    <col style="width:3%"><col style="width:15%"><col style="width:12%"><col style="width:19%"><col style="width:6%">
    <col style="width:4%"><col style="width:4%"><col style="width:12%"><col style="width:25%">
  </colgroup>
  <thead><tr>
    <th class="num">No</th><th>Satker</th><th>Polres</th><th>Pelapor</th><th>QW</th>
    <th class="num">Jml</th><th class="num">Hari</th><th>Rentang</th><th>Cuplikan isian</th>
  </tr></thead>
  <tbody>{c_rows(excerpt_limit=55)}</tbody>
</table>

<h2 class="section">Lampiran B — Daftar Lengkap Kategori B2 ({num(len(PAIRS_B2))} Pasangan)</h2>
<p class="small">Pasangan QW berbeda dengan narasi templat yang sama (tanpa unsur tanggal + tempat), jeda ≤6 jam.</p>
<table class="rank-table appendix">
  <colgroup>
    <col style="width:3%"><col style="width:14%"><col style="width:16%"><col style="width:8%"><col style="width:9%">
    <col style="width:8%"><col style="width:6%"><col style="width:7%"><col style="width:29%">
  </colgroup>
  <thead><tr>
    <th class="num">No</th><th>Satker</th><th>Pelapor</th><th>Tanggal</th><th>QW</th>
    <th class="num">Jeda</th><th class="num">Kar. sama</th><th class="num">Kemiripan</th><th>Cuplikan sama</th>
  </tr></thead>
  <tbody>{appendix_b2_rows()}</tbody>
</table>

</body>
</html>
"""

# ---------------------------------------------------------------------------
# Tulis HTML dan cetak PDF.
# ---------------------------------------------------------------------------
os.makedirs(os.path.dirname(OUT_HTML), exist_ok=True)
with open(OUT_HTML, "w", encoding="utf-8") as handle:
    handle.write(HTML)

print("HTML:", OUT_HTML, len(HTML), "bytes")

if os.path.exists(CHROME):
    args = [
        CHROME,
        "--headless=new",
        "--disable-gpu",
        "--no-pdf-header-footer",
        "--run-all-compositor-stages-before-draw",
        "--virtual-time-budget=12000",
        f"--print-to-pdf={os.path.abspath(OUT_PDF)}",
        "file:///" + os.path.abspath(OUT_HTML).replace("\\", "/"),
    ]
    subprocess.run(args, check=True)
    print("PDF:", OUT_PDF, os.path.getsize(OUT_PDF), "bytes")
else:
    print("Chrome tidak ditemukan:", CHROME)
