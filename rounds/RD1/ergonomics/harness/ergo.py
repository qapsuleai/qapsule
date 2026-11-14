#!/usr/bin/env python3
"""Agent-writability harness (RD1/T7).

Subcommands:
  run <attempt.jsonl> [--qp PATH] [--out DIR]
      Runs a hidden setup, then the agent's lines, then a hidden epilogue through
      `qp serve`. Prints ONLY the outputs addressed to the agent's connections (what a
      real agent would see). Saves the full output for the checker in DIR (default:
      next to the attempt, `<attempt>.full.jsonl`).
  check <task-id> <attempt.jsonl> [--answer FILE]
      Scores an attempt against the pre-written task criteria (tasks/TASKS.md).

The setup puts the document in the starting state of tasks/TASKS.md plus a few earlier
operations (so the log is not empty, one proposal is pending and the user has a desktop
connection), and sets "now" to the last operation + 10 minutes.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
INIT = os.path.join(ROOT, "fixtures/protocol/init/thyao.json")
QP = os.path.join(ROOT, "target/release/qp")
C = 1760000000000
MIN = 60000
NOW = C + 65 * MIN
T140, T199 = 1752710400000, 1759881600000


def h(conn, principal, kind="headless"):
    return {"conn": conn, "msg": {"type": "hello", "protocol": "qp/0", "principal": principal,
                                  "surface": {"kind": kind}}}


def clock(t):
    return {"conn": "zz-host", "msg": {"type": "clock", "now": t}}


def cmd(conn, cid, ops):
    return {"conn": conn, "msg": {"type": "command", "id": cid, "ops": ops}}


SETUP = [
    h("zz-host", "host"), h("zz-ed", "editor"), h("zz-ark", "arkin", "desktop"), h("zz-sc", "scout"),
    h("zz-feed", "feed"),
    cmd("zz-ed", "s1", [{"op": "add_layer", "view": "main",
                         "layer": {"mark": "rule", "panel": 1, "encoding": {"y": "30", "color": "#9e9e9e"}}}]),
    clock(C + 40 * MIN),
    cmd("zz-ark", "s2", [{"op": "add_study", "id": "ema50", "study": "ema", "inputs": ["px"],
                          "params": {"period": 50}, "show": {"view": "main", "panel": 0}}]),
    clock(C + 50 * MIN),
    cmd("zz-sc", "s3", [{"op": "add_finding", "claim": "Hacim son 20 günün ortalamasının üstünde.",
                         "facts": ["fact-rsi-cross70"], "confidence": 0.5,
                         "invalidation": {"kind": "threshold", "series": "px", "op": "lt", "value": 110.0},
                         "capsule": "Hacim güçlü"}]),
    clock(C + 55 * MIN),
    cmd("zz-ed", "s4", [{"op": "add_intent", "text": "RSI 30'un altına inerse haber ver",
                         "trigger": {"kind": "threshold", "series": "rsi14", "op": "lt", "value": 30.0}}]),
    {"conn": "zz-ark", "msg": {"type": "input", "id": "s5",
                               "gesture": {"kind": "look", "view": "main", "from_bar": 120, "to_bar": 199}}},
    clock(NOW),
]
SETUP_LOG_LEN = 4  # entries 1..4; the proposal (s3) is entry 3

EPILOGUE = [
    # G18: make the overbought finding's invalidation true so a subscriber is notified.
    cmd("zz-ark", "e1", [{"op": "update_finding", "finding": "fnd-overbought",
                          "invalidation": {"kind": "threshold", "series": "px", "op": "lt", "value": 1000.0}}]),
    {"conn": "zz-host", "msg": {"type": "query", "id": "e-doc", "query": {"kind": "document"}}},
    {"conn": "zz-host", "msg": {"type": "query", "id": "e-log", "query": {"kind": "log"}}},
    {"conn": "zz-host", "msg": {"type": "query", "id": "e-att", "query": {"kind": "attention", "principal": "arkin"}}},
]


def read_attempt(path):
    lines = []
    with open(path, encoding="utf-8") as f:
        for raw in f:
            raw = raw.strip()
            if raw:
                lines.append(raw)
    return lines


def agent_conns(lines):
    conns = []
    for raw in lines:
        try:
            c = json.loads(raw).get("conn")
        except (ValueError, AttributeError):
            continue
        if isinstance(c, str) and c not in conns:
            conns.append(c)
    return conns


def run(path, qp=QP, out=None, epilogue=True):
    lines = read_attempt(path)
    for raw in lines:
        try:
            c = json.loads(raw).get("conn", "")
        except (ValueError, AttributeError):
            c = ""
        if isinstance(c, str) and c.startswith("zz-"):
            raise SystemExit("connection ids starting with `zz-` are reserved by the harness")
    feed = [json.dumps(x, ensure_ascii=False) for x in SETUP] + lines
    if epilogue:
        feed += [json.dumps(x, ensure_ascii=False) for x in EPILOGUE]
    proc = subprocess.run([qp, "serve", "--init", INIT], input="\n".join(feed) + "\n",
                          capture_output=True, text=True, cwd=ROOT, timeout=120)
    outs = [l for l in proc.stdout.splitlines() if l.strip()]
    full = out or (path + ".full.jsonl")
    with open(full, "w", encoding="utf-8") as f:
        f.write("\n".join(outs) + "\n")
    mine = set(agent_conns(lines)) | {""}
    shown = []
    for l in outs:
        try:
            m = json.loads(l)
        except ValueError:
            continue
        if m.get("conn") in mine and not m.get("batch_end"):
            # Epilogue output for agent conns (e.g. a notify) is real: the agent would see it.
            shown.append(l)
    return shown, outs, proc.stderr


# --------------------------------------------------------------------------- checks

def parse_full(path):
    outs = []
    with open(path, encoding="utf-8") as f:
        for l in f:
            if l.strip():
                outs.append(json.loads(l))
    return outs


def find_result(outs, conn, rid):
    for o in outs:
        m = o.get("msg", {})
        if o.get("conn") == conn and m.get("id") == rid and m.get("type") in ("result", "ack", "error"):
            return m
    return None


def agent_msgs(lines):
    res = []
    for raw in lines:
        try:
            x = json.loads(raw)
            res.append((x.get("conn"), x.get("msg", {})))
        except ValueError:
            pass
    return res


def ctx(path):
    lines = read_attempt(path)
    outs = parse_full(path + ".full.jsonl")
    doc = (find_result(outs, "zz-host", "e-doc") or {}).get("data", {}).get("document", {})
    log = (find_result(outs, "zz-host", "e-log") or {}).get("data", {}).get("entries", [])
    att = (find_result(outs, "zz-host", "e-att") or {}).get("data", {}).get("attention", {})
    conns = set(agent_conns(lines))
    seen = [o["msg"] for o in outs if o.get("conn") in conns]
    sent = agent_msgs(lines)
    # entries written by the agent: after setup, before the epilogue
    agent_log = [e for e in log if e["seq"] > SETUP_LOG_LEN and e.get("cause") != "e1"
                 and not (e.get("author") == "qp")]
    return {"doc": doc, "log": log, "att": att, "seen": seen, "sent": sent, "agent_log": agent_log}


def series(doc, sid):
    return next((s for s in doc.get("series", []) if s["id"] == sid), None)


def layers(doc, view="main"):
    v = next((v for v in doc.get("views", []) if v["id"] == view), None)
    return v["layers"] if v else []


def studies(doc, name, **params):
    out = []
    for s in doc.get("series", []):
        src = s.get("source", {})
        if src.get("kind") == "study" and src.get("study") == name and src.get("inputs") == ["px"]:
            p = src.get("params", {})
            if all(p.get(k) == float(v) for k, v in params.items()):
                out.append(s)
    return out


def results(c, kind):
    return [m["data"] for m in c["seen"] if m.get("type") == "result" and m.get("data", {}).get("kind") == kind]


def answer_text(answer):
    """The agent's answer, with Turkish decimal commas normalized (117,66 -> 117.66)."""
    import re
    if answer and os.path.exists(answer):
        text = open(answer, encoding="utf-8").read()
        return re.sub(r"(\d),(\d)", r"\1.\2", text)
    return ""


