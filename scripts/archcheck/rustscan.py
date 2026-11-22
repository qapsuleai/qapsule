"""A small Rust lexer for arch-check: enough to find test regions, item kinds at the
top level of a file and function bodies, ignoring braces inside comments, strings, raw
strings and char literals. Not a parser; it errs on the side of reporting."""

import re

def strip(src):
    """Returns `src` with comments, string, raw string and char literal contents replaced
    by spaces (newlines kept), so braces and keywords outside code are invisible."""
    out = []
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        nxt = src[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            j = src.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif c == "/" and nxt == "*":
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            out.append(_blank(src[i:j]))
            i = j
        elif c == "r" and re.match(r'r#*"', src[i:i + 300]) and not _ident_before(src, i):
            m = re.match(r'r(#*)"', src[i:])
            assert m is not None
            hashes = m.group(1)
            end = src.find('"' + hashes, i + len(m.group(0)))
            j = n if end < 0 else end + 1 + len(hashes)
            out.append(_blank(src[i:j]))
            i = j
        elif c == "b" and nxt == '"' and not _ident_before(src, i):
            j = _end_string(src, i + 2)
            out.append(_blank(src[i:j]))
            i = j
        elif c == '"':
            j = _end_string(src, i + 1)
            out.append(_blank(src[i:j]))
            i = j
        elif c == "'":
            j = _end_char(src, i)
            if j is None:  # a lifetime or label
                out.append(c)
                i += 1
            else:
                out.append(_blank(src[i:j]))
                i = j
        else:
            out.append(c)
            i += 1
    return "".join(out)


def _ident_before(src, i):
    return i > 0 and (src[i - 1].isalnum() or src[i - 1] == "_")


def _blank(s):
    return "".join("\n" if ch == "\n" else " " for ch in s)


def _end_string(src, j):
    n = len(src)
    while j < n:
        if src[j] == "\\":
            j += 2
        elif src[j] == '"':
            return j + 1
        else:
            j += 1
    return n


def _end_char(src, i):
    """End index of a char literal starting at `i`, or None for a lifetime."""
    n = len(src)
    if i + 2 < n and src[i + 1] == "\\":
        k = src.find("'", i + 2)
        return None if k < 0 or k - i > 12 else k + 1
    if i + 2 < n and src[i + 2] == "'":
        return i + 3
    # multi-byte chars are one Python char, so the check above covers them
    return None


def line_of(text, pos):
    return text.count("\n", 0, pos) + 1


def match_brace(code, open_pos):
    """Index of the `}` matching the `{` at `open_pos` (in stripped code)."""
    depth = 0
    for k in range(open_pos, len(code)):
        ch = code[k]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return k
    return len(code) - 1


_CFG_TEST = re.compile(r"#\[cfg\(\s*test\s*\)\]")


def test_regions(code):
    """(start, end) character ranges of `#[cfg(test)]` items (modules, functions, impls)
    in stripped code. An item without a body (`mod tests;`) has no region."""
    regions = []
    for m in _CFG_TEST.finditer(code):
        k = m.end()
        # the item ends at the first `;` or at the matching brace of its first `{`
        semi = code.find(";", k)
        brace = code.find("{", k)
        if brace < 0 or (0 <= semi < brace):
            continue
        regions.append((m.start(), match_brace(code, brace)))
    return regions


def in_regions(pos, regions):
    return any(a <= pos <= b for a, b in regions)


def test_lines(code):
    """Number of lines inside `#[cfg(test)]` regions."""
    total = 0
    for a, b in test_regions(code):
        total += code.count("\n", a, b) + 1
    return total


_FN = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")
_TEST_ATTR_ANY = re.compile(r"#\[(?:test|tokio::test|wasm_bindgen_test)\b")


def functions(code):
    """[(name, first_line, last_line, is_test)] for every `fn` with a body. Nested
    functions are reported too (their lines also count in the outer one)."""
    regions = test_regions(code)
    out = []
    for m in _FN.finditer(code):
        k = m.end()
        # skip the signature: the body is the first `{` at paren/bracket depth 0 before `;`
        depth, j, body = 0, k, None
        while j < len(code):
            ch = code[j]
            if ch in "([<":
                depth += 1 if ch != "<" else 0
            elif ch in ")]":
                depth -= 1
            elif ch == ";" and depth <= 0:
                break
            elif ch == "{" and depth <= 0:
                body = j
                break
            j += 1
        if body is None:
            continue
        end = match_brace(code, body)
        before = code[max(0, m.start() - 400):m.start()]
        cut = max(before.rfind("}"), before.rfind(";"))
        attrs = before[cut + 1:]
        is_test = in_regions(m.start(), regions) or bool(_TEST_ATTR_ANY.search(attrs))
        out.append((m.group(1), line_of(code, m.start()), line_of(code, end), is_test))
    return out


_TOP_ITEM = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+|async\s+|const\s+(?=fn)|extern\s+\"[^\"]*\"\s+)*"
    r"(fn|impl|struct|enum|union|trait|static|const|macro_rules!)\b",
    re.M,
)


def top_level_items(code):
    """[(kind, line)] of logic-bearing items at brace depth 0 outside test regions:
    fn, impl, struct, enum, union, trait, static, const, macro_rules!."""
    regions = test_regions(code)
    depth_at = _depths(code)
    out = []
    for m in _TOP_ITEM.finditer(code):
        pos = m.start(1)
        if depth_at[pos] != 0 or in_regions(pos, regions):
            continue
        out.append((m.group(1), line_of(code, pos)))
    return out


def _depths(code):
    depth, res = 0, [0] * (len(code) + 1)
    for k, ch in enumerate(code):
        res[k] = depth
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
    res[len(code)] = depth
    return res


def is_generated(src):
    head = "\n".join(src.splitlines()[:5]).lower()
    return "üretilmiştir" in head or bool(re.search(r"(^|[^a-z])@?generated\b", head))


CFG_TEST_MOD = re.compile(r"#\[cfg\(\s*test\s*\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*[;{]")
PATH_ATTR = re.compile(r"#\[path\s*=\s*\"([^\"]+)\"\s*\]")
_INLINE_MOD = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+[A-Za-z_][A-Za-z0-9_]*\s*\{", re.M)


def inline_mods(code):
    """[("mod {…}", line)] for modules with an inline body at depth 0 outside test regions."""
    regions = test_regions(code)
    depth_at = _depths(code)
    out = []
    for m in _INLINE_MOD.finditer(code):
        pos = m.start() + len(m.group(0)) - len(m.group(0).lstrip())
        if depth_at[pos] != 0 or in_regions(pos, regions):
            continue
        out.append(("mod {}", line_of(code, pos)))
    return out
