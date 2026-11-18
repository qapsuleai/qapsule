"""The written-in-advance selection rule (SPEC §G9) and the harness self-test (§G8).
Pure and deterministic (seeded bootstrap); unit-tested in tests/test_select.py.
"""

from __future__ import annotations

import random
from collections import defaultdict

from . import stats as S

F_WEIGHT, H_WEIGHT = 2.0, 1.0      # SPEC §G9.2: the glance is for F; H keeps the expert honest
# SPEC §G9 after Ek G1 (dry run, before any alternative): the primary glance measure is
# the throttle copy (text illegible to the reader model, calibration/sigma.json); full-size
# reading is a non-inferiority gate (the model reads faded, inverted text as well as good
# text, so reading accuracy could not rank the broken design last).
PRIMARY_SETS = ("throttle",)
READING_SETS = ("face", "face_visitor")
HALF_SETS = ("face_half", "face_visitor_half")
CONFLICT_MAX = 0.30                 # RESEARCH U1: a pair with more order conflicts is "not told apart"
GATES = ("text_contrast", "nontext_contrast", "min_font", "touch", "colour", "k52", "numbers_visitor")  # M20: Ek G7e


# SPEC Ek G3: questions whose answer classes are unbalanced in the fixed state set are
# class-balanced: each key class weighs the same in total, so a reader who always gives one
# answer scores 1 / (number of classes) on them (BROKEN.md: `t_valid` response bias — "ended"
# always scored the closed states' share).
BALANCED = ("t_valid",)


def balance_weights(rows):
    """{(q, key class): weight} = 1 / (classes × share of the class), shares over all rows of
    the question (the state set is the same for every design)."""
    n = defaultdict(lambda: defaultdict(int))
    for r in rows:
        if r.get("q") in BALANCED and r.get("key"):
            n[r.get("q")][r["key"]] += 1
    w = {}
    for q, by in n.items():
        tot = sum(by.values())
        for k, c in by.items():
            w[(q, k)] = tot / (len(by) * c)
    return w


def _weight(r, w):
    return w.get((r.get("q"), r.get("key")), 1.0) if r.get("q") in BALANCED else 1.0


def unit_means(rows, keyfn, w=None):
    """Weighted mean credit per unit (`keyfn(row)`); weights from `balance_weights`."""
    w = balance_weights(rows) if w is None else w
    num, den = defaultdict(float), defaultdict(float)
    for r in rows:
        k = keyfn(r)
        if k is None:
            continue
        x = _weight(r, w)
        num[k] += x * r["credit"]
        den[k] += x
    return {k: num[k] / den[k] for k in num if den[k] > 0}


def _units(rows, design, sets):
    """Mean credit per (stimulus, persona, rep) unit, by persona (class-balanced, Ek G3)."""
    sel = [r for r in rows if r["qset"] in sets]
    w = balance_weights(sel)
    m = unit_means(sel, lambda r: (r["stim"], r["persona"], r["rep"], r.get("model", "")) if r["design"] == design else None, w)
    by = defaultdict(list)
    for (stim, persona, _rep, _m), v in sorted(m.items()):
        by[persona].append(v)
    return by


def weighted_accuracy(rows, design, sets, seed, reps=2000):
    """(estimate, lo, hi) of the F/H-weighted mean unit accuracy; the bootstrap resamples
    the F and H units separately (seeded)."""
    by = _units(rows, design, sets)
    f, h = by.get("F", []), by.get("H", [])
    if not f and not h:
        return None

    def est(fv, hv):
        parts = [(F_WEIGHT, fv), (H_WEIGHT, hv)]
        num = sum(w * sum(v) / len(v) for w, v in parts if v)
        den = sum(w for w, v in parts if v)
        return num / den

    rng = random.Random(seed)
    bs = []
    for _ in range(reps):
        fs = [f[rng.randrange(len(f))] for _ in f]
        hs = [h[rng.randrange(len(h))] for _ in h]
        bs.append(est(fs, hs))
    bs.sort()
    return (est(f, h), bs[int(0.025 * reps)], bs[min(reps - 1, int(0.975 * reps))], len(f), len(h))


