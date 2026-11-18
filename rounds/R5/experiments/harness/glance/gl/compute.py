"""Computed measures and hard gates (SPEC §G7), from the shot records of `shoot.mjs`:
the engine's face scenes (`SceneDesc`: layers with names, hit boxes, colours, text sizes)
and the page's DOM text runs. Element level, never pixels unless stated. Pure functions;
unit-tested in tests/test_compute.py.
"""

from __future__ import annotations

import re

from itertools import combinations

from . import color as C

TOUCH_MIN = 44.0        # CSS px = pt at DPR-independent layout (Apple HIG 44 pt; WCAG 2.5.5 AAA 44)
MIN_FONT = 11.0         # CSS px (R4 SPEC §2 threshold, kept)
NONTEXT_MIN = 3.0       # WCAG 1.4.11
DE_MIN = 10.0           # RESEARCH F6 (Szafir 2017): each meaning-colour pair, every condition
DE_MIN_SMALL = 15.0     # ... for marks <= 12 pt (16 CSS px)
SMALL_MARK = 16.0
GRAY_PAIR_INFO = 1.5
COLOUR_CODED = 5.0      # ΔE00 (normal vision) from which a pair counts as colour-coded    # RESEARCH F7, reported (the grey reader question is the behavioural check)


def _rgba(c):
    return tuple(c) if len(c) == 4 else (*c, 255)


def _parse_css(c: str):
    v = [float(x) for x in re.findall(r"[\d.]+", c)]
    if len(v) == 3:
        v.append(1.0)
    return (v[0], v[1], v[2], v[3] * 255)


def _inside_poly(x, y, pts):
    n, j, ok = len(pts), len(pts) - 1, False
    for i in range(n):
        xi, yi = pts[i]
        xj, yj = pts[j]
        if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / ((yj - yi) or 1e-12) + xi:
            ok = not ok
        j = i
    return ok


def scene_prims(scene):
    """All prims in paint order with their layer id (`under`, panel layers, `over`)."""
    out = [(None, p) for p in scene.get("under", [])]
    for panel in scene.get("panels", []):
        for layer in panel.get("layers", []):
            out += [(layer.get("id"), p) for p in layer.get("prims", [])]
    out += [(None, p) for p in scene.get("over", [])]
    return out


def background_at(scene, prims, idx, x, y):
    """Opaque colour under point (x, y) before prim `idx`: the scene background with every
    earlier `fill`/`rect` containing the point composited over it."""
    bg = tuple(float(v) for v in scene["background"][:3])
    for _, p in prims[:idx]:
        k = p.get("prim")
        hit = False
        if k == "fill" and len(p.get("points", [])) >= 3:
            hit = _inside_poly(x, y, p["points"])
        elif k == "rect":
            hit = p["x"] <= x < p["x"] + p["w"] and p["y"] <= y < p["y"] + p["h"]
        if hit:
            bg = C.composite(_rgba(p["color"]), bg)
    return bg


def _area(p) -> float:
    k = p.get("prim")
    if k == "fill":
        pts = p.get("points", [])
        return abs(sum(pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1]
                       for i in range(len(pts)))) / 2
    if k == "polyline":
        pts = p.get("points", [])
        ln = sum(((pts[i + 1][0] - pts[i][0]) ** 2 + (pts[i + 1][1] - pts[i][1]) ** 2) ** 0.5 for i in range(len(pts) - 1))
        return ln * float(p.get("width", 1.0))
    if k == "circle":
        return 3.141592653589793 * float(p.get("r", 0)) ** 2
    if k == "rect":
        return float(p.get("w", 0)) * float(p.get("h", 0))
    return 0.0


