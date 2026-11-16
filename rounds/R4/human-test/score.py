#!/usr/bin/env python3
"""R4/HT — insan testi v2 puanlayıcısı ve karar kuralı (yalnız Python standart kitaplığı).

Kullanım:
  python3 -I score.py answers.csv [key.json] [plan.json] > results.md   puan raporu + karar
  python3 -I score.py --sequence [n]                                   katılımcı sırası tablosu

Girdi (uzun biçim, bir satır = bir cevap; ayrıntı FORM.md):
  participant,group,mode,item,q,answer,hint,note,quote
Katılımcı kodu grup harfi + sıra: A01, A02 … (alışkın), G01, G02 … (genç). Her karşılaştırmada
hangi tasarımın önce gösterildiği koddan hesaplanır (`sequence`); CSV'de tasarım adı yoktur.
Boş ya da tanınmayan cevap yanlış; ipucuyla verilen cevap (hint = evet) yanlış sayılır ve ayrıca
raporlanır. Çıktı belirleyicidir: aynı girdi → bayt düzeyinde aynı rapor.
"""

import csv
import json
import math
import os
import re
import sys
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))

# ---- the counterbalancing (GUIDE "Sıra"): order = which design is shown first ----------
# Task order in the session is fixed (plan.json "order"); within a task set 1 always comes
# first, so "first design" = the design shown with set 1. Bit of comparison c for the k-th
# participant of a group (k = 0, 1, …): parity(k & MASK[c]) (XOR FLIP[c]); 0 → A first.
# Every mask has bit 0, so each comparison is balanced after every two participants of a group.
COMPARISONS = ["alphabet", "face", "transition", "outcome", "feed"]
MASK = {"alphabet": 1, "face": 3, "transition": 5, "outcome": 7, "feed": 3}
FLIP = {"alphabet": 0, "face": 0, "transition": 0, "outcome": 0, "feed": 1}
TASK = {"alphabet": "G5", "face": "G1", "transition": "G3", "outcome": "G4", "feed": "G2"}
TASK_TR = {"G5": "alfabe (işaret)", "G1": "kapsülün yüzü (ilk bakış)", "G3": "açılma geçişi",
           "G4": "sonuç işareti", "G2": "akış düzeni"}
GROUP_TR = {"A": "alışkın", "G": "genç"}
WEIGHT = {"A": 1, "G": 2}  # genç = geleceğin yatırımcısı (persona F, ağırlık 2), alışkın = H (1)


def code_parts(p):
    m = re.fullmatch(r"([AG])(\d{2})", p.strip().upper())
    return (m.group(1), int(m.group(2))) if m else (None, None)


def first_side(p, comp):
    g, n = code_parts(p)
    k = n - 1
    bit = (bin(k & MASK[comp]).count("1") & 1) ^ FLIP[comp]
    return "A" if bit == 0 else "B"


# ---- options (FORM.md; numbers on the card, words also accepted) ---------------------------
OPTS = {
    "s1": ["yukarı", "aşağı", "belirsiz", "anlamadım"],
    "s1b": ["fiyat önemli bir seviyeyi geçti", "yön değişiyor", "hareket sürüyor",
            "fiyat sıkıştı, hareketsiz", "aşırı gitti", "büyük alım ya da satım var", "anlamadım"],
    "s2": ["bugün", "dün", "birkaç gün önce", "bir haftadan önce", "birkaç saat önce", "bilmiyorum"],
    "s3": ["evet, sürüyor", "hayır, artık geçerli değil", "kapandı: gerçekleşti", "kapandı: tutmadı",
           "kapandı: süresi doldu ya da geri çekildi", "bilmiyorum"],
    "s4": ["geçmişte çoğunlukla tuttu", "yarı yarıya", "çoğunlukla tutmadı", "az örnek var, bilinmiyor",
           "hiçbir bilgi göremedim"],
    "s5": ["al", "sat", "bekle", "hiçbir şey, yalnız bilgi veriyor", "bilmiyorum"],
    "sonuc": ["gerçekleşti", "tutmadı", "hâlâ açık", "bilmiyorum"],
    "yer": ["doğru yer", "yanlış yer", "gösteremedi"],
    "ayni": ["evet", "hayır", "emin değilim"],
    "g2b_ne": ["tutmadı, geçersiz oldu", "zayıfladı", "geri çekildi", "başka", "bilmiyorum"],
    "pref": ["1", "2", "fark yok"],
}
OPTS["yon"], OPTS["ne"], OPTS["durum"] = OPTS["s1"], OPTS["s1b"], OPTS["s3"]
FACE_Q = ["s1", "s1b", "s2", "s3", "s4", "s5"]
GLYPH_Q = ["yon", "ne", "durum"]
TR_FOLD = str.maketrans({"ş": "s", "ç": "c", "ğ": "g", "ı": "i", "ö": "o", "ü": "u", "â": "a",
                         "Ş": "s", "Ç": "c", "Ğ": "g", "İ": "i", "Ö": "o", "Ü": "u", "Â": "a"})
