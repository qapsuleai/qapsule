"""`arch-check --config-diff` (K-9; RG1 denetim B5): prints the diff of
scripts/arch-config.json and scripts/arch-baseline.json against a base revision and judges
it. Tightening and removals pass. A loosening passes only when it cites a decision
(`K<n>` or `DECISION.md`) that the base did not already cite for it:

- a new exception / layer override: in its `status` or `reason`; a raised or dropped `max`,
  or a changed override layer: a citation new to that entry (`status`, `reason`, `max_note`);
- a new `generated_files` entry: written as `{"path", "reason"}`, the reason citing;
- a new or raised baseline item: in its `note` (item written as `{"value", "note"}`); a new
  item that only moved (same rule, same function name / file name, value not above the
  removed one) passes as a move; `lint.allow` may spread over new crates while the rule's
  total does not grow; a rule count may rise only by the cited new items;
- a raised budget, a removed hard rule / forbidden name / doc glob, a crate moved to another
  layer, or two crates swapped in a layer order: a new top-level `rule_changes` entry
  `{"key": "budgets.fn_lines" | "hard_rules.<r>" | "forbidden_names.<n>" | "doc_globs.<g>" |
  "layers.<crate>" | "order.<layer>", "reason": "K<n> …"}`.
"""

import json
import os
import re
import subprocess

FILES = ("scripts/arch-config.json", "scripts/arch-baseline.json")
ROUND_BASELINE = "rounds/RF1/experiments/baseline"  # default base on main (VERIFY 8)
DECISION = re.compile(r"\bK\d+\b|DECISION\.md")
SPREAD = ("lint.allow",)  # per-crate counts: a crate split spreads them (sum may not grow)


def _git(root, *args):
    r = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True)
    return r.stdout.strip() if r.returncode == 0 else None


def find_base(root, rev=None):
    """(revision, how) or (None, reason)."""
    if rev:
        return rev, "given"
    if os.environ.get("QP_CONFIG_BASE"):
        return os.environ["QP_CONFIG_BASE"], "$QP_CONFIG_BASE"
    head = _git(root, "rev-parse", "HEAD")
    for ref in ("main", "origin/main"):
        mb = _git(root, "merge-base", "HEAD", ref)
        if mb and mb != head:
            return mb, f"merge-base with {ref}"
        if mb:
            break  # on main itself (or behind it): compare with the round's baseline
    rb = _git(root, "log", "-1", "--format=%H", "--", ROUND_BASELINE)
    if rb:
        return rb, f"last commit touching {ROUND_BASELINE}"
    return None, "no main branch and no round baseline commit (shallow clone?)"


def _load_rev(root, rev, path):
    text = _git(root, "show", f"{rev}:{path}")
    return json.loads(text) if text else {}


def cites(*texts):
    return set(DECISION.findall(" ".join(t for t in texts if isinstance(t, str))))


def _entry_text(e):
    return [e.get(k, "") for k in ("status", "reason", "max_note", "note")] if isinstance(e, dict) else []


def _new_cite(old, new):
    return cites(*_entry_text(new)) - cites(*_entry_text(old))


def _rule_changes(old_cfg, new_cfg):
    """Keys covered by a `rule_changes` entry that is new and cites a decision."""
    seen = {(r.get("key"), r.get("reason")) for r in old_cfg.get("rule_changes", [])}
    return {r.get("key") for r in new_cfg.get("rule_changes", [])
            if (r.get("key"), r.get("reason")) not in seen and cites(r.get("reason", ""))}


class Verdict:
    def __init__(self):
        self.lines, self.fails = [], 0

    def ok(self, kind, text):
        self.lines.append(f"{kind:9} {text}")

    def loosen(self, text, cited):
        if cited:
            self.lines.append(f"LOOSER    {text} (cites {', '.join(sorted(cited))})")
        else:
            self.fails += 1
            self.lines.append(f"FAIL      {text}: a loosening must cite a decision (K<n> / DECISION.md)")