def per_question(rows, design):
    out = {}
    g = defaultdict(list)
    for r in rows:
        if r["design"] == design:
            g[(r["qset"], r["q"], r["persona"])].append(r["credit"])
    for (qs, q, p), v in sorted(g.items()):
        est, lo, hi = S.wilson(sum(v), len(v))
        out[f"{qs}/{q}/{p}"] = {"p": round(est, 3), "lo": round(lo, 3), "hi": round(hi, 3), "n": len(v)}
    return out


def gate_status(computed, designs):
    """Hard gates per design; a gate that NO design passes (reference included) is waived
    and reported (SPEC §G9.1, as R4 E6)."""
    waived = []
    for g in GATES:
        vals = [computed.get(d, {}).get(g, {}).get("pass") for d in designs if g in computed.get(d, {})]
        if vals and not any(vals):
            waived.append(g)
    status = {}
    for d in designs:
        c = computed.get(d)
        if not c:
            status[d] = {"computed": False, "failed": [], "pass": True}
            continue
        failed = [g for g in GATES if g in c and not c[g].get("pass") and g not in waived]
        status[d] = {"computed": True, "failed": failed, "pass": not failed}
    return status, waived


def model_agreement(rows):
    """Share of identical answer codes between two reader models on the same (stimulus,
    persona, question), first replicate of each (SPEC §G4: haiku vs sonnet)."""
    by = defaultdict(dict)
    for r in rows:
        if r["rep"] == 0:
            by[(r["stim"], r["persona"], r["q"])][r.get("model", "")] = r.get("code")
    pairs = [v for v in by.values() if len(v) >= 2]
    if not pairs:
        return None
    same = sum(1 for v in pairs if len(set(v.values())) == 1)
    return {"n": len(pairs), "agree": round(same / len(pairs), 3), "models": sorted({m for v in pairs for m in v})}


