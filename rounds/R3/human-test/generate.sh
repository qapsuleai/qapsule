#!/usr/bin/env bash
# R3/HT — insan testi paketini yeniden üretir (sabit tohum; aynı girdi → bayt düzeyinde aynı çıktı).
# Yazdıkları: pages/*.png, kitapcik-A.html, kitapcik-B.html, form.html, key.json.
# Disk az ise: CARGO_INCREMENTAL=0 (varsayılan) ve iş bitince `rm -rf gen/target`.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here/gen"
CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}" cargo build --release --quiet
./target/release/ht-gen "$here"
cd "$here"
shasum pages/*.png kitapcik-A.html kitapcik-B.html form.html key.json
