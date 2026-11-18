"""R5/fix47 — contrast of every element from `probe.mjs`'s output (acceptance criterion 4).

    python3 -I rounds/R5/experiments/S/contrast/analyze.py <probe.json> <look.json>
    (look.json: `qp look show oltan`)

Colour arithmetic: the auditor's `rounds/R5/audit/colour.py` (WCAG 2.2 relative luminance,
straight-alpha compositing in sRGB), imported from there.

Rules (stricter than the harness's, A46):
- **Text** (core-painted `text` primitives and DOM text): 4.5:1 against what is under its
  anchor point (x, y − 0.35·size): the topmost `rect`/`fill`/`circle` primitives painted
  before it, composited over the scene's `bg`. 3:1 only at ≥ 24 CSS px: **600 is not counted
  as bold** (no text of the presentation is 700 at ≥ 18.66 px either).
- **Non-text** (`rect`, `line`, `fill`, `circle`, `path`): 3:1 against what is under it
  (WCAG 1.4.11), except elements that are decorative: roles `grid` and `line` (rules and
  dividers that separate, not inform), translucent plates and bands (alpha < 255: the
  confidence plate, evidence zones; the information is the opaque edge or mark on them),
  and surfaces (a primitive whose colour equals what is under it, or roles `bg`, `surface`,
  `card`, `page`). Decorative elements are listed with their contrast, not gated.
- DOM controls (`input`, `select`): the border (if any) or the fill against the parent,
  3:1; native checkboxes from their pixels (`--shots`, PIL), the most distinct pixel against
  the parent. Buttons are marked by their text (measured as text), not by a boundary.
- A circle followed by a concentric smaller circle is a ring: measured against its hole.
- A stroke or a shape is measured against what lies just outside its outline too (both
  sides of each end and the middle of a stroke; each vertex of a shape pushed out 1 px)
  and passes on its most distinct surroundings, as a rect does.
- A rect's adjacent colours are sampled 1 px outside each edge; it passes on its most
  distinct edge. Rects at most 2 px thin are not backgrounds (a crossing rule).
Output: Markdown tables (per surface and theme: elements, worst text, worst non-text, count
below threshold) and every element below its threshold.
"""
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "audit"))
from colour import contrast, hex2rgba, over  # noqa: E402

d = json.load(open(sys.argv[1]))
look = json.load(open(sys.argv[2]))
ROLE = defaultdict(list)


def walk(theme, prefix, obj):
    for k, v in obj.items():
        if isinstance(v, str) and v.startswith("#"):
            ROLE[(theme, v[:7].lower())].append(prefix + k)
        elif isinstance(v, dict):
            walk(theme, prefix + k + ".", v)


for theme, cols in look["colors"].items():
    walk(theme, "", cols)

DECOR_ROLES = {"grid", "line", "chrome.line"}
SURFACE_ROLES = {"bg", "surface", "chrome.card", "chrome.bg", "chrome.page"}


def roles(theme, c):
    return ROLE.get((theme, c[:7].lower()), ["?"])


def inside(p, x, y):
    if p["k"] == "rect":
        return p["x"] <= x <= p["x"] + p["w"] and p["y"] <= y <= p["y"] + p["h"]
    if p["k"] == "fill":
        pts = p["pts"]
        xs, ys = [q[0] for q in pts], [q[1] for q in pts]
        if not (min(xs) <= x <= max(xs) and min(ys) <= y <= max(ys)):
            return False
        c = False
        for i in range(len(pts)):
            (x1, y1), (x2, y2) = pts[i], pts[i - 1]
            if (y1 > y) != (y2 > y) and x < (x2 - x1) * (y - y1) / (y2 - y1 + 1e-12) + x1:
                c = not c
        return c
    if p["k"] == "circle" and p.get("lw") is None:
        return (p["x"] - x) ** 2 + (p["y"] - y) ** 2 <= p["r"] ** 2
    return False


