#!/usr/bin/env python3
"""R5 M21 (motion) over the glance harness — SPEC §G, Ek G6. One command per stage:

  m21.py shoot  --run DIR --designs F.json     opening (motion) + state change (change) shots
  m21.py derive --run DIR                      motion pairs, notice frames, critic triples
  m21.py read   --run DIR [glance read args]   blind readers (glance.cmd_read, M21 stimuli)
  m21.py critic --run DIR [glance critic args] blind critic with the M21 prompt
  m21.py loss   --run DIR                      reduced-motion information loss (Ek G6.5)
  m21.py k52    --run DIR --designs F.json     K52 (k52.mjs) + K52 during motion (k52motion.mjs)
  m21.py score  --run DIR                      validity, continuity, notice, BT, rule (Ek G6.7)

designs.json: [{"name", "qp", "web", "params": {...}, "freeze": bool, "reduce": bool,
"mid_ms": n, "broken": bool, "info": bool, "reduced_of": "<name>"}]. Keys never leave DIR.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import glance as G  # noqa: E402
from gl import compute as CP  # noqa: E402
from gl import imageops as IO  # noqa: E402
from gl import keys as K  # noqa: E402
from gl import readers as R  # noqa: E402
from gl import select as SEL  # noqa: E402
from gl import stats as S  # noqa: E402
from PIL import Image, ImageDraw  # noqa: E402

jload, jdump, code = G.jload, G.jdump, G.code

# Ek G6.2: the fixed state changes (consecutive-step probe of the reader's whole feed,
# `m21probe.json`, main build, look r4, voice marla, sensitivity all). step = N (the page
# opens at N, the harness steps to N + 1); target = the capsule that changes; anchor_index =
# the row of the feed at N scrolled to the top before the step (the target first or second
# on screen); kind: born / state / none (controls).
CHANGE_STATES = [
    {"state": "c_born", "step": 216, "target": "cap.SYN09.breakout.up.143", "anchor_index": 18, "kind": "born"},
    {"state": "c_strengthened", "step": 225, "target": "cap.SYN09.breakout.up.143", "anchor_index": 19, "kind": "state"},
    {"state": "c_failed", "step": 333, "target": "cap.SYN20.breakout.down.154", "anchor_index": 10, "kind": "state"},
    {"state": "c_realized", "step": 207, "target": "cap.SYN11.breakout.up.140", "anchor_index": 19, "kind": "state"},
    {"state": "n_still1", "step": 211, "target": None, "anchor_index": 18, "kind": "none"},
    {"state": "n_still2", "step": 300, "target": None, "anchor_index": 10, "kind": "none"},
]
MOTION_STATES = ("live_event", "closed_failed")
REPS = {"motion": 6, "motion_final": 6, "notice": 6, "notice_pair": 3}
BANDS = 4
VIEW_H = 874


def anchors(run: Path):
    pr = jload(run / "probe" / "m21probe.json")["steps"]
    out = {}
    for c in CHANGE_STATES:
        rows = pr[str(c["step"])]["rows"]
        out[c["state"]] = rows[c["anchor_index"]]["id"]
    return out


# --- shoot -------------------------------------------------------------------------------

def cmd_shoot(a):
    run = Path(a.run)
    designs = jload(a.designs)
    st = {s["state"]: s for s in jload(run / "states.json")["states"]}
    key = jload(run / "key.json") if (run / "key.json").exists() else {}
    anc = anchors(run)
    only = set(a.only.split(",")) if a.only else None
    for d in designs:
        if only and d["name"] not in only:
            continue
        shots = []
        for lang in a.langs.split(","):
            for theme in a.themes.split(","):
                base = {"lang": lang, "theme": theme, "voice": a.voice, "params": d["params"], "reduce": bool(d.get("reduce")),
                        "look": None, "face": None}
                for name in MOTION_STATES:
                    s = st[name]
                    c = code(d["name"], name, "motion", lang, theme)
                    shots.append(dict(base, id=c, kind="motion", step=s["step"], principal=s.get("principal", "arkin"),
                                      candidates=s.get("candidates", []), watch=s.get("watch"), freeze=d.get("freeze", True),
                                      progress=[0.25, 0.5, 0.75, 1.0]))
                    key[c] = {"design": d["name"], "state": name, "kind": "motion", "lang": lang, "theme": theme, "principal": "arkin"}
                for cs in CHANGE_STATES:
                    c = code(d["name"], cs["state"], "change", lang, theme)
                    shots.append(dict(base, id=c, kind="change", step=cs["step"], principal="arkin", target=cs["target"],
                                      anchor=anc[cs["state"]], change_kind=cs["kind"], mid_ms=d.get("mid_ms", 180)))
                    key[c] = {"design": d["name"], "state": cs["state"], "kind": "change", "lang": lang, "theme": theme, "principal": "arkin"}
        jdump(run / "key.json", key)
        if a.max:
            shots = shots[: a.max]
        shots.sort(key=lambda s: (s["step"], s.get("watch") or ""))
        plan = {"qp": d["qp"], "web": d["web"], "port": a.port, "out": str(run / "shots"), "paint": "canvas",
                "sensitivity": "all", "voice": a.voice, "shots": shots}
        pp = run / "plans" / f"shoot-{d['name']}.json"
        jdump(pp, plan)
        print(f"shoot {d['name']}: {len(shots)} shots", flush=True)
        subprocess.run(["node", str(HERE / "shoot.mjs"), str(pp)], check=True, cwd=G.ROOT)
        p = subprocess.run([d["qp"], "look", "show", d["params"].get("look", "r4")], capture_output=True, text=True)
        if p.returncode == 0:
            (run / "looks").mkdir(parents=True, exist_ok=True)
            (run / "looks" / f"{d['name']}.json").write_text(p.stdout)


# --- scrolling to the anchor (shoot.mjs reads s.anchor) ------------------------------------

# --- derive ------------------------------------------------------------------------------

def bands(src, dst) -> None:
    """Numbered horizontal bands 1-4 (top to bottom) over a feed frame (M21 `n_where`): thin
    grey lines at the three inner borders, a small number at the left of each band."""
    im = Image.open(src).convert("RGB")
    w, h = im.size
    d = ImageDraw.Draw(im)
    f = IO._font(16)
    for i in range(1, BANDS):
        y = round(h * i / BANDS)
        d.line([(0, y), (w, y)], fill=(128, 128, 128), width=1)
    for i in range(BANDS):
        y = round(h * i / BANDS) + 4
        d.rectangle([2, y - 2, 18, y + 18], fill=(128, 128, 128))
        d.text((5, y), str(i + 1), fill=(255, 255, 255), font=f)
    IO.save(im, dst)


def triple(a, b, c, dst, labels=("0", "A", "B")) -> None:
    ims = [Image.open(x).convert("RGB") for x in (a, b, c)]
    w = sum(i.size[0] for i in ims) + IO.GAP * 2
    h = max(i.size[1] for i in ims) + IO.LABEL_H
    out = Image.new("RGB", (w, h), (40, 40, 40))
    x = 0
    d = ImageDraw.Draw(out)
    f = IO._font(20)
    for im, t in zip(ims, labels):
        out.paste(im, (x, IO.LABEL_H))
        d.text((x + im.size[0] // 2 - 6, 3), t, fill=(255, 255, 255), font=f)
        x += im.size[0] + IO.GAP
    IO.save(out, dst)


def stimuli(run: Path):
    key = jload(run / "key.json")
    out = []
    for c, k in sorted(key.items()):
        stim = run / "stim"
        shots = run / "shots"
        if k["kind"] == "motion" and (shots / f"{c}.json").exists():
            out.append((f"{c}.motion", stim / f"{c}.motion.png", "motion", shots / f"{c}.json"))
            out.append((f"{c}.motion_final", stim / f"{c}.motion_final.png", "motion_final", shots / f"{c}.json"))
        if k["kind"] == "change" and (shots / f"{c}.mid.json").exists():
            out.append((f"{c}.mid", stim / f"{c}.mid.n.png", "notice", shots / f"{c}.mid.json"))
            out.append((f"{c}.after", stim / f"{c}.after.n.png", "notice", shots / f"{c}.after.json"))
            out.append((f"{c}.pair", stim / f"{c}.pair.n.png", "notice_pair", shots / f"{c}.mid.json"))
    return out


def designs_in(run):
    return {d["name"]: d for d in jload(run / "designs.json")}


def cmd_derive(a):
    run = Path(a.run)
    stim = run / "stim"
    stim.mkdir(parents=True, exist_ok=True)
    key = jload(run / "key.json")
    shots = run / "shots"
    for c, k in sorted(key.items()):
        if k["kind"] == "motion" and (shots / f"{c}.p25.png").exists():
            IO.regions(shots / f"{c}.p25.png", stim / f"{c}.mid.r.png")
            IO.pair(shots / f"{c}.before.png", stim / f"{c}.mid.r.png", stim / f"{c}.motion.png", labels=("1", "2"))
            IO.regions(shots / f"{c}.png", stim / f"{c}.final.r.png")
            IO.pair(shots / f"{c}.before.png", stim / f"{c}.final.r.png", stim / f"{c}.motion_final.png", labels=("1", "2"))
        if k["kind"] == "change" and (shots / f"{c}.mid.png").exists():
            bands(shots / f"{c}.mid.png", stim / f"{c}.mid.n.png")
            bands(shots / f"{c}.after.png", stim / f"{c}.after.n.png")
            IO.pair(shots / f"{c}.before.png", stim / f"{c}.mid.n.png", stim / f"{c}.pair.n.png", labels=("1", "2"))
    # Critic triples (Ek G6.6): start (the reference r4's before frame) | A | B, tr/light,
    # every pair of selection designs (info designs out), both orders.
    ds = [d for d, v in designs_in(run).items() if not v.get("info")]
    by = defaultdict(dict)
    for c, k in key.items():
        if k["design"] in ds and k["lang"] == a.pair_lang and k["theme"] == a.pair_theme:
            by[k["state"]][k["design"]] = (c, k["kind"])
    ck = {}
    for state, m in sorted(by.items()):
        if a.reference not in m:
            continue
        rc, kind = m[a.reference]
        start = shots / f"{rc}.before.png"
        frame = (lambda c: shots / f"{c}.p25.png") if kind == "motion" else (lambda c: shots / f"{c}.mid.png")
        names = sorted(n for n in m if frame(m[n][0]).exists())
        for i in range(len(names)):
            for j in range(i + 1, len(names)):
                x, y = names[i], names[j]
                for order, (l, r) in enumerate(((x, y), (y, x))):
                    pid = code("m21pair", state, x, y, order)
                    triple(start, frame(m[l][0]), frame(m[r][0]), stim / f"{pid}.pair.png")
                    ck[pid] = {"state": state, "A": l, "B": r, "pair": [x, y], "order": order}
    jdump(run / "critic_key.json", ck)
    print(f"derive: {len(stimuli(run))} reader stimuli, {len(ck)} critic triples")


# --- read / critic: glance's, with the M21 stimuli and prompt ----------------------------

def cmd_read(a):
    G.stimuli = stimuli
    G.DEFAULT_REPS.update(REPS)
    G.cmd_read(a)


def cmd_critic(a):
    R.CRITIC = R.CRITIC_M21
    G.cmd_critic(a)


# --- keys --------------------------------------------------------------------------------

def band_keys(rect) -> set[str]:
    """Bands met by the middle half (vertical) of a face box (Ek G6.3)."""
    top, h = rect[1], rect[3]
    lo, hi = top + h / 4, top + 3 * h / 4
    out = set()
    for i in range(BANDS):
        a, b = VIEW_H * i / BANDS, VIEW_H * (i + 1) / BANDS
        if hi > a and lo < b:
            out.add(str(i + 1))
    return out


def change_keys(rec: dict, cs: dict) -> dict:
    if cs["kind"] == "none":
        return {"n_where": {"none"}, "n_where2": {"none"}, "n_what": {"none"}}
    face = next((f for f in rec["dom"]["faces"] if f["id"] == cs["target"]), None)
    if not face or face["rect"][1] + face["rect"][3] <= 0 or face["rect"][1] >= VIEW_H:
        where = set()  # target off screen: not scored
    else:
        where = band_keys(face["rect"])
    return {"n_where": where, "n_where2": where, "n_what": {cs["kind"]}}


def rows_of(run: Path):
    key = jload(run / "key.json")
    css = {c["state"]: c for c in CHANGE_STATES}
    recs = {}
    rows = []
    stim = {sid: (img, qs, rec) for sid, img, qs, rec in stimuli(run)}
    for p in sorted((run / "readers").glob("*.json")):
        r = jload(p)
        c = r["stim"].split(".")[0]
        k = key[c]
        rp = stim[r["stim"]][2]
        if rp not in recs:
            recs[rp] = jload(rp)
        rec = recs[rp]
        if k["kind"] == "motion":
            keys = K.keys_for(r["qset"], G.key_record(rec, r["qset"]))
        else:
            keys = change_keys(rec, css[k["state"]])
        for q, ans in r["answer"].items():
            acc = keys.get(q, set())
            if not acc:
                continue
            cls = acc_cls = "|".join(sorted(acc))
            if q in ("n_where", "n_where2"):
                acc_cls = "none" if acc == {"none"} else "change"
            rows.append({"design": k["design"], "state": k["state"], "lang": k["lang"], "theme": k["theme"],
                         "persona": r["persona"], "qset": r["qset"], "q": q, "code": ans["code"],
                         "credit": K.credit(q, ans["code"], acc), "cut": ans["cut"], "key": acc_cls, "accepted": cls,
                         "stim": r["stim"], "rep": r["rep"], "model": ",".join(r["log"]["models"]),
                         "one_look": r["log"]["one_look"], "cost": r["log"]["cost_usd"]})
    return rows


def judgements_of(run: Path):
    ck = jload(run / "critic_key.json") if (run / "critic_key.json").exists() else {}
    out = []
    for p in sorted((run / "critics").glob("*.json")):
        r = jload(p)
        if r["pair"] not in ck:
            continue
        k = ck[r["pair"]]
        v = r["verdict"]["verdict"]
        out.append({"a": k["pair"][0], "b": k["pair"][1], "rep": f"{k['state']}#{r['rep']}#{r['log']['models']}",
                    "order": k["order"], "winner": k[v] if v in ("A", "B") else None, "state": k["state"],
                    "cost": r["log"]["cost_usd"], "one_look": r["log"]["one_look"], "cut": r["verdict"]["cut"]})
    return out


# --- reduced-motion loss (Ek G6.5) -----------------------------------------------------------

def content(rec: dict) -> set:
    """Static content of a screen: face texts and glyph presence from the core's painted face
    scenes (`paints`, what the page drew), the open scene's texts and the DOM text runs."""
    s = set()
    for fid, sc in (rec.get("paints") or {}).items():
        if not isinstance(sc, dict) or "error" in sc:
            continue
        try:
            t, g = CP.face_elements(sc)
        except Exception:  # noqa: BLE001 - the open scene has another shape
            t, g = [], None
        s |= {("text", fid, x["text"]) for x in t}
        if g:
            s.add(("glyph", fid))
    s |= {("dom", t["t"]) for t in rec["dom"]["texts"]}
    return s


