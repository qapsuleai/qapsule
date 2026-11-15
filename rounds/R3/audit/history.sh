#!/usr/bin/env bash
# VERIFY 14: düzenek commit'leri alternatif dallarının ilk commit'inden önce mi; alternatif
# dallar birbirinin dosyalarına dokunuyor mu. Salt okunur (git log / merge-base).
set -euo pipefail
cd "$(dirname "$0")/../../.."
spec=a35d726   # SPEC
h12=71fd241    # M1/M2 düzeneği
hsim=176f77e   # M3–M6 düzeneği
printf '%-14s %-8s %-27s %3s %-5s %-4s %-5s %s\n' dal taban "ilk commit (zaman)" n SPEC H12 Hsim "dokunduğu M klasörleri"
for b in m1a-siluet m1b-rozet m1c-iz m2a-template m2b-llmslot m3a-rarity m3b-ledger m3c-confirm \
  m4a-kinddir m4b-arc m5a-barrier m5b-semantic m6a-coarse m6b-novel m7-dispute; do
  tip=$(git rev-parse "r3/$b")
  # Birleştirme commit'i (ikinci ebeveyni dalın ucu ya da atası); taban = ilk ebeveynle ortak ata.
  m=$(git log r3/int --merges --format='%H %P' | while read -r h p1 p2; do
        if [ "$p2" = "$tip" ] || git merge-base --is-ancestor "$tip" "$p2" 2>/dev/null; then echo "$h"; fi
      done | awk 'END{print}')
  base=$(git merge-base "$m^1" "r3/$b")
  first=$(git log --reverse --format='%h %ci' "$base..r3/$b" | awk 'NR==1' | cut -c1-27)
  n=$(git rev-list --count "$base..r3/$b")
  s=$(git merge-base --is-ancestor $spec "r3/$b" && echo y || echo n)
  a=$(git merge-base --is-ancestor $h12 "r3/$b" && echo y || echo n)
  c=$(git merge-base --is-ancestor $hsim "r3/$b" && echo y || echo n)
  dirs=$(git diff --name-only "$base" "r3/$b" | grep -o 'rounds/R3/experiments/M[0-9]/[^/]*' | sort -u | tr '\n' ' ' || true)
  printf '%-14s %-8s %-27s %3s %-5s %-4s %-5s %s\n' "$b" "$(git log -1 --format=%h "$base")" "$first" "$n" "$s" "$a" "$c" "$dirs"
done
echo
echo "Düzenek commit'leri:"
git log -1 --format='%h %ci %s' $spec
git log -1 --format='%h %ci %s' $h12
git log -1 --format='%h %ci %s' $hsim
