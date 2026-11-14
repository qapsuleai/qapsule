#!/usr/bin/env python3
"""R2/T9 scorer. Reads only the raw logs; re-running reproduces the numbers exactly.

usage: score.py <label> [variant ...]      (default variants: base A B)
writes raw/<label>/<variant>/score.json and prints markdown tables.

Per agent dir (raw/<label>/<variant>/agent<X>/): setup.json (seed, id mapping),
R??/runs/<n>-<file>.jsonl + .full.jsonl (what the daemon ran and the full authority
output: hidden setup + agent lines + hidden epilogue), R??/answer.txt, R??/notes.txt,
transcript.jsonl (+ .ts: arrival time of each line).

Criteria: rounds/R2/tasks/TASKS.md, checked on the final document (epilogue `document`
query), the authority's replies and answer.txt; independent of message form.
"""

import glob
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(os.path.dirname(HERE), "raw")

# --------------------------------------------------------------------------- reading


def read_jsonl(path):
    out = []
    with open(path, encoding="utf-8") as f:
        for l in f:
            l = l.strip()
            if l:
                try:
                    out.append(json.loads(l))
                except ValueError:
                    out.append({"_unparsed": l})
    return out


def agent_conns(lines):
    conns = []
    for x in lines:
        c = x.get("conn") if isinstance(x, dict) else None
        if isinstance(c, str) and c not in conns:
            conns.append(c)
    return conns


def result(outs, conn, rid):
    for o in outs:
        m = o.get("msg", {})
        if o.get("conn") == conn and isinstance(m, dict) and m.get("id") == rid:
            return m
    return {}


def ctx(run_path, full_path):
    lines = read_jsonl(run_path)
    outs = read_jsonl(full_path)
    conns = set(agent_conns(lines))
    seen = [o["msg"] for o in outs if o.get("conn") in conns | {""} and isinstance(o.get("msg"), dict)]
    return {
        "lines": lines,
        "sent": [x.get("msg", {}) for x in lines if isinstance(x, dict)],
        "seen": seen,
        "doc": result(outs, "zz-host", "e-doc").get("data", {}).get("document", {}),
        "screen": result(outs, "zz-host", "e-screen").get("data", {}),
        "log": result(outs, "zz-host", "e-log").get("data", {}).get("entries", []),
        "hit": result(outs, "zz-ref", "e-hit").get("data", {}).get("hit", {}),
        "errors": [m for m in seen if m.get("type") == "error"],
    }


def read_lines_raw(path):
    with open(path, encoding="utf-8") as f:
        return [l.strip() for l in f if l.strip()]


def answer_text(path):
    if os.path.exists(path):
        t = open(path, encoding="utf-8").read()
        return re.sub(r"(\d),(\d)", r"\1.\2", t)
    return ""


# --------------------------------------------------------------------------- doc helpers

THEME = {"up": "#26a69a", "down": "#ef5350", "accent": "#ffb74d", "warn": "#ffca28", "muted": "#828896",
         "s1": "#ffb74d", "s2": "#42a5f5", "s3": "#ab47bc", "s4": "#66bb6a", "s5": "#ec407a", "s6": "#26c6da",
         "s7": "#8d6e63", "s8": "#bdbdbd", "text": "#c8cdd7", "axis": "#545a68", "grid": "#222630", "bg": "#101218"}


def rgb(c):
    if not isinstance(c, str):
        return None
    c = THEME.get(c, c)
    if re.fullmatch(r"#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?", c):
        return tuple(int(c[i:i + 2], 16) for i in (1, 3, 5))
    return None


def is_red(c):
    if c == "down":
        return True
    v = rgb(c)
    return bool(v) and v[0] >= 150 and v[1] <= 110 and v[2] <= 110


def is_green(c):
    if c == "up":
        return True
    v = rgb(c)
    return bool(v) and v[1] >= 120 and v[0] <= 110 and v[1] > v[2] - 30


