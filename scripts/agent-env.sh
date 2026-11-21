#!/usr/bin/env bash
# R5B/T0: check that the Matriks MCP variables (K67, docs/AGENTS.md) are PRESENT.
# Values are never printed: only each name and "set" / "MISSING".
#
# The variables come from the environment; if one is unset and ./.env.local exists, that
# file is loaded (same rule as scripts/live-env.sh). The file is never printed or copied.
#
# Exit: 0 all present; 3 something is missing (names listed).
set -euo pipefail
cd "$(dirname "$0")/.."

REQUIRED=(MATRIKS_MCP_URL MATRIKS_MCP_CLIENT_ID MATRIKS_MCP_API_KEY)

need_load=0
for v in "${REQUIRED[@]}"; do [ -z "${!v:-}" ] && need_load=1; done
if [ "$need_load" = 1 ] && [ -f ./.env.local ]; then
  set -a
  # shellcheck disable=SC1091
  . ./.env.local >/dev/null 2>&1
  set +a
fi

missing=()
for v in "${REQUIRED[@]}"; do
  if [ -n "${!v:-}" ]; then echo "  $v: set"; else echo "  $v: MISSING"; missing+=("$v"); fi
done
# The URL is not a secret, but only its scheme is shown (no host or path in logs).
if [ -n "${MATRIKS_MCP_URL:-}" ]; then
  case "$MATRIKS_MCP_URL" in
    https://*) echo "  MATRIKS_MCP_URL scheme: https" ;;
    *) echo "agent-env: MATRIKS_MCP_URL must be an https:// URL" >&2; exit 3 ;;
  esac
fi
if [ "${#missing[@]}" -gt 0 ]; then
  echo "agent-env: missing: ${missing[*]} (enter them in .env.local)" >&2
  exit 3
fi
echo "agent-env: OK"