def under(prims, i, x, y, bg):
    # A rect at most 2 px thin (a rule, a crosshair) is a line crossing, not a background.
    col = bg
    for p in prims[:i]:
        if p["k"] == "rect" and min(p["w"], p["h"]) <= 2:
            continue
        if p["k"] in ("rect", "fill", "circle") and inside(p, x, y):
            col = over(hex2rgba(p["c"]), col)
    return col


def anchor(p):
    if p["k"] in ("rect",):
        return p["x"] + p["w"] / 2, p["y"] + p["h"] / 2
    if p["k"] == "circle":
        return p["x"], p["y"]
    if "pts" in p and p["pts"]:
        return p["pts"][0]
    return p.get("x", 0), p.get("y", 0)


TEXT = []  # (ratio, need, surface, theme, px, colour, roles, text, where)
NON = []  # (ratio, need or None (decorative), surface, theme, kind, colour, roles, why, where)


def scan(sc, theme, surface, where):
    if not sc:
        return
    s = sc.get("scale", 1)
    bg = hex2rgba(sc["bg"])[:3]
    prims = sc["prims"]
    for i, p in enumerate(prims):
        rgba = hex2rgba(p["c"])
        ro = roles(theme, p["c"])
        if p["k"] == "text":
            m = re.match(r"(\d+(?:\.\d+)?)px", p.get("font", ""))
            px = float(m.group(1)) / s if m else 0
            u = under(prims, i, p["x"], p["y"] - 0.35 * px * s, bg)
            c = contrast(over(rgba, u), u)
            need = 3.0 if px >= 24 else 4.5
            TEXT.append((c, need, surface, theme, px, p["c"], ",".join(ro), p["t"][:40], where))
            continue
        x, y = anchor(p)
        u = under(prims, i, x, y, bg)
        comp = over(rgba, u)
        if comp == tuple(u) or any(r in SURFACE_ROLES for r in ro) and p["k"] == "rect" and p.get("w", 0) * p.get("h", 0) > 2000 * s * s:
            continue  # a surface: nothing to see against itself
        c = contrast(comp, u)
        if p["k"] == "rect":
            # Adjacent colours of a rect: just outside each edge (1 px); the rect is told
            # apart by its most distinct edge (e.g. a 2 px bar on a chip's edge whose outer
            # side is the ground).
            for ox, oy in ((p["x"] - 1, y), (p["x"] + p["w"] + 1, y), (x, p["y"] - 1), (x, p["y"] + p["h"] + 1)):
                uo = under(prims, i, ox, oy, bg)
                c = max(c, contrast(over(rgba, uo), uo))
        if p["k"] in ("line", "fill") and p.get("pts"):
            # A stroke or a shape is told apart by its most distinct surroundings too: the
            # ground 1 px outside its outline (line: beside each end and the middle, by half
            # its width; shape: each vertex pushed out from the centre).
            pts = p["pts"]
            outs = []
            if p["k"] == "line":
                h = p.get("lw", 1) / 2 + 1
                for (x1, y1), (x2, y2) in zip(pts, pts[1:]):
                    dx, dy = x2 - x1, y2 - y1
                    n = (dx * dx + dy * dy) ** 0.5 or 1.0
                    nx, ny = -dy / n * h, dx / n * h
                    for (mx, my) in ((x1, y1), ((x1 + x2) / 2, (y1 + y2) / 2), (x2, y2)):
                        outs += [(mx + nx, my + ny), (mx - nx, my - ny)]
            else:
                cx = sum(q[0] for q in pts) / len(pts)
                cy = sum(q[1] for q in pts) / len(pts)
                for qx, qy in pts:
                    dx, dy = qx - cx, qy - cy
                    n = (dx * dx + dy * dy) ** 0.5 or 1.0
                    outs.append((qx + dx / n, qy + dy / n))
            for ox, oy in outs:
                uo = under(prims, i, ox, oy, bg)
                c = max(c, contrast(over(rgba, uo), uo))
        if p["k"] == "circle":
            # A ring: a later concentric smaller circle is its hole; the ring is told apart
            # against the hole (a badge's ink ring around a ground disc).
            for q in prims[i + 1:i + 4]:
                if q["k"] == "circle" and abs(q["x"] - p["x"]) < 0.5 and abs(q["y"] - p["y"]) < 0.5 and q["r"] < p["r"]:
                    hole = over(hex2rgba(q["c"]), comp)
                    c = max(c, contrast(comp, hole))
        why = None
        if any(r in DECOR_ROLES for r in ro):
            why = "rule/divider (`" + "/".join(r for r in ro if r in DECOR_ROLES) + "`)"
        elif rgba[3] < 255:
            why = f"translucent plate/band (alpha {rgba[3]})"
        elif any(r in SURFACE_ROLES for r in ro):
            why = "surface / knock-out (the ground showing through a mark)"
        NON.append((c, None if why else 3.0, surface, theme, p["k"], p["c"], ",".join(ro), why, where))


