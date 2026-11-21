#!/usr/bin/env bash
# R6: run `qp live …` with the live Matriks connection variables (K61) loaded from the
# environment or, if unset, from ./.env.local (never printed, never on a command line).
# Usage: scripts/live.sh record --symbols THYAO,GARAN,ASELS --seconds 120
#        scripts/live.sh inspect fixtures/local/live/raw/<file>.qpraw
#        scripts/live.sh record --trades --symbols THYAO,GARAN,ASELS --seconds 120
#          (RP4, K66: the trade endpoint `/trade` only, its own lock; `mx/trade/<SYM>`,
#          level 1; `--tradeex` adds `mx/tradeex/<SYM>`; `@lvl2` is refused. Raw log
#          fixtures/local/live/raw/<UTC time>-trade.qpraw without buyer / seller.)
#        scripts/live.sh inspect fixtures/local/live/raw/<file>-trade.qpraw
#          (trade counts, field presence, time-unit estimate, arrival lags, side values;
#          no price, no trade number.)
#        scripts/live.sh reset --endpoint delayed-market --note "<who, what was checked>"
#          (RP4/T5, K64: lifts a `.halted` endpoint; a PERSON runs it after checking why —
#          an agent never does on its own. Asks on a terminal, else needs --i-am-human.
#          No network, no connection variables.)
# Only the orchestrator opens a live connection, one at a time (the lock file under
# fixtures/local/ refuses a second process anyway). Log every attempt in
# rounds/R6/experiments/LIVE-LOG.md.
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "${1:-}" != "reset" ] && [ -z "${MATRIKS_TOKEN_SERVICE_URL:-}" ] && [ -f ./.env.local ]; then
  set -a
  # shellcheck disable=SC1091
  . ./.env.local >/dev/null 2>&1
  set +a
fi
qp="${QP_BIN:-}"
if [ -z "$qp" ]; then
  cargo build -q --release -p qp-cli
  qp="${CARGO_TARGET_DIR:-target}/release/qp"
fi
exec "$qp" live "$@"
