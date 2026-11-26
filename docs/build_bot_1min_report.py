import html
import json
import os
import subprocess
from collections import Counter, defaultdict
from datetime import datetime

RUNS_PATH = "data/tmp_bot_1min/runs.json"
REPORTS_PATH = "data/tmp_bot_1min/reports.json"
OUT_HTML = "data/tmp_bot_1min/laporan_bot.html"
OUT_PDF = "Laporan-Indikasi-Bot-Pelapor-Beruntun-11-24-November-2026.pdf"
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"

WIB = "WIB"
TITLE = "Indikasi Bot: Pelapor Beruntun ≤60 Detik"


def esc(value) -> str:
    return html.escape(str(value or ""), quote=True)


def truncate(value: str, limit: int) -> str:
    value = (value or "").strip()
    if len(value) <= limit:
        return value
    return value[: limit - 1].rstrip() + "…"


def num(value: float, digits: int = 0) -> str:
    return (
        f"{value:,.{digits}f}"
        .replace(",", "X")
        .replace(".", ",")
        .replace("X", ".")
    )


def sec(value: float) -> str:
    return f"{value:.2f}".replace(".", ",")


def percent(part: int, total: int) -> str:
    if not total:
        return "—"
    return f"{part / total * 100:.2f}".replace(".", ",") + "%"


def short_time(wib: str) -> str:
    # 2026-11-13 10:00:00.431 -> 10:00:00.431
    return wib[11:] if len(wib) >= 23 else wib


def date_part(wib: str) -> str:
    return wib[:10]


def add_days(dates) -> str:
    dates = sorted(set(dates))
    return ", ".join(d[8:10] + "/" + d[5:7] for d in dates)


with open(RUNS_PATH, encoding="utf-8") as handle:
    runs = json.load(handle)
with open(REPORTS_PATH, encoding="utf-8") as handle:
    reports = json.load(handle)

# ---------------------------------------------------------------------------
# Validasi waktu: kolom Tanggal lapor (WIB) vs timestamp UUID v7.
# ---------------------------------------------------------------------------
time_mismatch = 0
time_checked = 0
for report in reports:
    raw = report.get("tanggal_lapor", "")
    try:
        column = datetime.strptime(raw[:16], "%Y-%m-%d %H:%M")
    except ValueError:
        continue
    uuid_time = datetime.strptime(report["wib"][:16], "%Y-%m-%d %H:%M")
    time_checked += 1
    if abs((uuid_time - column).total_seconds()) > 60:
        time_mismatch += 1

# ---------------------------------------------------------------------------
# Agregasi.
# ---------------------------------------------------------------------------
polres_stats = defaultdict(
    lambda: {
        "runs": 0,
        "reports": 0,
        "reporters": set(),
        "polseks": set(),
        "dates": set(),
        "gaps": [],
    }
)
polsek_stats = defaultdict(
    lambda: {"runs": 0, "reports": 0, "reporters": set(), "polres": "", "dates": set(), "gaps": []}
)
reporter_stats = defaultdict(
    lambda: {
        "name": "",
        "satuan": set(),
        "polres": "",
        "runs": 0,
        "reports": 0,
        "dates": set(),
        "gaps": [],
        "start": None,
        "end": None,
    }
)
qw_counts = Counter()
gap_buckets = Counter()
all_gaps = []
run_sizes = Counter()

for index, run in enumerate(runs):
    run["index"] = index + 1
    polres = run["polres"]
    pstat = polres_stats[polres]
    pstat["runs"] += 1
    pstat["reports"] += run["count"]
    pstat["reporters"].add(run["reporter_key"])
    pstat["dates"].add(date_part(run["start_wib"]))
    for polsek in run["polseks"]:
        pstat["polseks"].add(polsek)
        sstat = polsek_stats[(polres, polsek)]
        sstat["runs"] += 1
        sstat["reports"] += run["count"]
        sstat["reporters"].add(run["reporter_key"])
        sstat["polres"] = polres
        sstat["dates"].add(date_part(run["start_wib"]))
    rstat_key = (run["reporter_key"], run["polres_key"])
    rstat = reporter_stats[rstat_key]
    rstat["name"] = run["reporter"]
    rstat["satuan"].update(run["units"])
    rstat["polres"] = polres
    rstat["runs"] += 1
    rstat["reports"] += run["count"]
    rstat["dates"].add(date_part(run["start_wib"]))
    rstat["start"] = run["start_ms"] if rstat["start"] is None else min(rstat["start"], run["start_ms"])
    rstat["end"] = run["end_ms"] if rstat["end"] is None else max(rstat["end"], run["end_ms"])
    run_sizes[run["count"]] += 1
    for report in run["reports"]:
        qw_counts[report["quick_wins"] or "Lainnya"] += 1
    gaps = [report["gap_s"] for report in run["reports"][1:]]
    all_gaps.extend(gaps)
    pstat["gaps"].extend(gaps)
    for polsek in run["polseks"]:
        polsek_stats[(polres, polsek)]["gaps"].extend(gaps)
    rstat["gaps"].extend(gaps)
    for gap in gaps:
        if gap < 35:
            gap_buckets["30–35 dtk"] += 1
        elif gap < 40:
            gap_buckets["35–40 dtk"] += 1
        elif gap < 45:
            gap_buckets["40–45 dtk"] += 1
        elif gap < 50:
            gap_buckets["45–50 dtk"] += 1
        elif gap < 55:
            gap_buckets["50–55 dtk"] += 1
        else:
            gap_buckets["55–60 dtk"] += 1

