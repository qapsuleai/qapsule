"""Denetçi (R5/A madde 3–4): görünüş kaydından (qp look show <ad>) rol düzeyinde
(1) WCAG karşıtlığı (metin ve metin dışı roller, iki tema) ve
(2) anlam renklerinin beş görüş koşulunda ΔE00'ı (Ek G5 kuralının yazılı metnine göre:
    roller up, down, neutral, valid, invalid, closed + işaretin mürekkep rolü `text`; her rol kendi
    alfasıyla tema zeminine bindirilir; normal görüşte ΔE00 < 5 olan çiftler "bilerek paylaşılan"
    sayılır, çift sayılmaz; eşik 10, yüzdeki en küçük işaret ≤ 16 px ise 15).
Kendi renk uygulamam: colour.py."""
import itertools
import json
import sys

from colour import contrast, de_conditions, de2000, hex2rgba, over

MEANING = ["up", "down", "neutral", "valid", "invalid", "closed", "text"]
TEXT_PAIRS = [  # (fg role, bg role, kind)
    ("text", "bg", "text"), ("muted", "bg", "text"), ("valid", "bg", "text"),
    ("invalid", "bg", "text?"), ("closed", "bg", "text?"), ("neutral", "bg", "text?"),
    ("up", "bg", "text?"), ("down", "bg", "text?"), ("accent", "bg", "text?"), ("warn", "bg", "text?"),
    ("chrome.fg", "chrome.bg", "text"), ("chrome.fg", "chrome.card", "text"),
    ("chrome.muted", "chrome.bg", "text"), ("chrome.muted", "chrome.card", "text"),
    ("chrome.accent", "chrome.card", "text"), ("chrome.accent", "chrome.bg", "text"),
    ("chrome.on_accent", "chrome.accent", "text"), ("chrome.note_fg", "chrome.note_bg", "text"),
    ("chrome.alert_fg", "chrome.alert_bg", "text"), ("chrome.req", "chrome.card", "text"),
]
NONTEXT = ["up", "down", "closed", "invalid", "neutral", "focus", "accent", "axis", "line", "grid", "chrome.line"]


def role(cols, name):
    d = cols
    for p in name.split("."):
        d = d[p]
    return hex2rgba(d)


def main(path, mark_px=None):
    look = json.load(open(path))
    name = look.get("name", path)
    mark_px = mark_px or look.get("glyph", {}).get("face_px", 28)
    thr = 15 if mark_px <= 16 else 10
    print(f"== look {name} (mark {mark_px} px → ΔE threshold {thr})")
    worst_pairs = []
    for theme, cols in look["colors"].items():
        bg = role(cols, "bg")[:3]
        alpha = {"closed": cols.get("closed_alpha", 255)}
        print(f"-- {theme}: contrast (role vs background)")
        for fg, b, kind in TEXT_PAIRS:
            try:
                c = contrast(over(role(cols, fg), role(cols, b)[:3]), role(cols, b)[:3])
            except KeyError:
                continue
            flag = "" if c >= 4.5 else ("  <4.5" if kind == "text" else "  (<4.5 if used as small text)")
            print(f"   {fg:18s} on {b:12s} {c:5.2f}{flag}")
        for nt in NONTEXT:
            try:
                rgba = role(cols, nt)
            except KeyError:
                continue
            c = contrast(over(rgba, bg, alpha.get(nt)), bg)
            print(f"   non-text {nt:12s} {c:5.2f}{'  <3' if c < 3 else ''}")
        blended = {r: over(role(cols, r), bg, alpha.get(r)) for r in MEANING}
        shared, rows = [], []
        for a, b in itertools.combinations(MEANING, 2):
            if de2000(blended[a], blended[b]) < 5:
                shared.append(f"{a}={b}")
                continue
            d = de_conditions(blended[a], blended[b])
            k = min(d, key=d.get)
            rows.append((d[k], a, b, k, d))
        rows.sort()
        fails = [r for r in rows if r[0] < thr]
        print(f"-- {theme}: ΔE00 meaning pairs: {len(rows)} counted, shared {shared}; "
              f"fails (<{thr}): {len(fails)}")
        for v, a, b, k, d in rows[:4]:
            print(f"   {a:8s}↔{b:8s} min {v:6.2f} ({k})  " + " ".join(f"{c}={x:.2f}" for c, x in d.items()))
        worst_pairs.append((rows[0][0], theme, rows[0][1], rows[0][2], rows[0][3]))
    w = min(worst_pairs)
    print(f"== {name}: worst counted meaning pair ΔE00 {w[0]:.2f} ({w[1]} {w[2]}↔{w[3]}, {w[4]})")


if __name__ == "__main__":
    main(sys.argv[1], float(sys.argv[2]) if len(sys.argv) > 2 else None)
