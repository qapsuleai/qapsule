#!/usr/bin/env bash
# RP4/D1: sequential, paced, cached REST fetch for the intraday calibration (orchestrator
# only; one process holds the Matriks identity). Every request is counted in
# fixtures/local/trades/requests.jsonl and fixtures/local/live/rest/requests.jsonl; a file
# already in the cache is never fetched again. Ranges: rounds/RP4/experiments/SPLIT.md.
# Usage: scripts/rp4-fetch.sh [phase...]   phases: trades bars5 bars1h bars1d m27 scale
set -euo pipefail
cd "$(dirname "$0")/.."
L=scripts/live.sh
U100=@fixtures/local/live/u100.txt
U30=@fixtures/local/live/u30.txt
phases=("$@"); [ ${#phases[@]} -eq 0 ] && phases=(trades bars5 bars1h bars1d m27)
# 90-day windows covering the 5-minute calibration + test range.
W5=("2025-11-03 2026-01-31" "2026-02-01 2026-04-30" "2026-05-01 2026-07-31" "2026-08-01 2026-10-31" "2026-11-01 2026-11-20")
WT=("2026-10-05 2026-10-18" "2026-10-19 2026-11-01" "2026-11-02 2026-11-15" "2026-11-16 2026-11-20")
for p in "${phases[@]}"; do
  echo "== phase $p $(date +%H:%M:%S)"
  case "$p" in
    trades) "$L" trades --symbols "$U100" --start 2026-10-05 --end 2026-11-20 --window-days 14 ;;
    bars5) for w in "${W5[@]}"; do set -- $w; "$L" bars --symbols "$U100" --period 5min --start "$1" --end "$2" | tail -1; done ;;
    bars1h) "$L" bars --symbols "$U100" --period 1hour --start 2025-11-03 --end 2026-11-20 | tail -1 ;;
    bars1d) "$L" bars --symbols "$U100" --period 1day --start 2018-01-01 --end 2026-11-20 | tail -1 ;;
    m27)
      for w in "${WT[@]}"; do set -- $w; "$L" bars --symbols "$U30" --period 1min --start "$1" --end "$2" --unadjusted | tail -1; done
      for per in 5min 1hour 1day; do "$L" bars --symbols "$U30" --period "$per" --start 2026-10-05 --end 2026-11-20 --unadjusted | tail -1; done ;;
    scale) "$L" trades --symbols @fixtures/local/live/utum.txt --start 2026-11-20 --end 2026-11-20 --window-days 1 ;;
    *) echo "unknown phase $p" >&2; exit 2 ;;
  esac
done
echo "== done $(date +%H:%M:%S)"
