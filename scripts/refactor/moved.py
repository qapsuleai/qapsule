#!/usr/bin/env python3
"""Move check (RG1, kept for RF1; K87): are the Rust items of a change only moved?

  scripts/refactor/moved.py <base> [<head>] [--list] [--strict-use] [-- <path prefixes>...]
  scripts/refactor/moved.py --self-test

Every changed `.rs` file is read at <base> and at <head> (default: the working tree) and
fingerprinted (rustitems.py): items — `fn` (with its body), `struct`, `enum`, `union`,
`trait`, `type`, `const`, `static`, `macro_rules!` and item-level macro invocations
(`foo! { … }`, `foo!(…);`) — each with the attributes and doc comments right above it, so
`#[cfg(…)]`, `#[serde(…)]`, `///` texts count. Text is normalised only by trimming lines,
undoing rustfmt's wrapping and dropping visibility (`pub`, `pub(crate)`, …). The two
multisets must be equal. An item that differs only in paths is PATH when, after rewriting
`a::b::` prefixes, the texts are equal *and* every path keeps its last segment
(`qp_render::synthetic_bars(` -> `qp_source::synthetic_series(` is CHANGED). An item that
only gained `#[allow(clippy::…)]` lines is ALLOW. Everything else is CHANGED (exit 1).

In addition (K-8; RG1 denetim B8):
- `impl` headers: an impl may be split, merged or moved, but the set of headers (with their
  attributes, generics, bounds, trait) must be equal up to paths; a new or lost header is
  CHANGED (`impl<T: A>` -> `impl<T: B>`, a trait changed, a `#[cfg]` dropped).
- Methods are bound to their impl: an item inside an `impl` block is fingerprinted with
  the block's header, so a method moved to an impl with other bounds or another `#[cfg]`
  is CHANGED even when both headers still exist.
- `mod` declarations with a `#[cfg(…)]` (other than a bare `#[cfg(test)]`): a changed or
  dropped cfg is CHANGED; a `mod x;` that lost its `#[cfg(test)]`, or a new plain
  `mod tests;`, is CHANGED. Inner `#![cfg(…)]` / `#![cfg_attr(…)]` must match across the
  change (CHANGED); other inner attributes (`#![allow]`, …) that do not match are FLAG.
- A primitive type is not a path: `i32::MAX` -> `i64::MAX`, `f32::EPSILON` ->
  `f64::EPSILON` are CHANGED, not PATH.
- `use`: plain imports are reported per file as PATH-ONLY counts (they follow moved items);
  `pub use` re-exports are listed as REEXPORT (public paths may change; list them); glob
  (`use a::*`), anonymous (`as _`), `#[cfg]`-gated and `extern crate` / `#[macro_use]`
  imports are matched across the whole change, and an unmatched one is a FLAG; so is a
  name imported from a different module (`exact::round` -> `fast::round`; a crate or
  folder rename that keeps the module before the name is not).
  FLAG lines do not fail by default (a split adds `use super::*;` routinely); the reviewer
  must read every one: a `use` CAN change behaviour unseen — a redirected import compiles
  when the other item has the same signature, and a glob can shadow a prelude name.
  `--strict-use` makes FLAG fail.
"""

import collections
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))  # scripts/ (archcheck)
sys.path.insert(0, HERE)
from rustitems import File  # noqa: E402

# A primitive type is not a path prefix: `i32::MAX` -> `i64::MAX` changes meaning (D2).
PRIM = r"(?:[iu](?:8|16|32|64|128|size)|f32|f64|bool|char|str)::"
PATH = re.compile(r"\b((?!" + PRIM + r")(?:crate|super|self|[a-z_][a-z0-9_]*)::)+(?=[A-Za-z_{*])")
ALLOW_IN = re.compile(r"#\[allow\(clippy::[a-z_]+(,\s*clippy::[a-z_]+)*\)\]\s*")
PATH_KINDS = ("plain", "reexport")


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


class Result:
    def __init__(self, strict_use=False):
        self.strict_use = strict_use
        self.moved = 0
        self.pathonly, self.allows, self.changed, self.flags = [], [], [], []
        self.uses, self.reexports = [], []

    @property
    def failed(self):
        return bool(self.changed or (self.strict_use and self.flags))


