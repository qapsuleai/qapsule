#!/usr/bin/env python3
"""Summarizes a macOS `sample` call graph of `tick-bench profile`.

For each key, sums the inclusive sample counts of frames whose symbol matches the key's
pattern and that have no ancestor matching the same pattern (no double counting under
recursion or nesting). Shares are relative to `Session::handle_raw`.

Usage: sample-summary.py <sample.txt>
"""
import re
import sys

KEYS = [
    ("handle_raw (total)", r"Session::handle_raw"),
    ("JSON parse (serde_json from_str to Value)", r"serde_json::de::from_str|serde_json::de::Deserializer"),
    ("typed parse from Value (serde_path_to_error / value::de)", r"serde_path_to_error|serde_json::value::de"),
    ("resolve_selectors", r"Session::resolve_selectors"),
    ("check_limits", r"Session::check_limits"),
    ("expand_ops (incl. doc clone)", r"expand::expand_ops"),
    ("Document::clone (anywhere)", r"Document\$u20\$as\$u20\$core\.\.clone\.\.Clone"),
    ("drop of a Document (anywhere)", r"drop_in_place\$LT\$qp_schema\.\.types\.\.Document"),
    ("commit", r"Session::commit"),
    ("settle (watchers, invalidations, events)", r"Session::settle"),
    ("eval_watchers", r"Session::eval_watchers"),
    ("invalidations", r"Session::invalidations"),
    ("entry_events", r"Session::entry_events"),
    ("compute_bars (anywhere)", r"qp_compute::compute_bars"),
    ("malloc/free (anywhere)", r"_xzm_|_malloc|_free\b|malloc_zone"),
]

LINE = re.compile(r"^([ +!:|]*?)(\d+) (.*)$")


def main(path: str) -> None:
    frames = []  # (depth, count, symbol)
    in_graph = False
    for raw in open(path, encoding="utf-8", errors="replace"):
        if raw.startswith("Call graph:"):
            in_graph = True
            continue
        if in_graph and (raw.startswith("Total number in stack") or raw.startswith("Sort by top")):
            break
        if not in_graph:
            continue
        m = LINE.match(raw.rstrip("\n"))
        if not m:
            continue
        frames.append((len(m.group(1)), int(m.group(2)), m.group(3)))
    res = {}
    for name, pat in KEYS:
        rx = re.compile(pat)
        total = 0
        stack = []  # depths of matching ancestors
        for depth, count, sym in frames:
            while stack and stack[-1] >= depth:
                stack.pop()
            if rx.search(sym):
                if not stack:
                    total += count
                stack.append(depth)
        res[name] = total
    base = res["handle_raw (total)"] or 1
    print(f"{'key':60} {'samples':>8} {'share':>7}")
    for name, _ in KEYS:
        print(f"{name:60} {res[name]:8d} {100.0 * res[name] / base:6.1f}%")


if __name__ == "__main__":
    main(sys.argv[1])
