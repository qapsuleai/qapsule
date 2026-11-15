#!/usr/bin/env python3
"""Audit the stream-json transcripts: every file the agent touched (VERIFY 12).

usage: audit.py <label> [variant ...]
Every tool call is paired with its result and classified as executed or denied (EPERM from
the OS sandbox, or Claude Code's permission layer). Bounds: Read in the packet or the own
work dir; Write/Edit in the own work dir; Bash = the packet's run.sh (optionally after
`cd <own dir> &&`, optionally piped into head/tail/cut) on a file in the own work dir.
An EXECUTED call outside these bounds is a violation; denied calls are only counted.
Bash is judged strictly (see bash_ok): compound commands that stayed inside the own work dir
(`cd <own> && run.sh ...`, `echo ... > R05/notes.txt`) are still reported as violations and
were reviewed by hand (RESULTS.md)."""

import glob
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import t9  # noqa: E402

DENIED = ("EPERM", "haven't granted", "require approval", "requires approval", "was blocked",
          "can't be checked", "expansion obfuscation", "Operation not permitted", "Brace expansion")


def bash_ok(cmd, work, pkt, inside):
    """Strict rule (shlex): exactly `<packet>/run.sh <file in own work dir>`, optionally piped
    into `head`/`tail` with option-only flags (`-n N`, `-c N`, `-N`, `-n +N`); no newlines, no
    other operators, no positional operands. Anything else is a violation.
    Returns (ok, number of run.sh calls)."""
    import shlex
    if "\n" in cmd:
        return False, 0
    try:
        lex = shlex.shlex(cmd, posix=True, punctuation_chars=True)
        lex.whitespace_split = True
        toks = list(lex)
    except ValueError:
        return False, 0
    segs, cur = [], []
    for t in toks:
        if t == "|":
            segs.append(cur)
            cur = []
        elif t in ("||", "&&", ";", "&", ">", ">>", "<", "|&", "(", ")") or set(t) <= set("|&;<>()"):
            return False, 0
        else:
            cur.append(t)
    segs.append(cur)
    first = segs[0]
    if len(first) != 2 or os.path.realpath(first[0]) != os.path.join(pkt, "run.sh") \
            or not os.path.isabs(first[0]) and False or not inside(first[1], work, work):
        return False, 0
    for seg in segs[1:]:
        if not seg or seg[0] not in ("head", "tail"):
            return False, 1
        i = 1
        while i < len(seg):
            t = seg[i]
            if t in ("-n", "-c") and i + 1 < len(seg) and re.fullmatch(r"\+?\d+", seg[i + 1]):
                i += 2
            elif re.fullmatch(r"-(\d+|[nc]\+?\d+)", t):
                i += 1
            else:
                return False, 1
    return True, 1


def audit_agent(adir, label, variant, agent):
    work = os.path.realpath(t9.agent_paths(label, variant, agent)["work"])
    pkt = os.path.realpath(t9.packet_dir(variant))

    def inside(p, root, cwd):
        p = os.path.realpath(p if os.path.isabs(p) else os.path.join(cwd, p))
        return p == root or p.startswith(root + "/")

    calls = {}
    order = []
    for l in open(os.path.join(adir, "transcript.jsonl"), encoding="utf-8"):
        e = json.loads(l)
        if e.get("type") == "assistant":
            for c in e["message"].get("content", []):
                if c.get("type") == "tool_use":
                    calls[c["id"]] = {"tool": c["name"], "input": c.get("input", {}), "result": ""}
                    order.append(c["id"])
        elif e.get("type") == "user":
            for c in e["message"].get("content", []):
                if isinstance(c, dict) and c.get("type") == "tool_result" and c.get("tool_use_id") in calls:
                    calls[c["tool_use_id"]]["result"] = json.dumps(c.get("content"), ensure_ascii=False)
    out = {"calls": len(order), "executed": 0, "denied": 0, "reads": 0, "writes": 0, "runs": 0,
           "violations": [], "denied_list": []}
    for cid in order:
        c = calls[cid]
        n, i = c["tool"], c["input"]
        denied = any(k in c["result"] for k in DENIED)
        ok, what = True, ""
        if n == "Read":
            ok = inside(i.get("file_path", ""), pkt, work) or inside(i.get("file_path", ""), work, work)
            what = i.get("file_path")
            out["reads"] += not denied
        elif n in ("Write", "Edit"):
            ok = inside(i.get("file_path", ""), work, work)
            what = i.get("file_path")
            out["writes"] += not denied
        elif n == "Bash":
            cmd = i.get("command", "")
            what = cmd[:200]
            ok, nrun = bash_ok(cmd, work, pkt, inside)
            out["runs"] += nrun if not denied else 0
        else:
            ok, what = False, n
        if denied:
            out["denied"] += 1
            out["denied_list"].append((n, what))
        else:
            out["executed"] += 1
            if not ok:
                out["violations"].append((n, what))
    return out


def main(argv):
    label = argv[0]
    tot_v = tot_d = 0
    for v in argv[1:] or ["base", "A", "B"]:
        for adir in sorted(glob.glob(os.path.join(t9.RAW, label, v, "agent?"))):
            a = os.path.basename(adir)[-1]
            r = audit_agent(adir, label, v, a)
            tot_v += len(r["violations"])
            tot_d += r["denied"]
            print(f"{label}/{v}/agent{a}: tool calls {r['calls']} (executed {r['executed']}: reads {r['reads']}, "
                  f"writes {r['writes']}, run.sh {r['runs']}; denied {r['denied']}); "
                  f"executed out of bounds: {len(r['violations'])}")
            for x in r["violations"]:
                print("    VIOLATION", x)
            for x in r["denied_list"]:
                print("    denied:", x[0], (x[1] or "")[:110].replace("\n", "\\n"))
    print(f"TOTAL executed out-of-bounds calls: {tot_v}; denied calls: {tot_d}")


if __name__ == "__main__":
    main(sys.argv[1:])
