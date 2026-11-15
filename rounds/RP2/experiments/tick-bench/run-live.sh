#!/usr/bin/env bash
# RP2/T5: per-tick cost of the live path, after (this tree) and before (the code at
# BEFORE_REV, default a310186 = rp2/int before T5, i.e. T1/T4 behaviour), native and
# wasm32-wasip1 under Node (opt-s and release). Raw outputs: ../raw/t5/.
# Env: BARS (10000) TICKS (600) REPS (15) STUDIES (5,50,500) RUNS (2) BEFORE_REV (a310186)
#      BEFORE (1: also measure before; the before session at N=500 `all` is slow, so the
#      before run measures the t6d and base layouts only, as T1 did for its ratio table).
set -euo pipefail
cd "$(dirname "$0")"
OUT=../raw/t5
mkdir -p "$OUT"
BARS=${BARS:-10000}; TICKS=${TICKS:-600}; REPS=${REPS:-15}; STUDIES=${STUDIES:-5,50,500}
RUNS=${RUNS:-2}; BEFORE_REV=${BEFORE_REV:-a310186}; BEFORE=${BEFORE:-1}
build() { # dir
  (cd "$1" && cargo build --release --bin live && cargo build --release --target wasm32-wasip1 --bin live \
    && cargo build --profile wasm --target wasm32-wasip1 --bin live)
}
run() { # dir tag watches reps
  local d=$1 tag=$2 w=$3 reps=$4
  for r in $(seq 1 "$RUNS"); do
    "$d/target/release/live" "$BARS" "$TICKS" "$reps" "$STUDIES" inline,bound "$w" > "$OUT/$tag-native-run$r.txt"
    node --no-warnings run-wasi.mjs "$d/target/wasm32-wasip1/wasm/live.wasm" "$BARS" "$TICKS" "$reps" "$STUDIES" inline,bound "$w" \
      > "$OUT/$tag-wasm-opt-s-run$r.txt" 2>/dev/null
    node --no-warnings run-wasi.mjs "$d/target/wasm32-wasip1/release/live.wasm" "$BARS" "$TICKS" "$reps" "$STUDIES" inline,bound "$w" \
      > "$OUT/$tag-wasm-release-run$r.txt" 2>/dev/null
  done
}
{
  echo "cpu = $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m)"
  echo "os = $(uname -sr)"; echo "rustc = $(rustc --version)"; echo "node = $(node --version)"
  echo "date = $(date -u +%Y-%m-%dT%H:%M:%SZ)"; echo "after = $(git rev-parse --short HEAD)"
  echo "args = bars $BARS ticks $TICKS reps $REPS studies $STUDIES runs $RUNS before $BEFORE_REV"
} > "$OUT/env.txt"
build .
run . after t6d,base,all "$REPS"
if [ "$BEFORE" = 1 ]; then
  tmp="$(mktemp -d)"
  git worktree add --detach "$tmp/before" "$BEFORE_REV" > /dev/null
  b="$tmp/before/rounds/RP2/experiments/tick-bench"
  mkdir -p "$b/src/bin"; cp src/bin/live.rs "$b/src/bin/"; cp Cargo.toml "$b/"
  build "$b"
  run "$b" before t6d,base 3
  git worktree remove --force "$tmp/before"
fi
