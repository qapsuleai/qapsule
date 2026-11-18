#!/usr/bin/env python3
"""R5/M20: the tables of RESULTS.md from a glance run folder (scores.json, computed.json,
k52/*.json, taps/*.json, reader and critic records). Deterministic; no model calls.

    python3 rounds/R5/experiments/M20/tables.py <run dir>  > tables.md
"""
from __future__ import annotations

import json
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "harness" / "glance"))
from gl import select as SEL  # noqa: E402
from gl import stats as S  # noqa: E402

DESIGNS = ["tesir", "nobra", "r4", "kestim"]
CAP = ("cap_evidence", "cap_evidence_visitor")
EVD = ("evidence", "evidence_visitor")
SEED = 20261117


def j(p):
    return json.loads(Path(p).read_text())


def f3(t):
    return "—" if not t else f"{t[0]:.3f} [{t[1]:.3f}, {t[2]:.3f}]"


def wil(v):
    if not v:
        return "—"
    e, lo, hi = S.wilson(sum(v), len(v))
    return f"{e:.2f} [{lo:.2f}, {hi:.2f}] n={len(v)}"


def main(run: Path):
    sc = j(run / "scores.json")
    rows, res = sc["rows"], sc["result"]
    comp = j(run / "computed.json")
    out = []
    P = out.append
    hk = [r for r in rows if "haiku" in r["model"]]

    # --- deterministic gate -------------------------------------------------------------
    P("### Deterministik kapı (§G7; açık sahnenin boyama listesi + DOM; kapsül ve kanıt derinliği)\n")
    P("| tasarım | metin karşıtlığı en kötü / eşik (öğe) | metin dışı en kötü (rol) | en küçük yazı px | dokunma < 44 (DOM) | renk: geçemeyen çift (açık/koyu) | yetkisizde rakam (Ek G7e) | sahne metni ort. karakter | ekran karakteri ort. |")
    P("|---|---|---|---|---:|---|---|---:|---:|")
    for d in DESIGNS:
        g = comp.get(d)
        if not g:
            continue
        t = g["text_contrast"]["worst"]
        m = g["nontext_contrast"]["worst_glyph"] or {}
        th = g["colour"]["themes"]
        bad = "/".join(str(sum(1 for r in th[x]["pairs"] if not r["pass"])) for x in ("light", "dark") if x in th)
        nv = g.get("numbers_visitor") or {}
        P(f"| `{d}` | {t['ratio']} / {t['threshold']} (`{t['text'][:34]}`, {t['px']:g} px) | {m.get('ratio')} ({'/'.join(m.get('roles', []))}, {m.get('prim')}) | "
          f"{g['min_font']['worst']['px']:g} | {g['touch']['count']} | {bad} | {nv.get('count')} ({nv.get('shots')} görüntü) | "
          f"{g['density']['face_chars_mean']} | {g['density']['screen_chars_mean']} |")
    P("")
    P(f"Uygulanmayan eşikler (hiçbir aday, `r4` dahil, geçemedi; §G9.1): {res.get('waived_gates')}\n")

    # --- K52 ----------------------------------------------------------------------------
    P("### K52 aynı ekranda: kapsül derinliği (k52open.mjs, Ek G7c; 3 tekrarın ortancası; ms)\n")
    P("| tasarım | kip | çerçeve | tik p95 | sahne boyama p95 (oynatma n / tekerlek n) | dururken uzun kare | tekerlek kipi | tekerlekte kare p99 | tekerlekte uzun kare payı | ilk kapsül | PHONE_BIAS ile tik / sahne | geçti |")
    P("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for d in DESIGNS:
        p = run / "k52" / f"{d}.json"
        if not p.exists():
            continue
        k = j(p)
        for c, v in k["configs"].items():
            P(f"| `{d}` | `{c}` | {v['frame'][0] if v['frame'] else '—'} | {v['tick_p95']} | {v['scene_p95']} ({v['scene_play_n']} / {v['scene_wheel_n']}) | "
              f"{v['idle_long']} | {sorted(set(m for m in v['wheel_mode'] if m)) or '—'} | {v['wheel_frame_p99']} | {v['wheel_long']} ({v['wheel_long_runs']}) | {v['first_capsule_ms']} | "
              f"{(str(round(v['biased']['tick_p95'], 1)) + ' / ' + str(round(v['biased']['scene_p95'], 1))) if c == 'phone' else '—'} | {v['pass']} |")
    P("")

    # --- primary ------------------------------------------------------------------------
    P("### Birincil: kapsül derinliğinde `evidence` + `evidence_visitor` (F 2 : H 1; birim = görüntü × persona × tekrar × model; %95 önyükleme)\n")
    P("| tasarım | hepsi | yetkili (`live_event`) | yetkisiz (`visitor`) | F | H | birim (F/H) |")
    P("|---|---|---|---|---|---|---|")
    capr = [r for r in rows if r["qset"] in CAP]
    for i, d in enumerate(DESIGNS):
        pa = res["per_design"][d]["primary"]
        a = SEL.weighted_accuracy([r for r in capr if r["state"] == "live_event"], d, CAP, SEED + 50 + i)
        b = SEL.weighted_accuracy([r for r in capr if r["state"] == "visitor"], d, CAP, SEED + 60 + i)
        by = SEL._units(capr, d, CAP)
        fm = sum(by["F"]) / len(by["F"]) if by.get("F") else 0
        hm = sum(by["H"]) / len(by["H"]) if by.get("H") else 0
        P(f"| `{d}` | {f3(pa)} | {f3(a)} | {f3(b)} | {fm:.3f} | {hm:.3f} | {len(by.get('F', []))}/{len(by.get('H', []))} |")
    P("")
    P("### İkincil: kanıt derinliğinde aynı kümeler (F 2 : H 1; %95)\n")
    P("| tasarım | hepsi | yetkili | yetkisiz |")
    P("|---|---|---|---|")
    evr = [r for r in rows if r["qset"] in EVD]
    for i, d in enumerate(DESIGNS):
        a = SEL.weighted_accuracy([r for r in evr if r["state"] == "live_event"], d, EVD, SEED + 70 + i)
        b = SEL.weighted_accuracy([r for r in evr if r["state"] == "visitor"], d, EVD, SEED + 75 + i)
        P(f"| `{d}` | {f3(res['per_design'][d]['reading'])} | {f3(a)} | {f3(b)} |")
    P("")
    P("Şans düzeyi (soru başına 1/k): e_rests 0,20 · e_who 0,333 · valid 0,20 · numbers 0,333.\n")

    # --- per question -------------------------------------------------------------------
    for title, sets in (("kapsül derinliği", CAP), ("kanıt derinliği", EVD)):
        P(f"### Soru başına doğruluk — {title} (haiku, F+H; Wilson %95)\n")
        P("| tasarım | `e_rests` neye dayanıyor (yetkili) | `e_who` kim doğruladı (yetkili; anahtar cihaz) | `e_who` (yetkisiz; anahtar sunucu) | `valid` | `numbers` neden rakam yok (yetkisiz) | `e_who` gevşek (sunucu ∪ cihaz; bilgi) |")
        P("|---|---|---|---|---|---|---|")
        for d in DESIGNS:
            sel = [r for r in hk if r["design"] == d and r["qset"] in sets]
            c = lambda q, st=None: [r["credit"] for r in sel if r["q"] == q and (st is None or r["state"] == st)]
            loose = [1.0 if r["code"] in ("server", "device") else 0.0 for r in sel if r["q"] == "e_who"]
            P(f"| `{d}` | {wil(c('e_rests'))} | {wil(c('e_who', 'live_event'))} | {wil(c('e_who', 'visitor'))} | {wil(c('valid'))} | {wil(c('numbers'))} | {wil(loose)} |")
        P("")
        P(f"Cevap dağılımı — {title} (haiku; kod: sayı)\n")
        P("| tasarım | e_who yetkili | e_who yetkisiz | numbers | e_rests |")
        P("|---|---|---|---|---|")
        for d in DESIGNS:
            def dist(q, st=None):
                x = defaultdict(int)
                for r in hk:
                    if r["design"] == d and r["qset"] in sets and r["q"] == q and (st is None or r["state"] == st):
                        x[r["code"] or "—"] += 1
                return dict(sorted(x.items(), key=lambda kv: -kv[1]))
            P(f"| `{d}` | {dist('e_who', 'live_event')} | {dist('e_who', 'visitor')} | {dist('numbers')} | {dist('e_rests')} |")
        P("")

    # --- throttle (info) ------------------------------------------------------------------
    P("### Kısma kopyası (kapsül derinliği, σ 4,5; bilgi; F 2 : H 1, %95)\n")
    P("| tasarım | `throttle` | t_first (işaret 1 / cümle 0,5) | t_dir | t_valid |")
    P("|---|---|---|---|---|")
    for i, d in enumerate(DESIGNS):
        th = SEL.weighted_accuracy(rows, d, ("throttle",), SEED + 200 + i)
        cells = []
        for q in ("t_first", "t_dir", "t_valid"):
            v = [r["credit"] for r in hk if r["design"] == d and r["qset"] == "throttle" and r["q"] == q]
            cells.append(f"{sum(v) / len(v):.2f} (n={len(v)})" if v else "—")
        P(f"| `{d}` | {f3(th)} | {' | '.join(cells)} |")
    P("")

    # --- sonnet subset ------------------------------------------------------------------
    sn = [r for r in rows if "sonnet" in r["model"]]
    if sn:
        P("### İkinci boy (sonnet; kapsül derinliği, tr/açık, 1 tekrar) ve boylar arası uyum\n")
        P("| tasarım | sonnet (F2:H1) | haiku aynı alt küme (tr/açık, tekrar 0) |")
        P("|---|---|---|")
        hs = [r for r in hk if r["lang"] == "tr" and r["theme"] == "light" and r["rep"] == 0]
        for i, d in enumerate(DESIGNS):
            P(f"| `{d}` | {f3(SEL.weighted_accuracy(sn, d, CAP, SEED + 80 + i))} | {f3(SEL.weighted_accuracy(hs, d, CAP, SEED + 90 + i))} |")
        P("")
        P(f"Uyum (aynı uyaran, persona, soru; ilk tekrar): {res.get('model_agreement')}\n")

    # --- taps ---------------------------------------------------------------------------
    P("### Dokunma sayısı (taps.mjs, Ek G7b; akıştan cevabın göründüğü ekrana en az dokunuş; tr/açık)\n")
    P("| tasarım | neye dayanıyor (yetkili) | kim doğruladı (yetkili) | neye dayanıyor (yetkisiz) | kim doğruladı (yetkisiz) | neden rakam göremiyorum (yetkisiz) | denenen dokunuş / görülen ekran |")
    P("|---|---|---|---|---|---|---|")
    for d in DESIGNS:
        p = run / "taps" / f"{d}.json"
        if not p.exists():
            continue
        t = j(p)["readers"]

        def cell(pr, q):
            b = (t.get(pr) or {}).get("best", {}).get(q)
            w = (t.get(pr) or {}).get("best_with_scroll", {}).get(q)
            if b:
                return f"{b['taps']} ({b['depth']})"
            if w:
                return f"{w['taps']} + kaydırma ({w['depth']})"
            return f"> {j(p)['max_taps']} (yok)"
        tried = "; ".join(f"{pr}: {v.get('taps_tried')} / {v.get('screens_seen')}" for pr, v in t.items())
        P(f"| `{d}` | {cell('arkin', 'e_rests')} | {cell('arkin', 'e_who')} | {cell('visitor', 'e_rests')} | {cell('visitor', 'e_who')} | {cell('visitor', 'numbers')} | {tried} |")
    P("")

    # --- critic -------------------------------------------------------------------------
    P("### Eleştirmen (sonnet; 6 çift × 2 durum (kapsül derinliği) × 2 sıra × 3 tekrar; tr/açık)\n")
    P("| tasarım | BT gücü [%95 önyükleme] |")
    P("|---|---|")
    for d in DESIGNS:
        b = res["bt"].get(d)
        P(f"| `{d}` | {f3(b) if b else '—'} |")
    P("")
    P("| çift | bölünme / toplam | çelişki oranı | ayırt edilemedi (> %30) |")
    P("|---|---|---|---|")
    for k, v in sorted(res["conflicts"].items()):
        P(f"| `{k}` | {v['split']}/{v['total']} | {v['rate']} | {'evet' if v['not_told_apart'] else 'hayır'} |")
    P("")
    lb = res.get("length_bias") or {}
    P(f"Uzunluk yanlılığı (U2): BT ~ ekran karakteri Spearman = {lb.get('spearman_bt_vs_chars')}; kesin sonuçlarda daha çok metinli olanın kazanma payı = {lb.get('decisive_won_by_more_text')}\n")
    jd = sc["judgements"]
    wins = defaultdict(lambda: [0, 0, 0])
    for x in jd:
        for d in (x["a"], x["b"]):
            if x["winner"] == d:
                wins[d][0] += 1
            elif x["winner"] is None:
                wins[d][1] += 1
            else:
                wins[d][2] += 1
    P("| tasarım | ham kararlar: kazandı / ayırt edemiyorum / kaybetti |")
    P("|---|---|")
    for d in DESIGNS:
        w = wins[d]
        P(f"| `{d}` | {w[0]} / {w[1]} / {w[2]} |")
    P("")
    P(f"Tek bakış dışı (birden çok Read) çıkarılan: {len(res.get('excluded_not_one_look') or [])}; "
      f"kesilen cevap: { {d: res['per_design'][d]['cut_answers'] for d in DESIGNS if d in res['per_design']} }\n")
    cost_r = sum(r["cost"] or 0 for r in {r["stim"] + r["persona"] + str(r["rep"]) + r["model"]: r for r in rows}.values())
    cost_c = sum(x["cost"] or 0 for x in jd)
    P(f"Maliyet (kayıtlardan): okuyucu {cost_r:.2f} $ ({len({r['stim'] + r['persona'] + str(r['rep']) + r['model'] for r in rows})} çağrı), eleştirmen {cost_c:.2f} $ ({len(jd)} çağrı).\n")
    print("\n".join(out))


if __name__ == "__main__":
    main(Path(sys.argv[1]))
