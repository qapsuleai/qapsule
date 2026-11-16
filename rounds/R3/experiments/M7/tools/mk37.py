"""Builds the skeleton of conformance transcript 37 (dispute and retraction, §19.9).

usage: python3 -I mk37.py > skeleton.json
The expectations are then taken from `qp serve --host-frames --batch-end` and checked
by hand (see rounds/R3/experiments/M7/RESULTS.md).
"""
import json

FEED = {"symbol": "THYAO", "interval": "1d"}
T0 = 1759708800000
DAY = 86400000
T = [T0 + i * DAY for i in range(8)]


def bar(i, c, o=None, h=None, low=None, v=1000.0):
    o = c if o is None else o
    return {"t": T[i], "o": o, "h": max(h if h is not None else c + 0.5, o, c),
            "l": min(low if low is not None else c - 0.5, o, c), "c": c, "v": v}


prov = {"author": {"kind": "agent", "name": "scout"}, "at": 1760000000000, "based_on": ["fact-hold"]}
doc = {
    "schema_version": "0", "id": "doc-dispute", "title": "İtiraz ve geri çekme",
    "series": [{"id": "px", "instrument": {"symbol": "THYAO"}, "interval": "1d",
                "source": {"kind": "bind", "symbol": "THYAO", "interval": "1d", "window": {"last": 5}}}],
    "facts": [{"id": "fact-hold", "kind": "threshold", "series": "px", "bar": 3, "t": T[3],
               "text": "THYAO 103'te kapandı, bar 3.", "evidence": {"value": 103.0},
               "provenance": {"author": {"kind": "engine", "name": "qp-facts", "version": "0.0.0"},
                              "at": 1760000000000, "based_on": ["px"]}}],
    "findings": [{"id": "fnd-floor", "claim": "THYAO 90'ın üstünde tutunuyor.", "facts": ["fact-hold"],
                  "confidence": 0.5,
                  "invalidation": {"kind": "threshold", "series": "px", "op": "lt", "value": 90.0},
                  "provenance": prov}],
    "capsules": [{"id": "cap-floor", "finding": "fnd-floor", "state": "live", "revision": 0,
                  "headline": "THYAO 90'ın üstünde.", "glyphs": [{"kind": "direction", "value": 0.0}],
                  "depth": "capsule",
                  "story": {"instrument": "THYAO", "theme": "floor", "kind": "continuation",
                            "direction": "up", "time": "state", "stage": "born", "families": ["price"],
                            "strength": 0.6, "importance": 0.5, "tier": 2, "series": "px",
                            "born_bar": 3, "last_bar": 3, "horizon_bars": 20,
                            "ledger_key": "continuation/threshold@1",
                            "ledger_at_birth": {"n": 0, "realized": 0}, "trust": "replica",
                            "history": [{"stage": "born", "bar": 3, "t": T[3], "facts": ["fact-hold"],
                                         "reason": "new_story"}]},
                  "provenance": prov}],
    "intents": [], "ledger": {"entries": []},
}
init = {
    "document": doc, "clock": 1760000000000, "epoch": 0, "proposal_ttl_ms": None,
    "principals": [
        {"id": "arkin", "author": {"kind": "human", "name": "arkin"}, "role": "owner"},
        {"id": "device", "author": {"kind": "human", "name": "device"}, "role": "viewer"},
        {"id": "host", "author": {"kind": "engine", "name": "host", "version": "0"}, "role": "host"},
    ],
    "data": {
        "feeds": [{"feed": FEED, "bars": [bar(0, 100.0), bar(1, 101.0), bar(2, 102.0), bar(3, 103.0),
                                          bar(4, 104.0)]}],
        "entitlements": {"host": ["*"], "arkin": ["*"], "device": ["*"]},
        "settled": {"feeds": [{"feed": FEED, "bars": [bar(4, 104.0)]}], "verify_closed": False},
    },
}


def send(conn, msg, note=None):
    s = {"send": {"conn": conn, "msg": msg}, "expect": []}
    if note:
        s = {"note": note, **s}
    return s


