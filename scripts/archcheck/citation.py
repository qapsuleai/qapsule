"""Decision citations for `--config-diff` (K-9; RF1/A denetimi O2). A citation counts only
if it is real and new:

- `K<n>`: a row `| K<n> | …` exists in a Markdown file under docs/decisions/ (working tree);
- `rounds/<R>/DECISION.md`: that file exists (working tree);

and the same citation appears nowhere in the base revision's arch-config.json /
arch-baseline.json (so a decision already used for another entry cannot be copied over)."""

import glob
import os
import re

CITE = re.compile(r"\bK\d+\b|\brounds/[A-Za-z0-9_-]+/DECISION\.md\b")
ROW = re.compile(r"^\|\s*(K\d+)\s*\|", re.M)


def found(*texts):
    return set(CITE.findall(" ".join(t for t in texts if isinstance(t, str))))


class Citer:
    def __init__(self, rows, exists, base_text):
        self.rows = set(rows)          # K numbers with a decision row
        self.exists = exists           # path -> bool
        self.used = found(base_text)   # citations already in the base configuration

    @classmethod
    def from_repo(cls, root, base_text):
        rows = set()
        for f in glob.glob(os.path.join(root, "docs", "decisions", "**", "*.md"), recursive=True):
            with open(f, encoding="utf-8") as fh:
                rows |= set(ROW.findall(fh.read()))
        return cls(rows, lambda p: os.path.isfile(os.path.join(root, p)), base_text)

    def real(self, c):
        return c in self.rows if c.startswith("K") else self.exists(c)

    def fresh(self, *texts):
        """(valid new citations, rejected ones with the reason)."""
        ok, bad = set(), []
        for c in sorted(found(*texts)):
            if not self.real(c):
                bad.append(f"{c}: no such decision row / file")
            elif c in self.used:
                bad.append(f"{c}: already cited in the base configuration (not a new decision)")
            else:
                ok.add(c)
        return ok, bad
