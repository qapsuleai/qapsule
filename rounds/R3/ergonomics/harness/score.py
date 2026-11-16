#!/usr/bin/env python3
"""R3/E scorer. Reads only the raw logs; re-running reproduces the numbers exactly.

usage: score.py <label> [<label> ...]       writes raw/<label>/score.json, prints markdown

Per agent dir (raw/<label>/agent<X>/): setup.json (seed, mapping), truth.json.gz (the
hidden starting state: every story, ledger per kind), C??/runs/<n>-<file>.jsonl (what the
daemon ran) + .out.jsonl.gz (what the agent saw) + .hidden.json (extract of the hidden
epilogue) + .png.json (render request and result), C??/answer.txt, C??/notes.txt,
transcript.jsonl.

Ground truth comes from the hidden setup's own queries in each run (`s-caps`: every story as
the host sees it before the agent's lines; `s-ledger`: ledger per kind), the final state from
the hidden epilogue (`e-doc`, `e-log`, `e-ed-doc`). Criteria: rounds/R3/tasks/TASKS.md
"Doğru sayılan", written out per task in `check()`.
"""

import glob
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(os.path.dirname(HERE), "raw")

OPEN = {"born", "confirmed", "strengthened", "weakened"}
KIND_WORDS = {"breakout": ["kırılım", "kirilim", "kırıl", "breakout"],
              "extreme": ["aşırı", "asiri", "extreme"],
              "continuation": ["devam", "continuation", "eğilim", "trend"],
              "flow": ["akış", "akis", "flow", "kurumsal"],
              "reversal": ["dönüş", "donus", "reversal", "uyumsuzluk", "divergence"],
              "squeeze": ["sıkışma", "sikisma", "squeeze"]}
STAGE_WORDS = {"born": ["born", "doğ", "dog"], "confirmed": ["confirmed", "teyit", "doğrulan"],
               "strengthened": ["strengthened", "güçlen", "guclen"], "weakened": ["weakened", "zayıfla", "zayifla"],
               "realized": ["realized", "gerçekleş", "gerceklest"], "invalidated": ["invalidated", "geçersiz", "gecersiz"],
               "expired": ["expired", "süre", "sure", "doldu"], "retracted": ["retracted", "geri çek"]}
REASON_WORDS = {"barrier.stop": ["barrier.stop", "stop", "durdur", "zarar kes", "bariyer", "destek", "direnç", "geri döndü",
                                 "geri dön", "altına", "altina", "aşağı", "geri"],
                "barrier.no_expansion": ["no_expansion", "genişle"],
                "opposite_fact": ["opposite", "karşıt", "ters"]}


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


def read_lines_raw(path):
    with open(path, encoding="utf-8") as f:
        return [l.strip() for l in f if l.strip()]


def result(outs, conn, rid):
    for o in outs:
        m = o.get("msg", {})
        if o.get("conn") == conn and isinstance(m, dict) and m.get("id") == rid:
            return m
    return {}


def ctx(run_path, truth_doc):
    import gzip
    lines = read_jsonl(run_path)
    with gzip.open(run_path[:-6] + ".out.jsonl.gz", "rt", encoding="utf-8") as f:
        seen = [json.loads(l) for l in f if l.strip()]
    seen = [o for o in seen if isinstance(o.get("msg"), dict)]
    hid = json.load(open(run_path[:-6] + ".hidden.json", encoding="utf-8"))
    # pair each sent message with id to its reply on the same connection
    sent = [x for x in lines if isinstance(x, dict) and isinstance(x.get("msg"), dict)]
    pairs = []
    for x in sent:
        m = x["msg"]
        rid = m.get("id")
        rep = next((o["msg"] for o in seen if o.get("conn") == x.get("conn") and o["msg"].get("id") == rid), {}) \
            if rid is not None else {}
        pairs.append((x.get("conn"), m, rep))
    hellos = {x.get("conn"): x["msg"].get("principal") for x in sent if x["msg"].get("type") == "hello"}
    png = None
    pj = run_path[:-6] + ".png.json"
    if os.path.exists(pj):
        png = json.load(open(pj))
    return {
        "lines": lines, "pairs": pairs, "hellos": hellos, "png": png,
        "caps0": truth_doc["s-caps"]["capsules"], "ledger0": truth_doc["s-ledger"],
        "truth_ok": hid["s_caps_sha256"] == truth_doc["s_caps_sha256"],
        "doc": hid["doc"], "log": hid["log"],
        "aud": hid.get("editor_audience") or {},
        "errors": [o["msg"] for o in seen if o["msg"].get("type") == "error"],
    }


