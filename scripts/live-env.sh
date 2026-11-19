#!/usr/bin/env bash
# R6/T0: check that the live Matriks connection variables (K61, docs/DATA_ACCESS.md
# "Canlı bağlantı değişkenleri") are PRESENT. Values are never printed: only each name and
# "set" / "MISSING" (and, for MATRIKS_MQTT_MODE, which of the two allowed words it is).
#
# The variables come from the environment; if a required one is unset and ./.env.local
# exists, that file is loaded (same rule as scripts/fetch-bars.sh). The file is never
# printed or copied.
#
# Exit: 0 all required present; 3 something required is missing (names listed).
set -euo pipefail
cd "$(dirname "$0")/.."

REQUIRED=(MATRIKS_TOKEN_SERVICE_URL MATRIKS_API_USERNAME MATRIKS_API_PASSWORD
  MATRIKS_SERVICE_TYPE_CODE MATRIKS_MQTT_MODE)
# One of the two market endpoints is required, depending on the mode.
OPTIONAL=(MATRIKS_HARDWARE_ID MATRIKS_MQTT_RT_HOST MATRIKS_MQTT_DL_HOST
  MATRIKS_MQTT_STATS_HOST MATRIKS_MQTT_NEWS_HOST MATRIKS_TOKEN)

need_load=0
for v in "${REQUIRED[@]}"; do [ -z "${!v:-}" ] && need_load=1; done
if [ "$need_load" = 1 ] && [ -f ./.env.local ]; then
  set -a
  # shellcheck disable=SC1091
  . ./.env.local >/dev/null 2>&1
  set +a
fi

missing=()
for v in "${REQUIRED[@]}" "${OPTIONAL[@]}"; do
  if [ -n "${!v:-}" ]; then echo "  $v: set"; else echo "  $v: MISSING"; fi
done
for v in "${REQUIRED[@]}"; do [ -z "${!v:-}" ] && missing+=("$v"); done

mode="${MATRIKS_MQTT_MODE:-}"
case "$mode" in
  delayed) host_var=MATRIKS_MQTT_DL_HOST ;;
  realtime) host_var=MATRIKS_MQTT_RT_HOST ;;
  "") host_var="" ;;
  *) echo "live-env: MATRIKS_MQTT_MODE must be 'realtime' or 'delayed'" >&2; exit 3 ;;
esac
[ -n "$mode" ] && echo "  mode: $mode"
if [ -n "$host_var" ] && [ -z "${!host_var:-}" ]; then missing+=("$host_var"); fi
if [ -z "${MATRIKS_HARDWARE_ID:-}" ]; then
  echo "  note: MATRIKS_HARDWARE_ID empty -> derived as qapsule-<host name> (deterministic)"
fi

if [ "${#missing[@]}" -gt 0 ]; then
  echo "live-env: missing: ${missing[*]} (enter them in .env.local; see docs/DATA_ACCESS.md)" >&2
  exit 3
fi
echo "live-env: OK"
