"""arch-check rules (docs/ENGINEERING.md §2–§4). Each rule yields findings
`(rule, item, value, limit, message)`; `item` is stable (a path, `crate`, `a -> b`), so the
baseline can name it."""

import fnmatch
import glob
import json
import os
import subprocess

from . import rustscan as rs

RUST_DIRS = ("src", "tests", "examples", "benches")


def finding(rule, item, value, limit, message):
    return {"rule": rule, "item": item, "value": value, "limit": limit, "message": message}


def is_test_path(rel):
    """Test code: crate `tests/` trees and `tests.rs` / `tests/` modules inside `src/`."""
    parts = rel.split("/")
    return "tests" in parts[2:] or parts[-1] == "tests.rs"


def rust_files(root, crate_dir):
    out = []
    for sub in RUST_DIRS:
        out += glob.glob(os.path.join(root, crate_dir, sub, "**", "*.rs"), recursive=True)
    return sorted(out)


def crates(root):
    return sorted(d for d in os.listdir(os.path.join(root, "crates"))
                  if os.path.isfile(os.path.join(root, "crates", d, "Cargo.toml")))


class Scan:
    """Reads every Rust file of every crate once."""

    def __init__(self, root):
        self.root = root
        self.files = {}  # rel -> (src, code)
        for c in crates(root):
            for f in rust_files(root, os.path.join("crates", c)):
                rel = os.path.relpath(f, root)
                with open(f, encoding="utf-8") as fh:
                    src = fh.read()
                self.files[rel] = (src, rs.strip(src))


def rule_sizes(scan, cfg):
    b = cfg["budgets"]
    crate_lines = {}
    for rel, (src, code) in sorted(scan.files.items()):
        parts = rel.split("/")
        crate, area = parts[1], parts[2]
        lines = src.count("\n") + (0 if src.endswith("\n") or not src else 1)
        generated = rs.is_generated(src)
        test = is_test_path(rel) or area in ("tests",)
        if area == "src" and not generated:
            if lines > b["file_hard"]:
                yield finding("file.hard", rel, lines, b["file_hard"],
                              f"{rel}: {lines} lines > hard limit {b['file_hard']}")
            elif not test and lines > b["file_soft"]:
                yield finding("file.soft", rel, lines, b["file_soft"],
                              f"{rel}: {lines} lines > budget {b['file_soft']}")
        if test and area in ("src", "tests") and lines > b["test_file"] and not generated:
            if area == "tests":  # src test files are under file.hard already
                yield finding("test.file", rel, lines, b["test_file"],
                              f"{rel}: test file {lines} lines > {b['test_file']}")
        if area == "src" and not test and not generated:
            crate_lines[crate] = crate_lines.get(crate, 0) + lines - rs.test_lines(code)
        if area == "src" and not test:
            for name, a, z, is_test in rs.functions(code):
                n = z - a + 1
                if not is_test and n > b["fn_lines"]:
                    yield finding("fn.lines", f"{rel}::{name}@{a}", n, b["fn_lines"],
                                  f"{rel}:{a} fn {name}: {n} lines > {b['fn_lines']}")
    shells = set(layer_of_map(cfg).get("__shells__", []))
    for crate, n in sorted(crate_lines.items()):
        if crate in shells:
            if n > b["shell_crate_lines"]:
                yield finding("shell.lines", crate, n, b["shell_crate_lines"],
                              f"{crate}: shell crate {n} non-test lines > {b['shell_crate_lines']}")
        elif n > b["crate_lines"]:
            yield finding("crate.lines", crate, n, b["crate_lines"],
                          f"{crate}: {n} non-test lines > {b['crate_lines']}")


def crate_line_table(scan):
    """{crate: (non-test src lines, test lines)} for reports."""
    t = {}
    for rel, (src, code) in scan.files.items():
        parts = rel.split("/")
        crate, area = parts[1], parts[2]
        lines = src.count("\n")
        a, z = t.get(crate, (0, 0))
        if area == "src" and not is_test_path(rel) and not rs.is_generated(src):
            tl = rs.test_lines(code)
            t[crate] = (a + lines - tl, z + tl)
        elif area in ("src", "tests"):
            t[crate] = (a, z + lines)
    return t


def rule_dirs(scan, cfg):
    limit = cfg["budgets"]["dir_files"]
    counts = {}
    for rel in scan.files:
        d = os.path.dirname(rel)
        counts[d] = counts.get(d, 0) + 1
    for d, n in sorted(counts.items()):
        if n > limit:
            yield finding("dir.files", d, n, limit, f"{d}: {n} source files > {limit}")


