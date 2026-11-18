"""Denetçi (R5/A madde 3): M16 birincil ölçüsünü (kısma kopyası, F2:H1, t_valid sınıf dengeli)
kayıtlı ham veriden, düzeneğin kodunu kullanmadan yeniden hesaplar. Yeni okuyucu çağrısı yok.

Anahtar SPEC metninden türetildi (§G4, §G6, Ek G3), düzeneğin keys.py/select.py'si okunmadı:
  t_first: işaret (mark) 1, cümle (sentence) 0,5, öteki 0
  t_dir:   hedefin yönü (up/down; neutral -> flat)
  t_valid: validity valid -> live; no_longer / closed -> ended; sınıf dengeli ağırlık
Birim = (uyaran, persona, tekrar, model); yalnız haiku, yalnız one_look == true.
Birim puanı = ağırlıklı ortalama; tasarım puanı = (2*ort(F) + ort(H)) / 3.
Önyükleme: birimler F ve H içinde ayrı yeniden örneklenir (2000; tohum sabit).
"""
import gzip
import json
import random
import sys
from collections import defaultdict

run = sys.argv[1]
key = json.load(open(f"{run}/key.json"))
states = {s["state"]: s for s in json.load(open(f"{run}/states.json"))["states"]}


def target(st):
    t = st.get("target") or (st.get("candidates") or [None])[0]
    return st["items"][t] if t else None


def truth(state):
    it = target(states[state])
    d = {"up": "up", "down": "down", "neutral": "flat"}[it["direction"]]
    v = "live" if it["validity"] in ("valid", "unknown") else "ended"
    return d, v


rows = [json.loads(l) for l in gzip.open(f"{run}/readers.jsonl.gz", "rt") if l.strip()]
rows = [r for r in rows if r["qset"] == "throttle" and r["log"].get("one_look")
        and (r["log"].get("models") or ["?"])[0].startswith("claude-haiku")]
# t_valid class weights over all rows of the question (same state set per design)
cls = defaultdict(int)
for r in rows:
    code = r["stim"].split(".")[0]
    cls[truth(key[code]["state"])[1]] += 1
tot = sum(cls.values())
w = {c: tot / (len(cls) * n) for c, n in cls.items()}

units = defaultdict(lambda: defaultdict(list))  # design -> persona -> [score]
for r in rows:
    code = r["stim"].split(".")[0]
    meta = key[code]
    d, v = truth(meta["state"])
    a = r["answer"]
    get = lambda q: (a.get(q) or {}).get("code")
    s_first = {"mark": 1.0, "sentence": 0.5}.get(get("t_first"), 0.0)
    s_dir = 1.0 if get("t_dir") == d else 0.0
    s_val = 1.0 if get("t_valid") == v else 0.0
    wv = w[v]
    units[meta["design"]][r["persona"]].append((s_first + s_dir + wv * s_val) / (2 + wv))


def score(f, h):
    return (2 * sum(f) / len(f) + sum(h) / len(h)) / 3


rng = random.Random(20261118)
print(f"t_valid classes {dict(cls)} weights {w}")
for design in sorted(units, key=lambda d: -score(units[d]["F"], units[d]["H"])):
    f, h = units[design]["F"], units[design]["H"]
    bs = []
    for _ in range(2000):
        bs.append(score([rng.choice(f) for _ in f], [rng.choice(h) for _ in h]))
    bs.sort()
    print(f"{design:8s} n=F{len(f)}/H{len(h)} primary={score(f, h):.3f} "
          f"[{bs[49]:.3f}, {bs[1949]:.3f}]")
