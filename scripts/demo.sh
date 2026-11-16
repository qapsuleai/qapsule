#!/usr/bin/env bash
# R4/T5: the demo in one command (shells/web/README.md).
#
#   scripts/demo.sh [qp demo options]     e.g. --synthetic, --data DIR, --from 2025-06-02,
#                                         --speed 8, --port 8390, --paused, --lan
#
#   --lan  listens on 0.0.0.0 so that a phone on the same network can open the page
#          (printed address). Devices on the local network can then see the demo.
#          Without it the demo listens on 127.0.0.1 (this machine only).
#
# 1. builds the web shell's core (qp-wasm, feature `replica`) with wasm-pack into
#    shells/web/pkg (skipped when up to date; QP_DEMO_REBUILD=1 forces it) and writes
#    shells/web/pkg/sizes.json (raw and gzip -9 sizes, for the metrics page);
# 2. builds `qp` (release) and runs `qp demo`: a local host on 127.0.0.1:8390 that serves
#    the page and the protocol on one origin. Without local data (fixtures/local/bars or
#    QAPSULE_LOCAL_BARS) it runs the synthetic market and the page says so.
# Requirements: rustup target wasm32-unknown-unknown, wasm-pack (cargo install wasm-pack).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
target_dir="${CARGO_TARGET_DIR:-$root/target}"
case "$target_dir" in /*) ;; *) target_dir="$root/$target_dir" ;; esac

pkg=shells/web/pkg
wasm_pack="$(command -v wasm-pack || echo "$HOME/.cargo/bin/wasm-pack")"
[ -x "$wasm_pack" ] || { echo "demo.sh: wasm-pack not found (cargo install wasm-pack)" >&2; exit 2; }

stale=1
if [ -f "$pkg/qp_wasm_bg.wasm" ] && [ "${QP_DEMO_REBUILD:-0}" != 1 ]; then
  newer="$(find crates -name '*.rs' -newer "$pkg/qp_wasm_bg.wasm" -print -quit 2>/dev/null || true)"
  [ -z "$newer" ] && stale=0
fi
if [ "$stale" = 1 ]; then
  echo "== building the shell core (wasm-pack, profile wasm, feature replica)"
  "$wasm_pack" build crates/qp-wasm --target web --profile wasm --no-opt \
    --out-dir ../../shells/web/pkg -- --features replica
  rm -f "$pkg/.gitignore" "$pkg/package.json" "$pkg/README.md"
fi
raw="$(wc -c <"$pkg/qp_wasm_bg.wasm" | tr -d ' ')"
gz="$(gzip -9 -c "$pkg/qp_wasm_bg.wasm" | wc -c | tr -d ' ')"
js="$(cat shells/web/main.js "$pkg/qp_wasm.js" | wc -c | tr -d ' ')"
jsgz="$(cat shells/web/main.js "$pkg/qp_wasm.js" | gzip -9 -c | wc -c | tr -d ' ')"
printf '{"wasm_raw":%s,"wasm_gzip":%s,"js_raw":%s,"js_gzip":%s}\n' "$raw" "$gz" "$js" "$jsgz" >"$pkg/sizes.json"
echo "== sizes: $(cat "$pkg/sizes.json")"

cargo build -q -p qp-cli --release
exec "$target_dir/release/qp" demo --web shells/web "$@"
