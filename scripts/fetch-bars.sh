#!/usr/bin/env bash
# Fetches historical bars from the Matriks bar endpoint into fixtures/local/bars/ (K23,
# docs/DATA_ACCESS.md). Licensed market data: the output stays local and is never
# committed (fixtures/local/ is git-ignored).
#
# Usage:
#   scripts/fetch-bars.sh --symbols fixtures/local/symbols.txt --period 1day \
#       --start 2018-01-01 [--end YYYY-MM-DD] [--unadjusted] [--out DIR] [--keep-raw]
#       [--pause SECONDS]
#
# Token: MATRIKS_TOKEN from the environment; if unset and ./.env.local exists, it is
# loaded from there. The token is never printed and never put on a command line: curl
# reads the header from a config on stdin.
#
# Output per symbol: <out>/<SYM>.<period>.<adj|unadj>.json (schema `Series`, inline bars,
# `t` = bar open in UTC ms per K14, see docs/DATA_ACCESS.md "R3 doğrulamaları") and
# <SYM>.<period>.<adj|unadj>.meta.json (fetch date, request parameters, counts; no token).
# Existing output files are skipped. With --keep-raw the raw JSON responses and response
# headers are kept under <out>/../raw/ for validation. HTTP 429/5xx and suspiciously short
# 200 responses are retried; a chunk still short after the retries is kept and marked
# `short` in the meta file (a symbol listed after --start is legitimately short).
set -euo pipefail
set +x

API="https://api.matriksdata.com"
SYMBOLS=""
PERIOD=""
START=""
END=""
ADJ="adj"
OUT="fixtures/local/bars"
KEEP_RAW=0
PAUSE=1
MAX_TRIES=4
RETRY_WAIT=5

die() {
  echo "fetch-bars: $*" >&2
  exit 2
}

while [ $# -gt 0 ]; do
  case "$1" in
    --symbols) SYMBOLS="${2:-}"; shift 2 ;;
    --period) PERIOD="${2:-}"; shift 2 ;;
    --start) START="${2:-}"; shift 2 ;;
    --end) END="${2:-}"; shift 2 ;;
    --unadjusted) ADJ="unadj"; shift ;;
    --out) OUT="${2:-}"; shift 2 ;;
    --keep-raw) KEEP_RAW=1; shift ;;
    --pause) PAUSE="${2:-}"; shift 2 ;;
    -h | --help) sed -n '2,19p' "$0"; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done

[ -n "$SYMBOLS" ] || die "--symbols is required"
[ -f "$SYMBOLS" ] || die "symbols file not found: $SYMBOLS"
[ -n "$START" ] || die "--start is required"
case "$PERIOD" in
  1min | 5min | 1hour | 1day) ENDPOINT="/dumrul/v1/tick/bar.gz" ;;
  10min | 15min | 30min | 120min | 240min | weekly | monthly | yearly) ENDPOINT="/dumrul/v1/tick/derived-bar" ;;
  *) die "--period must be one of 1min 5min 1hour 1day 10min 15min 30min 120min 240min weekly monthly yearly" ;;
