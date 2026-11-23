#!/usr/bin/env python3
"""RG1 move check: are the Rust items of a change only moved?

  scripts/refactor/moved.py <base> [<head>] [--list] [-- <path prefixes>...]

Item fingerprints (the decisive check). Every item — `fn` (with its body), `struct`,
`enum`, `union`, `trait`, `type`, `const`, `static`, `macro_rules!` — is cut from every
changed `.rs` file at <base> and at <head> (default: the working tree), together with the
attributes and doc comments right above it (so `#[serde(…)]`, `#[arg(…)]`, `///`
schemars/clap texts count). Text is normalised only by trimming each line and dropping a
leading visibility (`pub`, `pub(crate)`, `pub(super)`, `pub(in …)`) on any line. The two
multisets must be equal. An item that differs only in paths is reported as PATH when,
after rewriting `a::b::` prefixes, the texts are equal *and* every path keeps its last
segment (`qp_render::synthetic_bars(` -> `qp_source::synthetic_series(` is CHANGED) —
module renames `common` -> `support` count as paths. Everything else is CHANGED (exit 1).

`impl` and `mod` blocks are containers: their headers may be split or moved freely; the
items inside are checked one by one. Allowed in addition (not items): `mod`/`use` lines,
`#[allow(clippy::…)]` added above an item (listed as ALLOW), new `//!` module docs.
"""

import collections
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))  # scripts/ (archcheck)
from archcheck import rustscan as rs  # noqa: E402

VIS = re.compile(r"^pub(\((crate|super|self|in [^)]*)\))?\s+")
VIS_INLINE = re.compile(r"(?<![A-Za-z0-9_])pub(\((crate|super|self|in [^)]*)\))?\s*(?=[A-Za-z_])")
ITEM = re.compile(
    r"(?<!')\b(fn|struct|enum|union|trait|type|const|static|macro_rules!)(?:\s+|(?<=!)\s*)([A-Za-z_][A-Za-z0-9_]*)?")
PATH = re.compile(r"\b((?:crate|super|self|[a-z_][a-z0-9_]*)::)+(?=[A-Za-z_{*])")
ALLOW_IN = re.compile(r"#\[allow\(clippy::[a-z_]+(,\s*clippy::[a-z_]+)*\)\]\s*")


def git(*args):
    return subprocess.run(["git", *args], capture_output=True, text=True, check=True).stdout


