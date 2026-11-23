import html
import json
import math
import re
import statistics
import subprocess
import uuid
from collections import Counter, defaultdict
from datetime import datetime, timedelta, timezone
from pathlib import Path

BASE = Path(r"F:\app_lap\data\tmp_bot_nov")
RESULTS = BASE / "uuid_results.json"
PATTERN = BASE / "pattern_stats.json"
QW5_ALL = BASE / "qw5_all.json"
ROWS6 = BASE / "rows6.json"
HTML_PATH = BASE / "uuid_report6.html"
PDF_PATH = Path(r"F:\app_lap\Memo-Analisis-UUID-Bot-6-Polres-Jateng.pdf")
CHROME = Path(r"C:\Program Files\Google\Chrome\Application\chrome.exe")
CORE = ["Polres Temanggung", "Polres Rembang", "Polres Karanganyar"]
EXTRA = ["Polres Purbalingga", "Polresta Surakarta", "Polresta Batang"]
FOCUS = CORE + EXTRA
WIB = timezone(timedelta(hours=7))


def esc(value):
    return html.escape(str(value if value is not None else ""), quote=True)


def num(value):
    return f"{int(round(value)):,}".replace(",", ".")


def pct(value, digits=2):
    return f"{value:.{digits}f}".replace(".", ",") + "%"


def short_name(name):
    return name.replace("Polres ", "")


def color_for(name):
    if name in CORE:
        return "#a33a3a"
    if name in EXTRA:
        return "#c08a1e"
    return "#b7c3ba"


data = json.load(open(RESULTS, encoding="utf-8"))
stats = data["stats_period"]
focus = data["focus"]
pattern = json.load(open(PATTERN, encoding="utf-8"))
qw5_rows = json.load(open(QW5_ALL, encoding="utf-8"))
rows6 = json.load(open(ROWS6, encoding="utf-8"))


def uuid_ms(value):
    try:
        parsed = uuid.UUID(value)
    except (ValueError, AttributeError):
        return None
    if parsed.version != 7:
        return None
    return parsed.int >> 80


def stamp(value):
    ms = uuid_ms(value)
    if ms is None:
        return "—"
    return datetime.fromtimestamp(ms / 1000, WIB).strftime("%d %b %Y %H:%M:%S.%f")[:-3] + " WIB"


def qw5_rate(name):
    rows = [row for row in qw5_rows if row["polres"] == name]
    hits = sum(1 for row in rows if "penyidikan perkara?: Tidak" in (row["isian"] or ""))
    return hits / len(rows) * 100 if rows else 0.0


def svg_hbar(items, max_value, value_fmt, threshold=None, width=760, row_h=15, label_w=190, right=70, top=26):
    height = top + len(items) * row_h + 22
    plot_w = width - label_w - right
    parts = [
        f'<svg viewBox="0 0 {width} {height}" width="100%" xmlns="http://www.w3.org/2000/svg" font-family="Segoe UI, Arial, sans-serif">'
    ]
    for fraction in (0, 0.25, 0.5, 0.75, 1):
        x = label_w + plot_w * fraction
        parts.append(f'<line x1="{x:.1f}" y1="{top - 8}" x2="{x:.1f}" y2="{height - 20}" stroke="#e2e8e2" stroke-width="1"/>')
        parts.append(f'<text x="{x:.1f}" y="{top - 12}" text-anchor="middle" font-size="8" fill="#7d8a82">{value_fmt(max_value * fraction)}</text>')
    tx = label_w + plot_w * threshold / max_value if threshold is not None else None
    if tx is not None:
        parts.append(f'<line x1="{tx:.1f}" y1="{top - 6}" x2="{tx:.1f}" y2="{height - 20}" stroke="#8a6410" stroke-width="1.3" stroke-dasharray="4 3"/>')
        parts.append(f'<text x="{tx + 4:.1f}" y="{top + 9}" font-size="8" fill="#8a6410">median {value_fmt(threshold)}</text>')
    for index, (label, value) in enumerate(items):
        y = top + index * row_h
        bar_w = plot_w * (value / max_value if max_value else 0)
        parts.append(f'<rect x="{label_w}" y="{y + 2}" width="{bar_w:.1f}" height="{row_h - 5}" fill="{color_for(label)}" rx="1.5"/>')
        parts.append(f'<text x="{label_w - 7}" y="{y + row_h - 5}" text-anchor="end" font-size="8" fill="#26332c">{esc(label)}</text>')
        parts.append(f'<text x="{label_w + bar_w + 6:.1f}" y="{y + row_h - 5}" font-size="8" font-weight="700" fill="#26332c">{value_fmt(value)}</text>')
    parts.append("</svg>")
    return "".join(parts)


def svg_histogram(histogram, width=760, height=320):
    left, right, top, bottom = 52, 24, 26, 48
    plot_w = width - left - right
    plot_h = height - top - bottom
    bins = list(range(0, 61))
    values = [histogram.get(str(bucket), 0) for bucket in bins]
    y_max = max(values)
    y_max = math.ceil(y_max / 400) * 400
    parts = [
        f'<svg viewBox="0 0 {width} {height}" width="100%" xmlns="http://www.w3.org/2000/svg" font-family="Segoe UI, Arial, sans-serif">'
    ]
    for fraction in (0, 0.25, 0.5, 0.75, 1):
        y = top + plot_h * (1 - fraction)
        parts.append(f'<line x1="{left}" y1="{y:.1f}" x2="{left + plot_w}" y2="{y:.1f}" stroke="#e2e8e2" stroke-width="1"/>')
        parts.append(f'<text x="{left - 7}" y="{y + 3:.1f}" text-anchor="end" font-size="8" fill="#7d8a82">{num(y_max * fraction)}</text>')
    bar_w = plot_w / len(bins)
    for index, value in enumerate(values):
        x = left + index * bar_w
        bar_h = plot_h * value / y_max if y_max else 0
        color = "#a33a3a" if 30 <= bins[index] <= 33 else "#8fa89a"
        parts.append(f'<rect x="{x + 0.6:.1f}" y="{top + plot_h - bar_h:.1f}" width="{bar_w - 1.2:.1f}" height="{bar_h:.1f}" fill="{color}"/>')
    for index in range(0, 61, 5):
        x = left + index * bar_w + bar_w / 2
        parts.append(f'<text x="{x:.1f}" y="{top + plot_h + 16}" text-anchor="middle" font-size="8" fill="#7d8a82">{index}</text>')
    parts.append(f'<text x="{left + plot_w / 2:.1f}" y="{height - 8}" text-anchor="middle" font-size="9" fill="#3f4c45">Jeda kiriman (detik)</text>')
    parts.append(f'<text transform="translate(14 {top + plot_h / 2:.1f}) rotate(-90)" text-anchor="middle" font-size="9" fill="#3f4c45">Jumlah jeda</text>')
    parts.append(f'<text x="{left + 2}" y="{top + 12}" font-size="8.5" fill="#a33a3a" font-weight="700">Tidak ada jeda 0–4 detik; minimum 5,009 detik</text>')
    parts.append(f'<text x="{left + plot_w - 4}" y="{top + 12}" text-anchor="end" font-size="8" fill="#6d7872">Di atas 60 detik: {num(histogram.get("61", 0))} jeda (tidak ditampilkan)</text>')
    parts.append("</svg>")
    return "".join(parts)