polres_rank = sorted(
    polres_stats.items(),
    key=lambda item: (-item[1]["reports"], -item[1]["runs"], item[0]),
)
polsek_rank = sorted(
    polsek_stats.items(),
    key=lambda item: (-item[1]["reports"], -item[1]["runs"], item[0][0], item[0][1]),
)
reporter_rank = sorted(
    reporter_stats.values(),
    key=lambda item: (-item["reports"], -item["runs"], item["name"]),
)

total_reports = len(reports)
total_polres = len({report["polres"] for report in reports if report["polres"]})
flagged_reports = sum(run["count"] for run in runs)
total_gap = sum(all_gaps) if all_gaps else 0
avg_gap = total_gap / len(all_gaps) if all_gaps else 0

# ---------------------------------------------------------------------------
# HTML.
# ---------------------------------------------------------------------------
CSS = r"""
:root { --ink:#1f2a24; --forest:#143d30; --gold:#b8894b; --red:#a33a3a; --soft:#5c6a63; --muted:#7a857f; --line:#cfd8cf; --panel:#f5f8f3; --pale:#eef3ee; }
@page { size: A4 landscape; margin: 10mm 9mm 17mm; }
* { box-sizing: border-box; }
html, body { margin:0; padding:0; }
body { font-family:"Segoe UI", Arial, sans-serif; font-size:8.4pt; line-height:1.38; color:var(--ink); -webkit-print-color-adjust:exact; print-color-adjust:exact; }
h1,h2,h3,h4,p { margin:0; }
h1 { font-family:"Segoe UI Semibold","Segoe UI",Arial,sans-serif; font-size:25pt; line-height:1.08; color:#fff; }
h2 { font-size:14pt; color:var(--forest); margin:0 0 2mm; }
h3 { font-size:11pt; color:var(--forest); margin:0 0 1.4mm; }
h4 { font-size:9pt; color:var(--forest); margin:0 0 1mm; }
.mono { font-family:Consolas,"Courier New",monospace; }
.footer { display:none; }
.cover { page-break-after:always; background:linear-gradient(135deg,#0d2a21 0%,#143d30 58%,#1d4a3a 100%); color:#fff; padding:16mm 16mm 12mm; height:180mm; position:relative; overflow:hidden; }
.cover::after { content:""; position:absolute; right:-30mm; top:-30mm; width:110mm; height:110mm; border-radius:50%; background:rgba(184,137,75,.12); }
.cover .kicker { font-size:8pt; font-weight:700; letter-spacing:.22em; color:#e6d3b3; text-transform:uppercase; }
.cover .sub { margin-top:4mm; font-size:11pt; color:#cfe0d6; max-width:190mm; }
.cover .small { color:#cfe0d6; }
.cover .meta { position:absolute; left:16mm; bottom:14mm; right:16mm; display:flex; gap:6mm; font-size:7.2pt; color:#cfe0d6; }
.cover .meta div { border-top:1.4pt solid var(--gold); padding-top:2mm; min-width:38mm; }
.cover .meta strong { display:block; color:#fff; font-size:8pt; margin-bottom:.8mm; }
.cover .criteria { margin-top:8mm; border:1pt solid rgba(255,255,255,.25); border-radius:2mm; padding:3.5mm 4mm; background:rgba(255,255,255,.06); font-size:8.2pt; color:#e3ece6; }
.cover .criteria strong { color:#e6d3b3; }
.cards { display:flex; gap:3mm; margin:3mm 0 4mm; }
.card { flex:1; border:.5pt solid var(--line); border-radius:2mm; background:var(--panel); padding:2.6mm 3mm; }
.card .label { display:block; font-size:6.5pt; font-weight:700; letter-spacing:.06em; text-transform:uppercase; color:var(--muted); }
.card strong { display:block; margin-top:.8mm; font-size:15pt; line-height:1; color:var(--forest); }
.card small { display:block; margin-top:1mm; font-size:6.6pt; color:var(--soft); }
.note { border-left:1.5mm solid var(--gold); background:#fbf5e6; padding:2.5mm 3mm; font-size:7.8pt; color:#5b4a35; margin:3mm 0; }
.note strong { color:#7b5a2f; }
.red-note { border-left:1.5mm solid var(--red); background:#fbf0ee; color:#6b2f2b; }
.section { page-break-before:always; }
table { width:100%; border-collapse:collapse; table-layout:fixed; }
th,td { border:.4pt solid var(--line); padding:1.1mm 1.3mm; vertical-align:top; }
th { background:var(--forest); color:#fff; font-size:6.8pt; font-weight:700; text-align:left; letter-spacing:.03em; text-transform:uppercase; }
tbody tr:nth-child(even) td { background:var(--panel); }
thead { display:table-header-group; }
tr { page-break-inside:avoid; }
.num { text-align:right; white-space:nowrap; }
.nowrap { white-space:nowrap; }
.rank-table { font-size:7.4pt; }
.rank-table td { padding:1.2mm 1.4mm; }
.rank-1 td { background:#fbf3e3 !important; }
.rank-1 strong { color:#8a5a1f; }
.run { margin:0 0 4mm; border:.5pt solid var(--line); border-radius:2mm; overflow:hidden; }
.run-head { background:var(--pale); padding:2.4mm 3mm; border-bottom:.5pt solid var(--line); display:flex; justify-content:space-between; gap:4mm; break-after:avoid; page-break-after:avoid; }
.run-head .who { min-width:0; }
.run-head .who strong { font-size:9.6pt; color:var(--forest); }
.run-head .who span { display:block; font-size:7.2pt; color:var(--soft); margin-top:.4mm; }
.run-head .badge { flex:none; text-align:right; }
.badge { display:inline-block; border-radius:1.4mm; padding:.6mm 1.8mm; font-size:6.8pt; font-weight:700; }
.badge-red { background:#f6e2df; color:#8f2f28; }
.badge-gold { background:#f8eedb; color:#7b5a2f; }
.badge-green { background:#e4efe6; color:#255a36; }
.run-meta { display:flex; flex-wrap:wrap; gap:2mm 6mm; padding:2mm 3mm; font-size:7pt; color:var(--soft); border-bottom:.4pt solid var(--line); break-after:avoid; page-break-after:avoid; }
.run-meta b { color:var(--ink); }
.detail-table { font-size:6.7pt; }
.detail-table td { padding:.9mm 1.1mm; }
.kode { font-family:Consolas,monospace; font-weight:700; font-size:6.6pt; color:var(--forest); }
.isian { color:var(--muted); font-size:6.2pt; margin-top:.5mm; }
.gap { font-family:Consolas,monospace; font-size:6.4pt; }
.bars { margin:2mm 0 4mm; }
.bar-row { display:flex; align-items:center; gap:2.5mm; margin-bottom:1.4mm; font-size:7.2pt; }
.bar-label { width:26mm; color:var(--soft); }
.bar-track { flex:1; display:block; height:3.4mm; background:#edf1ed; border-radius:1mm; overflow:hidden; }
.bar-fill { display:block; height:100%; background:var(--forest); }
.bar-fill.gold { background:var(--gold); }
.bar-value { width:18mm; text-align:right; color:var(--ink); font-weight:700; }
.small { font-size:7pt; color:var(--soft); }
.two-col { display:flex; gap:5mm; }
.two-col > div { flex:1; }
ul.list { margin:1mm 0 0 4mm; padding:0; }
ul.list li { margin-bottom:1.2mm; }
.polres-head { margin:4mm 0 2mm; padding:2.2mm 3mm; background:var(--forest); color:#fff; border-radius:1.6mm; display:flex; justify-content:space-between; align-items:center; break-after:avoid; page-break-after:avoid; }
.polres-head h2 { color:#fff; margin:0; font-size:13pt; }
.polres-head span { font-size:7.6pt; color:#cfe0d6; }
.legend { font-size:6.8pt; color:var(--muted); margin-top:1mm; }
"""


