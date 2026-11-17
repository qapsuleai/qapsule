"""RP3/T1: Markdown tables from results/*.json (calibrate.mjs output).
Usage: python3 -I rounds/RP3/experiments/T1/tables.py rounds/RP3/experiments/T1/results
"""
import json
import os
import statistics
import sys

d = sys.argv[1]
ref = {"tick": (74, 118), "face": (32, 72), "scene": (71, 92)}


def f(x, n=1):
    return "-" if x is None else f"{x:.{n}f}".replace(".", ",")


rows = []
for name in sorted(os.listdir(d)):
    if not name.endswith(".json"):
        continue
    try:
        r = json.load(open(os.path.join(d, name)))
    except ValueError:
        continue  # empty (failed or still running)
    s = r["summary"]
    c = s["cal"]
    rows.append((name[:-5], r, s, c))

print("| config | yük (önce → sonra) | tik p50 / p95 | yüz p50 / p95 | sahne p50 / p95 | sahne/tik | akış tik p50 / p95 | oran tik·yüz·sahne p50 |")
print("|---|---|---:|---:|---:|---:|---:|---:|")
for name, r, s, c in rows:
    ratio = " · ".join(f((c[k]["p50"] or 0) / ref[k][0], 2) for k in ("tick", "face", "scene"))
    print(
        f"| {name} | {r['load_before'].split()[0]} → {r['load_after'].split()[0]} | "
        f"{f(c['tick']['p50'])} / {f(c['tick']['p95'])} | {f(c['face']['p50'])} / {f(c['face']['p95'])} | "
        f"{f(c['scene']['p50'])} / {f(c['scene']['p95'])} | {f(s.get('scenes_per_tick'), 2)} | "
        f"{f(s['feed']['tick']['p50'])} / {f(s['feed']['tick']['p95'])} | {ratio} |"
    )

# Probe runs: the per-tick split and the bench (medians over runs).
print()
for name, r, s, c in rows:
    if "runs" not in r or not r["runs"] or not r["runs"][0].get("bench"):
        continue
    runs = r["runs"]
    print(f"### {name}")

    def med(path):
        v = []
        for x in runs:
            o = x
            try:
                for p in path:
                    o = o[p]
                v.append(o)
            except (KeyError, TypeError):
                pass
        return statistics.median(v) if v else None

    for ph in ("feed", "chart"):
        keys = sorted(k for k in runs[0][ph]["span_ms"] if k.startswith("p_") or k in ("tick", "face", "scene"))
        print(f"{ph}: " + "; ".join(f"{k} p50 {f(med([ph, 'span_ms', k, 'p50']))} ort {f(med([ph, 'span_ms', k, 'mean']), 2)} n {f(med([ph, 'span_ms', k, 'n']), 0)}" for k in keys))
    print("| ölçek | tür | piksel | kurma | tarama | dışarı kopya | ImageData | put | boyut+put | put+okuma | tam yol |")
    print("|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|")
    for sc in ("1", "2", "3"):
        for kind in ("scene", "face"):
            px = runs[0]["bench"].get(sc, {}).get(kind, {}).get("px")
            if not px:
                continue
            vals = [f(med(["bench", sc, kind, k]), 2) for k in ("build", "raster", "take", "img", "put", "resize_put", "put_sync", "full")]
            print(f"| {sc} | {kind} | {px[0]}×{px[1]} | " + " | ".join(vals) + " |")
    print()
