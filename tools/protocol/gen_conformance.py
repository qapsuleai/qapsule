#!/usr/bin/env python3
"""Writes the qp/0 conformance transcripts (fixtures/protocol/conformance/*.json).

The transcripts are the language-independent contract of docs/PROTOCOL.md. They are
written from the spec, not from the implementation. Numbers that come from the engine
(study values) were taken from `qp compute` on fixtures/bars/sample.json and are
asserted exactly. Run from the repository root:

    python3 tools/protocol/gen_conformance.py
"""

import json
import os

OUT = "fixtures/protocol/conformance"
INIT = "fixtures/protocol/init/thyao.json"
T0 = 1760000000000
BAR_T = {80: 1745452800000, 140: 1752710400000, 150: 1753920000000, 199: 1759881600000}
DAY = 86400000

PROFILES = {
    "desktop": {"depth": "chart", "series_data": True, "max_points": 500, "max_glyphs": 6, "speech": False},
    "phone": {"depth": "capsule", "series_data": True, "max_points": 60, "max_glyphs": 3, "speech": False},
    "watch": {"depth": "sentence", "series_data": False, "max_points": 0, "max_glyphs": 1, "speech": False},
    "voice": {"depth": "sentence", "series_data": False, "max_points": 0, "max_glyphs": 0, "speech": True},
    "headless": {"depth": "chart", "series_data": True, "max_points": 10000, "max_glyphs": 6, "speech": False},
}
LIMITS = {"max_message_bytes": 1048576, "max_ops": 64, "max_bars": 10000, "max_subscriptions": 32, "max_string": 4096}
PERMS = {
    "owner": {"read": True, "write": True, "propose": True, "decide": True, "attend": True, "feed": False},
    "editor": {"read": True, "write": True, "propose": True, "decide": False, "attend": True, "feed": False},
    "proposer": {"read": True, "write": False, "propose": True, "decide": False, "attend": True, "feed": False},
    "viewer": {"read": True, "write": False, "propose": False, "decide": False, "attend": False, "feed": False},
    "feed": {"read": False, "write": False, "propose": False, "decide": False, "attend": False, "feed": True},
    "host": {"read": True, "write": True, "propose": False, "decide": False, "attend": False, "feed": True},
}
ROLE = {"arkin": "owner", "scout": "proposer", "editor": "editor", "viewer": "viewer", "feed": "feed", "host": "host"}


def send(conn, msg, expect, note=None):
    s = {"send": {"conn": conn, "msg": msg}, "expect": expect}
    if note:
        s["note"] = note
    return s


def raw(conn, line, expect, note=None):
    s = {"send_raw": {"conn": conn, "line": line}, "expect": expect}
    if note:
        s["note"] = note
    return s


def ex(conn, msg):
    return {"conn": conn, "msg": msg}


def hello(conn, principal, kind="headless", seq=0, **surface):
    surf = {"kind": kind, **surface}
    welcome = {
        "type": "welcome", "protocol": "qp/0", "doc": "doc-thyao", "principal": principal,
        "role": ROLE[principal], "permissions": PERMS[ROLE[principal]], "seq": seq, "epoch": 0,
        "profile": PROFILES[kind], "limits": LIMITS,
    }
    return send(conn, {"type": "hello", "protocol": "qp/0", "principal": principal, "surface": surf},
                [ex(conn, welcome)])


def cmd(cid, ops, **kw):
    return {"type": "command", "id": cid, "ops": ops, **kw}


def ack(cid, status="applied", **kw):
    return {"type": "ack", "id": cid, "status": status, **kw}


def query(qid, q):
    return {"type": "query", "id": qid, "query": q}


def result(qid, data, seq="$any"):
    return {"type": "result", "id": qid, "seq": seq, "data": data}


def err(cid, code, **kw):
    m = {"type": "error", "error": {"code": code, **kw}}
    if cid is not None:
        m["id"] = cid
    return m


def sub(sid, topic):
    return {"type": "subscribe", "id": sid, "topic": topic}


def notify(sid, event):
    return {"type": "notify", "sub": sid, "event": event}


def transcript(name, description, covers, steps, final=None, init=INIT):
    t = {"name": name, "description": description, "covers": covers, "init": init, "steps": steps}
    if final:
        t["final"] = final
    return t


T = []

# 01 -------------------------------------------------------------------------
T.append(transcript(
    "01-handshake",
    "hello must come first; unknown principal and wrong protocol are handshake errors; "
    "welcome carries role, permissions, profile and limits.",
    ["capability", "error"],
    [
        send("a", cmd("c1", [{"op": "remove", "id": "bb20"}]),
             [ex("a", err("c1", "handshake"))], "command before hello"),
        send("x", {"type": "hello", "protocol": "qp/0", "principal": "mallory", "surface": {"kind": "desktop"}},
             [ex("x", err(None, "handshake"))], "unknown principal"),
        send("y", {"type": "hello", "protocol": "qp/9", "principal": "arkin", "surface": {"kind": "desktop"}},
             [ex("y", err(None, "handshake", expected=["qp/0"]))], "unsupported version"),
        hello("a", "arkin", "desktop"),
        send("a", {"type": "hello", "protocol": "qp/0", "principal": "arkin", "surface": {"kind": "desktop"}},
             [ex("a", err(None, "handshake"))], "second hello"),
        hello("s", "scout", "headless"),
    ],
    final={"log_len": 0},
))

