#!/usr/bin/env python3
"""RP3/S: the paint pass broken down (scene build, change check, paint) per path and phase.

  python3 -I rounds/RP3/experiments/S/breakdown.py <setting dir> [phases…]

<setting dir> is one setting of measure.sh's output (e.g. <out>/phone). For each path and
phase (default idle, chart_ticks): medians over the runs of the spans' p50 / p95 / n and
their sum per data frame (`n × mean / data frames`): `scene_build` (the open scene's
build), `change_check` (the hand-out "did it change" key), `scene` / `face` (painting),
`paint_pass`, and `tick_plus_pass_ms`.
"""
import json
import statistics
import sys
from pathlib import Path

SPANS = ["paint_pass", "scene_build", "change_check", "scene", "face"]


def med(xs):
    xs = [x for x in xs if x is not None]
    return statistics.median(xs) if xs else None


def f(x):
    return "—" if x is None else (f"{x:.2f}".rstrip("0").rstrip(".") or "0").replace(".", ",")


def main():
    root = Path(sys.argv[1])
    phases = sys.argv[2:] or ["idle", "chart_ticks"]
    runs = [json.loads(next(d.glob("compare-*.json")).read_text()) for d in sorted(root.iterdir()) if d.is_dir() and list(d.glob("compare-*.json"))]
    names = []
    for s in runs:
        for p in s["paths"]:
            if p["name"] not in names:
                names.append(p["name"])
    print(f"{root} ({len(runs)} runs)\n")
    print("| path | phase | " + " | ".join(f"{s} p50/p95 (n) · ms/data frame" for s in SPANS) + " | tick+pass |")
    print("|---|---|" + "---|" * (len(SPANS) + 1))
    for n in names:
        for ph in phases:
            cells = []
            for sp in SPANS:
                p50, p95, cnt, per = [], [], [], []
                for s in runs:
                    for p in s["paths"]:
                        if p["name"] != n:
                            continue
                        x = p["phases"][ph]
                        v = x["span_ms"].get(sp)
                        df = x["counts"].get("data_frames") or 1
                        if v:
                            p50.append(v["p50"]); p95.append(v["p95"]); cnt.append(v["n"]); per.append(v["n"] * v["mean"] / df)
                        else:
                            cnt.append(0); per.append(0)
                cells.append(f"{f(med(p50))}/{f(med(p95))} ({f(med(cnt))}) · {f(med(per))}")
            tpp = med([p["phases"][ph]["counts"].get("tick_plus_pass_ms") for s in runs for p in s["paths"] if p["name"] == n])
            print(f"| {n} | {ph} | " + " | ".join(cells) + f" | {f(tpp)} |")


if __name__ == "__main__":
    main()
