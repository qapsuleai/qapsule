#!/usr/bin/env bash
# Builds qp-wasm for Node (wasm-pack, profile `wasm`) and runs the WASM sweep benchmark.
# Output: per-study timings and digests (must match the native sweep_bench digests).
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
out="$root/target/wasm-bench/pkg"
wasm_pack="$(command -v wasm-pack || echo "$HOME/.cargo/bin/wasm-pack")"
"$wasm_pack" build crates/qp-wasm --target nodejs --profile wasm --out-dir "$out" >/dev/null 2>&1 \
  || "$wasm_pack" build crates/qp-wasm --target nodejs --profile wasm --out-dir "$out"
node crates/qp-wasm/bench/sweep_bench.mjs "$out"