esac
for d in "$START" ${END:+"$END"}; do
  [[ "$d" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] || die "dates must be YYYY-MM-DD: $d"
done
command -v curl >/dev/null || die "curl is required"
command -v python3 >/dev/null || die "python3 is required"

# --- token ---------------------------------------------------------------------------
if [ -z "${MATRIKS_TOKEN:-}" ] && [ -f ./.env.local ]; then
  set -a
  # shellcheck disable=SC1091
  . ./.env.local
  set +a
fi
if [ -z "${MATRIKS_TOKEN:-}" ]; then
  echo "fetch-bars: MATRIKS_TOKEN is not set (environment or ./.env.local). Set it and rerun." >&2
  exit 3
fi

# Python helper (date chunks, response checks, conversion to `Series`). It does not
# use the token: it only reads the saved response bodies.
HELPER_PY=""
IFS= read -r -d '' HELPER_PY <<'PY' || true
"""Embedded helper of scripts/fetch-bars.sh: date chunking, checks, conversion.

Does not use the token. Usage:
  helper chunks <period> <start> <end>
  helper check <period> <cs> <ce> <today> <response>
  helper convert <sym> <period> <adj> <start> <end> <today> <chunks> <dest> <meta>
"""

import datetime as dt
import gzip
import json
import os
import sys

# Calendar days per request. With `start`/`end` the server returns the whole range (the
# `count` limit of 1000 / 1440 applies to `count` requests only; measured in R3, see
# docs/DATA_ACCESS.md); chunks only keep single responses small.
CHUNK_DAYS = {
    "1min": 1,
    "5min": 7,
    "10min": 14,
    "15min": 14,
    "30min": 30,
    "1hour": 90,
    "120min": 90,
    "240min": 180,
    "1day": 3 * 365,
    "weekly": 15 * 365,
    "monthly": 50 * 365,
    "yearly": 50 * 365,
}
INTERVAL = {
    "1min": "1m",
    "5min": "5m",
    "10min": "10m",
    "15min": "15m",
    "30min": "30m",
    "1hour": "1h",
    "120min": "2h",
    "240min": "4h",
    "1day": "1d",
    "weekly": "1w",
    "monthly": "1M",
    "yearly": "1y",
}
# Europe/Istanbul has been UTC+3 all year since 2016-09-07.
IST_OFFSET_MS = 3 * 3600 * 1000
DAY_MS = 86_400_000
# BIST continuous session opens 10:00 local = 07:00 UTC (K14: daily bar t = session open).
SESSION_OPEN_UTC_MS = 7 * 3600 * 1000


def parse_date(s):
    return dt.date.fromisoformat(s)


def chunks(period, start, end):
    # FETCH_CHUNK_DAYS overrides the chunk size (used to measure the server's own limit).
    step = dt.timedelta(days=int(os.environ.get("FETCH_CHUNK_DAYS", CHUNK_DAYS[period])))
    s, e = parse_date(start), parse_date(end)
    while s <= e:
        ce = min(s + step - dt.timedelta(days=1), e)
        print(s.isoformat(), ce.isoformat())
        s = ce + dt.timedelta(days=1)


# Raw bars per full trading day (BIST 10:00-18:00 plus the closing-auction bar).
BARS_PER_DAY = {"1min": 481, "5min": 97, "1hour": 9, "1day": 1}
# A chunk with fewer rows than this share of the expected count is treated as short.
MIN_SHARE = 0.85


def check(period, cs, ce, today, part):
    """Exit 0 if the response holds a plausible number of rows for [cs, ce], else 1.

    The server sometimes answers 200 with a partial or empty array (seen in R3); the
    caller retries such chunks. Expected = weekdays strictly before `today` x bars/day.
    Prints `rows expected`.
    """
    rows = len(load(part))
    per_day = BARS_PER_DAY.get(period)
    if per_day is None:
        print(rows, 0)
        return 0
    d, e = parse_date(cs), min(parse_date(ce), parse_date(today) - dt.timedelta(days=1))
    weekdays = 0
    while d <= e:
        weekdays += d.weekday() < 5
        d += dt.timedelta(days=1)
    expected = weekdays * per_day
    print(rows, expected)
    return 0 if rows >= MIN_SHARE * expected else 1


def load(path):
    with open(path, "rb") as f:
        data = f.read()
    if data[:2] == b"\x1f\x8b":
        data = gzip.decompress(data)
    text = data.decode("utf-8").strip()
    if not text:
        return []
    v = json.loads(text)
    if isinstance(v, dict):
        # Some endpoints wrap the array; accept the first list-valued field.
        for key in ("data", "bars", "result"):
            if isinstance(v.get(key), list):
                return v[key]
        raise ValueError("unexpected response object")
    if not isinstance(v, list):
        raise ValueError("unexpected response type")
    return v


# Local minute of the regular session close (18:00). Intraday bars starting at or after
# it are the closing auction (e.g. an 18:00 hourly bar, an 18:04 one-minute bar); they are
# merged into the previous bar of the same day so every bar lies inside the XIST session.
SESSION_CLOSE_LOCAL_MS = 18 * 3600 * 1000


def to_t(period, time_ms):
    """Maps Matriks `time` to the project bar time (UTC ms of the bar open, K14).

    See docs/DATA_ACCESS.md "R3 doğrulamaları": Matriks `time` is epoch milliseconds.
    Daily bars carry local (Istanbul) midnight of the trading day; we move them to the
    session open (10:00 local = 07:00 UTC). Intraday bars carry their start time, kept.
    """
    if period in ("1day", "weekly", "monthly", "yearly"):
        local_day = (time_ms + IST_OFFSET_MS) // DAY_MS
        return local_day * DAY_MS + SESSION_OPEN_UTC_MS
    return time_ms


def convert(sym, period, adj, start, end, today, chunks_path, dest, meta_path):
    bars = {}
    requests = []
    with open(chunks_path) as f:
        for line in f:
            part, cs, ce, short = line.split()
            rows = load(part)
            requests.append({"start": cs, "end": ce, "rows": len(rows), "short": short == "1"})
            for r in rows:
                t = to_t(period, int(r["time"]))
                v = r.get("volume")
                bars[t] = {
                    "t": t,
                    "o": float(r["open"]),
                    "h": float(r["high"]),
                    "l": float(r["low"]),
                    "c": float(r["close"]),
                    "v": float(v) if v is not None else 0.0,
                }
    out = []
    merged = 0
    intraday = period not in ("1day", "weekly", "monthly", "yearly")
    for t in sorted(bars):
        b = bars[t]
        local = t + IST_OFFSET_MS
        after_close = local % DAY_MS >= SESSION_CLOSE_LOCAL_MS
        if intraday and after_close and out and (out[-1]["t"] + IST_OFFSET_MS) // DAY_MS == local // DAY_MS:
            p = out[-1]
            p["h"] = max(p["h"], b["h"])
            p["l"] = min(p["l"], b["l"])
            p["c"] = b["c"]
            p["v"] = p["v"] + b["v"]
            merged += 1
            continue
        out.append(b)
    series = {
        "id": f"{sym}.{period}.{adj}",
        "instrument": {"symbol": sym, "exchange": "BIST", "calendar": "XIST"},
        "interval": INTERVAL[period],
        "source": {"kind": "inline"},
        "data": {"kind": "bars", "bars": out},
    }
    with open(dest, "w") as f:
        json.dump(series, f, separators=(",", ":"))
    meta = {
        "symbol": sym,
        "period": period,
        "adjusted": adj == "adj",
        "fetched": today,
        "endpoint": "bar.gz" if period in ("1min", "5min", "1hour", "1day") else "derived-bar",
        "start": start,
        "end": end,
        "requests": requests,
        "closing_auction_bars_merged": merged,
        "bars": len(out),
        "first_t": out[0]["t"] if out else None,
        "last_t": out[-1]["t"] if out else None,
        "t_convention": "bar open, UTC ms (K14); daily = 10:00 Europe/Istanbul; intraday = bar start, closing auction merged into the last bar",
    }
    with open(meta_path, "w") as f:
        json.dump(meta, f, indent=1)
    print(f"{sym}: {len(out)} bars ({len(requests)} requests, {merged} closing-auction bars merged)")


def main(argv):
    if len(argv) >= 2 and argv[1] == "chunks":
        chunks(*argv[2:5])
    elif len(argv) >= 2 and argv[1] == "check":
        return check(*argv[2:7])
    elif len(argv) >= 2 and argv[1] == "convert":
        convert(*argv[2:11])
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
PY

TODAY="$(date -u +%Y-%m-%d)"
END="${END:-$TODAY}"
mkdir -p "$OUT"
RAW="$(dirname "$OUT")/raw"
[ "$KEEP_RAW" = 1 ] && mkdir -p "$RAW"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
DIVIDE_PARAM=""
[ "$ADJ" = "unadj" ] && DIVIDE_PARAM="&divide=false"

ok=0
skipped=0
failed=0
FAILED_LIST=""
first_request=1

# One GET; body to $1, headers to $2. Echoes the HTTP status. The Authorization header
# goes through curl's config on stdin, so it never appears in a process argument list.
request() {
  local body="$1" headers="$2" url="$3"
  printf 'header = "Authorization: jwt %s"\n' "$MATRIKS_TOKEN" |
    curl -sS -K - --compressed --max-time 120 -o "$body" -D "$headers" -w '%{http_code}' "$url" ||
    echo "000"
}

while IFS= read -r line || [ -n "$line" ]; do
  sym="$(printf '%s' "$line" | tr -d '[:space:]')"
  [ -n "$sym" ] || continue
  case "$sym" in \#*) continue ;; esac
  [[ "$sym" =~ ^[A-Za-z0-9._-]+$ ]] || { echo "skip invalid symbol: $sym" >&2; continue; }
  dest="$OUT/$sym.$PERIOD.$ADJ.json"
  meta="$OUT/$sym.$PERIOD.$ADJ.meta.json"
  if [ -f "$dest" ]; then
    echo "$sym: exists, skipped"
    skipped=$((skipped + 1))
    continue
  fi
  chunks="$TMP/$sym.chunks"
  rm -f "$TMP/$sym".part.*
  : >"$chunks"
  sym_failed=0
  i=0
  while read -r cs ce; do
    [ "$first_request" = 1 ] || sleep "$PAUSE"
    first_request=0
    url="$API$ENDPOINT?symbol=$sym&start=$cs&end=$ce&period=$PERIOD$DIVIDE_PARAM"
    part="$TMP/$sym.part.$i"
    # Retries: HTTP 429/5xx, and 200 responses with too few rows (the server sometimes
    # answers 200 with a partial or empty array). Up to MAX_TRIES attempts, growing waits.
    try=1
    counts=""
    short=0
    while :; do
      status="$(request "$part" "$part.h" "$url")"
      short=0
      if [ "$status" = "200" ]; then
        if ! counts="$(python3 -I -c "$HELPER_PY" check "$PERIOD" "$cs" "$ce" "$TODAY" "$part")"; then
          short=1
        fi
      fi
      if [ "$status" = "200" ] && [ "$short" = 0 ]; then break; fi
      if [ "$try" -ge "$MAX_TRIES" ]; then break; fi
      if [ "$status" = "200" ]; then
        why="short response (rows expected: $counts)"
      else
        why="HTTP $status"
      fi
      wait_s=$((RETRY_WAIT * try))
      echo "$sym $cs..$ce: $why, retry $try/$((MAX_TRIES - 1)) in ${wait_s}s" >&2
      sleep "$wait_s"
      try=$((try + 1))
    done
    if [ "$KEEP_RAW" = 1 ]; then
      cp "$part" "$RAW/$sym.$PERIOD.$ADJ.$cs.$ce.json" 2>/dev/null || true
      cp "$part.h" "$RAW/$sym.$PERIOD.$ADJ.$cs.$ce.headers" 2>/dev/null || true
    fi
    if [ "$status" != "200" ]; then
      echo "$sym $cs..$ce: HTTP $status" >&2
      sym_failed=1
      break
    fi
    if [ "$short" = 1 ]; then
      echo "$sym $cs..$ce: still short after $MAX_TRIES tries (rows expected: $counts); kept and marked in meta" >&2
    fi
    echo "$part $cs $ce $short" >>"$chunks"
    i=$((i + 1))
  done < <(python3 -I -c "$HELPER_PY" chunks "$PERIOD" "$START" "$END")
  if [ "$sym_failed" = 1 ]; then
    failed=$((failed + 1))
    FAILED_LIST="$FAILED_LIST $sym"
    continue
  fi
  if python3 -I -c "$HELPER_PY" convert "$sym" "$PERIOD" "$ADJ" "$START" "$END" "$TODAY" "$chunks" "$dest" "$meta"; then
    ok=$((ok + 1))
  else
    echo "$sym: conversion failed" >&2
    rm -f "$dest" "$meta"
    failed=$((failed + 1))
    FAILED_LIST="$FAILED_LIST $sym"
  fi
done <"$SYMBOLS"

echo "fetch-bars: period=$PERIOD $ADJ ok=$ok skipped=$skipped failed=$failed${FAILED_LIST:+ (failed:$FAILED_LIST)}"
[ "$failed" = 0 ]
