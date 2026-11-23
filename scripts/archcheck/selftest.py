"""`arch-check --self-test`: builds a small fake workspace with one known violation per
rule (and a clean twin), runs the rules on it and checks that every violation is caught,
the clean twin passes, and the ratchet rejects a value above the baseline. Also: layer
orders (K-2), size-exception ceilings and `--config-diff` (K-9)."""

import copy
import os
import tempfile

from . import baseline as bl
from . import configdiff, rules

CFG = {
    "layers": [
        {"name": "base", "crates": ["qp-a"], "order": ["qp-a"]},
        {"name": "mid", "crates": ["qp-b", "qp-c"], "order": ["qp-c", "qp-b"]},
        {"name": "shell", "crates": ["qp-s"], "order": ["qp-s"]},
    ],
    "layer_overrides": {},
    "shell_layer": "shell",
    "budgets": {"file_soft": 30, "file_hard": 40, "fn_lines": 10, "dir_files": 3,
                "crate_lines": 120, "shell_crate_lines": 60, "test_file": 40,
                "doc_lines": 20, "doc_bytes": 4000},
    "forbidden_names": ["util", "common", "misc", "helpers"],
    "doc_globs": ["*.md", "docs/**/*.md"],
    "hard_rules": ["layer.up", "layer.lateral", "layer.order", "layer.unknown", "file.hard",
                   "name.forbidden", "readme.crate", "root.logic", "path.outside",
                   "lint.inner", "lints.workspace", "lint.flags"],
    "generated_files": [],
    "exceptions": [],
}


