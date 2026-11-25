"""`calls.restricted` (RL1/Ö2, A71): methods that write outside the command path may be
called only from the paths named in `restricted_calls` (scripts/arch-config.json).

Each entry: `{"id", "methods": [...], "callers": [path or path prefix ending in "/"],
"reason"}`. A call is the method name after `.` or `::` (a method call, a UFCS call
`Session::m(&mut s)` or a path `Session::m` passed as a function) in code, not in a
comment or a string; a definition (`fn m`) is not a call. Every Rust file of every crate
is checked (library, tests, examples, benches). Hard rule: no baseline."""

import re

from .rustscan import line_of


def _allowed(rel, callers):
    return any(rel == c or (c.endswith("/") and rel.startswith(c)) for c in callers)


def rule_restricted_calls(scan, cfg):
    for entry in cfg.get("restricted_calls", []):
        names = "|".join(re.escape(m) for m in entry["methods"])
        pat = re.compile(r"(?:\.|::)\s*(" + names + r")\b")
        callers = entry["callers"]
        for rel, (_, code) in sorted(scan.files.items()):
            if _allowed(rel, callers):
                continue
            hits = {}
            for m in pat.finditer(code):
                hits.setdefault(m.group(1), []).append(line_of(code, m.start()))
            for name, lines in sorted(hits.items()):
                where = ", ".join(str(n) for n in lines)
                yield {"rule": "calls.restricted", "item": f"{rel}::{name}", "value": len(lines),
                       "limit": 0,
                       "message": f"{rel}: calls `{name}` (line {where}); restricted by "
                                  f"{entry.get('id', '')} to {', '.join(callers)}"}
