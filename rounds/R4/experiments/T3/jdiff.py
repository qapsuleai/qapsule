#!/usr/bin/env python3
"""R4/T3: structural diff of two JSON files (old vs new), grouped by the changed key.

Usage: jdiff.py OLD NEW  -> per changed key: count and a couple of example paths.
Numbers in paths are collapsed to `*` for grouping. Deterministic.
"""
import json, re, sys
from collections import OrderedDict

def walk(a, b, path, out):
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            p = f"{path}/{k}"
            if k not in a:
                out.append(("added", p, None, b[k]))
            elif k not in b:
                out.append(("removed", p, a[k], None))
            else:
                walk(a[k], b[k], p, out)
    elif isinstance(a, list) and isinstance(b, list):
        for i in range(max(len(a), len(b))):
            p = f"{path}/{i}"
            if i >= len(a):
                out.append(("added", p, None, b[i]))
            elif i >= len(b):
                out.append(("removed", p, a[i], None))
            else:
                walk(a[i], b[i], p, out)
    elif isinstance(a, str) and isinstance(b, str) and a.startswith(("{", "[")):
        try:
            walk(json.loads(a), json.loads(b), path + "#", out)
        except Exception:
            if a != b:
                out.append(("changed", path, a, b))
    elif a != b:
        out.append(("changed", path, a, b))

def main():
    old, new = (json.load(open(f)) for f in sys.argv[1:3])
    out = []
    walk(old, new, "", out)
    groups = OrderedDict()
    for kind, p, a, b in out:
        key = re.sub(r"/\d+", "/*", p)
        last = key.rsplit("/", 1)[-1]
        g = groups.setdefault((kind, last), [])
        g.append((p, a, b))
    for (kind, last), v in sorted(groups.items(), key=lambda x: (-len(x[1]), x[0])):
        p, a, b = v[0]
        sa = json.dumps(a, ensure_ascii=False)[:90]
        sb = json.dumps(b, ensure_ascii=False)[:90]
        print(f"{kind:8} {last:22} {len(v):5}  e.g. {p}: {sa} -> {sb}")
    print(f"total {len(out)} leaf differences")

main()
