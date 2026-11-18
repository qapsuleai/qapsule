#!/usr/bin/env python3
"""M21 tables (RESULTS.md) from the run summary: scores (main and sensitivity), loss, K52.
  python3 tables.py <run dir> > run/tables.md
"""
import json
import sys
from pathlib import Path

run = Path(sys.argv[1])
S = json.loads((run / "scores.json").read_text())["result"]
ST = json.loads((run / "scores-text-rule.json").read_text())["result"] if (run / "scores-text-rule.json").exists() else None
L = json.loads((run / "loss.json").read_text())
ORDER = ["surek", "nabiz", "r4", "still", "kestim", "surek-az", "nabiz-az"]


def f(t):
    if not t:
        return "—"
    return f"{t[0]:.3f} [{t[1]:.3f}, {t[2]:.3f}]"


pd = S["per_design"]
print("## Okuyucu ölçüleri (haiku, F2:H1, %95 önyükleme)\n")
print("| tasarım | süreklilik (birincil; geçerli sorular) | geçerli sorular | `motion` hepsi | `motion_final` | fark edilme (`notice`, mid+after) | yalnız mid | yalnız after | çift (önce|mid) |")
print("|---|---|---|---|---|---|---|---|---|")
for d in ORDER:
    if d not in pd:
        continue
    p = pd[d]
    print(f"| `{d}` | {f(p['primary'])} | {', '.join(p['valid_questions']) or 'yok'} | {f(p['motion_all'])} | {f(p['motion_final'])} | "
          f"{f(p['notice'])} | {f(p['notice_mid'])} | {f(p['notice_after'])} | {f(p['notice_pair'])} |")
print("\n## Geçerlilik sınaması (son kareyle; Wilson %95)\n")
print("| tasarım | soru | n | son kareyle doğruluk [aralık] | şans | tek yönlü geçerli | lafzî (aralık şansı içerir) |")
print("|---|---|---|---|---|---|---|")
for d in ORDER:
    if d not in pd:
        continue
    for q, v in pd[d]["validity"].items():
        if not v.get("n"):
            print(f"| `{d}` | `{q}` | 0 | anahtar yok (puanlanmadı) | | | |")
            continue
        print(f"| `{d}` | `{q}` | {v['n']} | {v['p']:.3f} [{v['lo']:.3f}, {v['hi']:.3f}] | {v['chance']} | {'evet' if v['valid'] else 'hayır'} | {'evet' if v['two_sided_within'] else 'hayır'} |")
print("\n## Payları\n")
print("| tasarım | ara karede \"aynı nesne taşınıyor\" | son karede \"aynı nesne taşınıyor\" | değişimde \"değişen yok\" deme | denetimde yanlış alarm |")
print("|---|---|---|---|---|")
for d in ORDER:
    if d in pd:
        p = pd[d]
        print(f"| `{d}` | {p['moving_share_mid']} | {p['moving_share_final']} | {p['none_share_change']} | {round(p['false_alarm_control'], 3)} |")
print("\n## Soru başına (Wilson)\n")
print("| tasarım | küme/soru/persona | p [aralık] | n |")
print("|---|---|---|---|")
for d in ORDER:
    if d in pd:
        for k, v in pd[d]["questions"].items():
            print(f"| `{d}` | {k} | {v['p']} [{v['lo']}, {v['hi']}] | {v['n']} |")
print("\n## Eleştirmen (sonnet, üç kare, tr/açık)\n")
print("| tasarım | BT [%95] |")
print("|---|---|")
for d, v in sorted(S["bt"].items(), key=lambda x: -x[1][0]):
    print(f"| `{d}` | {v[0]:.2f} [{v[1]:.2f}, {v[2]:.2f}] |")
print("\n| çift | bölünme / toplam | oran | ayırt edilemedi |")
print("|---|---|---|---|")
for k, v in sorted(S["conflicts"].items()):
    print(f"| {k} | {v['split']}/{v['total']} | {v['rate']} | {'evet' if v['not_told_apart'] else ''} |")
print("\n## Hareketi azalt bilgi kaybı\n")
print("| tasarım | azaltılmış | (a) durağan içerik kaybı | (b) metin kuralı (öğe) | (b) herhangi ilke (ana) | (b) çıplak dizgi (ilk uygulama) | hareketli yüz sayısı |")
print("|---|---|---|---|---|---|---|")
for d, v in L.items():
    print(f"| `{d}` | `{v['reduced']}` | {len(v['static_lost'])} | {len(v['carried_lost'])} | {len(v['carried_lost_any_prim'])} | {len(v['carried_lost_bare_string'])} | {len(v['animated'])} |")
print("\n## Kapılar ve seçim adımları (ana: `any_prim`)\n")
print("```json")
print(json.dumps({"gates": S["gates"], "waived": S["waived"], "steps": S["steps"], "verdict": S["verdict"], "chars": S["chars"]}, ensure_ascii=False, indent=1))
print("```")
if ST:
    print("\n## Duyarlılık (G6-5 lafzı: yalnız metin)\n```json")
    print(json.dumps({"gates": ST["gates"], "steps": ST["steps"], "verdict": ST["verdict"]}, ensure_ascii=False, indent=1))
    print("```")
print("\n## Bozuk tasarım\n```json")
print(json.dumps({"broken_check": S.get("broken_check"), "paired_vs_broken": S.get("paired_vs_broken")}, ensure_ascii=False, indent=1))
print("```")
print(f"\nBoylar arası uyum (haiku–sonnet, `motion` tr/açık, ilk tekrar): {S.get('model_agreement')}")
print(f"\nTek bakış dışı (birden çok Read) çıkarılan: {len(S['excluded_not_one_look'])}")
print(f"\nMaliyet (puanlanan kayıtlar): {S['cost']}")
