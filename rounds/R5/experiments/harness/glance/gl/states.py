"""The fixed synthetic state set (SPEC §G2): found in `qp replay` (the engine) and in the
reader's own feed (the authority's feed query, probed in the page), never in a design.
Deterministic: same `qp` binary, same web build and arguments -> same `states.json`.

The demo (`qp demo --synthetic`) and `qp replay` build the same replay from the same flags;
a page seek to step N shows the authority state after N steps. `find` checks this for every
listed capsule (the page's sentence must equal the engine item's sentence) and records
mismatches.
"""

from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path

from .keys import MIN_LEDGER_N, _day, trading_days

# Candidate steps: every 20th step of the 1260-step demo replay (step 1 included: the
# history's closed capsules are there). Order = preference (ascending).
CANDIDATE_STEPS = [1] + list(range(20, 1261, 20))


def _led(it):
    led = it.get("ledger_at_birth") or {}
    return int(led.get("n", 0)), int(led.get("realized", 0))


def _age(it, now_t):
    a = _day(int(it["t"]))
    return trading_days(a, max(a, _day(now_t)))


# Face state rules (SPEC §G2.1). Each takes (item, now_t) -> bool. The first matching
# capsule in the reader's feed order is the target; up to 7 fallbacks are kept.
RULES = {
    "live_event": lambda it, now: it["time"] == "event" and it["validity"] == "valid"
    and it["kind"] in ("breakout", "reversal") and _led(it)[0] >= MIN_LEDGER_N and _age(it, now) <= 2,
    "no_longer": lambda it, now: it["validity"] == "no_longer",
    "closed_realized": lambda it, now: it["stage"] == "realized",
    "closed_failed": lambda it, now: it["stage"] == "invalidated",
    "few_cases": lambda it, now: it["validity"] == "valid" and 0 < _led(it)[0] < MIN_LEDGER_N,
    "no_record": lambda it, now: it["validity"] == "valid" and _led(it)[0] == 0,
    "state_live": lambda it, now: it["time"] == "state" and it["validity"] == "valid"
    and _led(it)[0] >= MIN_LEDGER_N,
}


def replay_at(qp: str, step: int | None, extra: list[str] | None = None, cwd: str | None = None):
    """Runs `qp replay [--steps step]` and returns (items by id at that step, now_t)."""
    with tempfile.TemporaryDirectory() as d:
        out = Path(d) / "r.jsonl"
        steps = [] if step is None else ["--steps", str(step)]
        subprocess.run([qp, "replay", *steps, "-o", str(out), *(extra or [])],
                       check=True, cwd=cwd, stdout=subprocess.DEVNULL)
        items, now_t = {}, 0
        for line in out.open():
            m = json.loads(line)["msg"]
            if m.get("type") == "welcome":
                now_t = max(now_t, int(m.get("now", 0)))
            if m.get("type") != "notify":
                continue
            e = m["event"]
            if e.get("kind") == "capsule":
                c = e["capsule"]
                items[c["id"]] = c
                now_t = max(now_t, int(c["t"]))
            elif e.get("kind") == "change":
                now_t = max(now_t, int(e.get("at", 0)))
        return items, now_t


def matches(items: dict, now_t: int, listed=None) -> dict[str, list[str]]:
    """Capsule ids per rule; `listed` (the reader's feed ids at that step) restricts them to
    capsules the page lists, keeping the feed's order."""
    out = {}
    order = list(listed) if listed is not None else sorted(items)
    for name, rule in RULES.items():
        ids = [i for i in order if i in items and rule(items[i], now_t)]
        if ids:
            out[name] = ids
    return out


