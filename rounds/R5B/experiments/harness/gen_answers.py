#!/usr/bin/env python3
"""R5B/H: the right answers, from the deterministic engine (not from a model).

Starts `qp serve` on session/init.json (paused at the fixed position, the end of the
replay's history; session/position.json may add a seek), and asks the reference authority, as the person each agent
acts for (claude -> arkin, claude-guest -> guest; SPEC.md §2), every question a task's
`truth` recipe names. Writes:

  answers.json            per task: the expected answer fields and context ids,
                          plus the guest leak set (licensed values and hidden ids)
  comments/comments.json  the comment set for M30 (correct and deliberately broken
                          drafts; cited ids and values taken from the engine)
  m31/events.json         every capsule transition of the M31 window (the days stepped
                          after the position), for the sentinel's filter scoring

Nothing here reads the clock, the network or a random source; two runs give the same
bytes (SPEC.md §3, checked with --out to two directories and `cmp`).

usage: python3 gen_answers.py [--out DIR] [--qp PATH]
"""
import argparse
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "lib"))
from qpserve import Serve, QpError  # noqa: E402
import commentset  # noqa: E402

READER = {"claude": "arkin", "claude-guest": "guest"}
CONN = {"arkin": "a", "guest": "g", "claude": "c", "claude-guest": "cg"}
DAY = 86_400_000


class Engine:
    """Read helpers over one paused session at the fixed position."""

    def __init__(self, s):
        self.s = s
        self._caps = {}

    def q(self, who, query):
        return self.s.query(CONN[who], query)

    def data(self, who, query):
        return self.q(who, query)["data"]

    def capsules(self, who, **filt):
        return self.data(who, dict({"kind": "capsules"}, **filt))["capsules"]

    def detail(self, who, cid):
        key = (who, cid)
        if key not in self._caps:
            self._caps[key] = self.data(who, {"kind": "capsule", "capsule": cid})["detail"]
        return self._caps[key]

    def resolve(self, who, sel):
        """A task selector -> exactly one capsule item (or raises: the task is ambiguous)."""
        filt = {}
        for k in ("instrument", "meaning", "open"):
            if k in sel:
                filt[k] = sel[k]
        items = self.capsules(who, **filt)
        if "direction" in sel:
            items = [c for c in items if c["direction"] == sel["direction"]]
        if "fact_kind_prefix" in sel:
            p = sel["fact_kind_prefix"]
            items = [c for c in items
                     if any(f["kind"].startswith(p) for f in self.born_facts(who, c["id"]))]
        if sel.get("latest_born"):
            items = sorted(items, key=lambda c: (born_t(self.detail(who, c["id"])), c["born_bar"], c["id"]))
            items = items[-1:]
        if len(items) != 1:
            raise QpError(f"selector {sel} as {who}: {len(items)} capsules")
        return items[0]

    def born_facts(self, who, cid):
        d = self.detail(who, cid)
        born = d["story"]["history"][0]["facts"]
        return [f for f in d["facts"] if f["id"] in born]

    def last_final_bar(self, who="arkin"):
        return self.data(who, {"kind": "at", "series": "syn01"})["at"]["bar"] - 1


def born_t(detail):
    return detail["story"]["history"][0]["t"]


def dig(obj, path):
    for p in path.split("."):
        obj = obj[p]
    return obj


# ----------------------------------------------------------------------------- recipes


def r_capsules_pick(e, who, r):
    items = e.capsules(who, **r["q"])
    out = {k: (items[0][v] if items else None) for k, v in r["out"].items()}
    if "status" in r:
        out["status"] = r["status"]
    return out, {"n_candidates": len(items)}


def r_capsules_ids(e, who, r):
    q = dict(r["q"])
    if "since_last_final" in r:
        q["since_bar"] = e.last_final_bar(who) - r["since_last_final"] + 1
    items = e.capsules(who, **q)
    if r.get("instruments_from") == "positions":
        pos = position_symbols(e, who)
        items = [c for c in items if c["instrument"] in pos]
    return {"capsule_ids": sorted(c["id"] for c in items)}, {"query": q}