def view(doc, m):
    return next((v for v in doc.get("views", []) if v.get("id") == m["main"]), {})


def series_by_id(doc, sid):
    return next((s for s in doc.get("series", []) if s.get("id") == sid), None)


def study_of(doc, sid):
    s = series_by_id(doc, sid) or {}
    src = s.get("source", {})
    return (src.get("study"), src.get("params", {}), src.get("inputs", [])) if src.get("kind") == "study" else (None, {}, [])


def is_study(doc, sid, m, name, **params):
    st, p, inp = study_of(doc, sid)
    return st == name and inp == [m["px"]] and all(p.get(k) == float(v) for k, v in params.items())


def main_field(doc, sid):
    s = series_by_id(doc, sid) or {}
    src = s.get("source", {})
    if src.get("kind") == "study":
        if src.get("output"):
            return src["output"]
        return "middle" if src.get("study") == "bollinger" else src.get("study")
    if s.get("data", {}).get("kind") == "bars":
        return "c"
    return "v"


def num(x):
    if isinstance(x, (int, float)) and not isinstance(x, bool):
        return float(x)
    if isinstance(x, str):
        try:
            return float(x)
        except ValueError:
            return None
    return None


def chan(layer, name, doc):
    """(series, field) a channel reads, or ('const', value), or None."""
    enc = layer.get("encoding", {}) or {}
    if name not in enc:
        return None
    v = enc[name]
    if isinstance(v, dict) and "field" in v:
        return (v.get("series"), v["field"])
    n = num(v)
    if n is not None:
        return ("const", n)
    if isinstance(v, str):
        return (layer.get("series"), v)
    return None


def drawn(layer, doc):
    """Set of (series, field) a layer draws."""
    sid = layer.get("series")
    out = set()
    y = chan(layer, "y", doc)
    y2 = chan(layer, "y2", doc)
    for c in (y, y2):
        if c and c[0] != "const":
            out.add(c)
    if y is None and sid:
        mark = layer.get("mark")
        st = study_of(doc, sid)[0]
        if mark == "candle":
            out |= {(sid, f) for f in "ohlc"}
        elif mark == "line" and st == "bollinger":
            out |= {(sid, f) for f in ("upper", "middle", "lower")}
        elif mark == "line" and st == "macd":
            out |= {(sid, f) for f in ("macd", "signal")}
        else:
            out.add((sid, main_field(doc, sid)))
    return out


def panel_order(v):
    return [p.get("id") for p in v.get("panels", [])]


def layers_in(v, panel=None):
    return [l for l in v.get("layers", []) if panel is None or l.get("panel") == panel]


def panel_with(v, pred):
    for l in v.get("layers", []):
        if pred(l):
            return l.get("panel")
    return None


def price_panel(v, m):
    return panel_with(v, lambda l: l.get("mark") == "candle" and l.get("series") == m["px"])


def scale_of(v, pid):
    p = next((p for p in v.get("panels", []) if p.get("id") == pid), {})
    return p.get("y", {})


def results_of(c, kind):
    return [x["data"] for x in c["seen"] if x.get("type") == "result" and x.get("data", {}).get("kind") == kind]


def nums_in(text):
    return [float(x) for x in re.findall(r"-?\d+(?:\.\d+)?", text)]


def norm(s):
    return re.sub(r"[^\w]+", " ", (s or "").lower()).strip()


ORIG_PANELS = {0, 1, 2}

# --------------------------------------------------------------------------- criteria


