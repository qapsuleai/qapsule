#!/usr/bin/env python3
"""R4/T3: for every changed sentence string between two JSON files, print the differing
tails (after the common prefix), deduplicated. Shows which clause changed."""
import json, sys
def strs(v, out, path=""):
    if isinstance(v, dict):
        for k, x in v.items(): strs(x, out, f"{path}/{k}")
    elif isinstance(v, list):
        for i, x in enumerate(v): strs(x, out, f"{path}/{i}")
    elif isinstance(v, str): out[path] = v
a, b = {}, {}
strs(json.load(open(sys.argv[1])), a); strs(json.load(open(sys.argv[2])), b)
seen = set()
for p in sorted(set(a) & set(b)):
    x, y = a[p], b[p]
    if x == y or p.rsplit("/",1)[-1] not in ("tr","en","text","headline","claim"): continue
    i = 0
    while i < min(len(x), len(y)) and x[i] == y[i]: i += 1
    j = x.rfind(". ", 0, i) + 2 if x.rfind(". ", 0, i) >= 0 else 0
    t = (x[j:], y[j:])
    if t not in seen:
        seen.add(t); print(f"- {x[j:]!r}\n+ {y[j:]!r}")
