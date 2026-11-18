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


def glyph_body(gl_prims, background):
    """The glyph's body (SPEC §G7.2, Ek G4-3): its most opaque prim, i.e. the fill/stroke prim
    of the glyph layer with the highest alpha. Prims that vanish into the background (e.g. a
    knockout in the background colour, ΔE00 < 2) are skipped. Ties (same alpha, different
    colours) go to the colour with the largest area among the tied prims. Its contrast against
    the scene background is the 1.4.11 gate and its colour the meaning colour (§G7.4).

    The dominant colour by alpha-weighted area (the rule applied before Ek G4, under which a
    large translucent plate won) is reported as `aux` when it differs from the body: a
    non-text helper, reported, never gated."""
    bgc = tuple(float(v) for v in background[:3])
    vis = [p for p in gl_prims if C.de(C.composite(_rgba(p["color"]), bgc), bgc) >= 2.0]
    if not vis:
        return None
    top = max(_rgba(p["color"])[3] for p in vis)
    tied, weight = {}, {}
    for p in vis:
        col = _rgba(p["color"])
        if col[3] == top:
            tied[col] = tied.get(col, 0.0) + _area(p)
        weight[col] = weight.get(col, 0.0) + _area(p) * col[3] / 255.0
    col = max(sorted(tied), key=lambda c: tied[c])
    fg = C.composite(col, bgc)
    body = {"color": col, "bg": bgc, "fg": fg, "ratio": C.contrast(fg, bgc)}
    aux = max(sorted(weight), key=lambda c: weight[c])
    if aux != col:
        afg = C.composite(aux, bgc)
        body["aux"] = {"color": aux, "fg": afg, "ratio": C.contrast(afg, bgc)}
    return body


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
    glyph = glyph_body(gl_prims, scene["background"])
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


def _theme(r) -> str:
    return (r.get("shot") or {}).get("theme") or "light"


def semantic_colours(records):
    """Glyph body colour (composited) per theme and meaning class from the shots' target
    faces: {theme: {class: colour}}. Classes are compared only within one theme (Ek G4-2:
    the separation of meaning colours is a separation inside one screen)."""
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
        out.setdefault(_theme(r), {}).setdefault(f"{axis}/{val}", {"fg": g["fg"], "bg": g["bg"], "size": g.get("size", 0)})
    return out


def colour_table(sem, theme=None):
    """ΔE00 of every meaning-colour pair of ONE theme (`sem`: {class: colour}) under normal,
    grey and three CVD conditions."""
    rows = []
    for (a, ca), (b, cb) in combinations(sorted(sem.items()), 2):
        size = min(ca["size"] or 99, cb["size"] or 99)
        thr = DE_MIN_SMALL if size <= SMALL_MARK else DE_MIN
        r = {"a": a, "b": b, "threshold": thr, "mark_px": size}
        if theme is not None:
            r["theme"] = theme
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


def colour_gate(tables):
    """The meaning-colour gate applied to each theme separately (Ek G4-2). `tables`:
    {theme: [pair rows of that theme]}. A theme passes when it has pairs and all pass."""
    themes = {th: {"pairs": rows, "pass": bool(rows) and all(r["pass"] for r in rows)}
              for th, rows in sorted(tables.items())}
    return {"pairs": [r for t in themes.values() for r in t["pairs"]], "themes": themes,
            "pass": bool(themes) and all(t["pass"] for t in themes.values())}


# --- Ek G5: the meaning-colour gate from the look record's ROLES --------------------------
# After Ek G4 the glyph's colour is its most opaque prim; an alphabet that draws its body
# edge in the text colour (`siluet`) then measures directional and non-directional marks as
# the same ink, and the scene-based table compares the marks' outline instead of their
# meaning colour. Ek G5: the pairs come from the look record's meaning roles per theme
# (`qp look show <name>`), plus the roles the design's marks are actually drawn in (glyph
# prims of the shots matched to the record's roles by RGB). The scene-based table stays as
# information (`colour_scene`).
MEANING_ROLES = ("up", "down", "neutral", "valid", "invalid", "closed")
_NOT_MEANING = {"bg", "surface", "grid", "line", "axis", "chrome"}


def _role_hex(look, theme):
    cols = ((look or {}).get("colors") or {}).get(theme) or {}
    return {r: _hex(v) for r, v in cols.items() if isinstance(v, str)}


def mark_roles(records, look):
    """{theme: sorted roles} the marks are drawn in: every glyph prim of every face scene whose
    RGB equals a role colour of the record (not background/surface/grid/series roles)."""
    out = {}
    for r in records:
        th = _theme(r)
        roles = {k: v for k, v in _role_hex(look, th).items()
                 if k not in _NOT_MEANING and not re.fullmatch(r"s\d+", k)}
        for f in ((r.get("engine") or {}).get("face_scenes") or {}).values():
            for lid, p in scene_prims(f["scene"]):
                if lid != "glyph" or p.get("prim") == "text" or "color" not in p:
                    continue
                rgb = tuple(_rgba(p["color"])[:3])
                for k, v in roles.items():
                    if tuple(v[:3]) == rgb:
                        out.setdefault(th, set()).add(k)
    return {th: sorted(v) for th, v in out.items()}


