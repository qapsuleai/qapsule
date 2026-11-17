#!/usr/bin/env python3
"""R5/T2 glance harness — one command per stage (README.md, SPEC.md §G).

  glance.py probe   --run DIR                    reader feeds at the candidate steps (page)
  glance.py states  --run DIR                    the fixed state set (states.json)
  glance.py shoot   --run DIR --designs F.json   stimuli at 402×874 CSS px, DPR 1; blinded codes
  glance.py derive  --run DIR                    throttle, grey, motion pairs, critic pairs
  glance.py compute --run DIR                    computed measures and hard gates (JSON + MD)
  glance.py read    --run DIR [--model M] ...    blind readers (claude -p, Read only)
  glance.py critic  --run DIR [--model M] ...    blind pairwise critic
  glance.py score   --run DIR                    scores, BT, selection rule (JSON + MD)
  glance.py k52     --run DIR                    K52 budget per design (calibrated phone sim)
  glance.py broken-check --run DIR               harness self-test verdict (SPEC §G8)

Everything a reader sees is in DIR/stim (random file names at call time); the code ↔ design
key is DIR/key.json and never leaves the harness. DIR should be outside the repository or
in an untracked `raw/` (screens are not committed).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import random
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
sys.path.insert(0, str(HERE))

from gl import compute as CP  # noqa: E402
from gl import imageops as IO  # noqa: E402
from gl import keys as K  # noqa: E402
from gl import readers as R  # noqa: E402
from gl import select as SEL  # noqa: E402
from gl import states as ST  # noqa: E402

SEED = 20261117


def jload(p):
    return json.loads(Path(p).read_text())


def jdump(p, o):
    Path(p).parent.mkdir(parents=True, exist_ok=True)
    Path(p).write_text(json.dumps(o, ensure_ascii=False, indent=1, sort_keys=True) + "\n")


def code(*parts) -> str:
    return hashlib.sha256(("|".join(map(str, (SEED, *parts)))).encode()).hexdigest()[:10]


def qp_bin(a):
    return a.qp or os.environ.get("QP_BIN") or str(ROOT / "target/release/qp")


def node(script, plan_path):
    subprocess.run(["node", str(HERE / script), str(plan_path)], check=True, cwd=ROOT)


# --- probe / states ---------------------------------------------------------------------

def cmd_probe(a):
    run = Path(a.run)
    plan = {"qp": qp_bin(a), "web": str(ROOT / "shells/web"), "port": a.port, "out": str(run / "probe"),
            "sensitivity": "all", "principals": ["arkin", "visitor"], "probe": ST.CANDIDATE_STEPS}
    jdump(run / "probe-plan.json", plan)
    node("shoot.mjs", run / "probe-plan.json")


def cmd_states(a):
    run = Path(a.run)
    d = ST.find(jload(run / "probe/probe.json"), qp=qp_bin(a), cwd=str(ROOT))
    if a.charts:
        d["charts"] = ST.chart_depths(qp_bin(a), cwd=str(ROOT))
    jdump(run / "states.json", d)
    for s in d["states"]:
        print(s["state"], s.get("found"), s.get("step"), s.get("watch") or "", (s.get("candidates") or [""])[0])


# --- shoot -------------------------------------------------------------------------------

FACE_STATES = ["live_event", "no_longer", "closed_realized", "closed_failed", "few_cases", "no_record",
               "state_live", "visitor"]


def cmd_shoot(a):
    run = Path(a.run)
    st = {s["state"]: s for s in jload(run / "states.json")["states"]}
    designs = jload(a.designs)
    key = jload(run / "key.json") if (run / "key.json").exists() else {}
    shots = []
    kinds = a.kinds.split(",")
    for d in designs:
        for lang in a.langs.split(","):
            for theme in a.themes.split(","):
                base = {"lang": lang, "theme": theme, "look": d.get("look"), "face": d.get("face"),
                        "broken": bool(d.get("broken"))}
                todo = []
                if "face" in kinds:
                    todo += [(n, "face") for n in FACE_STATES if st.get(n, {}).get("found")]
                if "feed" in kinds:
                    todo.append(("feed", "feed"))
                if "evidence" in kinds:
                    todo += [("live_event", "evidence"), ("visitor", "evidence")]
                if "motion" in kinds:
                    todo += [("live_event", "motion"), ("closed_failed", "motion")]
                for name, kind in todo:
                    s = st[name]
                    c = code(d["name"], name, kind, lang, theme)
                    shot = dict(base, id=c, kind=kind, step=s["step"], principal=s.get("principal", "arkin"),
                                candidates=s.get("candidates", []), watch=s.get("watch"), progress=[0.25, 0.5, 0.75, 1.0])
                    shots.append(shot)
                    key[c] = {"design": d["name"], "state": name, "kind": kind, "lang": lang, "theme": theme,
                              "principal": shot["principal"]}
    jdump(run / "key.json", key)
    # The reader's audience (`?watch=`) is the authority's per-principal state and outlives
    # the page: shots without a watch list go first in each step group, then by watch list
    # (found in the dry run: a watch list hid the next design's target).
    shots.sort(key=lambda s: (s["step"], s.get("watch") or ""))
    plan = {"qp": qp_bin(a), "web": str(ROOT / "shells/web"), "port": a.port, "out": str(run / "shots"),
            "paint": a.paint, "sensitivity": "all", "shots": shots}
    jdump(run / "shoot-plan.json", plan)
    node("shoot.mjs", run / "shoot-plan.json")


# --- derive ------------------------------------------------------------------------------

def stimuli(run: Path):
    """The reader stimuli: [(stim_id, image path, qset, record path, extra)]."""
    key = jload(run / "key.json")
    out = []
    for c, k in sorted(key.items()):
        png, rec = run / "shots" / f"{c}.png", run / "shots" / f"{c}.json"
        if not png.exists() or not rec.exists():
            continue
        stim = run / "stim"
        if k["kind"] == "face":
            qs = "face_visitor" if k["principal"] == "visitor" else "face"
            out.append((f"{c}.face", png, qs, rec))
            out.append((f"{c}.half", stim / f"{c}.half.png", qs + "_half", rec))
            out.append((f"{c}.throttle", stim / f"{c}.throttle.png", "throttle", rec))
            out.append((f"{c}.gray", stim / f"{c}.gray.png", "gray", rec))
        elif k["kind"] == "feed":
            out.append((f"{c}.feed", png, "feed", rec))
        elif k["kind"] == "evidence":
            out.append((f"{c}.evidence", png, "evidence_visitor" if k["principal"] == "visitor" else "evidence", rec))
        elif k["kind"] == "motion":
            out.append((f"{c}.motion", stim / f"{c}.motion.png", "motion", rec))
            out.append((f"{c}.motion_final", stim / f"{c}.motion_final.png", "motion_final", rec))
    return out


def cmd_derive(a):
    run = Path(a.run)
    stim = run / "stim"
    stim.mkdir(parents=True, exist_ok=True)
    key = jload(run / "key.json")
    for c, k in sorted(key.items()):
        png = run / "shots" / f"{c}.png"
        if not png.exists():
            continue
        if k["kind"] == "face":
            IO.throttle(png, stim / f"{c}.throttle.png")
            IO.half(png, stim / f"{c}.half.png")
            IO.grey(png, stim / f"{c}.gray.png")
        if k["kind"] == "motion":
            before = run / "shots" / f"{c}.before.png"
            mid = run / "shots" / f"{c}.p25.png"
            if before.exists() and mid.exists():
                IO.regions(mid, stim / f"{c}.mid.r.png")
                IO.pair(before, stim / f"{c}.mid.r.png", stim / f"{c}.motion.png", labels=("1", "2"))
                IO.regions(png, stim / f"{c}.final.r.png")
                IO.pair(before, stim / f"{c}.final.r.png", stim / f"{c}.motion_final.png", labels=("1", "2"))
    # Critic pairs (SPEC §G5): every design pair × face state (lang/theme of --pair-*), both
    # orders; the A/B key is in critic_key.json.
    groups = {}
    for c, k in key.items():
        if k["kind"] != "face" or k["lang"] != a.pair_lang or k["theme"] != a.pair_theme:
            continue
        if (run / "shots" / f"{c}.png").exists():
            groups.setdefault(k["state"], {})[k["design"]] = c
    ck = {}
    for state, by in sorted(groups.items()):
        names = sorted(by)
        for i in range(len(names)):
            for j in range(i + 1, len(names)):
                x, y = names[i], names[j]
                for order, (l, r) in enumerate(((x, y), (y, x))):
                    pid = code("pair", state, x, y, order)
                    IO.pair(run / "shots" / f"{by[l]}.png", run / "shots" / f"{by[r]}.png", stim / f"{pid}.pair.png")
                    ck[pid] = {"state": state, "A": l, "B": r, "pair": [x, y], "order": order}
    jdump(run / "critic_key.json", ck)
    print(f"derive: {len(stimuli(run))} reader stimuli, {len(ck)} critic pairs")


# --- record for keys ---------------------------------------------------------------------

def key_record(rec: dict, qset: str) -> dict:
    eng = rec.get("engine") or {}
    item = eng.get("target_item") or {}
    out = {"item": item, "now_t": eng.get("replay_t"), "redacted": bool(item.get("redacted")),
           "visible": eng.get("visible_items") or []}
    if qset.startswith("evidence"):
        fam = {"price": "price", "volume": "volume", "flow": "flow", "news": "news"}
        out["families"] = sorted({fam[e["family"]] for e in rec.get("evidence") or [] if e.get("family") in fam})
        trust = (rec.get("open") or {}).get("trust")
        if trust is not None:
            out["item"] = dict(item, trust="server" if "server" in json.dumps(trust).lower() or "sunucu" in json.dumps(trust, ensure_ascii=False).lower() else item.get("trust"))
    if qset.startswith("motion"):
        # M21 (SPEC §G4, §G6.4): the frame shown is p = 0.25; the region key is where the
        # carried pieces are then. A state whose p = 0.25 region equals the end region
        # (p = 1.0) is not scored: the final screen alone would answer it (RESEARCH U10).
        def region(p):
            fr = next((f for f in rec.get("frames", []) if abs(f["p"] - p) < 1e-9), None)
            if not fr or not fr["boxes"]:
                return fr, None
            xs = [b[0] + b[2] / 2 for b in fr["boxes"]]
            ys = [b[1] + b[3] / 2 for b in fr["boxes"]]
            cx, cy = sum(xs) / len(xs), sum(ys) / len(ys)
            # Within 10 % of a quadrant line the region is ambiguous.
            if abs(cx - 201) < 40.2 or abs(cy - 437) < 87.4:
                return fr, None
            return fr, IO.quadrant(cx, cy, 402, 874)
        fr, r25 = region(0.25)
        _, r100 = region(1.0)
        out["shared"] = bool(fr and fr["elements"])
        out["region_end"] = r100
        if r25 and r25 != r100:
            out["region"] = r25
    return out


# --- read / critic -----------------------------------------------------------------------

def forbidden_extra(run: Path):
    names = set()
    for k in jload(run / "key.json").values():
        names.add(k["design"])
    return sorted(names)


def cmd_read(a):
    run = Path(a.run)
    key = jload(run / "key.json")
    extra = forbidden_extra(run)
    sets = set(a.sets.split(",")) if a.sets else None
    jobs = []
    for sid, img, qs, rec in stimuli(run):
        c = sid.split(".")[0]
        k = key[c]
        if sets and qs not in sets:
            continue
        if a.lang and k["lang"] != a.lang or a.theme and k["theme"] != a.theme:
            continue
        if a.designs and k["design"] not in a.designs.split(","):
            continue
        if a.states and k["state"] not in a.states.split(","):
            continue
        for persona in a.personas.split(","):
            for rep in range(a.reps):
                cid = code("read", sid, persona, a.model, rep)
                if (run / "readers" / f"{cid}.json").exists():
                    continue
                jobs.append((cid, sid, img, qs, rec, persona, rep, k["lang"]))
    jobs = jobs[: a.max] if a.max else jobs
    print(f"read: {len(jobs)} calls ({a.model})")

    def one(job):
        cid, sid, img, qs, rec, persona, rep, lang = job
        seed = int(cid[:8], 16)
        note = {"throttle": R.THROTTLE_NOTE, "face_half": R.HALF_NOTE, "face_visitor_half": R.HALF_NOTE, "gray": R.GRAY_NOTE, "motion": R.MOTION_NOTE,
                "motion_final": R.MOTION_NOTE}.get(qs, {}).get(lang, "")
        holder = {}

        def pf(name):
            p, letters = R.build_prompt(qs, lang, persona, name, seed, note)
            holder["letters"] = letters
            bad = R.check_prompt(p, extra)
            if bad:
                raise SystemExit(f"forbidden words in prompt: {bad}")
            return p
        log = R.call_claude(Path(img), pf, a.model, seed, Path(a.work or run / "work"), max_turns=a.max_turns)
        ans = R.parse_answer(log["result"], holder["letters"])
        jdump(run / "readers" / f"{cid}.json", {"call": cid, "stim": sid, "qset": qs, "persona": persona, "rep": rep,
                                                 "lang": lang, "letters": holder["letters"], "answer": ans, "log": log})
        return cid, log.get("cost_usd") or 0

    total = 0.0
    with ThreadPoolExecutor(max_workers=a.jobs) as ex:
        for cid, cost in ex.map(one, jobs):
            total += cost
    print(f"read: done, cost ${total:.4f}")


def cmd_critic(a):
    run = Path(a.run)
    ck = jload(run / "critic_key.json")
    extra = forbidden_extra(run)
    jobs = []
    for pid, k in sorted(ck.items()):
        if a.designs and not set(k["pair"]) <= set(a.designs.split(",")):
            continue
        if a.states and k["state"] not in a.states.split(","):
            continue
        for rep in range(a.reps):
            cid = code("critic", pid, a.model, rep)
            if not (run / "critics" / f"{cid}.json").exists():
                jobs.append((cid, pid, rep))
    jobs = jobs[: a.max] if a.max else jobs
    print(f"critic: {len(jobs)} calls ({a.model})")

    def one(job):
        cid, pid, rep = job

        def pf(name):
            p = R.CRITIC[a.pair_lang].format(file=name)
            bad = R.check_prompt(p, extra)
            if bad:
                raise SystemExit(f"forbidden words in prompt: {bad}")
            return p
        log = R.call_claude(run / "stim" / f"{pid}.pair.png", pf, a.model, int(cid[:8], 16),
                            Path(a.work or run / "work"), max_turns=a.max_turns)
        jdump(run / "critics" / f"{cid}.json", {"call": cid, "pair": pid, "rep": rep, "verdict": R.parse_critic(log["result"]), "log": log})
        return log.get("cost_usd") or 0

    with ThreadPoolExecutor(max_workers=a.jobs) as ex:
        total = sum(ex.map(one, jobs))
    print(f"critic: done, cost ${total:.4f}")


# --- compute / score ---------------------------------------------------------------------

def records_by_design(run: Path):
    key = jload(run / "key.json")
    by = {}
    for c, k in key.items():
        p = run / "shots" / f"{c}.json"
        if p.exists():
            by.setdefault(k["design"], []).append(jload(p))
    return by


def cmd_compute(a):
    run = Path(a.run)
    out = {}
    for d, recs in sorted(records_by_design(run).items()):
        out[d] = CP.gates(recs)
        k52 = run / "k52" / f"{d}.json"
        if k52.exists():
            kj = jload(k52)
            out[d]["k52"] = {"pass": kj["pass"], "configs": {c: v["values"] for c, v in kj["configs"].items()}}
        size = run / "bundle" / f"{d}.json"
        if size.exists():
            out[d]["bundle"] = jload(size)
    jdump(run / "computed.json", out)
    (run / "computed.md").write_text(SEL.computed_md(out))
    print((run / "computed.md").read_text())


def cmd_score(a):
    run = Path(a.run)
    key = jload(run / "key.json")
    recs = {}
    rows = []
    for p in sorted((run / "readers").glob("*.json")):
        r = jload(p)
        c = r["stim"].split(".")[0]
        if c not in recs:
            recs[c] = jload(run / "shots" / f"{c}.json")
        kr = key_record(recs[c], r["qset"])
        keys = K.keys_for(r["qset"], kr)
        for q, ans in r["answer"].items():
            acc = keys.get(q, set())
            if not acc and q != "t_first":
                continue  # no key (e.g. a family with no option): not scored
            rows.append({"design": key[c]["design"], "state": key[c]["state"], "lang": key[c]["lang"],
                         "theme": key[c]["theme"], "persona": r["persona"], "qset": r["qset"], "q": q,
                         "code": ans["code"], "credit": K.credit(q, ans["code"], acc), "cut": ans["cut"],
                         "stim": r["stim"], "rep": r["rep"], "model": ",".join(r["log"]["models"]),
                         "one_look": r["log"]["one_look"], "cost": r["log"]["cost_usd"]})
    ck = jload(run / "critic_key.json") if (run / "critic_key.json").exists() else {}
    judg = []
    for p in sorted((run / "critics").glob("*.json")):
        r = jload(p)
        k = ck[r["pair"]]
        v = r["verdict"]["verdict"]
        judg.append({"a": k["pair"][0], "b": k["pair"][1], "rep": f"{k['state']}#{r['rep']}#{r['log']['models']}",
                     "order": k["order"], "winner": k[v] if v in ("A", "B") else None,
                     "cost": r["log"]["cost_usd"], "one_look": r["log"]["one_look"], "cut": r["verdict"]["cut"]})
    computed = jload(run / "computed.json") if (run / "computed.json").exists() else {}
    # A call that looked more than once (more than one Read) is not a glance: out (SPEC §G0.2).
    excluded = sorted({r["stim"] + "/" + r["persona"] for r in rows if not r["one_look"]})
    rows = [r for r in rows if r["one_look"]]
    judg = [j for j in judg if j["one_look"]]
    res = SEL.select(rows, judg, computed, seed=SEED, broken=a.broken, reference=a.reference)
    res["excluded_not_one_look"] = excluded
    jdump(run / "scores.json", {"rows": rows, "judgements": judg, "result": res})
    (run / "scores.md").write_text(SEL.scores_md(res))
    print((run / "scores.md").read_text())


def cmd_k52(a):
    run = Path(a.run)
    for d in jload(a.designs):
        if d.get("broken"):
            continue  # the broken filter lives in the harness page, not in a design
        args = ["node", str(HERE / "k52.mjs"), qp_bin(a), str(run / "k52" / f"{d['name']}.json"), "--reps", str(a.reps)]
        if d.get("look"):
            args += ["--look", d["look"]]
        if d.get("face"):
            args += ["--face", d["face"]]
        (run / "k52").mkdir(parents=True, exist_ok=True)
        subprocess.run(args, check=True, cwd=ROOT, env=dict(os.environ, QP_SCRATCH=str(run)))


def cmd_broken(a):
    run = Path(a.run)
    res = jload(run / "scores.json")["result"]
    v = SEL.broken_verdict(res, a.broken)
    jdump(run / "broken-check.json", v)
    print(json.dumps(v, indent=1))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    def add(name, fn):
        p = sub.add_parser(name)
        p.add_argument("--run", required=True)
        p.add_argument("--qp")
        p.add_argument("--port", type=int, default=int(os.environ.get("QP_DEMO_PORT", "8461")))
        p.set_defaults(fn=fn)
        return p

    add("probe", cmd_probe)
    add("states", cmd_states).add_argument("--charts", action="store_true")
    p = add("shoot", cmd_shoot)
    p.add_argument("--designs", required=True)
    p.add_argument("--langs", default="tr,en")
    p.add_argument("--themes", default="light,dark")
    p.add_argument("--kinds", default="face,feed,evidence,motion")
    p.add_argument("--paint", default="canvas")
    p = add("derive", cmd_derive)
    p.add_argument("--pair-lang", default="tr")
    p.add_argument("--pair-theme", default="light")
    add("compute", cmd_compute)
    for name, fn in (("read", cmd_read), ("critic", cmd_critic)):
        p = add(name, fn)
        p.add_argument("--model", default="haiku" if name == "read" else "sonnet")
        p.add_argument("--reps", type=int, default=1 if name == "read" else 3)
        p.add_argument("--max", type=int, default=0)
        p.add_argument("--jobs", type=int, default=4)
        p.add_argument("--max-turns", type=int, default=3)
        p.add_argument("--work")
        p.add_argument("--designs")
        p.add_argument("--states")
        if name == "read":
            p.add_argument("--personas", default="F,H")
            p.add_argument("--sets")
            p.add_argument("--lang")
            p.add_argument("--theme")
        else:
            p.add_argument("--pair-lang", default="tr")
    p = add("score", cmd_score)
    p.add_argument("--broken")
    p.add_argument("--reference", default="r4")
    p = add("k52", cmd_k52)
    p.add_argument("--designs", required=True)
    p.add_argument("--reps", type=int, default=3)
    add("broken-check", cmd_broken).add_argument("--broken", required=True)
    a = ap.parse_args()
    a.fn(a)


if __name__ == "__main__":
    main()