def bar_rows(counter: Counter, total: int, gold: bool = False, by_label: bool = False) -> str:
    rows = []
    items = list(counter.items())
    if by_label:
        items.sort(key=lambda item: item[0])
    else:
        items.sort(key=lambda item: (-item[1], item[0]))
    for label, value in items:
        width = value / total * 100 if total else 0
        rows.append(
            f'<div class="bar-row"><span class="bar-label">{esc(label)}</span>'
            f'<span class="bar-track"><span class="bar-fill{" gold" if gold else ""}" '
            f'style="width:{width:.1f}%"></span></span>'
            f'<span class="bar-value">{num(value)}</span></div>'
        )
    return "".join(rows)


def ranking_polres() -> str:
    rows = []
    for rank, (label, stat) in enumerate(polres_rank, start=1):
        gaps = stat["gaps"]
        avg = sum(gaps) / len(gaps) if gaps else 0
        rows.append(
            f'<tr class="{"rank-1" if rank == 1 else ""}">'
            f'<td class="num">{rank}</td>'
            f"<td><strong>{esc(label)}</strong></td>"
            f'<td class="num">{num(stat["runs"])}</td>'
            f'<td class="num"><strong>{num(stat["reports"])}</strong></td>'
            f'<td class="num">{num(len(stat["reporters"]))}</td>'
            f'<td class="num">{num(len(stat["polseks"]))}</td>'
            f'<td class="num">{sec(avg)} dtk</td>'
            f'<td>{add_days(stat["dates"])}</td>'
            "</tr>"
        )
    return "".join(rows)


