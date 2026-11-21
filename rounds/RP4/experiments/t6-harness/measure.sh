#!/bin/bash
# RP4/T6 measurement: qp demo --sim at 1x from 11:00, one headless viewer, samples ps.
# usage: measure.sh <qp> <symbols-file> <label> <secs> <port>
#   e.g. measure.sh target/release/qp fixtures/local/live/utum.txt after-580 1290 8476
# Then: python3 summ.py <label>... (reads $QP_T6_OUT, default fixtures/local/t6).
set -u
QP=$1; SYMS=$2; LABEL=$3; SECS=$4; PORT=$5
ROOT=${QP_ROOT:-$(cd "$(dirname "$0")/../../../.." && pwd)}
HERE=$(cd "$(dirname "$0")" && pwd)
# Counts derived from real data stay on this machine (fixtures/local, not committed).
OUT=${QP_T6_OUT:-$ROOT/fixtures/local/t6}/$LABEL; mkdir -p "$OUT"; rm -rf "$OUT/prof"
QP_LIVE_CONTROL=$(od -An -N16 -tx1 /dev/urandom | tr -d " \n"); export QP_LIVE_CONTROL
cd $ROOT
uptime > "$OUT/uptime-start.txt"
"$QP" demo --sim --symbols "@$SYMS" --start 2026-11-20 --sim-at 11:00 --sim-speed 1 --interval 5m --port $PORT > "$OUT/qp.log" 2>&1 &
QPPID=$!
echo $QPPID > "$OUT/pid"
for i in $(seq 1 600); do grep -q "127.0.0.1:$PORT\|localhost:$PORT" "$OUT/qp.log" && break; sleep 1; done
T0=$(date +%s)
echo "ready_after_s $i" > "$OUT/meta.txt"
node $HERE/viewer.mjs "http://127.0.0.1:$PORT" $((SECS-30)) 60 "$OUT/prof" > "$OUT/viewer.log" 2>&1 &
VPID=$!
: > "$OUT/ps.tsv"
while kill -0 $QPPID 2>/dev/null; do
  NOW=$(date +%s); EL=$((NOW-T0))
  echo -e "$EL\t$(ps -o rss=,time=,%cpu= -p $QPPID)\t$(uptime | sed 's/.*averages: //')" >> "$OUT/ps.tsv"
  if [ $((EL % 300)) -lt 10 ] && [ $EL -gt 5 ]; then
    curl -s -H "X-Qp-Live-Control: $QP_LIVE_CONTROL" localhost:$PORT/live/stats > "$OUT/stats-$EL.json"
  fi
  [ $EL -ge $SECS ] && break
  sleep 10
done
curl -s -H "X-Qp-Live-Control: $QP_LIVE_CONTROL" localhost:$PORT/live/stats > "$OUT/stats-end.json"
vmmap --summary $QPPID 2>/dev/null | grep -E "Physical footprint|DefaultMallocZone|Malloc Large|Malloc Small" > "$OUT/vmmap-end.txt"
uptime > "$OUT/uptime-end.txt"
wait $VPID 2>/dev/null
curl -s -X POST -H "X-Qp-Live-Control: $QP_LIVE_CONTROL" localhost:$PORT/live/quit >/dev/null
sleep 3; kill $QPPID 2>/dev/null; sleep 1; kill -9 $QPPID 2>/dev/null
rm -rf "$OUT/prof"
echo done
