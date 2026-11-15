#!/usr/bin/env bash
# R3/M7 measurement: bs2 and its ablations on AUD1–AUD3 + M7X, bs2 and bs2n on S0–S19
# (+ extras), extra counters of bs2; tables and compacted results into raw/bs2/.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$here/harness"
cargo build --release
out="$here/raw/bs2"
mkdir -p "$out"
specs=(--spec ../specs/aud1.json --spec ../specs/aud2.json --spec ../specs/aud3.json --spec ../specs/m7x.json)
for ap in bs bs2 bs2n bs2h bs2d; do
  ./target/release/harness run --approach "$ap" "${specs[@]}" --seeds 100 --out "$out/$ap-aud.json" >"$out/table-$ap-aud.txt" 2>&1
done
for ap in bs2 bs2n; do
  ./target/release/harness run --approach "$ap" --scenario all --out "$out/$ap-all.json" >"$out/table-$ap-all.txt" 2>&1
done
./target/release/m7stats "${specs[@]}" --seeds 100 >"$out/stats-bs2-aud.txt" 2>/dev/null
./target/release/m7stats --scenario all --seeds 100 >"$out/stats-bs2-all.txt" 2>/dev/null
for f in "$out"/*.json; do
  case "$f" in *.compact.json) continue ;; esac
  python3 -I "$here/tools/compact.py" "$f" "${f%.json}.compact.json"
  rm "$f"
done
echo done