def check(task, c, m, ans):
    d = c["doc"]
    v = view(d, m)
    why = []

    def need(cond, msg):
        if not cond:
            why.append(msg)

    if not d:
        return ["no final document (run failed?)"]
    pp = price_panel(v, m)
    order = panel_order(v)

    if task == "R01":
        r7 = [s["id"] for s in d.get("series", []) if is_study(d, s["id"], m, "rsi", period=7)]
        need(bool(r7), "an RSI(7) series on the price series")
        ls = [l for l in v.get("layers", []) if l.get("mark") == "line" and l.get("series") in r7]
        pn = [l.get("panel") for l in ls if l.get("panel") not in ORIG_PANELS]
        need(bool(pn), "an RSI(7) line layer in a NEW panel")
        need(bool(pn) and pp is not None and len(order) > 1 and order[0] == pp and order[1] in pn,
             "that panel is directly below the price panel (2nd in panel order)")
    elif task == "R02":
        bb = m["bb20"]
        lay = [l for l in layers_in(v, pp)]
        red_up = [l for l in lay if (bb, "upper") in drawn(l, d) and is_red((l.get("encoding") or {}).get("color"))]
        need(bool(red_up), "a layer drawing Bollinger upper in red in the price panel")
        for f in ("middle", "lower"):
            ok = [l for l in lay if (bb, f) in drawn(l, d) and not is_red((l.get("encoding") or {}).get("color"))]
            need(bool(ok), f"Bollinger {f} still drawn and not red")
    elif task == "R03":
        new_ids = {m["fnd-brk-new"], m["fact-brk-new"], m["cap-brk-new"]}
        old_ids = {m["fnd-brk-old"], m["fact-brk-old"], m["cap-brk-old"]}

        def covers(t, bar):
            a, b = t.get("from_bar"), t.get("to_bar")
            if a is None and b is None:
                return False
            a = a if a is not None else b
            b = b if b is not None else a
            return a <= bar <= b

        ems = [e for vv in d.get("views", []) for e in (vv.get("emphasis") or [])]
        hits_new = [e for e in ems if e.get("kind") in ("highlight", "focus")
                    and (e.get("target", {}).get("item") in new_ids
                         or (covers(e.get("target", {}), 185) and not covers(e.get("target", {}), 120)))]
        hits_old = [e for e in ems if e.get("target", {}).get("item") in old_ids or covers(e.get("target", {}), 120)]
        need(bool(hits_new), "a persistent emphasis targeting the new breakout (bar 185)")
        need(not hits_old, "the old breakout is not emphasized")
    elif task in ("R04", "R19"):
        depth = "sentence" if task == "R04" else "capsule"
        ps = [p["presentation"] for p in results_of(c, "presentation")]
        mine = [p for p in ps if p.get("finding") == m["fnd-brk-new"] and p.get("depth") == depth]
        need(bool(mine), f"a {depth}-depth presentation of the new breakout finding")
        if task == "R04":
            need(any(norm(p.get("text")) and norm(p.get("text")) in norm(ans) for p in mine),
                 "answer contains the presentation text (capsule headline)")
        else:
            a = ans.lower()
            need("kapsül" in a or "capsule" in a or "kapsul" in a, "answer names the depth (capsule)")
            counts = set()
            for p in mine:
                sp = p.get("spark")
                if isinstance(sp, dict):
                    for k in ("t", "values", "points", "v", "c"):
                        if isinstance(sp.get(k), list):
                            counts.add(len(sp[k]))
                elif isinstance(sp, list):
                    counts.add(len(sp))
            counts.add(60)  # phone profile max_points (PROTOCOL §10.1)
            need(any(float(n) in nums_in(ans) for n in counts), "answer gives the number of bars shown (60)")
    elif task == "R05":
        need(scale_of(v, pp).get("kind") == "log", "price panel y scale is log")
        need(scale_of(v, 1).get("kind") == "linear" and scale_of(v, 1).get("domain") == [0.0, 100.0]
             and scale_of(v, 2).get("kind") == "linear" and "domain" not in scale_of(v, 2),
             "other panels unchanged")
    elif task == "R06":
        rp = panel_with(v, lambda l: l.get("series") == m["rsi14"] and l.get("mark") == "line")
        rules = [l for l in layers_in(v, rp) if l.get("mark") == "rule"]
        r30 = [l for l in rules if chan(l, "y", d) == ("const", 30.0)
               and (l.get("origin") or {}).get("/encoding/y") == {"kind": "threshold", "name": "rsi.oversold"}]
        need(bool(r30), "rule y=30 in the RSI panel with origin rsi.oversold")
        need(any(chan(l, "y", d) == ("const", 70.0) for l in rules), "the 70 rule is still there")
    elif task == "R07":
        need(not any(l.get("series") == m["macd"] for l in v.get("layers", [])), "no layer draws MACD")
        need(2 not in order, "the MACD panel is gone")
        need(pp is not None and panel_with(v, lambda l: l.get("series") == m["rsi14"]) is not None
             and any(l.get("series") == m["bb20"] for l in layers_in(v, pp)), "other panels kept")
    elif task == "R08":
        vb = [l for l in v.get("layers", []) if l.get("mark") == "bar" and (m["px"], "v") in drawn(l, d)]
        need(bool(vb), "a bar layer drawing volume (v) of the price series")
        pv = [l.get("panel") for l in vb if l.get("panel") != pp]
        need(bool(pv), "in a panel other than the price panel")
        need(bool(pv) and len(order) > 1 and order[0] == pp and order[1] in pv,
             "that panel is directly below the price panel")
    elif task == "R09":
        bb = m["bb20"]
        ok = []
        for l in v.get("layers", []):
            y, y2 = chan(l, "y", d), chan(l, "y2", d)
            if l.get("mark") in ("band", "area") and y and y2 and {y, y2} == {(bb, "upper"), (bb, "lower")}:
                op = (l.get("encoding") or {}).get("opacity")
                if op is None or (num(op) is not None and num(op) < 1):
                    ok.append(l)
        need(bool(ok), "a band/area layer filling Bollinger upper..lower with opacity < 1")
    elif task == "R10":
        mc = m["macd"]
        ok = False
        for l in v.get("layers", []):
            if l.get("mark") != "bar" or chan(l, "y", d) != (mc, "histogram"):
                continue
            col = (l.get("encoding") or {}).get("color")
            if isinstance(col, dict) and "when" in col:
                w = col["when"]
                fld = w.get("field")
                th = num(w.get("than"))
                if fld == "histogram" and th == 0.0:
                    if w.get("op") in ("ge", "gt"):
                        ok = ok or (is_green(col.get("then")) and is_red(col.get("else")))
                    if w.get("op") in ("lt", "le"):
                        ok = ok or (is_red(col.get("then")) and is_green(col.get("else")))
            if isinstance(col, dict) and "thresholds" in col:
                if col.get("field") == "histogram" and [num(x) for x in col["thresholds"]] == [0.0] \
                        and len(col.get("colors", [])) == 2:
                    ok = ok or (is_red(col["colors"][0]) and is_green(col["colors"][1]))
        need(ok, "a bar layer of MACD histogram, colour by sign (>=0 green, <0 red)")
    elif task == "R11":
        px = m["px"]
        smas = {s["id"] for s in d.get("series", []) if is_study(d, s["id"], m, "sma", period=50)}
        ok = False
        for l in layers_in(v, pp):
            if (px, "c") not in drawn(l, d):
                continue
            col = (l.get("encoding") or {}).get("color")
            if not (isinstance(col, dict) and "when" in col):
                continue
            w = col["when"]
            than = w.get("than")
            if w.get("field") != "c" or not isinstance(than, dict) or than.get("series") not in smas \
                    or than.get("field") not in ("sma", None):
                continue
            if l.get("series") != px:
                continue
            if w.get("op") in ("ge", "gt"):
                ok = ok or (is_green(col.get("then")) and is_red(col.get("else")))
            else:
                ok = ok or (is_red(col.get("then")) and is_green(col.get("else")))
        need(ok, "a price-panel layer drawing the close, coloured by close >= SMA50 (green) / below (red)")
    elif task == "R12":
        bars = series_by_id(d, m["px"])["data"]["bars"]
        need((v.get("x") or {}).get("domain") == [float(bars[140]["t"]), float(bars[199]["t"])],
             "persistent x.domain = bars 140..199")
    elif task == "R13":
        need(any(s.get("screens") for s in results_of(c, "screen")), "the user's screen was read")
        n = len((c["screen"].get("screens") or [{}])[0].get("panels", []))
        a = ans.lower()
        need(str(n) in a or {3: "üç", 2: "iki", 4: "dört"}.get(n, "?") in a, f"answer gives the panel count ({n})")
        need("rsi" in a, "answer names RSI")
        need("macd" in a, "answer names MACD")
        need("bollinger" in a or "bb" in a, "answer names Bollinger")
        need(any(w in a for w in ("mum", "fiyat", "candle", "kapanış", "thyao")), "answer names the price/candles")
    elif task == "R14":
        hits = [x["hit"] for x in results_of(c, "hit")]
        need(bool(hits), "a point query (hit) was made")
        ref = c["hit"]
        cand = [h for h in hits if h.get("bar") == ref.get("bar") and h.get("panel") == ref.get("panel")]
        need(bool(cand), "the agent's hit is the reference point (same panel and bar)")
        ns = nums_in(ans)
        need(float(ref.get("bar", -1)) in ns, f"answer gives bar {ref.get('bar')}")
        vals = [ref.get("value")] + [x for h in cand for x in (h.get("item", {}) or {}).get("values", {}).values()]
        need(any(isinstance(x, (int, float)) and any(abs(x - y) <= 0.01 for y in ns) for x in vals),
             "answer gives the value at that pixel")
    elif task == "R15":
        ls = [l for l in v.get("layers", []) if l.get("mark") == "line" and l.get("series") == m["rsi14"]]
        need(any(num((l.get("encoding") or {}).get("width")) == 2.0 for l in ls), "RSI line width 2")
    elif task == "R16":
        ok = [l for l in layers_in(v, pp) if l.get("mark") == "point"
              and (l.get("fact") == m["fact-brk-old"] or l.get("finding") == m["fnd-brk-old"])]
        need(bool(ok), "a point layer in the price panel bound to the old breakout fact (bar 120)")
    elif task == "R17":
        facts = {f["id"]: f for f in d.get("facts", [])}
        bound, unbound = False, False
        for l in v.get("layers", []):
            y = chan(l, "y", d)
            if y and y[0] == "const" and abs(y[1] - 123.45) < 1e-9:
                o = (l.get("origin") or {}).get("/encoding/y") or {}
                f = facts.get(o.get("fact"))
                if o.get("kind") == "fact" and f and num((f.get("evidence") or {}).get(o.get("field"))) == 123.45 \
                        and l.get("panel") == pp and l.get("mark") == "rule":
                    bound = True
                else:
                    unbound = True
        told = bool(ans.strip()) and any(w in ans.lower() for w in ("köken", "olgu", "origin", "fact", "kaynak"))
        need(not unbound, "no origin-less 123.45 line")
        need(bound or told, "a rule bound to a fact with level 123.45, or the user is told why not and the option")
    elif task == "R18":
        mp = panel_with(v, lambda l: l.get("series") == m["macd"])
        rp = panel_with(v, lambda l: l.get("series") == m["rsi14"])
        need(None not in (pp, mp, rp) and order.index(pp) < order.index(mp) < order.index(rp),
             "panel order: price, MACD, RSI")
    elif task == "R20":
        ok = [l for l in v.get("layers", []) if l.get("mark") == "line" and (m["bb20"], "bandwidth") in drawn(l, d)
              and l.get("panel") != pp and l.get("panel") not in ORIG_PANELS]
        need(bool(ok), "a bandwidth line layer in a new panel (not the price panel)")
    else:
        why.append("unknown task")
    return why


