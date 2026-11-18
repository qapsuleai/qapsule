#!/usr/bin/env python3
"""R5 M21 re-measurement over the glance harness — SPEC §G, Ek G6 + **Ek G8**. One command per stage:

  m21v2.py probe  --run DIR                       consecutive-step feed probe (change states)
  m21v2.py shoot  --run DIR                       openings placed in bands (motion2) + state changes
  m21v2.py derive --run DIR                       motion2 / motion2_final pairs, notice frames, critic triples
  m21v2.py read   --run DIR [glance read args]    blind readers (glance.cmd_read, Ek G8 stimuli)
  m21v2.py critic --run DIR [glance critic args]  blind critic, CRITIC_M21
  m21v2.py unlimit --run DIR                      remove usage-limited (unanswered) records, count them
  m21v2.py loss   --run DIR                       reduced-motion information loss (Ek G6-5 / G8-6)
  m21v2.py k52    --run DIR                       K52 (k52.mjs) + K52 during motion (k52motion.mjs)
  m21v2.py score  --run DIR                       validity, continuity, notice, BT, rule (Ek G8-7)

Every design comes from ONE build of `main` (QP_BIN, QP_WEB) with its address parameters
(Ek G8-0). The design file `designs.json` is written into DIR by `shoot`; keys never leave DIR.
"""

from __future__ import annotations

import argparse
import json
import os
import random
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import glance as G  # noqa: E402
import m21 as M  # noqa: E402
from gl import imageops as IO  # noqa: E402
from gl import readers as R  # noqa: E402
from gl import select as SEL  # noqa: E402
from gl import stats as S  # noqa: E402

jload, jdump, code = G.jload, G.jdump, G.code

BROKEN = "tirvan"  # hidden name of the broken motion (Ek G8-2); only in the key files
LOOK = "oltan"
VOICE = "marla"
VIEW_H = 874
BANDS = 4
CHANCE_FROM = 0.25  # Ek G8-1: the key is always one of 4 balanced bands
SEED_NOTICE = 20261118

# Ek G8-0. mid_ms: half of the design's change animation under `oltan` (duration_scale 1.25).
DESIGNS = [
    {"name": "surek", "params": {"motion": "surek", "reduced_motion": "0"}, "mid_ms": 225},
    {"name": "nabiz", "params": {"motion": "nabiz", "reduced_motion": "0"}, "mid_ms": 562},
    {"name": "r4", "params": {"motion": "r4", "reduced_motion": "0"}, "mid_ms": 225},
    {"name": "still", "params": {"motion": "r4", "reduced_motion": "1"}, "mid_ms": 225, "freeze": False,
     "reduced_of": "r4", "reduced_by_definition": True, "no_primary": True},
    {"name": BROKEN, "params": {"motion": "surek", "reduced_motion": "0"}, "mid_ms": 225, "broken": True,
     "broken_motion": True},
    {"name": "surek-az", "params": {"motion": "surek", "reduced_motion": "1"}, "mid_ms": 225, "freeze": False,
     "info": True, "reduced_of": "surek"},
    {"name": "nabiz-az", "params": {"motion": "nabiz", "reduced_motion": "1"}, "mid_ms": 562, "freeze": False,
     "info": True, "reduced_of": "nabiz"},
]

# Ek G8-1: two targets (Ek G6's live_event and closed_failed, face step), each placed with its
# vertical centre at the centre of band k = 1..4.
OPEN_TARGETS = {
    "live": {"step": 1, "candidates": ["cap.SYN06.breakout.up.118"]},
    # Ek G8-9: the first closed_failed candidate with >= 3 faces above it in the feed (band 4
    # needs them; `cap.SYN26.breakout.down.47` is the 2nd row).
    "fail": {"step": 1, "candidates": ["cap.SYN26.breakout.down.51"]},
}
# Ek G8-9: bands of the FEED AREA (below the top bar), four equal bands.
OPEN_STATES = [{"state": f"o_{t}_{k}", "target": t, "band": k}
               for t in OPEN_TARGETS for k in range(1, BANDS + 1)]