def face_texts(rec: dict, fid: str) -> set:
    sc = (rec.get("paints") or {}).get(fid)
    if not isinstance(sc, dict) or "error" in sc:
        return set()
    t, _ = CP.face_elements(sc)
    return {x["text"] for x in t}


def animated_faces(meta: dict) -> set:
    """Faces with a motion layer at the half-way frame: overlay boxes on the face, copied
    pieces over it, or its canvas faded (Ek G6.5 b)."""
    ov = meta["frames"]["mid"]["overlay"]
    out = {m["face"] for m in ov["mk"] if m["face"] and m["opacity"] > 0}
    out |= {f for f, o in ov["face_opacity"].items() if o < 0.999}
    if ov["fx"]:
        out |= set(ov.get("changed") or [])
    return out


def cmd_loss(a):
    run = Path(a.run)
    key = jload(run / "key.json")
    ds = designs_in(run)
    idx = {(k["design"], k["state"], k["kind"], k["lang"], k["theme"]): c for c, k in key.items()}
    css = {c["state"]: c for c in CHANGE_STATES}
    out = {}
    for d, v in ds.items():
        red = next((n for n, w in ds.items() if w.get("reduced_of") == d), None)
        if not red or v.get("broken"):
            continue
        static, carried, detail = [], [], []
        for (dd, state, kind, lang, theme), c in sorted(idx.items()):
            if dd != d:
                continue
            cr = idx.get((red, state, kind, lang, theme))
            if not cr:
                continue
            if kind == "motion":
                ra, rr = run / "shots" / f"{c}.json", run / "shots" / f"{cr}.json"
            else:
                ra, rr = run / "shots" / f"{c}.end.json", run / "shots" / f"{cr}.end.json"
            if not ra.exists() or not rr.exists():
                continue
            A, B = jload(ra), jload(rr)
            lost = sorted(map(list, content(A) - content(B)))
            static += [[state, lang, theme] + x for x in lost]
            if kind == "change":
                meta = jload(run / "shots" / f"{c}.json")
                ref = idx.get((a.reference, state, kind, lang, theme))
                refrec = jload(run / "shots" / f"{ref}.end.json") if ref else None
                for fid in sorted(animated_faces(meta)):
                    marker = face_texts(B, fid) - (face_texts(refrec, fid) if refrec else set())
                    detail.append({"state": state, "lang": lang, "theme": theme, "face": fid,
                                   "target": fid == css[state]["target"], "reduced_marker": sorted(marker)})
                    if not marker:
                        carried.append([state, lang, theme, fid])
        out[d] = {"reduced": red, "static_lost": static, "carried_lost": carried, "animated": detail,
                  "count": len(static) + len(carried), "pass": not static and not carried}
    jdump(run / "loss.json", out)
    for d, v in out.items():
        print(d, "vs", v["reduced"], "static", len(v["static_lost"]), "carried", len(v["carried_lost"]),
              "animated faces", len(v["animated"]), "pass", v["pass"])


