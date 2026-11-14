#!/usr/bin/env bash
# Reproduces the T6d measurements. Writes raw outputs to rounds/RD1/experiments/raw/t6d/.
set -euo pipefail
cd "$(dirname "$0")"
OUT=../../rounds/RD1/experiments/raw/t6d
mkdir -p "$OUT"
BARS=${BARS:-10000}; SECS=${SECS:-60}; REPS=${REPS:-15}
cargo build --release
cargo build --release --target wasm32-wasip1
cargo build --profile wasm --target wasm32-wasip1
{
  echo "cpu = $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m)"
  echo "os = $(uname -sr)"
  echo "rustc = $(rustc --version)"
  echo "node = $(node --version)"
  echo "date = $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "$OUT/env.txt"
./target/release/t6d-handover "$BARS" "$SECS" "$REPS" > "$OUT/native-run1.txt"
./target/release/t6d-handover "$BARS" "$SECS" "$REPS" > "$OUT/native-run2.txt"
node --no-warnings run-wasi.mjs target/wasm32-wasip1/release/t6d-handover.wasm "$BARS" "$SECS" "$REPS" > "$OUT/wasm-release.txt" 2> "$OUT/wasm-release.stderr.txt"
node --no-warnings run-wasi.mjs target/wasm32-wasip1/wasm/t6d-handover.wasm "$BARS" "$SECS" "$REPS" > "$OUT/wasm-opt-s.txt" 2> "$OUT/wasm-opt-s.stderr.txt"
ls -l target/wasm32-wasip1/release/t6d-handover.wasm target/wasm32-wasip1/wasm/t6d-handover.wasm | awk '{print $5, $9}' > "$OUT/wasm-sizes.txt"