def find(feeds: dict, qp: str | None = None, cwd: str | None = None) -> dict:
    """Chooses the face step S*: the probed step whose reader feed satisfies the most face
    rules (ties: the earliest); a rule S* cannot satisfy is taken from the earliest step
    that can. `feeds` = probe.json from `shoot.mjs`: {principal: {step: {replay_t,
    replay_step, rows: [{id, text, item}]}}} -- the items are the authority's own
    `CapsuleItem`s as the page received them (engine data, not a design)."""
    own = feeds["arkin"]
    steps = sorted(int(k) for k in own)
    per_step, problems = {}, []
    for s in steps:
        p = own[str(s)]
        if p.get("replay_step") is not None and int(p["replay_step"]) != s:
            problems.append({"step": s, "problem": f"page at step {p['replay_step']}"})
        items = {r["id"]: r["item"] for r in p["rows"] if r.get("item")}
        missing = [r["id"] for r in p["rows"] if not r.get("item")]
        if missing:
            problems.append({"step": s, "problem": "rows without item", "ids": missing})
        now_t = int(p["replay_t"])
        per_step[s] = (items, now_t, matches(items, now_t, [r["id"] for r in p["rows"]]))
    best = max(steps, key=lambda s: (len(per_step[s][2]), -s))
    states = []
    for name in RULES:
        s = best if name in per_step[best][2] else next((x for x in steps if name in per_step[x][2]), None)
        if s is not None:
            items, now_t, m = per_step[s]
            states.append({"state": name, "found": True, "step": s, "now_t": now_t, "principal": "arkin",
                           "candidates": m[name][:8], "items": {i: items[i] for i in m[name][:8]}})
            continue
        # Not in the default reader feed (its 40-capsule limit favours important capsules):
        # the engine's full capsule list (`qp replay`), and the shot puts the capsule's
        # instrument on the reader's watch list (`?watch=`; the same for every design).
        found = None
        for x in ([best] + [y for y in steps if y != best]) if qp else []:
            items, now_t = replay_at(qp, x, cwd=cwd)
            m = matches(items, now_t)
            if name in m:
                found = (x, now_t, m[name][0], items)
                break
        if found is None:
            states.append({"state": name, "found": False})
            continue
        x, now_t, cid, items = found
        states.append({"state": name, "found": True, "step": x, "now_t": now_t, "principal": "arkin",
                       "watch": items[cid]["instrument"], "candidates": [cid], "items": {cid: items[cid]},
                       "note": "from qp replay; target listed through ?watch="})
    # Unentitled reader (K33): valid capsules of the visitor's feed at S*.
    vp = feeds.get("visitor", {}).get(str(best))
    vids, vitems = [], {}
    if vp:
        vitems = {r["id"]: r["item"] for r in vp["rows"] if r.get("item")}
        vids = [i for i in (r["id"] for r in vp["rows"]) if i in vitems and vitems[i]["validity"] == "valid"]
    if vids:
        states.append({"state": "visitor", "found": True, "step": best, "now_t": per_step[best][1],
                       "principal": "visitor", "redacted": True, "candidates": vids[:8],
                       "items": {i: vitems[i] for i in vids[:8]}})
    else:
        states.append({"state": "visitor", "found": False})
    states.append({"state": "feed", "found": True, "step": best, "now_t": per_step[best][1], "principal": "arkin"})
    return {"harness": "r5-glance/1", "face_step": best, "problems": problems,
            "method": "probe the reader feed (shoot.mjs) at CANDIDATE_STEPS; S* = step whose feed satisfies most rules",
            "states": states}


def chart_depths(qp: str, targets=(5, 50, 200), cwd: str | None = None) -> list[dict]:
    """M18 chart-depth states: an instrument and a synthetic length whose capsule count
    (born up to the replay's end) is the smallest >= target (SPEC §G2.3). `--days` grows
    the synthetic market (the demo takes the same flag)."""
    out = []
    for days in (260, 1000, 2600):
        items, now_t = replay_at(qp, None, ["--days", str(days)], cwd=cwd)
        by: dict[str, list] = {}
        for it in items.values():
            by.setdefault(it["instrument"], []).append(it)
        for tgt in targets:
            if any(o["target"] == tgt for o in out):
                continue
            fits = sorted((len(v), k) for k, v in by.items() if len(v) >= tgt)
            if fits:
                n, inst = fits[0]
                live = sorted(i["id"] for i in by[inst] if i["validity"] != "closed")
                out.append({"target": tgt, "days": days, "instrument": inst, "capsules": n,
                            "live": live, "realized": sum(1 for i in by[inst] if i["stage"] == "realized"),
                            "now_t": now_t})
    return out