def svg_scatter(points, width=760, height=430):
    left, right, top, bottom = 64, 28, 26, 66
    plot_w = width - left - right
    plot_h = height - top - bottom
    x_max = math.ceil((max(point["x"] for point in points) + 2) / 5) * 5
    y_max = math.ceil((max(point["y"] for point in points) + 1) / 2) * 2
    parts = [
        f'<svg viewBox="0 0 {width} {height}" width="100%" xmlns="http://www.w3.org/2000/svg" font-family="Segoe UI, Arial, sans-serif">'
    ]
    for fraction in (0, 0.25, 0.5, 0.75, 1):
        y = top + plot_h * (1 - fraction)
        parts.append(f'<line x1="{left}" y1="{y:.1f}" x2="{left + plot_w}" y2="{y:.1f}" stroke="#e2e8e2" stroke-width="1"/>')
        parts.append(f'<text x="{left - 7}" y="{y + 3:.1f}" text-anchor="end" font-size="8" fill="#7d8a82">{y_max * fraction:.1f}%</text>')
        x = left + plot_w * fraction
        parts.append(f'<line x1="{x:.1f}" y1="{top}" x2="{x:.1f}" y2="{top + plot_h}" stroke="#e2e8e2" stroke-width="1"/>')
        parts.append(f'<text x="{x:.1f}" y="{top + plot_h + 16}" text-anchor="middle" font-size="8" fill="#7d8a82">{x_max * fraction:.0f}%</text>')
    med_x = statistics.median(point["x"] for point in points)
    med_y = statistics.median(point["y"] for point in points)
    mx = left + plot_w * med_x / x_max
    my = top + plot_h * (1 - med_y / y_max)
    parts.append(f'<line x1="{mx:.1f}" y1="{top}" x2="{mx:.1f}" y2="{top + plot_h}" stroke="#8a6410" stroke-width="1" stroke-dasharray="4 3"/>')
    parts.append(f'<line x1="{left}" y1="{my:.1f}" x2="{left + plot_w}" y2="{my:.1f}" stroke="#8a6410" stroke-width="1" stroke-dasharray="4 3"/>')
    for point in points:
        x = left + plot_w * point["x"] / x_max
        y = top + plot_h * (1 - point["y"] / y_max)
        radius = 3 + math.sqrt(point["runs"]) * 2.2
        parts.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{radius:.1f}" fill="{color_for(point["name"])}" fill-opacity="0.72" stroke="#fff" stroke-width="1"/>')
    parts.append(f'<text x="{left + plot_w / 2:.1f}" y="{height - 10}" text-anchor="middle" font-size="9" fill="#3f4c45">Duplikat laporan (%)</text>')
    parts.append(f'<text transform="translate(16 {top + plot_h / 2:.1f}) rotate(-90)" text-anchor="middle" font-size="9" fill="#3f4c45">Jeda 30–33 detik (%)</text>')
    parts.append("</svg>")
    return "".join(parts)


def evidence_html(payload):
    links = []
    for row in payload["rows"]:
        for token in ((row.get("isian") or "") + " " + (row.get("bukti") or "")).split():
            candidate = token.strip("\"'()[]{} ,;<>").rstrip(".")
            if candidate.lower().startswith(("http://", "https://")) and candidate not in links:
                links.append(candidate)
    drive = [url for url in links if "drive.google.com" in url.lower() or "docs.google.com" in url.lower()]
    uploads = sorted(
        {
            token.strip("\"'()[]{} ,;<>").rstrip(".")
            for row in payload["rows"]
            for token in (row.get("isian") or "").split()
            if "/uploads/" in token
        }
    )
    parts = ['<div class="evidence">']
    parts.append('<span class="ev-label">Tautan Drive</span>')
    if drive:
        parts.append('<span class="ev-value">' + "<br>".join(f'<a href="{esc(url)}">{esc(url)}</a>' for url in drive) + "</span>")
    else:
        parts.append('<span class="ev-empty">— tidak ada</span>')
    if uploads:
        parts.append('<span class="ev-label">Foto unggahan</span>')
        parts.append('<span class="ev-value">' + "<br>".join(esc(path) for path in uploads) + "</span>")
    parts.append("</div>")
    return "".join(parts)


def report_block(payload, label, gap_label=None):
    dup_label = "Duplikat" if payload["all_dup"] else "Sebagian duplikat" if payload["any_dup"] else "Unik"
    gap_text = f'Jeda {payload["gap"]:.3f} dtk' if payload["gap"] is not None else "Jeda —"
    if gap_label:
        gap_text = gap_label
    rows = []
    for index, row in enumerate(payload["rows"], start=1):
        multi = len(payload["rows"]) > 1
        rows.append(
            '<div class="row-block">'
            f'<div class="row-head"><span class="qw">{esc(row["kode"] or "tanpa kode")}</span>'
            f'<span class="row-title">{esc(row["kegiatan"] or "—")}</span>'
            f'<span class="row-src">Baris Excel {num(row["row"])}</span></div>'
            f'<div class="isian">{esc((row.get("isian") or "").strip() or "—")}</div>'
            "</div>"
        )
    return (
        '<div class="report">'
        '<div class="rep-head">'
        f'<span class="badge">{esc(label)}</span>'
        f'<span class="badge qw-badge">{esc(" · ".join(payload["kodes"]) or "—")}</span>'
        f'<span class="when">{esc(payload["time"])}</span>'
        f'<span class="unit">{esc(payload["satker"] or "—")}</span>'
        f'<span class="reporter">{esc(payload["reporter"] or "—")}</span>'
        f'<span class="gap">{esc(gap_text)}</span>'
        f'<span class="dup {"dup-yes" if payload["all_dup"] else "dup-no"}">{dup_label}</span>'
        "</div>"
        f'<div class="rep-meta">Pelapor: {esc(payload["pelapor_raw"] or "—")} · ID: {esc(payload["id"])} · '
        f'{len(payload["rows"])} kegiatan · {esc(payload["date"])}</div>'
        + "".join(rows)
        + evidence_html(payload)
        + "</div>"
    )


# Aggregates
QW5_CODES = {"QW5.1", "QW5.3", "QW5.4"}


def norm_text(value):
    return " ".join((value or "").split()).lower()


def example_block(row, label):
    payload = {
        "id": row.get("report_id") or "",
        "time": stamp(row.get("report_id") or ""),
        "date": row.get("tanggal") or "",
        "satker": row.get("satuan") or "—",
        "reporter": row.get("pelapor") or "—",
        "pelapor_raw": row.get("pelapor") or "—",
        "kodes": [row.get("kode") or ""],
        "rows": [
            {
                "row": row.get("source_row") or 0,
                "kode": row.get("kode") or "",
                "kegiatan": row.get("kegiatan") or "",
                "isian": row.get("isian") or "",
                "bukti": row.get("bukti") or "",
            }
        ],
    }
    return (
        '<div class="report example">'
        '<div class="rep-head">'
        f'<span class="badge gold">{esc(label)}</span>'
        f'<span class="badge qw-badge">{esc(row.get("kode") or "—")}</span>'
        f'<span class="when">{esc(payload["time"])}</span>'
        f'<span class="unit">{esc(payload["satker"])}</span>'
        f'<span class="reporter">{esc(payload["reporter"])}</span>'
        f'<span class="gap">Baris Excel {num(payload["rows"][0]["row"])}</span>'
        "</div>"
        f'<div class="rep-meta">ID: {esc(payload["id"] or "—")} · kegiatan: {esc(payload["rows"][0]["kegiatan"] or "—")}</div>'
        f'<div class="isian">{esc(payload["rows"][0]["isian"].strip() or "—")}</div>'
        + evidence_html(payload)
        + "</div>"
    )


