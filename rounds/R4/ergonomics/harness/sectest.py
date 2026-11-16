#!/usr/bin/env python3
"""R4/E addition: harness hardening check (no daemon, no LLM). The attempt file and the --png
target are reached only by an O_NOFOLLOW walk from the agent's work dir. Checks: a normal
attempt runs; a symlinked attempt file, a hard-linked attempt file and an attempt behind a
symlinked intermediate dir are refused; a symlinked --png target and a --png path through a
symlinked intermediate dir are refused and nothing is written outside; a normal --png path
is accepted. usage: sectest.py (cleans up its own `sectest` label)."""

import json
import os
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import e3  # noqa: E402

HELLO = json.dumps({"conn": "a", "msg": {"type": "hello", "protocol": "qp/0", "principal": "editor",
                                         "surface": {"kind": "headless"}}}) + "\n"


def main():
    ap = e3.agent_paths("sectest", "A")
    out_dir = os.path.join(e3.SCR, "sectest-outside")
    for d in (ap["work"], ap["raw"], ap["docs"], out_dir):
        shutil.rmtree(d, ignore_errors=True)
    os.makedirs(out_dir)
    e3.prepare_setup("sectest", "A", ["C01", "C02"])
    w = ap["work"]
    os.makedirs(os.path.join(w, "C01"))
    e3.issue_token("sectest", "A", w)
    tok = open(os.path.join(w, ".qp-token")).read().strip()
    good = os.path.join(w, "C01", "attempt1.jsonl")
    open(good, "w").write(HELLO)
    secret = os.path.join(out_dir, "secret.jsonl")
    open(secret, "w").write(HELLO)
    os.symlink(secret, os.path.join(w, "C01", "attempt2.jsonl"))
    os.link(secret, os.path.join(w, "C01", "attempt3.jsonl"))
    os.symlink(out_dir, os.path.join(w, "C02"))  # symlinked intermediate dir
    shutil.copy(secret, os.path.join(out_dir, "attempt1.jsonl"))
    res = []

    def chk(name, cond):
        res.append((name, bool(cond)))

    chk("normal attempt runs", '"welcome"' in e3.handle_attempt(good, tok))
    chk("symlinked attempt refused", e3.handle_attempt(os.path.join(w, "C01", "attempt2.jsonl"), tok).startswith("error: cannot read"))
    chk("hard-linked attempt refused", e3.handle_attempt(os.path.join(w, "C01", "attempt3.jsonl"), tok).startswith("error: cannot read"))
    chk("attempt behind symlinked dir refused", e3.handle_attempt(os.path.join(w, "C02", "attempt1.jsonl"), tok).startswith("error"))
    os.symlink(os.path.join(out_dir, "x.png"), os.path.join(w, "C01", "x.png"))
    r = e3.handle_attempt(good, tok, {"out": os.path.join(w, "C01", "x.png"), "view": "main"})
    chk("symlinked png target refused", "png error" in r and not os.path.exists(os.path.join(out_dir, "x.png")))
    r = e3.handle_attempt(good, tok, {"out": os.path.join(w, "C02", "y.png"), "view": "main"})
    chk("png through symlinked dir refused", "png error" in r and not os.path.exists(os.path.join(out_dir, "y.png")))
    r = e3.handle_attempt(good, tok, {"out": os.path.join(w, "..", "agentB", "z.png"), "view": "main"})
    chk("png outside work dir refused", "png error: --png must name" in r)
    r = e3.handle_attempt(good, tok, {"out": os.path.join(w, "C01", "ok.png"), "view": "nope"})
    chk("normal png path reaches the renderer", "png error: --png must name" not in r)
    for d in (ap["work"], ap["raw"], ap["docs"], out_dir, os.path.join(e3.SCR, "full", "sectest")):
        shutil.rmtree(d, ignore_errors=True)
    for n, ok in res:
        print(("PASS " if ok else "FAIL ") + n)
    return 0 if all(ok for _, ok in res) else 1


if __name__ == "__main__":
    sys.exit(main())