def answer_text(path):
    if os.path.exists(path):
        return open(path, encoding="utf-8").read()
    return ""


# --------------------------------------------------------------------------- truth


def truth(c, n_bars):
    caps = c["caps0"]
    f = lambda sym=None, kind=None, stage=None: [
        x for x in caps if (sym is None or x["instrument"] == sym) and (kind is None or x["kind"] == kind)
        and (stage is None or (x["stage"] in OPEN if stage == "open" else x["stage"] == stage))]
    latest = lambda xs: max(xs, key=lambda x: (x["last_bar"], x["born_bar"])) if xs else None
    one = lambda xs: xs[0] if len(xs) == 1 else None
    last = n_bars - 1
    born10 = {}
    for x in caps:
        if x["born_bar"] >= last - 9:
            born10[x["kind"]] = born10.get(x["kind"], 0) + 1
    born10b = {}  # alternative reading: the ten bars up to the last bar with a story transition
    lb = max((x["last_bar"] for x in caps), default=last)
    for x in caps:
        if x["born_bar"] >= lb - 9:
            born10b[x["kind"]] = born10b.get(x["kind"], 0) + 1
    led = next((r for r in c["ledger0"].get("rows", []) if r.get("kind") == "breakout"), {})
    return {
        "thy_open": f("THYAO", stage="open"),
        "brk": one(f("THYAO", "breakout", "open")),
        "ext": one(f("THYAO", "extreme", "open")),
        "asels_inv": latest(f("ASELS", stage="invalidated")),
        "real_brk": latest(f("THYAO", "breakout", "realized")),
        # C13's other reading ("en son gerçekleşen kırılım" = the latest breakout that occurred)
        "latest_brk": max(f("THYAO", "breakout"), key=lambda x: (x["born_bar"], x["last_bar"])),
        "squeeze": latest(f("THYAO", "squeeze")),
        "flow": latest(f("THYAO", "flow")),
        "top3_all": caps[:3],
        "top3_open": [x for x in caps if x["stage"] in OPEN][:3],
        "born10": born10, "born10b": born10b,
        "realized": f(stage="realized"),
        "ledger_breakout": led,
    }


# --------------------------------------------------------------------------- helpers


def low(s):
    return s.lower().replace("ı", "i").replace("İ", "i") if s else ""


def has_any(text, words):
    t = low(text)
    return any(low(w) in t for w in words)


def ints_in(text):
    return [int(x) for x in re.findall(r"(?<![\d.,])\d+(?![\d]|[.,]\d)", text or "")]


def replies(c, mtype=None, qkind=None, ok=True):
    """(conn, sent msg, reply) for messages of this type / query kind with a non-error reply."""
    out = []
    for conn, m, rep in c["pairs"]:
        if mtype and m.get("type") != mtype:
            continue
        if qkind and (m.get("query") or {}).get("kind") != qkind:
            continue
        if ok and rep.get("type") in (None, "error"):
            continue
        out.append((conn, m, rep))
    return out


def mention_pos(ans, cap):
    """Earliest position where the answer names this capsule: its id, its finding id, or a
    line naming its instrument, a kind word and its birth bar."""
    pos = []
    for k in (cap["id"], cap["finding"]):
        i = ans.find(k)
        if i >= 0:
            pos.append(i)
    off = 0
    for line in ans.splitlines(True):
        if cap["instrument"] in line and has_any(line, KIND_WORDS[cap["kind"]]) and \
                re.search(r"(?<!\d)%d(?!\d)" % cap["born_bar"], line):
            pos.append(off)
        off += len(line)
    return min(pos) if pos else None