warnings = []


def fold(s):
    s = (s or "").strip().translate(TR_FOLD).lower()
    return " ".join(re.sub(r"[.,;:]", " ", s).split())


def norm(q, raw, where):
    """Option number or words → canonical option text; '' empty; '?' unrecognised."""
    v = (raw or "").strip()
    if v == "":
        return ""
    opts = OPTS[q]
    if v.isdigit() and 1 <= int(v) <= len(opts) and q != "pref":
        return opts[int(v) - 1]
    f = fold(v)
    for o in opts:
        if fold(o) == f:
            return o
    if q == "pref" and f in ("esit", "fark yok", "ayni", "0", "="):
        return "fark yok"
    warnings.append(f"{where}: `{q}` için tanınmayan cevap `{raw}` (yanlış sayıldı)")
    return "?"


def f2(x):
    return "—" if x is None else f"{x:.2f}".replace(".", ",")


def frac(k, n):
    return "—" if n == 0 else f"{f2(k / n)} ({k:g}/{n})"


def mean(v):
    return sum(v) / len(v) if v else None


def median(v):
    v = sorted(v)
    if not v:
        return None
    m = len(v) // 2
    return v[m] if len(v) % 2 else (v[m - 1] + v[m]) / 2


def sign_test(pos, neg):
    n = pos + neg
    if n == 0:
        return None
    k = min(pos, neg)
    return min(1.0, 2 * sum(math.comb(n, i) for i in range(k + 1)) / 2 ** n)


SYM = re.compile(r"syn\s*0*(\d{1,2})")
DIRW = {"yukari": "yukarı", "asagi": "aşağı", "belirsiz": "belirsiz", "yonsuz": "belirsiz"}


def parse_changes(raw):
    """'SYN09 yukarı; SYN03 aşağı' → {('SYN09','yukarı'), …}; 'yok' → empty set."""
    out = set()
    for part in re.split(r"[;\n/]+", raw or ""):
        f = fold(part)
        m = SYM.search(f)
        if not m:
            continue
        d = next((DIRW[w] for w in f.split() if w in DIRW), "")
        out.add((f"SYN{int(m.group(1)):02d}", d))
    return out


def f1(got, want):
    if not got and not want:
        return 1.0
    if not got or not want:
        return 0.0
    tp = len(got & want)
    if tp == 0:
        return 0.0
    p, r = tp / len(got), tp / len(want)
    return 2 * p * r / (p + r)


def instrument(raw):
    m = SYM.search(fold(raw))
    return f"SYN{int(m.group(1)):02d}" if m else ("yok" if fold(raw) in ("yok", "hayir", "hicbiri") else "")


