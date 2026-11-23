"""Decision citations for `--config-diff` (K-9; RF1/A denetimi O2, R1). A loosening must
cite a **new** decision taken for it:

- `K<n>`: a row `| K<n> | …` under docs/decisions/ in the working tree that is **not** in the
  base revision's docs/decisions/ (the decision was added after the base);
- `rounds/<R>/DECISION.md`: the file is new or changed against the base **and** names the
  entry (exception item, baseline item, override crate, generated path or rule_changes key).

Old decisions, old rounds' DECISION.md files and citations already present anywhere in
the base configuration do not count.

Known limit (R2): the decision files are read from the working tree, so a change can add
its own decision row; `--config-diff` prints every row added since the base so the
reviewer sees it (decision rows are written by the main-line session only).
"""

import glob
import os
import re
import subprocess

CITE = re.compile(r"\bK\d+\b|\brounds/[A-Za-z0-9_-]+/DECISION\.md\b")
ROW = re.compile(r"^\|\s*(K\d+)\s*\|", re.M)


def found(*texts):
    return set(CITE.findall(" ".join(t for t in texts if isinstance(t, str))))


class Citer:
    def __init__(self, rows, base_rows, changed_files, base_text):
        self.new_rows = set(rows) - set(base_rows)  # decision rows added since the base
        self.changed = dict(changed_files)          # DECISION.md path -> text, new/changed
        self.used = found(base_text)                # citations already in the base config

    @classmethod
    def from_repo(cls, root, base, base_text):
        def git(*a):
            r = subprocess.run(["git", *a], cwd=root, capture_output=True, text=True)
            return r.stdout if r.returncode == 0 else ""

        rows = set()
        for f in glob.glob(os.path.join(root, "docs", "decisions", "**", "*.md"), recursive=True):
            with open(f, encoding="utf-8") as fh:
                rows |= set(ROW.findall(fh.read()))
        base_rows = set()
        for f in git("ls-tree", "-r", "--name-only", base, "docs/decisions").split():
            if f.endswith(".md"):
                base_rows |= set(ROW.findall(git("show", f"{base}:{f}")))
        names = set(git("diff", "--name-only", base, "--", "rounds").split())
        names |= set(git("ls-files", "--others", "--exclude-standard", "--", "rounds").split())
        changed = {}
        for p in sorted(n for n in names if re.fullmatch(r"rounds/[A-Za-z0-9_-]+/DECISION\.md", n)):
            if os.path.isfile(os.path.join(root, p)):
                with open(os.path.join(root, p), encoding="utf-8") as fh:
                    changed[p] = fh.read()
        return cls(rows, base_rows, changed, base_text)

    def fresh(self, name, *texts):
        """(valid new citations for the entry `name`, rejected ones with the reason)."""
        ok, bad = set(), []
        for c in sorted(found(*texts)):
            if c in self.used:
                bad.append(f"{c}: already cited in the base configuration")
            elif c.startswith("K") and c not in self.new_rows:
                bad.append(f"{c}: not a decision row added since the base")
            elif not c.startswith("K") and c not in self.changed:
                bad.append(f"{c}: not new or changed since the base")
            elif not c.startswith("K") and name not in self.changed[c]:
                bad.append(f"{c}: does not name `{name}`")
            else:
                ok.add(c)
        return ok, bad