qw5_details = {}
dup_details = {}
for name in FOCUS:
    polres_rows = [row for row in rows6 if row["polres"] == name]
    qw_all = [row for row in qw5_rows if row["polres"] == name]
    qw_neg = [row for row in qw_all if "penyidikan perkara?: Tidak" in (row["isian"] or "")]
    satker_counts = Counter(row["satuan"] for row in qw_neg)
    examples = []
    seen_satker = set()
    for row in sorted(qw_neg, key=lambda item: uuid_ms(item.get("report_id") or "") or 0):
        if row["satuan"] in seen_satker:
            continue
        seen_satker.add(row["satuan"])
        examples.append(row)
        if len(examples) >= 5:
            break
    for row in qw_neg:
        if len(examples) >= 5:
            break
        if row not in examples:
            examples.append(row)
    qw5_details[name] = {
        "total": len(qw_all),
        "negative": len(qw_neg),
        "rate": len(qw_neg) / len(qw_all) * 100 if qw_all else 0,
        "counts": Counter(row["kode"] for row in qw_neg),
        "satker": satker_counts.most_common(5),
        "examples": examples,
    }

    groups = defaultdict(list)
    for row in polres_rows:
        if (row["isian"] or "").strip():
            groups[norm_text(row["isian"])].append(row)
    top_groups = sorted((group for group in groups.values() if len(group) >= 2), key=lambda item: -len(item))[:2]
    dup_groups = []
    for group in top_groups:
        ordered = sorted(group, key=lambda item: uuid_ms(item.get("report_id") or "") or 0)
        samples = ordered[:2]
        dup_groups.append(
            {
                "count": len(group),
                "dates": len({row["tanggal"] for row in group}),
                "satker": len({row["satuan"] for row in group}),
                "kodes": Counter(row["kode"] for row in group).most_common(3),
                "samples": samples,
            }
        )
    dup_details[name] = dup_groups


# Aggregates
total_reports = sum(item["reports"] for item in stats.values())
total_gaps = sum(item["gaps"] for item in stats.values())
full_stats = data["stats"]
all_gaps_scope = sum(item["gaps"] for item in full_stats.values())
total_rows_scope = sum(item["rows"] for item in full_stats.values())
total_boundary = sum(item["boundary"] for item in stats.values())
total_runs = sum(item["runs"] for item in stats.values())
total_nondup_runs = sum(item["nondup_runs"] for item in stats.values())
total_duplicates = sum(item["duplicate_reports"] for item in stats.values())
overall_boundary_rate = total_boundary / total_gaps * 100
overall_dup_rate = total_duplicates / total_reports * 100
median_boundary = statistics.median(item["boundary_rate"] for item in stats.values())
median_dup = statistics.median(item["duplicate_rate"] for item in stats.values())
qw_values = {name: qw5_rate(name) for name in stats}
median_qw = statistics.median(qw_values.values())


def average_ranks(values):
    ordered = sorted(values.items(), key=lambda pair: -pair[1])
    ranks = {}
    index = 0
    while index < len(ordered):
        end = index
        while end + 1 < len(ordered) and ordered[end + 1][1] == ordered[index][1]:
            end += 1
        rank = (index + end) / 2 + 1
        for position in range(index, end + 1):
            ranks[ordered[position][0]] = rank
        index = end + 1
    return ranks


boundary_rank = average_ranks({name: item["boundary_rate"] for name, item in stats.items()})
dup_rank = average_ranks({name: item["duplicate_rate"] for name, item in stats.items()})
qw_rank = average_ranks(qw_values)
average_rank = {
    name: statistics.mean((boundary_rank[name], dup_rank[name], qw_rank[name]))
    for name in stats
}
composite_order = sorted(
    stats,
    key=lambda name: (
        average_rank[name],
        qw_rank[name],
        dup_rank[name],
        boundary_rank[name],
        name,
    ),
)
composite_rank = {name: index + 1 for index, name in enumerate(composite_order)}
composite_score = {
    name: round((len(stats) - average_rank[name]) / (len(stats) - 1) * 100, 1)
    for name in stats
}

focus_reports = sum(stats[name]["reports"] for name in FOCUS)
core_reports = sum(stats[name]["reports"] for name in CORE)
core_duplicates = sum(stats[name]["duplicate_reports"] for name in CORE)
focus_gaps = sum(stats[name]["gaps"] for name in FOCUS)
focus_boundary = sum(stats[name]["boundary"] for name in FOCUS)
focus_runs = sum(stats[name]["runs"] for name in FOCUS)
focus_nondup = sum(stats[name]["nondup_runs"] for name in FOCUS)
focus_duplicates = sum(stats[name]["duplicate_reports"] for name in FOCUS)
focus_qw_negative = sum(1 for row in qw5_rows if row["polres"] in FOCUS and "penyidikan perkara?: Tidak" in (row["isian"] or ""))
focus_qw_total = sum(1 for row in qw5_rows if row["polres"] in FOCUS)

boundary_rank = {name: index + 1 for index, (name, _) in enumerate(sorted(stats.items(), key=lambda pair: -pair[1]["boundary_rate"]))}
dup_rank = {name: index + 1 for index, (name, _) in enumerate(sorted(stats.items(), key=lambda pair: -pair[1]["duplicate_rate"]))}

boundary_items = sorted(
    ((name, item["boundary_rate"]) for name, item in stats.items()),
    key=lambda pair: -pair[1],
)
duplicate_items = sorted(
    ((name, item["duplicate_rate"]) for name, item in stats.items()),
    key=lambda pair: -pair[1],
)
composite_items = [
    (name, composite_score[name])
    for name in composite_order[:12]
]
scatter_points = [
    {
        "name": name,
        "x": item["duplicate_rate"],
        "y": item["boundary_rate"],
        "runs": item["runs"],
    }
    for name, item in stats.items()
]

boundary_svg = svg_hbar(
    boundary_items,
    10,
    lambda value: f"{value:.2f}%",
    median_boundary,
)
duplicate_svg = svg_hbar(
    duplicate_items,
    25,
    lambda value: f"{value:.1f}%",
    median_dup,
)
histogram_svg = svg_histogram(data["histogram"])
scatter_svg = svg_scatter(scatter_points)
composite_svg = svg_hbar(
    composite_items,
    100,
    lambda value: f"{value:.0f}",
    None,
    row_h=18,
)

# Main comparison table
compare_rows = []
for name in composite_order:
    item = stats[name]
    css = ' class="focus-row"' if name in CORE else ' class="extra-row"' if name in EXTRA else ""
    compare_rows.append(
        f"<tr{css}>"
        f'<td class="center">{composite_rank[name]}</td>'
        f'<td class="strong">{esc(name)}</td>'
        f'<td class="right">{num(item["reports"])}</td>'
        f'<td class="right">{pct(item["boundary_rate"])}</td>'
        f'<td class="right">{pct(item["duplicate_rate"], 1)}</td>'
        f'<td class="right">{pct(qw_values[name], 1)}</td>'
        f'<td class="right strong">{composite_score[name]:.1f}</td>'
        "</tr>"
    )