def is_cap(x, cap):
    return cap is not None and x in (cap["id"], cap["finding"])


# --------------------------------------------------------------------------- criteria


def check(task, c, T, ans, n_bars):
    why = []

    def need(cond, msg):
        if not cond:
            why.append(msg)

    if task == "C01":
        r = [rep for _, m, rep in replies(c, "query", "capsules")
             if {x["id"] for x in T["thy_open"]} <= {x["id"] for x in rep.get("data", {}).get("capsules", [])}]
        need(r, "a capsules query whose answer holds every open THYAO capsule")
        miss = [x["id"] for x in T["thy_open"] if not has_any(ans, KIND_WORDS[x["kind"]])]
        need(ans and not miss, f"answer names every open THYAO capsule (missing: {miss})")
    elif task == "C02":
        b = T["brk"]
        r = [rep for _, m, rep in replies(c, "query", "capsule")
             if is_cap((rep.get("data", {}).get("detail", {}).get("item") or {}).get("id"), b)
             and any(e.get("values") for e in rep["data"]["detail"].get("evidence", []))]
        need(r, "capsule query on the open THYAO breakout with evidence values returned")
    elif task == "C03":
        a = T["asels_inv"]
        r = [rep for _, m, rep in replies(c, "query", "capsule")
             if (rep.get("data", {}).get("detail", {}).get("item") or {}).get("id") == a["id"]]
        need(r, f"capsule query on {a['id']}")
        inv = [h for h in (r[0]["data"]["detail"]["story"]["history"] if r else []) if h["stage"] == "invalidated"]
        reason = inv[-1]["reason"] if inv else None
        need(has_any(ans, STAGE_WORDS["invalidated"]), "answer names the invalidation")
        need(reason and has_any(ans, REASON_WORDS.get(reason, [reason])), f"answer gives the reason ({reason})")
    elif task == "C04":
        b = T["brk"]
        reqs = []
        for e in c["log"]:
            reqs += e.get("request") or []
        for p in c["doc"].get("proposals", []):
            reqs += p.get("ops", [])
        ok = [o for o in reqs if o.get("op") == "reject_finding" and o.get("finding") == b["finding"]]
        wrong = [o for o in reqs if o.get("op") == "reject_finding" and o.get("finding") != b["finding"]]
        need(ok, "reject_finding on the open THYAO breakout's finding, in the log (applied or proposed)")
        need(not wrong, f"no reject_finding on another finding ({[o.get('finding') for o in wrong]})")
    elif task == "C05":
        r = replies(c, "query", "capsules")
        need(r, "a capsules query")
        okk = False
        for key in ("top3_all", "top3_open"):
            ps = [mention_pos(ans, x) for x in T[key]]
            if all(p is not None for p in ps) and ps == sorted(ps):
                okk = key
        need(okk, "answer names the three most important capsules in order (all stories or open ones)")
    elif task == "C06":
        L = T["ledger_breakout"]
        r = replies(c, "query", "ledger")
        need(r, "a ledger query")
        nums = ints_in(ans)
        need(L and L["n"] in nums and L["realized"] in nums, f"answer gives n={L.get('n')} and realized={L.get('realized')}")
    elif task == "C07":
        need(c["aud"].get("sensitivity") == "important", f"editor's sensitivity important (got {c['aud'].get('sensitivity')})")
    elif task == "C08":
        need("ASELS" in (c["aud"].get("watchlist") or []), f"ASELS in editor's watch list (got {c['aud'].get('watchlist')})")
    elif task == "C09":
        e = T["ext"]
        r = [rep for _, m, rep in replies(c, "query", "present")
             if (rep.get("data", {}).get("presentation") or {}).get("finding") == e["finding"]
             and rep["data"]["presentation"].get("depth") == "capsule"]
        need(r, "present of the open THYAO extreme capsule at capsule depth")
    elif task == "C10":
        b = T["brk"]
        r = [rep for _, m, rep in replies(c, "query", "present")
             if (rep.get("data", {}).get("presentation") or {}).get("finding") == b["finding"]
             and rep["data"]["presentation"].get("depth") == "chart"]
        need(r, "present of the open THYAO breakout at chart depth")
        lvl = False
        for rep in r:
            v = rep["data"]["presentation"].get("view") or {}
            for l in v.get("layers", []):
                if l.get("mark") == "rule" and any((o or {}).get("kind") == "fact" for o in (l.get("origin") or {}).values()):
                    lvl = True
        yes = has_any(ans, ["evet", "çiziliyor", "ciziliyor", "çizili", "yes", "var"]) and \
            not has_any(ans, ["çizilmiyor", "cizilmiyor", "hayır"])
        need(ans and (yes == lvl), f"answer says the broken level is {'drawn' if lvl else 'not drawn'}")
    elif task == "C11":
        princ = set(c["hellos"].values())
        need(princ == {"viewer"}, f"only `viewer` connections (got {sorted(p or '' for p in princ)})")
        r = replies(c, "query", "capsule", ok=False) + replies(c, "query", "capsules") + \
            replies(c, "query", "verify_evidence", ok=False)
        need(r, "a capsule / capsules / verify_evidence query as viewer")
        need("flow/" in ans, "answer names the entitlement it needs (flow/…)")
    elif task == "C12":
        r = [m for _, m, rep in replies(c, "subscribe") if rep.get("status") == "subscribed"
             and (m.get("topic") or {}).get("kind") == "capsules" and (m.get("topic") or {}).get("instrument") == "THYAO"]
        need(r, "subscribe to the capsules topic, instrument THYAO (acked)")
    elif task in ("C13", "C13alt"):
        a = T["real_brk"] if task == "C13" else T["latest_brk"]
        r = [rep for _, m, rep in replies(c, "query", "capsule")
             if (rep.get("data", {}).get("detail", {}).get("item") or {}).get("id") == a["id"]]
        need(r, f"capsule query on {a['id']}")
        if r:
            hist = r[0]["data"]["detail"]["story"]["history"]
            miss = sorted({h["stage"] for h in hist if not has_any(ans, STAGE_WORDS[h["stage"]])})
            need(not miss, f"answer lists every transition (missing stages: {miss})")
            # every transition counted: as many history lines as transitions, or each bar named
            bars = [h["bar"] for h in hist]
            need(all(re.search(r"(?<!\d)%d(?!\d)" % b, ans) for b in bars) or
                 len([l for l in ans.splitlines() if has_any(l, sum(STAGE_WORDS.values(), []))]) >= len(hist),
                 "answer lists all transitions (bars or one line each)")
    elif task == "C14":
        b = T["brk"]
        fids = set()
        for x in c["doc"].get("findings", []):
            if x["id"] == b["finding"]:
                fids = set(x.get("facts", []))
        r = [rep for _, m, rep in replies(c, "query", "verify_evidence")
             if rep.get("data", {}).get("kind") == "evidence_check"
             and ((rep["data"].get("check") or {}).get("item") in fids | {b["finding"]})]
        need(r, "verify_evidence on the open THYAO breakout's fact (or its finding)")
    elif task == "C15":
        b = T["brk"]
        r = [rep for _, m, rep in replies(c, "dispute")
             if (m.get("claim") or {}).get("kind") == "capsule" and is_cap((m.get("claim") or {}).get("capsule"), b)
             and (rep.get("data") or {}).get("verdict") in ("rejected", "upheld", "pending")]
        need(r, "dispute on the open THYAO breakout capsule, decided (rejected/upheld/pending)")
    elif task == "C16":
        s = T["squeeze"]
        r = [rep for _, m, rep in replies(c, "query", "capsule")
             if (rep.get("data", {}).get("detail", {}).get("item") or {}).get("id") == s["id"]] + \
            [rep for _, m, rep in replies(c, "query", "capsules")
             if s["id"] in {x["id"] for x in rep.get("data", {}).get("capsules", [])}]
        need(r, "a capsule/capsules query that returned the THYAO squeeze capsule")
        valid = s["stage"] in OPEN
        neg = has_any(ans, ["hayır", "hayir", "geçerli değil", "gecerli degil", "geçersiz", "gecersiz", "artık", "sona er"])
        pos = has_any(ans, ["evet", "hâlâ geçerli", "hala gecerli"]) and not neg
        need(ans and (pos if valid else neg), f"answer says {'still valid' if valid else 'no longer valid'}")
    elif task == "C17":
        r = replies(c, "query", "capsules")
        need(r, "a capsules query")
        okk = False
        for key in ("born10", "born10b"):
            cnt = T[key]
            good = True
            for k, v in cnt.items():
                if not any(has_any(l, KIND_WORDS[k]) and v in ints_in(l) for l in ans.splitlines()):
                    good = False
            if good and cnt:
                okk = key
        need(okk, f"answer counts per kind = {T['born10']} (or {T['born10b']})")
    elif task == "C18":
        need((c["aud"].get("positions") or {}).get("THYAO") == 100, f"editor's position THYAO = 100 (got {c['aud'].get('positions')})")
    elif task == "C19":
        r = [rep for _, m, rep in replies(c, "query", "capsules")
             if (m["query"].get("stages") == ["realized"])]
        need(r, "capsules query with stages [realized]")
    elif task == "C20":
        b = T["brk"]
        p = c["png"]
        need(p and p.get("rc") == 0, "a PNG was drawn (run.sh --png)")
        if p and p.get("rc") == 0:
            req = p.get("png", {})
            okv = False
            if req.get("present"):
                okv = req["present"] in (b["finding"], b["id"])
            elif req.get("view"):
                for v in c["doc"].get("views", []):
                    if v.get("id") == req["view"]:
                        s = json.dumps(v)
                        okv = b["finding"] in s or any(f in s for f in fact_ids(c, b))
            need(okv, "the drawn view presents the open THYAO breakout (its finding or facts)")
    else:
        why.append("unknown task")
    return why


