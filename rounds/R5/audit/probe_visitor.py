"""Denetçi: yetkisiz okur (principal=visitor) kaydının özeti: yüz sayısı, ilkel türleri, açılmış
kapsül metni ve DOM'daki akış metinleri (rakam taraması simge hariç)."""
import json
import re
import sys
from collections import Counter

d = json.load(open(sys.argv[1]))
for v in d["visitor"]:
    kinds = Counter(p["k"] for sc in v["faces"].values() for p in sc["prims"])
    print("step", v["step"], "faces", len(v["faces"]), dict(kinds), "open:", str(v["open"])[:200])
    feedish = [t for t in v["dom"] if t["onscreen"] and t["id"] not in ("clock", "c_step", "c_step_n")]
    dig = [t["t"] for t in feedish if re.search(r"\d", re.sub(r"SYN\d+", "", t["t"]))]
    print("   on-screen DOM texts", len(feedish), "with digit (excl. demo clock/step):", dig[:10])
    print("   sample:", [t["t"] for t in feedish][:14])
