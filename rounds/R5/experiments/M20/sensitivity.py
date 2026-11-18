#!/usr/bin/env python3
"""R5/M20: information only (SPEC Ek G7d) — the selection rule's steps 2-5 with the loose
`e_who` key (server ∪ device: K48, an opened public-data capsule was verified by the server AND
checked on this device). Same rows, same bootstrap seeds; no model calls.

    python3 rounds/R5/experiments/M20/sensitivity.py <run dir>
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "harness" / "glance"))
from gl import select as SEL  # noqa: E402

SEED = 20261117
run = Path(sys.argv[1])
sc = json.loads((run / "scores.json").read_text())
rows = []
for r in sc["rows"]:
    r = dict(r)
    if r["q"] == "e_who":
        r["credit"] = 1.0 if r["code"] in ("server", "device") else 0.0
    rows.append(r)
comp = json.loads((run / "computed.json").read_text())
res = SEL.select(rows, sc["judgements"], comp, seed=SEED, broken="kestim", reference="r4",
                 primary_sets=("cap_evidence", "cap_evidence_visitor"), reading_sets=("evidence", "evidence_visitor"))
out = {"primary": {d: v["primary"] for d, v in res["per_design"].items()},
       "secondary_evidence_depth": {d: v["reading"] for d, v in res["per_design"].items()},
       "bt": res["bt"], "steps": res["steps"], "verdict": res.get("verdict")}
print(json.dumps(out, indent=1, ensure_ascii=False))