# 02 -------------------------------------------------------------------------
T.append(transcript(
    "02-add-study-same-panel",
    "G01: add RSI(7) in the panel where RSI(14) is drawn; subscribers get the change.",
    ["command", "change", "subscription"],
    [
        hello("a", "arkin", "desktop"),
        hello("s", "scout"),
        send("s", sub("s1", {"kind": "changes"}), [ex("s", ack("s1", "subscribed"))]),
        send("a", cmd("c1", [{"op": "add_study", "study": "rsi", "inputs": ["px"], "params": {"period": 7},
                              "show": {"view": "main", "with": "rsi14"}}]),
             [ex("a", ack("c1", seq=1, created=["series-1-0"])),
              ex("s", notify("s1", {"kind": "change", "seq": 1, "at": T0, "author": "arkin", "cause": "c1",
                                    "changes": [
                                        {"op": "put_series", "series": {"id": "series-1-0", "source": {
                                            "kind": "study", "study": "rsi", "inputs": ["px"],
                                            "params": {"period": 7.0}}}},
                                        {"op": "insert_layer", "view": "main", "index": 5,
                                         "layer": {"mark": "line", "series": "series-1-0", "panel": 1}}]}))]),
    ],
    final={"log_len": 1, "document": {"views": [{"id": "main", "layers": [
        {"series": "px"}, {"series": "bb20"}, {"series": "rsi14"}, {"mark": "rule"}, {"series": "macd"},
        {"mark": "line", "series": "series-1-0", "panel": 1}]}]}},
))

# 03 -------------------------------------------------------------------------
T.append(transcript(
    "03-errors-self-correction",
    "Errors carry enough to fix the message: parse, unknown field with did_you_mean, "
    "unknown id with did_you_mean, invalid parameter; a failing command applies nothing.",
    ["error", "command"],
    [
        hello("s", "editor"),
        raw("s", "{not json", [ex("s", err(None, "parse"))]),
        raw("s", json.dumps(cmd("c1", [{"op": "add_study", "study": "rsi", "inputs": ["px"], "parms": {"period": 7}}])),
            [ex("s", err("c1", "schema", did_you_mean=["params"]))], "typo in field name (sent raw: not schema-valid)"),
        raw("s", json.dumps(cmd("c1b", [{"op": "add_indicator", "study": "rsi"}])),
            [ex("s", err("c1b", "schema"))], "unknown op"),
        send("s", cmd("c2", [{"op": "add_study", "study": "rsi", "inputs": ["px"], "params": {"period": 7},
                              "show": {"view": "main", "with": "rsi"}}]),
             [ex("s", err("c2", "not_found", path="/ops/0/show/with", did_you_mean=["rsi14", "px"]))],
             "unknown series id"),
        send("s", cmd("c3", [{"op": "add_study", "study": "rsi", "inputs": ["px"], "params": {"period": 0}}]),
             [ex("s", err("c3", "invalid"))], "period must be >= 1"),
        send("s", cmd("c4", [{"op": "set_params", "series": "macd", "params": {"fast": 8}},
                             {"op": "remove", "id": "nope"}]),
             [ex("s", err("c4", "not_found", path="/ops/1/id"))], "all or nothing"),
        send("s", query("q1", {"kind": "get", "id": "macd"}),
             [ex("s", result("q1", {"kind": "item", "collection": "series", "item": {
                 "id": "macd", "source": {"params": {"fast": 12.0, "slow": 26.0, "signal": 9.0}}}}, seq=0))]),
    ],
    final={"log_len": 0},
))

# 04 -------------------------------------------------------------------------
T.append(transcript(
    "04-reject-finding",
    "G03: a person rejects a finding with a reason; capsule becomes rejected and the "
    "ledger records it with the note and author.",
    ["command", "change", "provenance"],
    [
        hello("a", "arkin", "desktop"),
        send("a", cmd("c1", [{"op": "reject_finding", "finding": "fnd-overbought", "note": "hacim teyidi yok"}]),
             [ex("a", ack("c1", seq=1))]),
        send("a", query("q1", {"kind": "log"}),
             [ex("a", result("q1", {"kind": "log", "entries": [
                 {"seq": 1, "author": "arkin", "changes": [
                     {"op": "set_capsule_state", "capsule": "cap-overbought", "state": "rejected"},
                     {"op": "push_ledger", "entry": {"finding": "fnd-overbought", "state": "rejected",
                                                     "outcome": "pending", "recorded_at": T0,
                                                     "note": "hacim teyidi yok"}}]}]}, seq=1))]),
    ],
    final={"log_len": 1, "document": {
        "capsules": [{"id": "cap-overbought", "state": "rejected"}],
        "ledger": {"entries": [{"finding": "fnd-overbought", "state": "rejected", "note": "hacim teyidi yok"}]}}},
))

# 05 -------------------------------------------------------------------------
NEW_FINDING = {"op": "add_finding", "claim": "Momentum güçlü: RSI 70'i yukarı kesti.",
               "facts": ["fact-rsi-cross70"], "confidence": 0.6,
               "invalidation": {"kind": "threshold", "series": "rsi14", "op": "cross_below", "value": 70.0},
               "capsule": "Momentum güçlü"}