def ranking_polsek() -> str:
    rows = []
    for rank, ((polres, polsek), stat) in enumerate(polsek_rank, start=1):
        gaps = stat["gaps"]
        avg = sum(gaps) / len(gaps) if gaps else 0
        rows.append(
            f'<tr class="{"rank-1" if rank == 1 else ""}">'
            f'<td class="num">{rank}</td>'
            f"<td><strong>{esc(polsek)}</strong></td>"
            f"<td>{esc(polres)}</td>"
            f'<td class="num">{num(stat["runs"])}</td>'
            f'<td class="num"><strong>{num(stat["reports"])}</strong></td>'
            f'<td class="num">{num(len(stat["reporters"]))}</td>'
            f'<td class="num">{sec(avg)} dtk</td>'
            f'<td>{add_days(stat["dates"])}</td>'
            "</tr>"
        )
    return "".join(rows)


def ranking_reporter() -> str:
    rows = []
    for rank, stat in enumerate(reporter_rank, start=1):
        gaps = stat["gaps"]
        low = min(gaps) if gaps else 0
        high = max(gaps) if gaps else 0
        rows.append(
            f'<tr class="{"rank-1" if rank == 1 else ""}">'
            f'<td class="num">{rank}</td>'
            f"<td><strong>{esc(stat['name'])}</strong></td>"
            f"<td>{esc(', '.join(sorted(stat['satuan'])))}</td>"
            f"<td>{esc(stat['polres'])}</td>"
            f'<td class="num">{num(stat["runs"])}</td>'
            f'<td class="num"><strong>{num(stat["reports"])}</strong></td>'
            f'<td class="num">{sec(low)}–{sec(high)}</td>'
            f'<td>{add_days(stat["dates"])}</td>'
            "</tr>"
        )
    return "".join(rows)


def detail_table(run: dict) -> str:
    rows = []
    for report in run["reports"]:
        codes = report.get("codes") or [report.get("kode", "")]
        code_list = [code for code in codes if code]
        if len(code_list) > 6:
            code_text = " · ".join(code_list[:6]) + f" +{len(code_list) - 6}"
        else:
            code_text = " · ".join(code_list)
        activities = [a for a in (report.get("activities") or [report.get("kegiatan", "")]) if a]
        activity_html = "<br>".join(esc(truncate(a, 150)) for a in activities[:3])
        if len(activities) > 3:
            activity_html += f"<br><em>+{len(activities) - 3} kegiatan lain</em>"
        isian = truncate(report.get("isian", ""), 220)
        if isian:
            activity_html += f'<div class="isian">{esc(isian)}</div>'
        gap = "—" if report["seq"] == 1 else sec(report["gap_s"])
        rows.append(
            "<tr>"
            f'<td class="num">{report["seq"]}</td>'
            f'<td class="num gap">{gap}</td>'
            f'<td class="nowrap">{esc(short_time(report["wib"]))}</td>'
            f'<td class="kode">{esc(code_text)}</td>'
            f"<td>{activity_html}</td>"
            f"<td>{esc(report.get('satuan', ''))}</td>"
            f'<td class="num">{report["excel_row"]}</td>'
            f'<td class="mono">{esc(report["uuid"])}</td>'
            "</tr>"
        )
    return "".join(rows)


def run_block(run: dict) -> str:
    units = ", ".join(run["units"])
    polseks = ", ".join(run["polseks"])
    codes = ", ".join(run["codes"])
    return f"""
<div class="run">
  <div class="run-head">
    <div class="who">
      <strong>{esc(run['reporter'])}</strong>
      <span>{esc(units)}{f' · Polsek: {esc(polseks)}' if polseks and polseks != units else ''} · {esc(run['polres'])}</span>
    </div>
    <div class="badge badge-red">{run['count']} laporan · {sec(run['duration_s'])} dtk</div>
  </div>
  <div class="run-meta">
    <span>Mulai <b>{esc(run['start_wib'])}</b></span>
    <span>Selesai <b>{esc(run['end_wib'])}</b></span>
    <span>Jeda min/median/maks <b>{sec(run['gap_min_s'])} / {sec(run['gap_median_s'])} / {sec(run['gap_max_s'])} dtk</b></span>
    <span>Kode QW <b>{esc(codes)}</b></span>
  </div>
  <table class="detail-table">
    <colgroup>
      <col style="width:3%"><col style="width:4%"><col style="width:8%"><col style="width:12%">
      <col style="width:34%"><col style="width:12%"><col style="width:3%"><col style="width:24%">
    </colgroup>
    <thead><tr>
      <th class="num">No</th><th class="num">Jeda</th><th>Waktu WIB<br>(UUID v7)</th><th>Kode / QW</th>
      <th>Kegiatan dan isian laporan</th><th>Satuan</th><th class="num">Baris</th><th>UUID laporan</th>
    </tr></thead>
    <tbody>{detail_table(run)}</tbody>
  </table>
</div>
"""