def r_capsules_instruments(e, who, r):
    items = e.capsules(who, **r["q"])
    items = [c for c in items if c["direction"] == r["direction"]]
    return {"instruments": sorted({c["instrument"] for c in items})}, {"ids": sorted(c["id"] for c in items)}


def r_capsules_count(e, who, r):
    items = e.capsules(who, **r["q"])
    return {"count": len(items)}, {"ids": sorted(c["id"] for c in items)}


def stories(e, who, instrument, rng):
    rows, after = [], None
    while True:
        q = {"kind": "stories", "instrument": instrument, "range": rng, "limit": 500}
        if after:
            q["after"] = after
        d = e.data(who, q)
        rows += d["stories"]
        after = d.get("next")
        if not after:
            return rows


def r_stories_count(e, who, r):
    rows = stories(e, who, r["instrument"], r["range"])
    hit = [x["id"] for x in rows if x.get("outcome") == r["outcome"]]
    return {"count": len(hit)}, {"ids": hit, "total": len(rows)}


def r_capsule_fact(e, who, r):
    c = e.resolve(who, r["sel"])
    facts = e.born_facts(who, c["id"])
    f = facts[0]
    out = {"capsule_id": c["id"]}
    for k, v in r["out"].items():
        out[k] = f["evidence"][v]
    return out, {"fact": f["id"], "finding": c["finding"]}


def r_capsule_story(e, who, r):
    c = e.resolve(who, r["sel"])
    d = e.detail(who, c["id"])
    out = {"capsule_id": c["id"]}
    for k, path in r["out"].items():
        if path == "history.stages":
            out[k] = [h["stage"] for h in d["story"]["history"]]
        elif path == "history.last.reason":
            out[k] = d["story"]["history"][-1]["reason"]
        else:
            out[k] = dig(d, path)
    return out, {"finding": c["finding"]}


def r_verify(e, who, r):
    c = e.resolve(who, r["sel"])
    chk = e.data(who, {"kind": "verify_evidence", "item": c["finding"]})["check"]
    return {"capsule_id": c["id"], "verified": chk["verified"]}, {"finding": c["finding"]}


def r_ledger(e, who, r):
    d = e.data(who, dict({"kind": "ledger"}, **r["q"]))
    row = d["total"] if r["row"] == "total" else d["rows"][r["row"]]
    return {"n": row["n"], "realized": row["realized"]}, {"rows": len(d["rows"])}


def close_of(e, who, series):
    return e.data(who, {"kind": "at", "series": series})["at"]["ohlcv"]["c"]


def r_at(e, who, r):
    a = e.data(who, {"kind": "at", "series": r["series"]})["at"]
    return {k: a["ohlcv"][v] for k, v in r["out"].items()}, {"bar": a["bar"]}


def unique_max(scores):
    best = max(scores.values())
    top = sorted(k for k, v in scores.items() if v == best)
    return top


def r_cmp_open_count(e, who, r):
    counts = {s: len(e.capsules(who, instrument=s, open=True)) for s in r["instruments"]}
    top = unique_max(counts)
    if len(top) != 1:
        raise QpError(f"cmp_open_count tie {counts}")
    return {"instrument": top[0], "counts": counts}, {}


def r_cmp_kind_rate(e, who, r):
    rows = {x["kind"]: x for x in e.data(who, {"kind": "ledger", "by_kind": True})["rows"]}
    rate = {k: rows[k]["realized"] / rows[k]["n"] for k in r["kinds"]}
    top = unique_max(rate)
    return {"kind": top[0]}, {"rows": {k: [rows[k]["n"], rows[k]["realized"]] for k in r["kinds"]}}


def r_cmp_close(e, who, r):
    closes = {s: close_of(e, who, s.lower()) for s in r["instruments"]}
    top = unique_max(closes)
    return {"instrument": top[0], "closes": closes}, {}


