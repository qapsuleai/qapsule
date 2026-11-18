#!/usr/bin/env bash
# Denetçi (R5/A, VERIFY 21): R5'in eklediği her şeyde gizli bilgi kalıpları, büyük dosyalar,
# gerçek veri işaretleri. Taban: RP3'ün son commit'i 4ccd4d6. Bütün dallar için ayrıca `git log --all`.
set -u
cd "$(dirname "$0")/../../.."
BASE=4ccd4d6
PAT='sk-ant-[A-Za-z0-9]|ghp_[A-Za-z0-9]{20}|github_pat_|AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY|xox[baprs]-[0-9A-Za-z]|eyJ[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]{20,}'
echo "== secret patterns in main since $BASE:"
git diff -U0 "$BASE" HEAD | grep -cE "$PAT"
echo "== secret patterns in every r5/ branch's own diff (vs main):"
n=0
for b in $(git for-each-ref --format='%(refname:short)' refs/heads/r5/); do
  c=$(git diff -U0 "$(git merge-base "$b" main)" "$b" | grep -cE "$PAT")
  n=$((n + c))
done
echo "$n"
echo "== .env / key-like files tracked:"
git ls-files | grep -E '(^|/)\.env|\.pem$|\.key$|id_rsa' || echo none
echo "== fixtures/local in any commit on any branch:"
git log --all --format='%h' -- fixtures/local .env.local | wc -l
echo "== files added since $BASE larger than 300 KB:"
git diff --name-only --diff-filter=A "$BASE" HEAD | while read -r f; do
  [ -f "$f" ] || continue
  s=$(wc -c < "$f"); [ "$s" -gt 300000 ] && echo "$s $f"
done
echo "== added binary files since $BASE by folder:"
git diff --numstat "$BASE" HEAD | awk '$1=="-"{print $3}' | sed 's|/[^/]*$||' | sort | uniq -c
echo "== real-symbol scan (BIST tickers, not SYNnn) in added R5 experiment files:"
git diff --name-only --diff-filter=AM "$BASE" HEAD -- rounds/R5 | grep -vE '\.(png|gz)$' | \
  xargs grep -lE '\b(THYAO|GARAN|AKBNK|ASELS|EREGL|KCHOL|SISE|TUPRS|BIMAS|YKBNK|ISCTR|SAHOL|PGSUS|FROTO|TCELL)\b' 2>/dev/null