def select(rows, judgements, computed, seed: int, broken: str | None = None, reference: str | None = None,
           primary_sets=PRIMARY_SETS, reading_sets=READING_SETS):
    # (M18, SPEC §G9 "M18 görevleri") the primary set is per topic: `chart` for M18; the
    # face reading sets do not exist there (the story chart is only at chart depth).
    res_sets = {"primary": list(primary_sets), "reading": list(reading_sets)}
    designs = sorted({r["design"] for r in rows} | {j["a"] for j in judgements} | {j["b"] for j in judgements} | set(computed))
    res = {"designs": designs, "broken": broken, "per_design": {}, "steps": [], "sets": res_sets}
    for i, d in enumerate(designs):
        res["per_design"][d] = {
            "primary": weighted_accuracy(rows, d, primary_sets, seed + i),
            "reading": weighted_accuracy(rows, d, reading_sets, seed + 100 + i) if reading_sets else None,
            "half": weighted_accuracy(rows, d, HALF_SETS, seed + 700 + i),
            "gray": weighted_accuracy(rows, d, ("gray",), seed + 200 + i),
            "feed": weighted_accuracy(rows, d, ("feed",), seed + 300 + i),
            "evidence": weighted_accuracy(rows, d, ("evidence", "evidence_visitor"), seed + 400 + i),
            "motion": weighted_accuracy(rows, d, ("motion",), seed + 500 + i),
            "motion_final": weighted_accuracy(rows, d, ("motion_final",), seed + 600 + i),
            "questions": per_question(rows, d),
            "cut_answers": sum(1 for r in rows if r["design"] == d and r["cut"]),
            "not_one_look": sum(1 for r in rows if r["design"] == d and not r["one_look"]),
        }
    outcomes, conflicts = S.pair_outcomes(judgements)
    bt = S.bootstrap_bt(outcomes, seed=seed + 999) if outcomes else {}
    res["bt"] = {d: [round(x, 3) for x in v] for d, v in bt.items()}
    res["conflicts"] = {f"{x}|{y}": {"split": s, "total": t, "rate": round(s / t, 3) if t else None,
                                      "not_told_apart": bool(t) and s / t > CONFLICT_MAX}
                        for (x, y), (s, t) in conflicts.items()}
    # Length bias (RESEARCH U2): rank correlation of BT strength with on-screen text, and
    # the share of decisive outcomes won by the design with more text.
    chars = {d: (computed.get(d, {}).get("density") or {}).get("screen_chars_mean") for d in designs}
    common = [d for d in bt if chars.get(d) is not None]
    res["length_bias"] = {
        "spearman_bt_vs_chars": S.spearman([bt[d][0] for d in common], [chars[d] for d in common]) if len(common) >= 3 else None,
        "decisive_won_by_more_text": _more_text_share(outcomes, chars),
    }
    # Step 1: hard gates.
    cand = [d for d in designs if d != broken]
    gstat, waived = gate_status(computed, cand)
    res["gates"] = gstat
    res["waived_gates"] = waived
    elig = [d for d in cand if gstat[d]["pass"]]
    res["steps"].append({"step": 1, "rule": "hard gates", "eligible": elig, "waived": waived,
                         "eliminated": {d: gstat[d]["failed"] for d in cand if not gstat[d]["pass"]}})
    # Step 1b: reading non-inferiority (full-size comprehension): out if the design's upper
    # bound is below the reference's point estimate (it clearly loses what the text says).
    ref = res["per_design"].get(reference, {}).get("reading") if reference else None
    if ref:
        lost = [d for d in elig if d != reference and res["per_design"][d]["reading"]
                and res["per_design"][d]["reading"][2] < ref[0]]
        elig = [d for d in elig if d not in lost]
        res["steps"].append({"step": "1b", "rule": "reading non-inferiority vs reference", "reference": reference,
                             "eliminated": lost, "eligible": elig})
    res["model_agreement"] = model_agreement(rows)
    if broken:
        res["paired_vs_broken"] = {d: paired_difference(rows, d, broken, primary_sets, seed + 31)
                                   for d in designs if d != broken}
    if not elig:
        res["steps"].append({"step": 1, "note": "no design passes the gates; the rule stops"})
        res["verdict"] = {"winner": None, "told_apart": False, "default": None, "top_two": []}
        return res
    # Step 2: primary (glance accuracy, F 2 : H 1, bootstrap 95 %).
    prim = {d: res["per_design"][d]["primary"] for d in elig if res["per_design"][d]["primary"]}
    order = sorted(prim, key=lambda d: -prim[d][0])
    group = [d for d in order if S.overlaps(prim[d], prim[order[0]])] if order else elig
    res["steps"].append({"step": 2, "rule": f"primary: {','.join(primary_sets)} accuracy (F2:H1)", "order": order,
                         "top_group": group})
    # Step 3: secondary within the group (critic BT), then step 4 (throttle).
    group2 = group
    if len(group) > 1 and bt:
        g_bt = sorted([d for d in group if d in bt], key=lambda d: -bt[d][0])
        if g_bt:
            group2 = [d for d in g_bt if S.overlaps(bt[d], bt[g_bt[0]])] + [d for d in group if d not in bt]
        res["steps"].append({"step": 3, "rule": "secondary: critic Bradley-Terry", "order": g_bt, "top_group": group2})
    group3 = group2
    if len(group2) > 1:
        thr = {d: res["per_design"][d]["reading"] for d in group2 if res["per_design"][d]["reading"]}
        t_ord = sorted(thr, key=lambda d: -thr[d][0])
        if t_ord:
            group3 = [d for d in t_ord if S.overlaps(thr[d], thr[t_ord[0]])] + [d for d in group2 if d not in thr]
        res["steps"].append({"step": 4, "rule": "full-size reading accuracy", "order": t_ord, "top_group": group3})
    told = len(group3) == 1

    # Tie-break for the integration default only (SPEC §G9.5): point estimates in order.
    def tb(d):
        p = prim.get(d, (0,))[0]
        b = bt.get(d, (0,))[0]
        t = (res["per_design"][d]["reading"] or (0,))[0]
        return (-p, -b, -t, chars.get(d) or 0, d)
    ranked = sorted(elig, key=tb)
    res["verdict"] = {"winner": group3[0] if told else None, "told_apart": told,
                      "not_told_apart": [] if told else group3, "default": ranked[0],
                      "top_two": ranked[:2], "ranking_point_estimates": ranked}
    return res