six_rows = []
for name in sorted(FOCUS, key=lambda item: composite_rank[item]):
    item = stats[name]
    six_rows.append(
        "<tr>"
        f'<td class="center">{composite_rank[name]}</td>'
        f'<td class="strong">{esc(name)}</td>'
        f'<td class="right">{pct(item["boundary_rate"])}</td>'
        f'<td class="right">{boundary_rank[name]:.0f}</td>'
        f'<td class="right">{pct(item["duplicate_rate"], 1)}</td>'
        f'<td class="right">{dup_rank[name]:.0f}</td>'
        f'<td class="right">{pct(qw_values[name], 1)}</td>'
        f'<td class="right">{qw_rank[name]:.0f}</td>'
        f'<td class="right strong">{composite_score[name]:.1f}</td>'
        "</tr>"
    )

# Focus content
focus_sections = []
for index, name in enumerate(FOCUS, start=1):
    item = stats[name]
    polres_focus = focus[name]
    overall = polres_focus["stats"]
    runs = sorted(polres_focus["runs"], key=lambda run: run["start"])
    run_reports = [report for run in runs for report in run["reports"]]
    qw_runs = Counter(kode for report in run_reports for kode in report["kodes"])
    qw_sub30 = Counter(kode for report in polres_focus["sub30"] for kode in report["kodes"])
    qw = qw5_details[name]
    dup_groups = dup_details[name]
    qw_satker_rows = []
    for satker, count in qw["satker"]:
        qw_satker_rows.append(
            "<tr>"
            f'<td class="strong">{esc(satker)}</td>'
            f'<td class="right">{num(count)}</td>'
            "</tr>"
        )
    qw_counts = ", ".join(f"{kode} ({count})" for kode, count in qw["counts"].most_common())
    qw_examples = "".join(example_block(row, "Contoh QW5") for row in qw["examples"])
    dup_html = []
    for group_number, group in enumerate(dup_groups, start=1):
        kodes = ", ".join(f"{kode} ({count})" for kode, count in group["kodes"])
        dup_html.append(
            f'<p class="note"><strong>Kelompok {group_number}</strong>: {num(group["count"])} baris identik · '
            f'{group["dates"]} tanggal · {group["satker"]} satker · kode {esc(kodes)}. '
            'Dua contoh baris ditampilkan lengkap di bawah.</p>'
        )
        for row in group["samples"]:
            dup_html.append(example_block(row, "Contoh duplikat"))
    reporter_rows = []
    for reporter in [entry for entry in data["top_reporters"] if entry["polres"] == name][:8]:
        reporter_rows.append(
            "<tr>"
            f'<td class="strong">{esc(reporter["reporter"])}</td>'
            f'<td>{esc(reporter["satker"])}</td>'
            f'<td class="right">{num(reporter["boundary"])}</td>'
            f'<td class="right">{num(reporter["gaps"])}</td>'
            f'<td class="right">{pct(reporter["boundary"] / reporter["gaps"] * 100 if reporter["gaps"] else 0)}</td>'
            f'<td class="right">{num(reporter["runs"])}</td>'
            "</tr>"
        )
    run_rows = []
    for number, run in enumerate(runs, start=1):
        run_rows.append(
            "<tr>"
            f'<td class="center">{number}</td>'
            f'<td class="strong">{esc(run["reporter"])}</td>'
            f'<td>{esc(run["satker"])}</td>'
            f'<td class="right">{num(run["length"])}</td>'
            f'<td class="nowrap">{esc(run["start"])}</td>'
            f'<td class="nowrap">{esc(run["end"])}</td>'
            f'<td class="mono small">{esc(", ".join(f"{gap:.2f}" for gap in run["gaps"]))}</td>'
            f'<td class="center {"yes" if run["nondup"] else "no"}">{"Ya" if run["nondup"] else "Tidak"}</td>'
            "</tr>"
        )
    run_details = []
    for number, run in enumerate(runs, start=1):
        run_details.append(
            f'<h3>Rangkaian {number} — {esc(run["reporter"])} · {esc(run["satker"])} '
            f'({run["length"]} jeda, {"nonduplikat" if run["nondup"] else "melibatkan duplikat"})</h3>'
            f'<p class="note">Mulai {esc(run["start"])} · selesai {esc(run["end"])} · jeda: '
            f'{esc(", ".join(f"{gap:.3f}" for gap in run["gaps"]))} detik.</p>'
        )
        for position, report in enumerate(run["reports"], start=1):
            run_details.append(
                report_block(report, f"Rangkaian {number} · laporan {position}/{len(run['reports'])}")
            )
    sub30_details = []
    for report in polres_focus["sub30"]:
        sub30_details.append(report_block(report, "Jeda <30 detik"))
    focus_sections.append(
        f'<section class="page-break">'
        f'<h2>A.{index} {esc(name)}</h2>'
        f'<div class="stats">'
        f'<div class="stat"><span>Laporan</span><strong>{num(item["reports"])}</strong></div>'
        f'<div class="stat"><span>Pelapor</span><strong>{num(overall["reporters"])}</strong></div>'
        f'<div class="stat"><span>Jeda 30–33 dtk</span><strong>{pct(item["boundary_rate"])}</strong><em>{num(item["boundary"])} dari {num(item["gaps"])}</em></div>'
        f'<div class="stat"><span>Rangkaian ≥3</span><strong>{num(item["runs"])}</strong><em>{num(item["nondup_runs"])} nonduplikat</em></div>'
        f'<div class="stat"><span>Duplikat</span><strong>{pct(item["duplicate_rate"], 1)}</strong><em>{num(item["duplicate_reports"])} laporan</em></div>'
        f'<div class="stat"><span>QW5 kontradiksi</span><strong>{pct(qw["rate"], 1)}</strong><em>{num(qw["negative"])} dari {num(qw["total"])}</em></div>'
        f'<div class="stat"><span>Rank gabungan</span><strong>#{composite_rank[name]}</strong><em>skor {composite_score[name]:.1f}</em></div>'
        f'<div class="stat"><span>Median jeda</span><strong>{overall["median_gap"]:.1f} dtk</strong><em>min {overall["min_gap"]:.2f} dtk</em></div>'
        "</div>"
        f'<p class="note">Rank gabungan: #{composite_rank[name]} dari 35 (rata-rata peringkat jeda 30–33, duplikat, dan QW5). '
        f'Peringkat jeda 30–33: {boundary_rank[name]:.1f}; duplikat: {dup_rank[name]:.1f}; QW5: {qw_rank[name]:.1f}. '
        f'Jeda &lt;3 detik: {num(overall["sub3"])}. Jeda &lt;30 detik: {num(item["sub30"])} (semua pada 10 November, masa transisi).</p>'
        '<h3>Kontradiksi QW5</h3>'
        f'<p class="note">{num(qw["negative"])} dari {num(qw["total"])} baris QW5.1/QW5.3/QW5.4 menjawab '
        '"Tidak menangani penyidikan perkara" (' + pct(qw["rate"], 1) + '). Sebaran: ' + esc(qw_counts) + '.</p>'
        '<table class="compact"><colgroup><col style="width:60%"><col style="width:40%"></colgroup>'
        '<thead><tr><th>Satker penyumbang QW5 kontradiktif</th><th class="right">Jumlah baris</th></tr></thead><tbody>'
        + "".join(qw_satker_rows)
        + "</tbody></table>"
        '<h3>Contoh baris QW5 kontradiktif</h3>'
        '<p class="note">Isian lengkap, pelapor, satker, waktu UUID, dan nomor baris Excel.</p>'
        + qw_examples
        + '<h3>Contoh kelompok laporan duplikat</h3>'
        f'<p class="note">Laporan duplikat dihitung dari Satker + kode + isi yang sama pada hari yang sama, atau isi sama lintas hari. Total duplikat: {num(item["duplicate_reports"])} laporan ({pct(item["duplicate_rate"], 1)}).</p>'
        + ("".join(dup_html) if dup_html else '<p class="note">Tidak ada kelompok duplikat pada data ini.</p>')
        + '<h3>Pelapor dengan laporan bertanda terbanyak</h3>'
        '<table class="compact"><colgroup><col style="width:30%"><col style="width:24%"><col style="width:12%"><col style="width:12%"><col style="width:10%"><col style="width:12%"></colgroup>'
        '<thead><tr><th>Pelapor</th><th>Satker</th><th class="right">Jeda 30–33</th><th class="right">Total jeda</th><th class="right">%</th><th class="right">Dalam rangkaian</th></tr></thead><tbody>'
        + "".join(reporter_rows)
        + "</tbody></table>"
        '<h3>Rangkaian jeda 30–33 detik (≥3 berturut-turut)</h3>'
        '<table class="compact"><colgroup><col style="width:5%"><col style="width:21%"><col style="width:20%"><col style="width:8%"><col style="width:12%"><col style="width:12%"><col style="width:16%"><col style="width:6%"></colgroup>'
        '<thead><tr><th>No</th><th>Pelapor</th><th>Satker</th><th class="right">Jeda</th><th>Mulai</th><th>Selesai</th><th>Nilai jeda (detik)</th><th class="center">Nondup</th></tr></thead><tbody>'
        + "".join(run_rows)
        + "</tbody></table>"
        f'<p class="note">Kode QW pada rangkaian: '
        f'{esc(", ".join(f"{kode} ({count})" for kode, count in qw_runs.most_common(10)))}.</p>'
        + "".join(run_details)
        + '<h3>Laporan dengan jeda &lt;30 detik — 10 November (masa transisi)</h3>'
        f'<p class="note">{len(polres_focus["sub30"])} laporan. Kode QW: '
        f'{esc(", ".join(f"{kode} ({count})" for kode, count in qw_sub30.most_common(10)))}.</p>'
        + "".join(sub30_details)
        + "</section>"
    )

