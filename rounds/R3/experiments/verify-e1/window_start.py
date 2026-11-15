#!/usr/bin/env python3
"""Denetçi sondası: ma_stack kararı geçmişin başladığı bara duyarlı mı?

recompute.py'nin motorla bit eşit 'engine' kipini kullanır (motorla 0 fark doğrulandı).
Seriyi 0'dan ve OFF'tan başlatıp, ikisinin de ısındığı barlardan sonra kenarları karşılaştırır.
Kullanım: python3 -I window_start.py <export.json> [OFF]
"""
import json
import os
import sys

here = os.path.dirname(os.path.abspath(__file__))
path = sys.argv[1]
off = int(sys.argv[2]) if len(sys.argv) > 2 else 137
sys.argv = [sys.argv[0], path, "engine"]
src = open(os.path.join(here, "recompute.py")).read().replace("\nmain()\n", "\n")
rc = {"__name__": "rc"}
exec(compile(src, "recompute.py", "exec"), rc)
data = json.load(open(path))
diff = tot = 0
for inst in data:
    b = inst["bars"]
    a = {(i, c) for (i, c, *_r) in rc["ma_stack"](b) if i >= off + 200}
    s = {(i + off, c) for (i, c, *_r) in rc["ma_stack"](b[off:]) if i + off >= off + 200}
    tot += len(a)
    diff += len(a ^ s)
print(f"offset {off}: ma_stack edges after warmup {tot}, differing (symmetric diff) {diff}")
