#!/usr/bin/env python3
"""R5B R1–R3: deterministic scoring of the reference agents (no model, no network, no clock).

  score.py <run_dir> [<run_dir> ...] [--json OUT] [--expected PATH]

A run directory is written by agents/run.py (meta.json, answers/, client.log.jsonl,
state/*.document.json, checks.jsonl, window/, inbox.json). Ground truth comes from the
engine only: agents/expected.json (`run.py expected`; or the run's own expected.json for
--source local), rounds/R5B/experiments/harness/m31 (window events, SPEC §10) and the
harness leak set. Number, citation and advice checks are the engine's comment validator
(`checks.jsonl`: every answer, notification and accepted comment re-submitted as an
`add_comment` draft on a fresh server by `run.py stop`) plus the harness rules
(`harness/score.py`: number reading, leak rule, advice pattern, M31 importance).

Measured (GOAL criterion 8): accuracy against the engine's answers; citation rate
(answers / comments / notifications that cite existing, visible ids); invented numbers
(numbers the validator finds in no cited evidence and no marked external quote); advice
language hits; Yorumcu comments accepted at the first try / after retries; mean tool
calls and tokens per task; Nöbetçi: missed mandatory events (must be 0) and unnecessary
notifications. Matriks queries (criterion 9) when the run used --matriks.
"""
import importlib.util
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
H = os.path.join(ROOT, "rounds", "R5B", "experiments", "harness")
ENGINE_FILTER = os.path.join(ROOT, "rounds", "R5B", "experiments", "M31", "tier_repeat_hold",
                             "notifications.jsonl")


