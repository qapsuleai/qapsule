#!/usr/bin/env python3
"""R4/E addition (not in R3): scorer check with id-free (selector) reference solutions.

R4/T3 added `CapsuleSel` and story keys in `FindingSel`; this checks that the unchanged R3
scorer credits selector-based solutions (the authority writes only ids into the log and
replies, PROTOCOL §17.4). Tasks without a selector path (C15 `dispute.claim`, C20
`--present <id>`) keep the id-based reference. usage: selftest_sel.py (daemon running).
Writes raw/selftest-sel; prints the score table.
"""

import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import e3  # noqa: E402
import score  # noqa: E402
import selftest as st  # noqa: E402

BRK_F = {"instrument": "THYAO", "meaning": "breakout", "open": True}
SEL = {
    "C02": lambda: [st.q("q", {"kind": "capsule", "capsule": {"instrument": "THYAO", "meaning": "breakout", "open": True}})],
    "C03": lambda: [st.q("q", {"kind": "capsule", "capsule": {"instrument": "ASELS", "stage": "invalidated", "latest": True}})],
    "C04": lambda: [st.cmd("c", [{"op": "reject_finding", "finding": BRK_F}])],
    "C09": lambda: [st.q("q", {"kind": "present", "finding": {"instrument": "THYAO", "meaning": "extreme", "open": True}, "surface": "phone"})],
    "C10": lambda: [st.q("q", {"kind": "present", "finding": BRK_F})],
    "C13": lambda: [st.q("q", {"kind": "capsule", "capsule": {"instrument": "THYAO", "meaning": "breakout", "stage": "realized", "latest": True}})],
    "C14": lambda: [st.q("q", {"kind": "verify_evidence", "item": {"finding": BRK_F}})],
    "C16": lambda: [st.q("q", {"kind": "capsule", "capsule": {"instrument": "THYAO", "meaning": "squeeze", "latest": True}})],
}


def solve_sel(t, d, caps, n_bars):
    if t not in SEL:
        return st.solve(t, d, caps, n_bars)
    r, _ = st.run(os.path.join(d, "attempt1.jsonl"), [st.hello()] + SEL[t]())
    m = r.get("q") or r.get("c") or {}
    ans = ""
    if t == "C03":
        h = [x for x in m["data"]["detail"]["story"]["history"] if x["stage"] == "invalidated"][-1]
        ans = f"bar {h['bar']}'de geçersiz oldu; neden: {h['reason']}."
    elif t == "C13":
        ans = "\n".join(f"bar {h['bar']}: {h['stage']}" for h in m["data"]["detail"]["story"]["history"])
    elif t == "C10":
        ans = "Evet, kırılan seviye kural olarak çiziliyor."
    elif t == "C16":
        stage = m["data"]["detail"]["story"]["stage"]
        ans = "Evet, hâlâ geçerli." if stage in st.OPEN else "Hayır, geçerli değil."
    if ans:
        with open(os.path.join(d, "answer.txt"), "w") as fh:
            fh.write(ans + "\n")


def main():
    label = "selftest-sel"
    for agent in "ABCDE":
        ap = st.setup_agent(label, agent)
        n_bars = json.load(open(os.path.join(ap["raw"], "setup.json")))["n_bars"]
        for t in e3.AGENTS[agent]:
            d = os.path.join(ap["work"], t)
            r, _ = st.run(os.path.join(d, "probe.jsonl"), [st.hello(), st.q("q", {"kind": "capsules"})])
            solve_sel(t, d, r["q"]["data"]["capsules"], n_bars)
        st.copy_back(ap, agent)
    score.main([label])


if __name__ == "__main__":
    main()