def polres_detail() -> str:
    blocks = []
    for rank, (label, stat) in enumerate(polres_rank, start=1):
        polres_runs = sorted(
            [run for run in runs if run["polres"] == label], key=lambda run: run["start_ms"]
        )
        blocks.append(
            f'<div class="polres-head"><h2>A.{rank} {esc(label)}</h2>'
            f'<span>{num(stat["reports"])} laporan · {num(stat["runs"])} rangkaian · '
            f'{num(len(stat["reporters"]))} pelapor · {num(len(stat["polseks"]))} polsek</span></div>'
        )
        for run in polres_runs:
            blocks.append(run_block(run))
    return "".join(blocks)


def runs_summary() -> str:
    rows = []
    for run in sorted(runs, key=lambda run: run["start_ms"]):
        rows.append(
            "<tr>"
            f'<td class="nowrap">{esc(run["start_wib"][:10])}</td>'
            f"<td><strong>{esc(run['reporter'])}</strong></td>"
            f"<td>{esc(', '.join(run['units']))}</td>"
            f"<td>{esc(run['polres'])}</td>"
            f'<td class="num">{run["count"]}</td>'
            f'<td class="nowrap">{esc(short_time(run["start_wib"]))}–{esc(short_time(run["end_wib"]))}</td>'
            f'<td class="num">{sec(run["duration_s"])}</td>'
            f'<td class="num">{sec(run["gap_min_s"])}–{sec(run["gap_max_s"])}</td>'
            f"<td>{esc(', '.join(run['codes']))}</td>"
            "</tr>"
        )
    return "".join(rows)


polres_rows = ranking_polres()
polsek_rows = ranking_polsek()
reporter_rows = ranking_reporter()
detail_blocks = polres_detail()
run_rows = runs_summary()

top_run = max(runs, key=lambda run: run["count"])
top_polres = polres_rank[0]
second_polres = polres_rank[1]
third_polres = polres_rank[2]

summary_html = f"""
<p class="small">Polda Jawa Tengah · 11–24 November 2026 · Sumber <strong>report-2026-11-11-to-2026-11-24.xlsx</strong></p>
<h1>Indikasi Bot:<br>Pelapor Beruntun ≤60 Detik</h1>
<p class="sub">Analisis {num(total_reports)} laporan unik berbasis UUID v7 untuk menemukan pelapor yang mengirim
≥{num(10)} laporan secara beruntun dengan jeda antar-laporan ≤60 detik.</p>
<div class="criteria">
  <strong>Kriteria temuan</strong><br>
  1) pelapor sama (nama dan polres sama); 2) setiap jeda antar-laporan ≤60 detik berdasarkan timestamp UUID v7;
  3) satu rangkaian memuat minimal 10 laporan. Rangkaian diambil yang terpanjang (maximal run).
</div>
"""