def rule_names(scan, cfg, root):
    bad = set(cfg["forbidden_names"])
    seen = set()
    for rel in scan.files:
        parts = rel.split("/")
        for i, p in enumerate(parts[2:], start=2):
            stem = p[:-3] if p.endswith(".rs") else p
            if stem in bad:
                item = "/".join(parts[: i + 1])
                if item not in seen:
                    seen.add(item)
                    yield finding("name.forbidden", item, 1, 0,
                                  f"{item}: forbidden name `{stem}` (util/common/misc/helpers)")


def rule_readmes(scan, cfg, root):
    for c in crates(root):
        if not os.path.isfile(os.path.join(root, "crates", c, "README.md")):
            yield finding("readme.crate", f"crates/{c}", 1, 0, f"crates/{c}: no README.md")
    dirs = set()
    for rel in scan.files:
        parts = rel.split("/")
        if parts[2] == "src" and len(parts) > 4:
            dirs.add("/".join(parts[:-1]))
    for d in sorted(dirs):
        if not os.path.isfile(os.path.join(root, d, "README.md")):
            yield finding("readme.module", d, 1, 0, f"{d}: module folder without README.md")


def rule_root_logic(scan, cfg):
    for rel, (src, code) in sorted(scan.files.items()):
        parts = rel.split("/")
        if parts[2] != "src" or parts[-1] not in ("lib.rs", "mod.rs"):
            continue
        items = rs.top_level_items(code)
        if items:
            kinds = ", ".join(sorted({k for k, _ in items}))
            yield finding("root.logic", rel, len(items), 0,
                          f"{rel}: {len(items)} logic items ({kinds}) in a lib.rs/mod.rs "
                          f"(declarations and re-exports only; first at line {items[0][1]})")


def rule_docs(cfg, root):
    b = cfg["budgets"]
    seen = set()
    for g in cfg["doc_globs"]:
        for f in glob.glob(os.path.join(root, g), recursive=True):
            rel = os.path.relpath(f, root)
            if rel in seen or "node_modules" in rel or rel.startswith("target/"):
                continue
            seen.add(rel)
            with open(f, "rb") as fh:
                data = fh.read()
            lines = data.count(b"\n")
            if lines > b["doc_lines"] or len(data) > b["doc_bytes"]:
                yield finding("doc.size", rel, max(lines, 0), b["doc_lines"],
                              f"{rel}: {lines} lines, {len(data)} bytes > "
                              f"{b['doc_lines']} lines / {b['doc_bytes']} bytes")


def layer_of_map(cfg):
    m = {}
    for i, layer in enumerate(cfg["layers"]):
        for c in layer["crates"]:
            m[c] = (i, layer["name"])
    for c, o in cfg.get("layer_overrides", {}).items():
        idx = [l["name"] for l in cfg["layers"]].index(o["layer"])
        m[c] = (idx, o["layer"])
    shell_idx = [l["name"] for l in cfg["layers"]].index(cfg["shell_layer"])
    m["__shells__"] = [c for c, v in m.items() if isinstance(v, tuple) and v[0] == shell_idx]
    return m


def cargo_edges(root):
    """[(from, to, kind)] between workspace crates from `cargo metadata`."""
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline"],
        cwd=root, capture_output=True, text=True, check=True).stdout
    meta = json.loads(out)
    names = {p["name"] for p in meta["packages"]}
    edges = []
    for p in meta["packages"]:
        for d in p["dependencies"]:
            if d["name"] in names and d["name"] != p["name"]:
                edges.append((p["name"], d["name"], d["kind"] or "normal"))
    return sorted(set(edges)), sorted(names)


def rule_layers(cfg, edges, names):
    m = layer_of_map(cfg)
    for n in names:
        if n not in m:
            yield finding("layer.unknown", n, 1, 0,
                          f"{n}: crate has no layer in scripts/arch-config.json (§2)")
    for a, b, kind in edges:
        if kind == "dev" or a not in m or b not in m:
            continue
        (ia, la), (ib, lb) = m[a], m[b]
        if ib > ia:
            yield finding("layer.up", f"{a} -> {b}", 1, 0,
                          f"{a} ({la}) depends on {b} ({lb}): upward dependency")
        elif ib == ia:
            yield finding("layer.lateral", f"{a} -> {b}", 1, 0,
                          f"{a} -> {b}: sideways dependency inside layer `{la}`")


def all_findings(root, cfg, edges=None, names=None):
    scan = Scan(root)
    if edges is None:
        edges, names = cargo_edges(root)
    out = []
    out += list(rule_layers(cfg, edges, names))
    out += list(rule_sizes(scan, cfg))
    out += list(rule_dirs(scan, cfg))
    out += list(rule_names(scan, cfg, root))
    out += list(rule_readmes(scan, cfg, root))
    out += list(rule_root_logic(scan, cfg))
    out += list(rule_docs(cfg, root))
    return out, scan


def match_any(item, patterns):
    return any(fnmatch.fnmatch(item, p) for p in patterns)
