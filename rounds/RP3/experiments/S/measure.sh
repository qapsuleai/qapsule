#!/usr/bin/env bash
# RP3/S: the merged build's paint-path comparison (criteria 2, 3). For each setting and each
# path order (forward = registry order, reverse), RUNS runs of shells/web/tests/compare.mjs
# (the page's own "Karşılaştır": idle, scroll, chart_ticks, drag; 8 s phases), one after
# the other (not in parallel), each with the machine load (`uptime`) before and after.
#
#   rounds/RP3/experiments/S/measure.sh <out dir> [settings…]
#   settings: phone (WebKit, JSC JIT off, DPR 3, 402×874 = T1's PHONE), phone_jit (the same
#   with JIT = PHONE_JIT_EST), cdp4 (Chromium headless shell, CDP 4×, DPR 3, 402×874; the
#   shell has no WebGPU, so `gpu` is not among its supported paths), cdp4full (the same in
#   the full Chromium, `--channel chromium`: WebGPU on the machine's GPU).
#   Env: RUNS (default 3), ORDERS (default "forward reverse"), QP (default target/release/qp), PORT (default 8493), PATHS
#   (default ref,canvas,cpu,gpu).
# The package must be built first (scripts/demo.sh with QP_DEMO_PACK_ONLY=1) and `qp` too.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
out="$(cd "$1" && pwd)"; shift
if [ $# -eq 0 ]; then settings=(phone phone_jit cdp4 cdp4full); else settings=("$@"); fi
runs="${RUNS:-3}"
qp="${QP:-$root/target/release/qp}"
port="${PORT:-8493}"
paths="${PATHS:-ref,canvas,cpu,gpu}"
cd "$root/shells/web"
for s in "${settings[@]}"; do
  case "$s" in
    phone) args=(--browsers webkit --jsc useJIT=false) ;;
    phone_jit) args=(--browsers webkit) ;;
    cdp4) args=(--browsers chromium --throttle 4) ;;
    cdp4full) args=(--browsers chromium --throttle 4 --channel chromium) ;;
    *) echo "unknown setting $s" >&2; exit 2 ;;
  esac
  for run in $(seq 1 "$runs"); do
    for order in ${ORDERS:-forward reverse}; do
      d="$out/$s/$order-$run"
      mkdir -p "$d"
      o=(); [ "$order" = reverse ] && o=(--order reverse)
      uptime >"$d/load-before.txt"
      node tests/compare.mjs "$qp" "${args[@]}" --dpr 3 --viewport 402x874 --port "$port" \
        --paths "$paths" ${o[@]+"${o[@]}"} --out "$d" >"$d/log.txt" 2>&1 || echo "FAILED $d" | tee -a "$out/failures.txt"
      uptime >"$d/load-after.txt"
      echo "$s $order $run done ($(cat "$d/load-after.txt" | sed 's/.*load averages*: //'))"
    done
  done
done
