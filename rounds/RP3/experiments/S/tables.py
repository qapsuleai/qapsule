#!/usr/bin/env python3
"""RP3/S: prints the comparison tables (Markdown) from measure.sh's output folder.

  python3 -I rounds/RP3/experiments/S/tables.py <out dir> [--json <file>]

Every number is the median over the runs of one setting and path order of that run's
statistic (p50/p95/p99 of the run's samples). `tick` is the data frame's own core work
(RP3/M14: without painting); `tick+pass` (R4-comparable) = the phase's mean tick plus its
paint passes per data frame (`counts.tick_plus_pass_ms`), idle / chart_ticks.
"""
import json
import re
import statistics
import sys
from pathlib import Path

PHASES = ["idle", "scroll", "chart_ticks", "drag"]


def med(xs):
    xs = [x for x in xs if x is not None]
    return statistics.median(xs) if xs else None


def f(x, nd=1):
    if x is None:
        return "—"
    if abs(x - round(x)) < 1e-9 and nd <= 1:
        return str(int(round(x)))
    return f"{x:.{nd}f}".replace(".", ",")


def load(d):
    out = []
    for name in ("load-before.txt", "load-after.txt"):
        p = d / name
        if p.exists():
            m = re.search(r"load averages?: ([\d.]+)", p.read_text())
            if m:
                out.append(float(m.group(1)))
    return out


def runs_of(root):
    """{setting: {order: [(summary, loads)]}}"""
    res = {}
    for sdir in sorted(p for p in root.iterdir() if p.is_dir()):
        for rdir in sorted(p for p in sdir.iterdir() if p.is_dir()):
            order = rdir.name.split("-")[0]
            js = list(rdir.glob("compare-*.json"))
            if not js:
                continue
            s = json.loads(js[0].read_text())
            res.setdefault(sdir.name, {}).setdefault(order, []).append((s, load(rdir)))
    return res


def path_stats(runs, name):
    """Median statistics of path `name` over runs."""
    ps = []
    for s, _ in runs:
        for p in s["paths"]:
            if p["name"] == name:
                ps.append(p)
    if not ps:
        return None
    o = {"n_runs": len(ps), "paint": ps[0]["paint"]}
    for sp in ("tick", "paint_pass", "face", "scene", "frame_to_paint"):
        for q in ("p50", "p95", "p99", "n"):
            o[f"{sp}.{q}"] = med([p["span_ms"].get(sp, {}).get(q) for p in ps])
    for ph in PHASES:
        for q in ("p50", "p95", "p99", "long"):
            o[f"frame.{ph}.{q}"] = med([p["phases"].get(ph, {}).get("frame_ms", {}).get(q) for p in ps])
        o[f"tpp.{ph}"] = med([p["phases"].get(ph, {}).get("counts", {}).get("tick_plus_pass_ms") for p in ps])
        for sp in ("face", "scene", "tick", "paint_pass"):
            for q in ("p50", "p95", "n"):
                o[f"{ph}.{sp}.{q}"] = med([p["phases"].get(ph, {}).get("span_ms", {}).get(sp, {}).get(q) for p in ps])
    o["wasm_mb"] = med([(p["memory_bytes"] or {}).get("wasm", 0) / 1048576 for p in ps])
    jh = [(p["memory_bytes"] or {}).get("js_heap") for p in ps]
    o["js_mb"] = med([x / 1048576 for x in jh if x])
    o["fallbacks"] = sorted({json.dumps(p["paint"]) for p in ps})
    return o


def ppp(o, k, nd=1):
    return " / ".join(f(o.get(f"{k}.{q}"), nd) for q in ("p50", "p95", "p99"))


def main():
    root = Path(sys.argv[1])
    data = runs_of(root)
    dump = {}
    for setting, orders in data.items():
        for order, runs in sorted(orders.items()):
            loads = [x for _, l in runs for x in l]
            names = []
            for s, _ in runs:
                for p in s["paths"]:
                    if p["name"] not in names:
                        names.append(p["name"])
            ref_names = sorted(names, key=lambda n: ["ref", "canvas", "cpu", "gpu"].index(n) if n in ["ref", "canvas", "cpu", "gpu"] else 9)
            print(f"\n#### {setting}, {order} ({len(runs)} runs; path order {', '.join(names)}; load 1 min {f(min(loads) if loads else None)}–{f(max(loads) if loads else None)})\n")
            print("| path | tick p50/p95/p99 | paint pass p50/p95/p99 | face p50/p95/p99 (n) | scene p50/p95/p99 (n) | tick+pass idle / chart | WASM MB | JS heap MB |")
            print("|---|---|---|---|---|---|---:|---:|")
            for n in ref_names:
                o = path_stats(runs, n)
                dump.setdefault(setting, {}).setdefault(order, {})[n] = o
                fb = "" if o["paint"]["fallback"] is None else f" (→{o['paint']['used']}, {o['paint']['fallback']})"
                print(f"| {n}{fb} | {ppp(o, 'tick')} | {ppp(o, 'paint_pass')} | {ppp(o, 'face')} ({f(o['face.n'])}) | {ppp(o, 'scene')} ({f(o['scene.n'])}) | {f(o['tpp.idle'], 2)} / {f(o['tpp.chart_ticks'], 2)} | {f(o['wasm_mb'])} | {f(o['js_mb'])} |")
            print()
            print("| path | frame idle p50/p95/p99 (long) | scroll | chart_ticks | drag |")
            print("|---|---|---|---|---|")
            for n in ref_names:
                o = path_stats(runs, n)
                cells = [f"{ppp(o, 'frame.' + ph)} ({f(o['frame.' + ph + '.long'], 3)})" for ph in PHASES]
                print(f"| {n} | " + " | ".join(cells) + " |")
    if "--json" in sys.argv:
        Path(sys.argv[sys.argv.index("--json") + 1]).write_text(json.dumps(dump, indent=1))


if __name__ == "__main__":
    main()