def _more_text_share(outcomes, chars):
    n = k = 0
    for x, y, r in outcomes:
        if r == 0.5 or chars.get(x) is None or chars.get(y) is None or chars[x] == chars[y]:
            continue
        winner = x if r == 1.0 else y
        more = x if chars[x] > chars[y] else y
        n += 1
        k += winner == more
    return {"share": round(k / n, 3) if n else None, "n": n}


def paired_difference(rows, a, b, sets, seed, reps=2000):
    """Paired F2:H1 difference a − b of the unit accuracy over the units both designs share
    (same state, language, theme, persona, replicate); seeded bootstrap 95 % (SPEC §G8:
    how clearly the broken design is behind each other design)."""
    sel = [r for r in rows if r["qset"] in sets]
    m = unit_means(sel, lambda r: (r["design"], r.get("state"), r.get("lang"), r.get("theme"), r["persona"], r["rep"],
                                   r.get("model", "")) if r["design"] in (a, b) else None, balance_weights(sel))
    keys = sorted(k[1:] for k in m if k[0] == a and (b,) + k[1:] in m)
    if not keys:
        return None
    diffs = [m[(a,) + k] - m[(b,) + k] for k in keys]
    w = [F_WEIGHT if k[3] == "F" else H_WEIGHT for k in keys]
    est = sum(x * y for x, y in zip(diffs, w)) / sum(w)
    rng = random.Random(seed)
    bs = []
    for _ in range(reps):
        idx = [rng.randrange(len(keys)) for _ in keys]
        bs.append(sum(diffs[i] * w[i] for i in idx) / sum(w[i] for i in idx))
    bs.sort()
    lo, hi = bs[int(0.025 * reps)], bs[min(reps - 1, int(0.975 * reps))]
    return {"diff": round(est, 4), "lo": round(lo, 4), "hi": round(hi, 4), "n": len(keys), "separated": lo > 0}


def broken_verdict(res, broken: str) -> dict:
    """SPEC §G8: the harness is valid only if the broken design is last on the primary
    measure and last on the critic BT (point estimates), and its primary interval lies
    below the best design's interval."""
    pd = res["per_design"]
    designs = [d for d in res["designs"] if pd.get(d, {}).get("primary")]
    out = {"broken": broken, "checks": {}}
    if broken not in designs:
        return dict(out, valid=None, note="broken design not measured")
    prim = {d: pd[d]["primary"][0] for d in designs}
    # Strictly last: a tie with another design is not last.
    out["checks"]["last_on_primary"] = all(prim[broken] < v for d, v in prim.items() if d != broken)
    best = max(designs, key=lambda d: prim[d])
    out["checks"]["primary_below_best_interval"] = pd[broken]["primary"][2] < pd[best]["primary"][1]
    bt = res.get("bt") or {}
    if broken in bt:
        out["checks"]["last_on_bt"] = all(bt[broken][0] < v[0] for d, v in bt.items() if d != broken)
    rd = {d: pd[d]["reading"][0] for d in designs if pd[d].get("reading")}
    if broken in rd:
        out["checks"]["last_on_reading_info"] = all(rd[broken] < v for d, v in rd.items() if d != broken)
    pv = res.get("paired_vs_broken") or {}
    if pv:
        out["paired_primary_info"] = pv
        out["checks"]["paired_separated_from_all_info"] = all(v and v["separated"] for v in pv.values())
    required = ["last_on_primary"] + (["last_on_bt"] if "last_on_bt" in out["checks"] else [])
    out["required"] = required
    out["valid"] = all(out["checks"][c] for c in required)
    return out


def _fmt(t):
    if not t:
        return "—"
    return f"{t[0]:.3f} [{t[1]:.3f}, {t[2]:.3f}] (F {t[3]}, H {t[4]})" if len(t) > 3 else f"{t[0]:.3f} [{t[1]:.3f}, {t[2]:.3f}]"


