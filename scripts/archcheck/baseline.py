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
        if limit is not None and rule not in hard_rules and len(fs) > limit:
            errors.append({"rule": rule, "item": "*", "value": len(fs), "limit": limit,
                           "message": f"{rule}: {len(fs)} findings > baseline count {limit}",
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
