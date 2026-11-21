#!/usr/bin/env python3
"""R5B/H: writes the harness session inits (synthetic; deterministic; no network).

  session/init.json      - runs on today's `qp` (no R5B fields): gen_answers.py uses it.
  session/init.r5b.json  - the same session plus the R5B contract fields (PLAN.md §1-§3):
                           `on_behalf` on the agent principals, `agent_policy`,
                           `comment_strictness`. `qp mcp serve` runs this one once S/M land.
  session/position.json  - the fixed replay position and the M31 window.

Both inits describe the same market, entitlements and audiences; each agent principal
reads exactly what the person it acts for reads (claude = arkin, claude-guest = guest),
so the answers produced through init.json hold for init.r5b.json (SPEC.md §2).

usage: python3 make_init.py      (writes the three files next to this script)
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))

N = 12
SYMS = [f"SYN{i:02d}" for i in range(1, N + 1)]
# Licensed flow (synthetic test host, init.capsules.flow): two instruments.
FLOW = [("syn03", 7), ("syn07", 11)]
# The guest reads the bars of SYN01..SYN09 only and no licensed flow.
GUEST = [f"bist/{s}" for s in SYMS[:9]]

# The fixed position: the replay starts paused after HISTORY daily bars, no seek needed
# (SPEC.md §2.2: the same answers as history 140 + `seek {step: 20}`, checked). M31 steps
# M31_STEPS days from there with the host's `replay step`.
HISTORY = 160
POSITION = None
M31_STEPS = 10


def principals(r5b):
    out = [
        {"id": "host", "author": {"kind": "engine", "name": "host", "version": "0"}, "role": "host"},
        {"id": "arkin", "author": {"kind": "human", "name": "arkin"}, "role": "owner"},
        {"id": "guest", "author": {"kind": "human", "name": "guest"}, "role": "viewer"},
        {"id": "claude", "author": {"kind": "agent", "name": "claude"}, "role": "proposer"},
        {"id": "claude-guest", "author": {"kind": "agent", "name": "claude-guest"}, "role": "proposer"},
    ]
    if r5b:
        out[3]["on_behalf"] = "arkin"
        out[4]["on_behalf"] = "guest"
    return out


def init(r5b):
    v = {
        "document": {
            "schema_version": "0",
            "id": "doc-r5b-harness",
            "title": "R5B ölçüm oturumu (sentetik)",
            "display_tz": "Europe/Istanbul",
            # One view (the propose tasks change it; series `syn04` is added by bind_all).
            "views": [{
                "id": "main", "title": "SYN04 1G", "x": {"kind": "time"},
                "y": [{"kind": "linear"}],
                "layers": [{"mark": "candle", "series": "syn04", "panel": 0, "encoding": {}}],
            }],
            "audiences": {
                "arkin": {
                    "group": "habituated",
                    "sensitivity": "balanced",
                    "watchlist": SYMS[:6],
                    "positions": {"SYN02": 100, "SYN05": 40, "SYN11": -20},
                },
                "guest": {"group": "future"},
            },
        },
        "principals": principals(r5b),
        "clock": 0,
        "epoch": 0,
        "capsules": {
            "series": [],
            "flow": [{"series": s, "seed": seed, "feed": "flow"} for s, seed in FLOW],
            "engine": "rp4",
        },
        "feed_rule": {"rule": "rate55", "exempt_negative": True},
        "replay": {
            "source": "synthetic",
            "instruments": N,
            "days": 170,
            "seed": 5,
            "interval": "1d",
            "history": HISTORY,
            "path": False,
            "feed": "bist",
            "speed": 4.0,
            "bind_all": True,
            "capsules_all": True,
            "entitlements": {
                "host": ["*"],
                "arkin": ["*"],
                "claude": ["*"],
                "guest": GUEST,
                "claude-guest": GUEST,
            },
        },
    }
    if r5b:
        v["agent_policy"] = "r5b"
        v["comment_strictness"] = "cites_numbers"
    return v


def main():
    for name, r5b in (("init.json", False), ("init.r5b.json", True)):
        with open(os.path.join(HERE, name), "w", encoding="utf-8") as f:
            json.dump(init(r5b), f, ensure_ascii=False, indent=1, sort_keys=True)
            f.write("\n")
    with open(os.path.join(HERE, "position.json"), "w", encoding="utf-8") as f:
        json.dump({"position": POSITION, "m31_steps": M31_STEPS, "drive_as": "arkin"}, f,
                  sort_keys=True, indent=1)
        f.write("\n")


if __name__ == "__main__":
    main()