def face_elements(scene):
    """Element-level colours of one face scene: text runs (colour over what is under them)
    and the glyph's body (its most opaque prim) over the background."""
    prims = scene_prims(scene)
    texts, glyph, gl_prims = [], None, []
    for i, (lid, p) in enumerate(prims):
        if p.get("prim") == "text":
            px = float(p.get("px", 0))
            x = p["x"] + (-0.3 * px if p.get("align") == "right" else 0.3 * px)
            y = p["baseline"] - 0.35 * px
            bg = background_at(scene, prims, i, x, y)
            fg = C.composite(_rgba(p["color"]), bg)
            texts.append({"text": p["text"], "px": px, "layer": lid, "fg": fg, "bg": bg,
                          "ratio": C.contrast(fg, bg), "threshold": C.text_threshold(px, False)})
        elif lid == "glyph" and p.get("prim") in ("fill", "polyline", "rect", "circle"):
            gl_prims.append(p)
    # The glyph's colour (SPEC §G7.2, §G7.4): the dominant visible colour of the glyph layer
    # (largest alpha-weighted area; prims that vanish into the background, e.g. a knockout
    # in the background colour, are skipped). Its contrast against the scene background.
    bgc = tuple(float(v) for v in scene["background"][:3])
    weight = {}
    for p in gl_prims:
        col = _rgba(p["color"])
        fg = C.composite(col, bgc)
        if C.de(fg, bgc) < 2.0:
            continue
        w = _area(p) * col[3] / 255.0
        weight[col] = weight.get(col, 0.0) + w
    if weight:
        col = max(sorted(weight), key=lambda c: weight[c])
        fg = C.composite(col, bgc)
        glyph = {"color": col, "bg": bgc, "fg": fg, "ratio": C.contrast(fg, bgc)}
    hits = []
    for panel in scene.get("panels", []):
        for layer in panel.get("layers", []):
            if layer.get("id") == "glyph":
                for it in layer.get("items", []):
                    h = it["hit"]
                    hits.append((h["right"] - h["left"], h["bottom"] - h["top"]))
    if glyph and hits:
        glyph["size"] = max(max(w, h) for w, h in hits)
        glyph["hit"] = hits
    return texts, glyph


def meaning_class(item) -> tuple[str, str]:
    """(axis, value) classes whose colours must differ (SPEC §G7.4)."""
    v = item["validity"]
    if v == "closed":
        return ("closed", {"realized": "realized", "invalidated": "failed"}.get(item["stage"], "expired"))
    return ("open", f"{v}:{ {'up': 'up', 'down': 'down'}.get(item['direction'], 'flat')}")


def semantic_colours(records):
    """Glyph body colour (composited) per meaning class from the shots' target faces."""
    out = {}
    for r in records:
        it = (r.get("engine") or {}).get("target_item")
        sc = ((r.get("engine") or {}).get("face_scenes") or {}).get((r.get("target") or ""))
        if not it or not sc:
            continue
        _, g = face_elements(sc["scene"])
        if not g:
            continue
        axis, val = meaning_class(it)
        out.setdefault(f"{axis}/{val}", {"fg": g["fg"], "bg": g["bg"], "size": g.get("size", 0)})
    return out


def colour_table(sem):
    """ΔE00 of every meaning-colour pair under normal, grey and three CVD conditions."""
    rows = []
    for (a, ca), (b, cb) in combinations(sorted(sem.items()), 2):
        size = min(ca["size"] or 99, cb["size"] or 99)
        thr = DE_MIN_SMALL if size <= SMALL_MARK else DE_MIN
        r = {"a": a, "b": b, "threshold": thr, "mark_px": size}
        for cond in C.CONDITIONS:
            r[cond] = round(C.pair_de(ca["fg"], cb["fg"], cond), 2)
        r["gray_lum_ratio"] = round(C.contrast(C.gray(ca["fg"]), C.gray(cb["fg"])), 3)
        r["min"] = min(r[c] for c in C.CONDITIONS)
        # A pair drawn in (nearly) the same colour is not colour-coded: its meaning must be
        # carried by shape (checked by the grey reader question), not gated here.
        r["colour_coded"] = r["normal"] >= COLOUR_CODED
        r["pass"] = (not r["colour_coded"]) or r["min"] >= thr
        rows.append(r)
    return rows