T.append(transcript(
    "05-proposal-accept",
    "G08+G09: an agent without write permission writes a finding; it becomes a proposal; "
    "the owner lists and accepts it; provenance shows proposer and approver.",
    ["proposal", "command", "provenance", "subscription"],
    [
        hello("a", "arkin", "desktop"),
        hello("s", "scout"),
        send("a", sub("p", {"kind": "proposals"}), [ex("a", ack("p", "subscribed"))]),
        send("s", cmd("c1", [NEW_FINDING], reason="RSI 70'i yukarı kesti"),
             [ex("s", ack("c1", "proposed", seq=1, proposal="prop-1", created=["fnd-1-0", "cap-1-1"])),
              ex("a", notify("p", {"kind": "proposal", "proposal": "prop-1", "state": "pending",
                                   "author": "scout", "seq": 1}))]),
        send("a", query("q1", {"kind": "list", "collection": "proposals", "state": "pending"}),
             [ex("a", result("q1", {"kind": "items", "collection": "proposals", "items": [
                 {"id": "prop-1", "state": "pending", "reason": "RSI 70'i yukarı kesti",
                  "provenance": {"author": {"kind": "agent", "name": "scout"}}}]}, seq=1))]),
        send("a", query("q2", {"kind": "get", "id": "fnd-1-0"}),
             [ex("a", err("q2", "not_found"))], "not in the document yet"),
        send("a", cmd("c2", [{"op": "accept_proposal", "proposal": "prop-1"}]),
             [ex("a", ack("c2", seq=2, created=["fnd-1-0", "cap-1-1"])),
              ex("a", notify("p", {"kind": "proposal", "proposal": "prop-1", "state": "accepted",
                                   "author": "scout", "seq": 2}))]),
        send("a", query("q3", {"kind": "log", "after_seq": 1}),
             [ex("a", result("q3", {"kind": "log", "entries": [
                 {"seq": 2, "author": "scout", "approved_by": "arkin"}]}, seq=2))]),
        send("a", cmd("c3", [{"op": "accept_proposal", "proposal": "prop-1"}]),
             [ex("a", err("c3", "not_pending"))]),
    ],
    final={"log_len": 2, "document": {
        "findings": [{"id": "fnd-overbought"}, {"id": "fnd-1-0", "provenance": {
            "author": {"kind": "agent", "name": "scout"}, "approved_by": {"kind": "human", "name": "arkin"}}}],
        "capsules": [{"id": "cap-overbought"}, {"id": "cap-1-1", "finding": "fnd-1-0", "state": "live",
                                                 "headline": "Momentum güçlü"}],
        "proposals": [{"id": "prop-1", "state": "accepted", "decided_by": {"kind": "human", "name": "arkin"},
                       "decided_seq": 2}]}},
))

# 06 -------------------------------------------------------------------------
INIT_TTL = {"document": "fixtures/protocol/docs/thyao.json", "clock": T0, "epoch": 0, "proposal_ttl_ms": 60000,
            "principals": json.load(open(INIT))["principals"]}
T.append(transcript(
    "06-proposal-reject-expire",
    "Proposal rejected with a note; another one expires when the host clock passes its "
    "TTL; deciding an expired proposal fails; a proposer cannot decide.",
    ["proposal", "time", "error"],
    [
        hello("a", "arkin", "desktop"),
        hello("s", "scout"),
        hello("h", "host"),
        send("s", sub("p", {"kind": "proposals"}), [ex("s", ack("p", "subscribed"))]),
        send("s", cmd("c1", [{"op": "set_params", "series": "rsi14", "params": {"period": 21}}]),
             [ex("s", ack("c1", "proposed", seq=1, proposal="prop-1")),
              ex("s", notify("p", {"kind": "proposal", "proposal": "prop-1", "state": "pending",
                                   "author": "scout", "seq": 1}))]),
        send("s", cmd("c2", [{"op": "accept_proposal", "proposal": "prop-1"}]),
             [ex("s", err("c2", "forbidden"))], "proposers cannot decide, and deciding is not proposable"),
        send("a", cmd("c3", [{"op": "reject_proposal", "proposal": "prop-1", "note": "14 kalsın"}]),
             [ex("a", ack("c3", seq=2)),
              ex("s", notify("p", {"kind": "proposal", "proposal": "prop-1", "state": "rejected",
                                   "author": "scout", "seq": 2}))]),
        send("s", cmd("c4", [{"op": "set_params", "series": "rsi14", "params": {"period": 9}}]),
             [ex("s", ack("c4", "proposed", seq=3, proposal="prop-3")),
              ex("s", notify("p", {"kind": "proposal", "proposal": "prop-3", "state": "pending",
                                   "author": "scout", "seq": 3}))]),
        send("h", {"type": "clock", "now": T0 + 30000}, [], "not yet expired"),
        send("h", {"type": "clock", "now": T0 + 61000},
             [ex("s", notify("p", {"kind": "proposal", "proposal": "prop-3", "state": "expired",
                                   "author": "scout", "seq": 4}))]),
        send("a", cmd("c5", [{"op": "accept_proposal", "proposal": "prop-3"}]),
             [ex("a", err("c5", "not_pending"))]),
        send("h", {"type": "clock", "now": T0}, [ex("h", err(None, "invalid"))], "clock must not go back"),
    ],
    init=INIT_TTL,
    final={"log_len": 4, "document": {
        "series": [{"id": "px"}, {"id": "rsi14", "source": {"params": {"period": 14.0}}}, {"id": "macd"},
                   {"id": "bb20"}],
        "proposals": [{"id": "prop-1", "state": "rejected", "note": "14 kalsın"},
                      {"id": "prop-3", "state": "expired", "decided_seq": 4}]}},
))

