"""Self-check: the few-shot outputs in prompts/system.txt must pass the verifier."""

import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import verify as V  # noqa: E402

txt = (HERE / "prompts" / "system.txt").read_text(encoding="utf-8")
bad = 0
for s, o in re.findall(r"^slots: (\{.*\})\noutput: (\{.*\})$", txt, re.M):
    s, o = json.loads(s), json.loads(o)
    lc = s.get("last_change") or {}
    s["last_change"] = {"bars_ago": lc.get("bars_ago", s["bars_ago"])}
    s.setdefault("when_phrases", {"tr": [""], "en": [""]})
    vals = [(n["slot"], float(n["en"].replace(",", ""))) for n in s["allowed_numbers"]]
    for lang in ("tr", "en"):
        e = V.check(s, lang, o[lang], vals)
        bad += bool(e)
        print(s["instrument"], lang, e or "ok")
sys.exit(1 if bad else 0)
