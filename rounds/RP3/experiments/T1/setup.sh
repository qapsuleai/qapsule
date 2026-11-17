#!/usr/bin/env bash
# RP3/T1: builds the WASM variants and the served shell directories of the calibration
# (rounds/RP3/CALIBRATION.md). Each web-<name>/ is a copy of shells/web's page (index.html,
# style.css, main.js — or the probe copy) with its own pkg/. Generated, not committed.
#
#   rounds/RP3/experiments/T1/setup.sh [base|simd|opt|probe|probe-simd ...]   (default: all)
#
#   base        the demo build (scripts/demo.sh): profile wasm, feature replica, stripped,
#               no wasm-opt, no SIMD
#   simd        base + `--features simd` + RUSTFLAGS="-C target-feature=+simd128"
#   opt         base after `wasm-opt -O3` (binaryen from wasm-pack's cache, no download)
#   probe       the probe core (probe/: R4 binding + probe_* calls) + main-probe.js
#   probe-simd  probe with SIMD
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../../../.." && pwd)"
cd "$root"
wasm_pack="$(command -v wasm-pack || echo "$HOME/.cargo/bin/wasm-pack")"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/target}"
export CARGO_PROFILE_WASM_STRIP=true
wasm_opt="$(ls "$HOME"/Library/Caches/.wasm-pack/wasm-opt-*/bin/wasm-opt 2>/dev/null | head -1 || true)"

page() { # dir main.js
  mkdir -p "$1"
  cp shells/web/index.html shells/web/style.css "$1/"
  cp "$2" "$1/main.js"
  rm -f "$1/pkg/.gitignore" "$1/pkg/package.json" "$1/pkg/README.md"
  local w="$1/pkg/qp_wasm_bg.wasm"
  printf '{"wasm_raw":%s,"wasm_gzip":%s,"js_raw":%s,"js_gzip":%s}\n' \
    "$(wc -c <"$w" | tr -d ' ')" "$(gzip -9 -c "$w" | wc -c | tr -d ' ')" \
    "$(cat "$1/main.js" "$1/pkg/qp_wasm.js" | wc -c | tr -d ' ')" \
    "$(cat "$1/main.js" "$1/pkg/qp_wasm.js" | gzip -9 -c | wc -c | tr -d ' ')" >"$1/pkg/sizes.json"
  echo "== $1: $(cat "$1/pkg/sizes.json")"
}

demo_build() { # out-dir [extra cargo args]
  local out="$1"; shift
  "$wasm_pack" build crates/qp-wasm --target web --profile wasm --no-opt --out-dir "$out" -- --features replica "$@"
}

probe_build() { # out-dir [extra cargo args]
  local out="$1"; shift
  (cd "$here/probe" && "$wasm_pack" build . --target web --profile wasm --no-opt --out-dir "$out" --out-name qp_wasm -- "$@")
}

for v in "${@:-base simd opt probe probe-simd}"; do for v in $v; do
  case "$v" in
    base) demo_build "$here/web-base/pkg"; page "$here/web-base" shells/web/main.js ;;
    simd) RUSTFLAGS="-C target-feature=+simd128" demo_build "$here/web-simd/pkg" --features simd
          page "$here/web-simd" shells/web/main.js ;;
    opt)
      [ -x "$wasm_opt" ] || { echo "setup: wasm-opt not found in wasm-pack's cache; skipping opt" >&2; continue; }
      [ -f "$here/web-base/pkg/qp_wasm_bg.wasm" ] || { demo_build "$here/web-base/pkg"; page "$here/web-base" shells/web/main.js; }
      mkdir -p "$here/web-opt/pkg"
      cp "$here/web-base/pkg/qp_wasm.js" "$here/web-opt/pkg/"
      "$wasm_opt" -O3 --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
        --enable-mutable-globals --enable-reference-types --enable-multivalue \
        "$here/web-base/pkg/qp_wasm_bg.wasm" -o "$here/web-opt/pkg/qp_wasm_bg.wasm"
      page "$here/web-opt" shells/web/main.js ;;
    probe) probe_build "$here/web-probe/pkg"; page "$here/web-probe" "$here/main-probe.js" ;;
    probe-simd) RUSTFLAGS="-C target-feature=+simd128" probe_build "$here/web-probe-simd/pkg" --features simd
                page "$here/web-probe-simd" "$here/main-probe.js" ;;
    *) echo "setup: unknown variant $v" >&2; exit 2 ;;
  esac
done; done