def dom_texts(rec):
    out = []
    for t in rec["dom"]["texts"]:
        fg = _parse_css(t["color"])
        bg = _parse_css(t["bg"])[:3]
        fg = (fg[0], fg[1], fg[2], fg[3] * t.get("opacity", 1.0))
        c = C.composite(fg, bg)
        bold = str(t.get("weight", "400")) in ("bold", "600", "700", "800", "900")
        out.append({"text": t["t"], "px": t["size"], "fg": c, "bg": bg, "ratio": C.contrast(c, bg),
                    "threshold": C.text_threshold(t["size"], bold), "where": t.get("id") or t.get("tag")})
    return out


def role_of(px: float, look) -> str | None:
    """The look's type role of a text size (T3 `TypeScale::mark_role` rule: the first of
    headline, body, label, note with that size; then title, figure)."""
    if not look:
        return None
    types = look.get("type") or look.get("types") or {}
    for r in ("headline", "body", "label", "note", "title", "figure"):
        v = types.get(r)
        if v and abs(float(v.get("px", -1)) - px) < 1e-3:
            return r
    return None


def per_shot(rec, look=None):
    """Element measures of one shot record."""
    texts, glyphs, taps, glyph_hits = [], [], [], []
    face_chars, face_lines = [], []
    for fid, f in ((rec.get("engine") or {}).get("face_scenes") or {}).items():
        t, g = face_elements(f["scene"])
        texts += [dict(x, face=fid, role=role_of(x["px"], look)) for x in t]
        face_chars.append(sum(len(x["text"]) for x in t))
        face_lines.append(len(t))
        if g:
            glyphs.append(dict(g, face=fid))
            glyph_hits += [{"what": "face-glyph-hit", "w": w, "h": h} for w, h in g.get("hit", [])]
    texts += dom_texts(rec)
    for t in rec["dom"]["taps"]:
        taps.append({"what": f"{t['tag']}#{t['id']}", "w": t["rect"][2], "h": t["rect"][3]})
    screen_chars = sum(len(t["text"]) for t in texts)
    return {"texts": texts, "glyphs": glyphs, "taps": taps, "glyph_hits": glyph_hits,
            "face_chars": face_chars, "face_lines": face_lines,
            "screen_chars": screen_chars}


def gates(records, look=None):
    """Hard gates of one design over all its shots (SPEC §G7, §G9 step 1); `look` = the
    design's look record (`qp look show`), for text roles."""
    worst_text = None
    worst_glyph = None
    min_font = None
    small_taps = []
    fc, fl, sc = [], [], []
    by_role = {}
    for r in records:
        m = per_shot(r, look)
        for t in m["texts"]:
            if t.get("role"):
                b = by_role.setdefault(t["role"], {"px": t["px"], "min_ratio": t["ratio"]})
                b["min_ratio"] = min(b["min_ratio"], t["ratio"])
        for t in m["texts"]:
            margin = t["ratio"] / t["threshold"]
            if worst_text is None or margin < worst_text["margin"]:
                worst_text = {"margin": margin, "ratio": round(t["ratio"], 2), "threshold": t["threshold"],
                              "text": t["text"][:40], "px": t["px"], "shot": r["shot"]["id"]}
            if min_font is None or t["px"] < min_font["px"]:
                min_font = {"px": t["px"], "text": t["text"][:40], "shot": r["shot"]["id"]}
        for g in m["glyphs"]:
            if worst_glyph is None or g["ratio"] < worst_glyph["ratio"]:
                worst_glyph = {"ratio": round(g["ratio"], 2), "shot": r["shot"]["id"], "face": g["face"]}
        for t in m["taps"]:
            # Only targets that act: the face (opens), the glyph hit box, buttons and fields.
            if min(t["w"], t["h"]) < TOUCH_MIN:
                small_taps.append({"what": t["what"], "w": round(t["w"], 1), "h": round(t["h"], 1), "shot": r["shot"]["id"]})
        fc += m["face_chars"]
        fl += m["face_lines"]
        sc.append(m["screen_chars"])
    sem = semantic_colours(records)
    table = colour_table(sem)
    out = {
        "text_contrast": {"worst": worst_text, "pass": bool(worst_text) and worst_text["margin"] >= 1.0},
        "nontext_contrast": {"worst_glyph": worst_glyph, "threshold": NONTEXT_MIN,
                             "pass": bool(worst_glyph) and worst_glyph["ratio"] >= NONTEXT_MIN},
        "min_font": {"worst": min_font, "threshold": MIN_FONT, "pass": bool(min_font) and min_font["px"] >= MIN_FONT},
        "touch": {"below_44": small_taps[:30], "count": len(small_taps), "pass": not small_taps},
        "colour": {"classes": sorted(sem), "pairs": table, "pass": bool(table) and all(r["pass"] for r in table)},
        "roles": {k: {"px": v["px"], "min_ratio": round(v["min_ratio"], 2)} for k, v in sorted(by_role.items())},
        "density": {"face_chars_mean": round(sum(fc) / len(fc), 1) if fc else None, "face_chars_max": max(fc) if fc else None,
                    "face_lines_max": max(fl) if fl else None,
                    "screen_chars_mean": round(sum(sc) / len(sc), 1) if sc else None},
    }
    out["pass"] = all(out[k]["pass"] for k in ("text_contrast", "nontext_contrast", "min_font", "touch", "colour"))
    return out


