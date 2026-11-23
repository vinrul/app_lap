import json
import re
import time
import zipfile
from collections import Counter
from datetime import date, datetime, timedelta
from lxml import etree

PATH = r"F:\app_lap\report-2026-11-10-to-2026-11-19.xlsx"
OUT = r"F:\app_lap\data\tmp_bot_nov\rows6.json"
MAIN = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
COL_RE = re.compile(r"([A-Z]+)")
SIX = {
    "Polres Temanggung",
    "Polres Rembang",
    "Polres Karanganyar",
    "Polres Purbalingga",
    "Polresta Surakarta",
    "Polresta Batang",
}

NEEDED = {
    "Tanggal kegiatan": "tanggal",
    "Tanggal lapor (WIB)": "waktu",
    "Satuan": "satuan",
    "Tingkat": "tingkat",
    "Polda": "polda",
    "Polres": "polres",
    "Polsek": "polsek",
    "Kode": "kode",
    "Kegiatan": "kegiatan",
    "Quick Wins": "quick_wins",
    "Pelapor": "pelapor",
    "ID laporan utama": "report_id",
    "Kegiatan ke-": "kegiatan_ke",
    "Bukti dukung": "bukti",
    "Isian laporan": "isian",
}


def normalize_date(value):
    text = (value or "").strip()
    if not text:
        return ""
    if len(text) >= 10 and text[4] == "-":
        candidate = text[:10]
        try:
            date.fromisoformat(candidate)
            return candidate
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
            missing = [name for name in NEEDED if name not in by_name]
            if missing:
                print("Header tidak ditemukan:", missing)
            elem.clear()
            continue

        if not any(value.strip() for value in values.values()):
            elem.clear()
            continue

        total += 1
        polres = values.get(header_map.get("polres", ""), "").strip()
        if polres not in SIX:
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
                "tingkat": values.get(header_map.get("tingkat", ""), "").strip(),
                "polda": values.get(header_map.get("polda", ""), "").strip(),
                "polres": polres,
                "polsek": values.get(header_map.get("polsek", ""), "").strip(),
                "kode": values.get(header_map.get("kode", ""), "").strip(),
                "kegiatan": values.get(header_map.get("kegiatan", ""), "").strip(),
                "quick_wins": values.get(header_map.get("quick_wins", ""), "").strip(),
                "pelapor": values.get(header_map.get("pelapor", ""), "").strip(),
                "report_id": values.get(header_map.get("report_id", ""), "").strip(),
                "kegiatan_ke": values.get(header_map.get("kegiatan_ke", ""), "").strip(),
                "bukti": values.get(header_map.get("bukti", ""), "").strip(),
                "isian": values.get(header_map.get("isian", ""), "").strip(),
            }
        )
        if total % 25_000 == 0:
            print(f"{total:,} baris, {len(rows):,} cocok, {time.time() - started:.0f} detik", flush=True)

        elem.clear()
        while elem.getprevious() is not None:
            del elem.getparent()[0]

with open(OUT, "w", encoding="utf-8") as file:
    json.dump(rows, file, ensure_ascii=False)

print(f"Selesai: {total:,} baris total, {len(rows):,} baris enam polres, {time.time() - started:.0f} detik")
for polres, count in Counter(row["polres"] for row in rows).most_common():
    print(f"  {count:5,} | {polres}")
