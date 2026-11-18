"""R5/D blind fact reading of the six R2 views: `oltan` vs `r4`, phone (402 px) and desktop
(1280 px), light and dark, personas F and H. Uses the glance harness's reader runner
(`gl/readers.py`: one fresh `claude -p --tools Read` process per call, one image under a
random name, one look, one sentence per answer; `check_prompt` keeps design names out) and
`gl/stats.wilson`. Questions are multiple choice with options shuffled per call (seeded) and a
don't-know option last; the key comes from the view's data (`fixtures/views/*.explain.txt`,
`fixtures/views/docs/*.json`), fixed before any call.

    python3 rounds/R5/experiments/D/readers_d.py run   --work DIR --model haiku --reps 3
    python3 rounds/R5/experiments/D/readers_d.py run   --work DIR --model sonnet --reps 1
    python3 rounds/R5/experiments/D/readers_d.py score --work DIR > readers.md

Records (prompt, image id, raw answer, cost, model) go to DIR/readers/ (outside the repo).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import random
import sys
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "harness" / "glance"))
from gl import readers as R  # noqa: E402
from gl import stats as S  # noqa: E402

UNK = "unk"
# qid: (question, [(code, text)], correct code). Turkish only (the views carry no words to
# translate: numbers, dates and study names).
Q = {
    "last": ("Grafikteki en son fiyat kaç?",
             [("a", "124.69"), ("b", "120.00"), ("c", "128.51"), ("d", "131.20")], "a"),
    "low": ("Fiyat en düşük seviyesini hangi dönemde gördü?",
            [("a", "2025-02 ile 2025-05 arasında"), ("b", "2025-05 ile 2025-08 arasında"),
             ("c", "2025-08'den sonra"), ("d", "2025-02'den önce")], "a"),
    "panels1": ("Ekranda üst üste duran kaç ayrı grafik bölmesi (panel) var?",
                [("a", "1"), ("b", "2"), ("c", "3"), ("d", "4")], "a"),
    "panels3": ("Ekranda üst üste duran kaç ayrı grafik bölmesi (panel) var?",
                [("a", "3"), ("b", "1"), ("c", "2"), ("d", "4")], "a"),
    "rsi_mid": ("Ortadaki bölmede adı yazan göstergenin (RSI) son değeri kaç?",
                [("a", "55.10"), ("b", "70.00"), ("c", "30.00"), ("d", "41.20")], "a"),
    "hist": ("En alttaki bölmedeki çubukların en sonuncusu sıfır çizgisinin üstünde mi altında mı?",
             [("a", "altında"), ("b", "üstünde"), ("c", "tam sıfırda")], "a"),
    "close2": ("Son kapanış fiyatı kaç?",
               [("a", "124.69"), ("b", "128.51"), ("c", "124.97"), ("d", "121.43")], "a"),
    "mid": ("Fiyatı saran bandın ortasından geçen çizginin son değeri kaç?",
            [("a", "124.97"), ("b", "124.69"), ("c", "128.51"), ("d", "120.00")], "a"),
    "bandpos": ("Son günlerde fiyat bandın neresinde?",
                [("a", "bandın içinde, orta çizgiye yakın"), ("b", "bandın üst sınırının üstünde"),
                 ("c", "bandın alt sınırının altında")], "a"),
    "fantop": ("Sağdaki gölgeli yelpazenin en üst ucu yaklaşık kaça ulaşıyor?",
               [("a", "yaklaşık 140"), ("b", "yaklaşık 130"), ("c", "yaklaşık 150"), ("d", "yaklaşık 125")], "a"),
    "fantones": ("Sağdaki yelpazede iç içe kaç farklı gölge tonu (bant) var?",
                 [("a", "3"), ("b", "1"), ("c", "2"), ("d", "5")], "a"),
    "last4": ("Grafikteki en son fiyat kaç?",
              [("a", "394.06"), ("b", "200.00"), ("c", "100.00"), ("d", "476.91")], "a"),
    "first4": ("Dönemin başında fiyat yaklaşık kaçtı?",
               [("a", "yaklaşık 10"), ("b", "yaklaşık 50"), ("c", "yaklaşık 100"), ("d", "yaklaşık 200")], "a"),
    "logscale": ("Dikey eksende 10 ile 20 arasındaki mesafe, 100 ile 200 arasındaki mesafeye göre nasıl?",
                 [("a", "aşağı yukarı aynı"), ("b", "çok daha kısa"), ("c", "çok daha uzun")], "a"),
    "above": ("Son günlerde fiyat çizgisi, ortalamasının (gölgeli alanın öbür kenarı) üstünde mi altında mı?",
              [("a", "üstünde"), ("b", "altında"), ("c", "ikisi aynı yerde")], "a"),
    "rsi_low": ("Alt bölmedeki göstergenin (RSI) son değeri kaç?",
                [("a", "55.10"), ("b", "70.00"), ("c", "30.00"), ("d", "44.90")], "a"),
}
SETS = {"v0": ["last", "low", "panels1"], "v1": ["panels3", "rsi_mid", "hist"],
        "v2": ["close2", "mid", "bandpos"], "v3": ["last", "fantop", "fantones"],
        "v4": ["last4", "first4", "logscale"], "v5": ["last", "above", "rsi_low"]}
LOOKS = ("oltan", "r4")
THEMES = ("light", "dark")
SIZES = ("phone", "desk")
PERSONAS = ("F", "H")
EXTRA_FORBIDDEN = ["oltan", "zemre", "bertan", "kivra", "kestim", "look", "tema", "tasarım"]


def seed_of(*parts) -> int:
    return int(hashlib.sha256("|".join(map(str, parts)).encode()).hexdigest()[:12], 16)


def build_prompt(view, persona, file, seed):
    rng = random.Random(seed)
    lines = [R.PERSONAS[persona]["tr"], R.INTRO["tr"].format(file=file)]
    letters = {}
    for n, qid in enumerate(SETS[view], 1):
        text, opts, _ = Q[qid]
        codes = [c for c, _ in opts]
        rng.shuffle(codes)
        codes.append(UNK)
        od = dict(opts)
        letters[f"q{n}"] = {"q": qid, "map": {R.LETTERS[i]: c for i, c in enumerate(codes)}}
        ot = "; ".join(f"{R.LETTERS[i]}) {od[c] if c != UNK else R.UNK_TEXT['tr']}" for i, c in enumerate(codes))
        lines.append(f"q{n}. {text} {ot}")
    keys = "{" + ", ".join(f'"q{n}": "..."' for n in range(1, len(SETS[view]) + 1)) + "}"
    lines.append(R.RULES["tr"].format(keys=keys))
    return "\n".join(lines), letters


def stimuli(work: Path):
    out = []
    for v in SETS:
        for lk in LOOKS:
            for th in THEMES:
                for sz in SIZES:
                    out.append((v, lk, th, sz, work / "r" / f"{v}-{lk}-{th}-{sz}.png"))
    return out


def one(job):
    work, model, v, lk, th, sz, img, persona, rep = job
    code = hashlib.sha256(f"{v}|{lk}|{th}|{sz}|{persona}|{rep}|{model}".encode()).hexdigest()[:10]
    path = work / "readers" / f"{code}.json"
    if path.exists():
        return json.loads(path.read_text())
    seed = seed_of(code)
    holder = {}

    def pf(name):
        p, letters = build_prompt(v, persona, name, seed)
        bad = R.check_prompt(p, EXTRA_FORBIDDEN)
        assert not bad, bad
        holder["letters"] = letters
        return p

    rec = R.call_claude(img, pf, model, seed, work / "tmp")
    rec["answers"] = R.parse_answer(rec.get("result") or "", holder["letters"])
    rec.update({"code": code, "view": v, "look": lk, "theme": th, "size": sz, "persona": persona, "rep": rep,
                "model": model})
    path.write_text(json.dumps(rec, ensure_ascii=False))
    return rec


def run(a):
    work = Path(a.work)
    (work / "readers").mkdir(parents=True, exist_ok=True)
    jobs = [(work, a.model, v, lk, th, sz, img, p, r) for (v, lk, th, sz, img) in stimuli(work)
            for p in PERSONAS for r in range(a.reps)]
    if a.limit:
        jobs = jobs[: a.limit]
    cost = 0.0
    with ThreadPoolExecutor(a.par) as ex:
        for i, rec in enumerate(ex.map(one, jobs), 1):
            cost += rec.get("cost_usd") or 0.0
            if i % 25 == 0:
                print(i, len(jobs), round(cost, 3), flush=True)
    print("done", len(jobs), round(cost, 3))


def fmt(x, d=3):
    return f"{x:.{d}f}".replace(".", ",")


def score(a):
    work = Path(a.work)
    recs = [json.loads(p.read_text()) for p in sorted((work / "readers").glob("*.json"))]
    out = []
    P = out.append
    n_calls = len(recs)
    cost = sum(r.get("cost_usd") or 0 for r in recs)
    failed = [r for r in recs if r.get("exit") != 0 or "hit your" in (r.get("result") or "")]
    recs = [r for r in recs if r not in failed]
    off = [r for r in recs if not r.get("one_look")]
    P(f"Kayıt: {n_calls} (başarısız çağrı, kullanım sınırı vb.: {len(failed)}; tek bakış dışı, puanlamadan "
      f"çıkarılan: {len(off)}); maliyet {fmt(cost, 2)} $.\n")
    recs = [r for r in recs if r.get("one_look")]
    P("Kapsam (puanlanan çağrı / planlanan; haiku 5 tekrar × 2 persona × 2 tema × 2 boy = 40, sonnet 3 tekrar = 24):\n")
    P("| görünüm | " + " | ".join(f"{m} `{lk}`" for m in ("haiku", "sonnet") for lk in LOOKS) + " |")
    P("|---|---:|---:|---:|---:|")
    for v in SETS:
        row = []
        for m, plan in (("haiku", 40), ("sonnet", 24)):
            for lk in LOOKS:
                row.append(f"{sum(1 for r in recs if r['model'] == m and r['view'] == v and r['look'] == lk)}/{plan}")
        P(f"| {v} | " + " | ".join(row) + " |")
    P("")

    def cells(rs):
        k = n = 0
        unk = 0
        for r in rs:
            for qid, ans in r["answers"].items():
                n += 1
                k += ans["code"] == Q[qid][2]
                unk += ans["code"] in (UNK, None)
        return k, n, unk

    def line(label, filt):
        row = [label]
        res = {}
        for lk in LOOKS:
            k, n, unk = cells([r for r in recs if r["look"] == lk and filt(r)])
            if n == 0:
                row.append("—")
                continue
            p, lo, hi = S.wilson(k, n)
            res[lk] = (lo, hi)
            row.append(f"{fmt(p)} [{fmt(lo)}, {fmt(hi)}] (n {n}, bilmiyorum {unk})")
        if len(res) == 2:
            (a1, b1), (a2, b2) = res["oltan"], res["r4"]
            row.append("ayırt edilemedi" if a1 <= b2 and a2 <= b1 else ("`oltan`" if a1 > b2 else "`r4`"))
        else:
            row.append("—")
        P("| " + " | ".join(row) + " |")

    for model in sorted({r["model"] for r in recs}):
        P(f"#### Okuyucu {model}\n")
        P("| kesim | `oltan` doğruluk [Wilson %95] | `r4` doğruluk [Wilson %95] | sonuç |")
        P("|---|---|---|---|")
        m = lambda f: (lambda r: r["model"] == model and f(r))  # noqa: E731
        plan = 40 if model == "haiku" else 24
        full = [v for v in SETS if all(sum(1 for r in recs if r["model"] == model and r["view"] == v and r["look"] == lk)
                                       >= 0.9 * plan for lk in LOOKS)]
        line(f"**eşleşmiş: {', '.join(full)}** (iki görünüşte de kapsam ≥ %90)", m(lambda r: r["view"] in full))
        line("hepsi (kapsam eşit değil; karşılaştırma için değil)", m(lambda r: True))
        for sz in SIZES:
            line(f"boy {sz}", m(lambda r, sz=sz: r["size"] == sz))
        for th in THEMES:
            line(f"tema {th}", m(lambda r, th=th: r["theme"] == th))
        for p in PERSONAS:
            line(f"persona {p}", m(lambda r, p=p: r["persona"] == p))
        for sz in SIZES:
            for th in THEMES:
                line(f"{sz} × {th}", m(lambda r, sz=sz, th=th: r["size"] == sz and r["theme"] == th))
        for v in SETS:
            line(f"görünüm {v}", m(lambda r, v=v: r["view"] == v))
        P("")
        P(f"Soru başına doğruluk ({model}; `oltan` | `r4`; phone / desk):\n")
        P("| görünüm | soru | doğru | `oltan` phone | `oltan` desk | `r4` phone | `r4` desk | en sık yanlış (hepsi) |")
        P("|---|---|---|---:|---:|---:|---:|---|")
        for v, qs in SETS.items():
            for qid in qs:
                row = [v, Q[qid][0], dict(Q[qid][1])[Q[qid][2]]]
                wrong = defaultdict(int)
                for lk in LOOKS:
                    for sz in SIZES:
                        rs = [r for r in recs if r["model"] == model and r["view"] == v and r["look"] == lk and r["size"] == sz]
                        k = sum(r["answers"][qid]["code"] == Q[qid][2] for r in rs)
                        for r in rs:
                            c = r["answers"][qid]["code"]
                            if c != Q[qid][2]:
                                wrong[f"{lk}:{dict(Q[qid][1]).get(c, 'bilmiyorum/yok')}"] += 1
                        row.append(f"{k}/{len(rs)}")
                top = sorted(wrong.items(), key=lambda x: -x[1])[:3]
                row.append("; ".join(f"{w} ×{n}" for w, n in top) or "—")
                P("| " + " | ".join(row) + " |")
        P("")
    print("\n".join(out))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", choices=["run", "score", "prompt"])
    ap.add_argument("--work", required=True)
    ap.add_argument("--model", default="haiku")
    ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("--par", type=int, default=8)
    ap.add_argument("--limit", type=int, default=0)
    a = ap.parse_args()
    if a.cmd == "run":
        run(a)
    elif a.cmd == "score":
        score(a)
    else:
        p, letters = build_prompt("v2", "F", "abc.png", 1)
        print(p)
        print(letters, R.check_prompt(p, EXTRA_FORBIDDEN))


if __name__ == "__main__":
    main()
