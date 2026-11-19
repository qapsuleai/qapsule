#!/usr/bin/env bash
# R6: run `qp live …` with the live Matriks connection variables (K61) loaded from the
# environment or, if unset, from ./.env.local (never printed, never on a command line).
# Usage: scripts/live.sh record --symbols THYAO,GARAN,ASELS --seconds 120
#        scripts/live.sh inspect fixtures/local/live/raw/<file>.qpraw
# Only the orchestrator opens a live connection, one at a time (the lock file under
# fixtures/local/ refuses a second process anyway). Log every attempt in
# rounds/R6/experiments/LIVE-LOG.md.
set -euo pipefail
cd "$(dirname "$0")/.."
if [ -z "${MATRIKS_TOKEN_SERVICE_URL:-}" ] && [ -f ./.env.local ]; then
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
