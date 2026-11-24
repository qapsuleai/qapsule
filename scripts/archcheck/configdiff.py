"""`arch-check --config-diff` (K-9; RG1 denetim B5; RF1/A denetimi O1–O4, D1, D5): prints
the diff of scripts/arch-config.json and scripts/arch-baseline.json against a base revision
and judges it. Tightening and removals pass. A loosening passes only with a real, new
decision citation (citation.py: a `K<n>` row under docs/decisions/ added since the base,
or a `rounds/<R>/DECISION.md` new or changed since the base that names the entry):

- a new exception / layer override, a raised or dropped `max`, a changed override layer:
  in the entry's `status` / `reason` / `max_note`;
- a new `generated_files` entry: written as `{"path", "reason"}`;
- a new or raised baseline item: in its `note` (item written as `{"value", "note"}`). A new
  item that replaces a vanished one (same rule, same function name / file name, value not
  above it) passes as a move only inside the same crate or into a crate new in this
  change; `lint.allow` may spread only onto new crates while the rule's total does not
  grow; a rule count may rise only by the accepted new items;
- any other configuration change that loosens — a raised budget; a removed hard rule /
  forbidden name / doc glob; a crate moved to another layer; two crates whose relative
  order (in any layer, whatever its name) is reversed; a layer added, removed or renamed;
  `shell_layer` changed; any key this check does not know — needs a new top-level
  `rule_changes` entry `{"key": "<key>", "reason": "K<n> …"}` (keys are printed).

Known limits: decision rows are read from the working tree, so a change could add its own
row; every row added since the base is printed (R2). A "moved" baseline item is paired by
name and crate, not by content: inside one crate an unrelated same-named function can take
over a vanished item's allowance, and a crate newly declared in this change is free ground
for moved items and `lint.allow` (R3); the `moved` lines are printed for the reviewer.
"""

import json
import os
import subprocess

from .citation import Citer

FILES = ("scripts/arch-config.json", "scripts/arch-baseline.json")
ROUND_BASELINE = "rounds/RF1/experiments/baseline"  # default base on main (VERIFY 8)
SPREAD = ("lint.allow",)  # per-crate counts: a crate split spreads them onto new crates
KNOWN = {"layers", "layer_overrides", "shell_layer", "budgets", "forbidden_names",
         "doc_globs", "hard_rules", "generated_files", "exceptions", "rule_changes"}


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


class Verdict:
    def __init__(self, citer):
        self.citer = citer
        self.lines, self.fails = [], 0
        self.covered = set()

    def ok(self, kind, text):
        self.lines.append(f"{kind:9} {text}")

    def fail(self, text, why):
        self.fails += 1
        self.lines.append(f"FAIL      {text}: {why}")

    def loosen(self, name, text, *cite_texts):
        good, bad = self.citer.fresh(name, *cite_texts)
        if good:
            self.lines.append(f"LOOSER    {text} (cites {', '.join(sorted(good))})")
            return True
        why = "; ".join(bad) if bad else "no decision cited"
        self.fail(text, f"a loosening needs a new decision (K<n> row added since the base / changed rounds/<R>/DECISION.md naming `{name}`) — {why}")
        return False

    def rule_change(self, key, text):
        if key in self.covered:
            self.lines.append(f"LOOSER    {text} (rule_changes[{key}])")
        else:
            self.fail(text, f"needs a new rule_changes entry {{\"key\": \"{key}\", \"reason\": \"K<n> …\"}}")


def _entry_text(e):
    return [e.get(k, "") for k in ("status", "reason", "max_note", "note")] if isinstance(e, dict) else []


def _cover(old, new, v):
    """`rule_changes` keys that are new and carry a real, new citation."""
    seen = {(r.get("key"), r.get("reason")) for r in old.get("rule_changes", [])}
    for r in new.get("rule_changes", []):
        if (r.get("key"), r.get("reason")) not in seen and v.citer.fresh(r.get("key", ""), r.get("reason", ""))[0]:
            v.covered.add(r.get("key"))


def judge_config(old, new, v):
    _cover(old, new, v)
    _judge_exceptions(old, new, v)
    og = {g if isinstance(g, str) else g.get("path") for g in old.get("generated_files", [])}
    ng = {g if isinstance(g, str) else g.get("path"): g for g in new.get("generated_files", [])}
    for path in sorted(ng.keys() - og):
        v.loosen(path, f"new generated file {path}", *_entry_text(ng[path]))
    for path in sorted(og - ng.keys()):
        v.ok("removed", f"generated file {path}")
    ob, nb = old.get("budgets", {}), new.get("budgets", {})
    for k in sorted(ob.keys() | nb.keys()):
        a, b = ob.get(k), nb.get(k)
        if a is not None and (b is None or b > a):
            v.rule_change(f"budgets.{k}", f"budget {k}: {a} -> {b}")
        elif b is not None and (a is None or b < a):
            v.ok("tighter", f"budget {k}: {a} -> {b}")
    for field in ("hard_rules", "forbidden_names", "doc_globs"):
        a, b = set(old.get(field, [])), set(new.get(field, []))
        for x in sorted(a - b):
            v.rule_change(f"{field}.{x}", f"{field}: {x} removed")
        for x in sorted(b - a):
            v.ok("tighter", f"{field}: {x} added")
    if old.get("shell_layer") != new.get("shell_layer") and "shell_layer" in old:
        v.rule_change("shell_layer", f"shell_layer: {old.get('shell_layer')} -> {new.get('shell_layer')}")
    for k in sorted((old.keys() | new.keys()) - KNOWN):
        if not k.startswith("_") and old.get(k) != new.get(k):
            v.rule_change(k, f"unknown configuration key `{k}` changed")
    _judge_layers(old, new, v)


