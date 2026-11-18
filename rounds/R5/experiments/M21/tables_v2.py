#!/usr/bin/env python3
"""M21 re-measurement (SPEC Ek G8): summary tables and the committed run summary.

  tables_v2.py <run dir> <out dir (M21/run-v2)>

Reads the run's scores.json, loss.json, k52/, key files and reader/critic records; writes
<out>/tables.md and the summary files (no screenshots): designs, keys, results, loss, K52,
reader and critic records without images (jsonl.gz), shot geometry, prompts.
"""

from __future__ import annotations

import gzip
import json
import shutil
import sys
from pathlib import Path

run, out = Path(sys.argv[1]), Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)
J = lambda p: json.loads(Path(p).read_text())  # noqa: E731
sc = J(run / "scores.json")
res = sc["result"]
pd = res["per_design"]
BROKEN = "tirvan"
order = ["surek", "nabiz", "r4", "still", BROKEN]


def f3(x):
    return "—" if x is None else f"{x:.3f}"


def ci(t):
    return "—" if not t else f"{t[0]:.3f} [{t[1]:.3f}, {t[2]:.3f}]"


L = ["# M21 yeniden ölçüm — tablolar (üretildi: `tables_v2.py`)", ""]
L += ["## Geçerlilik (Ek G8-1): son kareyle `m_from`, şans 0,25, tek yönlü", "",
      "| tasarım | uyaran | n | doğruluk [Wilson %95] | geçerli (alt sınır ≤ 0,25) | iki yönlü (0,25 aralıkta) |", "|---|---|---|---|---|---|"]
for d in order:
    v = pd[d]["validity"]
    L.append(f"| `{d}` | {v.get('qset')} | {v.get('n')} | {f3(v.get('p'))} [{f3(v.get('lo'))}, {f3(v.get('hi'))}] | "
             f"{'evet' if v.get('valid') else 'HAYIR'} | {'evet' if v.get('two_sided_within') else 'hayır'} |")
L += ["", f"Soru bütün tasarımlar için geçerli (`still` sınaması, G8-1 ii): **{'evet' if res['question_valid_globally'] else 'HAYIR'}**", ""]
L += ["## Süreklilik `m_from` (birincil; F2:H1, %95 önyükleme)", "",
      "| tasarım | birincil | `motion2` hepsi | şerit 1 / 2 / 3 / 4 (ham pay) | \"olduğu yerde beliriyor\" payı | son kare (`motion2_final`) |", "|---|---|---|---|---|---|"]
for d in order:
    p = pd[d]
    b = p["from_by_band"]
    L.append(f"| `{d}` | {ci(p['primary'])} | {ci(p['motion2_all'])} | {' / '.join(f3(b[k]) for k in '1234')} | {f3(p['from_none_share'])} | {ci(p['motion2_final'])} |")
L += ["", "## Fark edilme (Ek G8-3): isabet − yanlış alarm (F2:H1, %95 önyükleme)", "",
      "| tasarım | isabet − YA (mid + after) | yalnız mid | yalnız after | isabet F / H | YA F / H | isabet: hedef canlı / kapanmış | YA: canlılı / canlısız denetim |",
      "|---|---|---|---|---|---|---|---|"]
for d in order:
    p = pd[d]
    n = p["notice"]
    hit = n[3]["hit"] if n else {}
    fa = n[3]["fa"] if n else {}
    L.append(f"| `{d}` | {ci(n)} | {ci(p['notice_mid'])} | {ci(p['notice_after'])} | {f3(hit.get('F'))} / {f3(hit.get('H'))} | "
             f"{f3(fa.get('F'))} / {f3(fa.get('H'))} | {f3(p['hit_live'])} / {f3(p['hit_closed'])} | {f3(p['fa_live'])} / {f3(p['fa_nolive'])} |")
states = list(next(iter(pd.values()))["per_state"].keys())
L += ["", "Durum başına doğru cevap payı (değişimde doğru şerit, denetimde \"hayır\"; mid + after):", "",
      "| durum | " + " | ".join(f"`{d}`" for d in order) + " |", "|---|" + "---|" * len(order)]
for s in states:
    L.append(f"| `{s}` | " + " | ".join(f3(pd[d]["per_state"].get(s)) for d in order) + " |")
