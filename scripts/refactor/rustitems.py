"""What `moved.py` fingerprints in one Rust file (a lexer, not a parser; scripts/archcheck
/rustscan.py does the stripping of comments and literals):

- items: `fn` (with its body), `struct`, `enum`, `union`, `trait`, `type`, `const`,
  `static`, `macro_rules!`, and item-level macro invocations (`foo! { … }`, `foo!(…);`),
  each with the attributes and doc comments right above it (so a changed or dropped
  `#[cfg(…)]` changes the item);
- `impl` headers (attributes + `impl<…> Trait for Type where …`), as a set: an impl may be
  split or merged, but every header must survive unchanged;
- `mod` declarations carrying a `#[cfg(…)]` other than a bare `#[cfg(test)]`;
- `use` / `extern crate` declarations outside fn bodies, expanded to one leaf per name,
  with their `#[cfg]` / `#[macro_use]` attributes.
"""

import re

from archcheck import rustscan as rs

VIS = re.compile(r"^pub(\((crate|super|self|in [^)]*)\))?\s+")
VIS_INLINE = re.compile(r"(?<![A-Za-z0-9_])pub(\((crate|super|self|in [^)]*)\))?\s*(?=[A-Za-z_])")
ITEM = re.compile(
    r"(?<!')\b(fn|struct|enum|union|trait|type|const|static|macro_rules!)(?:\s+|(?<=!)\s*)([A-Za-z_][A-Za-z0-9_]*)?")
IMPL = re.compile(r"^[ \t]*(?:(?:unsafe|default)\s+)*impl\b", re.M)
MACRO = re.compile(r"^[ \t]*((?:[A-Za-z_][A-Za-z0-9_]*::)*[A-Za-z_][A-Za-z0-9_]*)\s*!\s*([({\[])", re.M)
MOD = re.compile(r"^[ \t]*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*[;{]", re.M)
USE = re.compile(r"^[ \t]*(pub(?:\([^)]*\))?\s+)?(use|extern\s+crate)\s+", re.M)


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


class File:
    """One file's fingerprints: `items` [(kind, name, text)], `impls` {text}, `cfg_mods`
    [(name, text)], `uses` [(category, key, leaf, attrs)]."""

    def __init__(self, src):
        self.src = src
        self.code = rs.strip(src)
        self.free = _not_in_fn_body(self.code)
        items, covered = self._items()
        macros, spans = self._macros(covered)
        # an item written inside a macro's arguments (`thread_local!(static N: …)`) is part
        # of the macro, not an item of its own
        self.items = [it for it, (a, _) in zip(items, covered)
                      if not any(x <= a <= z for x, z in spans)] + macros
        self.impls = self._impls()
        self.cfg_mods = self._cfg_mods()
        self.uses = self._uses()

    def _attrs(self, pos):
        start = _attr_start(self.src, _line_start(self.code, pos))
        return start, self.src[start:_line_start(self.code, pos)]

    def _items(self):
        code, out, covered = self.code, [], []
        for m in ITEM.finditer(code):
            kind, pos = m.group(1), m.start(1)
            if not self.free[pos]:
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
            start, _ = self._attrs(pos)
            covered.append((pos, end))
            out.append((kind, m.group(2) or "", norm(self.src[start:end + 1])))
        return out, covered

    def _macros(self, covered):
        code, out, spans = self.code, [], []
        for m in MACRO.finditer(code):
            pos = m.start(1)
            if m.group(1) == "macro_rules" or not self.free[pos] or any(a <= pos <= z for a, z in covered):
                continue
            end = _macro_end(code, m.end(2) - 1)
            start, _ = self._attrs(pos)
            spans.append((pos, end))
            out.append(("macro!", m.group(1), norm(self.src[start:end + 1])))
        return out, spans

    def _impls(self):
        out = set()
        for m in IMPL.finditer(self.code):
            pos = m.end() - 4
            if not self.free[pos]:
                continue
            brace = _header_end(self.code, m.end())
            if brace is None:
                continue
            start, _ = self._attrs(pos)
            out.add(norm(self.src[start:brace]))
        return out

    def _cfg_mods(self):
        out = []
        for m in MOD.finditer(self.code):
            if not self.free[m.start(1)]:
                continue
            _, attrs = self._attrs(m.start(1))
            cfgs = re.findall(r"#\[cfg\b.*", attrs)
            if cfgs and [norm(c) for c in cfgs] != ["#[cfg(test)]"]:
                out.append((m.group(1), norm(attrs + "mod " + m.group(1))))
        return out

    def _uses(self):
        out = []
        for m in USE.finditer(self.code):
            if not self.free[m.start(2)]:
                continue
            semi = self.code.find(";", m.end())
            if semi < 0:
                continue
            _, attrs = self._attrs(m.start(2))
            attrs = " ".join(norm(a) for a in re.findall(r"#\[(?:cfg|macro_use)\b.*", attrs))
            tree = re.sub(r"\s+", " ", self.code[m.end():semi]).strip()
            tree = re.sub(r"\s*(::|[{},])\s*", r"\1", tree)
            for leaf in ([tree] if m.group(2) != "use" else expand(tree)):
                cat, key = _classify(leaf, bool(m.group(1)), attrs, m.group(2) != "use")
                out.append((cat, key, leaf, attrs))
        return out