def reduced_motion_loss(rec_anim, rec_still) -> dict:
    """Information lost with reduced motion (SPEC §G7.7): the text runs and glyph classes of
    the final screen with motion on, minus those with motion off (must be empty)."""
    def content(r):
        s = set()
        for fid, f in ((r.get("engine") or {}).get("face_scenes") or {}).items():
            t, g = face_elements(f["scene"])
            s |= {("text", x["text"]) for x in t}
            if g:
                s.add(("glyph", fid))
        s |= {("dom", t["t"]) for t in r["dom"]["texts"]}
        return s
    lost = sorted(content(rec_anim) - content(rec_still))
    return {"lost": lost, "count": len(lost), "pass": not lost}


def tap_gate_note():
    return ("Tap targets: the DOM boxes of buttons, fields, faces and the open scene, and the glyph "
            "hit boxes of the engine's face scenes. Chart glyph hit boxes need the open scene's SceneDesc "
            "(sent with the chart depth); counted when present in the record.")


# --- M18 story chart (chart depth) ----------------------------------------------------------
# The open scene reaches the harness only as the engine's flat paint list (`scene_desc`:
# prims in paint order, no layers, no hit boxes; SPEC §G11.5). Element measures written
# before any M18 reading:
#  - text: every text prim over what is under its anchor (as faces), plus the DOM texts;
#  - non-text (1.4.11): every non-text prim drawn in a meaning colour of the design's look
#    (up, down, closed, invalid, valid, focus, neutral, warn) with alpha >= 0.5 (tints and
#    halos below that are decoration), over what is under it; closed marks included (A31);
#  - meaning colours: the pairs of those roles that the chart actually paints as non-text
#    (distinct colours), ΔE00 in five conditions as §G7.4 (colour-coded pairs only; marks
#    <= 16 px cannot be told without hit boxes, so the stricter threshold 15 is used);
#  - touch: the DOM boxes (story hit boxes are not in the paint list: open risk §G11.5).
CHART_ROLES = ("up", "down", "closed", "invalid", "valid", "focus", "neutral", "warn")
MARK_ALPHA_MIN = 0.5


def _hex(c: str):
    c = c.lstrip("#")
    v = [int(c[i:i + 2], 16) for i in range(0, len(c), 2)]
    return tuple(v) if len(v) == 4 else (*v, 255)


