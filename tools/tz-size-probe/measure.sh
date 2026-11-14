#!/usr/bin/env bash
# Builds each probe for wasm32-unknown-unknown (opt-level s, fat LTO) and prints raw and
# gzip -9 sizes, plus the delta over the empty `base` probe.
set -euo pipefail
cd "$(dirname "$0")"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-../../target/tz-size-probe}"
cargo build -q --release --target wasm32-unknown-unknown --workspace
base_raw=0; base_gz=0
printf '%-12s %10s %10s %10s %10s\n' probe raw gzip9 d_raw d_gzip9
for p in base rules table jiff_static chrono_tz jiff_bundle; do
  f="$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/probe_${p}.wasm"
  raw=$(wc -c <"$f" | tr -d ' '); gz=$(gzip -9 -c "$f" | wc -c | tr -d ' ')
  if [ "$p" = base ]; then base_raw=$raw; base_gz=$gz; fi
  printf '%-12s %10s %10s %10s %10s\n' "$p" "$raw" "$gz" $((raw-base_raw)) $((gz-base_gz))
done