def judge_config(old, new, v):
    covered = _rule_changes(old, new)

    def rule_change(key, text):
        v.loosen(text, {f"rule_changes[{key}]"} if key in covered else set())

    oe = {(e["rule"], e["item"]): e for e in old.get("exceptions", [])}
    ne = {(e["rule"], e["item"]): e for e in new.get("exceptions", [])}
    for k in sorted(oe.keys() - ne.keys()):
        v.ok("removed", f"exception {oe[k].get('id', '')} [{k[0]}] {k[1]}")
    for k in sorted(ne.keys() - oe.keys()):
        v.loosen(f"new exception {ne[k].get('id', '')} [{k[0]}] {k[1]}", cites(*_entry_text(ne[k])))
    for k in sorted(ne.keys() & oe.keys()):
        a, b = oe[k], ne[k]
        if "max" in a and ("max" not in b or b["max"] > a["max"]):
            v.loosen(f"exception [{k[0]}] {k[1]}: max {a['max']} -> {b.get('max', 'none')}",
                     _new_cite(a, b))
        elif "max" in b and ("max" not in a or b["max"] < a["max"]):
            v.ok("tighter", f"exception [{k[0]}] {k[1]}: max {a.get('max', 'none')} -> {b['max']}")
    oo, no = old.get("layer_overrides", {}), new.get("layer_overrides", {})
    for c in sorted(oo.keys() - no.keys()):
        v.ok("removed", f"layer override {c}")
    for c in sorted(no.keys() - oo.keys()):
        v.loosen(f"new layer override {c} -> {no[c].get('layer')}", cites(*_entry_text(no[c])))
    for c in sorted(no.keys() & oo.keys()):
        if no[c].get("layer") != oo[c].get("layer"):
            v.loosen(f"layer override {c}: {oo[c].get('layer')} -> {no[c].get('layer')}",
                     _new_cite(oo[c], no[c]))
    og = {g if isinstance(g, str) else g.get("path") for g in old.get("generated_files", [])}
    for g in new.get("generated_files", []):
        path = g if isinstance(g, str) else g.get("path")
        if path not in og:
            v.loosen(f"new generated file {path}", cites(*_entry_text(g)))
    for g in sorted(og - {g if isinstance(g, str) else g.get("path") for g in new.get("generated_files", [])}):
        v.ok("removed", f"generated file {g}")
    ob, nb = old.get("budgets", {}), new.get("budgets", {})
    for k in sorted(ob.keys() | nb.keys()):
        a, b = ob.get(k), nb.get(k)
        if a is not None and (b is None or b > a):
            rule_change(f"budgets.{k}", f"budget {k}: {a} -> {b}")
        elif b is not None and (a is None or b < a):
            v.ok("tighter", f"budget {k}: {a} -> {b}")
    for field in ("hard_rules", "forbidden_names", "doc_globs"):
        a, b = set(old.get(field, [])), set(new.get(field, []))
        for x in sorted(a - b):
            rule_change(f"{field}.{x}", f"{field}: {x} removed")
        for x in sorted(b - a):
            v.ok("tighter", f"{field}: {x} added")
    _judge_layers(old, new, v, rule_change)