def compare(pairs, strict_use=False):
    """`pairs`: [(old path | None, old src, new path | None, new src)] -> Result."""
    res = Result(strict_use)
    before, after = collections.Counter(), collections.Counter()
    where_a, where_b = {}, {}
    hdr_a, hdr_b, mods_a, mods_b = {}, {}, collections.Counter(), collections.Counter()
    use_a, use_b = [], []
    inner = {False: collections.Counter(), True: collections.Counter()}
    for old, osrc, new, nsrc in pairs:
        fa = File(osrc) if old else None
        fb = File(nsrc) if new else None
        for f, path, cnt, where, hdr, mods, uses in ((fa, old, before, where_a, hdr_a, mods_a, use_a),
                                                     (fb, new, after, where_b, hdr_b, mods_b, use_b)):
            if f is None:
                continue
            for kind, name, text in f.items:
                cnt[text] += 1
                where.setdefault(text, (path, kind, name))
            for h in f.impls:
                hdr.setdefault(h, path)
            for name, cfgs, text in f.mods:
                if cfgs and cfgs != ["#[cfg(test)]"]:
                    mods[text] += 1
                    where.setdefault(text, (path, "mod", name))
            for text in f.inner:
                key = ("#![cfg]" if text.startswith(("#![cfg(", "#![cfg_attr(")) else "#![]", text)
                inner[cnt is after][key] += 1
                where.setdefault(text, (path, key[0], text))
            uses += [(path, *u) for u in f.uses]
        _file_uses(res, old, new, fa, fb)
        _test_mods(res, new or old, fa, fb)
    _items(res, before, after, where_a, where_b)
    _headers(res, hdr_a, hdr_b)
    for t, n in (mods_a - mods_b).items():
        res.changed += [("-", where_a[t], t)] * n
    for t, n in (mods_b - mods_a).items():
        res.changed += [("+", where_b[t], t)] * n
    for sign, x, y in (("-", inner[False], inner[True]), ("+", inner[True], inner[False])):
        for (kind, t), n in (x - y).items():
            w = where_a.get(t) or where_b.get(t)
            if kind == "#![cfg]":
                res.changed += [(sign, w, t)] * n
            else:
                res.flags += [(sign, w[0], "inner attribute", t)] * n
    _global_uses(res, use_a, use_b)
    return res


def _test_mods(res, path, fa, fb):
    """D3: a `mod x;` that was `#[cfg(test)]` (in the same file) or a new plain `mod tests`
    brings test code into the product."""
    tests_a = {n for n, c, _ in (fa.mods if fa else []) if "#[cfg(test)]" in c}
    plain_a = {n for n, c, _ in (fa.mods if fa else []) if not c}
    for n, c, t in (fb.mods if fb else []):
        if not c and (n in tests_a or (n == "tests" and n not in plain_a)):
            res.changed.append(("+", (path, "mod", n), t + "  (without #[cfg(test)])"))


def _items(res, before, after, where_a, where_b):
    moved = before & after
    gone, added = before - moved, after - moved
    res.moved = sum(moved.values())
    gone_by_unpath = collections.defaultdict(list)
    gone_by_lead = collections.defaultdict(list)
    for t, n in gone.items():
        gone_by_unpath[unpath(t)] += [t] * n
        gone_by_lead[lead_strip(t)] += [t] * n
    for t, n in sorted(added.items()):
        for _ in range(n):
            key = lead_strip(t)  # an item that only gained `#[allow(clippy::…)]` lines
            hit = next((g for g in gone_by_lead.get(key, []) if gone[g] > 0), None)
            if hit is not None and hit != t:
                gone[hit] -= 1
                gone_by_lead[key].remove(hit)
                gone_by_unpath[unpath(hit)].remove(hit)
                res.allows.append(where_b[t])
                continue
            cands = gone_by_unpath.get(unpath(t), [])
            hit = next((c for c in cands if tails(c) == tails(t)), None)
            if hit is not None:
                cands.remove(hit)
                gone[hit] -= 1
                res.pathonly.append((where_a.get(hit), where_b[t]))
            else:
                res.changed.append(("+", where_b[t], t))
    for t, n in sorted(gone.items()):
        res.changed += [("-", where_a[t], t)] * max(n, 0)


def _headers(res, hdr_a, hdr_b):
    lost = {h for h in hdr_a if h not in hdr_b}
    for h in sorted(h for h in hdr_b if h not in hdr_a):
        hit = next((g for g in sorted(lost) if unpath(g) == unpath(h) and tails(g) == tails(h)), None)
        if hit is not None:
            lost.discard(hit)
            res.pathonly.append(((hdr_a[hit], "impl", hit), (hdr_b[h], "impl", h)))
        else:
            res.changed.append(("+", (hdr_b[h], "impl", h), h))
    for h in sorted(lost):
        res.changed.append(("-", (hdr_a[h], "impl", h), h))


def _file_uses(res, old, new, fa, fb):
    a = collections.Counter(u[2] for u in (fa.uses if fa else []) if u[0] == "plain")
    b = collections.Counter(u[2] for u in (fb.uses if fb else []) if u[0] == "plain")
    plus, minus = sorted((b - a).elements()), sorted((a - b).elements())
    if plus or minus:
        res.uses.append((new or old, plus, minus))


