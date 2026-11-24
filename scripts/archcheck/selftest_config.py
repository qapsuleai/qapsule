"""`arch-check --self-test`, part 2: `--config-diff` (K-9) — every loosening needs a real,
new decision; tightening and removals pass; the RF1/A audit's escapes (O1–O4, D1, D5)
are caught."""

import copy
import json
import os
import subprocess
import tempfile

from . import configdiff
from .citation import Citer

ROWS = {"K1", "K85", "K93", "K94"}       # decision rows in the working tree
BASE_ROWS = {"K1", "K85"}                 # … and at the base: K93, K94 are new decisions
CHANGED = {"rounds/RX/DECISION.md": "İstisna crates/qp-b/src/x.rs kabul.",  # names the item
           "rounds/RZ/DECISION.md": "Başka bir konu."}                       # does not


def run(CFG, check):
    ok = True
    ex = {"id": "T", "rule": "file.hard", "item": "crates/qp-b/src/x.rs", "max": 50,
          "status": "istek", "reason": "tek fonksiyon"}
    base_cfg = dict(CFG, exceptions=[dict(ex, item="crates/qp-b/src/old.rs", status="onaylandı (K85)")])
    base = {"rules": {"fn.lines": {"count": 1, "items": {"crates/qp-b/src/a.rs::f": 90}},
                      "lint.allow": {"count": 1, "items": {"qp-b": 5}}}}

    def fails(new_cfg, new_b=base, old_cfg=base_cfg, old_b=base):
        citer = Citer(ROWS, BASE_ROWS, CHANGED, json.dumps(old_cfg) + json.dumps(old_b))
        return configdiff.judge(old_cfg, new_cfg, old_b, new_b, citer).fails

    def with_ex(e):
        return dict(base_cfg, exceptions=base_cfg["exceptions"] + [e])

    cases = [
        ("new exception without a decision fails", fails(with_ex(ex)) == 1),
        ("new exception citing a new decision row (K93) passes",
         fails(with_ex(dict(ex, status="onaylandı (K93)"))) == 0),
        ("new exception citing a changed DECISION.md that names it passes",
         fails(with_ex(dict(ex, reason="rounds/RX/DECISION.md"))) == 0),
        ("R1: a changed DECISION.md that does not name the item fails",
         fails(with_ex(dict(ex, reason="rounds/RZ/DECISION.md"))) == 1),
        ("R1: an old round's unchanged rounds/RD2/DECISION.md fails",
         fails(with_ex(dict(ex, reason="rounds/RD2/DECISION.md"))) == 1),
        ("R1: an old decision row K1 (not cited in the base config) fails",
         fails(with_ex(dict(ex, status="onaylandı (K1)"))) == 1),
        ("O2: new exception citing K85 copied from another entry fails",
         fails(with_ex(dict(ex, status="onaylandı (K85)"))) == 1),
        ("O2: new exception citing a non-existent K999 fails",
         fails(with_ex(dict(ex, status="onaylandı (K999)"))) == 1),
        ("removing an exception passes", fails(dict(base_cfg, exceptions=[])) == 0),
        ("raising a max with only the old K85 fails", fails(dict(base_cfg, exceptions=[
            dict(base_cfg["exceptions"][0], max=60)])) == 1),
        ("raising a max with a new K94 in max_note passes", fails(dict(base_cfg, exceptions=[
            dict(base_cfg["exceptions"][0], max=60, max_note="K94")])) == 0),
    ]
    loose = copy.deepcopy(base_cfg)
    loose["budgets"]["fn_lines"] = 12
    cases.append(("a raised budget without rule_changes fails", fails(loose) == 1))
    cases.append(("a raised budget with rule_changes citing K93 passes", fails(dict(
        loose, rule_changes=[{"key": "budgets.fn_lines", "reason": "K93"}])) == 0))
    cases.append(("R1: a raised budget with rule_changes citing old K1 fails", fails(dict(
        loose, rule_changes=[{"key": "budgets.fn_lines", "reason": "K1"}])) == 1))
    cases.append(("new generated file without a decision fails",
                  fails(dict(base_cfg, generated_files=["crates/qp-b/src/t.rs"])) == 1))
    swap = copy.deepcopy(base_cfg)
    swap["layers"][1]["order"] = ["qp-b", "qp-c"]
    cases.append(("swapping a layer order fails", fails(swap) == 1))
    renamed = copy.deepcopy(swap)
    renamed["layers"][1]["name"] = "mid2"
    renamed["shell_layer"] = renamed["shell_layer"]
    cases.append(("O1: renaming the layer and reversing its order fails", fails(renamed) >= 2))
    sh = copy.deepcopy(base_cfg)
    sh["layers"].append({"name": "empty", "crates": [], "order": []})
    sh["shell_layer"] = "empty"
    cases.append(("O4: a new layer and shell_layer moved to it fail", fails(sh) == 2))
    cases.append(("O4: an unknown configuration key changed fails", fails(dict(base_cfg, extra=1)) == 1))
    newc = copy.deepcopy(base_cfg)
    newc["layers"][1]["crates"].append("qp-n")
    newc["layers"][1]["order"].append("qp-n")

    def fn_moved(to, cfg=base_cfg):
        b = copy.deepcopy(base)
        b["rules"]["fn.lines"]["items"] = {to: 90}
        return fails(cfg, b)

    cases += [
        ("a vanished fn item moved inside its crate passes", fn_moved("crates/qp-b/src/z.rs::f") == 0),
        ("a vanished fn item moved into a new crate passes",
         fn_moved("crates/qp-n/src/a.rs::f", newc) == 0),
        ("O3: a same-named fn in another existing crate is new debt", fn_moved("crates/qp-a/src/a.rs::f") == 1),
    ]
    raised = copy.deepcopy(base)
    raised["rules"]["fn.lines"]["items"]["crates/qp-b/src/a.rs::f"] = 95
    cases.append(("a raised baseline value fails", fails(base_cfg, raised) == 1))

    def spread(to, cfg=base_cfg):
        b = copy.deepcopy(base)
        b["rules"]["lint.allow"] = {"count": 2, "items": {"qp-b": 3, to: 2}}
        return fails(cfg, b)

    cases += [("lint.allow spread onto a new crate passes", spread("qp-n", newc) == 0),
              ("D1: lint.allow spread onto an existing crate fails", spread("qp-a") >= 1)]
    # S3.0: an exception moved onto a crate this change introduces (same id, rule, max)
    exc = {"id": "X", "rule": "crate.lines", "item": "qp-a", "max": 100, "status": "onaylandı (K85)"}
    old_x = dict(base_cfg, exceptions=[exc])
    newcr = dict(base_cfg, layers=[dict(l, crates=l["crates"] + ["qp-n"]) if i == 0 else l
                                   for i, l in enumerate(base_cfg["layers"])])
    cases += [
        ("an exception moved to a new crate (same id, rule, max) passes",
         fails(dict(newcr, exceptions=[dict(exc, item="qp-n")]), old_cfg=old_x) == 0),
        ("an exception moved to an existing crate fails",
         fails(dict(base_cfg, exceptions=[dict(exc, item="qp-b")]), old_cfg=old_x) == 1),
        ("an exception moved to a new crate with a raised max fails",
         fails(dict(newcr, exceptions=[dict(exc, item="qp-n", max=101)]), old_cfg=old_x) == 1),
        ("B7: two entries for one rule and item fail",
         fails(dict(newcr, exceptions=[dict(exc, item="qp-n"), dict(exc, item="qp-n")]), old_cfg=old_x) >= 1),
        ("B7: two old exceptions cannot both move onto one new item",
         fails(dict(newcr, exceptions=[dict(exc, item="qp-n")]),
               old_cfg=dict(base_cfg, exceptions=[exc, dict(exc, item="qp-c")])) == 0),
    ]
    for label, hit in cases:
        ok &= check(f"config-diff: {label}", hit)
    ok &= check("config-diff: D5: base = HEAD is refused, HEAD~1 accepted", _base_check())
    return ok


def _base_check():
    """D5: `_check_base` refuses HEAD (or a descendant) as the base."""
    with tempfile.TemporaryDirectory() as d:
        env = dict(os.environ, GIT_AUTHOR_NAME="t", GIT_AUTHOR_EMAIL="t@t", GIT_COMMITTER_NAME="t",
                   GIT_COMMITTER_EMAIL="t@t")
        subprocess.run(["git", "init", "-q", d], check=True, env=env)
        for i in range(2):
            subprocess.run(["git", "-C", d, "commit", "-q", "--allow-empty", "-m", str(i)],
                           check=True, env=env)
        return (configdiff._check_base(d, "HEAD", "t") is not None
                and configdiff._check_base(d, "HEAD~1", "t") is None)
