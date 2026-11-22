#!/usr/bin/env python3
"""RG1 move check: is a diff only moves plus `mod` / `use` / visibility / re-export lines?

  scripts/rg1-moved.py <base> [<head>] [-- <paths>...]

Reads `git diff <base> <head>` (default head: the working tree) for Rust files (and
Cargo.toml), and compares the multiset of removed lines with the multiset of added lines
after normalisation (whitespace trimmed; leading visibility `pub`, `pub(crate)`,
`pub(super)`, `pub(in …)` dropped). A line that was removed and added again is a move.
What is left on either side must be a structural line:

  mod / pub mod / use / pub use / extern crate, `impl … {` / `}` / `{` openers and closers,
  attributes (`#[…]`, `#![…]`), comments and doc comments, empty lines, Cargo.toml lines.

Everything else is printed as "CHANGED" (a body edit, or a path edit such as
`common::x` -> `support::x`) and the exit code is 1. Path-only edits are reported
separately as "PATH" when the line equals a removed line after rewriting paths
(`crate::a::b::`, `super::`, `self::`, crate names); they are allowed but listed.
"""

import collections
import re
import subprocess
import sys

VIS = re.compile(r"^pub(\((crate|super|self|in [^)]*)\))?\s+")
STRUCT = [
    re.compile(r"^(pub(\([^)]*\))?\s+)?mod\s+\w+\s*(;|\{)$"),
    re.compile(r"^(pub(\([^)]*\))?\s+)?use\s"),
    re.compile(r"^(pub(\([^)]*\))?\s+)?extern\s+crate\s"),
    re.compile(r"^[{}()\[\];,]*$"),
    re.compile(r"^#!?\["),
    re.compile(r"^//"),
    re.compile(r"^/?\*"),
    re.compile(r"^(unsafe\s+)?impl\b.*\{$"),
    # names inside a wrapped `use {…}` list (not a keyword or a value)
    re.compile(r"^(?!(return|break|continue|None|Some|Ok|Err|true|false|self|Self)\b)"
               r"[A-Za-z_][A-Za-z0-9_:, {}*]*[,{}]$"),
    re.compile(r"^\};?$"),
]
PATH = re.compile(r"\b(crate|super|self|qp_[a-z_]+)(::[a-z_][a-z0-9_]*)*::")


def norm(line):
    s = line.strip()
    s = VIS.sub("", s)
    return s


def structural(s):
    return any(p.match(s) for p in STRUCT)


def unpath(s):
    return PATH.sub("", s)


def main(argv):
    paths = []
    if "--" in argv:
        i = argv.index("--")
        argv, paths = argv[:i], argv[i + 1:]
    if not argv:
        print(__doc__)
        return 2
    rng = argv[:2]
    cmd = ["git", "diff", "-M", "--unified=0", "--no-color", *rng, "--", *(paths or ["*.rs", "**/Cargo.toml"])]
    out = subprocess.run(cmd, capture_output=True, text=True, check=True).stdout
    removed, added = collections.Counter(), collections.Counter()
    where = {}
    cur = None
    for line in out.splitlines():
        if line.startswith("+++ "):
            cur = line[6:] if line.startswith("+++ b/") else line[4:]
            continue
        if line.startswith("--- "):
            continue
        if line.startswith("-") and not line.startswith("---"):
            s = norm(line[1:])
            if s:
                removed[s] += 1
        elif line.startswith("+") and not line.startswith("+++"):
            s = norm(line[1:])
            if s:
                added[s] += 1
                where.setdefault(s, cur)
    moved = removed & added
    rem_left = removed - moved
    add_left = added - moved
    changed, pathonly = [], []
    rem_unpath = collections.Counter({unpath(k): v for k, v in rem_left.items() if not structural(k)})
    for s, n in sorted(add_left.items()):
        if structural(s) or (where.get(s) or "").endswith("Cargo.toml"):
            continue
        if rem_unpath.get(unpath(s), 0) > 0:
            rem_unpath[unpath(s)] -= 1
            pathonly.append((where.get(s), s))
        else:
            changed.append(("+", where.get(s), s, n))
    for s, n in sorted(rem_left.items()):
        if structural(s):
            continue
        if rem_unpath.get(unpath(s), 0) <= 0 and any(unpath(s) == unpath(p) for _, p in pathonly):
            continue
        if not any(unpath(s) == unpath(p) for _, p in pathonly):
            changed.append(("-", "", s, n))
    total = sum(moved.values())
    print(f"rg1-moved: {total} lines moved; {len(pathonly)} path-only edits; {len(changed)} changed")
    for f, s in pathonly:
        print(f"PATH    {f}: {s}")
    for sign, f, s, n in changed:
        print(f"CHANGED {sign} {f or ''}: {s}" + (f"  (x{n})" if n > 1 else ""))
    return 1 if changed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