exec_summary = f"""
<div class="cards">
  <div class="card"><span class="label">Laporan unik diperiksa</span><strong>{num(total_reports)}</strong><small>3.634 baris duplikat digabung</small></div>
  <div class="card"><span class="label">Rangkaian terindikasi</span><strong>{num(len(runs))}</strong><small>≥10 laporan beruntun</small></div>
  <div class="card"><span class="label">Laporan terindikasi</span><strong>{num(flagged_reports)}</strong><small>{percent(flagged_reports, total_reports)} dari laporan unik</small></div>
  <div class="card"><span class="label">Pelapor terindikasi</span><strong>{num(len(reporter_stats))}</strong><small>di {num(len(polres_stats))} polres · {num(len(polsek_stats))} polsek</small></div>
  <div class="card"><span class="label">Jeda rata-rata</span><strong>{sec(avg_gap)} dtk</strong><small>rentang {sec(min(all_gaps))}–{sec(max(all_gaps))} dtk</small></div>
</div>

<h2>Ringkasan eksekutif</h2>
<p>Ditemukan <strong>{num(len(runs))} rangkaian</strong> dari <strong>{num(len(reporter_stats))} pelapor</strong>
yang memenuhi kriteria. Seluruh rangkaian menghabiskan <strong>{num(flagged_reports)} laporan</strong>
({percent(flagged_reports, total_reports)} dari {num(total_reports)} laporan unik). Rangkaian terbesar:
<strong>{esc(top_run['reporter'])}</strong> di {esc(', '.join(top_run['units']))} ({esc(top_run['polres'])})
mengirim <strong>{top_run['count']} laporan</strong> dalam {sec(top_run['duration_s'])} detik
({esc(short_time(top_run['start_wib']))}–{esc(short_time(top_run['end_wib']))} WIB).</p>

<div class="two-col" style="margin-top:3mm">
  <div>
    <h3>Temuan utama</h3>
    <ul class="list">
      <li>Semua jeda berada pada <strong>{sec(min(all_gaps))}–{sec(max(all_gaps))} detik</strong>;
          tidak ada jeda di bawah 30 detik. Pola ini konsisten dengan pengiriman cepat yang
          menempel batas minimum aplikasi (30 detik), bukan ritme pelaporan lapangan normal.</li>
      <li>Setiap rangkaian memuat <strong>{min(run_sizes)}–{max(run_sizes)} laporan</strong> dengan
          <strong>{min(len(run['codes']) for run in runs)}–{max(len(run['codes']) for run in runs)} kode QW berbeda</strong>
          dalam hitungan menit. Satu pelapor mengisi banyak program sekaligus.</li>
      <li>Peringkat polres: <strong>{esc(top_polres[0])}</strong> ({num(top_polres[1]['reports'])} laporan,
          {num(top_polres[1]['runs'])} rangkaian), disusul <strong>{esc(second_polres[0])}</strong>
          ({num(second_polres[1]['reports'])}) dan <strong>{esc(third_polres[0])}</strong>
          ({num(third_polres[1]['reports'])}).</li>
      <li>Ada {num(len(polsek_stats))} polsek terlibat. Rangkaian terpusat pada satu pelapor per polsek;
          nama pelapor yang sama muncul berulang pada tanggal berbeda.</li>
      <li>Dua ejaan nama di Polsek Tretep (<em>AIPDA MMUH TULUS HASIM</em> dan
          <em>AIPDA MUH TULUS HASIM</em>) sangat mungkin orang yang sama; keduanya dihitung
          sebagai pelapor terpisah sesuai nama pada laporan.</li>
    </ul>
  </div>
  <div>
    <h3>Peringkat polres (laporan terindikasi)</h3>
    <div class="bars">{bar_rows(Counter({label: stat['reports'] for label, stat in polres_rank}), flagged_reports)}</div>
    <h3>Sebaran kelompok QW</h3>
    <div class="bars">{bar_rows(qw_counts, flagged_reports, gold=True, by_label=True)}</div>
  </div>
</div>

<div class="note red-note">
<strong>Batasan:</strong> temuan ini adalah <em>indikasi</em>, bukan bukti bot. UUID v7 hanya membuktikan
waktu pembuatan laporan, bukan perangkat atau pelakunya. Verifikasi memerlukan log aplikasi
(IP, device ID, sesi, user agent), pemeriksaan bukti kegiatan, dan konfirmasi satuan.
</div>
"""

method_html = f"""
<div class="section">
<h2>1. Metode dan validasi</h2>
<div class="two-col">
<div>
<h3>Sumber dan pembersihan</h3>
<ul class="list">
  <li>Sheet <strong>Data</strong> pada <span class="mono">report-2026-11-11-to-2026-11-24.xlsx</span>.</li>
  <li>Total <strong>{num(total_reports)} UUID v7 unik</strong>; {num(3634)} baris duplikat UUID digabung
      (kode/kegiatan tambahan tetap dihitung pada laporan yang sama).</li>
  <li>Tidak ada UUID tidak valid; seluruh laporan memiliki timestamp v7.</li>
  <li>Waktu diambil dari <strong>48 bit pertama UUID v7</strong> (Unix milidetik UTC), lalu
      dikonversi ke <strong>WIB (UTC+7)</strong>. Kolom “Tanggal lapor (WIB)” tidak dipakai
      sebagai dasar jeda.</li>
  <li>Validasi silang: {num(time_mismatch)} dari {num(time_checked)} laporan memiliki selisih
      &gt;1 menit antara UUID v7 dan kolom Tanggal lapor.</li>
</ul>
</div>
<div>
<h3>Algoritma</h3>
<ul class="list">
  <li>Kelompokkan laporan berdasarkan <strong>nama pelapor</strong> dan <strong>polres</strong>
      (nama dinormalisasi: spasi ganda dan kapitalisasi diseragamkan).</li>
  <li>Urutkan tiap kelompok berdasarkan timestamp UUID v7.</li>
  <li>Bentuk rangkaian selama jeda antar-laporan <strong>≤60.000 ms</strong>. Rangkaian dianggap
      temuan bila berisi <strong>≥10 laporan</strong>.</li>
  <li>Rangkaian diambil yang terpanjang; laporan yang sama tidak dihitung dua kali.</li>
  <li>Detail memuat pelapor, satuan/polsek, waktu WIB tiap laporan, jeda, kode QW, kegiatan,
      isian, nomor baris Excel, dan UUID.</li>
</ul>
</div>
</div>

<h3 style="margin-top:5mm">Sebaran jeda antar-laporan</h3>
<div class="bars">{bar_rows(gap_buckets, sum(gap_buckets.values()), gold=True, by_label=True)}</div>
<p class="small">Tidak ada jeda &lt;30 detik. Ini menunjukkan rangkaian berada di atas ambang minimum
aplikasi (sekitar 30 detik) dan di bawah 60 detik.</p>

<div class="note">
<strong>Catatan data:</strong> {num(3634)} baris dengan UUID sama dianggap satu laporan.
Bila satu laporan memuat beberapa kegiatan, seluruh kodenya digabungkan pada baris laporan tersebut.
</div>
</div>
"""