def r_max_stories(e, who, r):
    syms = sorted({c["instrument"] for c in e.capsules(who)})
    totals = {s: len(stories(e, who, s, r["range"])) for s in syms}
    return {"instrument": unique_max(totals)}, {"totals": totals}


def r_now(e, who, r):
    return {}, {"now": e.q(who, {"kind": "list", "collection": "intents"})["now"]}


def r_target(e, who, r):
    c = e.resolve(who, r["sel"])
    facts = e.born_facts(who, c["id"])
    ctx = {"capsule_id": c["id"], "finding": c["finding"], "facts": [f["id"] for f in facts],
           "instrument": c["instrument"]}
    if "value" in r:
        ctx["value"] = facts[0]["evidence"][r["value"]]
    return {}, ctx


def r_const(e, who, r):
    return dict(r["value"]), {}


RECIPES = {k[2:]: v for k, v in globals().items() if k.startswith("r_")}


def position_symbols(e, who):
    doc = e.data(who, {"kind": "document"})["document"]
    aud = doc.get("audiences", {}).get(who, {})
    return sorted(s for s, q in aud.get("positions", {}).items() if q)


# ----------------------------------------------------------------------------- leak set


def leak_set(e):
    """What the guest must never receive: ids that exist for arkin but not for the guest,
    and the values the guest's view withholds (licensed flow evidence; bars and evidence
    of the instruments without a bar entitlement)."""
    full = e.capsules("arkin")
    seen = {c["id"] for c in e.capsules("guest")}
    hidden_ids, values = set(), set()
    for c in full:
        d = e.detail("arkin", c["id"])
        if c["id"] not in seen:
            hidden_ids.update([c["id"], c["finding"]])
            hidden_ids.update(f["id"] for f in d["facts"])
            gd = None
        else:
            gd = e.detail("guest", c["id"])
        gvals = {}
        if gd:
            for f in gd["facts"]:
                gvals[f["id"]] = f.get("evidence") or {}
        for f in d["facts"]:
            for k, v in (f.get("evidence") or {}).items():
                if k.endswith("_bar"):
                    continue
                if gvals.get(f["id"], {}).get(k) != v:
                    values.add(v)
    ent = {s for s in guest_bar_symbols(e)}
    for c in sorted({c["instrument"] for c in full}):
        if c in ent:
            continue
        st = e.data("arkin", {"kind": "study", "series": c.lower(), "range": {"last": 30}})["values"]
        for out in ("o", "h", "l", "c"):
            values.update(x for x in st["outputs"][out] if x is not None)
    return {"ids": sorted(hidden_ids), "values": sorted(values)}


def guest_bar_symbols(e):
    syms = sorted({c["instrument"] for c in e.capsules("arkin")})
    ok = []
    for s in syms:
        m = e.s.query_or_error("g", {"kind": "at", "series": s.lower()})
        if m.get("type") == "result":
            ok.append(s)
    return ok


# ----------------------------------------------------------------------------- equivalence


def equivalence(e):
    """The agent principals read what their person reads (today's init has no
    `on_behalf`; the agents' own entitlements equal their person's). Recorded, checked."""
    out = {}
    pairs = [("claude", "arkin"), ("claude-guest", "guest")]
    probes = [{"kind": "capsules"}, {"kind": "ledger", "by_kind": True},
              {"kind": "stories", "instrument": "SYN03", "range": {"last": 60}},
              {"kind": "capsule", "capsule": {"instrument": "SYN07", "latest": True}}]
    for agent, person in pairs:
        same = []
        for p in probes:
            a = e.data(agent, p)
            b = e.data(person, p)
            same.append(strip_relevant(a) == strip_relevant(b))
        out[f"{agent}=={person}"] = same
    return out