# --------------------------------------------------------------------------- transcripts


def transcript_stats(adir, tasks):
    """Tokens and time per task from the stream-json transcript.

    Each assistant message (deduplicated by message id) is attributed to the task whose work
    dir it names in a tool call (Write/Read/Bash input), else to the task named last; messages
    before any task is named (spec reading) are overhead, split equally over the tasks."""
    tp = os.path.join(adir, "transcript.jsonl")
    if not os.path.exists(tp):
        return None
    ev = read_jsonl(tp)
    ts = [float(x) for x in open(os.path.join(adir, "transcript.ts"))] if os.path.exists(
        os.path.join(adir, "transcript.ts")) else [0.0] * len(ev)
    usage = {}
    owner = {}
    when = {}
    cur = None
    reads = []
    files = []
    final = {}
    for i, e in enumerate(ev):
        if e.get("type") == "assistant":
            msg = e.get("message", {})
            mid = msg.get("id") or f"_{i}"
            for cblk in msg.get("content", []):
                if cblk.get("type") == "tool_use":
                    s = json.dumps(cblk.get("input", {}), ensure_ascii=False)
                    files.append({"tool": cblk.get("name"), "input": cblk.get("input")})
                    hit = re.findall(r"/(R\d\d)/", s)
                    if hit and hit[-1] in tasks:
                        cur = hit[-1]
            usage[mid] = msg.get("usage", {})
            owner[mid] = cur or "_overhead"
            when.setdefault(mid, [ts[i], ts[i]])
            when[mid][1] = ts[i]
        elif e.get("type") == "result":
            final = e
    tot = lambda u: sum(u.get(k, 0) or 0 for k in ("input_tokens", "cache_creation_input_tokens",
                                                     "cache_read_input_tokens", "output_tokens"))
    per = {t: {"tokens": 0, "output_tokens": 0, "t0": None, "t1": None} for t in tasks + ["_overhead"]}
    for mid, u in usage.items():
        o = per[owner[mid]]
        o["tokens"] += tot(u)
        o["output_tokens"] += u.get("output_tokens", 0) or 0
        a, b = when[mid]
        o["t0"] = a if o["t0"] is None else min(o["t0"], a)
        o["t1"] = b if o["t1"] is None else max(o["t1"], b)
    ov = per.pop("_overhead")
    out = {}
    for t in tasks:
        o = per[t]
        out[t] = {"tokens": o["tokens"] + round(ov["tokens"] / len(tasks)),
                  "tokens_task_only": o["tokens"], "output_tokens": o["output_tokens"],
                  "wall_s": round(o["t1"] - o["t0"], 1) if o["t0"] is not None else 0.0}
    fu = final.get("usage", {})
    agent = {"tokens_sum_messages": sum(tot(u) for u in usage.values()), "overhead_tokens": ov["tokens"],
             "result_usage_total": tot(fu) if fu else None, "cost_usd": final.get("total_cost_usd"),
             "num_turns": final.get("num_turns"), "duration_ms": final.get("duration_ms"),
             "tool_calls": len(files), "tool_inputs": files}
    return out, agent