def _judge_layers(old, new, v, rule_change):
    def where(cfg):
        return {c: (i, l["name"]) for i, l in enumerate(cfg.get("layers", [])) for c in l["crates"]}

    wa, wb = where(old), where(new)
    for c in sorted(wb.keys() - wa.keys()):
        v.ok("placed", f"new crate {c} in layer `{wb[c][1]}`")
    for c in sorted(wa.keys() & wb.keys()):
        if wa[c][0] != wb[c][0]:
            rule_change(f"layers.{c}", f"crate {c}: layer `{wa[c][1]}` -> `{wb[c][1]}`")
    oo = {l["name"]: l.get("order") for l in old.get("layers", [])}
    for layer in new.get("layers", []):
        a, b = oo.get(layer["name"]), layer.get("order") or []
        if a is None:
            v.ok("declared", f"order of `{layer['name']}`: {' < '.join(b)}")
            continue
        both = [c for c in b if c in a]
        swapped = [(x, y) for i, x in enumerate(both) for y in both[i + 1:] if a.index(x) > a.index(y)]
        if swapped:
            pairs = ", ".join(f"{x}<{y}" for x, y in swapped)
            rule_change(f"order.{layer['name']}", f"order of `{layer['name']}` swaps {pairs}")


def _val(x):
    return x["value"] if isinstance(x, dict) else x


def _tail(rule, item):
    return item.rsplit("::", 1)[-1] if rule == "fn.lines" else os.path.basename(item)


def judge_baseline(old, new, v):
    orules, nrules = old.get("rules", {}), new.get("rules", {})
    for rule in sorted(orules.keys() - nrules.keys()):
        v.ok("removed", f"baseline rule {rule}")
    for rule in sorted(nrules):
        oi = orules.get(rule, {}).get("items", {})
        ni = nrules[rule].get("items", {})
        gone = {k: _val(oi[k]) for k in oi.keys() - ni.keys()}
        for k in sorted(ni.keys() & oi.keys()):
            a, b = _val(oi[k]), _val(ni[k])
            if b > a:
                v.loosen(f"baseline {rule} {k}: {a} -> {b}", _new_cite(oi[k], ni[k]))
            elif b < a:
                v.ok("tighter", f"baseline {rule} {k}: {a} -> {b}")
        cited_new = 0
        spread_ok = rule in SPREAD and sum(_val(x) for x in ni.values()) <= sum(_val(x) for x in oi.values())
        for k in sorted(ni.keys() - oi.keys()):
            b = _val(ni[k])
            mate = next((g for g in sorted(gone) if _tail(rule, g) == _tail(rule, k) and b <= gone[g]), None)
            if mate is not None:
                del gone[mate]
                v.ok("moved", f"baseline {rule} {mate} -> {k} ({b})")
            elif spread_ok:
                v.ok("spread", f"baseline {rule} {k} ({b}; the rule's total does not grow)")
                cited_new += 1
            else:
                c = cites(*_entry_text(ni[k]))
                cited_new += bool(c)
                v.loosen(f"new baseline item {rule} {k} ({b})", c)
        for k in sorted(gone):
            v.ok("removed", f"baseline {rule} {k}")
        a, b = orules.get(rule, {}).get("count", 0), nrules[rule].get("count", 0)
        if b > a + cited_new:
            v.loosen(f"baseline {rule} count {a} -> {b}", set())
        elif b < a:
            v.ok("tighter", f"baseline {rule} count {a} -> {b}")


def judge(old_cfg, new_cfg, old_base, new_base):
    v = Verdict()
    judge_config(old_cfg, new_cfg, v)
    judge_baseline(old_base, new_base, v)
    return v


def main(root, rev=None):
    base, how = find_base(root, rev)
    if base is None:
        print(f"config-diff: {how}")
        return 2
    short = _git(root, "rev-parse", "--short", base) or base
    print(f"config-diff: base {short} ({how})")
    diff = _git(root, "diff", "--no-color", base, "--", *FILES)
    print(diff if diff else "(no change in arch-config.json / arch-baseline.json)")
    cur = []
    for f in FILES:
        with open(os.path.join(root, f), encoding="utf-8") as fh:
            cur.append(json.load(fh))
    v = judge(_load_rev(root, base, FILES[0]), cur[0], _load_rev(root, base, FILES[1]), cur[1])
    for line in v.lines:
        print(line)
    print(f"config-diff: {len(v.lines)} change(s), {v.fails} uncited loosening(s)")
    return 1 if v.fails else 0
