"""Denetçi: probe çıktısında eşik altı ilkellerin ayrıntısı (renk+alfa, boy, tür, tema)."""
import json
import re
import sys
from collections import defaultdict

from colour import contrast, hex2rgba, over

d = json.load(open(sys.argv[1]))
kind = sys.argv[2] if len(sys.argv) > 2 else "faces"
groups = defaultdict(lambda: [99, 0, None])
texts = defaultdict(lambda: [99, 0, None])


def scenes():
    if kind == "faces":
        for f in d["faces"]:
            for fid, sc in f["faces"].items():
                yield f["theme"], f"face {fid}", sc
    else:
        for s in d["scenes"]:
            if s["scene"] and s["to"] == kind:
                yield s["theme"], f"{s['to']} {s['id']}", s["scene"]


for theme, where, sc in scenes():
    bg = hex2rgba(sc["bg"])[:3]
    s = sc.get("scale", 1)
    for p in sc["prims"]:
        rgba = hex2rgba(p["c"])
        if p["c"][:7] == sc["bg"][:7]:
            continue
        c = contrast(over(rgba, bg), bg)
        if p["k"] == "text":
            m = re.match(r"(\d+(?:\.\d+)?)px", p.get("font", ""))
            key = (theme, p["c"], round(float(m.group(1)) / s) if m else 0)
            g = texts[key]
        else:
            size = p.get("lw") or p.get("r") or (p.get("w"), p.get("h")) if p["k"] == "rect" else (p.get("lw") or p.get("r"))
            key = (theme, p["k"], p["c"])
            g = groups[key]
        g[1] += 1
        if c < g[0]:
            g[0], g[2] = c, (where, p.get("t", "")[:30], p.get("lw"), p.get("r"), len(p.get("pts", [])))
print(f"== {kind}: text colours (vs scene bg)")
for (theme, col, px), (c, n, ex) in sorted(texts.items()):
    print(f"   {theme:5s} {col} {px:3d}px  {c:5.2f} n={n:4d} {'<4.5' if c < 4.5 else ''}  e.g. {ex[0]} '{ex[1]}'")
print(f"== {kind}: non-text colours (vs scene bg), alpha shown in hex")
for (theme, k, col), (c, n, ex) in sorted(groups.items()):
    print(f"   {theme:5s} {k:6s} {col} {c:5.2f} n={n:4d} {'<3' if c < 3 else ''}  e.g. {ex[0]} lw={ex[2]} r={ex[3]} pts={ex[4]}")