def mark_sizes(records):
    """{theme: smallest glyph size (CSS px) over the face scenes} (threshold 15 when <= 16)."""
    out = {}
    for r in records:
        for f in ((r.get("engine") or {}).get("face_scenes") or {}).values():
            _, g = face_elements(f["scene"])
            if g and g.get("size"):
                th = _theme(r)
                out[th] = min(out.get(th, 1e9), g["size"])
    return out


def role_colour_gate(look, themes, used=None, sizes=None):
    """Ek G5 colour gate: ΔE00 of every pair of meaning roles (MEANING_ROLES + the roles the
    marks are drawn in, `used` {theme: [roles]}) of ONE look, per theme, five conditions.
    Each role colour is composited over the theme background at its own alpha. A pair whose
    roles share (nearly) one colour (normal ΔE00 < 5) is a deliberate share: not counted,
    listed (`shared`), its meaning carried by shape (grey reader question)."""
    tables, shared, sem = {}, {}, {}
    for th in themes:
        hx = _role_hex(look, th)
        if not hx:
            continue
        bg = (hx.get("bg") or (255, 255, 255, 255))[:3]
        roles = [r for r in MEANING_ROLES if r in hx]
        roles += [r for r in (used or {}).get(th, []) if r in hx and r not in roles]
        size = (sizes or {}).get(th) or float((look.get("glyph") or {}).get("face_px") or 0) or 99
        thr = DE_MIN_SMALL if size <= SMALL_MARK else DE_MIN
        fg = {r: C.composite(hx[r], bg) for r in roles}
        sem[th] = {r: "#%02x%02x%02x" % tuple(round(x) for x in fg[r]) for r in roles}
        rows, sh = [], []
        for a, b in combinations(roles, 2):
            row = {"a": a, "b": b, "theme": th, "threshold": thr, "mark_px": size}
            for cond in C.CONDITIONS:
                row[cond] = round(C.pair_de(fg[a], fg[b], cond), 2)
            row["gray_lum_ratio"] = round(C.contrast(C.gray(fg[a]), C.gray(fg[b])), 3)
            row["min"] = min(row[c] for c in C.CONDITIONS)
            row["colour_coded"] = row["normal"] >= COLOUR_CODED
            row["pass"] = (not row["colour_coded"]) or row["min"] >= thr
            (rows if row["colour_coded"] else sh).append(row)
        tables[th] = rows
        shared[th] = [{"a": r["a"], "b": r["b"], "normal": r["normal"]} for r in sh]
    gate = colour_gate(tables)
    for th in gate["themes"]:
        gate["themes"][th]["shared"] = shared.get(th, [])
        gate["themes"][th]["roles"] = sem.get(th, {})
    gate["rule"] = "Ek G5 (roles)"
    gate["shared"] = [dict(s, theme=th) for th, v in shared.items() for s in v]
    return gate


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
    worst_aux = None
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
            a = g.get("aux")
            if a and (worst_aux is None or a["ratio"] < worst_aux["ratio"]):
                worst_aux = {"ratio": round(a["ratio"], 2), "shot": r["shot"]["id"], "face": g["face"]}
        for t in m["taps"]:
            # Only targets that act: the face (opens), the glyph hit box, buttons and fields.
            if min(t["w"], t["h"]) < TOUCH_MIN:
                small_taps.append({"what": t["what"], "w": round(t["w"], 1), "h": round(t["h"], 1), "shot": r["shot"]["id"]})
        fc += m["face_chars"]
        fl += m["face_lines"]
        sc.append(m["screen_chars"])
    sem = semantic_colours(records)
    colour_scene = colour_gate({th: colour_table(s, th) for th, s in sem.items()})
    for th, s in sem.items():
        colour_scene["themes"][th]["classes"] = sorted(s)
    colour_scene["classes"] = sorted({c for s in sem.values() for c in s})
    themes = sorted({_theme(r) for r in records})
    if look and look.get("colors"):
        # Ek G5: the gate from the look record's meaning roles; the scene table is information.
        colour = role_colour_gate(look, themes, mark_roles(records, look), mark_sizes(records))
    else:
        colour = colour_scene
    out = {
        "text_contrast": {"worst": worst_text, "pass": bool(worst_text) and worst_text["margin"] >= 1.0},
        "nontext_contrast": {"worst_glyph": worst_glyph, "threshold": NONTEXT_MIN,
                             "pass": bool(worst_glyph) and worst_glyph["ratio"] >= NONTEXT_MIN,
                             # the largest translucent part when it is not the body (a plate):
                             # a non-text helper, reported, never gated (Ek G4-3)
                             "worst_aux": worst_aux},
        "min_font": {"worst": min_font, "threshold": MIN_FONT, "pass": bool(min_font) and min_font["px"] >= MIN_FONT},
        "touch": {"below_44": small_taps[:30], "count": len(small_taps), "pass": not small_taps},
        "colour": colour,
        "colour_scene": colour_scene,
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
        # Per theme (Ek G4-2): a light-theme colour is never paired with a dark-theme one.
        for role, hx in _role_colours(r).items():
            if _hex(hx)[:3] in u:
                used.setdefault(_theme(r), {}).setdefault(_hex(hx)[:3], set()).add(role)
        sc.append(sum(len(t["text"]) for t in texts))
        chars.append(sum(len(t["text"]) for t in stexts))
    tables = {}
    for th, cols in sorted(used.items()):
        table = tables.setdefault(th, [])
        for (ca, ra), (cb, rb) in combinations(sorted(cols.items()), 2):
            row = {"a": "/".join(sorted(ra)), "b": "/".join(sorted(rb)), "theme": th, "threshold": DE_MIN_SMALL}
            for cond in C.CONDITIONS:
                row[cond] = round(C.pair_de(ca, cb, cond), 2)
            row["min"] = min(row[c] for c in C.CONDITIONS)
            row["colour_coded"] = row["normal"] >= COLOUR_CODED
            row["pass"] = (not row["colour_coded"]) or row["min"] >= row["threshold"]
            table.append(row)
    colour = colour_gate(tables)
    # A chart that paints a single meaning colour in a theme has no pair to fail (as before).
    for t in colour["themes"].values():
        t["pass"] = all(r["pass"] for r in t["pairs"])
    colour["pass"] = all(t["pass"] for t in colour["themes"].values())
    colour["classes"] = sorted({p for cols in used.values() for v in cols.values() for p in v})
    out = {
        "text_contrast": {"worst": worst_text, "pass": bool(worst_text) and worst_text["margin"] >= 1.0},
        "nontext_contrast": {"worst_glyph": worst_mark, "threshold": NONTEXT_MIN,
                             "pass": bool(worst_mark) and worst_mark["ratio"] >= NONTEXT_MIN},
        "min_font": {"worst": min_font, "threshold": MIN_FONT, "pass": bool(min_font) and min_font["px"] >= MIN_FONT},
        "touch": {"below_44": small_taps[:30], "count": len(small_taps), "pass": not small_taps,
                  "note": "DOM boxes only; story hit boxes are not in the paint list (SPEC §G11.5)"},
        "colour": colour,
        "roles": {},
        "density": {"face_chars_mean": round(sum(chars) / len(chars), 1) if chars else None,
                    "face_chars_max": max(chars) if chars else None, "face_lines_max": None,
                    "screen_chars_mean": round(sum(sc) / len(sc), 1) if sc else None},
    }
    out["pass"] = all(out[k]["pass"] for k in ("text_contrast", "nontext_contrast", "min_font", "touch", "colour"))
    return out



# (M20, SPEC Ek G7e) Digits shown to an unentitled reader: words with a digit in the DOM text
# and the open scene's text primitives of the `visitor` shots, the instrument symbol (SYNnn)
# and an entitlement name holding it (`<family>/SYNnn`) excepted. Hard: must be 0 (K33/K34).
_SYM = re.compile(r"(?:[a-z_]+/)?SYN\d+", re.I)


def digit_words(text: str) -> list[str]:
    t = _SYM.sub(" ", text)
    return [w for w in re.split(r"\s+", t) if re.search(r"\d", w)]


def visitor_digits(records):
    hits = []
    n = 0
    for r in records:
        if r["shot"].get("principal") != "visitor":
            continue
        n += 1
        # Not capsule values (applied to every design alike; RESULTS M20 "uygulama notu"): the
        # app clock (`#clock`, the replay time in the header) and the evidence rows' data names
        # as the authority writes them (`requires`, `rests_on`: a series id such as
        # "bars SYN28 1d (feed default)").
        names = {e.get(k) for e in (r.get("evidence") or r.get("evidence_key") or []) for k in ("requires", "rests_on")}
        texts = [t["t"] for t in r["dom"]["texts"] if t.get("id") != "clock" and t["t"] not in names]
        paint = (r.get("paints") or {}).get("scene") or {}
        texts += [p.get("t", "") for p in paint.get("prims", []) if p.get("k") == "text"]
        for t in texts:
            for w in digit_words(t):
                hits.append({"shot": r["shot"]["id"], "kind": r["shot"]["kind"], "word": w, "text": t[:60]})
    return {"shots": n, "count": len(hits), "hits": hits[:20], "pass": n > 0 and not hits}