def data(seq, change, note=None):
    s = {"data": {"feed": FEED, "seq": seq, "change": change}, "expect": []}
    if note:
        s = {"note": note, **s}
    return s


hello = lambda p: {"type": "hello", "protocol": "qp/0", "principal": p, "surface": {"kind": "desktop"}}
steps = [
    send("a", hello("arkin"), "Owner; the anchor is stamped at load (entry 1)."),
    send("r", hello("device"), "A replica (viewer, entitled to the feed)."),
    send("a", {"type": "subscribe", "id": "ch", "topic": {"kind": "changes", "ids": ["cap-floor"]}}),
    send("a", {"type": "subscribe", "id": "ka", "topic": {"kind": "claims"}}),
    send("r", {"type": "subscribe", "id": "k", "topic": {"kind": "claims"}}),
    send("r", {"type": "subscribe", "id": "c", "topic": {"kind": "condition",
              "condition": {"kind": "threshold", "series": "px", "op": "gt", "value": 110.0}}}),
    data(1, {"kind": "patch", "t": T[4], "fields": {"c": 85.0, "l": 85.0}},
         "A bad tick on the authority's own feed: t4 at 85 (the source's t4 closes at 104). The capsule's invalidation (px < 90) holds: engine entry, capsule claim (provisional)."),
    data(2, {"kind": "bar", "bar": bar(5, 101.0)},
         "t5 opens: t4 is closed on the authority's data; the capsule claim is asserted final (wrong)."),
    send("r", {"type": "dispute", "id": "d1", "claim": {"kind": "capsule", "capsule": "cap-floor", "finding": "fnd-floor"},
               "t": T[4], "values": [{"series": "px", "t": T[4], "value": 104.0, "prev": 103.0}],
               "anchor_t": T[0], "digest": "1111111111111111"},
         "The replica's clean, refetched t4 closes at 104: it disputes. The authority re-checks on its provider's settled bars (test source: t4 = 104): upheld. Its t4 is corrected, the claim withdrawn (dispute_upheld) and the capsule retracted (story stage retracted, K37; no ledger row)."),
    data(3, {"kind": "patch", "t": T[5], "fields": {"c": 112.0, "h": 112.0}},
         "t5 crosses 110 on the replica's condition subscription (provisional)."),
    data(4, {"kind": "bar", "bar": bar(6, 112.0)}, "t6 opens: the condition claim at t5 is final."),
    send("r", {"type": "dispute", "id": "d2", "claim": {"kind": "condition", "sub": "c"}, "t": T[5],
               "values": [{"series": "px", "t": T[5], "value": 108.0, "prev": 104.0}],
               "anchor_t": T[0], "digest": "2222222222222222"},
         "A dispute the settled data does not support: rejected; the claim stands and its evidence goes back to the disputer only (reason dispute_rejected)."),
    send("r", {"type": "dispute", "id": "d3", "claim": {"kind": "condition", "sub": "c"}, "t": T[5],
               "values": [{"series": "px", "t": T[5], "value": 108.0, "prev": 104.0}],
               "anchor_t": T[0], "digest": "2222222222222222"},
         "The same dispute again (claim, t, digest): duplicate, nothing new."),
    send("r", {"type": "dispute", "id": "d4", "claim": {"kind": "condition", "sub": "c"}, "t": T[3]},
         "A claim the authority does not assert: unknown."),
]
out = {"name": "dispute-retract", "covers": ["dispute", "claims", "data"],
       "description": "R3/M7 (§19.9): a bad tick on the authority's own feed makes a wrong final capsule claim; the replica's dispute is upheld on the provider's settled bars (claim withdrawn with reason dispute_upheld, capsule retracted: story stage retracted, no ledger row); a dispute the settled data does not support is rejected (evidence to the disputer only); duplicate and unknown disputes.",
       "init": init, "steps": steps}
print(json.dumps(out, ensure_ascii=False, indent=2))