# --- K52 ---------------------------------------------------------------------------------

def cmd_k52(a):
    run = Path(a.run)
    for d in jload(a.designs):
        if d.get("broken") or d.get("info") or (a.only and d["name"] not in a.only.split(",")):
            continue
        params = [f"{k}={v}" for k, v in d["params"].items() if k != "look"]
        (run / "k52").mkdir(parents=True, exist_ok=True)
        o1 = run / "k52" / f"{d['name']}.json"
        if not o1.exists() and not a.skip_std:
            args = ["node", str(HERE / "k52.mjs"), d["qp"], str(o1), "--reps", str(a.reps), "--web", d["web"],
                    "--look", d["params"].get("look", "r4"), "--voice", a.voice]
            for p in params:
                args += ["--param", p]
            subprocess.run(args, check=True, cwd=G.ROOT, env=dict(os.environ, QP_SCRATCH=str(run)))
        o2 = run / "k52" / f"{d['name']}.motion.json"
        if not o2.exists():
            args = ["node", str(HERE / "k52motion.mjs"), d["qp"], d["web"], str(o2), "--runs", str(a.runs)]
            for p in [f"{k}={v}" for k, v in d["params"].items()] + [f"voice={a.voice}"]:
                args += ["--param", p]
            subprocess.run(args, check=True, cwd=G.ROOT)


