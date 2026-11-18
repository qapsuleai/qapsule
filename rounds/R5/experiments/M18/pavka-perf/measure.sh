#!/usr/bin/env bash
# K52 for the four builds, M18's method (glance.py k52 --charts, k52chart.mjs), one session.
set -uo pipefail
S="$(cd "$(dirname "$0")" && pwd)"
H="$S/trees/r4/rounds/R5/experiments/harness/glance"
export QP_DEMO_PORT=8747
( while true; do echo "$(date +%H:%M:%S) $(uptime | sed 's/.*load averages*: //')"; sleep 30; done ) > "$S/load.log" 2>&1 &
sampler=$!
one() { # run-dir config reps name
  python3 -c "import json,sys; d=[x for x in json.load(open('$S/designs.json')) if x['name']=='$4']; json.dump(d, open('$S/d-$4.json','w'))"
  echo "== $(date +%H:%M:%S) $(uptime | sed 's/.*load averages*: //') $4 $2 reps $3"
  python3 "$H/glance.py" k52 --run "$S/$1" --designs "$S/d-$4.json" --charts --configs "$2" --reps "$3"
}
for n in pavka-old pavka-new korda r4; do one run-phone phone 5 "$n"; done
for n in pavka-old pavka-new korda r4; do one run-jit phone-jit-est 3 "$n"; done
echo "== $(date +%H:%M:%S) $(uptime | sed 's/.*load averages*: //') done"
kill $sampler
