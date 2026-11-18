"""Denetçi: probe.mjs çıktısının çözümlemesi (karşıtlık kendi colour.py'mle).

Metin ilkeli: rengi (alfasıyla) altındaki zemine bindirilir; zemin = metnin çapa noktasını
(x, y − 0,35·boy) içeren en üstteki opak-sayılır (alfa ≥ 0,5) `rect`/`fill`, yoksa sahnenin `bg`'si.
Eşik: 4,5 (≥ 24 px ya da ≥ 18,66 px kalın: 3). Ölçek: boyama listesi aygıt pikseli, `scale` ile CSS px.
Metin dışı: `circle`, `fill`, `line` (lw ≥ 2 aygıt px) ilkelleri; altındaki zemine karşı ≥ 3
(WCAG 1.4.11; ızgara/çizgi/eksen rolleri ayrıca listelenir, "anlam taşımayan süs" sayılıp
sayılmayacağı yorum).
"""
import json
import re
import sys
from collections import Counter, defaultdict

from colour import contrast, hex2rgba, over

d = json.load(open(sys.argv[1]))
look = json.load(open(sys.argv[2]))  # qp look show oltan
ROLE = {}
for theme, cols in look["colors"].items():
    for k, v in cols.items():
        if isinstance(v, str):
            ROLE.setdefault((theme, v[:7].lower()), []).append(k)


def inside(p, x, y):
    if p["k"] == "rect":
        return p["x"] <= x <= p["x"] + p["w"] and p["y"] <= y <= p["y"] + p["h"]
    if p["k"] == "fill":
        pts = p["pts"]
        xs, ys = [q[0] for q in pts], [q[1] for q in pts]
        if not (min(xs) <= x <= max(xs) and min(ys) <= y <= max(ys)):
            return False
        c = False  # even-odd ray test (good enough for plates)
        for i in range(len(pts)):
            (x1, y1), (x2, y2) = pts[i], pts[i - 1]
            if (y1 > y) != (y2 > y) and x < (x2 - x1) * (y - y1) / (y2 - y1 + 1e-12) + x1:
                c = not c
        return c
    if p["k"] == "circle":
        return (p["x"] - x) ** 2 + (p["y"] - y) ** 2 <= p["r"] ** 2
    return False


def under(prims, i, x, y, bg):
    col = bg
    for p in prims[:i]:
        if p["k"] in ("rect", "fill") and inside(p, x, y):
            rgba = hex2rgba(p["c"])
            if rgba[3] >= 128:
                col = over(rgba, col)
            else:
                col = over(rgba, col)
    return col


def scan_scene(sc, theme, where, acc_t, acc_n):
    if not sc:
        return
    s = sc.get("scale", 1)
    bg = hex2rgba(sc["bg"])[:3]
    prims = sc["prims"]
    for i, p in enumerate(prims):
        if p["k"] == "text":
            m = re.match(r"(\d+(?:\.\d+)?)px", p.get("font", ""))
            px = float(m.group(1)) / s if m else 0
            bold = bool(re.search(r"\b(600|700|bold|Semi|Bold)", p.get("font", "")))
            u = under(prims, i, p["x"], p["y"] - 0.35 * px * s, bg)
            c = contrast(over(hex2rgba(p["c"]), u), u)
            need = 3.0 if (px >= 24 or (bold and px >= 18.66)) else 4.5
            acc_t.append((c, need, px, p["t"][:40], p["c"], where, theme))
        elif p["k"] in ("circle", "fill") or (p["k"] == "line" and p.get("lw", 0) >= 2):
            rgba = hex2rgba(p["c"])
            if p["k"] == "line":
                x, y = p["pts"][0]
            elif p["k"] == "circle":
                x, y = p["x"], p["y"]
            else:
                x, y = p["pts"][0]
            u = under(prims, i, x, y, bg) if p["k"] != "fill" else bg
            c = contrast(over(rgba, u), u)
            roles = ROLE.get((theme, p["c"][:7].lower()), ["?"])
            acc_n.append((c, p["k"], ",".join(roles), p["c"], where, theme))


acc_t, acc_n = [], []
for f in d["faces"]:
    for fid, sc in f["faces"].items():
        scan_scene(sc, f["theme"], f"face {f['lang']} step{f['step']} {fid}", acc_t, acc_n)
face_t, face_n = list(acc_t), list(acc_n)
for s in d["scenes"]:
    scan_scene(s["scene"], s["theme"], f"{s['to']} {s['lang']} {s['id']}", acc_t, acc_n)

