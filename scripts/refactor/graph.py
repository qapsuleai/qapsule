#!/usr/bin/env python3
"""Dependency graph of one crate around one central type (RF1 plan measurement; K87).

  scripts/refactor/graph.py <crate-dir> [--type Session] [--json out.json] [--group N]

Measures, over the crate's non-test code (the arch-check lexer: comments, strings and
`#[cfg(test)]` regions are invisible):

  files     non-test lines; methods of `impl <Type>` blocks defined there
  fields    which of <Type>'s fields each file touches (`self.f`, `s.f`, `this.f`, …: any
            `.f` access where `f` is a field name of <Type> and no other struct of the crate
            has a field of that name; ambiguous names are counted separately)
  calls     file → file edges: a `.m(` call where `m` is a method of <Type> defined in
            another file (names that are also methods of other types are left out)
  modules   file → file edges: `crate::a::b` / `super::` paths and names imported from
            the crate root re-exports
  groups    files grouped by their first N path segments (`--group 2`: capsule/claims);
            the strongly connected components of the group graph (calls + modules)

Not a parser: it errs on the side of reporting an edge. Prints a text report; `--json`
writes the raw graph.
"""

import argparse
import collections
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from archcheck import rustscan as rs  # noqa: E402

IDENT = r"[A-Za-z_][A-Za-z0-9_]*"


def rel_files(src_dir):
    out = []
    for d, _, fs in os.walk(src_dir):
        for f in fs:
            if f.endswith(".rs"):
                p = os.path.join(d, f)
                rel = os.path.relpath(p, src_dir)
                if rel.endswith("tests.rs") or "/tests/" in rel:
                    continue
                out.append(rel)
    return sorted(out)


def blank_tests(code):
    regions = rs.test_regions(code)
    chars = list(code)
    for a, b in regions:
        for k in range(a, min(b + 1, len(chars))):
            if chars[k] != "\n":
                chars[k] = " "
    return "".join(chars)


def struct_fields(code, name):
    m = re.search(r"\bstruct\s+" + name + r"\b[^{;]*\{", code)
    if not m:
        return None
    end = rs.match_brace(code, m.end() - 1)
    body = code[m.end():end]
    # field names at depth 0 of the body
    fields, depth = [], 0
    for line in body.split("\n"):
        if depth == 0:
            fm = re.match(r"\s*(?:pub(?:\([^)]*\))?\s+)?(" + IDENT + r")\s*:(?!:)", line)
            if fm:
                fields.append(fm.group(1))
        depth += line.count("{") + line.count("(") + line.count("<") \
            - line.count("}") - line.count(")") - line.count(">")
        depth = max(depth, 0)
    return fields


def all_struct_fields(code):
    out = collections.defaultdict(set)
    for m in re.finditer(r"\bstruct\s+(" + IDENT + r")\b[^{;]*\{", code):
        fs = struct_fields(code[m.start():], m.group(1)) or []
        for f in fs:
            out[f].add(m.group(1))
    return out


def impl_blocks(code, typ):
    """[(start, end, type)] of every `impl … { }` block; type = the self type's last name."""
    out = []
    for m in re.finditer(r"(?m)^\s*impl\b", code):
        brace = code.find("{", m.end())
        semi = code.find(";", m.end())
        if brace < 0 or (0 <= semi < brace):
            continue
        head = code[m.end():brace]
        head = re.sub(r"<[^<>]*(<[^<>]*>[^<>]*)*>", "", head)  # generics
        tm = re.search(r"\bfor\s+(?:&(?:'\w+\s+)?(?:mut\s+)?)?(" + IDENT + r")", head) \
            or re.search(r"(" + IDENT + r")\s*$", head.strip())
        out.append((m.start(), rs.match_brace(code, brace), tm.group(1) if tm else "?"))
    return out


def methods_in(code, start, end):
    names = []
    seg = code[start:end]
    depth0 = None
    for m in re.finditer(r"\bfn\s+(" + IDENT + r")", seg):
        names.append(m.group(1))
    _ = depth0
    return names


