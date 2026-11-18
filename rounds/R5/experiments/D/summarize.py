"""R5/D: tables (markdown) from measure.py's JSON.

    python3 rounds/R5/experiments/D/summarize.py measure.json --qp /path/to/qp > computed.md
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "harness" / "glance"))
from gl import color as C  # noqa: E402
from gl import compute as K  # noqa: E402

LOOKS = ("oltan", "r4")
THEMES = ("light", "dark")
SIZES = ("phone", "desk", "narrow", "quad")
TEXT_CLS = [("axis_y", "eksen değeri (y)"), ("axis_x", "zaman etiketi (x)"), ("last_value", "son değer etiketi"),
            ("title", "başlık + kapanış"), ("legend", "açıklama (lejant)")]
MARK_CLS = [("grid", "ızgara"), ("axis", "eksen çizgisi"), ("candle_up", "mum: yukarı"), ("candle_down", "mum: aşağı"),
            ("line", "seri çizgisi"), ("hist", "histogram"), ("rule", "eşik kuralı (70/30)"),
            ("band", "bant/alan dolgusu (saydam)"), ("tag_box", "son değer kutusu")]
LINE_LAYERS = {"sma", "rsi", "mid", "close"}
BAND_LAYERS = {"band", "b90", "b60", "b30", "fill"}


def mark_class(m, cols):
    c = m["cls"]
    if c == "under_rule":
        h = "#%02x%02x%02x" % tuple(m["rgba"][:3])
        return "grid" if h == cols["grid"][:7] else "axis" if h == cols["axis"][:7] else "under_other"
    if c == "tag_box":
        return "tag_box"
    if not c.startswith("series:"):
        return None
    lid = c.split(":", 1)[1]
    h = "#%02x%02x%02x" % tuple(m["rgba"][:3])
    if lid == "candles":
        return "candle_up" if h == cols["up"][:7] else "candle_down" if h == cols["down"][:7] else None
    if lid in LINE_LAYERS or lid.startswith("sma"):
        return "line"
    if lid in BAND_LAYERS:
        return "band"
    if lid in ("ob", "os"):
        return "rule"
    if lid == "hist":
        return "hist"
    return None


def key(stem):
    v, look, theme, size = stem.split("-")
    return v, look, theme, size


def fmt(x, d=2):
    return f"{x:.{d}f}".replace(".", ",")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("measure")
    ap.add_argument("--qp", required=True)
    a = ap.parse_args()
    d = json.loads(Path(a.measure).read_text())
    looks = {lk: json.loads(subprocess.run([a.qp, "look", "show", lk], check=True, capture_output=True,
                                           text=True).stdout) for lk in LOOKS}
    out = []
    P = out.append

    # --- 1. text contrast per element (worst over the six views + s8, all sizes) ---
    P("### Metin karşıtlığı, öğe başına (en kötü; yedi görünüm × dört boy)\n")
    P("| öğe | " + " | ".join(f"`{lk}` {th}" for lk in LOOKS for th in THEMES) + " |")
    P("|---|" + "---|" * 4)
    worst_text = {}
    for cls, name in TEXT_CLS:
        row = []
        for lk in LOOKS:
            for th in THEMES:
                ts = [(t, s) for s, m in d["scenes"].items() if key(s)[1] == lk and key(s)[2] == th
                      for t in m["texts"] if t["cls"] == cls]
                if not ts:
                    row.append("—")
                    continue
                t, s = min(ts, key=lambda x: x[0]["ratio"] / x[0]["threshold"])
                ok = t["ratio"] >= t["threshold"]
                row.append(f"{fmt(t['ratio'])} / {fmt(t['threshold'], 1)} ({int(t['px'])} px){'' if ok else ' **✗**'}")
                worst_text[(cls, lk, th)] = (t, s)
        P(f"| {name} | " + " | ".join(row) + " |")
    P("")

    # --- 2. non-text contrast per element ---
    P("### Metin dışı karşıtlık, öğe başına (en kötü; WCAG 1.4.11 ≥ 3)\n")
    P("| öğe | " + " | ".join(f"`{lk}` {th}" for lk in LOOKS for th in THEMES) + " |")
    P("|---|" + "---|" * 4)
    for cls, name in MARK_CLS:
        row = []
        for lk in LOOKS:
            for th in THEMES:
                cols = looks[lk]["colors"][th]
                rs = [mk["ratio"] for s, m in d["scenes"].items() if key(s)[1] == lk and key(s)[2] == th
                      for mk in m["marks"] if mark_class(mk, cols) == cls]
                if not rs:
                    row.append("—")
                    continue
                # grid: the most common value (a grid line's anchor on a crossing grid line reads 1,00)
                r = max(set(rs), key=rs.count) if cls == "grid" else min(rs)
                note = "" if r >= 3 or cls in ("grid", "band") else " **✗**"
                row.append(f"{fmt(r)}{note}")
        P(f"| {name} | " + " | ".join(row) + " |")
    P("\nIzgara: en sık değer (kesişen ızgara çizgisinin üstündeki ölçüm 1,00 okur). Izgara ve saydam dolgu WCAG 1.4.11'in kapsamı dışında sayıldı (süs/arka plan); bilgi için yazıldı.\n")

    # --- 3. min font, density, legends kept ---
    P("### En küçük yazı, metin yoğunluğu, açıklama kaybı (boy başına; yedi görünüm, iki tema aynı)\n")
    P("| boy | look | en küçük yazı | metin öğesi (ort.) | karakter / 10 000 px² (ort.) | metin mürekkep alanı (ort.) | çakışma | taşma | 2 px'ten yakın | açıklama: gösterilen / yazılan |")
    P("|---|---|---:|---:|---:|---:|---:|---:|---:|---|")
    wanted = {"v0": 1, "v1": 1, "v2": 2, "v3": 3, "v4": 0, "v5": 2, "s8": 9}
    for sz in SIZES:
        for lk in LOOKS:
            ms = [(s, m) for s, m in d["scenes"].items() if key(s)[1] == lk and key(s)[3] == sz and key(s)[2] == "light"]
            n = len(ms)
            leg = sum(sum(1 for t in m["texts"] if t["cls"] == "legend") for _, m in ms)
            want = sum(wanted[key(s)[0]] for s, _ in ms)
            P(f"| {sz} | `{lk}` | {fmt(min(m['min_px'] for _, m in ms), 0)} px | {fmt(sum(m['n_texts'] for _, m in ms) / n, 1)} | "
              f"{fmt(sum(m['chars_per_10k'] for _, m in ms) / n)} | {fmt(100 * sum(m['ink_share'] for _, m in ms) / n, 1)} % | "
              f"{sum(len(m['overlaps']) for _, m in ms)} | {sum(len(m['overflow']) for _, m in ms)} | "
              f"{sum(len(m['near']) for _, m in ms)} | {leg} / {want} |")
    P("\nAçıklama sayısı: v0 1, v1 1, v2 2, v3 3, v4 0, v5 2, s8 9 (yedi görünümün toplamı 18). Gramer başlık "
      "satırına sığmayan açıklamayı soldakilerle birlikte düşürür (`compile.rs`, \"the right-most are kept\").\n")
    P("Açıklama kaybı, görünüm başına (gösterilen/yazılan; `oltan` | `r4`):\n")
    P("| görünüm | " + " | ".join(SIZES) + " |")
    P("|---|" + "---|" * len(SIZES))
    for v in wanted:
        row = []
        for sz in SIZES:
            cell = []
            for lk in LOOKS:
                m = d["scenes"][f"{v}-{lk}-light-{sz}"]
                cell.append(str(sum(1 for t in m["texts"] if t["cls"] == "legend")))
            row.append(f"{' | '.join(cell)} / {wanted[v]}".replace(" | ", "·"))
        P(f"| {v} | " + " | ".join(row) + " |")
    P("")

    # --- 4. palette ---
    P("### Seri paleti s1–s8: ΔE00 ayrımı (28 çift; normal, gri, protan, deutan, tritan)\n")
    P("| look / tema | s1–s8 karşıtlık < 3 | en küçük ΔE00: normal / gri / prot / deut / trit | ΔE00 < 10 çift sayısı: normal / gri / prot / deut / trit | ilk k seride en kötü koşulda ΔE00 ≥ 10 kalan en büyük k |")
    P("|---|---|---|---|---|")
    conds = ("normal", "gray", "protan", "deutan", "tritan")
    for lk in LOOKS:
        for th in THEMES:
            pal = d["palette"][lk][th]
            low = [f"{s} {fmt(c)}" for s, c in pal["contrast"].items() if c < 3]
            mins = [min(v[c] for v in pal["pairs"].values()) for c in conds]
            cnt = [sum(1 for v in pal["pairs"].values() if v[c] < 10) for c in conds]
            kmax = 1
            for k in range(2, 9):
                ok = all(min(v.values()) >= 10 for p, v in pal["pairs"].items()
                         if int(p.split("-")[0][1:]) <= k and int(p.split("-")[1][1:]) <= k)
                if ok:
                    kmax = k
                else:
                    break
            P(f"| `{lk}` {th} | {', '.join(low) or 'yok'} | {' / '.join(fmt(x, 1) for x in mins)} | "
              f"{' / '.join(str(x) for x in cnt)} | {kmax} |")
    P("")
    P("En kötü çiftler (en kötü koşulda ΔE00 < 10; `oltan`):\n")
    for th in THEMES:
        pal = d["palette"]["oltan"][th]
        bad = sorted(((min(v.values()), p, min(v, key=v.get)) for p, v in pal["pairs"].items()), key=lambda x: x[0])
        P(f"- {th}: " + "; ".join(f"{p} {fmt(x, 1)} ({c})" for x, p, c in bad if x < 10))
    P("")
    P("### Anlam renkleri ve hiyerarşi (görünüş kaydından)\n")
    P("| look / tema | up–down ΔE00 (normal/gri/prot/deut/trit) | metin/zemin | muted/zemin | metin–muted ΔE00 | eksen/zemin | ızgara/zemin |")
    P("|---|---|---:|---:|---:|---:|---:|")
    for lk in LOOKS:
        for th in THEMES:
            pal = d["palette"][lk][th]
            ud = pal["meaning"]["up-down"]
            h = pal["hier"]
            P(f"| `{lk}` {th} | {' / '.join(fmt(ud[c], 1) for c in conds)} | {fmt(h['text/bg'])} | {fmt(h['muted/bg'])} | "
              f"{fmt(h['text-muted ΔE00'], 1)} | {fmt(h['axis/bg'])} | {fmt(h['grid/bg'])} |")
    P("")
    # candle width
    P("### Veri işaretinin boyu: mum gövdesi ve fitili (v1, px)\n")
    P("| boy | " + " | ".join(f"`{lk}`: gövde / fitil" for lk in LOOKS) + " |")
    P("|---|---|---|")
    for sz in SIZES:
        cells = []
        for lk in LOOKS:
            ws = [mk["w"] for mk in d["scenes"][f"v1-{lk}-light-{sz}"]["marks"] if mk["cls"] == "series:candles"]
            cells.append(f"{max(ws)} / {min(ws)}")
        P(f"| {sz} | " + " | ".join(cells) + " |")
    P("")
    # v3 fan: the three nested tones (b90; b90+b60; b90+b60+b30 over the background)
    P("### Yelpaze (v3) iç içe üç ton: komşu tonlar arası ΔE00 (normal / gri / prot / deut / trit)\n")
    P("| look / tema | zemin → dış | dış → orta | orta → iç |")
    P("|---|---|---|---|")
    for lk in LOOKS:
        for th in THEMES:
            m = d["scenes"][f"v3-{lk}-{th}-desk"]
            fills = {mk["layer"]: mk["rgba"] for mk in m["marks"] if mk["cls"] in ("series:b90", "series:b60", "series:b30")}
            bg = tuple(float(v) for v in K._hex(looks[lk]["colors"][th]["bg"])[:3])
            tones = [bg]
            for lid in ("b90", "b60", "b30"):
                tones.append(C.composite(tuple(fills[lid]), tones[-1]))
            cells = []
            for i in range(3):
                cells.append(" / ".join(fmt(C.pair_de(tones[i], tones[i + 1], c), 1) for c in conds))
            P(f"| `{lk}` {th} | " + " | ".join(cells) + " |")
    print("\n".join(out))


if __name__ == "__main__":
    main()
