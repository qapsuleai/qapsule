#!/usr/bin/env bash
# RP3/T1: the calibration matrix (rounds/RP3/CALIBRATION.md §3). Each line one config,
# 3 runs (median), results/<name>.json. Waits for the machine's 1-minute load average to
# drop to <= 12 before each config (shared machine; up to 3 min, then runs anyway and the
# load is in the JSON). Needs setup.sh to have built the web-* directories.
#   rounds/RP3/experiments/T1/matrix.sh [name-filter-regex]
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../../../.." && pwd)"
qp="$root/target/release/qp"
mkdir -p "$here/results"
filter="${1:-.}"
cd "$root/shells/web"

wait_load() {
  for _ in $(seq 1 6); do
    l=$(uptime | sed 's/.*load averages*: //' | awk '{print int($1)}')
    [ "$l" -le 12 ] && return
    sleep 30
  done
}

run() { # name web args...
  local name="$1" web="$2"; shift 2
  [[ "$name" =~ $filter ]] || return 0
  wait_load
  echo "== $name ($(uptime | sed 's/.*load/load/'))" >&2
  node tests/calibrate.mjs "$qp" --web "$here/web-$web" --reps 3 "$@" >"$here/results/$name.json"
}

B='{"iters":8}'
run wk-nojit-dpr3        base  --engine webkit   --dpr 3 --bg 0 --jsc useJIT=false
run wk-nojit-dpr3-bg     base  --engine webkit   --dpr 3 --bg 1 --jsc useJIT=false
run wk-nojit-dpr2        base  --engine webkit   --dpr 2 --bg 0 --jsc useJIT=false
run wk-nojit-dpr1        base  --engine webkit   --dpr 1 --bg 0 --jsc useJIT=false
run wk-nojit-dpr3-simd   simd  --engine webkit   --dpr 3 --bg 0 --jsc useJIT=false
run wk-nojit-dpr3-opt    opt   --engine webkit   --dpr 3 --bg 0 --jsc useJIT=false
run wk-jit-dpr3          base  --engine webkit   --dpr 3 --bg 0 --jsc ''
run wk-jit-dpr3-bg       base  --engine webkit   --dpr 3 --bg 1 --jsc ''
run wk-jit-dpr3-simd     simd  --engine webkit   --dpr 3 --bg 0 --jsc ''
run cr-dpr3-x1           base  --engine chromium --dpr 3 --rate 1 --bg 0
run cr-dpr3-x4           base  --engine chromium --dpr 3 --rate 4 --bg 0
run cr-dpr3-x20          base  --engine chromium --dpr 3 --rate 20 --bg 0
run desktop              base  --config desktop
run probe-wk-nojit-dpr3  probe --engine webkit   --dpr 3 --bg 0 --jsc useJIT=false --bench "$B" --raw
run probe-wk-jit-dpr3    probe --engine webkit   --dpr 3 --bg 0 --jsc '' --bench "$B" --raw
run probe-cr-dpr3        probe --engine chromium --dpr 3 --rate 1 --bg 0 --bench "$B" --raw
run probe-wk-nojit-simd  probe-simd --engine webkit --dpr 3 --bg 0 --jsc useJIT=false --bench "$B" --raw
echo "== done" >&2