# --- score (Ek G6.7) ---------------------------------------------------------------------

def validity(rows, design):
    """Per question: the final-frame accuracy (motion_final) and the verdict. A question is
    valid for a design when the Wilson lower bound of its final-frame accuracy is not above
    chance (one-sided, Ek G6.1); the two-sided literal reading is reported too."""
    out = {}
    for q in ("m_same", "m_where"):
        v = [r["credit"] for r in rows if r["design"] == design and r["qset"] == "motion_final" and r["q"] == q]
        if not v:
            out[q] = {"n": 0, "valid": False, "note": "not scored (no key)"}
            continue
        p, lo, hi = S.wilson(sum(v), len(v))
        ch = K.chance(q)
        out[q] = {"n": len(v), "p": round(p, 3), "lo": round(lo, 3), "hi": round(hi, 3), "chance": ch,
                  "valid": lo <= ch, "two_sided_within": lo <= ch <= hi}
    return out


def primary_of(rows, design, valid_qs, seed):
    sel = [r for r in rows if r["design"] == design and r["qset"] == "motion" and r["q"] in valid_qs]
    return SEL.weighted_accuracy(sel, design, ("motion",), seed) if sel else None


def cmd_score(a):
    run = Path(a.run)
    SEL.BALANCED = ("t_valid", "n_where", "n_where2")
    rows = rows_of(run)
    judg = judgements_of(run)
    excluded = sorted({r["stim"] + "/" + r["persona"] for r in rows if not r["one_look"]})
    rows = [r for r in rows if r["one_look"]]
    judg = [j for j in judg if j["one_look"]]
    ds = designs_in(run)
    sel_designs = [d for d, v in ds.items() if not v.get("info")]
    seed = G.SEED
    res = {"per_design": {}, "excluded_not_one_look": excluded, "steps": []}
    for i, d in enumerate(sorted(ds)):
        val = validity(rows, d)
        vq = [q for q, v in val.items() if v.get("valid")]
        res["per_design"][d] = {
            "validity": val, "valid_questions": vq,
            "primary": primary_of(rows, d, vq, seed + i),
            "motion_all": SEL.weighted_accuracy(rows, d, ("motion",), seed + 50 + i),
            "motion_final": SEL.weighted_accuracy(rows, d, ("motion_final",), seed + 60 + i),
            "notice": SEL.weighted_accuracy(rows, d, ("notice",), seed + 70 + i),
            "notice_mid": SEL.weighted_accuracy([r for r in rows if r["stim"].endswith(".mid")], d, ("notice",), seed + 80 + i),
            "notice_after": SEL.weighted_accuracy([r for r in rows if r["stim"].endswith(".after")], d, ("notice",), seed + 90 + i),
            "notice_pair": SEL.weighted_accuracy(rows, d, ("notice_pair",), seed + 100 + i),
            "moving_share_mid": _share(rows, d, "motion", "m_same", "moving"),
            "moving_share_final": _share(rows, d, "motion_final", "m_same", "moving"),
            "none_share_change": _share([r for r in rows if r["key"] == "change"], d, "notice", "n_where", "none"),
            "false_alarm_control": 1 - (_share([r for r in rows if r["key"] == "none"], d, "notice", "n_where", "none") or 0),
            "questions": SEL.per_question(rows, d),
            "cut_answers": sum(1 for r in rows if r["design"] == d and r["cut"]),
        }
    pj = [j for j in judg if j["a"] in sel_designs and j["b"] in sel_designs]
    outcomes, conflicts = S.pair_outcomes(pj)
    bt = S.bootstrap_bt(outcomes, seed=seed + 999) if outcomes else {}
    res["bt"] = {d: [round(x, 3) for x in v] for d, v in bt.items()}
    res["conflicts"] = {f"{x}|{y}": {"split": s, "total": t, "rate": round(s / t, 3) if t else None,
                                      "not_told_apart": bool(t) and s / t > SEL.CONFLICT_MAX}
                        for (x, y), (s, t) in conflicts.items()}
    # hard gates: K52 (both configs as Ek G6.4), reduced-motion loss (Ek G6.5)
    loss = jload(run / "loss.json") if (run / "loss.json").exists() else {}
    gates = {}
    for d in sel_designs:
        if ds[d].get("broken"):
            continue
        g = {}
        k1, k2 = run / "k52" / f"{d}.json", run / "k52" / f"{d}.motion.json"
        if k1.exists():
            kj = jload(k1)
            g["k52_jit"] = kj["configs"].get("phone-jit-est", {}).get("pass")
        if k2.exists():
            km = jload(k2)["configs"]
            g["k52_motion_jit"] = km.get("phone-jit-est", {}).get("pass")
            g["k52_motion_nojit"] = km.get("phone", {}).get("pass")
        if d in loss:
            g["reduced_motion_loss"] = loss[d]["pass"]
        elif ds[d].get("reduced_by_definition"):
            g["reduced_motion_loss"] = True
        gates[d] = g
    names = sorted({k for g in gates.values() for k in g})
    waived = [n for n in names if not any(g.get(n) for g in gates.values() if n in g)]
    elig = [d for d in gates if all(v or k in waived for k, v in gates[d].items() if v is not None)]
    res["gates"] = gates
    res["waived"] = waived
    res["steps"].append({"step": 1, "rule": "hard gates (K52 JIT on, K52 during motion JIT on / off, reduced-motion loss)",
                         "eligible": elig, "waived": waived,
                         "eliminated": {d: [k for k, v in gates[d].items() if v is False and k not in waived] for d in gates if d not in elig}})
    pd = res["per_design"]
    prim = {d: pd[d]["primary"] for d in elig if pd[d]["primary"]}
    order = sorted(prim, key=lambda d: -prim[d][0])
    group = [d for d in order if S.overlaps(prim[d], prim[order[0]])] if order else []
    res["steps"].append({"step": 2, "rule": "continuity (motion, valid questions only, F2:H1)", "order": order,
                         "no_primary": [d for d in elig if d not in prim], "top_group": group})
    g2 = group
    if len(group) > 1:
        nt = {d: pd[d]["notice"] for d in group if pd[d]["notice"]}
        no = sorted(nt, key=lambda d: -nt[d][0])
        g2 = [d for d in no if S.overlaps(nt[d], nt[no[0]])]
        res["steps"].append({"step": "2b", "rule": "notice (n_where, mid + after, class-balanced, F2:H1)", "order": no, "top_group": g2})
    g3 = g2
    if len(g2) > 1 and bt:
        gb = sorted([d for d in g2 if d in bt], key=lambda d: -bt[d][0])
        g3 = [d for d in gb if S.overlaps(bt[d], bt[gb[0]])] + [d for d in g2 if d not in bt]
        res["steps"].append({"step": 3, "rule": "critic Bradley-Terry", "order": gb, "top_group": g3})
    told = len(g3) == 1

    def chars(d):
        v = []
        for c, k in jload(run / "key.json").items():
            if k["design"] == d and k["kind"] == "change" and (run / "shots" / f"{c}.end.json").exists():
                r = jload(run / "shots" / f"{c}.end.json")
                v.append(sum(len(t["t"]) for t in r["dom"]["texts"]) + sum(
                    len(x) for f in r["dom"]["faces"] if f["full"] for x in face_texts(r, f["id"])))
        return sum(v) / len(v) if v else 0

    def tb(d):
        return (0 if d in prim else 1, -(prim.get(d) or (0,))[0], -((pd[d]["notice"] or (0,))[0]),
                -(bt.get(d) or (0,))[0], chars(d), d)
    ranked = sorted(elig, key=tb)
    res["chars"] = {d: round(chars(d), 1) for d in sel_designs}
    res["verdict"] = {"winner": g3[0] if told else None, "told_apart": told, "not_told_apart": [] if told else g3,
                      "default": ranked[0] if ranked else None, "top_two": ranked[:2], "ranking": ranked}
    # broken check (§G8, Ek G3): strictly last in the primary (among designs with one) and in BT
    br = a.broken
    if br:
        allp = {d: pd[d]["primary"][0] for d in sel_designs if pd[d]["primary"]}
        others_p = [v for d, v in allp.items() if d != br]
        others_b = [bt[d][0] for d in bt if d != br]
        res["broken_check"] = {
            "primary": allp.get(br), "primary_strict_last": br in allp and all(allp[br] < v for v in others_p),
            "bt": bt.get(br, [None])[0], "bt_strict_last": br in bt and all(bt[br][0] < v for v in others_b),
            "notice": pd[br]["notice"][0] if pd[br]["notice"] else None,
            "notice_strict_last": bool(pd[br]["notice"]) and all(pd[br]["notice"][0] < pd[d]["notice"][0] for d in sel_designs if d != br and pd[d]["notice"]),
            "motion_all_strict_last": bool(pd[br]["motion_all"]) and all(pd[br]["motion_all"][0] < pd[d]["motion_all"][0] for d in sel_designs if d != br and pd[d]["motion_all"]),
        }
        res["broken_check"]["pass"] = res["broken_check"]["primary_strict_last"] and res["broken_check"]["bt_strict_last"]
        res["paired_vs_broken"] = {d: SEL.paired_difference(rows, d, br, ("motion",), seed + 31) for d in sel_designs if d != br}
    res["model_agreement"] = SEL.model_agreement(rows)
    res["cost"] = {"readers": round(sum(r["cost"] or 0 for r in _uniq(rows)), 4), "reader_calls": len({r["stim"] + r["persona"] + str(r["rep"]) + r["model"] for r in rows}),
                   "critics": round(sum(j["cost"] or 0 for j in judg), 4), "critic_calls": len(judg)}
    jdump(run / "scores.json", {"rows": rows, "judgements": judg, "result": res})
    print(json.dumps({k: res[k] for k in ("steps", "verdict", "bt", "gates", "waived")}, indent=1))
    if br:
        print(json.dumps(res["broken_check"], indent=1))


