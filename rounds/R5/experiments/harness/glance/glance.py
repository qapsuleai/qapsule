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
# SPEC Ek G3: replicates per reader set (primary measure 6; BROKEN.md: 3 did not separate).
DEFAULT_REPS = {"throttle": 6, "chart": 6, "cap_evidence": 6, "cap_evidence_visitor": 6}  # M20: Ek G7a  # M18: `chart` is the primary set (SPEC §G9)


def jload(p):
    return json.loads(Path(p).read_text())


def jdump(p, o):
    Path(p).parent.mkdir(parents=True, exist_ok=True)
    Path(p).write_text(json.dumps(o, ensure_ascii=False, indent=1, sort_keys=True) + "\n")


def code(*parts) -> str:
    return hashlib.sha256(("|".join(map(str, (SEED, *parts)))).encode()).hexdigest()[:10]


def qp_bin(a):
    return a.qp or os.environ.get("QP_BIN") or str(ROOT / "target/release/qp")


def web_dir(a):
    """The shell directory served by `qp demo` (a broken-look build has its own, SPEC §G8)."""
    return getattr(a, "web", None) or os.environ.get("QP_WEB") or str(ROOT / "shells/web")


VOICE_HELP = ("capsule voice for `?voice=` (marla, selvi, r4); default: none sent, the product's "
              "presentation default (the core sends marla); the same voice for every design")


def voice_of(a, d=None):
    """(Ek G4-1) The capsule voice of a shot or K52 run, the same for every design: `--voice`,
    else None = no `?voice=` (the product's presentation default). M16 is measured with one
    text for every design (`--voice marla`). `d` is accepted for call-site symmetry."""
    return getattr(a, "voice", None) or None


def node(script, plan_path):
    subprocess.run(["node", str(HERE / script), str(plan_path)], check=True, cwd=ROOT)


# --- probe / states ---------------------------------------------------------------------

def cmd_probe(a):
    run = Path(a.run)
    plan = {"qp": qp_bin(a), "web": web_dir(a), "port": a.port, "out": str(run / "probe"),
            "sensitivity": "all", "voice": voice_of(a), "principals": ["arkin", "visitor"], "probe": ST.CANDIDATE_STEPS}
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


# (M18) Chart states (GOAL M18: "görünen aralıkta 5 / 50 / 200 kapsül"). One synthetic market
# long enough for 200 stories in view (`--days`, `--history`: the story list holds the
# history's stories, so the replay itself stays short); the depths are visible-range story
# counts reached by zooming the same opened chart (shoot.mjs `zoomTo`). Rule, written before
# any shot of an alternative:
#   instrument = the first by symbol (among the reader's feed at the last step) whose list has
#     >= 200 stories, at least one live (not closed) story, and whose most recently closed
#     story closed within the last 10 bars (in view at every depth);
#   target (the opened capsule) = that most recently closed story (ties: later born, id).
CHART_MARKET = ["--days", "2600", "--history", "2500"]
CHART_DEPTHS = (5, 50, 200)