ranking_html = f"""
<div class="section">
<h2>2. Peringkat per polres</h2>
<p class="small">Diurutkan dari jumlah laporan terindikasi terbanyak, lalu jumlah rangkaian. 
{num(total_polres - len(polres_rank))} polres lain tidak memiliki rangkaian yang memenuhi kriteria.</p>
<table class="rank-table">
<colgroup><col style="width:5%"><col style="width:24%"><col style="width:8%"><col style="width:9%"><col style="width:8%"><col style="width:8%"><col style="width:10%"><col style="width:28%"></colgroup>
<thead><tr><th class="num">Rank</th><th>Polres</th><th class="num">Rangkaian</th><th class="num">Laporan</th><th class="num">Pelapor</th><th class="num">Polsek</th><th class="num">Jeda rata-rata</th><th>Tanggal</th></tr></thead>
<tbody>{polres_rows}</tbody>
</table>

<h2 style="margin-top:7mm">3. Peringkat per polsek</h2>
<p class="small">Satuan pelaksana yang melaporkan rangkaian. Satu polsek dapat memiliki lebih dari satu pelapor. 
Hanya polsek dengan temuan yang ditampilkan; polsek lain tidak memiliki rangkaian.</p>
<table class="rank-table">
<colgroup><col style="width:5%"><col style="width:22%"><col style="width:23%"><col style="width:8%"><col style="width:9%"><col style="width:8%"><col style="width:10%"><col style="width:15%"></colgroup>
<thead><tr><th class="num">Rank</th><th>Polsek</th><th>Polres</th><th class="num">Rangkaian</th><th class="num">Laporan</th><th class="num">Pelapor</th><th class="num">Jeda rata-rata</th><th>Tanggal</th></tr></thead>
<tbody>{polsek_rows}</tbody>
</table>
</div>
"""

reporter_html = f"""
<div class="section">
<h2>4. Peringkat pelapor</h2>
<p class="small">Nama pelapor sesuai kolom Pelapor pada laporan. Nama yang berbeda ejaan dihitung terpisah.</p>
<table class="rank-table">
<colgroup><col style="width:5%"><col style="width:22%"><col style="width:20%"><col style="width:20%"><col style="width:7%"><col style="width:8%"><col style="width:9%"><col style="width:9%"></colgroup>
<thead><tr><th class="num">Rank</th><th>Pelapor</th><th>Satuan</th><th>Polres</th><th class="num">Rangkaian</th><th class="num">Laporan</th><th class="num">Jeda min–maks</th><th>Tanggal</th></tr></thead>
<tbody>{reporter_rows}</tbody>
</table>

<h3 style="margin-top:7mm">Daftar rangkaian</h3>
<table class="rank-table" style="font-size:6.8pt">
<colgroup><col style="width:7%"><col style="width:18%"><col style="width:12%"><col style="width:15%"><col style="width:5%"><col style="width:12%"><col style="width:6%"><col style="width:8%"><col style="width:17%"></colgroup>
<thead><tr><th>Tanggal</th><th>Pelapor</th><th>Satuan</th><th>Polres</th><th class="num">Jml</th><th>Waktu</th><th class="num">Durasi</th><th class="num">Jeda</th><th>Kode QW</th></tr></thead>
<tbody>{run_rows}</tbody>
</table>
</div>
"""

detail_html = f"""
<div class="section">
<h2>5. Detail temuan per polres</h2>
<p class="small">Setiap rangkaian menampilkan pelapor, satuan/polsek, waktu UUID v7 dalam WIB,
jeda antar-laporan, kode QW, kegiatan, isian, nomor baris Excel, dan UUID laporan.</p>
{detail_blocks}
</div>
"""