def _judge_exceptions(old, new, v):
    oe = {(e["rule"], e["item"]): e for e in old.get("exceptions", [])}
    ne = {(e["rule"], e["item"]): e for e in new.get("exceptions", [])}
    gone = oe.keys() - ne.keys()
    fresh = crates_of(new) - crates_of(old)
    moved_from = set()
    for k in sorted(ne.keys() - oe.keys()):
        src = _moved_exception(oe, gone - moved_from, ne[k], fresh)
        if src is not None:
            # A crate split moves an exception onto a crate this change introduces: same
            # id, same rule, the old item gone, ceiling not raised (RF1 DECISION.md, S3.0).
            moved_from.add(src)
            v.ok("moved", f"exception {ne[k].get('id', '')} [{k[0]}] {src[1]} -> {k[1]} (new crate; max {ne[k].get('max', 'none')})")
            continue
        v.loosen(k[1], f"new exception {ne[k].get('id', '')} [{k[0]}] {k[1]}", *_entry_text(ne[k]))
    for k in sorted(gone - moved_from):
        v.ok("removed", f"exception {oe[k].get('id', '')} [{k[0]}] {k[1]}")
    for k in sorted(ne.keys() & oe.keys()):
        a, b = oe[k], ne[k]
        if "max" in a and ("max" not in b or b["max"] > a["max"]):
            v.loosen(k[1], f"exception [{k[0]}] {k[1]}: max {a['max']} -> {b.get('max', 'none')}", *_entry_text(b))
        elif "max" in b and ("max" not in a or b["max"] < a["max"]):
            v.ok("tighter", f"exception [{k[0]}] {k[1]}: max {a.get('max', 'none')} -> {b['max']}")
    oo, no = old.get("layer_overrides", {}), new.get("layer_overrides", {})
    for c in sorted(oo.keys() - no.keys()):
        v.ok("removed", f"layer override {c}")
    for c in sorted(no.keys() - oo.keys()):
        v.loosen(c, f"new layer override {c} -> {no[c].get('layer')}", *_entry_text(no[c]))
    for c in sorted(no.keys() & oo.keys()):
        if no[c].get("layer") != oo[c].get("layer"):
            v.loosen(c, f"layer override {c}: {oo[c].get('layer')} -> {no[c].get('layer')}", *_entry_text(no[c]))


def _moved_exception(oe, gone, e, fresh):
    """The old exception key `e` moved from: same id and rule, the old one gone, `e`'s item
    on a crate this change introduces, and its `max` not above the old one's."""
    if _crate(e["item"]) not in fresh:
        return None
    for k in sorted(gone):
        a = oe[k]
        if a.get("id") != e.get("id") or a["rule"] != e["rule"]:
            continue
        if "max" in a and ("max" not in e or e["max"] > a["max"]):
            continue
        return k
    return None


def crates_of(cfg):
    return {c for l in cfg.get("layers", []) for c in l.get("crates", [])} | set(cfg.get("layer_overrides", {}))


def _judge_layers(old, new, v):
    ol, nl = old.get("layers", []), new.get("layers", [])
    if [l["name"] for l in ol] != [l["name"] for l in nl] and ol:
        v.rule_change("layers.names", f"layers {[l['name'] for l in ol]} -> {[l['name'] for l in nl]}")
    wa = {c: i for i, l in enumerate(ol) for c in l["crates"]}
    wb = {c: i for i, l in enumerate(nl) for c in l["crates"]}
    for c in sorted(wb.keys() - wa.keys()):
        v.ok("placed", f"new crate {c} in layer `{nl[wb[c]]['name']}`")
    for c in sorted(wa.keys() & wb.keys()):
        if wa[c] != wb[c]:
            v.rule_change(f"layers.{c}", f"crate {c}: layer {wa[c]} -> {wb[c]}")
    # relative order of every crate pair, whatever the layer is called (O1)
    below = {(x, y) for l in ol for i, x in enumerate(l.get("order") or [])
             for y in (l.get("order") or [])[i + 1:]}
    for l in nl:
        order = l.get("order") or []
        if not any(l2.get("order") for l2 in ol):
            v.ok("declared", f"order of `{l['name']}`: {' < '.join(order)}")
            continue
        for i, x in enumerate(order):
            for y in order[i + 1:]:
                if (y, x) in below:
                    v.rule_change(f"order.{y}<{x}", f"order of `{l['name']}`: {x} now below {y} (was above)")