def _paint_bg_at(paint, prims, idx, x, y):
    bg = _hex(paint["bg"])[:3]
    for p in prims[:idx]:
        k = p.get("k")
        hit = False
        if k == "fill" and len(p.get("pts", [])) >= 3:
            hit = _inside_poly(x, y, p["pts"])
        elif k == "rect":
            hit = p["x"] <= x < p["x"] + p["w"] and p["y"] <= y < p["y"] + p["h"]
        if hit:
            bg = C.composite(_hex(p["c"]), bg)
    return bg


def _anchor(p):
    k = p.get("k")
    if k == "circle":
        return p["x"], p["y"]
    if k == "rect":
        return p["x"] + p["w"] / 2, p["y"] + p["h"] / 2
    pts = p.get("pts") or []
    if not pts:
        return None
    if k == "fill":
        return sum(q[0] for q in pts) / len(pts), sum(q[1] for q in pts) / len(pts)
    a, b = pts[0], pts[min(1, len(pts) - 1)]
    return (a[0] + b[0]) / 2, (a[1] + b[1]) / 2


def _role_colours(rec, roles=CHART_ROLES):
    look = rec.get("look_record") or {}
    theme = (rec.get("shot") or {}).get("theme", "light")
    cols = (look.get("colors") or {}).get(theme) or {}
    return {r: cols[r] for r in roles if isinstance(cols.get(r), str)}


def chart_plot(rec):
    """[x0, x1, y0, y1] of the price plot in screenshot CSS px: the x extent of the candle
    bodies (rects in the look's up/down colours) and the scene's height; None if unknown."""
    paint = (rec.get("paints") or {}).get("scene")
    sr = rec.get("scene_rect")
    if not paint or not sr:
        return None
    s = float(paint.get("scale", 1)) or 1.0
    cols = {_hex(c)[:3] for c in _role_colours(rec, ("up", "down")).values()}
    # A rect under a text (the last-price tag on the axis) is a label, not a candle.
    anchors = [(p["x"], p["y"] - 3) for p in paint["prims"] if p.get("k") == "text"]
    xs = [(p["x"] / s, (p["x"] + p["w"]) / s) for p in paint["prims"]
          if p.get("k") == "rect" and _hex(p["c"])[:3] in cols and p["h"] > 0
          and not any(p["x"] - 2 <= ax <= p["x"] + p["w"] + 2 and p["y"] <= ay <= p["y"] + p["h"] for ax, ay in anchors)]
    if len(xs) < 2:
        return None
    return [sr[0] + min(a for a, _ in xs), sr[0] + max(b for _, b in xs), sr[1], sr[1] + sr[3]]


def chart_elements(rec):
    paint = (rec.get("paints") or {}).get("scene")
    if not paint:
        return [], [], set()
    s = float(paint.get("scale", 1)) or 1.0
    prims = paint["prims"]
    roles = {}
    for r, c in _role_colours(rec).items():
        roles.setdefault(_hex(c)[:3], []).append(r)
    texts, marks, used = [], [], set()
    for i, p in enumerate(prims):
        k = p.get("k")
        if k == "text":
            m = re.search(r"([\d.]+)px", p.get("font", ""))
            px = float(m.group(1)) / s if m else 0.0
            x = p["x"] + (-0.3 * px * s if p.get("align") == "right" else 0.3 * px * s)
            y = p["y"] - 0.35 * px * s
            bg = _paint_bg_at(paint, prims, i, x, y)
            fg = C.composite(_hex(p["c"]), bg)
            texts.append({"text": p["t"], "px": px, "fg": fg, "bg": bg, "ratio": C.contrast(fg, bg),
                          "threshold": C.text_threshold(px, False)})
            continue
        col = _hex(p["c"])
        rs = roles.get(col[:3])
        if not rs or col[3] / 255.0 < MARK_ALPHA_MIN:
            continue
        a = _anchor(p)
        if not a:
            continue
        bg = _paint_bg_at(paint, prims, i, a[0], a[1])
        fg = C.composite(col, bg)
        if C.de(fg, bg) < 2.0:
            continue
        marks.append({"roles": rs, "k": k, "fg": fg, "bg": bg, "ratio": C.contrast(fg, bg)})
        used.add(col[:3])
    return texts, marks, used


