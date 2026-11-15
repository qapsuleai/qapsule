#!/usr/bin/env python3
"""R3/E hidden setup: two bound instruments (THYAO, ASELS) with the capsule engine on.

`make(seed)` returns the session init, the id mapping, the hidden setup lines and the
hidden epilogue. Everything is a pure function of the seed and of the bars printed by
`gen/` (`qp_source::synthetic_market(2, N_GEN, DATA_SEED)`), so a run can be rebuilt from
the seed recorded in its raw logs.

Per-agent randomization (the protocol assigns capsule/finding/fact ids itself, from the
symbol, kind, direction and birth bar: `cap.THYAO.breakout.down.124`; they cannot be
renamed):
- the two series ids (random 5-character ids);
- the data window: bars `[off, END)` of the generated series, `off` in `0..OFF_MAX` drawn
  from the seed, so every engine id (birth bar) and every time differs per agent while the
  last bars (and so the open stories) stay the same. DATA_SEED, END and OFF_MAX were chosen
  by a scan (RESULTS.md §1.1) so that every offset has: one open THYAO breakout, one open
  THYAO extreme, an invalidated ASELS story, an expired story, THYAO flow (licensed)
  stories and one THYAO squeeze story.

Hidden state (rounds/R3/tasks/TASKS.md): THYAO and ASELS, 315-320 daily bars each on feed
`bist`, engine on both, synthetic licensed flow on both (entitlement `flow/*`);
settled source = the same bars (disputes are decided at once: no correction -> rejected).
Principals: host, arkin (owner, desktop), editor (the agent, headless), viewer (bars only,
no `flow/*`).
"""

import json
import os
import random
import string
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
SCR = os.environ.get("E_SCR", "/private/tmp/claude-501/-Users-arkns-Works-Mchart-qapsule/"
                     "9888bcbf-bbc3-4088-87e1-bd73f855b8fc/scratchpad")
GEN = os.path.join(SCR, "target", "release", "r3-ergo-gen")

DATA_SEED = 2
N_GEN = 340     # bars generated per instrument
END = 320       # the window always ends here (same last bars, same open stories)
OFF_MAX = 6     # window start drawn in 0..OFF_MAX-1: N = END - off bars (315..320)
SYMS = ["THYAO", "ASELS"]
FLOW_SEEDS = {"THYAO": 1, "ASELS": 2}
DAY = 86_400_000

SPEC_IDS = {"px", "rsi14", "rsi7", "macd", "bb20", "sma20", "sma50", "main", "candles", "line",
            "level", "fact", "hist", "cap", "fnd"}
ALPH = string.ascii_lowercase + string.digits

_BARS = None


def bars_all():
    global _BARS
    if _BARS is None:
        cache = os.path.join(SCR, "gen-bars-%d-%d.json" % (N_GEN, DATA_SEED))
        if not os.path.exists(cache):
            out = subprocess.run([GEN, str(N_GEN), str(DATA_SEED)], check=True, capture_output=True,
                                 text=True).stdout
            with open(cache, "w") as f:
                f.write(out)
        _BARS = json.load(open(cache))
    return _BARS


def rand_id(rng, used):
    while True:
        s = rng.choice(string.ascii_lowercase) + "".join(rng.choice(ALPH) for _ in range(4))
        if s in used or s in SPEC_IDS or (s[0] == "l" and s[1:].isdigit()):
            continue
        used.add(s)
        return s


def feed(sym):
    return {"symbol": sym, "interval": "1d", "feed": "bist"}


def make_init(m, off):
    b = bars_all()
    win = {s: b[i][off:END] for i, s in enumerate(SYMS)}
    last_t = win["THYAO"][-1]["t"]
    series = [{"id": m[s], "instrument": {"symbol": s}, "interval": "1d",
               "source": {"kind": "bind", "symbol": s, "interval": "1d", "window": {"last": 10000},
                          "feed": "bist"}} for s in SYMS]
    return {
        "document": {"schema_version": "0", "id": "doc-" + m["doc"], "title": "Kapsüller",
                     "series": series},
        "principals": [
            {"id": "host", "author": {"kind": "engine", "name": "host", "version": "0"}, "role": "host"},
            {"id": "arkin", "author": {"kind": "human", "name": "arkin"}, "role": "owner"},
            {"id": "editor", "author": {"kind": "agent", "name": "editor"}, "role": "editor"},
            {"id": "viewer", "author": {"kind": "human", "name": "viewer"}, "role": "viewer"},
        ],
        # the clock: the day after the last bar (every bar is closed and final)
        "clock": last_t + DAY + 10 * 3_600_000,
        "epoch": 0,
        "data": {
            "feeds": [{"feed": feed(s), "bars": win[s]} for s in SYMS],
            "entitlements": {"host": ["*"], "arkin": ["*"], "editor": ["*"],
                             "viewer": ["bist/THYAO", "bist/ASELS"]},
            "settled": {"feeds": [{"feed": feed(s), "bars": win[s]} for s in SYMS], "verify_closed": False},
        },
        "capsules": {"series": [m[s] for s in SYMS],
                     "flow": [{"series": m[s], "seed": FLOW_SEEDS[s]} for s in SYMS]},
    }


def h(conn, principal, kind="headless"):
    return {"conn": conn, "msg": {"type": "hello", "protocol": "qp/0", "principal": principal,
                                  "surface": {"kind": kind}}}


def setup_lines(m):
    """Hidden setup: the host and arkin connect; the host reads the starting state (the
    scorer's ground truth: every story, the ledger per kind), before the agent's lines."""
    q = lambda i, query: {"conn": "zz-host", "msg": {"type": "query", "id": i, "query": query}}
    return [h("zz-host", "host"), h("zz-ark", "arkin", "desktop"),
            q("s-caps", {"kind": "capsules"}),
            q("s-ledger", {"kind": "ledger", "by_kind": True})]


def epilogue_lines(m):
    """Hidden final state, read by the scorer: the document as the host sees it, the log,
    and the document as `editor` sees it (its own audience)."""
    q = lambda c, i, query: {"conn": c, "msg": {"type": "query", "id": i, "query": query}}
    return [q("zz-host", "e-doc", {"kind": "document"}),
            q("zz-host", "e-log", {"kind": "log"}),
            h("zz-ed", "editor"),
            q("zz-ed", "e-ed-doc", {"kind": "document"})]


def make_mapping(seed):
    rng = random.Random(seed)
    used = set()
    m = {k: rand_id(rng, used) for k in SYMS + ["doc"]}
    m["off"] = rng.randrange(OFF_MAX)
    return m


def make(seed, off=None):
    m = make_mapping(seed)
    if off is not None:
        m["off"] = off
    return {"seed": seed, "mapping": m, "init": make_init(m, m["off"]),
            "setup": setup_lines(m), "epilogue": epilogue_lines(m)}


if __name__ == "__main__":
    import sys
    r = make(int(sys.argv[1]) if len(sys.argv) > 1 else 1)
    print(json.dumps(r["mapping"], indent=1))