def main():  # noqa: C901
    ap = argparse.ArgumentParser()
    ap.add_argument("crate")
    ap.add_argument("--type", default="Session")
    ap.add_argument("--json")
    ap.add_argument("--group", type=int, default=2)
    ap.add_argument("--map", help="JSON {target: [path prefixes]}: crate-level sizes, edges and cycles of a proposed split")
    a = ap.parse_args()
    src_dir = os.path.join(a.crate, "src")
    files = rel_files(src_dir)
    code = {}
    lines = {}  # counted as arch-check counts a crate: lines minus test lines; generated 0
    for f in files:
        raw = open(os.path.join(src_dir, f), encoding="utf-8").read()
        stripped = rs.strip(raw)
        code[f] = blank_tests(stripped)
        n = raw.count("\n") + (0 if raw.endswith("\n") or not raw else 1)
        lines[f] = 0 if rs.is_generated(raw) else n - rs.test_lines(stripped)

    # fields of the type and of every other struct
    tfields = None
    for f in files:
        tfields = struct_fields(code[f], a.type)
        if tfields:
            break
    tfields = tfields or []
    other = collections.defaultdict(set)
    for f in files:
        for fld, structs in all_struct_fields(code[f]).items():
            other[fld] |= {s for s in structs if s != a.type}
    unique_fields = [x for x in tfields if not other.get(x)]
    shared_fields = [x for x in tfields if other.get(x)]

    # methods of the type, and of other types
    tmethods = collections.defaultdict(set)  # name -> files
    omethods = set()
    for f in files:
        for s, e, t in impl_blocks(code[f], a.type):
            for n in methods_in(code[f], s, e):
                (tmethods[n].add(f) if t == a.type else omethods.add(n))
    callable_ = {n: fs for n, fs in tmethods.items() if n not in omethods and len(fs) == 1}

    def group(f):
        parts = f[:-3].split("/")
        return "/".join(parts[: a.group])

    field_use = collections.defaultdict(lambda: collections.Counter())
    for f in files:
        for fld in unique_fields:
            n = len(re.findall(r"\." + fld + r"\b(?!\s*\()", code[f]))
            if n:
                field_use[f][fld] = n
    calls = collections.Counter()
    for f in files:
        for m in re.finditer(r"\.(" + IDENT + r")\s*(?:::<[^>]*>)?\(", code[f]):
            n = m.group(1)
            if n in callable_:
                g = next(iter(callable_[n]))
                if g != f:
                    calls[(f, g)] += 1
    # module references: crate::x[::y] paths, mapped to files
    stems = {f[:-3].replace("/", "::"): f for f in files}
    lib = code.get("lib.rs", "")
    alias = {}
    for m in re.finditer(r"pub(?:\(crate\))?\s+use\s+(" + IDENT + r")::\{([^}]*)\}", lib):
        for n in re.split(r"\s*,\s*", m.group(2).strip()):
            if n:
                alias[n.split(" as ")[-1].strip()] = m.group(1) + "::" + n.split(" as ")[0].strip()
    for m in re.finditer(r"pub(?:\(crate\))?\s+use\s+(" + IDENT + r")::(" + IDENT + r");", lib):
        alias[m.group(2)] = m.group(1) + "::" + m.group(2)

    def resolve(path):
        segs = path.split("::")
        if segs and segs[0] in alias:
            segs = alias[segs[0]].split("::") + segs[1:]
        for k in range(len(segs), 0, -1):
            key = "::".join(segs[:k])
            if key in stems:
                return stems[key]
        return None

    mods = collections.Counter()
    for f in files:
        for m in re.finditer(r"\bcrate::((?:" + IDENT + r"::)*" + IDENT + r")", code[f]):
            g = resolve(m.group(1))
            if g and g != f:
                mods[(f, g)] += 1
    # group graph and SCCs
    gedges = collections.defaultdict(set)
    for (x, y) in list(calls) + list(mods):
        gx, gy = group(x), group(y)
        if gx != gy:
            gedges[gx].add(gy)
    groups = sorted({group(f) for f in files})
    glines = collections.Counter()
    for f in files:
        glines[group(f)] += lines[f]
    sccs = tarjan(groups, gedges)

    out = []
    out.append(f"# graph {a.crate} type {a.type}: {len(files)} files, {sum(lines.values())} lines (arch-check count)")
    out.append(f"fields of {a.type}: {len(tfields)} ({len(shared_fields)} also named in other structs, not counted: {', '.join(shared_fields)})")
    out.append(f"methods of {a.type}: {len(tmethods)} names ({len(callable_)} counted as call edges)")
    out.append("\n## groups (lines, methods, fields touched, out-edges)")
    for g in groups:
        gf = [f for f in files if group(f) == g]
        meth = sum(1 for n, fs in tmethods.items() if fs & set(gf))
        fl = sorted({fld for f in gf for fld in field_use[f]})
        out.append(f"{g:28s} {glines[g]:6d}  m={meth:3d}  fields={len(fl):2d}  -> {', '.join(sorted(gedges[g]))}")
    out.append("\n## strongly connected components (groups)")
    for c in sorted(sccs, key=lambda c: -sum(glines[g] for g in c)):
        if len(c) > 1:
            out.append(f"{sum(glines[g] for g in c):6d}  {' '.join(sorted(c))}")
    singles = [c[0] for c in sccs if len(c) == 1]
    out.append(f"outside: {sum(glines[g] for g in singles)} lines in {len(singles)} groups: {' '.join(sorted(singles))}")
    out.append("\n## field -> groups touching it (uses)")
    for fld in unique_fields:
        users = collections.Counter()
        for f in files:
            if field_use[f][fld]:
                users[group(f)] += field_use[f][fld]
        out.append(f"{fld:22s} {len(users):2d}  " + " ".join(f"{g}:{n}" for g, n in users.most_common()))
    if a.map:
        out.append(map_report(json.load(open(a.map)), files, lines, calls, mods))
    print("\n".join(out))
    if a.json:
        json.dump({
            "files": {f: {"lines": lines[f], "group": group(f),
                          "fields": dict(field_use[f]),
                          "methods": sorted(n for n, fs in tmethods.items() if f in fs)} for f in files},
            "calls": [[x, y, n] for (x, y), n in sorted(calls.items())],
            "modules": [[x, y, n] for (x, y), n in sorted(mods.items())],
            "fields": tfields, "shared_fields": shared_fields,
        }, open(a.json, "w"), indent=1, ensure_ascii=False)


