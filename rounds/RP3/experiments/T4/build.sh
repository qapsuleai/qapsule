#!/usr/bin/env bash
# T4: builds the demo package (as scripts/demo.sh does) and the release `qp`, without
# starting the demo.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
cd "$root"
CARGO_PROFILE_WASM_STRIP=true "$HOME/.cargo/bin/wasm-pack" build crates/qp-wasm --target web \
  --profile wasm --no-opt --out-dir ../../shells/web/pkg -- --features replica
rm -f shells/web/pkg/.gitignore shells/web/pkg/package.json shells/web/pkg/README.md
raw="$(wc -c <shells/web/pkg/qp_wasm_bg.wasm | tr -d ' ')"
printf '{"wasm_raw":%s,"wasm_gzip":0,"js_raw":0,"js_gzip":0}\n' "$raw" >shells/web/pkg/sizes.json
cargo build -q -p qp-cli --release