css = """
@page { size: A4 portrait; margin: 14mm 13mm 15mm; }
* { box-sizing: border-box; }
body { margin: 0; font-family: "Segoe UI", Arial, sans-serif; font-size: 8.8pt; line-height: 1.42; color: #1f2a25; }
h1, h2, h3, p { margin: 0; }
h1 { font-size: 18pt; line-height: 1.15; color: #143d30; letter-spacing: -0.2px; }
h2 { font-size: 13pt; color: #143d30; margin: 7mm 0 2mm; border-bottom: 0.5mm solid #d7ddd6; padding-bottom: 1.2mm; break-after: avoid; }
h3 { font-size: 9.8pt; color: #143d30; margin: 4mm 0 1.5mm; break-after: avoid; }
.eyebrow { font-size: 7.3pt; font-weight: 700; letter-spacing: 1.6px; color: #9a6b12; text-transform: uppercase; }
.memo-head { border-bottom: 2.4mm solid #143d30; padding-bottom: 4mm; margin-bottom: 4mm; }
.subtitle { font-size: 10.4pt; color: #4d5b53; margin-top: 1.6mm; }
.meta { display: grid; grid-template-columns: repeat(4, 1fr); gap: 2.4mm; margin-top: 3.5mm; }
.meta div { border: 0.3mm solid #d7ddd6; border-radius: 2mm; padding: 1.8mm 2.2mm; background: #fbfcfa; }
.meta span { display: block; font-size: 6.6pt; color: #6d7872; text-transform: uppercase; letter-spacing: 0.4px; }
.meta strong { font-size: 8.2pt; color: #143d30; }
.callout { background: #f4f7f4; border-left: 1.3mm solid #8a6410; padding: 3mm 3.4mm; margin: 3mm 0; break-inside: avoid; }
.callout h3 { margin: 0 0 1.4mm; font-size: 9.3pt; }
.callout ul { margin: 0; padding-left: 4.4mm; }
.callout li { margin: 1.1mm 0; }
.note { color: #5e6b63; font-size: 7.8pt; margin: 1mm 0 2.2mm; }
.chart { margin: 2.5mm 0 4.5mm; break-inside: avoid; }
.chart svg { display: block; width: 100%; height: auto; }
.caption { color: #5e6b63; font-size: 7.5pt; margin-top: 1.2mm; }
table { width: 100%; border-collapse: collapse; table-layout: fixed; }
table.compare { font-size: 6.4pt; margin: 1.5mm 0 2mm; }
table.compare th { background: #143d30; color: #fff; font-size: 5.9pt; text-align: left; padding: 1.3mm 0.8mm; text-transform: uppercase; }
table.compare td { border-bottom: 0.22mm solid #e2e7e2; padding: 1mm 0.8mm; vertical-align: top; overflow-wrap: anywhere; }
table.compare tbody tr:nth-child(even) td { background: #f8faf8; }
table.compare tr.focus-row td { background: #fbeeee; }
table.compare tr.extra-row td { background: #fdf6e8; }
table.compact { font-size: 7.1pt; margin: 1mm 0 3mm; }
table.compact th { background: #eef3ee; color: #143d30; font-size: 6.5pt; text-align: left; padding: 1.3mm 0.9mm; text-transform: uppercase; }
table.compact td { border-bottom: 0.24mm solid #e2e7e2; padding: 1.1mm 0.9mm; vertical-align: top; overflow-wrap: anywhere; }
thead { display: table-header-group; }
tr { break-inside: avoid; }
.center { text-align: center; }
.right { text-align: right; }
.strong { font-weight: 700; }
.mono { font-family: Consolas, monospace; }
.small { font-size: 6.6pt; }
.nowrap { white-space: nowrap; }
.yes { color: #245a46; font-weight: 700; }
.no { color: #a33a3a; font-weight: 700; }
.stats { display: grid; grid-template-columns: repeat(4, 1fr); gap: 2mm; margin: 2.5mm 0 3mm; }
.stat { border: 0.3mm solid #d7ddd6; border-radius: 1.6mm; padding: 1.8mm; background: #fbfcfa; }
.stat span { display: block; font-size: 6.2pt; color: #6d7872; text-transform: uppercase; }
.stat strong { display: block; font-size: 12pt; color: #143d30; line-height: 1.2; margin-top: 0.4mm; }
.stat em { display: block; font-style: normal; font-size: 6.6pt; color: #8a6410; margin-top: 0.3mm; }
.report { border-top: 0.3mm solid #e2e8e2; padding: 2.1mm 0 2.3mm; break-inside: avoid; }
.report.example { border-left: 0.8mm solid #c08a1e; padding-left: 2mm; }
.rep-head { display: flex; flex-wrap: wrap; gap: 1.3mm; align-items: center; font-size: 6.8pt; }
.badge { background: #143d30; color: #fff; border-radius: 1mm; padding: 0.35mm 1.2mm; font-weight: 700; font-size: 6.3pt; }
.badge.gold { background: #8a6410; }
.qw-badge { background: #8a6410; }
.when { color: #8a6410; font-weight: 700; }
.unit, .reporter { color: #143d30; font-weight: 700; }
.gap { color: #3f4c45; }
.dup { border-radius: 1mm; padding: 0.3mm 1.1mm; font-size: 6.2pt; font-weight: 700; }
.dup-yes { background: #fbeeee; color: #a33a3a; }
.dup-no { background: #eef3ee; color: #245a46; }
.rep-meta { color: #6d7872; font-size: 6.6pt; margin-top: 1mm; overflow-wrap: anywhere; }
.row-block { margin-top: 1.2mm; }
.row-head { display: flex; flex-wrap: wrap; gap: 1.3mm; align-items: center; font-size: 6.6pt; }
.qw { background: #eef3ee; color: #143d30; border-radius: 0.8mm; padding: 0.25mm 1mm; font-family: Consolas, monospace; font-weight: 700; }
.row-title { color: #3f4c45; }
.row-src { color: #6d7872; font-family: Consolas, monospace; font-size: 6.4pt; }
.isian { white-space: pre-wrap; overflow-wrap: anywhere; font-size: 7.2pt; line-height: 1.36; margin-top: 0.9mm; color: #26332c; }
.evidence { display: flex; flex-wrap: wrap; gap: 1.2mm; align-items: baseline; margin-top: 1mm; font-size: 6.6pt; }
.ev-label { background: #eef3ee; color: #143d30; border-radius: 1mm; padding: 0.3mm 1.1mm; font-weight: 700; text-transform: uppercase; font-size: 6.1pt; }
.ev-value { color: #245a46; overflow-wrap: anywhere; }
.ev-value a { color: #245a46; text-decoration: none; }
.ev-empty { color: #a33a3a; font-weight: 700; }
.page-break { break-before: page; }
.footer-note { margin-top: 5mm; padding-top: 2.4mm; border-top: 0.3mm solid #d7ddd6; color: #6d7872; font-size: 7.2pt; }
ul.clean { margin: 1mm 0 2mm; padding-left: 4.4mm; }
ul.clean li { margin: 1.1mm 0; }
ol.steps { margin: 1mm 0 2mm; padding-left: 5mm; }
ol.steps li { margin: 1.3mm 0; }
"""

