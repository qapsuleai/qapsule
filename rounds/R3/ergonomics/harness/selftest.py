#!/usr/bin/env python3
"""R3/E scorer self-check (no LLM): through the real daemon and run.sh, (1) reference
solutions written with ids learned from a discovery run must pass 20/20, (2) empty attempts
(hello only) must fail 20/20. usage: selftest.py   (the daemon `e3.py serve` must be running)
Writes raw/selftest-ref and raw/selftest-empty; prints both score tables.
"""

import json
import os
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import e3  # noqa: E402
import score  # noqa: E402

RUN = os.path.join(e3.packet_dir(), "run.sh")
OPEN = score.OPEN


def hello(p="editor"):
    return {"conn": "a", "msg": {"type": "hello", "protocol": "qp/0", "principal": p, "surface": {"kind": "headless"}}}


def q(i, query):
    return {"conn": "a", "msg": {"type": "query", "id": i, "query": query}}


def cmd(i, ops):
    return {"conn": "a", "msg": {"type": "command", "id": i, "ops": ops}}


def run(path, lines, *png):
    with open(path, "w") as f:
        f.write("\n".join(json.dumps(x, ensure_ascii=False) for x in lines) + "\n")
    out = subprocess.run([RUN, path] + list(png), capture_output=True, text=True).stdout
    res = {}
    for l in out.splitlines():
        try:
            m = json.loads(l)["msg"]
        except (ValueError, KeyError, TypeError):
            continue
        if "id" in m:
            res[m["id"]] = m
    return res, out


def setup_agent(label, agent):
    ap = e3.agent_paths(label, agent)
    for d in (ap["work"], ap["raw"]):
        if os.path.exists(d):
            shutil.rmtree(d)
    e3.prepare_setup(label, agent, e3.AGENTS[agent])
    os.makedirs(ap["work"])
    for t in e3.AGENTS[agent]:
        os.makedirs(os.path.join(ap["work"], t))
    e3.issue_token(label, agent, ap["work"])
    return ap


def copy_back(ap, agent):
    for t in e3.AGENTS[agent]:
        src, dst = os.path.join(ap["work"], t), os.path.join(ap["raw"], t)
        os.makedirs(dst, exist_ok=True)
        for f in os.listdir(src):
            if os.path.isfile(os.path.join(src, f)) and not f.endswith(".png"):
                shutil.copy(os.path.join(src, f), os.path.join(dst, f))


def kindw(k):
    return score.KIND_WORDS[k][0]