# 07 -------------------------------------------------------------------------
T.append(transcript(
    "07-undo-redo-conflict",
    "G10: undo the speaker's own last change without touching others' work; undo refuses "
    "when a later entry by someone else touched the same item; undoing an undo redoes.",
    ["command", "undo", "error"],
    [
        hello("a", "arkin", "desktop"),
        hello("e", "editor"),
        send("a", cmd("c1", [{"op": "add_study", "id": "sma20", "study": "sma", "inputs": ["px"],
                              "params": {"period": 20}, "show": {"view": "main", "panel": 0}}]),
             [ex("a", ack("c1", seq=1, created=["sma20"]))]),
        send("e", cmd("c2", [{"op": "set_params", "series": "macd", "params": {"fast": 8}}]),
             [ex("e", ack("c2", seq=2))]),
        send("a", cmd("c3", [{"op": "undo"}]), [ex("a", ack("c3", seq=3))], "undoes seq 1, not seq 2"),
        send("a", query("q1", {"kind": "get", "id": "sma20"}), [ex("a", err("q1", "not_found"))]),
        send("a", query("q2", {"kind": "get", "id": "macd"}),
             [ex("a", result("q2", {"kind": "item", "collection": "series",
                                    "item": {"source": {"params": {"fast": 8.0}}}}, seq=3))]),
        send("a", cmd("c4", [{"op": "set_params", "series": "rsi14", "params": {"period": 10}}]),
             [ex("a", ack("c4", seq=4))]),
        send("e", cmd("c5", [{"op": "set_params", "series": "rsi14", "params": {"period": 12}}]),
             [ex("e", ack("c5", seq=5))]),
        send("a", cmd("c6", [{"op": "undo"}]),
             [ex("a", err("c6", "undo_conflict", conflicts=[5]))], "editor changed rsi14 after seq 4"),
        send("a", cmd("c7", [{"op": "undo", "seq": 3}]), [ex("a", ack("c7", seq=6))], "redo the sma20"),
        send("e", cmd("c8", [{"op": "undo", "seq": 1}]), [ex("e", err("c8", "forbidden"))],
             "only the owner may undo someone else's entry"),
        send("a", query("q3", {"kind": "log", "after_seq": 2}),
             [ex("a", result("q3", {"kind": "log", "entries": [
                 {"seq": 3, "author": "arkin", "undoes": 1}, {"seq": 4}, {"seq": 5},
                 {"seq": 6, "author": "arkin", "undoes": 3}]}, seq=6))]),
    ],
    final={"log_len": 6, "document": {"series": [
        {"id": "px"}, {"id": "rsi14", "source": {"params": {"period": 12.0}}},
        {"id": "macd", "source": {"params": {"fast": 8.0}}}, {"id": "bb20"}, {"id": "sma20"}]}},
))

# 08 -------------------------------------------------------------------------
T.append(transcript(
    "08-offline-base-conflict",
    "K21-2 scenario: an agent edits offline from base 0 and comes back; its write to an "
    "item changed meanwhile is refused with the conflicting seq; a non-conflicting write "
    "from the same base applies; resending with the new base applies.",
    ["command", "concurrency", "error"],
    [
        hello("a", "arkin", "desktop"),
        hello("e", "editor"),
        send("a", cmd("c1", [{"op": "set_params", "series": "macd", "params": {"fast": 10}}]),
             [ex("a", ack("c1", seq=1))]),
        send("e", cmd("c2", [{"op": "set_params", "series": "macd", "params": {"fast": 8, "slow": 21, "signal": 5}}],
                      base=0),
             [ex("e", err("c2", "conflict", conflicts=[1]))]),
        send("e", cmd("c3", [{"op": "add_layer", "view": "main",
                              "layer": {"mark": "rule", "panel": 1, "encoding": {"y": "30"}}}], base=0),
             [ex("e", ack("c3", seq=2))], "view `main` unchanged since base 0"),
        send("e", cmd("c4", [{"op": "set_params", "series": "macd", "params": {"fast": 8, "slow": 21, "signal": 5}}],
                      base=1),
             [ex("e", ack("c4", seq=3))], "G14: editor writes directly"),
        send("e", cmd("c5", [{"op": "set_params", "series": "macd", "params": {"fast": 9}}], base=99),
             [ex("e", err("c5", "invalid", path="/base"))], "a base from the future is refused (audit O2)"),
    ],
    final={"log_len": 3, "document": {"series": [
        {"id": "px"}, {"id": "rsi14"}, {"id": "macd", "source": {"params": {"fast": 8.0, "slow": 21.0, "signal": 5.0}}},
        {"id": "bb20"}]}},
))