def fact_ids(c, b):
    for x in c["doc"].get("findings", []):
        if x["id"] == b["finding"]:
            return x.get("facts", [])
    return []


ID_RE = re.compile(r"\b(?:cap|fnd|fx)\.(?:THYAO|ASELS)\.[a-z_.]+\d+|\bprop-\d+")


def ids_used(lines_text, mapping, png):
    found = set(ID_RE.findall(lines_text))
    for k in ("THYAO", "ASELS"):
        if re.search(r'"%s"' % re.escape(mapping[k]), lines_text):
            found.add(mapping[k])
    if png:
        for k in ("present", "view"):
            v = (png.get("png") or {}).get(k) or ""
            if ID_RE.search(v):
                found.add(v)
    return sorted(found)


# --------------------------------------------------------------------------- transcripts


def transcript_stats(adir):
    tp = os.path.join(adir, "transcript.jsonl")
    if not os.path.exists(tp):
        return {}
    final = {}
    ncalls = 0
    for e in read_jsonl(tp):
        if e.get("type") == "assistant":
            ncalls += sum(1 for b in e.get("message", {}).get("content", []) if b.get("type") == "tool_use")
        elif e.get("type") == "result":
            final = e
    fu = final.get("usage", {})
    tot = sum(fu.get(k, 0) or 0 for k in ("input_tokens", "cache_creation_input_tokens",
                                           "cache_read_input_tokens", "output_tokens"))
    return {"tokens": tot, "output_tokens": fu.get("output_tokens", 0), "cost_usd": final.get("total_cost_usd"),
            "num_turns": final.get("num_turns"), "duration_ms": final.get("duration_ms"), "tool_calls": ncalls,
            "final_text": final.get("result", "")}


