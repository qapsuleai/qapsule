#!/usr/bin/env bash
# Reproduces every number in crates/qp-render/SPIKE.md (R0/T3 render-stack experiment).
# Not part of verification: needs a GPU (Metal/Vulkan/DX12), wasm-pack, node, Playwright's
# full Chromium (WebGPU) and, for the cross-arch rows, the x86_64-apple-darwin target +
# Rosetta. Output: target/spikes/report/*.txt (and PNGs in target/spikes/).
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
out="$root/target/spikes/report"
mkdir -p "$out"
cd "$root"
wasm_pack="$(command -v wasm-pack || echo "$HOME/.cargo/bin/wasm-pack")"

step() { echo "== $1"; }

step "native: reference stack (tiny-skia + fontdue)"
cargo run -q --release -p qp-render --example bench | tee "$out/ref-aarch64-scalar.txt"
cargo run -q --release -p qp-render --example bench --features simd | tee "$out/ref-aarch64-simd.txt"
if rustup target list --installed | grep -q x86_64-apple-darwin; then
  cargo run -q --release -p qp-render --example bench --target x86_64-apple-darwin \
    | tee "$out/ref-x86_64-scalar.txt"
  RUSTFLAGS="-C target-cpu=haswell" CARGO_TARGET_DIR="$root/target/haswell" \
    cargo run -q --release -p qp-render --example bench --features simd \
    --target x86_64-apple-darwin | tee "$out/ref-x86_64-avx2-simd.txt"
fi

step "native: vello_cpu"
cargo run -q --release -p qp-render --example spike_vello_cpu --features spike-vello-cpu-mt \
  | tee "$out/vello-cpu-aarch64.txt"
if rustup target list --installed | grep -q x86_64-apple-darwin; then
  cargo run -q --release -p qp-render --example spike_vello_cpu \
    --features spike-vello-cpu-mt --target x86_64-apple-darwin | tee "$out/vello-cpu-x86_64.txt"
  for c in speed-u8_1T quality-f32_1T; do
    for s in sample stress; do
      echo "$c $s aarch64-vs-x86_64: $(node "$here/pngdiff.mjs" \
        "target/spikes/vello_cpu_aarch64_${c}_$s.png" "target/spikes/vello_cpu_x86_64_${c}_$s.png")"
    done
  done | tee "$out/vello-cpu-cross-arch.txt"
fi

step "native: vello GPU (wgpu)"
cargo run -q --release -p qp-render --example spike_vello_gpu --features spike-vello-gpu \
  | tee "$out/vello-gpu-native.txt"

step "wasm builds"
build() { # $1 out-dir name, $2 features, $3 extra RUSTFLAGS
  RUSTFLAGS="${3:-}" CARGO_TARGET_DIR="$root/target/wasm-$1" "$wasm_pack" build crates/qp-wasm \
    --target web --profile wasm --no-opt --out-dir "../../target/web-spike/$1" \
    -- --features "$2" >/dev/null 2>&1 || echo "build $1 FAILED"
}
build pkg-simd simd "-C target-feature=+simd128"
build pkg-vello-cpu spike-vello-cpu
build pkg-vello-gpu-only spike-vello-gpu
build pkg-vello-gpu spike-vello-cpu,spike-vello-gpu
[ -f shells/web-spike/pkg/qp_wasm_bg.wasm ] || "$here/run.sh" >/dev/null

step "wasm sizes"
opt="$root/target/web-spike/size.opt.wasm"
for d in shells/web-spike/pkg target/web-spike/pkg-simd target/web-spike/pkg-vello-cpu \
  target/web-spike/pkg-vello-gpu-only target/web-spike/pkg-vello-gpu; do
  w="$d/qp_wasm_bg.wasm"
  "$here/node_modules/.bin/wasm-opt" -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
    --enable-sign-ext --enable-mutable-globals --enable-simd --enable-reference-types \
    --strip-debug --strip-producers "$w" -o "$opt"
  echo "$d raw=$(wc -c <"$w") gzip9=$(gzip -9c "$w" | wc -c) wasm-opt-Oz=$(wc -c <"$opt") Oz+gzip9=$(gzip -9c "$opt" | wc -c)"
done | tee "$out/wasm-sizes.txt"

step "browser compares"
cd "$here"
node compare.mjs > "$out/browser-reference.txt" 2>&1
QP_WEB_PKG=../../target/web-spike/pkg-simd node compare.mjs > "$out/browser-simd128.txt" 2>&1
QP_WEB_PKG=../../target/web-spike/pkg-vello-cpu node compare.mjs > "$out/browser-vello-cpu.txt" 2>&1
QP_WEB_WEBGPU=1 QP_WEB_PKG=../../target/web-spike/pkg-vello-gpu node compare.mjs \
  > "$out/browser-vello-gpu-webgpu.txt" 2>&1
grep -h -E "OK:|FAIL" "$out"/browser-*.txt
echo "reports in $out"
