#!/usr/bin/env python3
"""R4/E addition (not in R3): second reading that also credits the R4 `input` gesture `open`.

R4/T4 added `input {gesture: {kind: open, capsule, to}}` (PROTOCOL §4.6.1): opening a capsule
to a depth returns `interaction.opening` with the presenter's output at that depth
(§10.2) and, at `evidence`, the same per-fact evidence access as the `capsule` query. The R3
scorer (score.py, unchanged; the strict reading) only knows the `capsule`/`present` queries.
Here a failing R3 check for C02, C09, C10, C11 is re-tried with the equivalent `open` reply:

  C02  open on the open THYAO breakout, to = evidence, an evidence entry with values
  C09  open on the open THYAO extreme, to = capsule (capsule depth)
  C10  open on the open THYAO breakout, to = chart; answer consistent with the drawn view
       (fact-origin `rule` layer), as in R3
  C11  an `open` as `viewer` counts as the query; the answer must still name `flow/…`

usage: score_r4.py <label> [...]   writes raw/<label>/score_r4.json, prints the summary rows
"""

import glob
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import score  # noqa: E402

BASE = score.check


def openings(c, who=None):
    out = []
    for conn, m, rep in c["pairs"]:
        if m.get("type") != "input" or (m.get("gesture") or {}).get("kind") != "open":
            continue
        if rep.get("type") in (None, "error"):
            continue
        o = ((rep.get("data") or {}).get("interaction") or {}).get("opening") or {}
        if o:
            out.append((conn, o))
    return out


def alt(task, c, T, ans):
    if task == "C02":
        b = T["brk"]
        return any(o.get("capsule") == b["id"] and o.get("to") == "evidence"
                   and any(e.get("values") for e in o.get("evidence") or []) for _, o in openings(c))
    if task == "C09":
        e = T["ext"]
        return any(o.get("capsule") == e["id"] and o.get("to") == "capsule" for _, o in openings(c))
    if task == "C10":
        b = T["brk"]
        r = [o for _, o in openings(c) if o.get("capsule") == b["id"] and o.get("to") == "chart"]
        if not r:
            return False
        lvl = False
        for o in r:
            v = (o.get("presentation") or {}).get("view") or {}
            for l in v.get("layers", []):
                if l.get("mark") == "rule" and any((x or {}).get("kind") == "fact" for x in (l.get("origin") or {}).values()):
                    lvl = True
        yes = score.has_any(ans, ["evet", "çiziliyor", "ciziliyor", "çizili", "yes", "var"]) and \
            not score.has_any(ans, ["çizilmiyor", "cizilmiyor", "hayır"])
        return bool(ans) and yes == lvl
    if task == "C11":
        princ = set(c["hellos"].values())
        return princ == {"viewer"} and bool(openings(c)) and "flow/" in ans
    return False


def check_r4(task, c, T, ans, n_bars):
    why = BASE(task, c, T, ans, n_bars)
    if why and task in ("C02", "C09", "C10", "C11") and alt(task, c, T, ans):
        return []
    return why


def main(argv):
    score.check = check_r4
    for lb in argv or ["r1", "r2"]:
        ldir = os.path.join(score.RAW, lb)
        rows = []
        for adir in sorted(glob.glob(os.path.join(ldir, "agent?"))):
            r, _, _ = score.score_agent(adir)
            rows += r
        rows.sort(key=lambda r: r["task"])
        s = {k: sum(r[k] for r in rows) for k in ("first", "final", "final_lenient", "first_idfree", "final_idfree")}
        json.dump({"summary": s, "rows": [{k: r[k] for k in ("task", "agent", "attempts", "first", "final",
                                                              "final_lenient", "first_idfree", "final_idfree")}
                                          for r in rows]},
                  open(os.path.join(ldir, "score_r4.json"), "w"), indent=1, sort_keys=True)
        print(lb, json.dumps(s), " ".join(f"{r['task']}:{score.yn(r['first'])}{score.yn(r['final'])}" for r in rows))
    score.check = BASE


if __name__ == "__main__":
    main(sys.argv[1:])
