"""SPEC Ek G5: re-score a recorded run's meaning-colour gate from the look records' roles
(no new calls, no new shots). The shot records of the recorded runs were not kept, so the
roles the marks are drawn in cannot be read from the scenes; the recorded alphabets draw
their ink/edge in `text` (siluet edge, tg5 realized/failed ink), so `text` is added to the
six meaning roles. Mark size: the recorded table's `mark_px` (faces) or 16 (M18 chart: the
stricter threshold, as recorded).

    python3 g5_rescore.py <computed.json> <design>=<look record json> ... [--write]

Prints one line per design and theme; with --write, puts the Ek G5 gate into `colour`, keeps
the earlier table as `colour_scene`, recomputes `pass`, and rewrites computed.json.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from gl import compute as CP  # noqa: E402

GATES = ("text_contrast", "nontext_contrast", "min_font", "touch", "colour")


def main(argv):
    write = "--write" in argv
    argv = [x for x in argv if x != "--write"]
    cpath = Path(argv[0])
    comp = json.loads(cpath.read_text())
    looks = dict(x.split("=", 1) for x in argv[1:])
    for d, g in sorted(comp.items()):
        if d not in looks:
            continue
        look = json.loads(Path(looks[d]).read_text())
        old = g.get("colour_scene") or g["colour"]
        themes = sorted(old.get("themes") or {"light": 1, "dark": 1})
        px = [r.get("mark_px") for r in old.get("pairs", []) if r.get("mark_px")]
        size = min(px) if px else 16
        used = {th: ["text"] for th in themes}
        gate = CP.role_colour_gate(look, themes, used, {th: size for th in themes})
        for th, t in gate["themes"].items():
            fails = [f"{r['a']}↔{r['b']} {r['min']}" for r in t["pairs"] if not r["pass"]]
            shared = [f"{s['a']}={s['b']}" for s in t["shared"]]
            print(f"{d} {th}: {'pass' if t['pass'] else 'FAIL'} ({len(t['pairs'])} pairs, {len(fails)} fail; thr {t['pairs'][0]['threshold'] if t['pairs'] else '-'}) "
                  f"fails={fails} shared={shared}")
        if write:
            g["colour_scene"] = old
            g["colour"] = gate
            g["pass"] = all(g[k]["pass"] for k in GATES if k in g)
    if write:
        cpath.write_text(json.dumps(comp, indent=1, ensure_ascii=False, sort_keys=True) + "\n")


if __name__ == "__main__":
    main(sys.argv[1:])
