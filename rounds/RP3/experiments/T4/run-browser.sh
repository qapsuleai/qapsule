#!/usr/bin/env bash
# T4: the browser memory runs (mem.mjs), 3 repetitions each, raw lines into raw/<tag>-*.jsonl.
#   run-browser.sh <tag> [variants...]   e.g. run-browser.sh before feed chart scroll
# Variants: feed chart scroll reconnect webkit-feed. Ports 8482-8489 (one per variant).
# WEB=DIR serves the page from DIR (a patched copy of shells/web with its own pkg).
set -uo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
cd "$root"
tag="$1"; shift
out=rounds/RP3/experiments/T4/raw
mkdir -p "$out"
port=8482
for v in "$@"; do
  (
    b=chromium; var="$v"
    case "$v" in webkit-*) b=webkit; var="${v#webkit-}" ;; esac
    for rep in 1 2 3; do
      echo "{\"load\":\"$(uptime | sed 's/.*load averages*: //')\",\"rep\":$rep}" >>"$out/$tag-$v.jsonl"
      node rounds/RP3/experiments/T4/mem.mjs target/release/qp --browser "$b" --variant "$var" \
        --steps "${STEPS:-5000}" --every 250 --speed "${SPEED:-64}" --port "$port" \
        --web "${WEB:-$root/shells/web}" >>"$out/$tag-$v.jsonl" 2>&1
    done
  ) &
  port=$((port + 1))
done
wait