# --------------------------------------------------------------------------- scoring


def score_agent(adir):
    st = json.load(open(os.path.join(adir, "setup.json")))
    m, tasks, n_bars = st["mapping"], st["tasks"], st["n_bars"]
    rows = []
    import gzip
    with gzip.open(os.path.join(adir, "truth.json.gz"), "rt", encoding="utf-8") as f:
        td = json.load(f)
    allruns = [r for r in sorted(glob.glob(os.path.join(adir, "C??", "runs", "[0-9][0-9]-*.jsonl")))
               if not r.endswith(".out.jsonl")]
    content = {r: read_lines_raw(r) for r in allruns}
    names = {f"attempt{k}.jsonl" for k in (1, 2, 3)}
    for t in tasks:
        tdir = os.path.join(adir, t)
        runs = [r for r in allruns if os.path.dirname(os.path.dirname(r)) == tdir]
        first_run, last = {}, {}
        for r in runs:
            first_run.setdefault(os.path.basename(r)[3:], r)
            last[os.path.basename(r)[3:]] = r
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
                how = "re-run (first run scored; last run counts for final)"
            if r is None and wlines is not None:
                same = [x for x in allruns if content[x] == wlines]
                if same:
                    r, how = same[-1], "identical file run from " + os.path.relpath(same[-1], adir)
            if r is None:
                att.append({"file": name, "run": None, "how": "never run", "pass": False, "why": ["never run"],
                            "msgs": len(wlines or []), "errors": 0, "error_codes": [], "ids": []})
                continue
            c = ctx(r, td)
            T = truth(c, n_bars)
            why = check(t, c, T, ans, n_bars)
            if not c["truth_ok"]:
                why.append("harness: hidden starting state differs from truth.json")
            att.append({"file": name, "run": os.path.relpath(r, adir), "how": how, "pass": not why, "why": why,
                        "ids": ids_used("\n".join(content[r]), m, c["png"]),
                        "msgs": len(c["lines"]), "errors": len(c["errors"]),
                        "error_codes": [e.get("error", {}).get("code") for e in c["errors"]]})
        att_runs = [r for r in runs if os.path.basename(r)[3:] in names]
        final_pass = att[-1]["pass"] if att else False
        if att_runs and att and att[-1]["run"] and att_runs[-1] != os.path.join(adir, att[-1]["run"]):
            c = ctx(att_runs[-1], td)
            final_pass = not check(t, c, truth(c, n_bars), ans, n_bars)
        alt = None
        if t == "C13" and att and att[-1]["run"]:
            c = ctx(os.path.join(adir, att[-1]["run"]), td)
            alt = not check("C13alt", c, truth(c, n_bars), ans, n_bars)
        other = sorted({os.path.basename(r)[3:] for r in runs} - names)
        rows.append({
            "task": t, "agent": os.path.basename(adir)[-1], "attempts": len(att), "other_runs": other,
            "first": bool(att) and att[0]["pass"], "final": final_pass,
            "final_lenient": final_pass or bool(alt), "c13_alt_reading": alt,
            "first_idfree": bool(att) and att[0]["pass"] and not att[0]["ids"],
            "final_idfree": bool(att) and final_pass and not att[-1]["ids"],
            "first_error": bool(att) and att[0]["errors"] > 0,
            "msgs_first": att[0]["msgs"] if att else 0, "msgs_total": sum(a["msgs"] for a in att),
            "detail": att,
            "notes": open(os.path.join(tdir, "notes.txt"), encoding="utf-8").read().strip()
            if os.path.exists(os.path.join(tdir, "notes.txt")) else "",
            "answer": ans.strip()})
    ts = transcript_stats(adir)
    return rows, ts, st