html_doc = f"""<!doctype html>
<html lang="id"><head><meta charset="utf-8">
<title>Analisis UUID v7 dan Rank Indikasi — Enam Polres Jawa Tengah</title>
<style>{css}</style></head>
<body>

<section class="memo-head">
  <p class="eyebrow">Memo Analisis Internal</p>
  <h1>Analisis UUID v7 dan Rank Indikasi Enam Polres</h1>
  <p class="subtitle">Temanggung, Rembang, Karanganyar, Purbalingga, Surakarta, dan Batang<br>
  Perbandingan 35 polres Polda Jawa Tengah · 10–19 November 2026</p>
  <div class="meta">
    <div><span>Sumber</span><strong>report-2026-11-10-to-2026-11-19.xlsx</strong></div>
    <div><span>Cakupan</span><strong>{num(total_reports)} laporan · {num(total_rows_scope)} baris</strong></div>
    <div><span>Metode</span><strong>UUID v7 · jeda per pelapor</strong></div>
    <div><span>Sifat</span><strong>Terbatas — audit internal</strong></div>
  </div>
</section>

<div class="callout">
  <h3>Ringkasan eksekutif</h3>
  <ul>
    <li>Dari <strong>{num(all_gaps_scope)} jeda</strong> antar-kiriman pelapor yang sama, <strong>tidak ada satu pun jeda &lt;3 detik</strong>. Definisi BOT (&lt;3 detik dan bukan duplikat) menghasilkan <strong>0 laporan</strong> di seluruh 35 polres.</li>
    <li>Jeda minimum {data["min_gap"]:.3f} detik terjadi pada 10 November (masa transisi). Setelah 10 November, hanya <strong>1 dari {num(total_gaps)} jeda</strong> yang &lt;30 detik; aplikasi jelas menerapkan batas minimum 30 detik.</li>
    <li>Tiga polres awal: QW5 {pct(qw_values["Polres Temanggung"], 1)}, {pct(qw_values["Polres Rembang"], 1)}, {pct(qw_values["Polres Karanganyar"], 1)}; duplikat {pct(stats["Polres Temanggung"]["duplicate_rate"], 1)}, {pct(stats["Polres Rembang"]["duplicate_rate"], 1)}, {pct(stats["Polres Karanganyar"]["duplicate_rate"], 1)}; jeda 30–33 {pct(stats["Polres Temanggung"]["boundary_rate"])}; rank gabungan #{composite_rank["Polres Temanggung"]}, #{composite_rank["Polres Rembang"]}, #{composite_rank["Polres Karanganyar"]}.</li>
    <li>Purbalingga, Surakarta, dan Batang adalah <strong>peringkat 1–3 pada jeda 30–33 detik</strong> (8,28%; 7,93%; 6,86%). QW5 kontradiksi mereka juga tinggi: 80,4%; 92,9%; 84,1%. Jadi ketiganya memang masuk audit prioritas.</li>
    <li><strong>Rank gabungan tiga indikator</strong> (jeda 30–33, duplikat, QW5): Surakarta #{composite_rank["Polresta Surakarta"]}, Temanggung #{composite_rank["Polres Temanggung"]}, Batang #{composite_rank["Polresta Batang"]}, Tegal Kota #{composite_rank["Polres Tegal Kota"]}, Rembang #{composite_rank["Polres Rembang"]}, Pekalongan Kota #{composite_rank["Polres Pekalongan Kota"]}, Purbalingga #{composite_rank["Polres Purbalingga"]}, Pekalongan #{composite_rank["Polres Pekalongan"]}, Karanganyar #{composite_rank["Polres Karanganyar"]}.</li>
    <li>Dari <strong>{num(focus_runs)} rangkaian 30–33 detik</strong> di enam polres, hanya <strong>{num(focus_nondup)} rangkaian</strong> yang tidak melibatkan duplikat. Pola ini lebih tepat disebut <strong>pengiriman berulang/duplikat pada batas minimum</strong>, bukan bot.</li>
    <li><strong>Kesimpulan:</strong> metode UUID tidak membuktikan bot. Yang terlihat adalah duplikasi dan pengiriman cepat yang konsisten; label “bot” memerlukan log aplikasi (IP, perangkat, sesi, user agent) dan verifikasi lapangan.</li>
  </ul>
</div>

<h2>1. Metode dan batasan</h2>
<div class="two" style="display:grid;grid-template-columns:1fr 1fr;gap:3.4mm;">
  <div class="callout" style="margin:0;border-left-color:#143d30;">
    <h3>Perhitungan</h3>
    <ul class="clean">
      <li><strong>Waktu kirim:</strong> 48 bit pertama UUID v7 dikonversi ke Unix milidetik, lalu ke WIB (UTC+7).</li>
      <li><strong>Pelapor:</strong> nama dinormalisasi menjadi huruf saja, tanpa gelar setelah koma, lalu digabung dengan Polres. Satu kiriman = satu ID laporan utama; beberapa kegiatan dalam ID yang sama dihitung satu kiriman.</li>
      <li><strong>Jeda:</strong> selisih waktu kirim dengan kiriman sebelumnya oleh pelapor yang sama di Polres yang sama.</li>
      <li><strong>BOT:</strong> jeda &lt;3 detik dan kiriman bukan duplikat.</li>
      <li><strong>Duplikat:</strong> Satker + kode + isi sama pada hari yang sama, atau isi sama lintas hari dengan uraian ≥30 karakter; kiriman pertama tidak dihitung. Untuk kiriman berisi beberapa kegiatan, status “Duplikat” diberikan bila seluruh kegiatannya duplikat; bila hanya sebagian, ditandai “Sebagian duplikat”.</li>
      <li><strong>Batas 30 detik:</strong> jeda 30–33 detik; rangkaian ≥3 kali berturut-turut dipakai sebagai indikator tambahan.</li>
    </ul>
  </div>
  <div class="callout" style="margin:0;border-left-color:#8a6410;">
    <h3>Batasan</h3>
    <ul class="clean">
      <li>UUID v7 dapat dihasilkan klien; timestamp bisa dimanipulasi. Log server tetap diperlukan.</li>
      <li>Nama pelapor tidak unik sempurna; typo nama (mis. MMUH vs MUH) dapat memecah atau menggabungkan orang.</li>
      <li>Data hanya periode 10–19 November 2026; 10 November adalah masa transisi batas 30 detik. 200 laporan tanpa nama Polres tidak dianalisis.</li>
      <li>Indikasi pola waktu bukan bukti penggunaan aplikasi otomatis.</li>
      <li>Tidak ada data IP, perangkat, sesi, atau user agent di workbook.</li>
    </ul>
  </div>
</div>

<h2>2. Validasi UUID v7 dan ambang 30 detik</h2>
<p>UUID v7 pada kolom <em>ID laporan utama</em> cocok dengan kolom <em>Tanggal lapor (WIB)</em>. Contoh: UUID <span class="mono">01a1f1f0-6e9d-7af3-968f-152891003660</span> menghasilkan <strong>19 November 2026, 10:53:48.445 WIB</strong>, sesuai kolom laporan 10:53.</p>
<div class="chart">
{histogram_svg}
<p class="caption">Gambar 1. Distribusi jeda 0–60 detik. Tidak ada jeda 0–4 detik; lonjakan tajam terjadi pada 30–33 detik. Sebanyak {num(data["histogram"].get("61", 0))} jeda &gt;60 detik tidak ditampilkan.</p>
</div>
<p>Pada 10 November terdapat <strong>875 jeda &lt;30 detik</strong> (masa transisi). Mulai 11 November hanya <strong>1 jeda &lt;30 detik</strong> dari {num(total_gaps)} jeda, yaitu di Polresta Pati. Ini konsisten dengan pembatasan minimum 30 detik pada aplikasi.</p>

<h2>3. Perbandingan 35 polres</h2>
<div class="chart">
{boundary_svg}
<p class="caption">Gambar 2. Persentase jeda 30–33 detik terhadap seluruh jeda, 11–19 November 2026. Merah = tiga polres awal; kuning = Purbalingga, Surakarta, Batang. Ketiganya justru peringkat 1–3 pada indikator ini.</p>
</div>
<div class="chart">
{scatter_svg}
<p class="caption">Gambar 3. Duplikasi laporan versus jeda 30–33 detik. Merah = tiga polres awal; kuning = Purbalingga, Surakarta, Batang. Titik abu-abu = 29 polres lain; ukuran gelembung = jumlah rangkaian 30–33 detik.</p>
</div>
<div class="chart">
{duplicate_svg}
<p class="caption">Gambar 4. Persentase laporan duplikat per polres, 10–19 November 2026. Merah = tiga polres awal; kuning = Purbalingga, Surakarta, Batang.</p>
</div>

<table class="compare">
<colgroup><col style="width:5%"><col style="width:30%"><col style="width:10%"><col style="width:15%"><col style="width:15%"><col style="width:15%"><col style="width:10%"></colgroup>
<thead><tr><th>Rank</th><th>Polres</th><th class="right">Laporan</th><th class="right">Jeda 30–33</th><th class="right">Duplikat</th><th class="right">QW5 kontradiksi</th><th class="right">Skor</th></tr></thead>
<tbody>{"".join(compare_rows)}</tbody>
</table>
<p class="note">Rank = rank gabungan rata-rata peringkat tiga indikator: jeda 30–33 detik, duplikat laporan, dan QW5 kontradiksi. Skor 0–100; makin tinggi makin menonjol. Data 11–19 November 2026.</p>

<h2>4. Rank gabungan tiga indikator</h2>
<p>Rank gabungan dihitung dengan memberi bobot sama pada tiga indikator: <strong>persentase jeda 30–33 detik</strong>, <strong>persentase laporan duplikat</strong>, dan <strong>persentase QW5 kontradiktif</strong>. Untuk setiap indikator, polres diberi peringkat 1–35 (semakin tinggi nilai, semakin kecil peringkat). Rank gabungan = rata-rata ketiga peringkat; jika terjadi seri, QW5 dipakai sebagai penentu.</p>
<div class="chart">
{composite_svg}
<p class="caption">Gambar 5. Skor rank gabungan tiga indikator (0–100). Merah = tiga polres awal; kuning = Purbalingga, Surakarta, Batang.</p>
</div>
<table class="compact">
<colgroup><col style="width:8%"><col style="width:27%"><col style="width:12%"><col style="width:6%"><col style="width:12%"><col style="width:6%"><col style="width:12%"><col style="width:6%"><col style="width:11%"></colgroup>
<thead><tr><th>Rank</th><th>Polres</th><th class="right">Jeda 30–33</th><th class="right">Rank</th><th class="right">Duplikat</th><th class="right">Rank</th><th class="right">QW5</th><th class="right">Rank</th><th class="right">Skor</th></tr></thead>
<tbody>{"".join(six_rows)}</tbody>
</table>
<p class="note">Purbalingga, Surakarta, dan Batang adalah peringkat 1–3 pada jeda 30–33 detik. QW5 kontradiksi mereka juga tinggi (80,4%; 92,9%; 84,1%), sehingga ketiganya tetap masuk audit prioritas. Rank gabungan menempatkan Surakarta #1, Temanggung #2, Batang #3, Tegal Kota #4, Rembang #5, Pekalongan Kota #6, Purbalingga #7, Pekalongan #8, dan Karanganyar #9.</p>
<ul class="clean">
  <li><strong>Surakarta #1:</strong> QW5 92,9% (rank 4), duplikat 18,9% (rank 6), jeda 30–33 7,93% (rank 2).</li>
  <li><strong>Temanggung #2:</strong> QW5 86,9% (rank 7), duplikat 22,1% (rank 2), jeda 30–33 6,50% (rank 5).</li>
  <li><strong>Batang #3:</strong> QW5 84,1% (rank 10), duplikat 21,4% (rank 3), jeda 30–33 6,86% (rank 3).</li>
  <li><strong>Tegal Kota #4:</strong> QW5 93,8% (rank 3), duplikat 23,1% (rank 1), jeda 30–33 5,02% (rank 15).</li>
  <li><strong>Rembang #5:</strong> QW5 82,1% (rank 12), duplikat 18,9% (rank 7), jeda 30–33 6,50% (rank 4).</li>
  <li><strong>Pekalongan Kota #6:</strong> QW5 88,1% (rank 6), duplikat 18,2% (rank 10), jeda 30–33 5,56% (rank 11).</li>
  <li><strong>Purbalingga #7:</strong> QW5 80,4% (rank 15), duplikat 17,8% (rank 12), jeda 30–33 8,28% (rank 1).</li>
  <li><strong>Pekalongan #8:</strong> QW5 80,0% (rank 16), duplikat 20,7% (rank 4), jeda 30–33 6,22% (rank 8).</li>
  <li><strong>Karanganyar #9:</strong> QW5 78,7% (rank 18), duplikat 18,4% (rank 8), jeda 30–33 6,50% (rank 6).</li>
</ul>
<p><strong>Interpretasi:</strong> QW5 kontradiksi tidak lagi bisa disebut khas tiga polres; banyak polres juga di atas 70%. Karena itu rank gabungan memakai tiga indikator sekaligus. Surakarta dan Batang naik ke puncak karena kombinasi jeda 30–33 dan QW5; Temanggung tetap #2 karena duplikasi dan QW5; Purbalingga #7 karena jeda 30–33 tertinggi tetapi QW5/duplikat lebih rendah. Tegal Kota (#4) dan Pekalongan Kota (#6) juga perlu masuk daftar audit.</p>

<h2>5. Solusi dan pencegahan</h2>
<h3>5.1 Untuk anggota sebelum mengirim laporan</h3>
<ol class="steps">
  <li><strong>Laporkan hanya kegiatan nyata.</strong> Jangan mengisi kode QW yang tidak dilaksanakan hanya untuk memenuhi kewajiban harian.</li>
  <li><strong>Jangan menyalin laporan kemarin.</strong> Ubah tanggal, lokasi, jumlah personel, jumlah perkara, dan foto sesuai kegiatan hari ini.</li>
  <li><strong>Satu laporan, satu kegiatan utama.</strong> Gunakan ID laporan baru untuk kegiatan yang berbeda; jangan menggabungkan banyak kegiatan tanpa bukti.</li>
  <li><strong>Lampirkan bukti asli.</strong> Foto kegiatan harus baru, berbeda, dan dapat diakses; cantumkan tautan Drive yang benar, bukan tautan berulang.</li>
  <li><strong>Periksa sebelum kirim.</strong> Pastikan pelapor, satker, tanggal, QW, dan isian sudah benar; hindari klik kirim berulang.</li>
  <li><strong>Gunakan akun sendiri.</strong> Jangan meminjam akun atau nama pelapor anggota lain.</li>
  <li><strong>Jangan mengejar batas minimum.</strong> Jeda 30 detik bukan target; kecepatan pengiriman bukan ukuran kinerja.</li>
</ol>
<h3>5.2 Untuk satker dan pimpinan</h3>
<ol class="steps">
  <li><strong>Verifikasi harian:</strong> periksa laporan yang duplikat, QW5 kontradiktif, dan bukti foto/Drive sebelum disetujui.</li>
  <li><strong>Ubah orientasi target:</strong> dari jumlah laporan menjadi kualitas dan validitas kegiatan.</li>
  <li><strong>Uji petik:</strong> ambil 5–10 laporan per satker per minggu, cocokkan dengan register perkara, buku kegiatan, dan foto.</li>
  <li><strong>Beri umpan balik:</strong> tegur pengisian asal-asalan; apresiasi laporan yang lengkap dan faktual.</li>
</ol>
<h3>5.3 Untuk pengelola aplikasi</h3>
<ol class="steps">
  <li><strong>Deteksi duplikat di server:</strong> tolak atau tandai laporan dengan Satker + kode + isi sama pada hari yang sama, dan isi sama lintas hari.</li>
  <li><strong>Idempotency dan validasi:</strong> satu ID laporan tidak boleh dikirim ulang; nonaktifkan tombol kirim setelah berhasil; validasi QW5 hanya jika ada perkara.</li>
  <li><strong>Audit log:</strong> simpan timestamp server, IP, device ID, user agent, session, dan akun; jangan hanya bergantung pada UUID v7 dari klien.</li>
  <li><strong>Deteksi anomali:</strong> alarm untuk jeda &lt;3 detik, rangkaian 30–33 detik ≥3, dan duplikasi &gt;10% per satker.</li>
  <li><strong>Identitas pelapor:</strong> gunakan NRP/NIP, bukan nama; satu akun satu anggota; batasi login per perangkat.</li>
  <li><strong>Bukti dukung:</strong> wajibkan foto/tautan Drive untuk kegiatan lapangan; validasi tautan dan metadata foto.</li>
</ol>

<h2>6. Keterbatasan dan kesimpulan</h2>
<p>Metode UUID v7 menunjukkan <strong>0 laporan dengan jeda &lt;3 detik</strong> di seluruh 35 polres. Enam polres fokus memiliki jeda 30–33 detik dan duplikasi di atas median; setelah duplikat dikeluarkan, hanya <strong>3 dari 55 rangkaian</strong> yang nonduplikat. Rank gabungan menempatkan <strong>Surakarta #1, Temanggung #2, Batang #3, Tegal Kota #4, Rembang #5, Pekalongan Kota #6, Purbalingga #7, Pekalongan #8, Karanganyar #9</strong>. Karena itu audit prioritas harus mencakup Surakarta, Temanggung, Batang, Tegal Kota, Rembang, Pekalongan Kota, Purbalingga, Pekalongan, dan Karanganyar — bukan hanya tiga polres awal. Data ini paling tepat menunjukkan <strong>indikasi pengiriman berulang, duplikasi, dan QW5 tidak konsisten</strong>, bukan bukti bot. Untuk memastikan bot atau kecurangan, diperlukan log aplikasi dan verifikasi lapangan.</p>

{''.join(focus_sections)}

<p class="footer-note">Sumber: report-2026-11-10-to-2026-11-19.xlsx · {num(total_reports)} laporan unik, {num(total_rows_scope)} baris, 35 polres · Periode 10–19 November 2026 · UUID v7 dikonversi ke WIB · Memo internal untuk audit, bukan penilaian hukum.</p>

</body></html>"""

HTML_PATH.write_text(html_doc, encoding="utf-8")
print("HTML:", HTML_PATH)

subprocess.run(
    [
        str(CHROME),
        "--headless=new",
        "--disable-gpu",
        "--no-pdf-header-footer",
        "--run-all-compositor-stages-before-draw",
        "--virtual-time-budget=20000",
        f"--print-to-pdf={PDF_PATH}",
        HTML_PATH.as_uri(),
    ],
    check=True,
)
print("PDF:", PDF_PATH)
print("size:", PDF_PATH.stat().st_size)
