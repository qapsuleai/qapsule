"""R5/D computed measures over the scene descriptions written by render.py (element level,
from the `SceneDesc`; never pixels). Uses the glance harness (`harness/glance/gl/`):
`compute.scene_prims`/`background_at` for what is under a prim, `color.contrast`,
`color.text_threshold`, `color.pair_de` (ΔE00 in normal, grey and three dichromacies).

    python3 rounds/R5/experiments/D/measure.py --work DIR --qp /path/to/qp > out.json

Text boxes: the ink box of each text run measured with the very font file the `ref` renderer
uses (`crates/qp-render/assets/fonts`; Pillow/FreeType advances and ink bounds; `fontdue`
applies no kerning, so FreeType's basic layout is used — `layout_engine=BASIC`).
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from itertools import combinations
from pathlib import Path

from PIL import ImageFont

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "harness" / "glance"))
from gl import color as C  # noqa: E402
from gl import compute as K  # noqa: E402

FONTS = HERE.parents[3] / "crates" / "qp-render" / "assets" / "fonts"
FONT_FILES = {None: "IBMPlexMono-Regular.ttf", "atkinson-next": "AtkinsonHyperlegibleNext-Regular.ttf",
              "atkinson-next-semibold": "AtkinsonHyperlegibleNext-SemiBold.ttf"}
CONDS = C.CONDITIONS
_fonts = {}


def font(name, px):
    key = (name, px)
    if key not in _fonts:
        _fonts[key] = ImageFont.truetype(str(FONTS / FONT_FILES[name]), size=px,
                                         layout_engine=ImageFont.Layout.BASIC)
    return _fonts[key]


def ink_box(t):
    f = font(t.get("font"), float(t["px"]))
    x0, y0, x1, y1 = f.getbbox(t["text"], anchor="ls")
    w = f.getlength(t["text"])
    ox = t["x"] - w if t.get("align") == "right" else (t["x"] - w / 2 if t.get("align") == "center" else t["x"])
    return [ox + x0, t["baseline"] + y0, ox + x1, t["baseline"] + y1]


def inter(a, b, pad=0.0):
    return a[0] < b[2] + pad and b[0] < a[2] + pad and a[1] < b[3] + pad and b[1] < a[3] + pad


def classify_text(where, t, on_tag):
    if on_tag:
        return "last_value"
    role = t.get("role")
    if where == "under":
        return "axis_y" if role == "figure" else "axis_x"
    if role == "title":
        return "title"
    return "legend"


def scene_measures(s):
    W, H = s["width"], s["height"]
    prims = K.scene_prims(s)
    bg0 = tuple(float(v) for v in s["background"][:3])
    under_n = len(s.get("under", []))
    over_start = len(prims) - len(s.get("over", []))
    over_rects = [p for p in s.get("over", []) if p["prim"] == "rect"]
    texts, marks = [], []
    for i, (lid, p) in enumerate(prims):
        k = p["prim"]
        where = "under" if i < under_n else ("over" if i >= over_start else "layer")
        if k == "text":
            px = float(p["px"])
            x = p["x"] + (-0.3 * px if p.get("align") == "right" else 0.3 * px)
            y = p["baseline"] - 0.35 * px
            bg = K.background_at(s, prims, i, x, y)
            on_tag = where == "over" and any(r["x"] <= x < r["x"] + r["w"] and r["y"] <= y < r["y"] + r["h"]
                                             for r in over_rects)
            fg = C.composite(K._rgba(p["color"]), bg)
            box = ink_box(p)
            texts.append({"text": p["text"], "px": px, "cls": classify_text(where, p, on_tag), "where": where,
                          "ratio": C.contrast(fg, bg), "threshold": C.text_threshold(px, False),
                          "fg": fg, "box": box, "font": p.get("font") or "plex-mono"})
            continue
        col = K._rgba(p["color"])
        if where == "under":
            # grid lines (look `grid`) and axis rules (look `axis`); told apart in the summary
            cls = "under_rule"
        elif where == "over":
            cls = "tag_box" if k == "rect" else "over_" + k
        else:
            cls = "series:" + str(lid)
        a = K._anchor({"k": k, **({"pts": p["points"]} if "points" in p else {}),
                       **{kk: p[kk] for kk in ("x", "y", "w", "h") if kk in p}}) if k != "text" else None
        if not a:
            continue
        bg = K.background_at(s, prims, i, a[0], a[1]) if where != "layer" else bg0
        fg = C.composite(col, bg)
        marks.append({"cls": cls, "layer": lid, "k": k, "rgba": col, "fg": fg, "bg": bg, "ratio": C.contrast(fg, bg),
                      "w": p.get("w"), "h": p.get("h"), "width": p.get("width")})
    # overlaps and overflow
    boxes = [(t["text"], t["cls"], t["box"]) for t in texts]
    overlaps, near = [], []
    for (ta, ca, ba), (tb, cb, bb) in combinations(boxes, 2):
        if inter(ba, bb):
            overlaps.append(f"{ca}:{ta} × {cb}:{tb}")
        elif inter(ba, bb, pad=2.0):
            near.append(f"{ca}:{ta} × {cb}:{tb}")
    overflow = [f"{t['cls']}:{t['text']}" for t in texts
                if t["box"][0] < 0 or t["box"][1] < 0 or t["box"][2] > W or t["box"][3] > H]
    # under texts hidden by an over rect (last-value tag drawn over an axis label)
    hidden = [f"{t['cls']}:{t['text']}" for t in texts if t["where"] == "under"
              and any(inter(t["box"], [r["x"], r["y"], r["x"] + r["w"], r["y"] + r["h"]]) for r in over_rects)]
    ink = sum((t["box"][2] - t["box"][0]) * (t["box"][3] - t["box"][1]) for t in texts)
    chars = sum(len(t["text"]) for t in texts)
    # candle body width (density of the data marks)
    cw = [m["w"] for m in marks if m["cls"] == "series:candles" and m["k"] == "rect" and m["w"] and m["h"] and m["h"] > m["w"]]
    return {"W": W, "H": H, "texts": texts, "marks": marks, "overlaps": overlaps, "near": near,
            "overflow": overflow, "hidden": hidden, "n_texts": len(texts), "chars": chars,
            "chars_per_10k": round(chars / (W * H) * 1e4, 2), "ink_share": round(ink / (W * H), 4),
            "min_px": min(t["px"] for t in texts), "candle_w": (round(sorted(cw)[len(cw) // 2], 2) if cw else None)}


def palette(look_json, theme):
    cols = look_json["colors"][theme]
    hexes = [cols[f"s{i}"] for i in range(1, 9)]
    rgb = [K._hex(h)[:3] for h in hexes]
    bg = K._hex(cols["bg"])[:3]
    pairs = {}
    for (i, a), (j, b) in combinations(enumerate(rgb, 1), 2):
        pairs[f"s{i}-s{j}"] = {c: round(C.pair_de(a, b, c), 1) for c in CONDS}
    contrast = {f"s{i}": round(C.contrast(c, bg), 2) for i, c in enumerate(rgb, 1)}
    meaning = {}
    for a, b in (("up", "down"), ("up", "s1"), ("down", "s1"), ("s1", "s2"), ("up", "s2"), ("down", "s2")):
        ca, cb = K._hex(cols[a])[:3], K._hex(cols[b])[:3]
        meaning[f"{a}-{b}"] = {c: round(C.pair_de(ca, cb, c), 1) for c in CONDS}
    hier = {"text/bg": round(C.contrast(K._hex(cols["text"])[:3], bg), 2),
            "muted/bg": round(C.contrast(K._hex(cols["muted"])[:3], bg), 2),
            "text-muted ΔE00": round(C.de(K._hex(cols["text"])[:3], K._hex(cols["muted"])[:3]), 1),
            "axis/bg": round(C.contrast(K._hex(cols["axis"])[:3], bg), 2),
            "grid/bg": round(C.contrast(K._hex(cols["grid"])[:3], bg), 2)}
    return {"pairs": pairs, "contrast": contrast, "meaning": meaning, "hier": hier}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", required=True)
    ap.add_argument("--qp", required=True)
    a = ap.parse_args()
    out = {"scenes": {}, "palette": {}}
    for f in sorted(Path(a.work).glob("*-*-*-*.json")):
        m = scene_measures(json.loads(f.read_text()))
        for t in m["texts"]:
            t["box"] = [round(v, 1) for v in t["box"]]
            t["fg"] = [round(v) for v in t["fg"]]
            t["ratio"] = round(t["ratio"], 2)
        for mk in m["marks"]:
            mk["fg"] = [round(v) for v in mk["fg"]]
            mk["bg"] = [round(v) for v in mk["bg"]]
            mk["ratio"] = round(mk["ratio"], 2)
        out["scenes"][f.stem] = m
    for look in ("oltan", "r4"):
        lj = json.loads(subprocess.run([a.qp, "look", "show", look], check=True, capture_output=True, text=True).stdout)
        out["palette"][look] = {th: palette(lj, th) for th in ("light", "dark")}
    json.dump(out, sys.stdout, ensure_ascii=False)


if __name__ == "__main__":
    main()