closing_html = f"""
<div class="section">
<h2>6. Tindak lanjut yang disarankan</h2>
<ul class="list">
  <li><strong>Verifikasi lapangan:</strong> cocokkan {num(flagged_reports)} laporan pada {num(len(polsek_stats))} polsek
      dengan register kegiatan, foto/bukti dukung, dan keberadaan pelapor pada jam tersebut.</li>
  <li><strong>Audit log aplikasi:</strong> tarik IP, device ID, sesi, user agent, dan pola login untuk
      {num(len(reporter_stats))} pelapor; bandingkan apakah beberapa laporan berasal dari satu perangkat.</li>
  <li><strong>Uji petik per pelapor:</strong> mulai dari rangkaian terbesar (Polsek Tretep, 17 laporan
      dalam {sec(top_run['duration_s'])} detik) lalu tiga rangkaian berikutnya.</li>
  <li><strong>Pembinaan:</strong> ingatkan bahwa jeda 30 detik bukan target; kualitas dan kebenaran
      kegiatan lebih penting daripada jumlah laporan.</li>
  <li><strong>Pencegahan:</strong> aktifkan idempotency per UUID, validasi kegiatan ganda dalam waktu
      singkat, dan alarm untuk ≥10 laporan dengan jeda ≤60 detik.</li>
</ul>

<h3 style="margin-top:6mm">Keterbatasan</h3>
<p class="small">Analisis ini hanya memakai data laporan (UUID v7 dan atribut laporan). Tidak ada
kesimpulan tentang niat atau identitas perangkat. Nama pelapor dapat berupa ejaan berbeda untuk
orang yang sama; sebaliknya, nama yang sama belum tentu akun yang sama. Keputusan disiplin
memerlukan verifikasi bukti dan log aplikasi.</p>

<div class="note">
<strong>Definisi temuan:</strong> pelapor sama (nama + polres), setiap jeda antar-laporan ≤60 detik,
dan satu rangkaian ≥10 laporan. Angka “laporan” adalah UUID v7 unik; baris duplikat UUID tidak
dihitung dua kali.
</div>
</div>
"""

HTML = f"""<!doctype html>
<html lang="id"><head><meta charset="utf-8">
<title>{esc(TITLE)}</title>
<style>{CSS}</style></head>
<body>

<section class="cover">
  <div class="kicker">Laporan Analisis Internal</div>
  <div style="margin-top:6mm">{summary_html}</div>
  <div class="meta">
    <div><strong>Sumber</strong>report-2026-11-11-to-2026-11-24.xlsx</div>
    <div><strong>Cakupan</strong>{num(total_reports)} laporan unik · {num(total_polres)} polres</div>
    <div><strong>Metode</strong>UUID v7 · jeda ≤60 detik · ≥10 laporan</div>
    <div><strong>Sifat</strong>Terbatas — audit internal</div>
  </div>
</section>

<section>
{exec_summary}
</section>

{method_html}
{ranking_html}
{reporter_html}
{detail_html}
{closing_html}
</body></html>"""

with open(OUT_HTML, "w", encoding="utf-8") as handle:
    handle.write(HTML)

print("HTML:", OUT_HTML)
print("laporan unik:", total_reports)
print("rangkaian:", len(runs), "laporan:", flagged_reports)
print("gap min/max/avg:", sec(min(all_gaps)), sec(max(all_gaps)), sec(avg_gap))
print("mismatch waktu >1 menit:", time_mismatch, "dari", time_checked)
print("polres:", [(label, stat["reports"]) for label, stat in polres_rank])
print("polsek:", [((polres, polsek), stat["reports"]) for (polres, polsek), stat in polsek_rank])
print("reporter:", [(stat["name"], stat["reports"], stat["runs"]) for stat in reporter_rank])

if not os.path.exists(CHROME):
    raise SystemExit(f"Chrome tidak ditemukan: {CHROME}")

out_pdf_abs = os.path.abspath(OUT_PDF)
html_abs = os.path.abspath(OUT_HTML)
file_url = "file:///" + html_abs.replace("\\", "/")
profile = os.path.abspath("data/tmp_bot_1min/chrome-profile")
subprocess.run(
    [
        CHROME,
        "--headless=new",
        "--disable-gpu",
        "--disable-extensions",
        "--hide-scrollbars",
        "--no-first-run",
        "--no-default-browser-check",
        f"--user-data-dir={profile}",
        "--no-pdf-header-footer",
        "--run-all-compositor-stages-before-draw",
        "--virtual-time-budget=12000",
        f"--print-to-pdf={out_pdf_abs}",
        file_url,
    ],
    check=True,
)
print("PDF:", out_pdf_abs, os.path.getsize(out_pdf_abs), "bytes")
