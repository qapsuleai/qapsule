#!/usr/bin/env python3
"""Uydurma örnek cevaplar üretir (GERÇEK VERİ DEĞİL): puanlayıcıyı, CSV biçimini ve karar kuralını
göstermek için.

Kullanım:  python3 -I make_example.py [key.json] [answers.example.csv]
Tohumlu (random.Random(11)); 6 hayali katılımcı (A01–A03 alışkın, G01–G03 genç). Cevaplar formdaki
numaralarla yazılır. Tasarım "yetenekleri" uydurmadır (aşağıda BONUS); karar kuralının her dalını
göstermek için seçildi, hiçbir şeyi ölçmez.
"""

import csv
import importlib.util
import json
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("score", os.path.join(HERE, "score.py"))
score = importlib.util.module_from_spec(spec)
spec.loader.exec_module(score)

PEOPLE = [("A01", "ekran"), ("A02", "ekran"), ("A03", "masaüstü"), ("G01", "ekran"), ("G02", "ekran"), ("G03", "ekran")]
SKILL = {"A": 0.80, "G": 0.65}
# Made-up advantages of side A over side B per comparison (+ favours A).
BONUS = {"alphabet": 0.0, "face": 0.25, "transition": 0.0, "outcome": 0.30, "feed": -0.25}
HEADER = ["participant", "group", "mode", "item", "q", "answer", "hint", "note", "quote"]


def pick(r, q, truth, p_ok):
    opts = score.OPTS[q]
    if r.random() < p_ok:
        return str(opts.index(truth) + 1)
    return str(r.randint(1, len(opts)))


def main(key_path, out_path):
    key = json.load(open(key_path, encoding="utf-8"))["items"]
    plan = json.load(open(os.path.join(HERE, "plan.json"), encoding="utf-8"))
    r = random.Random(11)
    rows = []
    for p, mode in PEOPLE:
        g = p[0]
        common = {"participant": p, "group": "alışkın" if g == "A" else "genç", "mode": mode}
        add = lambda item, q, answer, **kw: rows.append(dict(common, item=item, q=q, answer=answer, **kw))  # noqa: E731
        for task in plan["order"]:
            if task == "P0":
                continue
            comp = next(c for c, t in score.TASK.items() if t == task)
            first = score.first_side(p, comp)
            for item in sorted(k for k, v in key.items() if v["task"] == task):
                kv = key[item]
                side = first if kv["set"] == 1 else ("B" if first == "A" else "A")
                q_ok = min(0.97, max(0.05, SKILL[g] + (BONUS[comp] / 2 if side == "A" else -BONUS[comp] / 2)))
                if task == "G1":
                    for q in score.FACE_Q:
                        quote = "çarpı var, demek ki tutmamış" if (p, item, q) == ("A02", "G1-1a", "s3") else ""
                        add(item, q, pick(r, q, kv["key"][q], q_ok), quote=quote)
                    add(item, "sure", str(r.randint(25, 60)))
                elif task in ("G5", "G4"):
                    for j, row in enumerate(kv["rows"], start=1):
                        if task == "G5":
                            for q, kq in zip(score.GLYPH_Q, ["s1", "s1b", "s3"]):
                                add(f"{item}.{j}", q, pick(r, q, row["key"][kq], q_ok))
                        else:
                            add(f"{item}.{j}", "sonuc", pick(r, "sonuc", row["key"]["outcome"], q_ok))
                    add(item, "sure", str(r.randint(15, 45)))
                elif task == "G3":
                    add(item, "yer", "1" if r.random() < q_ok + 0.15 else "2")
                    add(item, "ayni", "1" if r.random() < q_ok + 0.1 else "3")
                    add(item, "s3", pick(r, "s3", kv["key"]["s3"], q_ok))
                    add(item, "kayip", str(r.randint(1, 3) if side == "A" else r.randint(1, 4)))
                    add(item, "sure", str(r.randint(3, 9) if side == "A" else r.randint(4, 12)))
                elif task == "G2":
                    k2 = kv["key"]
                    add(item, "g2a", "; ".join(k2["g1"]) if r.random() < q_ok else "yok")
                    add(item, "g2b", k2["g2_which"][0] if r.random() < q_ok else "yok")
                    add(item, "g2b_ne", "1" if r.random() < q_ok else "5")
                    hint = "evet" if (p == "G02" and item == "G2-1") else ""
                    c = k2["g3"][0]
                    add(item, "g2c", f"{c['instrument']} {c['ledger']}" if r.random() < q_ok else "SYN26 24/41", hint=hint,
                        note="kaydırmadan önce ipucu istedi" if hint else "")
                    for q in ("g2a", "g2b", "g2c"):
                        add(f"{item}.{q}", "sure", str(r.randint(5, 30)))
            # preference: first or second shown
            fav = "1" if (BONUS[comp] > 0) == (first == "A") else "2"
            add("T", task, fav if r.random() < 0.7 else r.choice(["1", "2", "fark yok"]))
        add("T", "serbest", "Kapanmış olanlar açık olanlardan daha çok göze batıyor." if p == "G03" else "")
    with open(out_path, "w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(fh, fieldnames=HEADER)
        w.writeheader()
        for row in rows:
            if row["q"] == "serbest" and not row["answer"]:
                continue
            w.writerow(row)


if __name__ == "__main__":
    a = sys.argv[1:]
    main(a[0] if a else os.path.join(HERE, "key.json"), a[1] if len(a) > 1 else os.path.join(HERE, "answers.example.csv"))
