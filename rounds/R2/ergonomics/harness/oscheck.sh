#!/bin/sh
# OS-level isolation check, outside Claude Code: runs plain commands under the same
# sandbox profile as a measured agent and prints what the OS allows/denies.
# usage: oscheck.sh <profile.sb> <packet dir> <hidden docs dir> <repo root> [<own work dir> <other agent work dir>]
PROF="$1"; P="$2"; D="$3"; REPO="$4"
echo "# sandbox-exec -f $(basename "$PROF") <cmd>"
for f in "$REPO/CLAUDE.md" "$REPO/rounds/R2/tasks/TASKS.md" "$REPO/rounds/R2/ergonomics/harness/setup.py" \
         "$D/doc.json" "$P/../../target-base/release/qp" "$P/PROTOCOL.md"; do
  echo "\$ head -c 40 $f"
  sandbox-exec -f "$PROF" head -c 40 "$f" 2>&1 | head -2
  echo
done
echo "\$ ls /Users/arkns/.claude/projects/-Users-arkns-Works-Mchart-qapsule"
sandbox-exec -f "$PROF" ls /Users/arkns/.claude/projects/-Users-arkns-Works-Mchart-qapsule 2>&1 | head -2
echo "\$ ls $REPO"
sandbox-exec -f "$PROF" ls "$REPO" 2>&1 | head -2
# write probes: repo, another agent's work dir, hidden docs, packet -> denied; own work dir -> allowed
W="$5"; OTHER="$6"
[ -n "$W" ] || exit 0
echo
for f in "$REPO/rounds/R2/ergonomics/PWNED.txt" "$OTHER/PWNED.txt" "$D/PWNED.txt" "$P/PWNED.txt" "$W/own-write-ok.txt"; do
  echo "\$ touch $f"
  sandbox-exec -f "$PROF" touch "$f" 2>&1 | head -2
  [ -e "$f" ] && echo "(created)"
  echo
done
rm -f "$W/own-write-ok.txt"