def scores_md(res) -> str:
    L = ["# Tek bakış düzeneği — puanlar", "",
         "> Ajan okuyucu insan değildir; bu ölçüm ön elemedir (SPEC §G0).", "",
         f"| tasarım | birincil: {','.join((res.get('sets') or {}).get('primary', ['throttle']))} (F2:H1) | okuma: tam boy | okuma: yarım boy | gri | akış | kanıt | hareket | hareket (yalnız son kare) | BT [%95] | kesilen |",
         "|---|---|---|---|---|---|---|---|---|---|---:|"]
    for d in res["designs"]:
        p = res["per_design"][d]
        b = res.get("bt", {}).get(d)
        L.append(f"| `{d}` | {_fmt(p['primary'])} | {_fmt(p['reading'])} | {_fmt(p['half'])} | {_fmt(p['gray'])} | "
                 f"{_fmt(p['feed'])} | {_fmt(p['evidence'])} | {_fmt(p['motion'])} | {_fmt(p['motion_final'])} | "
                 f"{_fmt(b) if b else '—'} | {p['cut_answers']} |")
    L += ["", "## Konum çelişkisi (U1)", ""]
    for k, v in sorted(res.get("conflicts", {}).items()):
        L.append(f"- `{k}`: {v['split']}/{v['total']} = {v['rate']}{' → ayırt edilemedi' if v['not_told_apart'] else ''}")
    L += ["", f"Uzunluk yanlılığı (U2): {res.get('length_bias')}", "",
          f"Okuyucu modelleri arası uyum: {res.get('model_agreement')}", "",
          f"Tek bakış dışı çağrılar (birden çok Read; puanlamadan çıkarıldı): {res.get('excluded_not_one_look')}", "",
          "## Seçim kuralının uygulanışı", ""]
    for s in res["steps"]:
        L.append(f"- {s}")
    L += ["", f"**Karar:** {res.get('verdict')}", ""]
    return "\n".join(L)


def computed_md(out) -> str:
    L = ["# Hesapla ölçülenler (sert eşikler)", "",
         "| tasarım | metin karşıtlığı (en kötü / eşik) | işaret karşıtlığı (≥ 3) | en küçük yazı (≥ 11 px) | dokunma < 44 | renk ΔE00 en küçük / eşik | yüz karakter ort./en çok | ekran karakter ort. | K52 | geçti |",
         "|---|---|---|---|---:|---|---|---|---|---|"]
    for d, g in sorted(out.items()):
        tc = g["text_contrast"]["worst"] or {}
        gl = g["nontext_contrast"]["worst_glyph"] or {}
        mf = g["min_font"]["worst"] or {}
        # The gate's worst pair: colour-coded pairs first (the others are not gated), per
        # theme (Ek G4-2: pairs are within one theme).
        pairs = [r for r in g["colour"]["pairs"] if r.get("colour_coded", True)] or g["colour"]["pairs"]
        cm = min(pairs, key=lambda r: r["min"] - r["threshold"]) if pairs else None
        if cm and cm.get("theme"):
            cm = dict(cm, a=f"{cm['theme']}: {cm['a']}")
        k52 = g.get("k52", {}).get("pass", "—")
        dn = g["density"]
        L.append(f"| `{d}` | {tc.get('ratio')} / {tc.get('threshold')} | {gl.get('ratio')} | {mf.get('px')} | {g['touch']['count']} | "
                 f"{(str(cm['min']) + ' / ' + str(cm['threshold']) + ' (' + cm['a'] + ' ↔ ' + cm['b'] + ')') if cm else '—'} | "
                 f"{dn['face_chars_mean']} / {dn['face_chars_max']} | {dn['screen_chars_mean']} | {k52} | {g['pass']} |")
    sh = [(d, g["colour"].get("shared")) for d, g in sorted(out.items()) if g["colour"].get("rule")]
    if sh:
        L += ["", "Renk kapısı Ek G5 (görünüş kaydının rolleri). Bilerek paylaşılan roller (sayılmaz; gri soruyla sınanır):", ""]
        for d, s in sh:
            L.append(f"- `{d}`: " + (", ".join(f"{x['theme']}: {x['a']} = {x['b']}" for x in s) or "yok"))
    L.append("")
    return "\n".join(L)