def solve(t, d, caps, n_bars):
    """Reference solution for task t (ids from the discovery run); returns answer text."""
    f = lambda sym=None, kind=None, stage=None: [
        x for x in caps if (sym is None or x["instrument"] == sym) and (kind is None or x["kind"] == kind)
        and (stage is None or (x["stage"] in OPEN if stage == "open" else x["stage"] == stage))]
    latest = lambda xs: max(xs, key=lambda x: (x["last_bar"], x["born_bar"]))
    brk = f("THYAO", "breakout", "open")[0]
    a1 = os.path.join(d, "attempt1.jsonl")
    ans = ""
    if t == "C01":
        r, _ = run(a1, [hello(), q("q", {"kind": "capsules", "instrument": "THYAO", "open": True})])
        ans = "\n".join(f"{x['id']} ({kindw(x['kind'])}): {x['sentence']['tr']}" for x in r["q"]["data"]["capsules"])
    elif t == "C02":
        run(a1, [hello(), q("q", {"kind": "capsule", "capsule": brk["id"]})])
    elif t == "C03":
        a = latest(f("ASELS", stage="invalidated"))
        r, _ = run(a1, [hello(), q("q", {"kind": "capsule", "capsule": a["id"]})])
        h = [x for x in r["q"]["data"]["detail"]["story"]["history"] if x["stage"] == "invalidated"][-1]
        ans = f"{a['id']} bar {h['bar']}'de geçersiz oldu; neden: {h['reason']}."
    elif t == "C04":
        run(a1, [hello(), cmd("c", [{"op": "reject_finding", "finding": brk["finding"]}])])
    elif t == "C05":
        r, _ = run(a1, [hello(), q("q", {"kind": "capsules", "limit": 3})])
        ans = "\n".join(f"{i + 1}. {x['id']}" for i, x in enumerate(r["q"]["data"]["capsules"]))
    elif t == "C06":
        r, _ = run(a1, [hello(), q("q", {"kind": "ledger", "meaning": "breakout", "by_kind": True})])
        row = r["q"]["data"]["rows"][0]
        ans = f"Kırılım: {row['n']} kapanış, {row['realized']} gerçekleşti."
    elif t == "C07":
        run(a1, [hello(), cmd("c", [{"op": "set_sensitivity", "sensitivity": "important"}])])
    elif t == "C08":
        run(a1, [hello(), cmd("c", [{"op": "set_audience", "audience": {"group": "habituated", "watchlist": ["ASELS"]}}])])
    elif t == "C09":
        e = f("THYAO", "extreme", "open")[0]
        run(a1, [hello(), q("q", {"kind": "present", "finding": e["finding"], "surface": "phone"})])
    elif t == "C10":
        run(a1, [hello(), q("q", {"kind": "present", "finding": brk["finding"]})])
        ans = "Evet, kırılan seviye kural olarak çiziliyor."
    elif t == "C11":
        r, _ = run(a1, [hello("viewer"), q("q", {"kind": "capsules", "instrument": "THYAO", "meaning": "flow"})])
        ent = r["q"]["data"]["comments"][-1]["attachments"]["entitlement"]
        ans = f"Kanıt değerleri açılamıyor; {ent} yetkisi gerekir."
    elif t == "C12":
        run(a1, [hello(), {"conn": "a", "msg": {"type": "subscribe", "id": "s", "topic": {"kind": "capsules", "instrument": "THYAO"}}}])
    elif t == "C13":
        a = latest(f("THYAO", "breakout", "realized"))
        r, _ = run(a1, [hello(), q("q", {"kind": "capsule", "capsule": a["id"]})])
        ans = "\n".join(f"bar {h['bar']}: {h['stage']}" for h in r["q"]["data"]["detail"]["story"]["history"])
    elif t == "C14":
        run(a1, [hello(), q("q", {"kind": "verify_evidence", "item": brk["finding"]})])
    elif t == "C15":
        run(a1, [hello(), {"conn": "a", "msg": {"type": "dispute", "id": "x", "claim": {"kind": "capsule", "capsule": brk["id"], "finding": brk["finding"]}, "t": brk["t"]}}])
    elif t == "C16":
        s = latest(f("THYAO", "squeeze"))
        run(a1, [hello(), q("q", {"kind": "capsule", "capsule": s["id"]})])
        ans = "Evet, hâlâ geçerli." if s["stage"] in OPEN else "Hayır, geçerli değil."
    elif t == "C17":
        r, _ = run(a1, [hello(), q("q", {"kind": "capsules", "since_bar": n_bars - 10})])
        cnt = {}
        for x in r["q"]["data"]["capsules"]:
            if x["born_bar"] >= n_bars - 10:
                cnt[x["kind"]] = cnt.get(x["kind"], 0) + 1
        ans = "\n".join(f"{kindw(k)}: {v}" for k, v in sorted(cnt.items()))
    elif t == "C18":
        run(a1, [hello(), cmd("c", [{"op": "set_audience", "audience": {"group": "habituated", "positions": {"THYAO": 100}}}])])
    elif t == "C19":
        run(a1, [hello(), q("q", {"kind": "capsules", "stages": ["realized"]})])
    elif t == "C20":
        run(a1, [hello()], "--png", os.path.join(d, "k.png"), "--present", brk["finding"], "--surface", "desktop")
    if ans:
        with open(os.path.join(d, "answer.txt"), "w") as fh:
            fh.write(ans + "\n")


def main():
    for label, mode in (("selftest-ref", "ref"), ("selftest-empty", "empty")):
        for agent in "ABCDE":
            ap = setup_agent(label, agent)
            n_bars = json.load(open(os.path.join(ap["raw"], "setup.json")))["n_bars"]
            for t in e3.AGENTS[agent]:
                d = os.path.join(ap["work"], t)
                if mode == "empty":
                    run(os.path.join(d, "attempt1.jsonl"), [hello("viewer" if t == "C11" else "editor")])
                    continue
                r, _ = run(os.path.join(d, "probe.jsonl"), [hello(), q("q", {"kind": "capsules"})])
                solve(t, d, r["q"]["data"]["capsules"], n_bars)
            copy_back(ap, agent)
    score.main(["selftest-ref", "selftest-empty"])


if __name__ == "__main__":
    main()
