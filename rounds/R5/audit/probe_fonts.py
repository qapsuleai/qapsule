"""Denetçi: eşik altı metin ilkellerinin yazı tipi dizgisi ve tam metni (açılmış kapsül, grafik)."""
import json
import sys

d = json.load(open(sys.argv[1]))
seen = set()
for s in d["scenes"]:
    sc = s["scene"]
    if not sc:
        continue
    for p in sc["prims"]:
        if p["k"] == "text" and p["c"][:7] in ("#888a8e", "#6e7076"):
            k = (s["to"], s["theme"], s["lang"], s["id"], p["font"])
            if k in seen:
                continue
            seen.add(k)
            line = " | ".join(q["t"] for q in sc["prims"] if q["k"] == "text" and q["c"] == p["c"])
            print(k, "→", line[:160])