# --------------------------------------------------------------------------- scoring


def score_agent(adir):
    st = json.load(open(os.path.join(adir, "setup.json")))
    m, tasks = st["mapping"], st["tasks"]
    ts = transcript_stats(adir, tasks)
    rows = []
    allruns = [r for r in sorted(glob.glob(os.path.join(adir, "R??", "runs", "[0-9][0-9]-*.jsonl")))
               if not r.endswith(".full.jsonl")]
    content = {r: read_lines_raw(r) for r in allruns}
    for t in tasks:
        tdir = os.path.join(adir, t)
        runs = [r for r in allruns if os.path.dirname(os.path.dirname(r)) == tdir]
        # Attempts follow the agent's own numbering (attempt1..3). Each attempt is scored on
        # the run of that file in this task dir (the last one if re-run). An attempt file the
        # agent wrote but ran only elsewhere (identical content run from another task dir) is
        # scored on that run; one never run counts as failed.
        last, first_run = {}, {}
        for r in runs:
            last[os.path.basename(r)[3:]] = r
            first_run.setdefault(os.path.basename(r)[3:], r)
        names = {f"attempt{k}.jsonl" for k in (1, 2, 3)}
        have = sorted({n for n in last if n in names} |
                      {os.path.basename(f) for f in glob.glob(os.path.join(tdir, "attempt[123].jsonl"))})
        nmax = max([int(n[7]) for n in have] or [0])
        ans = answer_text(os.path.join(tdir, "answer.txt"))
        att = []
        for k in range(1, nmax + 1):
            name = f"attempt{k}.jsonl"
            wf = os.path.join(tdir, name)
            wlines = read_lines_raw(wf) if os.path.exists(wf) else None
            r, how = first_run.get(name), "run"
            if r and r != last[name]:
                how = "re-run after editing; first run scored, last run counts for final"
            elif r and wlines is not None and content[r] != wlines:
                how = "edited after run (scored as run)"
            if r is None and wlines is not None:
                same = [x for x in allruns if content[x] == wlines]
                if same:
                    r, how = same[-1], "identical file run from " + os.path.relpath(same[-1], adir)
            if r is None:
                att.append({"file": name, "run": None, "how": "never run", "pass": False, "why": ["never run"],
                            "msgs": len(wlines or []), "errors": 0, "error_codes": []})
                continue
            c = ctx(r, r[:-6] + ".full.jsonl")
            why = check(t, c, m, ans)
            att.append({"file": name, "run": os.path.relpath(r, adir), "how": how, "pass": not why, "why": why,
                        "msgs": len(c["lines"]), "errors": len(c["errors"]),
                        "error_codes": [e.get("error", {}).get("code") for e in c["errors"]]})
        # final: the last run of an attempt file in this task dir (re-runs included)
        att_runs = [r for r in runs if os.path.basename(r)[3:] in names]
        final_pass = att[-1]["pass"] if att else False
        if att_runs and att and att_runs[-1] != os.path.join(adir, att[-1]["run"] or ""):
            final_pass = not check(t, ctx(att_runs[-1], att_runs[-1][:-6] + ".full.jsonl"), m, ans)
        other = sorted({os.path.basename(r)[3:] for r in runs} - names)
        row = {"task": t, "agent": os.path.basename(adir)[-1], "attempts": len(att),
               "other_runs": other, "reruns": sum(1 for r in runs if os.path.basename(r)[3:] in names) - len(
                   [n for n in last if n in names]),
               "first": bool(att) and att[0]["pass"], "final": final_pass,
               "any": any(a["pass"] for a in att),
               "first_error": bool(att) and att[0]["errors"] > 0,
               "msgs_first": att[0]["msgs"] if att else 0, "msgs_total": sum(a["msgs"] for a in att),
               "detail": att,
               "notes": open(os.path.join(tdir, "notes.txt"), encoding="utf-8").read().strip()
               if os.path.exists(os.path.join(tdir, "notes.txt")) else ""}
        if ts:
            row.update(ts[0][t])
        rows.append(row)
    return rows, (ts[1] if ts else {}), st