def cmd_chartstates(a):
    run = Path(a.run)
    plan = {"qp": qp_bin(a), "web": web_dir(a), "port": a.port, "out": str(run / "probe"), "sensitivity": "all",
            "demo_args": CHART_MARKET, "chart_probe": True, "voice": voice_of(a)}
    jdump(run / "chart-probe-plan.json", plan)
    if not (run / "probe" / "chart-probe.json").exists():
        node("shoot.mjs", run / "chart-probe-plan.json")
    pr = jload(run / "probe" / "chart-probe.json")
    chosen = None
    for inst in sorted(pr["instruments"]):
        st = pr["instruments"][inst].get("stories") or {}
        rows = st.get("stories") or []
        if len(rows) < 200:
            continue
        last = max(int(r["last_bar"]) for r in rows)
        live = [r for r in rows if r.get("closed_bar") is None]
        closed = [r for r in rows if r.get("closed_bar") is not None]
        if not live or not closed:
            continue
        tgt = max(closed, key=lambda r: (int(r["closed_bar"]), int(r["born_bar"]), r["id"]))
        if last - int(tgt["closed_bar"]) > 10:
            continue
        chosen = (inst, tgt, rows, live, last, st)
        break
    if not chosen:
        raise SystemExit("chartstates: no instrument satisfies the rule")
    inst, tgt, rows, live, last, st = chosen
    d = jload(run / "states.json") if (run / "states.json").exists() else {"states": []}
    d["charts"] = [{"target": n, "market": CHART_MARKET, "instrument": inst, "candidates": [tgt["id"]],
                    "opened": tgt, "live": [r["id"] for r in live], "stories_total": len(rows), "last_bar": last,
                    "status": pr.get("status")} for n in CHART_DEPTHS]
    d["chart_stories"] = st
    jdump(run / "states.json", d)
    print(f"chartstates: {inst}, {len(rows)} stories, live {[r['id'] for r in live]}, target {tgt['id']} "
          f"(closed bar {tgt['closed_bar']}, last {last})")


# --- shoot -------------------------------------------------------------------------------

FACE_STATES = ["live_event", "no_longer", "closed_realized", "closed_failed", "few_cases", "no_record",
               "state_live", "visitor"]


def shoot_charts(a, run, designs, key):
    """(M18) One plan per design: each design may come from its own build (`qp`, `web` in
    designs.json: the M18 alternatives cannot be merged). Same market, same replay step (the
    last), same opened capsule, same zoom rule for every design; the canonical story list
    (states.json) is passed for the zoom count (the reference device may hold none)."""
    sj = jload(run / "states.json")
    charts = sj.get("charts") or []
    canon = (sj.get("chart_stories") or {}).get("stories")
    for d in designs:
        shots = []
        for ch in charts:
            for lang in a.langs.split(","):
                for theme in a.themes.split(","):
                    name = f"chart{ch['target']}"
                    c = code(d["name"], name, "chart", lang, theme)
                    key[c] = {"design": d["name"], "state": name, "kind": "chart", "lang": lang, "theme": theme,
                              "principal": "arkin", "instrument": ch["instrument"], "voice": voice_of(a, d)}
                    if (run / "shots" / f"{c}.png").exists():
                        continue
                    shots.append({"id": c, "kind": "chart", "step": -1, "principal": "arkin", "lang": lang,
                                  "theme": theme, "look": d.get("look"), "face": d.get("face"), "story": d.get("story"), "voice": voice_of(a, d),
                                  "broken": bool(d.get("broken")), "candidates": ch["candidates"],
                                  "watch": ch["instrument"], "zoom_to": ch["target"], "story_rows": canon,
                                  "open_direct": True})
        jdump(run / "key.json", key)
        if shots:
            plan = {"qp": d.get("qp") or qp_bin(a), "web": d.get("web") or web_dir(a), "port": a.port,
                    "out": str(run / "shots"), "paint": a.paint, "sensitivity": "all",
                    "demo_args": charts[0]["market"], "shots": shots}
            pp = run / "plans" / f"chart-{d['name']}.json"
            jdump(pp, plan)
            node("shoot.mjs", pp)
        p = subprocess.run([d.get("qp") or qp_bin(a), "look", "show", d.get("look") or "r4"],
                           capture_output=True, text=True, cwd=ROOT)
        if p.returncode == 0:
            (run / "looks").mkdir(parents=True, exist_ok=True)
            (run / "looks" / f"{d['name']}.json").write_text(p.stdout)
    jdump(run / "key.json", key)


