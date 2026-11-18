"""Denetçi: NI denemesinin döktüğü yetkisiz satırlarında (c: hiç yetkisi yok) rakam içeren metin
alanlarının yolları (hangi mesaj, hangi alan) ve örnekleri."""
import json
import re
import sys
from collections import Counter

TEXT_KEYS = {"headline", "detail", "ledger", "face", "text", "sentence", "tr", "en", "event",
             "direction_word", "when", "moved", "validity_phrase", "ledger_short", "title", "term"}


def walk(v, path, acc):
    if isinstance(v, dict):
        for k, x in v.items():
            walk(x, path + [k], acc)
    elif isinstance(v, list):
        for x in v:
            walk(x, path + ["[]"], acc)
    elif isinstance(v, str) and path and path[-1] in TEXT_KEYS:
        t = v.replace("THYAO", "")
        if re.search(r"[0-9]", t):
            acc.append(("/".join(path), v))


for f in sys.argv[1:]:
    hits = []
    for line in open(f):
        m = json.loads(line)
        kind = m.get("type")
        acc = []
        walk(m, [kind or "?"], acc)
        hits += acc
    c = Counter(p for p, _ in hits)
    print(f"== {f}: {len(hits)} text fields with a digit")
    for p, n in c.most_common():
        ex = next(v for q, v in hits if q == p)
        print(f"   {n:4d}  {p}   e.g. {ex[:120]!r}")
