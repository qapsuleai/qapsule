#!/usr/bin/env bash
# build.sh <tree dir> <name>: qp release + the web shell's wasm package (as scripts/demo.sh)
set -euo pipefail
tree="$1"; name="$2"
export CARGO_TARGET_DIR=/Users/arkns/Works/Mchart/qapsule/target/wt/m18-pp-$name
cd "$tree"
QP_DEMO_PACK_ONLY=1 QP_DEMO_REBUILD=1 bash scripts/demo.sh
cargo build -q -p qp-cli --release
ls -la "$CARGO_TARGET_DIR/release/qp" shells/web/pkg/qp_wasm_bg.wasm
echo "built $name"