def cmd_shoot(a):
    run = Path(a.run)
    designs = jload(a.designs)
    if a.kinds == "chart":
        key = jload(run / "key.json") if (run / "key.json").exists() else {}
        return shoot_charts(a, run, designs, key)
    st = {s["state"]: s for s in jload(run / "states.json")["states"]}
    key = jload(run / "key.json") if (run / "key.json").exists() else {}
    shots = []
    kinds = a.kinds.split(",")
    for d in designs:
        for lang in a.langs.split(","):
            for theme in a.themes.split(","):
                base = {"lang": lang, "theme": theme, "look": d.get("look"), "face": d.get("face"), "voice": voice_of(a, d),
                        "broken": bool(d.get("broken")), "open": d.get("open")}
                todo = []
                if "face" in kinds:
                    todo += [(n, "face") for n in FACE_STATES if st.get(n, {}).get("found")]
                if "feed" in kinds:
                    todo.append(("feed", "feed"))
                if "evidence" in kinds:
                    todo += [("live_event", "evidence"), ("visitor", "evidence")]
                if "capsule" in kinds:  # (M20, Ek G7a) capsule depth
                    todo += [("live_event", "capsule"), ("visitor", "capsule")]
                if "motion" in kinds:
                    todo += [("live_event", "motion"), ("closed_failed", "motion")]
                for name, kind in todo:
                    s = st[name]
                    c = code(d["name"], name, kind, lang, theme)
                    shot = dict(base, id=c, kind=kind, step=s["step"], principal=s.get("principal", "arkin"),
                                candidates=s.get("candidates", []), watch=s.get("watch"), progress=[0.25, 0.5, 0.75, 1.0])
                    shot["_design"] = d["name"]
                    shots.append(shot)
                    key[c] = {"design": d["name"], "state": name, "kind": kind, "lang": lang, "theme": theme,
                              "principal": shot["principal"], "voice": shot["voice"]}
    jdump(run / "key.json", key)
    # The reader's audience (`?watch=`) is the authority's per-principal state and outlives
    # the page: shots without a watch list go first in each step group, then by watch list
    # (found in the dry run: a watch list hid the next design's target).
    shots.sort(key=lambda s: (s["step"], s.get("watch") or ""))
    # The look records (T3, `qp look show`): type roles and colour roles per design, for the
    # element measures (role of each text, SPEC §G7).
    (run / "looks").mkdir(parents=True, exist_ok=True)
    for d in designs:
        name = d.get("look") or "r4"
        p = subprocess.run([d.get("qp") or qp_bin(a), "look", "show", name], capture_output=True, text=True, cwd=ROOT)
        if p.returncode == 0:
            (run / "looks" / f"{d['name']}.json").write_text(p.stdout)
    # (M20) a design may come from its own build (`qp`, `web` in designs.json; the
    # alternatives are not merged): one plan per build; already shot codes are skipped.
    builds = {}
    for s in shots:
        dn = s.pop("_design")
        d = next(x for x in designs if x["name"] == dn)
        if (run / "shots" / f"{s['id']}.png").exists():
            continue
        builds.setdefault((d.get("qp") or qp_bin(a), d.get("web") or web_dir(a)), []).append(s)
    for i, ((qp, web), ss) in enumerate(sorted(builds.items())):
        plan = {"qp": qp, "web": web, "port": a.port, "out": str(run / "shots"),
                "paint": a.paint, "sensitivity": "all", "shots": ss}
        pp = run / "shoot-plan.json" if len(builds) == 1 else run / "plans" / f"shoot-{i}.json"
        jdump(pp, plan)
        node("shoot.mjs", pp)


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
        elif k["kind"] == "capsule":  # (M20, Ek G7a) capsule depth + its throttle copy
            out.append((f"{c}.capsule", png, "cap_evidence_visitor" if k["principal"] == "visitor" else "cap_evidence", rec))
            out.append((f"{c}.throttle", stim / f"{c}.throttle.png", "throttle", rec))
        elif k["kind"] == "chart":
            out.append((f"{c}.chart", stim / f"{c}.chart.png", "chart", rec))
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
        if k["kind"] == "capsule":
            IO.throttle(png, stim / f"{c}.throttle.png")
        if k["kind"] == "face":
            IO.throttle(png, stim / f"{c}.throttle.png")
            IO.half(png, stim / f"{c}.half.png")
            IO.grey(png, stim / f"{c}.gray.png")
        if k["kind"] == "chart":
            # (M18) lettered columns A-D over the visible bar range of the plot (c_live).
            r = jload(run / "shots" / f"{c}.json")
            lp = run / "looks" / f"{k['design']}.json"
            r["look_record"] = jload(lp) if lp.exists() else None
            plot = CP.chart_plot(r)
            if plot is None:
                raise SystemExit(f"derive: no price plot found in {c}")
            IO.columns(png, stim / f"{c}.chart.png", plot)
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
        if k["kind"] not in ("face", "chart", "capsule") or k["lang"] != a.pair_lang or k["theme"] != a.pair_theme:
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

