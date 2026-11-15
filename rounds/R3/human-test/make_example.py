#!/usr/bin/env python3
"""Uydurma örnek cevaplar üretir (GERÇEK VERİ DEĞİL): puanlayıcıyı ve CSV biçimini göstermek için.

Kullanım:  python3 -I make_example.py key.json answers.example.csv
Tohumlu (random.Random(7)); 6 hayali katılımcı, kitapçık A/B dengeli, iki grup. Cevaplar
kâğıttaki kodlarla (DV, ↑, Z, AD, geçmişte, evet…) yazılır; puanlayıcı bunları tanır.
"""

import csv
import json
import random
import sys

CODES = {
    "kind": {"continuation": "DV", "reversal": "DN", "breakout": "KR", "squeeze": "SK",
             "extreme": "AŞ", "flow": "AK"},
    "direction": {"up": "↑", "down": "↓", "neutral": "—"},
    "strength": {"low": "Z", "high": "G"},
    "phase": {"candidate": "AD", "final": "KE", "realized": "GE", "failed": "GM", "retracted": "GÇ"},
}
SKILL = {"nolegend": 0.55, "legend": 0.85, "learn": 0.75}
ALT_BONUS = {"siluet": 0.03, "rozet": 0.0}
PEOPLE = [("P01", "trader", "A"), ("P02", "young", "B"), ("P03", "trader", "B"),
          ("P04", "young", "A"), ("P05", "trader", "A"), ("P06", "young", "B")]
HEADER = ["participant", "group", "booklet", "mode", "item", "kind", "direction", "strength",
          "phase", "confidence", "when", "bars_ago", "still_valid", "text"]
WHEN = {"past": "geçmişte", "now": "şu an", "conditional": "bir koşula bağlı"}
VALID = {"yes": "evet", "no": "hayır", "unknown": "bilinmiyor"}
PREFS = {"P01": "M", "P02": "N", "P03": "M", "P04": "fark yok", "P05": "M", "P06": "N"}
T2 = {"P01": "N-A1 ile N-A2 dönüş oklarını karıştırdım", "P02": "M-B1 akış mı sıkışma mı?",
      "P04": "gri olanların aşamasını ayıramadım"}


def main(key_path, out_path):
    with open(key_path, encoding="utf-8") as fh:
        key = json.load(fh)
    r = random.Random(7)
    rows = []
    for p, g, b in PEOPLE:
        base = 0.08 if g == "trader" else 0.0
        mode = "kâğıt" if p in ("P01", "P02", "P03") else "ekran"
        common = {"participant": p, "group": g, "booklet": b, "mode": mode}
        for sec, alt in enumerate(key["booklets"][b], start=1):
            for lab, t in key["alphabets"][alt]["items"].items():
                q = min(0.97, SKILL[t["condition"]] + base + ALT_BONUS[alt])
                row = dict(common, item=f"{sec}-{lab}")
                for f, m in CODES.items():
                    row[f] = m[t[f]] if r.random() < q else r.choice(list(m.values()))
                row["confidence"] = str(r.choice([2, 3, 4, 4, 5]) if r.random() < q else r.choice([1, 2, 3]))
                if r.random() < 0.02:
                    row["phase"] = ""
                rows.append(row)
        for s, t in key["sentences"].items():
            q = 0.85 + base
            row = dict(common, item=s)
            row["kind"] = CODES["kind"][t["kind"]] if r.random() < q else r.choice(list(CODES["kind"].values()))
            row["direction"] = CODES["direction"][t["direction"]] if r.random() < q else "—"
            row["when"] = WHEN[t["when"]] if r.random() < q else r.choice(list(WHEN.values()))
            if row["when"] == "geçmişte" and t["bars_ago"] is not None and not t["redacted"] and r.random() < 0.7:
                row["bars_ago"] = str(t["bars_ago"])
            row["still_valid"] = VALID[t["still_valid"]] if r.random() < q else r.choice(list(VALID.values()))
            rows.append(row)
        rows.append(dict(common, item="T1", text=PREFS[p]))
        if p in T2:
            rows.append(dict(common, item="T2", text=T2[p]))
        if p == "P06":
            rows.append(dict(common, item="T3", text="Renkler yardımcı oldu; küçük boyutta çizgiler kayboluyor."))
    with open(out_path, "w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(fh, fieldnames=HEADER)
        w.writeheader()
        for row in rows:
            w.writerow(row)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
