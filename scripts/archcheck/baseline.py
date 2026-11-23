"""The debt baseline (docs/DEBT.md, scripts/arch-baseline.json) and the ratchet: a finding
of a non-hard rule passes only if the baseline names its item with a value at least as
large; every rule's count may not exceed the baseline's count for that rule."""

import json
import os


def load(path):
    if not os.path.isfile(path):
        return {"rules": {}}
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def split_exceptions(findings, cfg):
    """(kept, excepted, unused): findings named in cfg["exceptions"] (rule + item) are
    excepted. An exception with a `max` is a latch (K89): a finding whose value exceeds it is
    kept, marked with `ceiling`, and `judge` turns it into an error."""
    ex = {(e["rule"], e["item"]): e for e in cfg.get("exceptions", [])}
    kept, excepted = [], []
    for f in findings:
        e = ex.get((f["rule"], f["item"]))
        if e is not None and "max" in e and f["value"] > e["max"]:
            kept.append(dict(f, ceiling=e))
        elif e is not None:
            excepted.append(dict(f, exception=e))
        else:
            kept.append(f)
    unused = [e for k, e in ex.items() if not any((f["rule"], f["item"]) == k for f in findings)]
    return kept, excepted, unused


# Size rules: an exception from one of these must carry a `max` latch (K89; RF1/A2 for
# file.hard and fn.lines: a single-function file or function may not grow past it).
SIZED = ("crate.lines", "shell.lines", "file.hard", "file.soft", "fn.lines", "dir.files",
         "test.file", "doc.size")


def missing_ceilings(cfg):
    """A size-rule exception without a `max` latch is a configuration error (K89)."""
    return [{"rule": "exception.max", "item": e["item"], "value": 0, "limit": 0,
             "message": f"exception {e.get('id', '')} {e['rule']} {e['item']} has no `max`",
             "why": "a size-rule exception must carry a ceiling (K89)"}
            for e in cfg.get("exceptions", []) if e["rule"] in SIZED and "max" not in e]


def judge(findings, base, hard_rules):
    """Splits findings into (errors, tolerated, stale). `stale` are baseline items that no
    longer appear or got smaller (the baseline can be tightened)."""
    rules = base.get("rules", {})
    errors, tolerated = [], []
    per_rule = {}
    for f in findings:
        per_rule.setdefault(f["rule"], []).append(f)
    for rule, fs in sorted(per_rule.items()):
        entry = rules.get(rule, {})
        items = entry.get("items", {})
        for f in fs:
            if "ceiling" in f:
                c = f["ceiling"]
                errors.append(dict(f, why=f"above the exception ceiling {c['max']} ({c.get('id', '')})"))
                continue
            if rule in hard_rules:
                errors.append(dict(f, why="hard rule (no baseline)"))
                continue
            allowed = items.get(f["item"])
            if allowed is None:
                errors.append(dict(f, why="not in the baseline (new debt)"))
            elif f["value"] > _value(allowed):
                errors.append(dict(f, why=f"above the baseline ({_value(allowed)})"))
            else:
                tolerated.append(f)
        limit = entry.get("count")
        n = sum(1 for f in fs if "ceiling" not in f)
        if limit is not None and rule not in hard_rules and n > limit:
            errors.append({"rule": rule, "item": "*", "value": n, "limit": limit,
                           "message": f"{rule}: {n} findings > baseline count {limit}",
                           "why": "rule count above the baseline"})
    stale = []
    present = {(f["rule"], f["item"]): f["value"] for f in findings}
    for rule, entry in sorted(rules.items()):
        for item, allowed in sorted(entry.get("items", {}).items()):
            v = present.get((rule, item))
            if v is None:
                stale.append(f"{rule} {item}: gone (remove it from the baseline)")
            elif v < _value(allowed):
                stale.append(f"{rule} {item}: {v} < baseline {_value(allowed)} (tighten)")
    return errors, tolerated, stale


def _value(allowed):
    return allowed["value"] if isinstance(allowed, dict) else allowed


def build(findings, hard_rules, old=None, notes=None):
    """A baseline holding the current non-hard findings (used once at round end; the
    ratchet forbids raising an existing value: callers check `judge` first)."""
    old = old or {"rules": {}}
    rules = {}
    for f in findings:
        if f["rule"] in hard_rules:
            continue
        e = rules.setdefault(f["rule"], {"count": 0, "items": {}})
        e["count"] += 1
        prev = old.get("rules", {}).get(f["rule"], {}).get("items", {}).get(f["item"])
        note = (notes or {}).get((f["rule"], f["item"]))
        if isinstance(prev, dict) or note:
            d = dict(prev) if isinstance(prev, dict) else {}
            d["value"] = f["value"]
            if note:
                d.update(note)
            e["items"][f["item"]] = d
        else:
            e["items"][f["item"]] = f["value"]
    return {"_doc": "arch-check debt baseline (ratchet: values and counts may only go down). "
                    "Human-readable: docs/DEBT.md.", "rules": rules}