def who_key(rec: dict, item: dict):
    """(Ek G7d) The e_who key: the open capsule's trust claim (`open().trust`, SPEC §G4) --
    server when it says so, device when it names the device (checked / checking) or the
    sources differ; without one (no data entitlement) the feed item's trust."""
    t = (rec.get("open") or {}).get("trust")
    if not t:
        return item.get("trust")
    t = json.dumps(t, ensure_ascii=False).lower()
    if "server" in t or "sunucu" in t:
        return "server"
    if any(w in t for w in ("cihaz", "device", "farklı", "differ")):
        return "device"
    return item.get("trust")


def key_record(rec: dict, qset: str) -> dict:
    eng = rec.get("engine") or {}
    item = eng.get("target_item") or {}
    out = {"item": item, "now_t": eng.get("replay_t"), "redacted": bool(item.get("redacted")),
           "visible": eng.get("visible_items") or []}
    if "evidence" in qset:
        fam = {"price": "price", "volume": "volume", "flow": "flow", "news": "news"}
        # (M20) a capsule-depth shot carries the capsule's evidence rows as `evidence_key`.
        rows = rec.get("evidence") or rec.get("evidence_key") or []
        out["families"] = sorted({fam[e["family"]] for e in rows if e.get("family") in fam})
        out["item"] = dict(item, trust=who_key(rec, item))
    if qset == "chart":
        # (M18) the story list (PROTOCOL §23, the engine's; the canonical list of the state
        # when this design's device did not hold one) and the visible bar range.
        st = rec.get("stories_canon") or rec.get("stories") or {}
        rows = st.get("stories") or []
        rng = ((rec.get("view") or {}).get("range") or {})
        out["stories"] = rows
        if "from_bar" in rng:
            a, b = int(rng["from_bar"]), int(rng["to_bar"])
            out["range"] = [a, b]
            last = max([int(r.get("last_bar", 0)) for r in rows] + [b])
            out["stories_visible"] = [r for r in rows if int(r["born_bar"]) <= b
                                      and int(r.get("closed_bar", last)) >= a]
        tgt = next((r for r in rows if r["id"] == rec.get("target")), None)
        if tgt:
            out["item"] = dict(item, kind=tgt["kind"])
        out["now_t"] = max([int(r.get("last_t", 0)) for r in rows] + [int(eng.get("replay_t") or 0)])
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
            for rep in range(a.reps or DEFAULT_REPS.get(qs, 3)):
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
                "motion_final": R.MOTION_NOTE, "chart": R.CHART_NOTE, "notice": R.NOTICE_NOTE,
                "notice_pair": R.NOTICE_PAIR_NOTE}.get(qs, {}).get(lang, "")
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
        lp = run / "looks" / f"{d}.json"
        look = jload(lp) if lp.exists() else None
        # (M18) chart-depth shots: element measures from the open scene's paint list.
        # (M20) capsule- and evidence-depth shots: the same open-scene measures.
        out[d] = (CP.chart_gates(recs, look) if all(r["shot"]["kind"] in ("chart", "capsule", "evidence") for r in recs)
                  else CP.gates(recs, look))
        if any(r["shot"]["kind"] in ("capsule", "evidence") for r in recs):
            out[d]["numbers_visitor"] = CP.visitor_digits(recs)  # (Ek G7e) hard
        k52 = run / "k52" / f"{d}.json"
        if k52.exists():
            kj = jload(k52)
            if kj.get("open", "absent") != "absent" and all("checks" in v for v in kj["configs"].values()):
                # (M20, k52open.mjs, Ek G7c) the same screen (capsule depth) for every design.
                out[d]["k52"] = {"pass": kj["pass"], "configs": {c: {k: v.get(k) for k in (
                    "tick_p95", "scene_p95", "scene_play_n", "scene_wheel_n", "wheel_long", "wheel_mode", "wheel_frame_p99",
                    "idle_long", "first_capsule_ms", "biased", "pass")} for c, v in kj["configs"].items()}}
            elif all("by_depth" in v for v in kj["configs"].values()):
                # (M18, k52chart.mjs) per depth; a JIT-off re-run (k52-rerun/, more reps, no
                # model calls in parallel) replaces the first JIT-off run when present.
                cfgs = dict(kj["configs"])
                rr = run / "k52-rerun" / f"{d}.json"
                if rr.exists():
                    cfgs.update({c: dict(v, rerun=True) for c, v in jload(rr)["configs"].items()})
                out[d]["k52"] = {"pass": all(v["pass"] for v in cfgs.values()),
                                 "configs": {c: {dep: {k: x[k] for k in ("tick_p95", "scene_p95", "drag_long", "pass")}
                                                 for dep, x in v["by_depth"].items()} for c, v in cfgs.items()},
                                 "rerun": rr.exists()}
            else:
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
    # (M18) the canonical story list of the chart state (the engine's, same for every design).
    sj = jload(run / "states.json") if (run / "states.json").exists() else {}
    canon = sj.get("chart_stories")
    for p in sorted((run / "readers").glob("*.json")):
        r = jload(p)
        c = r["stim"].split(".")[0]
        if c not in recs:
            recs[c] = jload(run / "shots" / f"{c}.json")
            if canon is not None:
                recs[c]["stories_canon"] = canon
        kr = key_record(recs[c], r["qset"])
        keys = K.keys_for(r["qset"], kr)
        for q, ans in r["answer"].items():
            acc = keys.get(q, set())
            if not acc and q != "t_first":
                continue  # no key (e.g. a family with no option): not scored
            rows.append({"design": key[c]["design"], "state": key[c]["state"], "lang": key[c]["lang"],
                         "theme": key[c]["theme"], "persona": r["persona"], "qset": r["qset"], "q": q,
                         "code": ans["code"], "credit": K.credit(q, ans["code"], acc), "cut": ans["cut"],
                         "key": "|".join(sorted(acc)),
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
    prim = tuple(a.primary.split(",")) if a.primary else SEL.PRIMARY_SETS
    readg = tuple(x for x in a.reading.split(",") if x) if a.reading is not None else SEL.READING_SETS
    res = SEL.select(rows, judg, computed, seed=SEED, broken=a.broken, reference=a.reference,
                     primary_sets=prim, reading_sets=readg)
    res["excluded_not_one_look"] = excluded
    jdump(run / "scores.json", {"rows": rows, "judgements": judg, "result": res})
    (run / "scores.md").write_text(SEL.scores_md(res))
    print((run / "scores.md").read_text())


def k52_charts(a, run):
    """(M18) K52 at 5 / 50 / 200 stories in view, per design build (k52chart.mjs)."""
    sj = jload(run / "states.json")
    ch = sj["charts"][0]
    rows = sj["chart_stories"]["stories"]
    # A capsule that exists at every replay step: stories born in the history (before the
    # first replayed bar), latest first.
    first_replayed = int(ch["last_bar"]) - 100
    hist = sorted((r for r in rows if int(r["born_bar"]) < first_replayed - 5),
                  key=lambda r: (-int(r["born_bar"]), r["id"]))
    for d in jload(a.designs):
        if d.get("broken") or d["name"] in (a.skip or "").split(","):
            continue
        out = run / "k52" / f"{d['name']}.json"
        if out.exists():
            continue
        plan = {"story": d.get("story"), "look": d.get("look"), "voice": voice_of(a, d), "market": ch["market"], "instrument": ch["instrument"],
                "open": [r["id"] for r in hist[:5]], "depths": [c["target"] for c in sj["charts"]], "rows": rows,
                "configs": a.configs.split(","), "reps": a.reps, "play_ms": 5000, "drag_ms": 6000, "back_steps": 400}
        pp = run / "plans" / f"k52-{d['name']}.json"
        jdump(pp, plan)
        out.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(["node", str(HERE / "k52chart.mjs"), d.get("qp") or qp_bin(a), d.get("web") or web_dir(a), str(out),
                        "--plan", str(pp)], check=True, cwd=ROOT)


def cmd_k52(a):
    run = Path(a.run)
    if a.charts:
        return k52_charts(a, run)
    for d in jload(a.designs):
        if d.get("broken"):
            continue  # the broken filter lives in the harness page, not in a design
        args = ["node", str(HERE / "k52.mjs"), qp_bin(a), str(run / "k52" / f"{d['name']}.json"), "--reps", str(a.reps),
                "--web", web_dir(a)]
        if d.get("look"):
            args += ["--look", d["look"]]
        if d.get("face"):
            args += ["--face", d["face"]]
        if voice_of(a, d):
            args += ["--voice", voice_of(a, d)]
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
        p.add_argument("--web", help="shell directory (default shells/web or $QP_WEB)")
        p.set_defaults(fn=fn)
        return p

    add("probe", cmd_probe).add_argument("--voice", help=VOICE_HELP)
    add("states", cmd_states).add_argument("--charts", action="store_true")
    add("chartstates", cmd_chartstates).add_argument("--voice", help=VOICE_HELP)
    p = add("shoot", cmd_shoot)
    p.add_argument("--designs", required=True)
    p.add_argument("--langs", default="tr,en")
    p.add_argument("--themes", default="light,dark")
    p.add_argument("--kinds", default="face,feed,evidence,motion")
    p.add_argument("--paint", default="canvas")
    p.add_argument("--voice", help=VOICE_HELP)
    p = add("derive", cmd_derive)
    p.add_argument("--pair-lang", default="tr")
    p.add_argument("--pair-theme", default="light")
    add("compute", cmd_compute)
    for name, fn in (("read", cmd_read), ("critic", cmd_critic)):
        p = add(name, fn)
        p.add_argument("--model", default="haiku" if name == "read" else "sonnet")
        # read: 0 = the per-set default (DEFAULT_REPS: primary 6, others 3; SPEC Ek G3).
        p.add_argument("--reps", type=int, default=0 if name == "read" else 3)
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
    p.add_argument("--primary", help="primary reader sets (default throttle; M18: chart)")
    p.add_argument("--reading", help="full-size reading sets (default face,face_visitor; M18: empty)")
    p = add("k52", cmd_k52)
    p.add_argument("--designs", required=True)
    p.add_argument("--reps", type=int, default=3)
    p.add_argument("--charts", action="store_true", help="(M18) 5/50/200 stories in view, per design build")
    p.add_argument("--configs", default="phone-jit-est,phone")
    p.add_argument("--skip", help="design names to skip")
    p.add_argument("--voice", help=VOICE_HELP)
    add("broken-check", cmd_broken).add_argument("--broken", required=True)
    a = ap.parse_args()
    a.fn(a)


if __name__ == "__main__":
    main()