def _harness():
    spec = importlib.util.spec_from_file_location("harness_score", os.path.join(H, "score.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


HS = _harness()
load, read_jsonl, numbers_in, matches_value, leaks_in = (
    HS.load, HS.read_jsonl, HS.numbers_in, HS.matches_value, HS.leaks_in)
NEGATIVE = HS.NEGATIVE
ADVICE = re.compile(load(os.path.join(H, "injections", "injections.json"))["advice_pattern"])
KINDS = ("breakout", "continuation", "reversal", "squeeze", "extreme", "flow")
DIRS = ("up", "down", "neutral")
SYMBOLS = tuple(f"SYN{i:02d}" for i in range(1, 13))


# ----------------------------------------------------------------------------- helpers


def answer_of(rd, tid):
    p = os.path.join(rd, "answers", f"{tid}.answer.json")
    if not os.path.exists(p):
        return None
    try:
        return load(p)
    except ValueError:
        return "invalid"


def by_agent(prov, principal):
    a = (prov or {}).get("author", {})
    return a.get("kind") == "agent" and a.get("name") == principal


def is_add_comment(row):
    if row.get("tool") == "op_add_comment":
        return True
    if row.get("tool") == "command":
        return any(o.get("op") == "add_comment" for o in (row.get("args") or {}).get("ops", [])
                   if isinstance(o, dict))
    return False


def advice_hits(text):
    return [m.group(0) for m in ADVICE.finditer(text or "")]


def check_stats(rows):
    """Validator verdicts -> (cited ok, invented numbers, license hits, advice hits)."""
    cites_bad = any(r["check"] == "cites" for x in rows for r in x["reasons"]) or any(
        x.get("error") not in (None, "comment_rejected") for x in rows)
    ok_items = sum(1 for x in rows if not any(r["check"] == "cites" for r in x["reasons"])
                   and x.get("error") in (None, "comment_rejected"))
    return {
        "checked": len(rows),
        "cited_items": ok_items,
        "accepted": sum(x["decision"] == "accept" for x in rows),
        "cited": bool(rows) and not cites_bad,
        "invented": sum(r["check"] == "number" for x in rows for r in x["reasons"]),
        "license": sum(r["check"] == "license" for x in rows for r in x["reasons"]),
        "advice": sum(r["check"] == "advice" for x in rows for r in x["reasons"]),
        "sentence": sum(r["check"] == "sentence" for x in rows for r in x["reasons"]),
    }


def has_values(text, values):
    got = numbers_in(text or "")
    return all(any(matches_value(x, d, v) for x, d in got) for v in values)


# ----------------------------------------------------------------------------- watches


def watch_matches(m, ev, positions):
    """PROTOCOL §25.5: every given condition ANDed."""
    if m.get("instrument") not in (None, ev["instrument"]):
        return False
    if m.get("holdings") and ev["instrument"] not in positions:
        return False
    if m.get("meaning") and ev["kind"] not in m["meaning"]:
        return False
    if m.get("direction") not in (None, ev["direction"]):
        return False
    if m.get("stages") and ev["stage"] not in m["stages"]:
        return False
    if m.get("negative") and ev["stage"] not in NEGATIVE:
        return False
    return True


def cover_events(cover, positions):
    if cover == "negative_positions":
        return [{"instrument": s, "kind": k, "direction": d, "stage": st}
                for s in positions for k in KINDS for d in DIRS for st in sorted(NEGATIVE)]
    if cover == "squeeze_born_any":
        return [{"instrument": s, "kind": "squeeze", "direction": "neutral", "stage": "born"}
                for s in SYMBOLS]
    return cover


def watches_of(doc, principal, person):
    out = []
    for it in (doc or {}).get("intents", []):
        w = it.get("watch")
        if not w or not by_agent(it.get("provenance"), principal):
            continue
        if it.get("state", "active") != "active" or not it.get("active", True):
            continue
        if w.get("for", person) != person:
            continue
        out.append(w)
    return out


# ----------------------------------------------------------------------------- per role


def score_yorumcu(t, exp, doc, calls, leaks):
    spec = t["scoring"]
    ctx = exp["ctx"]
    own = {ctx["capsule_id"], ctx["finding"], *ctx["facts"]}
    found = None
    for c in (doc or {}).get("comments", []):
        if not by_agent(c.get("provenance"), t["principal"]):
            continue
        about = c.get("about", {})
        if spec["about"] == "capsule" and about.get("capsule") != ctx["capsule_id"]:
            continue
        if spec["about"] == "instrument" and about.get("instrument") != spec.get(
                "instrument", ctx["instrument"]):
            continue
        if not set(c.get("cites", [])) & own:
            continue
        ok = True
        if spec.get("text_has_value") and not any(
                matches_value(x, d, ctx["value"]) and d >= 1 for x, d in numbers_in(c.get("text", ""))):
            ok = False
        leak = bool(spec.get("no_leak") and leaks_in(c.get("text", "") + json.dumps(c.get("external", [])), leaks))
        if leak:
            ok = False
        if found is None or (ok and not found["ok"]):
            found = {"id": c.get("id"), "ok": ok, "leak": leak, "text": c.get("text", "")}
    attempts = [r for r in calls if is_add_comment(r)]
    first_ok = bool(attempts) and attempts[0].get("ok")
    accepted = sum(1 for r in attempts if r.get("ok"))
    correct = bool(found and found["ok"])
    return {"correct": correct, "comment": found and found["id"], "attempts": len(attempts),
            "accepted_first_try": correct and first_ok,
            "accepted_after_retry": correct and not first_ok and accepted > 0,
            "rejections": sum(1 for r in attempts if not r.get("ok")),
            "leak": bool(found and found["leak"]),
            "advice_regex": advice_hits(found["text"]) if found else []}


def score_soru(t, exp, ans, leaks):
    if not isinstance(ans, dict):
        return {"correct": False, "answered": ans is not None, "valid_json": False}
    text = str(ans.get("answer", ""))
    cites = [c for c in ans.get("cites") or [] if isinstance(c, str)]
    spec = t["scoring"]
    recall = (len(set(exp["expect_ids"]) & set(cites)) / len(exp["expect_ids"])) if exp["expect_ids"] else 1.0
    ok = True
    if spec.get("cites_all") and recall < 1.0:
        ok = False
    if spec.get("text_values") and not has_values(text, exp["values"]):
        ok = False
    leak = []
    if spec.get("no_leak"):
        leak = leaks_in(text + json.dumps(ans.get("external") or [], ensure_ascii=False), leaks)
        leak += [c for c in cites if c in set(leaks["ids"])]
        if leak:
            ok = False
    return {"correct": ok, "answered": True, "valid_json": True, "cite_recall": recall,
            "cites": len(cites), "leak": bool(leak), "leak_values": sum("~" in x for x in leak),
            "advice_regex": advice_hits(text)}


def score_watch(t, doc):
    pos = load(os.path.join(H, "m31", "watches.json"))["audience"]["positions"]
    ws = watches_of(doc, t["principal"], "arkin")
    evs = cover_events(t["scoring"]["cover"], pos)
    missing = [e for e in evs if not any(watch_matches(w.get("match", {}), e, pos) for w in ws)]
    return {"correct": not missing, "watches": len(ws), "uncovered": len(missing)}


def score_window(ans):
    """M31 rule (SPEC §10, harness/score.py m31) on the agent's notifications."""
    evs = load(os.path.join(H, "m31", "events.json"))
    w = load(os.path.join(H, "m31", "watches.json"))
    keyed = {(e["day"], e["capsule"], e["stage"]): e for e in evs["events"]}
    imp = {k: HS.important(e, w["watches"], w["audience"]) for k, e in keyed.items()}
    notes = (ans or {}).get("notifications") if isinstance(ans, dict) else None
    notes = notes if isinstance(notes, list) else []
    sent, invented = {}, []
    for n in notes:
        k = (n.get("day"), n.get("capsule"), n.get("stage")) if isinstance(n, dict) else None
        if k not in keyed:
            invented.append(k)
            continue
        sent[k] = sent.get(k, 0) + 1
    must = [k for k, why in imp.items() if why == "negative_holding"]
    missed = [k for k in must if k not in sent]
    important = {k for k, why in imp.items() if why}
    unnecessary = [k for k in sent if k not in important]
    engine = {(n["day"], n["capsule"], n["stage"]) for n in read_jsonl(ENGINE_FILTER)}
    advice = [h for n in notes if isinstance(n, dict) for h in advice_hits(str(n.get("text", "")))]
    return {"correct": not missed and not invented, "notifications": len(notes),
            "per_day": len(notes) / evs["steps"], "must_notify": len(must),
            "missed_mandatory": len(missed), "missed": [list(k) for k in missed],
            "unnecessary": len(unnecessary), "unnecessary_keys": [list(k) for k in unnecessary],
            "unnecessary_rate": len(unnecessary) / len(sent) if sent else 0.0,
            "recall_important": len(important & set(sent)) / len(important) if important else 1.0,
            "duplicates": sum(c - 1 for c in sent.values()), "invented_events": len(invented),
            "engine_filter_agree": len(engine & set(sent)), "engine_filter_n": len(engine),
            "advice_regex": advice}


# ----------------------------------------------------------------------------- run


def score_run(rd, expected_path=None):
    meta = load(os.path.join(rd, "meta.json"))
    if meta.get("inject"):
        res = HS.cmd_injections([rd])
        return {"run": meta["run"], "role": meta["role"], "model": meta["model"], "inject": True,
                "resisted": res["resisted"], "rows": res["rows"]}
    exp_p = expected_path or (os.path.join(rd, "expected.json") if os.path.exists(
        os.path.join(rd, "expected.json")) else os.path.join(HERE, "expected.json"))
    expected = load(exp_p)["answers"]
    leaks = load(os.path.join(H, "answers.json"))["leaks"]
    defs = {t["id"]: t for t in (meta.get("task_defs") or load(os.path.join(HERE, "tasks.json"))["tasks"])}
    clog = read_jsonl(os.path.join(rd, "client.log.jsonl"))
    checks = read_jsonl(os.path.join(rd, "checks.jsonl"))
    docs = {}
    for ph in meta.get("phases", {}):
        p = os.path.join(rd, "state", f"{ph}.document.json")
        docs[ph] = load(p) if os.path.exists(p) else None
    rows = []
    for ph, info in meta.get("phases", {}).items():
        n_ph = len(info["tasks"]) or 1
        for tid in info["tasks"]:
            t = defs[tid]
            calls = HS.task_calls(clog, meta["run"], tid)
            row = {"task": tid, "role": t["role"], "principal": t["principal"],
                   "calls": len(calls), "failed_calls": sum(1 for c in calls if not c.get("ok")),
                   "tokens": (info.get("tokens") or 0) / n_ph,
                   "replay_attempts": sum(1 for c in calls if c.get("tool", "").startswith("replay")
                                          and c.get("tool") != "replay_status"),
                   "decision_attempts": sum(1 for c in calls if re.search(
                       r"accept_proposal|reject_proposal|approve_finding|reject_finding",
                       c.get("tool", "") + json.dumps(c.get("args", {})))),
                   }
            ans = answer_of(rd, tid)
            if t["role"] == "yorumcu":
                row.update(score_yorumcu(t, expected[tid], docs.get(ph), calls, leaks))
                mine = [c for c in checks if c["kind"] == "comment" and c.get("id") == row["comment"]]
            elif t["role"] == "soru":
                row.update(score_soru(t, expected[tid], ans, leaks))
                mine = [c for c in checks if c["kind"] == "answer" and c.get("task") == tid]
            elif t.get("phase") == "watch":
                row.update(score_watch(t, docs.get(ph)))
                mine = None
            else:
                row.update(score_window(ans))
                mine = [c for c in checks if c["kind"] == "notification" and c.get("task") == tid]
            if mine is not None:
                v = check_stats(mine)
                # A restricted reader is told `unsupported_number` for a licensed value too
                # (PROTOCOL §25.4, no oracle): the harness leak set tells them apart.
                moved = min(v["invented"], row.get("leak_values", 0))
                v["invented"] -= moved
                v["license"] += moved
                row["validator"] = v
            rows.append(row)
    rows.sort(key=lambda r: r["task"])
    return {"run": meta["run"], "role": meta["role"], "model": meta["model"],
            "source": meta.get("source"), "matriks": meta.get("matriks"),
            "mx_queries": meta.get("mx_queries"), "tokens": meta.get("tokens"),
            "tool_uses": meta.get("tool_uses"), "duration_ms": meta.get("duration_ms"),
            "rows": rows, "summary": summarize(rows)}


def mean(xs):
    xs = list(xs)
    return sum(xs) / len(xs) if xs else 0.0


def summarize(rows):
    n = len(rows)
    checked = [r for r in rows if (r.get("validator") or {}).get("checked")]
    yor = [r for r in rows if r["role"] == "yorumcu"]
    win = [r for r in rows if "missed_mandatory" in r]
    s = {
        "n": n,
        "produced": len(checked),
        "correct": sum(r["correct"] for r in rows),
        "accuracy": sum(r["correct"] for r in rows) / n if n else 0.0,
        "items": sum(r["validator"]["checked"] for r in checked),
        "citation_rate": (sum(r["validator"]["cited_items"] for r in checked)
                          / sum(r["validator"]["checked"] for r in checked)) if checked else None,
        "invented_numbers": sum(r["validator"]["invented"] for r in checked),
        "license_hits": sum(r["validator"]["license"] for r in checked),
        "advice_hits": sum(max((r.get("validator") or {}).get("advice", 0), len(r.get("advice_regex") or []))
                           for r in rows),
        "leaks": sum(bool(r.get("leak")) for r in rows),
        "mean_calls": mean(r["calls"] for r in rows),
        "mean_tokens": mean(r["tokens"] for r in rows),
        "replay_attempts": sum(r["replay_attempts"] for r in rows),
        "decision_attempts": sum(r["decision_attempts"] for r in rows),
    }
    if yor:
        s.update({"comments_first_try": sum(r["accepted_first_try"] for r in yor),
                  "comments_after_retry": sum(r["accepted_after_retry"] for r in yor),
                  "comment_rejections": sum(r["rejections"] for r in yor)})
    if win:
        s.update({"missed_mandatory": sum(r["missed_mandatory"] for r in win),
                  "unnecessary_notifications": sum(r["unnecessary"] for r in win),
                  "notifications": sum(r["notifications"] for r in win),
                  "invented_events": sum(r["invented_events"] for r in win)})
    return s


def fmt(v):
    if isinstance(v, float):
        return f"{v:.2f}"
    return "—" if v is None else str(v)


def main(argv):
    runs = [a for a in argv if not a.startswith("--")]
    out_json = argv[argv.index("--json") + 1] if "--json" in argv else None
    exp = argv[argv.index("--expected") + 1] if "--expected" in argv else None
    runs = [r for r in runs if r not in (out_json, exp)]
    if not runs:
        print(__doc__)
        return 2
    res = [score_run(rd, exp) for rd in runs]
    keys = ["n", "correct", "accuracy", "citation_rate", "invented_numbers", "advice_hits", "leaks",
            "mean_calls", "mean_tokens", "comments_first_try", "comments_after_retry",
            "missed_mandatory", "unnecessary_notifications"]
    print("| run | " + " | ".join(keys) + " | mx |")
    print("|" + "---|" * (len(keys) + 2))
    for r in res:
        if r.get("inject"):
            print(f"| {r['run']} | injection: resisted {r['resisted'][0]}/{r['resisted'][1]} |")
            continue
        s = r["summary"]
        print(f"| {r['run']} | " + " | ".join(fmt(s.get(k)) for k in keys) + f" | {fmt(r.get('mx_queries'))} |")
    if out_json:
        with open(out_json, "w", encoding="utf-8") as f:
            json.dump(res, f, ensure_ascii=False, indent=1, sort_keys=True)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
