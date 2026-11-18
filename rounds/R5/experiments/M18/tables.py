#!/usr/bin/env python3
"""R5/M18: the tables of RESULTS.md from a glance run folder (scores.json, computed.json,
k52/*.json, critic and reader records). Deterministic; no model calls.

    python3 rounds/R5/experiments/M18/tables.py <run dir>  > tables.md
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

DESIGNS = ["korda", "selen", "pavka", "r4", "kestim"]
DEPTHS = ["chart5", "chart50", "chart200"]
QS = ["c_recent", "c_held", "c_live", "c_evid"]
SEED = 20261117


def j(p):
    return json.loads(Path(p).read_text())


def f3(t):
    return "—" if not t else f"{t[0]:.3f} [{t[1]:.3f}, {t[2]:.3f}]"


def main(run: Path):
    sc = j(run / "scores.json")
    rows, res = sc["rows"], sc["result"]
    comp = j(run / "computed.json")
    out = []
    P = out.append

    # --- deterministic gate -------------------------------------------------------------
    P("### Deterministik kapı (§G7; açık sahnenin boyama listesi + DOM)\n")
    P("| tasarım | metin karşıtlığı en kötü / eşik (öğe) | metin dışı en kötü (rol) | en küçük yazı px | dokunma < 44 (DOM) | renk ΔE00 en kötü çift (koşul) | sahne metni ort. karakter | geçti |")
    P("|---|---|---|---|---:|---|---:|---|")
    for d in DESIGNS:
        g = comp.get(d)
        if not g:
            continue
        t = g["text_contrast"]["worst"]
        m = g["nontext_contrast"]["worst_glyph"] or {}
        pr = min(g["colour"]["pairs"], key=lambda r: r["min"] - r["threshold"]) if g["colour"]["pairs"] else None
        cond = min(("normal", "gray", "protan", "deutan", "tritan"), key=lambda c: pr[c]) if pr else "—"
        P(f"| `{d}` | {t['ratio']} / {t['threshold']} (`{t['text']}`, {t['px']:g} px) | {m.get('ratio')} ({'/'.join(m.get('roles', []))}, {m.get('prim')}) | "
          f"{g['min_font']['worst']['px']:g} | {g['touch']['count']} | "
          f"{(pr['a'] + ' ↔ ' + pr['b'] + ': ' + str(pr['min']) + ' / ' + str(pr['threshold']) + ' (' + cond + ')') if pr else '—'} | "
          f"{g['density']['face_chars_mean']} | {g['pass']} |")
    P("")
    P(f"Uygulanmayan eşikler (hiçbir aday, `r4` dahil, geçemedi; §G9.1): {res.get('waived_gates')}\n")

    # --- K52 ----------------------------------------------------------------------------
    P("### K52 (k52chart.mjs; 3 tekrarın ortancası; ms)\n")
    P("| tasarım | kip | görünen hikâye (5/50/200) | tik p95 | sahne boyama p95 | sürüklemede kare p99 | sürüklemede uzun kare payı | PHONE_BIAS ile sahne p95 | geçti |")
    P("|---|---|---|---|---|---|---|---|---|")
    for d in DESIGNS:
        p = run / "k52" / f"{d}.json"
        if not p.exists():
            continue
        k = j(p)
        for c, v in k["configs"].items():
            bd = v["by_depth"]
            ds = [bd[x] for x in sorted(bd, key=int)]
            P(f"| `{d}` | `{c}` | {' / '.join(str(sorted(x['visible'])[len(x['visible']) // 2]) for x in ds)} | "
              f"{' / '.join(str(x['tick_p95']) for x in ds)} | {' / '.join(str(x['scene_p95']) for x in ds)} | "
              f"{' / '.join(str(x['drag_frame_p99']) for x in ds)} | {' / '.join(str(x['drag_long']) for x in ds)} | "
              f"{' / '.join(str(round(x['biased']['scene_p95'], 1)) for x in ds) if c == 'phone' else '—'} | {v['pass']} |")
    P("")
    if (run / "k52-rerun").exists():
        P("JIT kapalı yeniden koşu (`phone`, 5 tekrar, model çağrısı yokken; seçimde bu kullanılır):\n")
        P("| tasarım | derinlik | görünen (5 koşu) | sürüklemede uzun kare payı (ortanca; 5 koşu) | sahne boyama p95 (5 koşu) | kare p99 | geçti (≤ %10) |")
        P("|---|---|---|---|---|---|---|")
        for d in DESIGNS:
            p = run / "k52-rerun" / f"{d}.json"
            if not p.exists():
                continue
            bd = j(p)["configs"]["phone"]["by_depth"]
            for dep in sorted(bd, key=int):
                x = bd[dep]
                P(f"| `{d}` | {dep} | {x['visible']} | {x['drag_long']} ({x.get('drag_long_runs')}) | {x.get('scene_drag_p95_runs')} | {x['drag_frame_p99']} | {x['pass']} |")
        P("")

    # --- primary ------------------------------------------------------------------------
    P("### Birincil: `chart` kümesi doğruluğu (F 2 : H 1, birim = görüntü × persona × tekrar × model — seçim kuralının kendi girdisi: haiku 6 tekrar + sonnet alt kümesi; %95 önyükleme)\n")
    P("| tasarım | hepsi | 5 hikâye | 50 hikâye | 200 hikâye | F | H | birim (F/H) |")
    P("|---|---|---|---|---|---|---|---|")
    hk = [r for r in rows if r["qset"] == "chart" and "haiku" in r["model"]]
    allr = [r for r in rows if r["qset"] == "chart"]
    for i, d in enumerate(DESIGNS):
        pa = res["per_design"][d]["primary"]
        cells = []
        for dep in DEPTHS:
            sub = [r for r in allr if r["state"] == dep]
            cells.append(f3(SEL.weighted_accuracy(sub, d, ("chart",), SEED + 50 + i)))
        by = SEL._units(allr, d, ("chart",))
        fm = sum(by["F"]) / len(by["F"]) if by.get("F") else None
        hm = sum(by["H"]) / len(by["H"]) if by.get("H") else None
        P(f"| `{d}` | {f3(pa)} | {' | '.join(cells)} | {fm:.3f} | {hm:.3f} | {len(by.get('F', []))}/{len(by.get('H', []))} |")
    P("")
    P("Şans düzeyi (soru başına 1/k): c_recent 0,20 · c_held 0,25 · c_live 0,20 · c_evid 0,167 → birim ≈ 0,204.\n")

    # --- per question -------------------------------------------------------------------
    P("### Soru başına doğruluk (haiku, F+H birlikte; Wilson %95)\n")
    P("| tasarım | " + " | ".join(f"{q}" for q in QS) + " |")
    P("|---|" + "---|" * len(QS))
    for d in DESIGNS:
        cells = []
        for q in QS:
            v = [r["credit"] for r in hk if r["design"] == d and r["q"] == q]
            if not v:
                cells.append("—")
                continue
            e, lo, hi = S.wilson(sum(v), len(v))
            cells.append(f"{e:.2f} [{lo:.2f}, {hi:.2f}] n={len(v)}")
        P(f"| `{d}` | " + " | ".join(cells) + " |")
    P("")
    P("### Soru × derinlik (haiku; oran, n = 24 her hücre)\n")
    P("| tasarım | " + " | ".join(f"{q} {dep[5:]}" for q in QS for dep in DEPTHS) + " |")
    P("|---|" + "---|" * (len(QS) * len(DEPTHS)))
    for d in DESIGNS:
        cells = []
        for q in QS:
            for dep in DEPTHS:
                v = [r["credit"] for r in hk if r["design"] == d and r["q"] == q and r["state"] == dep]
                cells.append(f"{sum(v) / len(v):.2f}" if v else "—")
        P(f"| `{d}` | " + " | ".join(cells) + " |")
    P("")

    # --- "kapanmışlar görünürken canlı" -------------------------------------------------
    P("### Kapanmışlar görünürken canlının bulunması (`c_live`, haiku; Wilson %95)\n")
    P("| tasarım | 5 | 50 | 200 | cevap dağılımı (hepsi) |")
    P("|---|---|---|---|---|")
    for d in DESIGNS:
        cells = []
        for dep in DEPTHS:
            v = [r["credit"] for r in hk if r["design"] == d and r["q"] == "c_live" and r["state"] == dep]
            e, lo, hi = S.wilson(sum(v), len(v))
            cells.append(f"{e:.2f} [{lo:.2f}, {hi:.2f}]")
        dist = defaultdict(int)
        for r in hk:
            if r["design"] == d and r["q"] == "c_live":
                dist[r["code"]] += 1
        P(f"| `{d}` | " + " | ".join(cells) + f" | {dict(sorted(dist.items(), key=lambda kv: str(kv[0])))} |")
    P("")

    # --- sonnet subset / agreement -------------------------------------------------------
    sn = [r for r in rows if r["qset"] == "chart" and "sonnet" in r["model"]]
    if sn:
        P("### İkinci boy (sonnet; tr/açık, 1 tekrar) ve boylar arası uyum\n")
        P("| tasarım | sonnet doğruluk (F2:H1) | haiku aynı alt küme (tr/açık, tekrar 0) |")
        P("|---|---|---|")
        for i, d in enumerate(DESIGNS):
            hs = [r for r in hk if r["lang"] == "tr" and r["theme"] == "light" and r["rep"] == 0]
            P(f"| `{d}` | {f3(SEL.weighted_accuracy(sn, d, ('chart',), SEED + 80 + i))} | {f3(SEL.weighted_accuracy(hs, d, ('chart',), SEED + 90 + i))} |")
        P("")
        P(f"Uyum (aynı uyaran, persona, soru; ilk tekrar): {res.get('model_agreement')}\n")

    # --- critic -------------------------------------------------------------------------
    P("### Eleştirmen (sonnet; 10 çift × 3 derinlik × 2 sıra × 3 tekrar; tr/açık)\n")
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
    # critic verdicts raw
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

    # --- validity of the run ------------------------------------------------------------
    P(f"Tek bakış dışı (birden çok Read) çıkarılan: {len(res.get('excluded_not_one_look') or [])}; "
      f"kesilen cevap: { {d: res['per_design'][d]['cut_answers'] for d in DESIGNS if d in res['per_design']} }\n")
    bc = run / "broken-check.json"
    if bc.exists():
        P("### Bozuk tasarım sınaması (§G8)\n")
        P("```json\n" + json.dumps(j(bc), ensure_ascii=False, indent=1) + "\n```\n")
    P("### Seçim kuralı adımları (gl/select.py çıktısı)\n")
    for s in res["steps"]:
        P(f"- {json.dumps(s, ensure_ascii=False)}")
    P("")
    P(f"Karar: `{json.dumps(res.get('verdict'), ensure_ascii=False)}`\n")
    # cost
    cost_r = sum(r["log"]["cost_usd"] or 0 for r in (j(p) for p in (run / "readers").glob("*.json")))
    cost_c = sum(r["log"]["cost_usd"] or 0 for r in (j(p) for p in (run / "critics").glob("*.json")))
    nr = len(list((run / "readers").glob("*.json")))
    nc = len(list((run / "critics").glob("*.json")))
    P(f"Maliyet: okuyucu {nr} çağrı {cost_r:.2f} $, eleştirmen {nc} çağrı {cost_c:.2f} $, toplam {cost_r + cost_c:.2f} $.\n")
    print("\n".join(out))


if __name__ == "__main__":
    main(Path(sys.argv[1]))