def score_variant(label, variant):
    vdir = os.path.join(RAW, label, variant)
    rows, agents = [], {}
    for adir in sorted(glob.glob(os.path.join(vdir, "agent?"))):
        r, a, st = score_agent(adir)
        rows += r
        a = {k: v for k, v in a.items() if k != "tool_inputs"}
        a["seed"] = st["seed"]
        agents[os.path.basename(adir)] = a
    rows.sort(key=lambda r: r["task"])
    n = len(rows)
    ff = [r for r in rows if not r["first"]]
    errd = [r for r in rows if r["first_error"]]
    summ = {"variant": variant, "label": label, "tasks": n,
            "first": sum(r["first"] for r in rows), "final": sum(r["final"] for r in rows),
            "any": sum(r["any"] for r in rows),
            "recovered": sum(r["final"] for r in ff), "failed_first": len(ff),
            "first_error": len(errd), "recovered_after_error": sum(r["final"] for r in errd),
            "attempts": sum(r["attempts"] for r in rows),
            "msgs_first": sum(r["msgs_first"] for r in rows), "msgs_total": sum(r["msgs_total"] for r in rows),
            "tokens": sum(a.get("tokens_sum_messages", 0) for a in agents.values()),
            "output_tokens": sum(r.get("output_tokens", 0) for r in rows),
            "wall_s": round(sum((a.get("duration_ms") or 0) for a in agents.values()) / 1000, 1),
            "cost_usd": round(sum((a.get("cost_usd") or 0) for a in agents.values()), 4)}
    json.dump({"summary": summ, "agents": agents, "rows": rows},
              open(os.path.join(vdir, "score.json"), "w"), ensure_ascii=False, indent=1, sort_keys=True)
    return summ, rows, agents


