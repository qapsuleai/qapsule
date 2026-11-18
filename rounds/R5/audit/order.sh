#!/usr/bin/env bash
# Denetçi (R5/A, madde 2): düzenek ↔ alternatif commit sırası.
# Her r5/ dalı için: dalın main'in ilk-ebeveyn zincirinde olmayan ilk commit'i (zamanı),
# ve düzenek commit'lerinin dalın atası olup olmadığı.
set -u
cd "$(dirname "$0")/../../.."
echo "== düzenek / ek commit'leri"
for c in fa0487d 70d8e11 0e85cb6 9c01848 85338e2 fca8f33 655e93e 69d27cc; do
  git log -1 --format="%h %ci %s" "$c" | cut -c1-150
done
git log --first-parent --format=%H main > /tmp/qp-audit-mainfp.$$
echo "== dallar"
for b in $(git for-each-ref --format='%(refname:short)' refs/heads/r5/); do
  first=""
  for h in $(git rev-list --reverse "$b" ^"$(git merge-base "$b" main)"); do
    first=$(git log -1 --format="%h %ci %s" "$h" | cut -c1-110); break
  done
  if [ -z "$first" ]; then
    # dal tamamen main'de: main'in ilk ebeveyninde olmayan, dalın ilk commit'i
    for h in $(git rev-list --reverse "$b"); do
      if ! grep -q "$h" /tmp/qp-audit-mainfp.$$; then first="(main'de) $(git log -1 --format='%h %ci %s' "$h" | cut -c1-100)"; break; fi
    done
  fi
  a=""
  for h in fa0487d 70d8e11 0e85cb6 85338e2 fca8f33 655e93e; do
    if git merge-base --is-ancestor "$h" "$b"; then a="$a $h+"; else a="$a $h-"; fi
  done
  printf '%-22s first=[%s]\n   anc:%s\n' "$b" "$first" "$a"
done
rm -f /tmp/qp-audit-mainfp.$$
