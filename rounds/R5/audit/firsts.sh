#!/usr/bin/env bash
# Denetçi (R5/A, madde 2): her alternatif/ölçüm dalının kendi ilk ve son commit'i
# (ileti öneki "R5/<dal-adı>"), dal ile main'in ortak atası ve düzenek commit'lerine göre konumu.
set -u
cd "$(dirname "$0")/../../.."
for b in m17-selvi m17-tunca m17-marla m17-measure m17-int m19-feed m18-protocol m18-korda m18-selen m18-pavka m18-measure m18-pavka-perf m16-bertan m16-kivra m16-oltan m16-zemre m16-measure m20-tesir m20-nobra m20-measure m21-surek m21-nabiz m21-measure d-density s-int; do
  ref="r5/$b"
  own=$(git log --reverse --format="%h %ci %s" "$ref" | grep -F " R5/$b" )
  first=$(printf '%s\n' "$own" | head -1 | cut -c1-34)
  last=$(printf '%s\n' "$own" | tail -1 | cut -c1-34)
  n=$(printf '%s\n' "$own" | grep -c .)
  # fork point: parent of first own commit
  fh=$(printf '%s\n' "$own" | head -1 | cut -d' ' -f1)
  par=""; [ -n "$fh" ] && par=$(git log -1 --format="%h %ci" "$fh^" | cut -c1-27)
  printf '%-15s n=%-3s first=%s | last=%s | parent=%s\n' "$b" "$n" "$first" "$last" "$par"
done
