#!/usr/bin/env python3
"""R3/HT — insan testi puanlayıcısı (yalnız Python standart kitaplığı).

Kullanım:  python3 -I score.py answers.csv [key.json] > sonuc.md

Girdi (uzun biçim, bir satır = bir madde; ayrıntı SCORING.md):
  participant,group,booklet,mode,item,kind,direction,strength,phase,confidence,when,bars_ago,still_valid,text
  item: 1-A1 … 1-D4, 1-L1 … 1-L4 (Bölüm 1), 2-… (Bölüm 2), S1 … S8 (cümleler), T1 … T3 (tercih).
Bölüm → alfabe eşlemesi kitapçıktan gelir (key.json "booklets": A = siluet önce, B = rozet önce).
Boş ya da tanınmayan cevap yanlış sayılır (tanınmayanlar uyarı olarak listelenir).
Çıktı belirleyicidir: aynı girdi → bayt düzeyinde aynı rapor.
"""

import csv
import json
import math
import os
import sys
from collections import Counter, defaultdict

GLYPH_FIELDS = ["kind", "direction", "strength", "phase"]
KINDS = ["continuation", "reversal", "breakout", "squeeze", "extreme", "flow"]
KIND_TR = {"continuation": "devam", "reversal": "dönüş", "breakout": "kırılım",
           "squeeze": "sıkışma", "extreme": "aşırılık", "flow": "akış"}
CHANCE = {"kind": 1 / 6, "direction": 1 / 3, "strength": 1 / 2, "phase": 1 / 5,
          "full": 1 / 180, "fields": (1 / 6 + 1 / 3 + 1 / 2 + 1 / 5) / 4}
S_CHANCE = {"kind": 1 / 6, "direction": 1 / 3, "what": 1 / 18, "when": 1 / 3,
            "still_valid": 1 / 3, "full": 1 / 162}
CONDITIONS = ["nolegend", "legend", "learn"]
COND_TR = {"nolegend": "açıklamasız (A1–B4)", "legend": "açıklamalı (C1–D4)",
           "learn": "öğrenme (L1–L4)", "all": "hepsi (20 madde)"}
PREF_TR = {"siluet": "siluet (M)", "rozet": "rozet (N)", "same": "fark yok", "boş": "boş", "?": "tanınmadı"}
GROUP_TR = {"trader": "alışkın", "young": "genç", "?": "grup yok"}

TR_FOLD = str.maketrans({"ş": "s", "ç": "c", "ğ": "g", "ı": "i", "ö": "o", "ü": "u",
                         "Ş": "s", "Ç": "c", "Ğ": "g", "İ": "i", "Ö": "o", "Ü": "u", "â": "a"})


def fold(s):
    s = (s or "").strip().translate(TR_FOLD).lower()
    return " ".join(s.replace("_", " ").split())


ALIASES = {
    "kind": {
        "continuation": ["dv", "devam", "continuation"],
        "reversal": ["dn", "donus", "reversal"],
        "breakout": ["kr", "kirilim", "breakout"],
        "squeeze": ["sk", "sikisma", "squeeze"],
        "extreme": ["as", "asirilik", "extreme"],
        "flow": ["ak", "akis", "flow"],
    },
    "direction": {
        "up": ["up", "u", "↑", "yukari", "y"],
        "down": ["down", "d", "↓", "asagi", "a"],
        "neutral": ["neutral", "n", "none", "—", "-", "–", "yonsuz", "0", "belirsiz"],
    },
    "strength": {"low": ["low", "l", "z", "zayif"], "high": ["high", "h", "g", "guclu"]},
    "phase": {
        "candidate": ["candidate", "ad", "aday"],
        "final": ["final", "ke", "kesin"],
        "realized": ["realized", "ge", "gerceklesti"],
        "failed": ["failed", "gm", "gerceklesmedi"],
        "retracted": ["retracted", "gc", "geri cekildi"],
    },
    "when": {
        "past": ["past", "g", "gecmis", "gecmiste"],
        "now": ["now", "s", "su an", "su an suruyor", "simdi"],
        "conditional": ["conditional", "k", "kosul", "kosullu", "bir kosula bagli"],
    },
    "still_valid": {
        "yes": ["yes", "e", "evet"],
        "no": ["no", "h", "hayir"],
        "unknown": ["unknown", "b", "bilinmiyor"],
    },
    "group": {"trader": ["trader", "t", "alisik", "alisik kullanici", "islem yapan"],
              "young": ["young", "y", "genc", "genc islem yapmayan", "islem yapmayan genc"]},
    "pref": {"siluet": ["m", "siluet"], "rozet": ["n", "rozet"],
             "same": ["fark yok", "esit", "same", "="]},
}
LOOKUP = {f: {a: canon for canon, al in m.items() for a in al} for f, m in ALIASES.items()}
warnings = []


