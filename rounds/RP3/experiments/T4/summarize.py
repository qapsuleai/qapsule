#!/usr/bin/env python3
"""T4: medians over the repetitions of raw/<tag>-<variant>.jsonl (mem.mjs lines), by
checkpoint (step rounded down to the sample grid), as a Markdown table in MB.

    python3 summarize.py raw/before-feed.jsonl [raw/...]
"""
import json
import statistics
import sys

EVERY = 250


def load(path):
    reps, cur, loads = [], None, []
    for line in open(path):
        line = line.strip()
        if not line.startswith('{'):
            continue
        r = json.loads(line)
        if 'load' in r:
            loads.append(r['load'])
            cur = {}
            reps.append(cur)
            continue
        if 'step' not in r or cur is None:
            continue
        k = (r['step'] // EVERY) * EVERY
        cur[k] = r
    return reps, loads


def med(xs):
    xs = [x for x in xs if isinstance(x, (int, float))]
    return statistics.median(xs) if xs else None


def mb(x):
    return '—' if x is None else f'{x / 1e6:.1f}'


for path in sys.argv[1:]:
    reps, loads = load(path)
    keys = sorted(set().union(*[r.keys() for r in reps])) if reps else []
    print(f'\n### {path} ({len(reps)} koşu; yük: {"; ".join(loads)})\n')
    print('| adım | WASM (MB) | JS yığını (MB) | DOM düğümü | tuval |')
    print('|---:|---:|---:|---:|---:|')
    for k in keys:
        rows = [r[k] for r in reps if k in r]
        w = med([r.get('wasm') for r in rows])
        j = med([r.get('js_heap') for r in rows])
        n = med([r.get('nodes', r.get('elements')) for r in rows])
        c = med([r.get('canvases') for r in rows])
        print(f'| {k} | {mb(w)} | {mb(j)} | {n if n is not None else "—"} | {c} |')