def score_label(label):
    ldir = os.path.join(RAW, label)
    rows, agents = [], {}
    for adir in sorted(glob.glob(os.path.join(ldir, "agent?"))):
        r, a, st = score_agent(adir)
        rows += r
        a = {k: v for k, v in a.items() if k != "final_text"}
        a["seed"] = st["seed"]
        a["off"] = st["mapping"]["off"]
        agents[os.path.basename(adir)] = a
    rows.sort(key=lambda r: r["task"])
    n = len(rows)
    summ = {"label": label, "tasks": n,
            "first": sum(r["first"] for r in rows), "final": sum(r["final"] for r in rows),
            "final_lenient": sum(r["final_lenient"] for r in rows),
            "first_idfree": sum(r["first_idfree"] for r in rows), "final_idfree": sum(r["final_idfree"] for r in rows),
            "first_error": sum(r["first_error"] for r in rows),
            "attempts": sum(r["attempts"] for r in rows), "unnumbered_runs": sum(len(r["other_runs"]) for r in rows),
            "msgs_first": sum(r["msgs_first"] for r in rows), "msgs_total": sum(r["msgs_total"] for r in rows),
            "tokens": sum(a.get("tokens", 0) for a in agents.values()),
            "output_tokens": sum(a.get("output_tokens", 0) or 0 for a in agents.values()),
            "wall_s": round(sum((a.get("duration_ms") or 0) for a in agents.values()) / 1000, 1),
            "cost_usd": round(sum((a.get("cost_usd") or 0) for a in agents.values()), 4)}
    json.dump({"summary": summ, "agents": agents, "rows": rows},
              open(os.path.join(ldir, "score.json"), "w"), ensure_ascii=False, indent=1, sort_keys=True)
    return summ, rows, agents