def _global_uses(res, use_a, use_b):
    """Re-exports are listed; non-path kinds must balance across the change."""
    a = collections.Counter((u[1], u[2]) for u in use_a if u[1] not in PATH_KINDS)
    b = collections.Counter((u[1], u[2]) for u in use_b if u[1] not in PATH_KINDS)
    where = {(u[1], u[2]): (u[0], u[3]) for u in use_a + use_b}
    for k in sorted((a - b).elements()):
        res.flags.append(("-", where[k][0], k[0], k[1]))
    for k in sorted((b - a).elements()):
        res.flags.append(("+", where[k][0], k[0], k[1]))
    _redirects(res, use_a, use_b)
    ra = collections.Counter(u[3] for u in use_a if u[1] == "reexport")
    rb = collections.Counter(u[3] for u in use_b if u[1] == "reexport")
    pa = {u[3]: u for u in use_a if u[1] == "reexport"}
    pb = {u[3]: u for u in use_b if u[1] == "reexport"}
    for leaf in sorted((ra - rb).elements()):
        res.reexports.append(("-", pa[leaf][0], leaf))
    for leaf in sorted((rb - ra).elements()):
        res.reexports.append(("+", pb[leaf][0], leaf))


def _parent(leaf):
    path = leaf.partition(" as ")[0].split("::")[:-1]
    return [p for p in path if p not in ("crate", "self", "super")]


def _same_source(a, b):
    """Two import paths of one name point to the same source when the module right before
    the name is the same (a crate or folder rename), or one module path extends the other
    (`a::round` -> `a::b::round`). `exact::round` -> `fast::round` is a redirect (D4)."""
    pa, pb = _parent(a), _parent(b)
    if not pa or not pb or pa[-1] == pb[-1]:
        return True
    return pa == pb[:len(pa)] or pb == pa[:len(pb)]


def _redirects(res, use_a, use_b):
    """D4: the same name imported from a different source compiles and may bind another
    item with the same signature: a FLAG for the reviewer."""
    a = collections.Counter((u[2], u[3]) for u in use_a if u[1] == "plain")
    b = collections.Counter((u[2], u[3]) for u in use_b if u[1] == "plain")
    gone = collections.defaultdict(list)
    for name, leaf in (a - b).elements():
        gone[name].append(leaf)
    where = {(u[2], u[3]): u[0] for u in use_b}
    for name, leaf in sorted((b - a).elements()):
        olds = gone.get(name, [])
        if olds and not any(_same_source(o, leaf) for o in olds):
            res.flags.append(("~", where[(name, leaf)], "redirected", f"{' | '.join(sorted(set(olds)))} -> {leaf}"))


def report(res, listing=False):
    print(f"moved: {res.moved} items unchanged (moved or in place); {len(res.pathonly)} path-only; "
          f"{len(res.allows)} with an added #[allow]; {len(res.changed)} CHANGED; "
          f"{len(res.flags)} FLAG (use); {len(res.uses)} file(s) with path-only import changes; "
          f"{len(res.reexports)} re-export change(s)")
    for a, b in res.pathonly:
        print(f"PATH    {b[0]} {b[1]} {b[2]}" + (f"  (was {a[0]})" if a and a[0] != b[0] else ""))
    for b in res.allows:
        print(f"ALLOW   {b[0]} {b[1]} {b[2]}")
    for path, plus, minus in res.uses:
        print(f"USE     {path}: +{len(plus)} -{len(minus)} (path-only)")
        if listing:
            for p in plus:
                print(f"        + {p}")
            for m in minus:
                print(f"        - {m}")
    for sign, path, leaf in res.reexports:
        print(f"REEXPORT {sign} {path} pub use {leaf}")
    for sign, path, kind, key in res.flags:
        print(f"FLAG    {sign} {path} use ({kind}) {key}")
    for sign, w, t in res.changed:
        print(f"CHANGED {sign} {w[0]} {w[1]} {w[2]}")
        if listing:
            print("        | " + t[:2000])


def main(argv):
    if "--self-test" in argv:
        import moved_selftest
        return 0 if moved_selftest.run(compare) else 1
    prefixes = []
    if "--" in argv:
        i = argv.index("--")
        argv, prefixes = argv[:i], argv[i + 1:]
    listing = "--list" in argv
    strict = "--strict-use" in argv
    argv = [a for a in argv if a not in ("--list", "--strict-use")]
    if not argv:
        print(__doc__)
        return 2
    base = argv[0]
    head = argv[1] if len(argv) > 1 else None
    rng = [base] + ([head] if head else [])
    names = git("diff", "--name-status", "-M", *rng, "--", "*.rs").splitlines()
    if head is None:  # working tree: untracked new files too
        names += [f"A\t{p}" for p in git("ls-files", "--others", "--exclude-standard", "--", "*.rs").split()]
    pairs = []
    for line in names:
        parts = line.split("\t")
        st, paths = parts[0], parts[1:]
        old, new = paths[0], paths[-1]
        if prefixes and not any(p.startswith(tuple(prefixes)) for p in (old, new)):
            continue
        pairs.append((None if st[0] == "A" else old, "" if st[0] == "A" else show(base, old),
                      None if st[0] == "D" else new, "" if st[0] == "D" else show(head, new)))
    res = compare(pairs, strict)
    report(res, listing)
    return 1 if res.failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
