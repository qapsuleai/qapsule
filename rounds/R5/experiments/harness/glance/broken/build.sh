#!/usr/bin/env bash
# R5/T2 harness: builds `qp` and the web shell's WASM package WITH the broken look
# (`look.patch`, SPEC §G8) in a throw-away copy of the repository. The broken look never
# enters the product: the patch is applied to an exported copy (git archive of HEAD), not to
# the working tree, and nothing is committed.
#
#   rounds/R5/experiments/harness/glance/broken/build.sh <out dir>
#
# Writes <out dir>/src (patched copy), <out dir>/qp (release binary) and the package in
# <out dir>/src/shells/web/pkg; prints `QP=...` and `WEB=...`. Every design of a run that
# includes the broken look is shot with this same build (same code, one extra look record).
# CARGO_TARGET_DIR: default <repo>/target/wt/t2-broken (K56: one build folder per task).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../../../../../.." && pwd)"
out="${1:?usage: build.sh <out dir>}"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
case "$out" in "$root"/*) echo "build.sh: the out dir must be outside the repository" >&2; exit 2 ;; esac
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/target/wt/t2-broken}"

rm -rf "$out/src"
mkdir -p "$out/src"
git -C "$root" archive HEAD | tar -x -C "$out/src"
patch -p1 -d "$out/src" --forward --silent < "$here/look.patch"
grep -q '"kestim"' "$out/src/crates/qp-grammar/src/look.rs" || { echo "build.sh: patch did not apply" >&2; exit 1; }

# The demo package (same profile and flags as scripts/demo.sh), then the binary.
(cd "$out/src" && QP_DEMO_PACK_ONLY=1 QP_DEMO_REBUILD=1 scripts/demo.sh >/dev/null)
(cd "$out/src" && cargo build --release -q -p qp-cli)
cp "$CARGO_TARGET_DIR/release/qp" "$out/qp"
echo "QP=$out/qp"
echo "WEB=$out/src/shells/web"
