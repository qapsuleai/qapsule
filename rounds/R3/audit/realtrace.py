"""VERIFY 17: depoda gerçek veri izi var mı (yalnız toplu sayı yazar; değer yazmaz).

Gerçek barlar `<yerel>/bars/*.json` (depo dışında, `fixtures/local/`). Depodaki her izlenen
metin dosyasındaki sayı dizgileri, gerçek serilerin
  (a) ardışık >= 5 kapanışıyla aynı sırada eşleşen dizileri ("dizi izi"),
  (b) ayırt edici hacim değerleriyle (>= 1e6, tam sayı olmayan ya da 7+ hane) tekil eşleşmeleri
ile karşılaştırılır. Çıktı: dosya başına yalnız sayılar; hiçbir değer yazılmaz.
Kullanım: python3 -I realtrace.py <depo kökü> <fixtures/local yolu>
"""
import json
import os
import re
import subprocess
import sys

repo, local = sys.argv[1], sys.argv[2]
bars_dir = os.path.join(local, "bars")
series = {}
for fn in sorted(os.listdir(bars_dir)):
    if not fn.endswith(".json") or fn.endswith(".meta.json"):
        continue
    d = json.load(open(os.path.join(bars_dir, fn)))
    bars = d["data"]["bars"]
    series[fn] = bars

def sig(x):
    t = repr(float(x)).replace("-", "").replace(".", "").lstrip("0")
    return len(t.rstrip("0")) if "e" not in t else 0


# Only distinctive closes (>= 4 significant digits, not an integer) can start or continue a
# run; a run counts when it reaches 5 values with >= 5 significant digits or 8 with >= 4.
close_idx = {}  # value -> list of (series, index)
vols = set()
for name, bars in series.items():
    for i, b in enumerate(bars):
        if b["c"] != int(b["c"]) and sig(b["c"]) >= 4:
            close_idx.setdefault(b["c"], []).append((name, i))
        v = b["v"]
        if v >= 1e6 and (v != int(v) or len(str(int(v))) >= 7):
            vols.add(v)

num = re.compile(rb"-?\d+(?:\.\d+)?(?:[eE][-+]?\d+)?")
files = subprocess.run(["git", "-C", repo, "ls-files"], capture_output=True, check=True).stdout.decode().split("\n")
scanned = 0
hits_seq = {}
hits_vol = {}
for f in files:
    p = os.path.join(repo, f)
    if not f or not os.path.isfile(p) or os.path.getsize(p) > 20_000_000:
        continue
    data = open(p, "rb").read()
    if b"\0" in data[:4096]:
        continue
    scanned += 1
    toks = []
    for m in num.finditer(data):
        try:
            toks.append(float(m.group()))
        except ValueError:
            pass
    # (a) runs of >= 5 consecutive closes of one real series
    # (gaps of up to 8 other numbers between two closes are allowed: o, h, l, v, t…)
    best = 0
    run = {}  # (series, index) -> (run length, token position)
    for pos, x in enumerate(toks):
        for key in close_idx.get(x, ()):
            name, i = key
            prev = run.get((name, i - 1))
            strong = 1 if sig(x) >= 5 else 0
            if prev and pos - prev[1] <= 9:
                n, ns = prev[0] + 1, prev[2] + strong
            else:
                n, ns = 1, strong
            old = run.get(key)
            if old and old[0] > n:  # the same value again as o/h/l: keep the longer run
                n, ns = old[0], old[2]
            run[key] = (n, pos, ns)
            if ns >= 5 or n >= 8:
                best = max(best, n)
    if best >= 5:
        hits_seq[f] = best
    vh = sum(1 for x in toks if x in vols)
    if vh >= 3:
        hits_vol[f] = vh

print(f"gerçek seri: {len(series)}; taranan izlenen metin dosyası: {scanned}")
print(f"(a) ardışık gerçek kapanış dizisi (>= 5 tanesi >= 5 anlamlı haneli ya da >= 8 tanesi >= 4 haneli) içeren dosya: {len(hits_seq)}")
for f, b in sorted(hits_seq.items()):
    print(f"    {f}: en uzun dizi {b}")
print(f"(b) >= 3 ayırt edici gerçek hacim içeren dosya: {len(hits_vol)}")
for f, b in sorted(hits_vol.items()):
    print(f"    {f}: {b} eşleşme")
