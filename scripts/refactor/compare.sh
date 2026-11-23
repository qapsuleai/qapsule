#!/usr/bin/env bash
# Behaviour comparison: regenerate the snapshot (scripts/refactor/snapshot.sh) and compare
# it byte for byte with the round's baseline (default rounds/RF1/experiments/baseline/
# manifest.sha256; another with --baseline <manifest>).
#
# Usage: scripts/refactor/compare.sh [--baseline <manifest>] [--keep <dir>] [--no-wasm] [--no-x86] [--no-gallery]
# Exit 0 iff every baseline line is present and equal and there are no extra files.
# A difference prints the differing paths; the regenerated tree is kept for inspection
# (default under $CARGO_TARGET_DIR/refactor-compare-*).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
baseline="$root/rounds/RF1/experiments/baseline/manifest.sha256"

target_dir="${CARGO_TARGET_DIR:-$root/target}"
case "$target_dir" in /*) ;; *) target_dir="$root/$target_dir" ;; esac
dir="$target_dir/refactor-compare-$(printf %s "$root" | shasum | cut -c1-8)"
pass=()
while [ $# -gt 0 ]; do
  case "$1" in
    --keep) dir="$2"; shift 2 ;;
    --baseline) baseline="$2"; shift 2 ;;
    --no-wasm|--no-x86|--no-gallery) pass+=("$1"); shift ;;
    *) echo "unknown flag $1" >&2; exit 2 ;;
  esac
done

[ -f "$baseline" ] || { echo "compare: no baseline at $baseline" >&2; exit 2; }
"$root/scripts/refactor/snapshot.sh" "$dir" ${pass[@]+"${pass[@]}"} >/dev/null

base_cmp="$(mktemp)"
trap 'rm -f "$base_cmp"' EXIT
cp "$baseline" "$base_cmp"
# Skipped parts (--no-wasm / --no-x86) are left out of the comparison and said so.
for p in ${pass[@]+"${pass[@]}"}; do
  case "$p" in
    --no-wasm) grep -v ' compute/digest.wasm32.txt$' "$base_cmp" >"$base_cmp.t" || true; mv "$base_cmp.t" "$base_cmp"; echo "note: wasm32 digest not compared" ;;
    --no-x86) grep -v ' compute/digest.x86_64.txt$' "$base_cmp" >"$base_cmp.t" || true; mv "$base_cmp.t" "$base_cmp"; echo "note: x86_64 digest not compared" ;;
    --no-gallery) grep -v ' gallery/' "$base_cmp" >"$base_cmp.t" || true; mv "$base_cmp.t" "$base_cmp"; echo "note: gallery not compared" ;;
  esac
done

if diff -u "$base_cmp" "$dir/manifest.sha256" >"$dir.diff"; then
  echo "compare: $(wc -l <"$base_cmp" | tr -d ' ') items byte-identical to the baseline"
  rm -f "$dir.diff"
  exit 0
fi
echo "compare: DIFFERENT (regenerated tree: $dir; diff: $dir.diff)" >&2
grep -E '^[-+][0-9a-f]{64}' "$dir.diff" | awk '{print $1 "  " $2}' | sed 's/^\([-+]\)[0-9a-f]*/\1/' >&2
exit 1