def strip_relevant(v):
    """`relevant` depends on the person's own audience (positions), which the agent
    principal has none of in today's init; everything else must be equal."""
    if isinstance(v, dict):
        return {k: strip_relevant(x) for k, x in v.items() if k not in ("relevant", "in_feed", "hidden")}
    if isinstance(v, list):
        return [strip_relevant(x) for x in v]
    return v


# ----------------------------------------------------------------------------- M31


def m31_events(s, steps):
    """Steps the replay day by day and records every capsule transition arkin's
    connection would see with `all: true` (no filter: the filters are what M31 measures)."""
    s.reply("m", {"type": "subscribe", "topic": {"kind": "capsules", "all": True}})
    events = []
    for day in range(1, steps + 1):
        out = s.send("h", {"type": "replay", "id": f"step{day}", "control": {"kind": "step", "n": 1}})
        for o in out:
            m = o.get("msg", {})
            if o.get("conn") == "m" and m.get("type") == "notify" and m["event"].get("kind") == "capsule":
                ev = m["event"]
                c = ev["capsule"]
                events.append({"day": day, "seq": ev.get("seq"), "capsule": c["id"],
                               "instrument": c["instrument"], "kind": c["kind"],
                               "direction": c["direction"], "stage": ev["transition"]["stage"]
                               if isinstance(ev.get("transition"), dict) else c["stage"],
                               "tier": c.get("tier"), "relevant": bool(c.get("relevant")),
                               "coin_flip": bool(c.get("coin_flip"))})
    return events


# ----------------------------------------------------------------------------- main


def dump(path, v):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(v, f, ensure_ascii=False, indent=1, sort_keys=True)
        f.write("\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=HERE)
    ap.add_argument("--qp", default=None)
    a = ap.parse_args()
    pos = json.load(open(os.path.join(HERE, "session", "position.json")))
    tasks = json.load(open(os.path.join(HERE, "tasks.json")))["tasks"]
    init = os.path.join(HERE, "session", "init.json")
    with Serve(init, qp=a.qp) as s:
        s.hello("h", pos["drive_as"])
        if pos["position"]:
            p = dict(pos["position"])
            kind = p.pop("kind")
            state = s.replay("h", dict({"kind": kind}, **p))
        else:
            state = s.replay("h", {"kind": "status"})
        welcome = {}
        for who, conn in CONN.items():
            w = s.hello(conn, who)
            welcome[who] = {"seq": w["seq"], "now": w["now"], "role": w["role"]}
        e = Engine(s)
        answers = {}
        for t in tasks:
            r = t.get("truth")
            if not r:
                answers[t["id"]] = {"expect": None, "context": {}, "source": "state"}
                continue
            who = r.get("as") or READER[t["principal"]]
            exp, ctx = RECIPES[r["fn"]](e, who, r)
            answers[t["id"]] = {"expect": exp or None, "context": ctx, "reader": who,
                                "source": "engine" if r["fn"] != "const" else "rule"}
        meta = {"position": pos["position"], "replay": {k: state[k] for k in ("t", "step", "steps", "epoch")},
                "welcome": welcome, "last_final_bar": e.last_final_bar(),
                "positions": position_symbols(e, "arkin"),
                "guest_bar_symbols": guest_bar_symbols(e),
                "equivalence": equivalence(e)}
        leaks = leak_set(e)
        comments = commentset.build(e)
        s.hello("m", "arkin")
        events = m31_events(s, pos["m31_steps"])
    dump(os.path.join(a.out, "answers.json"), {"meta": meta, "answers": answers, "leaks": leaks})
    dump(os.path.join(a.out, "comments", "comments.json"), comments)
    dump(os.path.join(a.out, "m31", "events.json"), {"steps": pos["m31_steps"], "events": events})
    n_eng = sum(1 for v in answers.values() if v["source"] == "engine")
    print(f"answers: {len(answers)} tasks ({n_eng} from engine queries); leaks: {len(leaks['ids'])} ids, "
          f"{len(leaks['values'])} values; comments: {len(comments['items'])}; m31 events: {len(events)}")


if __name__ == "__main__":
    main()