# 09 -------------------------------------------------------------------------
T.append(transcript(
    "09-queries",
    "G06, G15, G02: reduced study query, everything at one bar, log by time.",
    ["query", "time"],
    [
        hello("a", "arkin", "desktop"),
        hello("s", "scout"),
        hello("h", "host"),
        send("s", query("q1", {"kind": "study", "series": "rsi14", "range": {"last": 50}, "reduce": "max"}),
             [ex("s", result("q1", {"kind": "reduced", "series": "rsi14", "reduce": "max", "bars": 50,
                                    "values": {"rsi": {"value": 78.11504477458953, "bar": 154}}}, seq=0))]),
        send("s", query("q2", {"kind": "at", "series": "px", "bar": 150}),
             [ex("s", result("q2", {"kind": "at", "at": {
                 "series": "px", "bar": 150,
                 "ohlcv": {"t": BAR_T[150], "o": 115.97, "h": 118.24, "l": 115.76, "c": 117.66, "v": 2081563.81},
                 "studies": {
                     "rsi14": {"rsi": 72.83959265032422},
                     "macd": {"histogram": 0.24850564697777333, "macd": 2.476200324624031,
                              "signal": 2.227694677646258},
                     "bb20": {"bandwidth": 0.0878235386233608, "lower": 107.63450083442969,
                              "middle": 112.57800000000005, "upper": 117.5214991655704}}}}, seq=0))]),
        send("s", query("q3", {"kind": "study", "series": "px", "range": {"from_bar": 198, "to_bar": 199},
                               "output": "c"}),
             [ex("s", result("q3", {"kind": "values", "values": {"series": "px", "from_bar": 198,
                                                                  "outputs": {"c": ["$any", 124.69]}}}, seq=0))]),
        send("a", cmd("c1", [{"op": "set_view_range", "view": "main", "from_bar": 140, "to_bar": 199}]),
             [ex("a", ack("c1", seq=1))]),
        send("h", {"type": "clock", "now": T0 + 20 * 60000}, []),
        send("a", cmd("c2", [{"op": "remove", "id": "bb20"}]), [ex("a", ack("c2", seq=2))]),
        send("h", {"type": "clock", "now": T0 + 40 * 60000}, []),
        send("s", query("q4", {"kind": "log", "since_t": T0 + 10 * 60000}),
             [ex("s", result("q4", {"kind": "log", "entries": [
                 {"seq": 2, "at": T0 + 20 * 60000, "author": "arkin"}]}, seq=2))],
             "G02: what changed in the last 30 minutes"),
        send("s", query("q5", {"kind": "study", "series": "rsi14", "range": {"from_bar": 10, "to_bar": 5}}),
             [ex("s", err("q5", "invalid"))]),
    ],
    final={"log_len": 2},
))

# 10 -------------------------------------------------------------------------
NEW_BAR = {"t": BAR_T[199] + DAY, "o": 124.0, "h": 124.5, "l": 98.0, "c": 99.0, "v": 3000000.0}
T.append(transcript(
    "10-live-invalidation",
    "G19 then G18: invalidation changed to close < 100; a subscriber waits; the feed "
    "appends a bar closing at 99; the engine invalidates the capsule in its own entry; "
    "finding and condition subscribers are notified in order.",
    ["subscription", "command", "live", "engine"],
    [
        hello("a", "arkin", "desktop"),
        hello("s", "scout"),
        hello("f", "feed"),
        send("a", cmd("c1", [{"op": "update_finding", "finding": "fnd-overbought",
                              "invalidation": {"kind": "threshold", "series": "px", "op": "lt", "value": 100.0}}]),
             [ex("a", ack("c1", seq=1))], "close is 124.69: still live"),
        send("s", sub("w", {"kind": "finding", "finding": "fnd-overbought"}), [ex("s", ack("w", "subscribed"))]),
        send("s", sub("k", {"kind": "condition", "condition": {"kind": "threshold", "series": "px", "op": "lt",
                                                                "value": 100.0}}),
             [ex("s", ack("k", "subscribed"))]),
        send("f", cmd("b1", [{"op": "append_bars", "series": "px", "bars": [NEW_BAR]}]),
             [ex("f", ack("b1", seq=2)),
              ex("s", notify("w", {"kind": "finding_state", "finding": "fnd-overbought",
                                   "capsule": "cap-overbought", "state": "invalidated", "seq": 3})),
              ex("s", notify("k", {"kind": "condition_met", "bar": 200, "t": BAR_T[199] + DAY, "seq": 2}))]),
        send("s", query("q1", {"kind": "log", "after_seq": 2}),
             [ex("s", result("q1", {"kind": "log", "entries": [
                 {"seq": 3, "author": "qp", "changes": [
                     {"op": "set_capsule_state", "capsule": "cap-overbought", "state": "invalidated"}]}]}, seq=3))]),
        send("f", cmd("b2", [{"op": "set_params", "series": "rsi14", "params": {"period": 9}}]),
             [ex("f", err("b2", "forbidden"))], "a feed only appends data"),
    ],
    final={"log_len": 3, "document": {"capsules": [{"id": "cap-overbought", "state": "invalidated"}],
                                      "findings": [{"id": "fnd-overbought", "invalidation": {
                                          "kind": "threshold", "series": "px", "op": "lt", "value": 100.0}}]}},
))

# 11 -------------------------------------------------------------------------
T.append(transcript(
    "11-capability-depths",
    "Criterion 10, G04, G20: the same finding presented to watch, phone, desktop and voice "
    "surfaces comes back at three different depths.",
    ["capability", "query"],
    [
        hello("w", "arkin", "watch", screen=[198, 242], input=["touch", "crown"]),
        hello("p", "arkin", "phone", screen=[390, 844], input=["touch"]),
        hello("d", "arkin", "desktop", screen=[1600, 1000], input=["pointer", "keyboard"]),
        hello("v", "scout", "voice", input=["voice"]),
        send("w", query("q1", {"kind": "present", "finding": "fnd-overbought"}),
             [ex("w", result("q1", {"kind": "presentation", "presentation": {
                 "finding": "fnd-overbought", "depth": "sentence", "surface": "watch",
                 "text": "THYAO aşırı alımda: RSI 70'in üstünde.", "state": "live"}}, seq=0))]),
        send("p", query("q2", {"kind": "present", "finding": "fnd-overbought"}),
             [ex("p", result("q2", {"kind": "presentation", "presentation": {
                 "finding": "fnd-overbought", "depth": "capsule", "surface": "phone", "confidence": 0.65,
                 "glyphs": [{"kind": "direction", "value": 0.6}, {"kind": "extremity", "value": 0.8},
                            {"kind": "uncertainty", "value": 0.35}],
                 "spark": {"series": "rsi14", "from_bar": 140}}}, seq=0))]),
        send("d", query("q3", {"kind": "present", "finding": "fnd-overbought"}),
             [ex("d", result("q3", {"kind": "presentation", "presentation": {
                 "finding": "fnd-overbought", "depth": "chart", "surface": "desktop",
                 "facts": [{"id": "fact-rsi-cross70"}], "view": "$any",
                 "spark": {"series": "rsi14", "from_bar": 0}}}, seq=0))]),
        send("v", query("q4", {"kind": "present"}),
             [ex("v", result("q4", {"kind": "presentation", "presentation": {
                 "finding": "fnd-overbought", "depth": "sentence", "surface": "voice",
                 "text": "THYAO aşırı alımda: RSI 70'in üstünde."}}, seq=0))],
             "G20: no finding given -> most important live finding"),
        send("d", query("q5", {"kind": "present", "finding": "fnd-overbought", "surface": "watch"}),
             [ex("d", result("q5", {"kind": "presentation", "presentation": {
                 "depth": "sentence", "surface": "watch"}}, seq=0))], "present for another surface"),
    ],
    final={"log_len": 0},
))