def _uniq(rows):
    seen, out = set(), []
    for r in rows:
        k = (r["stim"], r["persona"], r["rep"], r["model"])
        if k not in seen:
            seen.add(k)
            out.append(r)
    return out


def _share(rows, d, qset, q, codev):
    v = [r for r in rows if r["design"] == d and r["qset"] == qset and r["q"] == q]
    return round(sum(1 for r in v if r["code"] == codev) / len(v), 3) if v else None


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    def add(name, fn):
        p = sub.add_parser(name)
        p.add_argument("--run", required=True)
        p.add_argument("--port", type=int, default=int(os.environ.get("QP_DEMO_PORT", "8737")))
        p.set_defaults(fn=fn)
        return p
    p = add("shoot", cmd_shoot)
    p.add_argument("--designs", required=True)
    p.add_argument("--langs", default="tr,en")
    p.add_argument("--themes", default="light,dark")
    p.add_argument("--voice", default="marla")
    p.add_argument("--only")
    p.add_argument("--max", type=int, default=0)
    p = add("derive", cmd_derive)
    p.add_argument("--pair-lang", default="tr")
    p.add_argument("--pair-theme", default="light")
    p.add_argument("--reference", default="r4")
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
    p = add("loss", cmd_loss)
    p.add_argument("--reference", default="r4")
    p = add("k52", cmd_k52)
    p.add_argument("--designs", required=True)
    p.add_argument("--reps", type=int, default=3)
    p.add_argument("--runs", type=int, default=3)
    p.add_argument("--voice", default="marla")
    p.add_argument("--only")
    p.add_argument("--skip-std", action="store_true")
    p = add("score", cmd_score)
    p.add_argument("--broken", default="kestim")
    a = ap.parse_args()
    a.fn(a)


if __name__ == "__main__":
    main()