def yn(b):
    return "✓" if b else "✗"


def main(argv):
    label = argv[0] if argv else "v1"
    variants = argv[1:] or ["base", "A", "B"]
    allrows = {}
    print("| Variant | First try | Final | Recovered (failed first → final) | First try had an error → final | Attempts | Msgs (first / all) | Tokens | Output tokens | Agent wall time (s) |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    for v in variants:
        if not os.path.isdir(os.path.join(RAW, label, v)):
            continue
        s, rows, agents = score_variant(label, v)
        allrows[v] = rows
        print(f"| {v} | {s['first']}/{s['tasks']} | {s['final']}/{s['tasks']} | {s['recovered']}/{s['failed_first']} | "
              f"{s['recovered_after_error']}/{s['first_error']} | {s['attempts']} | {s['msgs_first']} / {s['msgs_total']} | "
              f"{s['tokens']:,} | {s['output_tokens']:,} | {s['wall_s']} |")
    print()
    tasks = sorted({r["task"] for rows in allrows.values() for r in rows})
    print("| Task | " + " | ".join(f"{v} first / final (att)" for v in allrows) + " |")
    print("|---|" + "---|" * len(allrows))
    for t in tasks:
        cells = []
        for v, rows in allrows.items():
            r = next((r for r in rows if r["task"] == t), None)
            cells.append(f"{yn(r['first'])} / {yn(r['final'])} ({r['attempts']})" if r else "—")
        print(f"| {t} | " + " | ".join(cells) + " |")
    for v, rows in allrows.items():
        print(f"\n### {v}\n")
        print("| Task | Agent | Att | First | Final | Msgs first/all | Tokens | Wall (s) | Errors (codes, per attempt) | Why (first attempt) | Why (last attempt) |")
        print("|---|---|---|---|---|---|---|---|---|---|---|")
        for r in rows:
            d = r["detail"]
            ec = "; ".join(",".join(a["error_codes"]) or "—" for a in d)
            tag = lambda a: "" if a["how"] == "run" else f"[{a['how']}] "
            w1 = (tag(d[0]) + "; ".join(d[0]["why"])) if d else "no attempt"
            wl = (tag(d[-1]) + "; ".join(d[-1]["why"])) if d else "no attempt"
            print(f"| {r['task']} | {r['agent']} | {r['attempts']} | {yn(r['first'])} | {yn(r['final'])} | "
                  f"{r['msgs_first']}/{r['msgs_total']} | {r.get('tokens', 0):,} | {r.get('wall_s', 0)} | {ec} | "
                  f"{w1 or '—'} | {wl or '—'} |")


if __name__ == "__main__":
    main(sys.argv[1:])