# 12 -------------------------------------------------------------------------
T.append(transcript(
    "12-attention-input",
    "G11, G12: the user's viewport follows input; an agent reads where the user looks "
    "and sends an ordered two-step attention directive; the document does not change.",
    ["input", "attention", "subscription"],
    [
        hello("a", "arkin", "desktop"),
        hello("s", "scout"),
        send("a", sub("me", {"kind": "attention"}), [ex("a", ack("me", "subscribed"))]),
        send("s", sub("watch", {"kind": "attention", "principal": "arkin"}), [ex("s", ack("watch", "subscribed"))]),
        send("s", query("q0", {"kind": "attention", "principal": "arkin"}),
             [ex("s", result("q0", {"kind": "attention", "attention": {
                 "principal": "arkin", "viewport": {"view": "main", "from_bar": 80, "to_bar": 199}}}, seq=0))],
             "initial viewport: last 120 bars on desktop"),
        send("a", {"type": "input", "id": "i1", "gesture": {"kind": "look", "view": "main", "from_bar": 100,
                                                            "to_bar": 199}},
             [ex("a", result("i1", {"kind": "viewport", "viewport": {"view": "main", "from_bar": 100,
                                                                     "to_bar": 199}}, seq=0)),
              ex("a", notify("me", {"kind": "viewport", "principal": "arkin",
                                    "viewport": {"view": "main", "from_bar": 100, "to_bar": 199}})),
              ex("s", notify("watch", {"kind": "viewport", "principal": "arkin",
                                       "viewport": {"view": "main", "from_bar": 100, "to_bar": 199}}))]),
        send("a", {"type": "input", "id": "i2", "gesture": {"kind": "zoom", "view": "main", "factor": 2.0}},
             [ex("a", result("i2", {"kind": "viewport", "viewport": {"view": "main", "from_bar": 150,
                                                                     "to_bar": 199}}, seq=0)),
              ex("a", notify("me", {"kind": "viewport", "principal": "arkin",
                                    "viewport": {"from_bar": 150, "to_bar": 199}})),
              ex("s", notify("watch", {"kind": "viewport", "principal": "arkin",
                                       "viewport": {"from_bar": 150, "to_bar": 199}}))]),
        send("a", {"type": "input", "id": "i3", "gesture": {"kind": "pan", "view": "main", "bars": 30}},
             [ex("a", result("i3", {"kind": "viewport", "viewport": {"from_bar": 150, "to_bar": 199}}, seq=0)),
              ex("a", notify("me", {"kind": "viewport", "principal": "arkin", "viewport": {"from_bar": 150}})),
              ex("s", notify("watch", {"kind": "viewport", "principal": "arkin", "viewport": {"from_bar": 150}}))],
             "clamped at the last bar"),
        send("a", {"type": "input", "id": "i4", "gesture": {"kind": "tap", "view": "main", "panel": 1, "bar": 150}},
             [ex("a", result("i4", {"kind": "at", "at": {"series": "px", "bar": 150,
                                                         "studies": {"rsi14": {"rsi": 72.83959265032422}}}}, seq=0))]),
        send("s", query("q1", {"kind": "attention", "principal": "arkin"}),
             [ex("s", result("q1", {"kind": "attention", "attention": {
                 "principal": "arkin", "viewport": {"view": "main", "from_bar": 150, "to_bar": 199},
                 "focus": {"view": "main", "panel": 1, "from_bar": 150, "to_bar": 150},
                 "directives": []}}, seq=0))], "G11"),
        send("s", {"type": "attend", "id": "a1", "directive": {"to": "arkin", "steps": [
            {"target": {"view": "main", "panel": 1, "series": "rsi14", "from_bar": 150, "to_bar": 150},
             "say": "RSI burada 70'i kesti."},
            {"target": {"view": "main", "panel": 0, "series": "px", "from_bar": 199, "to_bar": 199},
             "say": "Son barın hacmi."}]}},
             [ex("s", ack("a1", "delivered", created=["att-1"])),
              ex("a", notify("me", {"kind": "attention", "directive": {
                  "id": "att-1", "to": "arkin", "from": "scout", "at": T0,
                  "steps": [{"target": {"series": "rsi14", "from_bar": 150}},
                            {"target": {"series": "px", "from_bar": 199}}]}})),
              ex("s", notify("watch", {"kind": "attention", "directive": {"id": "att-1", "from": "scout"}}))],
             "G12"),
        send("s", {"type": "attend", "id": "a2", "directive": {"to": "nobody", "steps": []}},
             [ex("s", err("a2", "not_found"))]),
    ],
    final={"log_len": 0},
))

