#!/usr/bin/env bash
# Performance and size record for refactoring rounds (RG1, RF1: K17, K52, WASM ±2 %).
#
# Usage: scripts/refactor/perf.sh <out-file>
#   K17  qp-compute sweep_bench (10,000 bars x 1,000 parameter sets, single thread,
#        median of 15 after a warm-up; profiles off)
#   K52  shells/web/tests/metrics.mjs at CPU throttling 4 (headless Chromium, phone page):
#        the metrics page summary (tick, face and scene paint p95, long frames)
#   WASM verify.sh step 8 build (profile wasm): raw and gzip -9 bytes; the shell package
#        (wasm-pack web, --features replica, as in verify.sh step 18)
# Timing numbers are noisy; the round compares them by eye within noise (GOAL.md).
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
out="${1:?usage: perf.sh <out-file>}"
target_dir="${CARGO_TARGET_DIR:-$root/target}"
case "$target_dir" in /*) ;; *) target_dir="$root/$target_dir" ;; esac
qp="$target_dir/release/qp"
{
  echo "# refactor-perf $(date -u +%Y-%m-%dT%H:%MZ) $(uname -sm) commit $(git rev-parse --short HEAD)"
  echo "## K17 sweep_bench"
  cargo run -q --release -p qp-compute --example sweep_bench -- --reps 15 --profiles none
  echo "## WASM (verify step 8)"
  cargo build -q -p qp-wasm --target wasm32-unknown-unknown --profile wasm
  w="$target_dir/wasm32-unknown-unknown/wasm/qp_wasm.wasm"
  echo "raw=$(wc -c <"$w" | tr -d ' ') gzip9=$(gzip -9 -c "$w" | wc -c | tr -d ' ')"
  echo "## WASM shell package (web, replica)"
  wp="$(command -v wasm-pack || echo "$HOME/.cargo/bin/wasm-pack")"
  pkg="$root/shells/web/pkg"
  "$wp" build crates/qp-wasm --target web --profile wasm --no-opt --out-dir ../../shells/web/pkg -- --features replica >/dev/null 2>&1
  rm -f "$pkg/.gitignore" "$pkg/package.json" "$pkg/README.md"
  echo "raw=$(wc -c <"$pkg/qp_wasm_bg.wasm" | tr -d ' ') gzip9=$(gzip -9 -c "$pkg/qp_wasm_bg.wasm" | wc -c | tr -d ' ')"
  echo "## K52 metrics.mjs (CPU x4)"
  cargo build -q -p qp-cli --release
  (cd shells/web && node tests/metrics.mjs "$qp" 4 2>&1)
} >"$out"
echo "perf -> $out"
