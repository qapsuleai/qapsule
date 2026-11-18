"""Denetçi: dump.json'da bir düzenli ifadeye uyan yüzeyleri iki dilde yan yana gösterir."""
import json
import re
import sys
from collections import Counter

d = json.load(open(sys.argv[1]))
rx = re.compile(sys.argv[2], re.I)
lim = int(sys.argv[3]) if len(sys.argv) > 3 else 5
n = 0
cells = Counter()
for row in d["rows"]:
    for surf in ("headline", "detail", "ledger"):
        if rx.search(row["en"][surf]) or rx.search(row["tr"][surf]):
            n += 1
            cells[row["cell"].split("|")[1] + "|" + row["cell"].split("|")[2]] += 1
            if n <= lim:
                print(row["id"], row["cell"], surf)
                print("   TR:", row["tr"][surf])
                print("   EN:", row["en"][surf])
print("matches:", n, dict(cells))
