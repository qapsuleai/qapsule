"""M16 tables from a run's scores.json / computed.json (no calls). Usage:
    python3 tables.py <run dir> > tables.md
"""

import json
import sys
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "harness/glance"))
from gl import stats as S  # noqa: E402
from gl import select as SEL  # noqa: E402

run = Path(sys.argv[1])
sc = json.loads((run / "scores.json").read_text())
comp = json.loads((run / "computed.json").read_text())
rows, res = sc["rows"], sc["result"]
D = ["bertan", "kivra", "oltan", "zemre", "r4", "kestim"]
D = [d for d in D if d in res["per_design"]]
out = []
P = out.append


def f(t):
    return "—" if not t else f"{t[0]:.3f} [{t[1]:.3f}, {t[2]:.3f}]"


def wil(v):
    if not v:
        return "—"
    e, lo, hi = S.wilson(sum(v), len(v))
    return f"{e:.2f} [{lo:.2f}, {hi:.2f}] (n {len(v)})"


P("## Özet (F2:H1, %95 önyükleme)\n")
P("| tasarım | birincil: kısma | tam boy okuma | yarım boy | gri | akış | BT |")
P("|---|---|---|---|---|---|---|")
for d in D:
    p = res["per_design"][d]
    P(f"| `{d}` | {f(p['primary'])} | {f(p['reading'])} | {f(p['half'])} | {f(p['gray'])} | {f(p['feed'])} | {f(res['bt'].get(d))} |")

P("\n## Birincil, dil × tema (F2:H1)\n")
P("| tasarım | " + " | ".join(f"{l}/{t}" for l in ("tr", "en") for t in ("light", "dark")) + " |")
P("|---|---|---|---|---|")
for d in D:
    cells = []
    for l in ("tr", "en"):
        for t in ("light", "dark"):
            sub = [r for r in rows if r["lang"] == l and r["theme"] == t]
            cells.append(f(SEL.weighted_accuracy(sub, d, ("throttle",), 7)))
    P(f"| `{d}` | " + " | ".join(cells) + " |")

P("\n## Soru başına (haiku, F+H birlikte, Wilson %95)\n")
qs = [("throttle", "t_first"), ("throttle", "t_dir"), ("throttle", "t_valid"), ("face", "dir"), ("face", "kind"),
      ("face", "valid"), ("face", "when"), ("face", "trust"), ("face", "advice"), ("face_visitor", "numbers"),
      ("gray", "dir"), ("gray", "valid"), ("feed", "f_live"), ("feed", "f_down")]
P("| soru | " + " | ".join(f"`{d}`" for d in D) + " |")
P("|---|" + "---|" * len(D))
for qs_, q in qs:
    cells = []
    for d in D:
        v = [r["credit"] for r in rows if r["design"] == d and r["qset"] == qs_ and r["q"] == q and "haiku" in r["model"]]
        cells.append(wil(v))
    P(f"| {qs_}/{q} | " + " | ".join(cells) + " |")

P("\n## Gri kopya, durum başına (dir + valid ortalaması; Ek G5-3)\n")
states = sorted({r["state"] for r in rows if r["qset"] == "gray"})
P("| durum | " + " | ".join(f"`{d}`" for d in D) + " |")
P("|---|" + "---|" * len(D))
for s in states:
    cells = []
    for d in D:
        v = [r["credit"] for r in rows if r["design"] == d and r["qset"] == "gray" and r["state"] == s]
        cells.append(wil(v))
    P(f"| {s} | " + " | ".join(cells) + " |")
P("\nRenkli tam boy aynı sorular (`face` dir + valid), karşılaştırma için:\n")
P("| tasarım | renkli tam boy dir+valid | gri dir+valid |")
P("|---|---|---|")
for d in D:
    a = [r["credit"] for r in rows if r["design"] == d and r["qset"] in ("face", "face_visitor") and r["q"] in ("dir", "valid") and "haiku" in r["model"]]
    b = [r["credit"] for r in rows if r["design"] == d and r["qset"] == "gray"]
    P(f"| `{d}` | {wil(a)} | {wil(b)} |")

P("\n## Eşleştirilmiş fark (birincil, tasarım − bozuk)\n")
for d, v in (res.get("paired_vs_broken") or {}).items():
    P(f"- `{d}`: {v}")

P("\n## Çelişki (iki sıra)\n")
for k, v in sorted(res["conflicts"].items()):
    P(f"- `{k}`: {v['split']}/{v['total']} = {v['rate']}{' → ayırt edilemedi' if v['not_told_apart'] else ''}")
P(f"\nUzunluk yanlılığı: {res['length_bias']}")
P(f"\nModel uyumu (haiku–sonnet, tr/açık, ilk tekrar): {res.get('model_agreement')}")
P(f"\nTek bakış dışı (çıkarılan): {len(sc['result'].get('excluded_not_one_look', []))}")

cost = defaultdict(float)
n = defaultdict(int)
for r in rows:
    pass
for p in (run / "readers").glob("*.json"):
    j = json.loads(p.read_text())
    m = ",".join(j["log"]["models"])
    cost["read " + m] += j["log"]["cost_usd"] or 0
    n["read " + m] += 1
for p in (run / "critics").glob("*.json"):
    j = json.loads(p.read_text())
    cost["critic"] += j["log"]["cost_usd"] or 0
    n["critic"] += 1
P("\n## Maliyet\n")
for k in sorted(cost):
    P(f"- {k}: {n[k]} çağrı, {cost[k]:.2f} $")
P(f"- toplam: {sum(n.values())} çağrı, {sum(cost.values()):.2f} $")

P("\n## Seçim adımları\n")
for s in res["steps"]:
    P(f"- {s}")
P(f"\nKarar: {res['verdict']}")
print("\n".join(out))