L += ["", "## Eleştirmen BT (%95)", "", "| tasarım | BT |", "|---|---|"]
for d, v in sorted(res["bt"].items(), key=lambda x: -x[1][0]):
    L.append(f"| `{d}` | {v[0]:.2f} [{v[1]:.2f}, {v[2]:.2f}] |")
L += ["", "Çelişki oranları: " + "; ".join(f"{k} {v['split']}/{v['total']}" + (" (ayırt edilemedi)" if v["not_told_apart"] else "")
                                         for k, v in sorted(res["conflicts"].items())), ""]
L += ["## Bozuk hareketin yeri (Ek G8-2)", "", "```", json.dumps(res["broken_check"], indent=1, ensure_ascii=False), "```", ""]
L += ["## Kapılar", "", "```", json.dumps({"gates": res["gates"], "waived": res["waived"]}, indent=1), "```", ""]
loss = J(run / "loss.json") if (run / "loss.json").exists() else {}
L += ["## Bilgi kaybı (Ek G8-6)", "", "| tasarım → azaltılmış | (a) durağan | (b) metin kuralı | (b) herhangi bir ilke (duyarlılık) | (b) çıplak dizgi | hareketli yüz |", "|---|---|---|---|---|---|"]
for d, v in loss.items():
    L.append(f"| `{d}` → `{v['reduced']}` | {len(v['static_lost'])} | {len(v['carried_lost'])} | {len(v['carried_lost_any_prim'])} | "
             f"{len(v['carried_lost_bare_string'])} | {len(v['animated'])} |")
L += ["", "## Seçim adımları", "", "```", json.dumps(res["steps"], indent=1, ensure_ascii=False), "```", "",
      "## Karar", "", "```", json.dumps(res["verdict"], indent=1, ensure_ascii=False), "```", "",
      f"Boylar arası uyum: {json.dumps(res.get('model_agreement'), ensure_ascii=False)}", "",
      f"Maliyet: {json.dumps(res['cost'])}; tek bakış dışı okuyucu birimi {len(res['excluded_not_one_look'])}, eleştirmen {res['critic_excluded_not_one_look']}", ""]
(out / "tables.md").write_text("\n".join(L))

# summary files
for name in ("designs.json", "key.json", "critic_key.json", "loss.json", "limited.json"):
    if (run / name).exists():
        shutil.copy(run / name, out / name)
(out / "scores-result.json").write_text(json.dumps(res, indent=1, ensure_ascii=False) + "\n")
if (run / "k52").exists():
    shutil.copytree(run / "k52", out / "k52", dirs_exist_ok=True)
for sub, fn in (("readers", "readers.jsonl.gz"), ("critics", "critics.jsonl.gz")):
    with gzip.open(out / fn, "wt") as g:
        for p in sorted((run / sub).glob("*.json")):
            r = J(p)
            g.write(json.dumps(r, ensure_ascii=False) + "\n")
geo = {}
key = J(run / "key.json")
for c, k in key.items():
    p = run / "shots" / f"{c}.json"
    if not p.exists():
        continue
    r = J(p)
    if k["kind"] == "motion2":
        geo[c] = {"target": r.get("target"), "target_rect_before": r.get("target_rect_before"), "top_bar_bottom": r.get("top_bar_bottom"),
                  "frames": [{"p": f["p"], "elements": f["elements"], "boxes": f["boxes"]} for f in r.get("frames", [])]}
    else:
        geo[c] = {"status": [r.get("status_before"), r.get("status_after")], "t_change": r.get("t_change"),
                  "mid": {kk: r["frames"]["mid"]["overlay"].get(kk) for kk in ("changed", "face_changes", "mk", "fx", "face_opacity")}}
with gzip.open(out / "shots-geometry.json.gz", "wt") as g:
    json.dump(geo, g)
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "harness" / "glance"))
from gl import readers as R  # noqa: E402

P = ["# Örnek istemler (birebir; seçenek sırası her çağrıda tohumla karışır)", ""]
for qs, note in (("motion2", R.MOTION2_NOTE), ("motion2_final", R.MOTION2_NOTE), ("notice", R.NOTICE_NOTE)):
    p, _ = R.build_prompt(qs, "tr", "F", "abcdefgh.png", 1, note["tr"])
    P += [f"## {qs}", "", "```", p, "```", ""]
P += ["## eleştirmen (CRITIC_M21)", "", "```", R.CRITIC_M21["tr"].format(file="abcdefgh.png"), "```", ""]
(out / "prompts.md").write_text("\n".join(P))
print("wrote", out)