def yn(b):
    return "✓" if b else "✗"


def main(argv):
    labels = argv or ["r1", "r2"]
    allrows = {}
    print("| Run | First try | First try id-free | Final | Final id-free | First try had an error | Attempts | Unnumbered runs | Msgs (first / all) | Tokens | Output tokens | Agent time (s) | Cost |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for lb in labels:
        if not os.path.isdir(os.path.join(RAW, lb)):
            continue
        s, rows, agents = score_label(lb)
        allrows[lb] = rows
        print(f"| {lb} | {s['first']}/{s['tasks']} | {s['first_idfree']}/{s['tasks']} | {s['final']}/{s['tasks']} | "
              f"{s['final_idfree']}/{s['tasks']} (lenient final {s['final_lenient']}) | {s['first_error']} | {s['attempts']} | {s['unnumbered_runs']} | "
              f"{s['msgs_first']} / {s['msgs_total']} | {s['tokens']:,} | {s['output_tokens']:,} | {s['wall_s']} | {s['cost_usd']:.2f} $ |")
    print()
    tasks = sorted({r["task"] for rows in allrows.values() for r in rows})
    print("| Task | " + " | ".join(f"{lb} first / final (att) id-free" for lb in allrows) + " |")
    print("|---|" + "---|" * len(allrows))
    for t in tasks:
        cells = []
        for lb, rows in allrows.items():
            r = next((r for r in rows if r["task"] == t), None)
            cells.append(f"{yn(r['first'])} / {yn(r['final'])} ({r['attempts']}) {'id-free' if r['final_idfree'] else ('ids' if r['final'] else '')}" if r else "—")
        print(f"| {t} | " + " | ".join(cells) + " |")
    for lb, rows in allrows.items():
        print(f"\n### {lb}\n")
        print("| Task | Agent | Att | First | Final | Ids used (first; last) | Errors (codes, per attempt) | Why (first attempt) | Why (last attempt) |")
        print("|---|---|---|---|---|---|---|---|---|")
        for r in rows:
            d = r["detail"]
            ec = "; ".join(",".join(x or "?" for x in a["error_codes"]) or "—" for a in d)
            ids = (", ".join(d[0]["ids"]) or "—") + "; " + (", ".join(d[-1]["ids"]) or "—") if d else "—"
            w1 = "; ".join(d[0]["why"]) if d else "no attempt"
            wl = "; ".join(d[-1]["why"]) if d else "no attempt"
            print(f"| {r['task']} | {r['agent']} | {r['attempts']} | {yn(r['first'])} | {yn(r['final'])} | {ids} | {ec} | "
                  f"{w1 or '—'} | {wl or '—'} |")


if __name__ == "__main__":
    main(sys.argv[1:])