# ---- the report ---------------------------------------------------------------------------
def main(argv):
    if len(argv) >= 2 and argv[1] == "--sequence":
        n = int(argv[2]) if len(argv) > 2 else 6
        plan = json.load(open(os.path.join(HERE, "plan.json"), encoding="utf-8"))
        print_sequence(plan, n)
        return 0
    if len(argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    key_path = argv[2] if len(argv) > 2 else os.path.join(HERE, "key.json")
    plan_path = argv[3] if len(argv) > 3 else os.path.join(HERE, "plan.json")
    key = json.load(open(key_path, encoding="utf-8"))["items"]
    plan = json.load(open(plan_path, encoding="utf-8"))
    with open(argv[1], encoding="utf-8", newline="") as fh:
        rows = list(csv.DictReader(fh))

    def design(comp, side):
        v = plan["comparisons"][comp][side]
        return plan["faces"][v[1:]] if v.startswith("@") else v

    info, ans = {}, defaultdict(dict)
    for i, r in enumerate(rows, start=2):
        p = (r.get("participant") or "").strip().upper()
        if not p:
            continue
        g, n = code_parts(p)
        if g is None:
            warnings.append(f"satır {i}: katılımcı kodu `{p}` biçimde değil (A01 / G01); atlandı")
            continue
        info.setdefault(p, {"group": g, "mode": (r.get("mode") or "").strip()})
        item, q = (r.get("item") or "").strip().upper(), (r.get("q") or "").strip().lower()
        if (item, q) in ans[p]:
            warnings.append(f"satır {i}: {p} {item} {q} iki kez yazılmış (sonuncusu alındı)")
        ans[p][(item, q)] = {"raw": r.get("answer") or "", "hint": fold(r.get("hint")) in ("evet", "e", "1", "yes"),
                             "where": f"satır {i} ({p} {item} {q})", "quote": (r.get("quote") or "").strip(),
                             "note": (r.get("note") or "").strip()}
    people = sorted(info, key=lambda s: (s[0], int(s[1:])))

    def get(p, item, q, opt=None):
        a = ans[p].get((item.upper(), q.lower()))
        if a is None:
            return "", False
        v = norm(opt or q, a["raw"], a["where"]) if (opt or q) in OPTS else a["raw"].strip()
        return v, a["hint"]

    def seconds(p, item):
        a = ans[p].get((item.upper(), "sure"))
        if not a:
            return None
        try:
            return float(a["raw"].replace(",", "."))
        except ValueError:
            warnings.append(f"{a['where']}: süre sayı değil `{a['raw']}`")
            return None

    # records: one per participant × task × set (design) with per-item correctness
    recs = []  # {p, group, task, comp, side, design, set, scores:[…], fields:{q:[0/1…]}, hints, secs}
    for p in people:
        g = info[p]["group"]
        for comp in COMPARISONS:
            task = TASK[comp]
            first = first_side(p, comp)
            for set_ in (1, 2):
                side = first if set_ == 1 else ("B" if first == "A" else "A")
                rec = {"p": p, "group": g, "task": task, "comp": comp, "side": side, "design": design(comp, side),
                       "set": set_, "pos": set_, "scores": [], "fields": defaultdict(list), "hints": 0, "secs": []}
                items = sorted(k for k, v in key.items() if v["task"] == task and v["set"] == set_)
                for it in items:
                    kv = key[it]
                    if task == "G1":
                        oks = []
                        for q in FACE_Q:
                            v, h = get(p, it, q)
                            ok = (v == kv["key"][q]) and not h
                            rec["hints"] += h
                            rec["fields"][q].append(ok)
                            oks.append(ok)
                        rec["scores"].append(all(oks))
                        rec["fields"]["s2+s3"].append(oks[2] and oks[3])
                        s = seconds(p, it)
                        if s is not None:
                            rec["secs"].append(s)
                    elif task in ("G5", "G4"):
                        for j, row in enumerate(kv["rows"], start=1):
                            sub = f"{it}.{j}"
                            if task == "G5":
                                oks = []
                                for q, kq in zip(GLYPH_Q, ["s1", "s1b", "s3"]):
                                    v, h = get(p, sub, q)
                                    ok = (v == row["key"][kq]) and not h
                                    rec["hints"] += h
                                    rec["fields"][q].append(ok)
                                    oks.append(ok)
                                rec["scores"].append(all(oks))
                            else:
                                v, h = get(p, sub, "sonuc")
                                ok = (v == row["key"]["outcome"]) and not h
                                rec["hints"] += h
                                rec["fields"]["sonuc"].append(ok)
                                rec["scores"].append(ok)
                        s = seconds(p, it)
                        if s is not None:
                            rec["secs"].append(s)
                    elif task == "G3":
                        vy, hy = get(p, it, "yer")
                        va, ha = get(p, it, "ayni")
                        vs, hs = get(p, it, "s3")
                        oks = [vy == "doğru yer" and not hy, va == kv["key"]["same"] and not ha, vs == kv["key"]["s3"] and not hs]
                        rec["hints"] += hy + ha + hs
                        for q, ok in zip(["yer", "ayni", "s3"], oks):
                            rec["fields"][q].append(ok)
                        rec["scores"].append(sum(oks) / 3)
                        kl, _ = get(p, it, "kayip")
                        if kl.strip().isdigit() and 1 <= int(kl) <= 5:
                            rec["fields"]["kayip"].append(int(kl))
                        s = seconds(p, it)
                        if s is not None:
                            rec["secs"].append(s)
                    elif task == "G2":
                        k2 = kv["key"]
                        want = {tuple(x.split(" ", 1)) for x in k2["g1"]}
                        raw, h1 = get(p, it, "g2a")
                        got = parse_changes(raw)
                        a1 = 0.0 if h1 else f1(got, want)
                        a1i = 0.0 if h1 else f1({x[0] for x in got}, {x[0] for x in want})
                        rb, h2 = get(p, it, "g2b")
                        vb = instrument(rb)
                        wb = set(k2["g2_which"]) or {"yok"}
                        okb_which = vb in wb and not h2
                        vn, h3 = get(p, it, "g2b_ne")
                        okb_what = (not k2["g2_what"] and vb == "yok") or (vn in k2["g2_what"] and not h3)
                        okb = okb_which and okb_what
                        rc, h4 = get(p, it, "g2c")
                        inst = instrument(rc)
                        led = re.search(r"(\d+)\s*/\s*(\d+)", rc or "")
                        okc = any(c["instrument"] == inst and (not led or c["ledger"] == f"{led.group(1)}/{led.group(2)}") for c in k2["g3"]) and not h4
                        rec["hints"] += h1 + h2 + h3 + h4
                        rec["fields"]["g2a"].append(a1)
                        rec["fields"]["g2a_enstrüman"].append(a1i)
                        rec["fields"]["g2b"].append(okb)
                        rec["fields"]["g2c"].append(okc)
                        rec["scores"].append((a1 + okb + okc) / 3)
                        for q in ("g2a", "g2b", "g2c"):
                            s = seconds(p, f"{it}.{q}")
                            if s is not None:
                                rec["secs"].append(s)
                if not items:
                    continue
                recs.append(rec)

    out = []
    w = out.append
    w("# İnsan testi v2 — puan raporu\n")
    w(f"Girdi: `{os.path.basename(argv[1])}` · anahtar: `{os.path.basename(key_path)}` · plan: "
      f"`{os.path.basename(plan_path)}`. Ağırlık: genç 2, alışkın 1 (persona F/H gibi). "
      "Persona ajanı insan değildir; bu rapor insan sonucudur.\n")
    if any(re.fullmatch(r"<<.*>>", v) for v in plan["faces"].values()):
        w("> **Uyarı:** `plan.json` yüz adları doldurulmamış; yüz satırlarında yer tutucu görünür.\n")
    w("## Katılımcılar\n")
    cnt = Counter(info[p]["group"] for p in people)
    modes = Counter(info[p]["mode"] or "?" for p in people)
    w(f"Toplam {len(people)}: alışkın {cnt['A']}, genç {cnt['G']}. Gösterim: "
      + ", ".join(f"{k} {v}" for k, v in sorted(modes.items())) + ".\n")
    w("| katılımcı | " + " | ".join(f"{TASK[c]} önce" for c in COMPARISONS) + " |")
    w("|---|" + "---|" * len(COMPARISONS))
    for p in people:
        w(f"| {p} | " + " | ".join(f"`{design(c, first_side(p, c))}`" for c in COMPARISONS) + " |")
    w("")

    PRIMARY = {"G5": "işaret başına tam doğru (yön + ne + durum)", "G1": "yüz başına tam doğru (S1–S5)",
               "G3": "kapsül başına ortalama (yer, aynı şey, hâlâ geçerli)", "G4": "işaret başına sonuç doğru",
               "G2": "pencere başına ortalama (G2a F1, G2b, G2c)"}
    FIELDS = {"G5": ["yon", "ne", "durum"], "G1": FACE_Q + ["s2+s3"], "G3": ["yer", "ayni", "s3"],
              "G4": ["sonuc"], "G2": ["g2a", "g2a_enstrüman", "g2b", "g2c"]}
    decisions = []
    for comp in COMPARISONS:
        task = TASK[comp]
        da, db = design(comp, "A"), design(comp, "B")
        w(f"## {task} — {TASK_TR[task]}: `{da}` (A) ile `{db}` (B)\n")
        w(f"Birincil ölçü: {PRIMARY[task]}.\n")
        sel = [r for r in recs if r["comp"] == comp]
        w("| tasarım | grup | n | birincil | " + " | ".join(FIELDS[task]) + " | süre medyan (sn) | ipucu |")
        w("|---|---|---:|---:|" + "---:|" * len(FIELDS[task]) + "---:|---:|")
        for side in ("A", "B"):
            for g in ("A", "G", "*"):
                rs = [r for r in sel if r["side"] == side and (g == "*" or r["group"] == g)]
                if not rs:
                    continue
                sc = [x for r in rs for x in r["scores"]]
                cells = []
                for q in FIELDS[task]:
                    v = [x for r in rs for x in r["fields"][q]]
                    cells.append(frac(sum(v), len(v)) if q not in ("g2a", "g2a_enstrüman") else f2(mean(v)))
                secs = [x for r in rs for x in r["secs"]]
                w(f"| `{design(comp, side)}` | {'hepsi' if g == '*' else GROUP_TR[g]} | {len(rs)} | {f2(mean(sc))} | "
                  + " | ".join(cells) + f" | {f2(median(secs))} | {sum(r['hints'] for r in rs)} |")
        if task == "G3":
            for side in ("A", "B"):
                kl = [x for r in sel if r["side"] == side for x in r["fields"]["kayip"]]
                w(f"\n`{design(comp, side)}` — \"nerede olduğumu kaybettim\" (1 hiç … 5 tamamen) ortalaması: {f2(mean(kl))} (n={len(kl)})")
            w("")
        # order effect
        w("\nSıra etkisi (birincil, ilk / ikinci gösterilen): " + " · ".join(
            f"`{design(comp, s)}` {f2(mean([x for r in sel if r['side'] == s and r['pos'] == 1 for x in r['scores']]))} / "
            f"{f2(mean([x for r in sel if r['side'] == s and r['pos'] == 2 for x in r['scores']]))}" for s in ("A", "B")) + "\n")
        # paired
        w("| katılımcı | grup | " + f"`{da}` | `{db}` | fark (A − B) | süre A | süre B | tercih |")
        w("|---|---|---:|---:|---:|---:|---:|---|")
        pos = neg = tie = 0
        diffs, wdiffs, wsum, prefs, tdiffs = [], [], 0, Counter(), []
        per_group = defaultdict(list)
        for p in people:
            ra = next((r for r in sel if r["p"] == p and r["side"] == "A"), None)
            rb = next((r for r in sel if r["p"] == p and r["side"] == "B"), None)
            if not ra or not rb or not ra["scores"] or not rb["scores"]:
                continue
            a, b = mean(ra["scores"]), mean(rb["scores"])
            d = a - b
            diffs.append(d)
            wgt = WEIGHT[info[p]["group"]]
            wdiffs.append(wgt * d)
            wsum += wgt
            per_group[info[p]["group"]].append(d)
            pos, neg, tie = pos + (d > 1e-12), neg + (d < -1e-12), tie + (abs(d) <= 1e-12)
            ta, tb = median(ra["secs"]), median(rb["secs"])
            if ta is not None and tb is not None:
                tdiffs.append(ta - tb)
            pv, _ = get(p, "T", task, "pref")
            pside = {"1": first_side(p, comp), "2": "B" if first_side(p, comp) == "A" else "A"}.get(pv, pv or "boş")
            prefs[pside] += 1
            w(f"| {p} | {GROUP_TR[info[p]['group']]} | {f2(a)} | {f2(b)} | {f2(d)} | {f2(ta)} | {f2(tb)} | "
              f"{design(comp, pside) if pside in ('A', 'B') else pside} |")
        n = len(diffs)
        wd = sum(wdiffs) / wsum if wsum else None
        p_ = sign_test(pos, neg)
        w("")
        w(f"- Eşli: {n} katılımcı; A daha iyi {pos}, B daha iyi {neg}, eşit {tie}; ortalama fark {f2(mean(diffs))}, "
          f"ağırlıklı fark {f2(wd)}; işaret testi p = {f2(p_)} (bilgi için; güç düşük).")
        w(f"- Gruplar: " + ", ".join(f"{GROUP_TR[g]} ort. fark {f2(mean(v))} (n={len(v)})" for g, v in sorted(per_group.items())))
        w(f"- Süre farkı (A − B, katılımcı medyanlarının medyanı): {f2(median(tdiffs))} sn (eksi: A hızlı).")
        w(f"- Tercih: `{da}` {prefs['A']}, `{db}` {prefs['B']}, fark yok {prefs['fark yok']}"
          + (f", boş {prefs['boş']}" if prefs['boş'] else "") + (f", tanınmadı {prefs['?']}" if prefs['?'] else "") + ".\n")
        decisions.append(decide(comp, task, da, db, n, cnt_groups(per_group), wd, pos, neg, prefs, median(tdiffs), per_group))

    # ---- decisions
    w("## Karar (önceden yazılmış kural, `DECISION.md`)\n")
    w("Persona ajanı insan değildir: aşağıdaki insan sonucu persona ölçümünden (M8–M11) farklıysa insan "
      "sonucu geçerlidir.\n")
    w("| karşılaştırma | ön seçim (A) | sonuç | gerekçe |\n|---|---|---|---|")
    for d in decisions:
        w(f"| {d['what']} | `{d['a']}` | **{d['result']}** | {d['why']} |")
    w("")

    # ---- free text
    w("## Katılımcının kendi sözleri ve yürütücü notları\n")
    any_ = False
    for p in people:
        for (item, q), a in sorted(ans[p].items()):
            if a["quote"] or a["note"] or (item == "T" and q == "serbest" and a["raw"].strip()):
                any_ = True
                txt = " · ".join(x for x in [a["raw"].strip() if q == "serbest" else "", f"\"{a['quote']}\"" if a["quote"] else "",
                                              f"(not: {a['note']})" if a["note"] else ""] if x)
                w(f"- {p} {item} {q}: {txt}")
    if not any_:
        w("- (yok)")
    w("")
    if warnings:
        w("## Uyarılar\n")
        for x in warnings:
            w(f"- {x}")
        w("")
    sys.stdout.write("\n".join(out))
    return 0


def cnt_groups(per_group):
    return {g: len(v) for g, v in per_group.items()}


# ---- the pre-written decision rule (DECISION.md; written before any participant) ----------
MIN_N, MIN_PER_GROUP, MIN_DIFF, MIN_MARGIN, PREF_SHARE, TIME_DIFF = 5, 2, 0.10, 2, 2 / 3, 2.0
WHAT = {"alphabet": "A21 alfabe", "face": "yüz (M8 ilk iki)", "transition": "M10 geçiş", "outcome": "M11 sonuç işareti",
        "feed": "M9 akış düzeni"}


def decide(comp, task, da, db, n, groups, wd, pos, neg, prefs, tdiff, per_group):
    d = {"what": WHAT[comp], "a": da}
    if n < MIN_N or any(groups.get(g, 0) < MIN_PER_GROUP for g in ("A", "G")):
        d["result"] = "karar yok (yetersiz katılımcı)"
        d["why"] = f"n = {n} (en az {MIN_N}; her grupta en az {MIN_PER_GROUP}); test sürer, ön seçim `{da}` geçici kalır"
        return d
    # 1. accuracy: weighted paired difference and the sign count; not when the two groups
    #    disagree (each group's mean difference ≥ MIN_DIFF in opposite directions).
    split = None
    if wd is not None and abs(wd) >= MIN_DIFF and ((wd > 0 and pos - neg >= MIN_MARGIN) or (wd < 0 and neg - pos >= MIN_MARGIN)):
        ga, gg = mean(per_group.get("A", [])), mean(per_group.get("G", []))
        if ga is not None and gg is not None and abs(ga) >= MIN_DIFF and abs(gg) >= MIN_DIFF and (ga > 0) != (gg > 0):
            split = f"gruplar ayrışıyor (alışkın {f2(ga)}, genç {f2(gg)}), kural 1 uygulanmadı; "
        else:
            win = da if wd > 0 else db
            d["result"] = f"`{win}`" + (" (onay)" if win == da else " (değişir)")
            d["why"] = f"birincil ölçüde ağırlıklı fark {f2(wd)} (eşik {f2(MIN_DIFF)}), işaret {pos}–{neg}"
            return d
    # 2. preference among those who chose, if accuracy is not decisive
    chose = prefs["A"] + prefs["B"]
    if chose and max(prefs["A"], prefs["B"]) / chose >= PREF_SHARE and max(prefs["A"], prefs["B"]) >= MIN_N // 2 + 1:
        win = da if prefs["A"] > prefs["B"] else db
        d["result"] = f"`{win}`" + (" (onay)" if win == da else " (değişir)")
        d["why"] = (split or f"doğruluk farkı belirleyici değil ({f2(wd)}); ") + f"tercih {prefs['A']}–{prefs['B']} (en az ⅔ ve çoğunluk)"
        return d
    # 3. time (only the transition, where accuracy is expected at the ceiling)
    if task == "G3" and tdiff is not None and abs(tdiff) >= TIME_DIFF:
        win = da if tdiff < 0 else db
        d["result"] = f"`{win}`" + (" (onay)" if win == da else " (değişir)")
        d["why"] = (split or "") + f"doğruluk ve tercih belirleyici değil; süre farkı {f2(tdiff)} sn (eşik {f2(TIME_DIFF)})"
        return d
    d["result"] = f"eşit → `{da}` kalır (onaylanmadı, değiştirilmedi)"
    d["why"] = (split or "") + f"ağırlıklı fark {f2(wd)}, işaret {pos}–{neg}, tercih {prefs['A']}–{prefs['B']}: kural 1–3 karar vermedi"
    return d


# links.html codes (prepare.mjs: U0 base, then A/B of face, transition, outcome, alphabet, feed)
LINK = {"face": (1, 2), "transition": (3, 4), "outcome": (5, 6), "alphabet": (7, 8), "feed": (9, 10)}


def print_sequence(plan, n):
    """Per participant and task: the link for set 1 → the link for set 2 (names: plan.json)."""
    order = [x for x in plan["order"] if x != "P0"]
    comp_of = {TASK[c]: c for c in COMPARISONS}
    print("| kod | " + " | ".join(f"{t} ({TASK_TR[t]})" for t in order) + " |")
    print("|---|" + "---|" * len(order))
    for g in ("A", "G"):
        for k in range(1, n + 1):
            p = f"{g}{k:02d}"
            cells = []
            for t in order:
                c = comp_of[t]
                a, b = LINK[c]
                cells.append(f"U{a} → U{b}" if first_side(p, c) == "A" else f"U{b} → U{a}")
            print(f"| {p} | " + " | ".join(cells) + " |")


if __name__ == "__main__":
    sys.exit(main(sys.argv))