# 13 -------------------------------------------------------------------------
NARR = {"op": "add_narrative", "about": "fnd-overbought", "steps": [
    {"at_ms": 0, "target": {"view": "main", "panel": 0, "series": "px", "from_bar": 130, "to_bar": 150},
     "say": "Fiyat yükseliyor."},
    {"at_ms": 4000, "target": {"view": "main", "panel": 1, "series": "rsi14", "from_bar": 150, "to_bar": 150},
     "say": "RSI 70'i kesti."},
    {"at_ms": 8000, "target": {"view": "main", "panel": 0, "series": "bb20", "from_bar": 150, "to_bar": 150},
     "say": "Fiyat üst banda yakın."}]}
BAD_NARR = json.loads(json.dumps(NARR))
BAD_NARR["steps"][2]["at_ms"] = 1000
T.append(transcript(
    "13-narrative-timeline",
    "G13: a three-step narration about a finding, proposed by an agent and accepted; "
    "steps out of order are refused when the proposal is made.",
    ["narrative", "proposal", "time"],
    [
        hello("a", "arkin", "desktop"),
        hello("s", "scout"),
        send("s", cmd("c0", [BAD_NARR]), [ex("s", err("c0", "invalid", path="/ops/0/steps/2/at_ms"))],
             "validated before the proposal exists"),
        send("s", cmd("c1", [NARR]), [ex("s", ack("c1", "proposed", seq=1, proposal="prop-1",
                                                   created=["narr-1-0"]))]),
        send("a", cmd("c2", [{"op": "accept_proposal", "proposal": "prop-1"}]),
             [ex("a", ack("c2", seq=2, created=["narr-1-0"]))]),
        send("s", query("q1", {"kind": "list", "collection": "narratives"}),
             [ex("s", result("q1", {"kind": "items", "collection": "narratives", "items": [
                 {"id": "narr-1-0", "about": "fnd-overbought",
                  "steps": [{"at_ms": 0}, {"at_ms": 4000}, {"at_ms": 8000}],
                  "provenance": {"author": {"kind": "agent", "name": "scout"},
                                 "approved_by": {"kind": "human", "name": "arkin"}}}]}, seq=2))]),
    ],
    final={"log_len": 2},
))

# 14 -------------------------------------------------------------------------
T.append(transcript(
    "14-view-range-remove-cascade",
    "G07, G16: persistent view range by bars; removing a series removes its layers "
    "(highest index first) then the series; removing a series others depend on is refused.",
    ["command", "change", "error"],
    [
        hello("a", "arkin", "desktop"),
        hello("s", "scout"),
        send("s", sub("c", {"kind": "changes", "ids": ["main"]}), [ex("s", ack("c", "subscribed"))]),
        send("a", cmd("c1", [{"op": "set_view_range", "view": "main", "from_bar": 140, "to_bar": 199}]),
             [ex("a", ack("c1", seq=1)),
              ex("s", notify("c", {"kind": "change", "seq": 1, "author": "arkin", "changes": [
                  {"op": "set_x_domain", "view": "main", "domain": [float(BAR_T[140]), float(BAR_T[199])]}]}))]),
        send("a", cmd("c2", [{"op": "remove", "id": "bb20"}]),
             [ex("a", ack("c2", seq=2)),
              ex("s", notify("c", {"kind": "change", "seq": 2, "changes": [
                  {"op": "delete_layer", "view": "main", "index": 1}, {"op": "delete", "id": "bb20"}]}))]),
        send("a", cmd("c3", [{"op": "remove", "id": "px"}]),
             [ex("a", err("c3", "in_use", expected=["macd", "rsi14"]))]),
        send("a", cmd("c4", [{"op": "set_view_range", "view": "main", "from_bar": 150, "to_bar": 400}]),
             [ex("a", err("c4", "invalid", path="/ops/0/to_bar"))]),
    ],
    final={"log_len": 2, "document": {
        "series": [{"id": "px"}, {"id": "rsi14"}, {"id": "macd"}],
        "views": [{"id": "main", "x": {"kind": "time", "domain": [float(BAR_T[140]), float(BAR_T[199])]},
                   "layers": [{"series": "px"}, {"series": "rsi14"}, {"mark": "rule"}, {"series": "macd"}]}]}},
))

# 15 -------------------------------------------------------------------------
T.append(transcript(
    "15-authority-intent",
    "G05 and roles: SMA20/SMA50 with a crossing intent in one command; a viewer cannot "
    "write or propose; a proposer in direct mode is refused with a hint; ids are unique.",
    ["command", "authority", "error"],
    [
        hello("a", "arkin", "desktop"),
        hello("s", "scout"),
        hello("v", "viewer", "phone"),
        send("a", cmd("c1", [
            {"op": "add_study", "id": "sma20", "study": "sma", "inputs": ["px"], "params": {"period": 20},
             "show": {"view": "main", "panel": 0}},
            {"op": "add_study", "id": "sma50", "study": "sma", "inputs": ["px"], "params": {"period": 50},
             "show": {"view": "main", "panel": 0}},
            {"op": "add_intent", "text": "SMA20 SMA50'yi yukarı keserse haber ver",
             "trigger": {"kind": "compare", "left": "sma20", "op": "cross_above", "right": "sma50"}}]),
             [ex("a", ack("c1", seq=1, created=["sma20", "sma50", "intent-1-2"]))]),
        send("v", cmd("c2", [{"op": "remove", "id": "sma20"}]), [ex("v", err("c2", "forbidden"))]),
        send("s", cmd("c3", [{"op": "remove", "id": "sma20"}], mode="direct"),
             [ex("s", err("c3", "forbidden", hint="$any"))]),
        send("a", cmd("c4", [{"op": "add_study", "id": "macd", "study": "ema", "inputs": ["px"]}]),
             [ex("a", err("c4", "duplicate", path="/ops/0/id"))]),
        send("a", cmd("c5", [{"op": "add_intent", "text": "x",
                              "trigger": {"kind": "compare", "left": "sma20", "op": "gt", "right": "sma100"}}]),
             [ex("a", err("c5", "not_found", path="/ops/0/trigger/right"))]),
    ],
    final={"log_len": 1, "document": {"intents": [{"id": "intent-1-2", "active": True, "trigger": {
        "kind": "compare", "left": "sma20", "op": "cross_above", "right": "sma50"},
        "provenance": {"author": {"kind": "human", "name": "arkin"}}}]}},
))