for f in d["faces"]:
    for fid, sc in f["faces"].items():
        scan(sc, f["theme"], "face (akış)", f"{f['lang']} step {f['step']} {fid}")
SURF = {"capsule": "açılmış kapsül (`tesir`)", "evidence": "kanıt derinliği", "chart": "hikâye çizelgesi (`pavka`)"}
for sc in d["scenes"]:
    tag = "kapanmış/geçersiz" if sc["closed"] else "canlı"
    scan(sc["scene"], sc["theme"], SURF[sc["to"]], f"{sc['lang']} {sc['id']} ({tag})")


def rgb(s):
    v = [float(x) for x in re.findall(r"[\d.]+", s)]
    return v + [1.0] * (4 - len(v))


DOMT = []
DOMC = []
for blk in d["dom"]:
    for t in blk["texts"]:
        fg, bg = rgb(t["color"]), rgb(t["bg"])
        a = fg[3] * t["opacity"]
        u = tuple(int(round(x)) for x in bg[:3])
        c = contrast(over((int(fg[0]), int(fg[1]), int(fg[2]), int(round(a * 255))), u), u)
        need = 3.0 if t["px"] >= 24 else 4.5
        DOMT.append((c, need, "kabuk (DOM)", blk["theme"], t["px"], t["color"], t["id"] or t["cls"] or t["tag"], t["t"], blk["where"]))
    for k in blk.get("controls", []):
        if k["tag"] == "BUTTON":
            continue  # a button is marked by its text (measured above), not its boundary
        if k["type"] in ("checkbox", "radio"):
            continue  # drawn natively: measured from its pixels below
        u = tuple(int(round(x)) for x in rgb(k["bg"])[:3])
        best = 1.0
        what = "fill"
        for name, col in (("border", k["border"]), ("fill", k["fill"])):
            if not col:
                continue
            v = rgb(col)
            if v[3] == 0:
                continue
            cc = contrast(over((int(v[0]), int(v[1]), int(v[2]), int(round(v[3] * 255))), u), u)
            if cc > best:
                best, what = cc, name
        DOMC.append((best, 3.0, "kabuk (DOM)", blk["theme"], k["tag"].lower(), what, k["id"] or k["type"], None, blk["where"]))

# Native checkboxes (pixels): the most distinct pixel of the box against its parent.
for cb in d.get("checkboxes", []):
    try:
        from PIL import Image  # optional
    except ImportError:
        break
    im = Image.open(cb["file"]).convert("RGB")
    u = tuple(int(round(x)) for x in rgb(cb["bg"])[:3])
    best = max(contrast(px, u) for px in im.convert("RGB").getdata())  # noqa
    DOMC.append((best, 3.0, "kabuk (DOM)", cb["theme"], "checkbox (pixels)", "box", Path(cb["file"]).name, None, "settings"))