print(f"text prims: faces {len(face_t)}, all {len(acc_t)}")
for label, rows in (("faces", face_t), ("faces+open+evidence+chart", acc_t)):
    bad = [r for r in rows if r[0] < r[1]]
    worst = min(rows, key=lambda r: r[0] / r[1])
    print(f"  {label}: below threshold {len(bad)}; worst {worst[0]:.2f} (need {worst[1]}) "
          f"{worst[2]:.0f}px '{worst[3]}' {worst[4]} [{worst[5]} {worst[6]}]; min px {min(r[2] for r in rows):.1f}")
    for r in sorted(bad)[:8]:
        print(f"     BELOW {r[0]:.2f}<{r[1]} {r[2]:.0f}px '{r[3]}' {r[4]} [{r[5]} {r[6]}]")
print("non-text prims by colour role (min contrast, count):")
agg = defaultdict(lambda: [99, 0, ""])
for c, k, roles, col, where, theme in acc_n:
    key = (theme, roles, k)
    a = agg[key]
    a[1] += 1
    if c < a[0]:
        a[0], a[2] = c, where
for (theme, roles, k), (mn, n, where) in sorted(agg.items()):
    flag = " <3" if mn < 3 else ""
    print(f"  {theme:5s} {k:6s} {roles:28s} min {mn:5.2f} n={n:4d}{flag}   e.g. {where}")
fn = [r for r in face_n if r[2] != "?"]
print("faces only, meaning-role non-text min:",
      {(t, ro): round(min(c for c, k, r2, col, w, t2 in face_n if r2 == ro and t2 == t), 2)
       for t, ro in {(r[5], r[2]) for r in face_n}})

# DOM text
dom_bad = []
ndom = 0
for blk in d["dom"]:
    for t in blk["texts"]:
        if not t["onscreen"]:
            continue
        ndom += 1
        fg = [float(x) for x in re.findall(r"[\d.]+", t["color"])]
        bg = [float(x) for x in re.findall(r"[\d.]+", t["bg"])]
        a = (fg[3] if len(fg) > 3 else 1) * t["opacity"]
        u = tuple(int(round(x)) for x in bg[:3])
        c = contrast(over((int(fg[0]), int(fg[1]), int(fg[2]), int(round(a * 255))), u), u)
        need = 3.0 if (t["px"] >= 24 or (int(t["weight"]) >= 700 and t["px"] >= 18.66)) else 4.5
        if c < need:
            dom_bad.append((round(c, 2), need, t["px"], t["t"], t["id"] or t["cls"], blk["theme"], blk["where"]))
print(f"DOM text nodes on screen: {ndom}; below threshold: {len(dom_bad)}")
for r in sorted(set(dom_bad))[:12]:
    print("   ", r)

print("paint=cpu:", d["paint_cpu"])
print("foreign requests:", len(d["foreign"]), d["foreign"][:5])
print("pages:", [(p["q"], len(p["reqs"]), len(p["errors"])) for p in d["pages"]])
print("gallery:", d.get("gallery"))


def sig(faces):
    o = {}
    for fid, sc in faces.items():
        o[fid] = (sorted(p["t"] for p in sc["prims"] if p["k"] == "text"),
                  sum(1 for p in sc["prims"] if p["k"] in ("circle", "fill")))
    return o


a, b = d["reduce"]
sa, sb = sig(a["faces"]), sig(b["faces"])
print("reduced motion: same face ids", sa.keys() == sb.keys(), "; texts equal",
      all(sa[k][0] == sb.get(k, ([], 0))[0] for k in sa),
      "; extra texts under reduce:", {k: sorted(set(sb[k][0]) - set(sa[k][0])) for k in sb if k in sa and set(sb[k][0]) - set(sa[k][0])},
      "; missing under reduce:", {k: sorted(set(sa[k][0]) - set(sb[k][0])) for k in sa if k in sb and set(sa[k][0]) - set(sb[k][0])})
print("reduced motion DOM texts missing under reduce:", sorted(set(a["dom"]) - set(b["dom"]))[:10])

dig = 0
nvis = 0
for v in d["visitor"]:
    for fid, sc in v["faces"].items():
        sym = fid.split(".")[1] if "." in fid else ""
        for p in sc["prims"]:
            if p["k"] == "text":
                nvis += 1
                t = re.sub(r"SYN\d+", "", p["t"])
                if re.search(r"\d", t):
                    dig += 1
                    print("   visitor digit:", p["t"])
    for t in v["dom"]:
        tt = re.sub(r"SYN\d+", "", t["t"])
        if re.search(r"\d", tt) and t["onscreen"]:
            print("   visitor DOM digit:", t["t"], t["id"] or t["cls"])
print(f"visitor face texts {nvis}, with digit {dig}")
