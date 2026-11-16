"""Independent leaf diff of changed fixtures (main vs HEAD); classifies each changed leaf.
Usage: python3 -I leafdiff.py <path> ..."""
import json, re, subprocess, sys
from collections import Counter

def load(rev, path):
    try:
        return json.loads(subprocess.check_output(["git", "show", f"{rev}:{path}"], stderr=subprocess.DEVNULL))
    except subprocess.CalledProcessError:
        return None

def walk(a, b, p, out):
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            walk(a.get(k, "<absent>"), b.get(k, "<absent>"), p + [k], out)
    elif isinstance(a, list) and isinstance(b, list) and len(a) == len(b):
        for i, (x, y) in enumerate(zip(a, b)):
            walk(x, y, p + [i], out)
    elif a != b:
        out.append((p, a, b))

# Karne (ledger) clause of a sentence, TR and EN, R3 and R4 forms.
KARNE = [
    r"Karnesi[^.]*\.", r"Henüz karnesi yok\.", r"Its record[^.]*\.", r"No track record yet\.",
    r"\d+ seferin \d+'?\w*\s*\w*\.", r"in \d+ of \d+ cases?\.", r"Karne[^.]*\.",
    r"It played out[^.]*\.", r"Geçmişte[^.]*\.", r"In the past[^.]*\.",
]
def strip_karne(s):
    for k in KARNE:
        s = re.sub(k, "", s)
    return re.sub(r"\s+", " ", s).strip()

K42_KEYS = {"validity", "ledger_at_birth", "n", "realized", "importance", "ledger_key", "condition",
            "resolution", "direction", "key", "total", "rows", "confidence", "title", "expected", "tier"}
SENT_KEYS = {"text", "tr", "en", "headline", "claim", "summary"}

for path in sys.argv[1:]:
    a, b = load("main", path), load("HEAD", path)
    out = []
    walk(a, b, [], out)
    cls = Counter()
    other = []
    for p, x, y in out:
        keys = [k for k in p if isinstance(k, str)]
        last = keys[-1] if keys else ""
        if any(k in K42_KEYS for k in keys[-3:]):
            cls["k42-field:" + next(k for k in reversed(keys[-3:]) if k in K42_KEYS)] += 1
        elif last in SENT_KEYS and isinstance(x, str) and isinstance(y, str):
            if strip_karne(x) == strip_karne(y):
                cls["sentence: karne clause only"] += 1
            else:
                cls["sentence: other change"] += 1
                other.append((p, x, y))
        else:
            cls["other:" + last] += 1
            other.append((p, x, y))
    print(f"== {path}: {len(out)} changed leaves")
    for k, v in sorted(cls.items()):
        print(f"   {v:5d} {k}")
    for p, x, y in other[:6]:
        print("   OTHER", "/".join(map(str, p))[:160])
        print("     -", str(x)[:260])
        print("     +", str(y)[:260])