ALLT = TEXT + DOMT
surfaces = ["face (akış)", SURF["capsule"], SURF["evidence"], SURF["chart"], "kabuk (DOM)"]
print("| yüzey | tema | metin öğesi | en kötü metin (oran / eşik, boy, rol, örnek) | metin dışı öğe (eşikli) | en kötü metin dışı (oran, tür, rol) | eşik altı |")
print("|---|---|---|---|---|---|---|")
worst_all = []
for sf in surfaces:
    for th in ("dark", "light"):
        tt = [r for r in ALLT if r[2] == sf and r[3] == th]
        nn = [r for r in NON + DOMC if r[2] == sf and r[3] == th and r[1] is not None]
        if not tt and not nn:
            continue
        bad = sum(1 for r in tt if r[0] < r[1]) + sum(1 for r in nn if r[0] < r[1])
        wt = min(tt, key=lambda r: r[0] / r[1]) if tt else None
        wn = min(nn, key=lambda r: r[0] / r[1]) if nn else None
        if wt:
            worst_all.append(("metin", wt))
        if wn:
            worst_all.append(("metin dışı", wn))
        ts = f"**{wt[0]:.2f}** / {wt[1]}, {wt[4]:.0f} px, `{wt[6]}`, “{wt[7][:28]}”" if wt else "—"
        ns = f"**{wn[0]:.2f}**, {wn[4]}, `{wn[6]}`" if wn else "—"
        print(f"| {sf} | {th} | {len(tt)} | {ts} | {len(nn)} | {ns} | {bad} |")
print()
wk = min(worst_all, key=lambda r: r[1][0] / r[1][1])
print(f"En kötü eşikli öğe: {wk[0]} {wk[1][0]:.2f} (eşik {wk[1][1]}) — {wk[1][2]}, {wk[1][3]}, `{wk[1][6]}`, {wk[1][8]}")
print()
print("Eşik altı (metin):")
for r in sorted(r for r in ALLT if r[0] < r[1])[:30]:
    print(f"- {r[0]:.2f} < {r[1]} {r[2]} {r[3]} {r[4]:.0f} px {r[5]} `{r[6]}` “{r[7]}” [{r[8]}]")
print("Eşik altı (metin dışı):")
for r in sorted((r for r in NON + DOMC if r[1] is not None and r[0] < r[1]), key=lambda r: r[0])[:30]:
    print(f"- {r[0]:.2f} < 3 {r[2]} {r[3]} {r[4]} {r[5]} `{r[6]}` [{r[8]}]")
print()
print("Süs sayılan metin dışı öğeler (eşiksiz; rol başına en düşük):")
agg = {}
for r in NON:
    if r[1] is None:
        key = (r[2], r[3], r[6], r[7])
        if key not in agg or r[0] < agg[key][0]:
            agg[key] = (r[0], r[4], r[5])
print("| yüzey | tema | rol | neden | en düşük | tür |")
print("|---|---|---|---|---|---|")
for (sf, th, ro, why), (c, k, col) in sorted(agg.items()):
    print(f"| {sf} | {th} | `{ro}` | {why} | {c:.2f} | {k} {col} |")
print()
print("Eşikli metin dışı öğeler, rol başına en düşük:")
agg = {}
for r in NON:
    if r[1] is not None:
        key = (r[2], r[3], r[6])
        if key not in agg or r[0] < agg[key][0]:
            agg[key] = (r[0], r[4], r[5])
print("| yüzey | tema | rol | en düşük | tür |")
print("|---|---|---|---|---|")
for (sf, th, ro), (c, k, col) in sorted(agg.items()):
    print(f"| {sf} | {th} | `{ro}` | {c:.2f} | {k} {col} |")
print()
print(f"Sayılar: metin ilkeli {len(TEXT)}, DOM metni {len(DOMT)}, metin dışı {len(NON)} (süs {sum(1 for r in NON if r[1] is None)}), DOM denetimi {len(DOMC)}; sayfa hatası {len(d.get('errors', []))}")
