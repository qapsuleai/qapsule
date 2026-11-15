"""Compacts a `harness run --out` JSON into what M7 commits: per scenario the summary and,
per seed, the error/miss counters and the display-log hash (the full JSON stays local).

usage: python3 -I compact.py <in.json> <out.json>
"""
import json
import sys

KEEP = ["seed", "m1_early", "m2_contradictory", "m3_wrong_final", "m3_wrong_provisional",
        "m5_eligible", "m5_missed", "m8_blocked", "m13_dispute_ms", "m10_work",
        "m10_replica_events", "log_lines", "log_hash"]


def main(src, dst):
    d = json.load(open(src))
    out = {"approach": d["approach"], "results": []}
    for r in d["results"]:
        s = dict(r["summary"])
        s.pop("wall_secs", None)
        s.pop("m10_wall_ns_per_event", None)
        out["results"].append({
            "scenario": s["scenario"],
            "summary": s,
            "runs": [{k: run[k] for k in KEEP if k in run} for run in r["runs"]],
        })
    json.dump(out, open(dst, "w"), sort_keys=True, separators=(",", ":"))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