# Ek G8-3: 4 changes + 4 controls, chosen from this build's probe (`probe/m21probe.json`) by the
# written rule; filled in before any reading (the table is in SPEC Ek G8).
CHANGE_STATES: list[dict] = []
_cs = HERE / "m21v2_states.json"
if _cs.exists():
    CHANGE_STATES = json.loads(_cs.read_text())["states"]

CRITIC_OPEN = ("o_live_2", "o_fail_3")
REPS = {"motion2": 6, "motion2_final": 6, "notice": 6}


def build():
    qp = os.environ.get("QP_BIN") or str(G.ROOT / "target/release/qp")
    web = os.environ.get("QP_WEB") or str(G.ROOT / "shells/web")
    return qp, web


def designs_in(run: Path):
    return {d["name"]: d for d in jload(run / "designs.json")}


def band_of(y: float, y0: float = 0.0) -> int:
    return min(BANDS, max(1, int((y - y0) // ((VIEW_H - y0) / BANDS)) + 1))


def feed_bands(src, dst, y0: float) -> None:
    """Ek G8-9: numbered bands 1-4 over the feed area [y0, H] (below the top bar): thin grey
    lines at the band borders (y0 included), a small number at the left of each band."""
    from PIL import Image, ImageDraw
    im = Image.open(src).convert("RGB")
    w, h = im.size
    d = ImageDraw.Draw(im)
    f = IO._font(16)
    bh = (h - y0) / BANDS
    for i in range(0, BANDS):
        y = round(y0 + bh * i)
        d.line([(0, y), (w, y)], fill=(128, 128, 128), width=1)
    for i in range(BANDS):
        y = round(y0 + bh * i) + 4
        d.rectangle([2, y - 2, 18, y + 18], fill=(128, 128, 128))
        d.text((5, y), str(i + 1), fill=(255, 255, 255), font=f)
    IO.save(im, dst)


# --- probe ---------------------------------------------------------------------------------

def cmd_probe(a):
    run = Path(a.run)
    qp, web = build()
    plan = {"qp": qp, "web": web, "port": a.port, "out": str(run / "probe"), "paint": "canvas", "sensitivity": "all",
            "voice": VOICE, "params": {"look": LOOK, "motion": "r4", "reduced_motion": "1"},
            "m21probe": {"from": a.frm, "to": a.to}}
    jdump(run / "plans" / "m21probe.json", plan)
    subprocess.run(["node", str(HERE / "shoot.mjs"), str(run / "plans" / "m21probe.json")], check=True, cwd=G.ROOT)


# --- shoot ---------------------------------------------------------------------------------

def cmd_shoot(a):
    run = Path(a.run)
    qp, web = build()
    ds = []
    for d in DESIGNS:
        d = json.loads(json.dumps(d))
        d["params"] = {"look": LOOK, **d["params"]}
        d["qp"], d["web"] = qp, web
        ds.append(d)
    jdump(run / "designs.json", ds)
    key = jload(run / "key.json") if (run / "key.json").exists() else {}
    pr = jload(run / "probe" / "m21probe.json")["steps"] if CHANGE_STATES else {}
    only = set(a.only.split(",")) if a.only else None
    for d in ds:
        if only and d["name"] not in only:
            continue
        shots = []
        for lang in a.langs.split(","):
            for theme in a.themes.split(","):
                base = {"lang": lang, "theme": theme, "voice": VOICE, "params": d["params"], "look": None, "face": None,
                        "broken_motion": bool(d.get("broken_motion"))}
                if not a.changes_only:
                    for s in OPEN_STATES:
                        t = OPEN_TARGETS[s["target"]]
                        c = code(d["name"], s["state"], "motion2", lang, theme)
                        shots.append(dict(base, id=c, kind="motion", step=t["step"], principal="arkin", candidates=t["candidates"],
                                          freeze=d.get("freeze", True), progress=[0.25, 0.5], band_k=s["band"]))
                        key[c] = {"design": d["name"], "state": s["state"], "kind": "motion2", "lang": lang, "theme": theme}
                if not a.opens_only:
                    for cs in CHANGE_STATES:
                        c = code(d["name"], cs["state"], "change", lang, theme)
                        anchor = pr[str(cs["step"])]["rows"][cs["anchor_index"]]["id"]
                        shots.append(dict(base, id=c, kind="change", step=cs["step"], principal="arkin", target=cs["target"],
                                          anchor=anchor, change_kind=cs["kind"], mid_ms=d.get("mid_ms", 225),
                                          end_ms=int(os.environ.get("QP_M21_END_MS", "5000"))))
                        key[c] = {"design": d["name"], "state": cs["state"], "kind": "change", "lang": lang, "theme": theme}
        jdump(run / "key.json", key)
        if a.max:
            shots = shots[: a.max]
        shots.sort(key=lambda s: (s["step"], s.get("watch") or ""))
        plan = {"qp": qp, "web": web, "port": a.port, "out": str(run / "shots"), "paint": "canvas",
                "sensitivity": "all", "voice": VOICE, "shots": shots}
        pp = run / "plans" / f"shoot-{d['name']}.json"
        jdump(pp, plan)
        print(f"shoot {d['name']}: {len(shots)} shots", flush=True)
        subprocess.run(["node", str(HERE / "shoot.mjs"), str(pp)], check=True, cwd=G.ROOT)


# --- derive --------------------------------------------------------------------------------

def stimuli(run: Path):
    key = jload(run / "key.json")
    ds = designs_in(run)
    out = []
    stim, shots = run / "stim", run / "shots"
    for c, k in sorted(key.items()):
        d = ds.get(k["design"], {})
        if d.get("info"):
            continue
        if k["kind"] == "motion2" and (shots / f"{c}.json").exists():
            out.append((f"{c}.m2", stim / f"{c}.m2.png", "motion2", shots / f"{c}.json"))
            if not d.get("no_primary"):  # still: its in-between frames ARE the final frame (Ek G8-1 ii)
                out.append((f"{c}.m2f", stim / f"{c}.m2f.png", "motion2_final", shots / f"{c}.json"))
        if k["kind"] == "change" and (shots / f"{c}.mid.json").exists():
            out.append((f"{c}.mid", stim / f"{c}.mid.n.png", "notice", shots / f"{c}.mid.json"))
            out.append((f"{c}.after", stim / f"{c}.after.n.png", "notice", shots / f"{c}.after.json"))
    return out


def cmd_derive(a):
    run = Path(a.run)
    stim, shots = run / "stim", run / "shots"
    stim.mkdir(parents=True, exist_ok=True)
    key = jload(run / "key.json")
    for c, k in sorted(key.items()):
        if k["kind"] == "motion2" and (shots / f"{c}.p25.png").exists():
            y0 = jload(shots / f"{c}.json").get("top_bar_bottom") or 0.0
            feed_bands(shots / f"{c}.p25.png", stim / f"{c}.p25.n.png", y0)
            feed_bands(shots / f"{c}.p50.png", stim / f"{c}.p50.n.png", y0)
            feed_bands(shots / f"{c}.png", stim / f"{c}.final.n.png", y0)
            IO.pair(stim / f"{c}.p25.n.png", stim / f"{c}.p50.n.png", stim / f"{c}.m2.png", labels=("I", "II"))
            IO.pair(stim / f"{c}.final.n.png", stim / f"{c}.final.n.png", stim / f"{c}.m2f.png", labels=("I", "II"))
        if k["kind"] == "change" and (shots / f"{c}.mid.png").exists():
            M.bands(shots / f"{c}.mid.png", stim / f"{c}.mid.n.png")
            M.bands(shots / f"{c}.after.png", stim / f"{c}.after.n.png")
    # Critic triples (Ek G8-4): start (r4's frame before) | A | B, tr/light, selection designs + broken.
    ds = [d for d, v in designs_in(run).items() if not v.get("info")]
    by = defaultdict(dict)
    want = set(CRITIC_OPEN) | {cs["state"] for cs in CHANGE_STATES if cs["kind"] != "none"}
    for c, k in key.items():
        if k["design"] in ds and k["lang"] == "tr" and k["theme"] == "light" and k["state"] in want:
            by[k["state"]][k["design"]] = (c, k["kind"])
    ck = {}
    for state, m in sorted(by.items()):
        if "r4" not in m:
            continue
        rc, kind = m["r4"]
        start = shots / f"{rc}.before.png"
        frame = (lambda c: shots / f"{c}.p25.png") if kind == "motion2" else (lambda c: shots / f"{c}.mid.png")
        names = sorted(n for n in m if frame(m[n][0]).exists())
        for i in range(len(names)):
            for j in range(i + 1, len(names)):
                x, y = names[i], names[j]
                for order, (l, r) in enumerate(((x, y), (y, x))):
                    pid = code("m21v2pair", state, x, y, order)
                    M.triple(start, frame(m[l][0]), frame(m[r][0]), stim / f"{pid}.pair.png")
                    ck[pid] = {"state": state, "A": l, "B": r, "pair": [x, y], "order": order}
    jdump(run / "critic_key.json", ck)
    print(f"derive: {len(stimuli(run))} reader stimuli, {len(ck)} critic triples")


# --- read / critic -------------------------------------------------------------------------

def cmd_read(a):
    G.stimuli = stimuli
    G.DEFAULT_REPS.update(REPS)
    G.cmd_read(a)


def cmd_critic(a):
    R.CRITIC = R.CRITIC_M21
    G.cmd_critic(a)


LIMIT_RE = re.compile(r"(usage limit|rate limit|limit reached|hit your limit|5-hour limit|session limit|out of extra usage)", re.I)


def limited(log: dict) -> bool:
    """An unanswered call (usage limit): no answer text or a limit message, and no cost."""
    res = log.get("result") or ""
    return (not res.strip() or bool(LIMIT_RE.search(res)) or (log.get("exit") not in (0, None) and not res.strip())) \
        and not (log.get("cost_usd") or 0)


def cmd_unlimit(a):
    run = Path(a.run)
    n = defaultdict(int)
    for sub in ("readers", "critics"):
        for p in sorted((run / sub).glob("*.json")):
            r = jload(p)
            if limited(r["log"]):
                n[sub + "/" + r.get("qset", "critic")] += 1
                if not a.dry:
                    p.unlink()
    tot = jload(run / "limited.json") if (run / "limited.json").exists() else {}
    if not a.dry:
        for k, v in n.items():
            tot[k] = tot.get(k, 0) + v
        jdump(run / "limited.json", tot)
    print("limited (this pass):", dict(n), "| total:", tot)


# --- keys and rows -------------------------------------------------------------------------

def from_key(rec: dict):
    """Ek G8-1: band of the target face's vertical centre before the transition; the middle
    half must stay in one band (else both bands met are accepted and the record says so)."""
    r = rec.get("target_rect_before")
    if not r:
        return set(), None
    y0 = rec.get("top_bar_bottom") or 0.0
    bh = (VIEW_H - y0) / BANDS
    top, h = r[1], r[3]
    lo, hi = top + h / 4, top + 3 * h / 4
    met = {str(i + 1) for i in range(BANDS) if hi > y0 + bh * i and lo < y0 + bh * (i + 1)}
    centre = str(band_of(top + h / 2, y0))
    return ({centre} if len(met) <= 1 else met), {"centre": centre, "met": sorted(met), "rect": r}


def change_state(name):
    return next(c for c in CHANGE_STATES if c["state"] == name)


def rows_of(run: Path):
    key = jload(run / "key.json")
    recs, rows = {}, []
    stim = {sid: rec for sid, _img, _qs, rec in stimuli(run)}
    for p in sorted((run / "readers").glob("*.json")):
        r = jload(p)
        if r["stim"] not in stim:
            continue
        c = r["stim"].split(".")[0]
        k = key[c]
        rp = stim[r["stim"]]
        if rp not in recs:
            recs[rp] = jload(rp)
        rec = recs[rp]
        if r["qset"] in ("motion2", "motion2_final"):
            keys = {"m_from": from_key(rec)[0]}
        else:
            keys = M.change_keys(rec, change_state(k["state"]))
        for q, ans in r["answer"].items():
            acc = keys.get(q, set())
            if not acc:
                continue
            cls = "|".join(sorted(acc))
            if q == "n_where":
                cls = "none" if acc == {"none"} else "change"
            rows.append({"design": k["design"], "state": k["state"], "lang": k["lang"], "theme": k["theme"],
                         "persona": r["persona"], "qset": r["qset"], "q": q, "code": ans["code"],
                         "credit": 1.0 if ans["code"] in acc else 0.0, "cut": ans["cut"], "key": cls,
                         "stim": r["stim"], "rep": r["rep"], "model": ",".join(r["log"]["models"]),
                         "one_look": r["log"]["one_look"], "cost": r["log"]["cost_usd"]})
    return rows


# --- loss, K52 (Ek G6-5 / G6-4 code, Ek G8 designs) ----------------------------------------

def cmd_loss(a):
    # m21.cmd_loss keys on (design, state, kind): the change shots (kind "change") and the
    # opening end frames (kind "motion2" here; m21 compares `<code>.json` for kind "motion").
    M.CHANGE_STATES = CHANGE_STATES
    run = Path(a.run)
    key = jload(run / "key.json")
    tmp = {c: dict(k, kind="motion" if k["kind"] == "motion2" else k["kind"]) for c, k in key.items()}
    real = run / "key.json"
    bak = run / "key.v2.json"
    real.rename(bak)
    try:
        jdump(real, tmp)
        M.cmd_loss(a)
    finally:
        real.unlink()
        bak.rename(real)


def cmd_k52(a):
    run = Path(a.run)
    ds = jload(run / "designs.json")
    p = run / "designs-k52.json"
    jdump(p, [d for d in ds if not d.get("broken") and not d.get("info")])
    a.designs = str(p)
    M.cmd_k52(a)


# --- score (Ek G8-7) -----------------------------------------------------------------------

def validity(rows, d, ds):
    """Ek G8-1: one-sided; invalid when the Wilson lower bound of the final-frame accuracy is
    above 0.25 (still: its motion2 reading, which is the final frame)."""
    qset = "motion2" if ds[d].get("no_primary") else "motion2_final"
    v = [r["credit"] for r in rows if r["design"] == d and r["qset"] == qset and r["q"] == "m_from"]
    if not v:
        return {"n": 0, "valid": False, "note": "no reading"}
    p, lo, hi = S.wilson(sum(v), len(v))
    return {"qset": qset, "n": len(v), "p": round(p, 3), "lo": round(lo, 3), "hi": round(hi, 3), "chance": CHANCE_FROM,
            "valid": lo <= CHANCE_FROM, "two_sided_within": lo <= CHANCE_FROM <= hi}


def notice_score(rows, d, seed, reps=2000, states=None):
    """Ek G8-3: hit - false alarm per persona (state-balanced means), F2:H1; bootstrap within
    persona x state strata. Returns (est, lo, hi, parts)."""
    sel = [r for r in rows if r["design"] == d and r["qset"] == "notice" and r["q"] == "n_where"
           and (states is None or r["state"] in states)]
    if not sel:
        return None
    strata = defaultdict(list)  # (persona, state) -> [(is_change, value)]
    for r in sel:
        ch = r["key"] == "change"
        val = r["credit"] if ch else (1.0 if r["code"] in ("1", "2", "3", "4") else 0.0)
        strata[(r["persona"], r["state"])].append((ch, val))

    def est(st):
        out = {}
        for persona in ("F", "H"):
            hs = [sum(v for _, v in xs) / len(xs) for (p, _s), xs in st.items() if p == persona and xs and xs[0][0]]
            fs = [sum(v for _, v in xs) / len(xs) for (p, _s), xs in st.items() if p == persona and xs and not xs[0][0]]
            if hs and fs:
                out[persona] = (sum(hs) / len(hs), sum(fs) / len(fs))
        if not out:
            return None, out
        w = {"F": SEL.F_WEIGHT, "H": SEL.H_WEIGHT}
        num = sum(w[p] * (h - f) for p, (h, f) in out.items())
        return num / sum(w[p] for p in out), out
    e, parts = est(strata)
    if e is None:
        return None
    rng = random.Random(seed)
    bs = []
    for _ in range(reps):
        st = {k: [xs[rng.randrange(len(xs))] for _ in xs] for k, xs in strata.items()}
        b, _ = est(st)
        bs.append(b)
    bs.sort()
    hit = {p: round(h, 3) for p, (h, f) in parts.items()}
    fa = {p: round(f, 3) for p, (h, f) in parts.items()}
    return (e, bs[int(0.025 * reps)], bs[min(reps - 1, int(0.975 * reps))], {"hit": hit, "fa": fa})


def share(rows, d, state_pred, fn):
    v = [r for r in rows if r["design"] == d and r["qset"] == "notice" and state_pred(r["state"])]
    return round(sum(1 for r in v if fn(r)) / len(v), 3) if v else None


def cmd_score(a):
    run = Path(a.run)
    rows = rows_of(run)
    judg = M.judgements_of(run)
    excluded = sorted({r["stim"] + "/" + r["persona"] + "/" + str(r["rep"]) for r in rows if not r["one_look"]})
    rows = [r for r in rows if r["one_look"]]
    judg_excl = sum(1 for j in judg if not j["one_look"])
    judg = [j for j in judg if j["one_look"]]
    ds = designs_in(run)
    sel_designs = [d for d, v in ds.items() if not v.get("info")]
    seed = G.SEED
    cls = {c["state"]: c.get("cls") for c in CHANGE_STATES}
    res = {"per_design": {}, "excluded_not_one_look": excluded, "critic_excluded_not_one_look": judg_excl, "steps": []}
    for i, d in enumerate(sorted(sel_designs)):
        val = validity(rows, d, ds)
        has_primary = val.get("valid") and not ds[d].get("no_primary")
        res["per_design"][d] = {
            "validity": val,
            "primary": SEL.weighted_accuracy([r for r in rows if r["qset"] == "motion2"], d, ("motion2",), seed + i) if has_primary else None,
            "motion2_all": SEL.weighted_accuracy([r for r in rows if r["qset"] == "motion2"], d, ("motion2",), seed + 40 + i),
            "motion2_final": SEL.weighted_accuracy([r for r in rows if r["qset"] == "motion2_final"], d, ("motion2_final",), seed + 60 + i),
            "from_none_share": round(sum(1 for r in rows if r["design"] == d and r["qset"] == "motion2" and r["code"] == "none")
                                     / max(1, sum(1 for r in rows if r["design"] == d and r["qset"] == "motion2")), 3),
            "from_by_band": {b: S.wilson(*_kn([r for r in rows if r["design"] == d and r["qset"] == "motion2" and r["state"].endswith("_" + b)]))[0]
                             for b in "1234"},
            "notice": notice_score(rows, d, SEED_NOTICE + i),
            "notice_mid": notice_score([r for r in rows if r["stim"].endswith(".mid")], d, SEED_NOTICE + 10 + i),
            "notice_after": notice_score([r for r in rows if r["stim"].endswith(".after")], d, SEED_NOTICE + 20 + i),
            "hit_live": share(rows, d, lambda s: cls.get(s) == "change_live", lambda r: r["credit"] == 1.0),
            "hit_closed": share(rows, d, lambda s: cls.get(s) == "change_closed", lambda r: r["credit"] == 1.0),
            "fa_live": share(rows, d, lambda s: cls.get(s) == "control_live", lambda r: r["code"] in ("1", "2", "3", "4")),
            "fa_nolive": share(rows, d, lambda s: cls.get(s) == "control_nolive", lambda r: r["code"] in ("1", "2", "3", "4")),
            "per_state": {s: share(rows, d, lambda x, s=s: x == s, (lambda r: r["credit"] == 1.0)) for s in cls},
            "cut_answers": sum(1 for r in rows if r["design"] == d and r["cut"]),
        }
    pd = res["per_design"]
    still_ok = pd.get("still", {}).get("validity", {}).get("valid", False)
    res["question_valid_globally"] = still_ok
    pj = [j for j in judg if j["a"] in sel_designs and j["b"] in sel_designs]
    outcomes, conflicts = S.pair_outcomes(pj)
    bt = S.bootstrap_bt(outcomes, seed=seed + 999) if outcomes else {}
    res["bt"] = {d: [round(x, 3) for x in v] for d, v in bt.items()}
    res["conflicts"] = {f"{x}|{y}": {"split": s, "total": t, "rate": round(s / t, 3) if t else None,
                                      "not_told_apart": bool(t) and s / t > SEL.CONFLICT_MAX}
                        for (x, y), (s, t) in conflicts.items()}
    # broken check (Ek G8-2): strictly last in the primary (among designs with one) and in BT
    allp = {d: pd[d]["primary"][0] for d in sel_designs if pd[d]["primary"]}
    others_b = [bt[d][0] for d in bt if d != BROKEN]
    bc = {"primary": allp.get(BROKEN), "primary_strict_last": BROKEN in allp and all(allp[BROKEN] < v for d, v in allp.items() if d != BROKEN),
          "bt": bt.get(BROKEN, [None])[0], "bt_strict_last": BROKEN in bt and all(bt[BROKEN][0] < v for v in others_b),
          "notice": pd[BROKEN]["notice"][0] if pd[BROKEN]["notice"] else None,
          "notice_strict_last": bool(pd[BROKEN]["notice"]) and all(pd[BROKEN]["notice"][0] < pd[d]["notice"][0] for d in sel_designs if d != BROKEN and pd[d]["notice"])}
    bc["pass"] = bc["primary_strict_last"] and bc["bt_strict_last"]
    res["broken_check"] = bc
    # hard gates
    loss = jload(run / "loss.json") if (run / "loss.json").exists() else {}
    gates = {}
    cands = [d for d in sel_designs if d != BROKEN]
    for d in cands:
        g = {}
        k1, k2 = run / "k52" / f"{d}.json", run / "k52" / f"{d}.motion.json"
        if k1.exists():
            g["k52_jit"] = jload(k1)["configs"].get("phone-jit-est", {}).get("pass")
        if k2.exists():
            km = jload(k2)["configs"]
            g["k52_motion_jit"] = km.get("phone-jit-est", {}).get("pass")
            g["k52_motion_nojit"] = km.get("phone", {}).get("pass")
        if d in loss:
            g["reduced_motion_loss"] = loss[d]["pass"] if a.loss_rule == "text" else loss[d]["count_any_prim"] == 0
        elif ds[d].get("reduced_by_definition"):
            g["reduced_motion_loss"] = True
        gates[d] = g
    names = sorted({k for g in gates.values() for k in g})
    waived = [n for n in names if not any(g.get(n) for g in gates.values() if n in g)]
    elig = [d for d in cands if all(v or k in waived for k, v in gates[d].items() if v is not None)]
    res["gates"], res["waived"] = gates, waived
    res["steps"].append({"step": 1, "rule": "hard gates (K52; reduced-motion loss, text rule)", "eligible": elig, "waived": waived,
                         "eliminated": {d: [k for k, v in gates[d].items() if v is False and k not in waived] for d in cands if d not in elig}})
    prim = {d: pd[d]["primary"] for d in elig if pd[d]["primary"]}
    order = sorted(prim, key=lambda d: -prim[d][0])
    group = [d for d in order if S.overlaps(prim[d], prim[order[0]])] if order else []
    res["steps"].append({"step": 2, "rule": "continuity m_from (valid designs only, F2:H1)", "order": order,
                         "no_primary": [d for d in elig if d not in prim], "top_group": group})
    g2 = group
    if len(group) > 1:
        nt = {d: pd[d]["notice"] for d in group if pd[d]["notice"]}
        no = sorted(nt, key=lambda d: -nt[d][0])
        g2 = [d for d in no if S.overlaps(nt[d], nt[no[0]])]
        res["steps"].append({"step": "2b", "rule": "notice hit - false alarm (F2:H1)", "order": no, "top_group": g2})
    g3 = g2
    if len(g2) > 1 and bt:
        gb = sorted([d for d in g2 if d in bt], key=lambda d: -bt[d][0])
        g3 = [d for d in gb if S.overlaps(bt[d], bt[gb[0]])] + [d for d in g2 if d not in bt]
        res["steps"].append({"step": 3, "rule": "critic Bradley-Terry", "order": gb, "top_group": g3})
    usable = bc["pass"] and still_ok
    told = len(g3) == 1 and usable

    def chars(d):
        v = []
        for c, k in jload(run / "key.json").items():
            if k["design"] == d and k["kind"] == "change" and (run / "shots" / f"{c}.end.json").exists():
                r = jload(run / "shots" / f"{c}.end.json")
                v.append(sum(len(t["t"]) for t in r["dom"]["texts"]) + sum(
                    len(x) for f in r["dom"]["faces"] if f["full"] for x in M.face_texts(r, f["id"])))
        return sum(v) / len(v) if v else 0

    def tb(d):
        return (0 if d in prim else 1, -(prim.get(d) or (0,))[0], -((pd[d]["notice"] or (0,))[0]),
                -(bt.get(d) or (0,))[0], chars(d), d)
    ranked = sorted(elig, key=tb)
    res["chars"] = {d: round(chars(d), 1) for d in cands}
    res["verdict"] = {"usable": usable, "winner": g3[0] if told else None, "told_apart": told,
                      "not_told_apart": [] if told else g3, "default": ranked[0] if (ranked and usable) else None,
                      "top_two": ranked[:2] if usable else [], "ranking": ranked}
    res["model_agreement"] = SEL.model_agreement(rows)
    res["cost"] = {"readers": round(sum(r["cost"] or 0 for r in M._uniq(rows)), 4),
                   "reader_calls": len({(r["stim"], r["persona"], r["rep"], r["model"]) for r in rows}),
                   "critics": round(sum(j["cost"] or 0 for j in judg), 4), "critic_calls": len(judg)}
    res["loss_rule"] = a.loss_rule
    jdump(run / a.out, {"rows": rows, "judgements": judg, "result": res})
    print(json.dumps({k: res[k] for k in ("steps", "verdict", "bt", "gates", "waived", "broken_check", "question_valid_globally")}, indent=1))


def _kn(rows):
    return sum(r["credit"] for r in rows), max(1, len(rows))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    def add(name, fn):
        p = sub.add_parser(name)
        p.add_argument("--run", required=True)
        p.add_argument("--port", type=int, default=int(os.environ.get("QP_DEMO_PORT", "8737")))
        p.set_defaults(fn=fn)
        return p
    p = add("probe", cmd_probe)
    p.add_argument("--frm", type=int, default=1)
    p.add_argument("--to", type=int, default=500)
    p = add("shoot", cmd_shoot)
    p.add_argument("--langs", default="tr,en")
    p.add_argument("--themes", default="light,dark")
    p.add_argument("--only")
    p.add_argument("--max", type=int, default=0)
    p.add_argument("--opens-only", action="store_true")
    p.add_argument("--changes-only", action="store_true")
    add("derive", cmd_derive)
    for name, fn in (("read", cmd_read), ("critic", cmd_critic)):
        p = add(name, fn)
        p.add_argument("--model", default="haiku" if name == "read" else "sonnet")
        p.add_argument("--reps", type=int, default=0 if name == "read" else 3)
        p.add_argument("--max", type=int, default=0)
        p.add_argument("--jobs", type=int, default=6)
        p.add_argument("--max-turns", type=int, default=3)
        p.add_argument("--work")
        p.add_argument("--designs")
        p.add_argument("--states")
        p.add_argument("--qp")
        p.add_argument("--web")
        if name == "read":
            p.add_argument("--personas", default="F,H")
            p.add_argument("--sets")
            p.add_argument("--lang")
            p.add_argument("--theme")
        else:
            p.add_argument("--pair-lang", default="tr")
    p = add("unlimit", cmd_unlimit)
    p.add_argument("--dry", action="store_true")
    p = add("loss", cmd_loss)
    p.add_argument("--reference", default="r4")
    p = add("k52", cmd_k52)
    p.add_argument("--reps", type=int, default=3)
    p.add_argument("--runs", type=int, default=3)
    p.add_argument("--voice", default=VOICE)
    p.add_argument("--only")
    p.add_argument("--skip-std", action="store_true")
    p = add("score", cmd_score)
    p.add_argument("--loss-rule", default="text", choices=("any_prim", "text"))
    p.add_argument("--out", default="scores.json")
    a = ap.parse_args()
    a.fn(a)


if __name__ == "__main__":
    main()