# 16 -------------------------------------------------------------------------
T.append(transcript(
    "16-screen-describe-presenter",
    "G17: an agent reads what is on the user's screen semantically; describe lists what "
    "the endpoint offers; the presenter builds a view for a finding deterministically.",
    ["query", "capability", "presenter"],
    [
        hello("a", "arkin", "desktop"),
        hello("s", "scout"),
        hello("e", "editor"),
        send("s", query("q1", {"kind": "screen", "principal": "arkin"}),
             [ex("s", result("q1", {"kind": "screen", "screens": [{
                 "principal": "arkin", "surface": "desktop",
                 "viewport": {"view": "main", "from_bar": 80, "to_bar": 199},
                 "symbol": "THYAO", "interval": "1d",
                 "panels": [{"panel": 0, "series": ["px", "bb20"], "marks": ["candle", "line"]},
                            {"panel": 1, "series": ["rsi14"], "marks": ["line", "rule"]},
                            {"panel": 2, "series": ["macd"], "marks": ["line"]}],
                 "capsules": ["cap-overbought"], "last_close": 124.69}]}, seq=0))]),
        send("s", query("q2", {"kind": "describe"}),
             [ex("s", result("q2", {"kind": "description", "description": {
                 "protocol": "qp/0", "doc": "doc-thyao", "role": "proposer", "permissions": PERMS["proposer"],
                 "studies": {"rsi": {"period": 14.0}, "macd": {"fast": 12.0, "slow": 26.0, "signal": 9.0},
                             "sma": {"period": 20.0}, "ema": {"period": 20.0},
                             "bollinger": {"period": 20.0, "k": 2.0}}}}, seq=0))]),
        send("e", cmd("c1", [{"op": "show_finding", "finding": "fnd-overbought", "view": "v-overbought"}]),
             [ex("e", ack("c1", seq=1, created=["v-overbought"]))]),
        send("e", query("q3", {"kind": "get", "id": "v-overbought"}),
             [ex("e", result("q3", {"kind": "item", "collection": "views", "item": {
                 "id": "v-overbought",
                 "x": {"kind": "time", "domain": [1750291200000.0, float(BAR_T[199])]},
                 "layers": [{"mark": "candle", "series": "px", "panel": 0},
                            {"mark": "line", "series": "rsi14", "panel": 1},
                            {"mark": "rule", "panel": 1, "encoding": {"y": "70"}}]}}, seq=1))]),
    ],
    final={"log_len": 1, "document": {"capsules": [{"id": "cap-overbought", "view": "v-overbought"}]}},
))

# 17 -------------------------------------------------------------------------
T.append(transcript(
    "17-now-lists-undo-ack",
    "v2 (from the agent-writability measurement): welcome and results carry the "
    "authority's `now`; `log` takes `within_ms`; `list` of series omits bar data unless "
    "`full`; an applied undo's ack says which entry it undid.",
    ["capability", "query", "time", "undo"],
    [
        send("h", {"type": "hello", "protocol": "qp/0", "principal": "host", "surface": {"kind": "headless"}},
             [ex("h", {"type": "welcome", "now": T0})]),
        hello("a", "arkin", "desktop"),
        send("a", cmd("c1", [{"op": "set_params", "series": "rsi14", "params": {"period": 10}}]),
             [ex("a", ack("c1", seq=1))]),
        send("h", {"type": "clock", "now": T0 + 45 * 60000}, []),
        send("a", cmd("c2", [{"op": "set_params", "series": "macd", "params": {"fast": 10}}]),
             [ex("a", ack("c2", seq=2))]),
        send("a", query("q1", {"kind": "log", "within_ms": 30 * 60000}),
             [ex("a", {"type": "result", "id": "q1", "seq": 2, "now": T0 + 45 * 60000,
                       "data": {"kind": "log", "entries": [{"seq": 2}]}})]),
        send("a", query("q2", {"kind": "list", "collection": "series"}),
             [ex("a", result("q2", {"kind": "items", "collection": "series", "items": [
                 {"id": "px", "data": "$absent"}, {"id": "rsi14"}, {"id": "macd"}, {"id": "bb20"}]}))]),
        send("a", cmd("c3", [{"op": "undo"}]), [ex("a", ack("c3", seq=3, undoes=2))]),
    ],
    final={"log_len": 3},
))


def main():
    os.makedirs(OUT, exist_ok=True)
    for t in T:
        path = os.path.join(OUT, t["name"] + ".json")
        with open(path, "w", encoding="utf-8") as f:
            json.dump(t, f, ensure_ascii=False, indent=1)
            f.write("\n")
    print(f"wrote {len(T)} transcripts to {OUT}")


if __name__ == "__main__":
    main()