def norm(field, raw, where):
    v = fold(raw)
    if v == "":
        return ""
    out = LOOKUP[field].get(v)
    if out is None:
        warnings.append(f"{where}: `{field}` için tanınmayan cevap `{raw}` (yanlış sayıldı)")
        return "?"
    return out


def f2(x):
    return "—" if x is None else f"{x:.2f}".replace(".", ",")


def frac(k, n):
    return "—" if n == 0 else f"{f2(k / n)} ({k}/{n})"


def sign_test(pos, neg):
    """Exact two-sided sign test (ties dropped)."""
    n = pos + neg
    if n == 0:
        return None
    k = min(pos, neg)
    p = sum(math.comb(n, i) for i in range(k + 1)) / 2 ** n
    return min(1.0, 2 * p)


def main(argv):
    if len(argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    here = os.path.dirname(os.path.abspath(__file__))
    key_path = argv[2] if len(argv) > 2 else os.path.join(here, "key.json")
    with open(key_path, encoding="utf-8") as fh:
        key = json.load(fh)
    with open(argv[1], encoding="utf-8", newline="") as fh:
        rows = list(csv.DictReader(fh))

    # participant → info, answers
    info = {}
    ans = defaultdict(dict)
    for i, r in enumerate(rows, start=2):
        p = (r.get("participant") or "").strip()
        if not p:
            continue
        where = f"satır {i} ({p} {r.get('item', '')})"
        b = (r.get("booklet") or "").strip().upper()
        g = norm("group", r.get("group"), where) or "?"
        if p in info and (info[p]["booklet"], info[p]["group"]) != (b, g):
            warnings.append(f"{where}: katılımcının kitapçık/grubu satırlar arasında farklı")
        info.setdefault(p, {"booklet": b, "group": g, "mode": (r.get("mode") or "").strip()})
        ans[p][(r.get("item") or "").strip().upper()] = (r, where)

    participants = sorted(info, key=lambda s: (len(s), s))
    alph_names = sorted(key["alphabets"])  # rozet, siluet
    # ---- glyph records
    recs = []  # dict per participant × alphabet × item
    for p in participants:
        b = info[p]["booklet"]
        if b not in key["booklets"]:
            warnings.append(f"{p}: bilinmeyen kitapçık `{b}`; glif maddeleri atlandı")
            continue
        order = key["booklets"][b]
        for sec, alt in enumerate(order, start=1):
            for label, t in key["alphabets"][alt]["items"].items():
                r, where = ans[p].get(f"{sec}-{label}", ({}, f"{p} {sec}-{label}"))
                got = {f: norm(f, r.get(f), where) for f in GLYPH_FIELDS}
                ok = {f: got[f] == t[f] for f in GLYPH_FIELDS}
                conf = (r.get("confidence") or "").strip()
                recs.append({
                    "p": p, "group": info[p]["group"], "alt": alt, "pos": sec,
                    "label": label, "cond": t["condition"], "truth": t, "got": got, "ok": ok,
                    "full": all(ok.values()), "conf": int(conf) if conf in ("1", "2", "3", "4", "5") else None,
                })

    out = []
    w = out.append
    w("# İnsan testi — puan raporu\n")
    w(f"Girdi: `{os.path.basename(argv[1])}` · anahtar: `{os.path.basename(key_path)}` "
      f"(tohum {key['seed']}) · M = siluet, N = rozet.\n")
    w("## Katılımcılar\n")
    cnt = Counter((info[p]["group"], info[p]["booklet"]) for p in participants)
    w("| grup | kitapçık A (siluet önce) | kitapçık B (rozet önce) | toplam |")
    w("|---|---:|---:|---:|")
    for g in ["trader", "young", "?"]:
        a, bb = cnt[(g, "A")], cnt[(g, "B")]
        if a + bb:
            w(f"| {GROUP_TR[g]} | {a} | {bb} | {a + bb} |")
    w(f"| **toplam** | {sum(v for (g, b), v in cnt.items() if b == 'A')} | "
      f"{sum(v for (g, b), v in cnt.items() if b == 'B')} | {len(participants)} |\n")

    def acc_line(sel, label):
        n = len(sel)
        cells = [frac(sum(r["ok"][f] for r in sel), n) for f in GLYPH_FIELDS]
        cells.append(frac(sum(r["full"] for r in sel), n))
        return f"| {label} | " + " | ".join(cells) + " |"

    head = "| | tür | yön | güç | aşama | tam doğru |\n|---|---:|---:|---:|---:|---:|"
    w("## İşaretler: alan başına doğruluk\n")
    w("Şans düzeyi: tür 0,17 · yön 0,33 · güç 0,50 · aşama 0,20 · tam 0,01. "
      "Çoğunluk tabanı (anahtardaki en sık doğru değeri hep söylemek) aşağıda ayrıca.\n")
    for alt in alph_names:
        code = key["alphabets"][alt]["code"]
        w(f"### {alt} (Alfabe {code})\n")
        w(head)
        for c in CONDITIONS + ["all"]:
            sel = [r for r in recs if r["alt"] == alt and (c == "all" or r["cond"] == c)]
            w(acc_line(sel, COND_TR[c]))
        items = key["alphabets"][alt]["items"].values()
        base = []
        for f in GLYPH_FIELDS:
            c = Counter(t[f] for t in items)
            v, k = c.most_common(1)[0]
            base.append(f"{f} {f2(k / len(items))} (`{v}`)")
        w("\nÇoğunluk tabanı (20 madde): " + " · ".join(base) + "\n")

    w("## Gruba göre (bütün 20 madde)\n")
    w("| alfabe · grup | tür | yön | güç | aşama | tam doğru |\n|---|---:|---:|---:|---:|---:|")
    for alt in alph_names:
        for g in ["trader", "young", "?"]:
            sel = [r for r in recs if r["alt"] == alt and r["group"] == g]
            if sel:
                w(acc_line(sel, f"{alt} · {GROUP_TR[g]}"))
    w("")
    w("## Sıra etkisi (bölüm 1 mi bölüm 2 mi)\n")
    w("| alfabe · sıra | tür | yön | güç | aşama | tam doğru |\n|---|---:|---:|---:|---:|---:|")
    for alt in alph_names:
        for pos in (1, 2):
            sel = [r for r in recs if r["alt"] == alt and r["pos"] == pos]
            if sel:
                w(acc_line(sel, f"{alt} · {pos}. bölüm"))
    w("")

    w("## Güven ve doğruluk (tür alanı)\n")
    w("| alfabe | tür doğruyken ort. güven | tür yanlışken ort. güven | güveni boş |\n|---|---:|---:|---:|")
    for alt in alph_names:
        sel = [r for r in recs if r["alt"] == alt]
        okc = [r["conf"] for r in sel if r["ok"]["kind"] and r["conf"]]
        bad = [r["conf"] for r in sel if not r["ok"]["kind"] and r["conf"]]
        mean = lambda v: sum(v) / len(v) if v else None  # noqa: E731
        w(f"| {alt} | {f2(mean(okc))} (n={len(okc)}) | {f2(mean(bad))} (n={len(bad)}) | "
          f"{sum(1 for r in sel if r['conf'] is None)} |")
    w("")

    w("## Tür karışıklık matrisi (satır = doğru tür, sütun = verilen cevap)\n")
    cols = KINDS + ["", "?"]
    for alt in alph_names:
        w(f"### {alt}\n")
        w("| doğru \\ cevap | " + " | ".join(KIND_TR.get(c, "boş" if c == "" else "tanınmadı") for c in cols) + " |")
        w("|---|" + "---:|" * len(cols))
        for k in KINDS:
            c = Counter(r["got"]["kind"] for r in recs if r["alt"] == alt and r["truth"]["kind"] == k)
            w(f"| {KIND_TR[k]} | " + " | ".join(str(c[x]) for x in cols) + " |")
        w("")

    w("## Madde başına tür ve tam doğru (zor işaretleri bulmak için)\n")
    w("| alfabe | etiket | koşul | doğru (tür.yön, güç, aşama) | tür | tam |\n|---|---|---|---|---:|---:|")
    for alt in alph_names:
        for label, t in key["alphabets"][alt]["items"].items():
            sel = [r for r in recs if r["alt"] == alt and r["label"] == label]
            w(f"| {alt} | {label} | {t['condition']} | {t['kind']}.{t['direction']}, {t['strength']}, "
              f"{t['phase']} | {frac(sum(r['ok']['kind'] for r in sel), len(sel))} | "
              f"{frac(sum(r['full'] for r in sel), len(sel))} |")
    w("")

    # ---- paired comparison
    w("## Eşli karşılaştırma: siluet ve rozet\n")
    w("Katılımcı başına iki ölçü: **alan doğruluğu** (20 maddenin 4 alanının ortalaması) ve "
      "**tam doğru oranı**. Fark = siluet − rozet. İşaret testi (eşitlikler atılır, iki yönlü, kesin binom).\n")
    w("| katılımcı | grup | kitapçık | alan siluet | alan rozet | fark | tam siluet | tam rozet | fark |")
    w("|---|---|---|---:|---:|---:|---:|---:|---:|")
    tally = {"fields": [0, 0, 0], "full": [0, 0, 0]}
    diffs = {"fields": [], "full": []}
    for p in participants:
        sc = {}
        for alt in ("siluet", "rozet"):
            sel = [r for r in recs if r["p"] == p and r["alt"] == alt]
            if not sel:
                continue
            sc[alt] = (sum(sum(r["ok"].values()) for r in sel) / (4 * len(sel)),
                       sum(r["full"] for r in sel) / len(sel))
        if len(sc) < 2:
            continue
        row = [p, GROUP_TR[info[p]["group"]], info[p]["booklet"]]
        for i, m in enumerate(("fields", "full")):
            d = sc["siluet"][i] - sc["rozet"][i]
            diffs[m].append(d)
            tally[m][0 if d > 1e-12 else 1 if d < -1e-12 else 2] += 1
            row += [f2(sc["siluet"][i]), f2(sc["rozet"][i]), f2(d)]
        w("| " + " | ".join(row) + " |")
    w("")
    for m, name in (("fields", "alan doğruluğu"), ("full", "tam doğru oranı")):
        pos, neg, tie = tally[m]
        p = sign_test(pos, neg)
        md = sum(diffs[m]) / len(diffs[m]) if diffs[m] else None
        w(f"- **{name}:** siluet daha iyi {pos}, rozet daha iyi {neg}, eşit {tie}; "
          f"ortalama fark {f2(md)}; işaret testi p = {f2(p) if p is not None else '—'}")
    w("")
    w("> **Güç uyarısı.** 5–10 kişiyle işaret testi ancak çok tek yönlü sonuçları yakalar: "
      "n = 6'da bile 6/6 aynı yönde olmadan p < 0,05 olmaz (6/6 → p = 0,031; 5/6 → p = 0,22). "
      "Anlamlı çıkmayan fark \"fark yok\" demek değildir. Karar, sayılarla birlikte karışıklık "
      "matrisi, madde tablosu ve nitel cevaplarla verilir (K39).\n")

    # ---- sentences
    w("## Kapsül cümleleri\n")
    w("Şans düzeyi: tür 0,17 · yön 0,33 · ne zaman 0,33 · hâlâ geçerli 0,33. \"Ne zaman\": "
      "gerçek \"0 bar önce\" (bugün) olan geçmiş olaya `now` da doğru sayılır; \"kesin\" = geçmişte "
      "ise yazılan bar sayısı da doğru (rakamsız cümlelerde bar sayısı yazılamaz; bilgi amaçlı).\n")
    srec = []
    for p in participants:
        for s, t in key["sentences"].items():
            r, where = ans[p].get(s, ({}, f"{p} {s}"))
            got = {f: norm(f, r.get(f), where) for f in ("kind", "direction", "when", "still_valid")}
            ok_when = got["when"] == t["when"] or (t["when"] == "past" and t["bars_ago"] == 0 and got["when"] == "now")
            ba = (r.get("bars_ago") or "").strip()
            exact = ok_when and (t["when"] != "past" or got["when"] == "now"
                                 or (ba.isdigit() and int(ba) == t["bars_ago"]))
            ok = {"kind": got["kind"] == t["kind"], "direction": got["direction"] == t["direction"],
                  "when": ok_when, "still_valid": got["still_valid"] == t["still_valid"]}
            ok["what"] = ok["kind"] and ok["direction"]
            ok["when_exact"] = exact
            ok["full"] = ok["what"] and ok["when"] and ok["still_valid"]
            srec.append({"p": p, "group": info[p]["group"], "s": s, "t": t, "ok": ok})
    sf = ["kind", "direction", "what", "when", "when_exact", "still_valid", "full"]
    w("| | tür | yön | ne (ikisi) | ne zaman | ne zaman (kesin) | hâlâ geçerli | tam |")
    w("|---|---:|---:|---:|---:|---:|---:|---:|")
    for g in ["all", "trader", "young"]:
        sel = [r for r in srec if g == "all" or r["group"] == g]
        if sel:
            w(f"| {'hepsi' if g == 'all' else GROUP_TR[g]} | " +
              " | ".join(frac(sum(r["ok"][f] for r in sel), len(sel)) for f in sf) + " |")
    w("")
    w("| cümle | zaman türü | doğru (tür.yön, ne zaman, geçerli) | ne | ne zaman | hâlâ geçerli | tam |")
    w("|---|---|---|---:|---:|---:|---:|")
    for s, t in key["sentences"].items():
        sel = [r for r in srec if r["s"] == s]
        tag = " **(K26)**" if t["situation"] == "k26" else ""
        w(f"| {s}{tag} | {t['time']}{', rakamsız' if t['redacted'] else ''} | {t['kind']}.{t['direction']}, "
          f"{t['when']}{'' if t['bars_ago'] is None else ' ' + str(t['bars_ago'])}, {t['still_valid']} | "
          + " | ".join(frac(sum(r["ok"][f] for r in sel), len(sel)) for f in ("what", "when", "still_valid", "full")) + " |")
    w("")

    # ---- preferences
    w("## Tercih ve nitel cevaplar\n")
    pref = Counter()
    for p in participants:
        r, where = ans[p].get("T1", ({}, f"{p} T1"))
        pref[norm("pref", r.get("text"), where) or "boş"] += 1
    w("T1 (hangi alfabe daha kolay okundu): " + " · ".join(
        f"{PREF_TR[k]} {pref[k]}" for k in ("siluet", "rozet", "same", "boş", "?") if pref[k]) + "\n")
    for q, name in (("T2", "T2 — karışan işaretler"), ("T3", "T3 — serbest")):
        w(f"**{name}:**\n")
        any_ = False
        for p in participants:
            txt = (ans[p].get(q, ({}, ""))[0].get("text") or "").strip()
            if txt:
                any_ = True
                w(f"- {p} ({info[p]['booklet']}, {GROUP_TR[info[p]['group']]}): {txt}")
        if not any_:
            w("- (boş)")
        w("")

    if warnings:
        w("## Uyarılar\n")
        for x in warnings:
            w(f"- {x}")
        w("")
    sys.stdout.write("\n".join(out))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