def expand(tree):
    """`a::{b,c::{d as e,*},self}` -> ['a::b', 'a::c::d as e', 'a::c::*', 'a']."""
    i = tree.find("{")
    if i < 0:
        return [tree]
    prefix, inner = tree[:i], tree[i + 1:tree.rfind("}")]
    parts, depth, cur = [], 0, ""
    for ch in inner:
        depth += ch == "{"
        depth -= ch == "}"
        if ch == "," and depth == 0:
            parts.append(cur)
            cur = ""
        else:
            cur += ch
    parts.append(cur)
    out = []
    for p in (p.strip() for p in parts):
        if p == "self":
            out.append(prefix.rstrip(":"))
        elif p:
            out += [prefix + x for x in expand(p)]
    return out


def _classify(leaf, public, attrs, extern):
    """(category, key): `plain` / `reexport` are path-only kinds (key: the bound name);
    `glob`, `anon` (`as _`), `cfg`, `extern` are not (a semantic change if unmatched)."""
    path, _, alias = leaf.partition(" as ")
    last = path.rstrip(":").split("::")[-1]
    if "#[cfg" in attrs or "#[macro_use" in attrs or extern:
        return ("cfg" if "#[cfg" in attrs else "extern"), f"{attrs} {alias or last}"
    if last == "*":
        key = path.split("::")[-2] if "::" in path else "*"
        return ("reexport" if public else "glob"), key + "::*"
    if alias == "_":
        return "anon", last
    return ("reexport" if public else "plain"), alias or last


def _line_start(code, pos):
    return code.rfind("\n", 0, pos) + 1


def _attr_start(src, ls):
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
    while k < len(code):
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


def _header_end(code, k):
    """Index of the `{` that opens an impl body (None if a `;` comes first)."""
    depth = 0
    while k < len(code):
        ch = code[k]
        depth += ch in "(["
        depth -= ch in ")]"
        if ch == ";" and depth <= 0:
            return None
        if ch == "{" and depth <= 0:
            return k
        k += 1
    return None


def _macro_end(code, k):
    """Last index of a macro invocation whose opener is at `k` (with a trailing `;`)."""
    close = {"(": ")", "[": "]", "{": "}"}[code[k]]
    depth, j = 0, k
    while j < len(code):
        if code[j] in "([{":
            depth += 1
        elif code[j] in ")]}":
            depth -= 1
            if depth == 0:
                break
        j += 1
    rest = code[j + 1:j + 40].lstrip()
    if close != "}" and rest.startswith(";"):
        return code.index(";", j + 1)
    return j


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