def chart_gates(records, look=None):
    """Hard gates of one design over its chart shots (M18; SPEC §G7, §G9 step 1)."""
    worst_text = worst_mark = min_font = None
    small_taps, sc, chars = [], [], []
    used = {}
    for r in records:
        r = dict(r, look_record=look)
        stexts, marks, u = chart_elements(r)
        texts = stexts + dom_texts(r)
        for t in texts:
            margin = t["ratio"] / t["threshold"]
            if worst_text is None or margin < worst_text["margin"]:
                worst_text = {"margin": margin, "ratio": round(t["ratio"], 2), "threshold": t["threshold"],
                              "text": t["text"][:40], "px": t["px"], "shot": r["shot"]["id"]}
            if t["px"] and (min_font is None or t["px"] < min_font["px"]):
                min_font = {"px": t["px"], "text": t["text"][:40], "shot": r["shot"]["id"]}
        for m in marks:
            if worst_mark is None or m["ratio"] < worst_mark["ratio"]:
                worst_mark = {"ratio": round(m["ratio"], 2), "roles": m["roles"], "prim": m["k"],
                              "shot": r["shot"]["id"]}
        for t in r["dom"]["taps"]:
            if min(t["rect"][2], t["rect"][3]) < TOUCH_MIN:
                small_taps.append({"what": f"{t['tag']}#{t['id']}", "w": round(t["rect"][2], 1),
                                   "h": round(t["rect"][3], 1), "shot": r["shot"]["id"]})
        for role, hx in _role_colours(r).items():
            if _hex(hx)[:3] in u:
                used.setdefault(_hex(hx)[:3], set()).add(role)
        sc.append(sum(len(t["text"]) for t in texts))
        chars.append(sum(len(t["text"]) for t in stexts))
    table = []
    for (ca, ra), (cb, rb) in combinations(sorted(used.items()), 2):
        row = {"a": "/".join(sorted(ra)), "b": "/".join(sorted(rb)), "threshold": DE_MIN_SMALL}
        for cond in C.CONDITIONS:
            row[cond] = round(C.pair_de(ca, cb, cond), 2)
        row["min"] = min(row[c] for c in C.CONDITIONS)
        row["colour_coded"] = row["normal"] >= COLOUR_CODED
        row["pass"] = (not row["colour_coded"]) or row["min"] >= row["threshold"]
        table.append(row)
    out = {
        "text_contrast": {"worst": worst_text, "pass": bool(worst_text) and worst_text["margin"] >= 1.0},
        "nontext_contrast": {"worst_glyph": worst_mark, "threshold": NONTEXT_MIN,
                             "pass": bool(worst_mark) and worst_mark["ratio"] >= NONTEXT_MIN},
        "min_font": {"worst": min_font, "threshold": MIN_FONT, "pass": bool(min_font) and min_font["px"] >= MIN_FONT},
        "touch": {"below_44": small_taps[:30], "count": len(small_taps), "pass": not small_taps,
                  "note": "DOM boxes only; story hit boxes are not in the paint list (SPEC §G11.5)"},
        "colour": {"classes": sorted(p for v in used.values() for p in v), "pairs": table,
                   "pass": all(r["pass"] for r in table)},
        "roles": {},
        "density": {"face_chars_mean": round(sum(chars) / len(chars), 1) if chars else None,
                    "face_chars_max": max(chars) if chars else None, "face_lines_max": None,
                    "screen_chars_mean": round(sum(sc) / len(sc), 1) if sc else None},
    }
    out["pass"] = all(out[k]["pass"] for k in ("text_contrast", "nontext_contrast", "min_font", "touch", "colour"))
    return out

