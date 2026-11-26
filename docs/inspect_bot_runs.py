import json

with open("data/tmp_bot_1min/runs.json", encoding="utf-8") as handle:
    runs = json.load(handle)

print(f"{len(runs)} rangkaian\n")
for number, run in enumerate(sorted(runs, key=lambda item: -item["count"]), start=1):
    print(f"[{number}] {run['reporter']} · {run['polres']}")
    print(
        f"    satuan: {', '.join(run['units'])} | polsek: {', '.join(run['polseks']) or '-'}"
    )
    print(
        f"    {run['count']} laporan · {run['start_wib']} → {run['end_wib']} "
        f"({run['duration_s']:.0f} dtk) · jeda min/median/maks "
        f"{run['gap_min_s']:.2f}/{run['gap_median_s']:.2f}/{run['gap_max_s']:.2f} dtk"
    )
    print(f"    kode: {', '.join(run['codes'])}")
    for report in run["reports"][:4]:
        print(
            f"      {report['seq']:>2}. +{report['gap_s']:>5.2f}s {report['wib']} "
            f"{report['kode']} · {report['kegiatan'][:70]}"
        )
    if run["count"] > 4:
        print(f"      … {run['count'] - 4} laporan lagi")
    print()