def show(rev, path):
    if rev is None:
        try:
            with open(path, encoding="utf-8") as fh:
                return fh.read()
        except FileNotFoundError:
            return ""
    r = subprocess.run(["git", "show", f"{rev}:{path}"], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else ""


def norm(text):
    """Trimmed lines without leading visibility; then rustfmt's re-wrapping is undone:
    whitespace collapsed, no space next to brackets, no trailing comma before a closer."""
    out = []
    for line in text.split("\n"):
        s = VIS.sub("", line.strip())
        if s:
            out.append(s)
    t = " ".join(out)
    t = re.sub(r"\s+", " ", t)
    t = re.sub(r"\s*([()\[\]{}<>,;])\s*", r"\1", t)
    t = re.sub(r",([)\]}>])", r"\1", t)
    return VIS_INLINE.sub("", t)


def items(src):
    """[(kind, name, normalised text)] of every item; fn bodies are part of the fn (items
    nested in a fn are not reported separately)."""
    code = rs.strip(src)
    depth_ok = _not_in_fn_body(code)
    out = []
    for m in ITEM.finditer(code):
        kind = m.group(1)
        pos = m.start(1)
        if not depth_ok[pos]:
            continue
        prev = code[:pos].rstrip()[-1:]
        if kind == "const" and prev in ("<", ","):
            continue  # a const generic parameter (`fn f<const N: usize>`), not an item
        if kind == "const" and code[m.end(1):m.end(1) + 4].lstrip().startswith("fn"):
            continue  # `const fn`: the fn match reports it
        if kind == "type" and not m.group(2):
            continue
        end = _item_end(code, m.end())
        if end is None:
            continue
        start = _attr_start(src, code, _line_start(code, pos))
        out.append((kind, m.group(2) or "", norm(src[start:end + 1])))
    return out


def _line_start(code, pos):
    k = code.rfind("\n", 0, pos)
    return k + 1


def _attr_start(src, code, ls):
    """Start of the attribute / doc-comment block directly above the line at `ls`."""
    start = ls
    while start > 0:
        prev = src.rfind("\n", 0, start - 1) + 1
        line = src[prev:start - 1].strip()
        if line.startswith(("#[", "///", "//!")) and not line.startswith("#!["):
            start = prev
        else:
            break
    return start


def _item_end(code, k):
    """Index of the item's last character: the matching `}` of its body or the `;`."""
    depth = 0
    n = len(code)
    while k < n:
        ch = code[k]
        if ch in "([":
            depth += 1
        elif ch in ")]":
            depth -= 1
        elif ch == ";" and depth <= 0:
            return k
        elif ch == "{" and depth <= 0:
            return rs.match_brace(code, k)
        k += 1
    return None


def _not_in_fn_body(code):
    """ok[pos]: position is not inside a fn body."""
    ok = [True] * (len(code) + 1)
    for m in re.finditer(r"\bfn\s+[A-Za-z_]", code):
        end = _item_end(code, m.end())
        if end is None:
            continue
        body = code.find("{", m.end(), end + 1)
        if body < 0:
            continue
        for i in range(body + 1, end):
            ok[i] = False
    return ok


def lead_strip(text):
    """The item without the `#[allow(clippy::…)]` attributes at its very start (RG1/L puts
    the grandfathering attribute above the item's own attributes and docs)."""
    while True:
        m = ALLOW_IN.match(text)
        if not m:
            return text
        text = text[m.end():]


def unpath(text):
    return PATH.sub("", text)


def tails(text):
    """Every path's last segment in order (`a::b::c(` -> `c`)."""
    return re.findall(r"(?:[A-Za-z_][A-Za-z0-9_]*::)+([A-Za-z_][A-Za-z0-9_]*)", text)


def main(argv):
    prefixes = []
    if "--" in argv:
        i = argv.index("--")
        argv, prefixes = argv[:i], argv[i + 1:]
    listing = "--list" in argv
    argv = [a for a in argv if a != "--list"]
    if not argv:
        print(__doc__)
        return 2
    base = argv[0]
    head = argv[1] if len(argv) > 1 else None
    rng = [base] + ([head] if head else [])
    names = git("diff", "--name-status", "-M", *rng, "--", "*.rs").splitlines()
    if head is None:  # working tree: untracked new files too
        names += [f"A\t{p}" for p in git("ls-files", "--others", "--exclude-standard", "--", "*.rs").split()]
    before, after = collections.Counter(), collections.Counter()
    where_a, where_b = {}, {}
    for line in names:
        parts = line.split("\t")
        st, paths = parts[0], parts[1:]
        old, new = (paths[0], paths[-1])
        if prefixes and not any(p.startswith(tuple(prefixes)) for p in (old, new)):
            continue
        if st[0] != "A":
            for it in items(show(base, old)):
                before[it[2]] += 1
                where_a.setdefault(it[2], (old, it[0], it[1]))
        if st[0] != "D":
            for it in items(show(head, new)):
                after[it[2]] += 1
                where_b.setdefault(it[2], (new, it[0], it[1]))
    moved = before & after
    gone, added = before - moved, after - moved
    pathonly, changed, allows = [], [], []
    gone_by_unpath = collections.defaultdict(list)
    gone_by_lead = collections.defaultdict(list)
    for t, n in gone.items():
        gone_by_unpath[unpath(t)] += [t] * n
        gone_by_lead[lead_strip(t)] += [t] * n
    for t, n in sorted(added.items()):
        for _ in range(n):
            # an item that only gained `#[allow(clippy::…)]` lines
            key = lead_strip(t)
            hit = next((g for g in gone_by_lead.get(key, []) if gone[g] > 0), None)
            if hit is not None and hit != t:
                gone[hit] -= 1
                gone_by_lead[key].remove(hit)
                gone_by_unpath[unpath(hit)].remove(hit)
                allows.append(where_b[t])
                continue
            cands = gone_by_unpath.get(unpath(t), [])
            hit = next((c for c in cands if tails(c) == tails(t)), None)
            if hit is not None:
                cands.remove(hit)
                gone[hit] -= 1
                pathonly.append((where_a.get(hit), where_b[t]))
            else:
                changed.append(("+", where_b[t], t))
    for t, n in sorted(gone.items()):
        for _ in range(max(n, 0)):
            changed.append(("-", where_a[t], t))
    total = sum(moved.values())
    print(f"moved: {total} items unchanged (moved or in place); {len(pathonly)} path-only; "
          f"{len(allows)} with an added #[allow]; {len(changed)} CHANGED")
    for a, b in pathonly:
        print(f"PATH    {b[0]} {b[1]} {b[2]}" + (f"  (was {a[0]})" if a and a[0] != b[0] else ""))
    for b in allows:
        print(f"ALLOW   {b[0]} {b[1]} {b[2]}")
    for sign, w, t in changed:
        print(f"CHANGED {sign} {w[0]} {w[1]} {w[2]}")
        if listing:
            print("        | " + t.replace("\n", "\n        | ")[:2000])
    return 1 if changed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