def _val(x):
    return x["value"] if isinstance(x, dict) else x


def _tail(rule, item):
    return item.rsplit("::", 1)[-1] if rule == "fn.lines" else os.path.basename(item)


def _crate(item):
    parts = item.split("/")
    return parts[1] if len(parts) > 1 and parts[0] == "crates" else item


def judge_baseline(old, new, v, new_crates=frozenset()):
    orules, nrules = old.get("rules", {}), new.get("rules", {})
    for k in sorted((old.keys() | new.keys()) - {"rules"}):
        if not k.startswith("_") and old.get(k) != new.get(k):
            v.rule_change(f"baseline.{k}", f"unknown baseline key `{k}` changed")
    for rule in sorted(orules.keys() - nrules.keys()):
        v.ok("removed", f"baseline rule {rule}")
    for rule in sorted(nrules):
        oi = orules.get(rule, {}).get("items", {})
        ni = nrules[rule].get("items", {})
        gone = {k: _val(oi[k]) for k in oi.keys() - ni.keys()}
        for k in sorted(ni.keys() & oi.keys()):
            a, b = _val(oi[k]), _val(ni[k])
            if b > a:
                v.loosen(k, f"baseline {rule} {k}: {a} -> {b}", *_entry_text(ni[k]))
            elif b < a:
                v.ok("tighter", f"baseline {rule} {k}: {a} -> {b}")
        accepted = 0
        total_ok = sum(_val(x) for x in ni.values()) <= sum(_val(x) for x in oi.values())
        for k in sorted(ni.keys() - oi.keys()):
            b, crate = _val(ni[k]), _crate(k)
            mate = next((g for g in sorted(gone) if _tail(rule, g) == _tail(rule, k) and b <= gone[g]
                         and (_crate(g) == crate or crate in new_crates)), None)
            if mate is not None:
                del gone[mate]
                v.ok("moved", f"baseline {rule} {mate} -> {k} ({b})")
            elif rule in SPREAD and total_ok and crate in new_crates:
                v.ok("spread", f"baseline {rule} {k} ({b}; new crate; the rule's total does not grow)")
                accepted += 1
            else:
                accepted += v.loosen(k, f"new baseline item {rule} {k} ({b})", *_entry_text(ni[k]))
        for k in sorted(gone):
            v.ok("removed", f"baseline {rule} {k}")
        a, b = orules.get(rule, {}).get("count", 0), nrules[rule].get("count", 0)
        if b > a + accepted:
            v.fail(f"baseline {rule} count {a} -> {b}", "more than the accepted new items")
        elif b < a:
            v.ok("tighter", f"baseline {rule} count {a} -> {b}")


def judge(old_cfg, new_cfg, old_base, new_base, citer):
    v = Verdict(citer)
    judge_config(old_cfg, new_cfg, v)
    judge_baseline(old_base, new_base, v, frozenset(crates_of(new_cfg) - crates_of(old_cfg)))
    return v


def _check_base(root, base, how):
    """The base must be a commit, and not HEAD or a descendant of it (D5)."""
    full = _git(root, "rev-parse", "--verify", f"{base}^{{commit}}")
    if not full:
        return f"base {base} ({how}) is not a commit here"
    if _git(root, "merge-base", "--is-ancestor", "HEAD", full) is not None:
        return (f"WARNING: base {base} ({how}) is HEAD or a descendant of HEAD — the diff would "
                f"be empty or reversed and judge nothing")
    return None


def main(root, rev=None):
    base, how = find_base(root, rev)
    if base is None:
        print(f"config-diff: {how}")
        return 2
    bad = _check_base(root, base, how)
    if bad and how not in ("given", "$QP_CONFIG_BASE") and "WARNING" in bad:
        print(f"config-diff: base {base} ({how}) is HEAD itself: nothing to judge")
        return 0
    if bad:  # an explicit base that is not a commit, or HEAD / a descendant (D5)
        print(f"config-diff: {bad}")
        return 1
    print(f"config-diff: base {_git(root, 'rev-parse', '--short', base)} ({how})")
    diff = _git(root, "diff", "--no-color", base, "--", *FILES)
    print(diff if diff else "(no change in arch-config.json / arch-baseline.json)")
    old_text = [_git(root, "show", f"{base}:{f}") or "" for f in FILES]
    cur = []
    for f in FILES:
        with open(os.path.join(root, f), encoding="utf-8") as fh:
            cur.append(json.load(fh))
    old = [json.loads(t) if t else {} for t in old_text]
    citer = Citer.from_repo(root, base, "\n".join(old_text))
    for k in sorted(citer.new_rows, key=lambda x: int(x[1:])):
        print(f"decision row added since the base (R2: check who wrote it): {k}")
    for p in sorted(citer.changed):
        print(f"DECISION.md new or changed since the base: {p}")
    v = judge(old[0], cur[0], old[1], cur[1], citer)
    for line in v.lines:
        print(line)
    print(f"config-diff: {len(v.lines)} change(s), {v.fails} uncited loosening(s)")
    return 1 if v.fails else 0