def _w(root, rel, text):
    p = os.path.join(root, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w", encoding="utf-8") as fh:
        fh.write(text)


def _fn(name, body_lines):
    body = "".join(f"    let _x{i} = {i};\n" for i in range(body_lines))
    return f"pub fn {name}() {{\n{body}}}\n"


def _crate(root, name, readme=True):
    lints = "" if name == "qp-x" else "\n[lints]\nworkspace = true\n"
    _w(root, f"crates/{name}/Cargo.toml", f'[package]\nname = "{name}"\n{lints}')
    if readme:
        _w(root, f"crates/{name}/README.md", f"# {name}\n")
    _w(root, f"crates/{name}/src/lib.rs", "//! ok\npub mod m;\npub use m::*;\n")
    _w(root, f"crates/{name}/src/m.rs", _fn("small", 3))


def _clean(root):
    for c in ("qp-a", "qp-b", "qp-c", "qp-s"):
        _crate(root, c)
    _w(root, "README.md", "# ok\n")
    # Tricky but legal: braces and `fn` inside strings, raw strings, comments, chars.
    _w(root, "crates/qp-b/src/tricky.rs",
       '//! { fn fake() {\npub fn f() -> char { let _s = "}{ fn x() {"; '
       'let _r = r#"} fn y() {"#; /* } { */ \'{\' }\n'
       "pub fn g<'a>(x: &'a str) -> &'a str { x }\n"
       "#[cfg(test)]\nmod tests {\n" + "".join(f"    // filler {i}\n" for i in range(20)) + "}\n")
    # qp-b -> qp-c is sideways but along the declared order of `mid` (qp-c below qp-b)
    edges = [("qp-b", "qp-a", "normal"), ("qp-s", "qp-b", "normal"), ("qp-a", "qp-s", "dev"),
             ("qp-b", "qp-c", "normal")]
    return edges, ["qp-a", "qp-b", "qp-c", "qp-s"]


EXPECT = {
    "layer.up": "qp-a -> qp-b",
    "layer.lateral": "qp-c -> qp-b",  # against the declared order
    "layer.unknown": "qp-x",
    "file.hard": "crates/qp-b/src/big.rs",
    "file.soft": "crates/qp-b/src/mid.rs",
    "fn.lines": "crates/qp-b/src/long.rs::long",
    "dir.files": "crates/qp-c/src",
    "crate.lines": "qp-b",
    "shell.lines": "qp-s",
    "test.file": "crates/qp-b/tests/big.rs",
    "name.forbidden": "crates/qp-a/src/util.rs",
    "readme.crate": "crates/qp-x",
    "readme.module": "crates/qp-a/src/topic",
    "root.logic": "crates/qp-a/src/topic/mod.rs",
    "doc.size": "docs/BIG.md",
    "path.outside": "crates/qp-a/src/escape.rs->../../qp-b/hidden.rs",
}
EXPECT_EXTRA = [
    ("root.logic", "crates/qp-c/src/lib.rs"),          # an inline `mod x { fn … }` body
    ("fn.lines", "crates/qp-c/src/tests.rs::fake_test"),  # tests.rs not declared cfg(test)
    ("file.hard", "crates/qp-c/src/gen.rs"),             # "üretilmiştir" but not listed
    ("lint.allow", "qp-a"),                               # a grandfathered unwrap allow
    ("lint.allow", "qp-b"),                               # #[expect] / cfg_attr / restriction
    ("lint.inner", "crates/qp-b/src/blanket.rs:1"),       # module-level #![allow] in src
    ("lints.workspace", "crates/qp-x"),                   # crate without workspace lints
    ("lint.allow", "qp-c"),                               # `clippy :: unwrap_used` with spaces
    ("lint.flags", ".cargo/config.toml"),                 # -A clippy::… in rustflags
    ("fn.lines", "crates/qp-b/src/fake/tests.rs::not_a_test"),  # tests.rs not cfg(test) by its parent
]


def _dirty(root):
    edges, names = _clean(root)
    _crate(root, "qp-x", readme=False)
    names.append("qp-x")
    edges += [("qp-a", "qp-b", "normal"), ("qp-c", "qp-b", "normal")]
    _w(root, "crates/qp-b/src/big.rs", "".join(f"// {i}\n" for i in range(45)))
    _w(root, "crates/qp-b/src/mid.rs", "".join(f"// {i}\n" for i in range(35)))
    _w(root, "crates/qp-b/src/long.rs", _fn("long", 12))
    for i in range(3):
        _w(root, f"crates/qp-c/src/f{i}.rs", "// x\n")
    _w(root, "crates/qp-b/src/pad.rs", "".join(f"// {i}\n" for i in range(29)))
    _w(root, "crates/qp-b/src/pad2.rs", "".join(f"// {i}\n" for i in range(29)))
    _w(root, "crates/qp-s/src/pad.rs", "".join(f"// {i}\n" for i in range(29)))
    _w(root, "crates/qp-s/src/pad2.rs", "".join(f"// {i}\n" for i in range(29)))
    _w(root, "crates/qp-b/tests/big.rs", "".join(f"// {i}\n" for i in range(45)))
    _w(root, "crates/qp-a/src/util.rs", "// x\n")
    _w(root, "crates/qp-a/src/topic/mod.rs", "pub mod inner;\npub fn logic() {}\n")
    _w(root, "crates/qp-a/src/topic/inner.rs", "// x\n")
    _w(root, "docs/BIG.md", "".join(f"line {i}\n" for i in range(25)))
    _w(root, "crates/qp-a/src/escape.rs", '#[path = "../../qp-b/hidden.rs"]\nmod hidden;\n')
    _w(root, "crates/qp-c/src/lib.rs", "//! ok\npub mod m;\nmod inner {\n    pub fn x() {}\n}\n")
    _w(root, "crates/qp-c/src/tests.rs", _fn("fake_test", 12))
    _w(root, "crates/qp-a/src/lax.rs", "#[allow(clippy::unwrap_used)]\npub fn f() {}\n")
    _w(root, "crates/qp-b/src/quiet.rs",
       "#[expect(clippy::unwrap_used)]\npub fn a() {}\n"
       "#[cfg_attr(not(test), allow(clippy::panic))]\npub fn b() {}\n"
       "#[allow(clippy::restriction)]\npub fn c() {}\n")
    _w(root, "crates/qp-b/src/blanket.rs", "#![allow(clippy::unwrap_used)]\npub fn d() {}\n")
    _w(root, "crates/qp-c/src/spaced.rs", "#[allow(clippy :: unwrap_used)]\npub fn e() {}\n")
    _w(root, ".cargo/config.toml", '[build]\nrustflags = ["-Aclippy::unwrap_used"]\n')
    # B3: `fake/tests.rs` reached by a plain `pub mod tests;`, while another module has a
    # real `#[cfg(test)] mod tests` — it must still count as library code.
    _w(root, "crates/qp-b/src/fake.rs", "pub mod tests;\n")
    _w(root, "crates/qp-b/src/fake/tests.rs", _fn("not_a_test", 12))
    _w(root, "crates/qp-b/src/real.rs", "pub fn r() {}\n#[cfg(test)]\nmod tests {\n}\n")
    _w(root, "crates/qp-c/src/gen.rs", "// üretilmiştir\n" + "".join(f"// {i}\n" for i in range(45)))
    return edges, names


def run():
    ok = True
    with tempfile.TemporaryDirectory() as clean_root:
        edges, names = _clean(clean_root)
        fs, _ = rules.all_findings(clean_root, CFG, edges, names)
        if fs:
            ok = False
            print("self-test: the clean workspace has findings:")
            for f in fs:
                print("  ", f["message"])
        else:
            print("self-test: clean workspace passes (tricky strings, raw strings, chars, "
                  "lifetimes, comments, test module; dev-dependency upward ignored; a sideways "
                  "dependency along the declared order)")
    with tempfile.TemporaryDirectory() as root:
        edges, names = _dirty(root)
        fs, _ = rules.all_findings(root, CFG, edges, names)
        got = {(f["rule"], f["item"]) for f in fs}
        for rule, item in sorted(EXPECT.items()) + EXPECT_EXTRA:
            hit = (rule, item) in got
            ok &= hit
            print(f"self-test: {rule:15} {item:40} {'caught' if hit else 'MISSED'}")
        # ratchet: a baseline built from these findings passes, one line raised fails
        base = bl.build(fs, CFG["hard_rules"])
        errs, tol, _ = bl.judge(fs, base, CFG["hard_rules"])
        soft_errs = [e for e in errs if e["rule"] not in CFG["hard_rules"]]
        hard_errs = [e for e in errs if e["rule"] in CFG["hard_rules"]]
        good = not soft_errs and hard_errs and tol
        print(f"self-test: baseline tolerates soft debt, never hard rules  "
              f"{'ok' if good else 'FAILED'}")
        ok &= bool(good)
        lowered = copy.deepcopy(base)
        lowered["rules"]["fn.lines"]["items"]["crates/qp-b/src/long.rs::long"] = 11
        errs, _, _ = bl.judge(fs, lowered, CFG["hard_rules"])
        hit = any(e["rule"] == "fn.lines" for e in errs)
        print(f"self-test: ratchet: value above the baseline is an error   "
              f"{'caught' if hit else 'MISSED'}")
        ok &= hit
        _w(root, "crates/qp-b/src/long2.rs", _fn("long2", 12))
        fs2, _ = rules.all_findings(root, CFG, edges, names)
        errs, _, _ = bl.judge(fs2, base, CFG["hard_rules"])
        hit = any(e["rule"] == "fn.lines" and "long2" in e["item"] for e in errs)
        print(f"self-test: ratchet: new debt item is an error              "
              f"{'caught' if hit else 'MISSED'}")
        ok &= hit
        # exceptions: a named exception turns a hard finding into a printed exception
        cfg = dict(CFG, exceptions=[{"rule": "layer.up", "item": "qp-a -> qp-b", "id": "T"}])
        kept, exc, _ = bl.split_exceptions(fs, cfg)
        hit = any(f["rule"] == "layer.up" for f in exc) and not any(
            f["rule"] == "layer.up" for f in kept)
        print(f"self-test: a named exception is reported, not an error    {'ok' if hit else 'MISSED'}")
        ok &= hit
        # latch (K89): a crate.lines exception holds up to its `max`, fails above it
        val = next(f["value"] for f in fs if f["rule"] == "crate.lines" and f["item"] == "qp-b")
        for cap, want in ((val, False), (val - 1, True)):
            ex = {"rule": "crate.lines", "item": "qp-b", "id": "T", "max": cap}
            kept, _, _ = bl.split_exceptions(fs, dict(CFG, exceptions=[ex]))
            errs, _, _ = bl.judge(kept, base, CFG["hard_rules"])
            got = any(e["rule"] == "crate.lines" and "ceiling" in e.get("why", "") for e in errs)
            hit = got == want
            print(f"self-test: latch: crate.lines {val} vs ceiling {cap:<5} "
                  f"{'error' if want else 'holds'}     {'ok' if hit else 'MISSED'}")
            ok &= hit
        hit = bool(bl.missing_ceilings(dict(CFG, exceptions=[
            {"rule": "crate.lines", "item": "qp-b", "id": "T"}])))
        print(f"self-test: latch: crate.lines exception without max       {'caught' if hit else 'MISSED'}")
        ok &= hit
    ok &= _orders()
    ok &= _ceilings()
    ok &= _config_diff()
    print("self-test:", "PASS" if ok else "FAIL")
    return ok


def _check(label, hit):
    print(f"self-test: {label:58} {'ok' if hit else 'MISSED'}")
    return bool(hit)


def _orders():
    """K-2: the layer orders themselves are validated."""
    ok = True
    for order, item, label in ((["qp-b"], "mid: qp-c", "order misses a crate of its layer"),
                               (["qp-c", "qp-b", "qp-c"], "mid: qp-c twice", "order names a crate twice"),
                               (["qp-c", "qp-b", "qp-a"], "mid: qp-a foreign", "order names a crate of another layer")):
        cfg = copy.deepcopy(CFG)
        cfg["layers"][1]["order"] = order
        got = {f["item"] for f in rules.rule_layer_orders(cfg)}
        ok &= _check(f"layer.order: {label}", item in got)
    cfg = copy.deepcopy(CFG)
    cfg["layers"][1]["order"] = ["qp-b"]
    got = {f["item"] for f in rules.rule_layers(cfg, [("qp-b", "qp-c", "normal")], ["qp-b", "qp-c"])}
    ok &= _check("layer.lateral: edge to a crate missing from the order", "qp-b -> qp-c" in got)
    ok &= _check("layer.order: the clean config has none", not list(rules.rule_layer_orders(CFG)))
    return ok


def _ceilings():
    """İ6 / fn: a size exception holds up to its `max`, fails above; it must carry one; a
    fn.lines baseline value cannot grow."""
    ok = True
    f = rules.finding("file.hard", "crates/qp-b/src/one_fn.rs", 45, 40, "one_fn.rs: 45 lines")
    for cap, want in ((45, False), (44, True)):
        ex = {"rule": "file.hard", "item": f["item"], "id": "T", "max": cap}
        kept, _, _ = bl.split_exceptions([f], dict(CFG, exceptions=[ex]))
        errs, _, _ = bl.judge(kept, {"rules": {}}, CFG["hard_rules"])
        ok &= _check(f"ceiling: file.hard 45 vs max {cap} {'fails' if want else 'holds'}",
                     any("ceiling" in e["why"] for e in errs) == want)
    ok &= _check("ceiling: file.hard / fn.lines exception without max", all(bl.missing_ceilings(
        dict(CFG, exceptions=[{"rule": r, "item": "x", "id": "T"}])) for r in ("file.hard", "fn.lines")))
    g = rules.finding("fn.lines", "crates/qp-b/src/one_fn.rs::one", 31, 10, "fn one: 31 lines")
    for v, want in ((31, False), (32, True)):
        errs, _, _ = bl.judge([dict(g, value=v)], {"rules": {"fn.lines": {"count": 1, "items": {
            g["item"]: 31}}}}, CFG["hard_rules"])
        ok &= _check(f"ceiling: fn.lines {v} vs baseline 31 {'fails' if want else 'holds'}",
                     bool(errs) == want)
    return ok


def _config_diff():
    """K-9: a loosening must cite a decision; tightening and removals pass."""
    ok = True
    ex = {"id": "T", "rule": "file.hard", "item": "crates/qp-b/src/x.rs", "max": 50,
          "status": "istek", "reason": "tek fonksiyon"}
    base = {"rules": {"fn.lines": {"count": 1, "items": {"crates/qp-b/src/a.rs::f": 90}}}}

    def run(old_cfg, new_cfg, old_b=base, new_b=base):
        return configdiff.judge(old_cfg, new_cfg, old_b, new_b).fails

    ok &= _check("config-diff: new exception without a decision fails",
                 run(CFG, dict(CFG, exceptions=[ex])) == 1)
    ok &= _check("config-diff: new exception citing K93 passes",
                 run(CFG, dict(CFG, exceptions=[dict(ex, status="onaylandı (K93)")])) == 0)
    ok &= _check("config-diff: removing an exception passes", run(dict(CFG, exceptions=[ex]), CFG) == 0)
    ok &= _check("config-diff: raising a max without a new decision fails", run(
        dict(CFG, exceptions=[dict(ex, status="K85")]),
        dict(CFG, exceptions=[dict(ex, status="K85", max=60)])) == 1)
    loose = copy.deepcopy(CFG)
    loose["budgets"]["fn_lines"] = 12
    ok &= _check("config-diff: a raised budget without rule_changes fails", run(CFG, loose) == 1)
    ok &= _check("config-diff: new generated file without a decision fails",
                 run(CFG, dict(CFG, generated_files=["crates/qp-b/src/t.rs"])) == 1)
    swap = copy.deepcopy(CFG)
    swap["layers"][1]["order"] = ["qp-b", "qp-c"]
    ok &= _check("config-diff: swapping a layer order fails", run(CFG, swap) == 1)
    raised = {"rules": {"fn.lines": {"count": 1, "items": {"crates/qp-b/src/a.rs::f": 95}}}}
    moved = {"rules": {"fn.lines": {"count": 1, "items": {"crates/qp-d/src/a.rs::f": 90}}}}
    ok &= _check("config-diff: a raised baseline value fails", run(CFG, CFG, base, raised) == 1)
    ok &= _check("config-diff: a moved baseline item (same fn, not larger) passes",
                 run(CFG, CFG, base, moved) == 0)
    return ok
