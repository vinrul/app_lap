import json
import re
import time
import zipfile
from collections import Counter
from datetime import date, datetime, timedelta
from lxml import etree

PATH = r"F:\app_lap\report-2026-11-10-to-2026-11-19.xlsx"
OUT = r"F:\app_lap\data\tmp_bot_nov\qw5_all.json"
MAIN = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
COL_RE = re.compile(r"([A-Z]+)")
QW5 = {"QW5.1", "QW5.3", "QW5.4"}

NEEDED = {
    "Tanggal kegiatan": "tanggal",
    "Tanggal lapor (WIB)": "waktu",
    "Satuan": "satuan",
    "Polda": "polda",
    "Polres": "polres",
    "Polsek": "polsek",
    "Kode": "kode",
    "Kegiatan": "kegiatan",
    "Pelapor": "pelapor",
    "ID laporan utama": "report_id",
    "Bukti dukung": "bukti",
    "Isian laporan": "isian",
}


def normalize_date(value):
    text = (value or "").strip()
    if len(text) >= 10 and text[4] == "-":
        try:
            date.fromisoformat(text[:10])
            return text[:10]
        except ValueError:
            pass
    for fmt in ("%d/%m/%Y", "%m/%d/%Y", "%d-%m-%Y", "%d.%m.%Y"):
        try:
            return datetime.strptime(text, fmt).strftime("%Y-%m-%d")
        except ValueError:
            continue
    if text.isdigit():
        serial = int(text)
        if 20_000 <= serial <= 80_000:
            return (date(1899, 12, 30) + timedelta(days=serial)).isoformat()
    return ""


def cell_text(cell):
    if cell.get("t") == "inlineStr":
        node = cell.find(f"{{{MAIN}}}is/{{{MAIN}}}t")
        return node.text if node is not None and node.text else ""
    node = cell.find(f"{{{MAIN}}}v")
    return node.text if node is not None and node.text else ""


rows = []
header_map = {}
total = 0
started = time.time()
with zipfile.ZipFile(PATH) as archive, archive.open("xl/worksheets/sheet1.xml") as handle:
    for _, elem in etree.iterparse(handle, tag=f"{{{MAIN}}}row", huge_tree=True):
        source_row = int(elem.get("r", "0"))
        values = {}
        for cell in elem.findall(f"{{{MAIN}}}c"):
            match = COL_RE.match(cell.get("r", ""))
            if match:
                values[match.group(1)] = cell_text(cell)

        if source_row == 1:
            by_name = {text.strip(): col for col, text in values.items()}
            header_map = {field: by_name[name] for name, field in NEEDED.items() if name in by_name}
            elem.clear()
            continue

        if not any(value.strip() for value in values.values()):
            elem.clear()
            continue

        total += 1
        kode = values.get(header_map.get("kode", ""), "").strip()
        if kode not in QW5:
            elem.clear()
            while elem.getprevious() is not None:
                del elem.getparent()[0]
            continue

        rows.append(
            {
                "source_row": source_row,
                "tanggal": normalize_date(values.get(header_map.get("tanggal", ""), "")),
                "waktu": values.get(header_map.get("waktu", ""), "").strip(),
                "satuan": values.get(header_map.get("satuan", ""), "").strip(),
                "polda": values.get(header_map.get("polda", ""), "").strip(),
                "polres": values.get(header_map.get("polres", ""), "").strip(),
                "polsek": values.get(header_map.get("polsek", ""), "").strip(),
                "kode": kode,
                "kegiatan": values.get(header_map.get("kegiatan", ""), "").strip(),
                "pelapor": values.get(header_map.get("pelapor", ""), "").strip(),
                "report_id": values.get(header_map.get("report_id", ""), "").strip(),
                "bukti": values.get(header_map.get("bukti", ""), "").strip(),
                "isian": values.get(header_map.get("isian", ""), "").strip(),
            }
        )
        if total % 25_000 == 0:
            print(f"{total:,} baris, {len(rows):,} QW5, {time.time() - started:.0f} detik", flush=True)

        elem.clear()
        while elem.getprevious() is not None:
            del elem.getparent()[0]

with open(OUT, "w", encoding="utf-8") as file:
    json.dump(rows, file, ensure_ascii=False)

print(f"Selesai: {total:,} baris, {len(rows):,} baris QW5, {time.time() - started:.0f} detik")
counts = Counter(row["polres"] for row in rows)
for polres, count in counts.most_common():
    polres_rows = [row for row in rows if row["polres"] == polres]
    negative = sum(1 for row in polres_rows if "penyidikan perkara?: Tidak" in row["isian"])
    print(f"  {count:4} | {negative:4} negatif | {negative / count * 100:5.1f}% | {polres}")