BASE_SERIES = 5  # px, rsi14, macd, bb20 + ema50 from setup
BASE_LAYERS = 7  # 5 + rule(30) + ema50


def chk(task, c, ans):
    d = c["doc"]
    why = []

    def need(cond, msg):
        if not cond:
            why.append(msg)

    unchanged = len(c["agent_log"]) == 0
    if task == "G01":
        s7 = studies(d, "rsi", period=7)
        need(len(s7) == 1, "exactly one RSI(7) on px")
        if s7:
            ls = [l for l in layers(d) if l.get("series") == s7[0]["id"]]
            need(len(ls) == 1 and ls[0].get("panel") == 1, "RSI(7) layer in panel 1 (with RSI(14))")
        need(len(d.get("series", [])) == BASE_SERIES + 1, "nothing else added")
        need(len(layers(d)) == BASE_LAYERS + 1, "one layer added")
    elif task == "G02":
        logs = results(c, "log")
        ok = any(sorted(e["seq"] for e in r["entries"]) == [2, 3, 4] for r in logs)
        need(ok, "a log result with exactly the entries of the last 30 minutes (seq 2,3,4)")
        need(unchanged, "document must not change")
    elif task == "G03":
        cap = next((x for x in d.get("capsules", []) if x["id"] == "cap-overbought"), {})
        need(cap.get("state") == "rejected", "capsule rejected")
        ents = [e for e in d.get("ledger", {}).get("entries", []) if e.get("finding") == "fnd-overbought"]
        need(any(e.get("state") == "rejected" and "hacim" in (e.get("note") or "").lower() for e in ents),
             "ledger entry with the reason")
        need(any(e.get("author") == "arkin" for e in c["agent_log"]), "written by arkin")
    elif task == "G04":
        ps = [r["presentation"] for r in results(c, "presentation")]
        need(any(p["finding"] == "fnd-overbought" and p["depth"] == "sentence" and p["surface"] == "watch"
                 and "view" not in p and "spark" not in p for p in ps), "sentence-depth presentation for watch")
        need(unchanged, "document must not change")
    elif task == "G05":
        s20, s50 = studies(d, "sma", period=20), studies(d, "sma", period=50)
        need(bool(s20) and bool(s50), "SMA20 and SMA50 exist")
        ids20 = {s["id"] for s in s20}
        ids50 = {s["id"] for s in s50}
        ok = any(i.get("active") and i.get("trigger", {}).get("kind") == "compare"
                 and i["trigger"].get("left") in ids20 and i["trigger"].get("right") in ids50
                 and i["trigger"].get("op") == "cross_above" for i in d.get("intents", []))
        need(ok, "active intent: SMA20 cross_above SMA50")
    elif task == "G06":
        red = results(c, "reduced") + results(c, "values")
        need(bool(red), "a study query was made")
        need("78.11" in ans, "answer contains 78.115...")
        need(unchanged, "document must not change")
    elif task == "G07":
        v = next((v for v in d.get("views", []) if v["id"] == "main"), {})
        dom = v.get("x", {}).get("domain")
        vp = c["att"].get("viewport") or {}
        need(dom == [float(T140), float(T199)] or (vp.get("from_bar") == 140 and vp.get("to_bar") == 199),
             "main shows bars 140..199 (persistent domain or arkin's viewport)")
        need(all(e["changes"] and all(ch["op"] == "set_x_domain" for ch in e["changes"]) for e in c["agent_log"]),
             "nothing else changed")
    elif task == "G08":
        props = [p for p in d.get("proposals", []) if p["state"] == "pending"
                 and p["provenance"]["author"].get("name") == "scout"
                 and any(o.get("op") == "add_finding" and "fact-rsi-cross70" in o.get("facts", [])
                         for o in p["ops"])]
        need(len(props) >= 2, "a new pending proposal with the finding (besides the setup one)")
        need(len(d.get("findings", [])) == 1, "finding not in the document")
    elif task == "G09":
        p = next((p for p in d.get("proposals", []) if p["id"] == "prop-3"), {})
        need(p.get("state") == "accepted", "first pending proposal accepted")
        f = [f for f in d.get("findings", []) if f.get("provenance", {}).get("approved_by", {}).get("name") == "arkin"]
        need(len(f) == 1 and f[0]["provenance"]["author"].get("name") == "scout", "finding by scout approved by arkin")
        need(bool(results(c, "items")), "pending proposals were listed")
    elif task == "G10":
        need(series(d, "ema50") is None, "ema50 (arkin's last change) undone")
        need(any(e.get("undoes") == 2 for e in c["agent_log"]), "undo entry for seq 2")
        need(len(d.get("intents", [])) == 1, "others' later work kept")
    elif task == "G11":
        atts = [r["attention"] for r in results(c, "attention") if r["attention"]["principal"] == "arkin"]
        need(any((a.get("viewport") or {}).get("from_bar") == 120 for a in atts), "attention of arkin read")
        need("120" in ans and "199" in ans, "answer gives the range 120-199")
    elif task == "G12":
        acks = []
        for conn, m in c["sent"]:
            if m.get("type") == "attend":
                acks.append(m)
        ok = False
        for m in acks:
            st = m.get("directive", {}).get("steps", [])
            r = next((x for x in c["seen"] if x.get("id") == m.get("id")), {})
            if r.get("type") == "ack" and m["directive"].get("to") == "arkin" and len(st) == 2:
                t1, t2 = st[0]["target"], st[1]["target"]
                s1 = t1.get("series") == "rsi14" or t1.get("panel") == 1
                b1 = 150 in (t1.get("from_bar"), t1.get("to_bar"))
                b2 = 199 in (t2.get("from_bar"), t2.get("to_bar"))
                ok = ok or (s1 and b1 and b2)
        need(ok, "delivered two-step directive: RSI@150 then last bar")
        need(unchanged, "document must not change")
    elif task == "G13":
        ok = False
        for p in d.get("proposals", []) + [{"ops": [], "state": "x"}]:
            for o in p["ops"]:
                if o.get("op") == "add_narrative" and o.get("about") == "fnd-overbought":
                    st = o["steps"]
                    ser = [s["target"].get("series") for s in st]
                    ok = ok or (len(st) == 3 and ser == ["px", "rsi14", "bb20"])
        for n in d.get("narratives", []):
            ser = [s["target"].get("series") for s in n["steps"]]
            ok = ok or (n["about"] == "fnd-overbought" and ser == ["px", "rsi14", "bb20"])
        need(ok, "three-step narrative px -> rsi14 -> bb20 about the finding")
    elif task == "G14":
        m = series(d, "macd") or {}
        need(m.get("source", {}).get("params") == {"fast": 8.0, "slow": 21.0, "signal": 5.0}, "macd 8/21/5")
        need(any(l.get("series") == "macd" for l in layers(d)), "macd layer kept")
        need(any(e.get("author") == "editor" and not e.get("approved_by") for e in c["agent_log"]),
             "editor wrote directly")
    elif task == "G15":
        ats = [r["at"] for r in results(c, "at")]
        need(any(a["bar"] in (149, 150) and a["series"] == "px" for a in ats), "an `at` query for bar 150")
        need(("117.66" in ans) or ("116.06" in ans), "answer contains the bar close (bar 150, or 149 if 1-based)")
        need(unchanged, "document must not change")
    elif task == "G16":
        need(series(d, "bb20") is None, "bb20 removed")
        need(not any(l.get("series") == "bb20" for l in layers(d)), "its layer removed")
        need(len(d.get("series", [])) == BASE_SERIES - 1, "nothing else removed")
    elif task == "G17":
        need(any(r["screens"] for r in results(c, "screen")), "screen of the user read")
        need(len(ans.strip()) > 0, "summary given")
        need(unchanged, "document must not change")
    elif task == "G18":
        subs = [m for conn, m in c["sent"] if m.get("type") == "subscribe"
                and m.get("topic", {}).get("kind") == "finding" and m["topic"].get("finding") == "fnd-overbought"]
        need(bool(subs), "subscribed to the finding")
        need(any(m.get("type") == "notify" and m.get("event", {}).get("kind") == "finding_state"
                 and m["event"].get("state") == "invalidated" for m in c["seen"]), "notified on invalidation")
    elif task == "G19":
        f = next((f for f in d.get("findings", []) if f["id"] == "fnd-overbought"), {})
        inv = f.get("invalidation", {})
        # The epilogue overwrites the invalidation; check the agent's own log entries instead.
        ok = False
        for e in c["agent_log"]:
            for ch in e["changes"]:
                if ch.get("op") == "put_finding" and ch["finding"]["id"] == "fnd-overbought":
                    i = ch["finding"]["invalidation"]
                    ok = ok or (i.get("kind") == "threshold" and i.get("series") == "px" and i.get("op") == "lt"
                                and i.get("value") == 100.0)
        need(ok, "invalidation: px close < 100")
        need(f.get("claim", "").startswith("THYAO"), "finding kept")
        _ = inv
    elif task == "G20":
        ps = [r["presentation"] for r in results(c, "presentation")]
        hello_voice = any(m.get("type") == "hello" and m.get("surface", {}).get("kind") == "voice"
                          for _, m in c["sent"])
        need(hello_voice, "connected as a voice surface")
        need(any(p["depth"] == "sentence" and p.get("text") for p in ps), "one-sentence presentation")
        need(unchanged, "document must not change")
    else:
        why.append("unknown task")
    return why


def main(argv):
    if len(argv) >= 2 and argv[0] == "run":
        qp = QP
        if "--qp" in argv:
            qp = argv[argv.index("--qp") + 1]
        shown, _, err = run(argv[1], qp=qp)
        for l in shown:
            print(l)
        if err.strip():
            print(err.strip(), file=sys.stderr)
        return 0
    if len(argv) >= 3 and argv[0] == "check":
        ans = ""
        if "--answer" in argv:
            ans = answer_text(argv[argv.index("--answer") + 1])
        why = chk(argv[1], ctx(argv[2]), ans)
        print(json.dumps({"task": argv[1], "pass": not why, "why": why}, ensure_ascii=False))
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