def map_report(plan, files, lines, calls, mods):
    """Sizes, edges and strongly connected components of `files` grouped by `plan`
    (first matching prefix wins; a prefix matches a file path or its start)."""
    def target(f):
        for k, pre in plan.items():
            if any(f == p or f.startswith(p) for p in pre):
                return k
        return "?"
    size = collections.Counter()
    for f in files:
        size[target(f)] += lines[f]
    edges = collections.defaultdict(collections.Counter)
    for (x, y), n in list(calls.items()) + list(mods.items()):
        tx, ty = target(x), target(y)
        if tx != ty:
            edges[tx][ty] += n
    out = ["\n## proposed split (--map)"]
    for k in sorted(size, key=lambda k: -size[k]):
        out.append(f"{k:18s} {size[k]:6d}  -> " + ", ".join(f"{t}:{n}" for t, n in edges[k].most_common()))
    cyc = [sorted(c) for c in tarjan(list(size), {k: set(v) for k, v in edges.items()}) if len(c) > 1]
    out.append("cycles: " + ("; ".join(" ".join(c) for c in cyc) if cyc else "none"))
    unmapped = [f for f in files if target(f) == "?"]
    if unmapped:
        out.append("unmapped: " + " ".join(unmapped))
    return "\n".join(out)


def tarjan(nodes, edges):
    index, low, on, stack, out = {}, {}, set(), [], []
    counter = [0]

    def strong(v):
        index[v] = low[v] = counter[0]
        counter[0] += 1
        stack.append(v)
        on.add(v)
        for w in edges.get(v, ()):
            if w not in index:
                strong(w)
                low[v] = min(low[v], low[w])
            elif w in on:
                low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            comp = []
            while True:
                w = stack.pop()
                on.discard(w)
                comp.append(w)
                if w == v:
                    break
            out.append(comp)

    sys.setrecursionlimit(10000)
    for v in nodes:
        if v not in index:
            strong(v)
    return out


if __name__ == "__main__":
    main()
