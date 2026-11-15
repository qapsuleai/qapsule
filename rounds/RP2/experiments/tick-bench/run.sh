#!/usr/bin/env bash
# Reproduces the RP2/T1 baseline. Writes raw outputs to rounds/RP2/experiments/raw/t1/.
# Env: BARS (10000), TICKS (600), REPS (15), STUDIES (5,50,500), RUNS (2).
set -euo pipefail
cd "$(dirname "$0")"
OUT=../raw/t1
mkdir -p "$OUT"
BARS=${BARS:-10000}; TICKS=${TICKS:-600}; REPS=${REPS:-15}; STUDIES=${STUDIES:-5,50,500}
RUNS=${RUNS:-2}
cargo build --release
cargo build --release --target wasm32-wasip1
cargo build --profile wasm --target wasm32-wasip1
{
  echo "cpu = $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m)"
  echo "os = $(uname -sr)"
  echo "rustc = $(rustc --version)"
  echo "node = $(node --version)"
  echo "date = $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "args = bars $BARS ticks $TICKS reps $REPS studies $STUDIES runs $RUNS"
} > "$OUT/env.txt"
for r in $(seq 1 "$RUNS"); do
  ./target/release/tick-bench "$BARS" "$TICKS" "$REPS" "$STUDIES" > "$OUT/native-run$r.txt"
  node --no-warnings run-wasi.mjs target/wasm32-wasip1/wasm/tick-bench.wasm "$BARS" "$TICKS" "$REPS" "$STUDIES" \
    > "$OUT/wasm-opt-s-run$r.txt" 2> "$OUT/wasm-opt-s-run$r.stderr.txt"
  node --no-warnings run-wasi.mjs target/wasm32-wasip1/release/tick-bench.wasm "$BARS" "$TICKS" "$REPS" "$STUDIES" \
    > "$OUT/wasm-release-run$r.txt" 2> "$OUT/wasm-release-run$r.stderr.txt"
done
ls -l target/wasm32-wasip1/release/tick-bench.wasm target/wasm32-wasip1/wasm/tick-bench.wasm \
  | awk '{print $5, $9}' > "$OUT/wasm-sizes.txt"
# Sampling profile of the session tick loop (macOS `sample`; skipped elsewhere).
if command -v sample > /dev/null; then
  for n in 5 500; do
    ./target/release/tick-bench profile "$n" 30000 2> "$OUT/profile-n$n.stderr.txt" &
    pid=$!
    sleep 1.5
    sample "$pid" 4 -file "$OUT/sample-n$n.txt" > /dev/null 2>&1 || true
    wait "$pid"
    python3 -I sample-summary.py "$OUT/sample-n$n.txt" > "$OUT/sample-n$n.summary.txt"
  done
fi
