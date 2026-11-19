#!/usr/bin/env bash
# R4/T5: the demo in one command (shells/web/README.md).
#
#   scripts/demo.sh [qp demo options]     e.g. --synthetic, --data DIR, --from 2025-06-02,
#                                         --speed 8, --port 8390, --paused, --lan
#
#   --lan  listens on 0.0.0.0 so that a phone on the same network can open the page
#          (printed address). Devices on the local network can then see the demo.
#          Without it the demo listens on 127.0.0.1 (this machine only).
#   --gallery  (R5/G) also serves the comparison gallery at /gallery.html (printed
#          address; with --lan / --https-host also the phone's): every topic's candidates
#          one under the other over the synthetic world, "I'd pick this", notes, copy.
#
# 1. builds the web shell's core (qp-wasm, feature `replica`) with wasm-pack into
#    shells/web/pkg, stripped of function names (skipped when up to date;
#    QP_DEMO_REBUILD=1 forces it) and writes
#    shells/web/pkg/sizes.json (raw, gzip -9 and brotli sizes, for the metrics page) and
#    pre-compressed copies of the package (pkg/*.gz, and pkg/*.br when `brotli` exists)
#    that the host serves by Accept-Encoding (RP3/M13). QP_DEMO_PACK_ONLY=1 stops here;
# 2. builds `qp` (release) and runs `qp demo`: a local host on 127.0.0.1:8390 that serves
#    the page and the protocol on one origin. Without local data (fixtures/local/bars or
#    QAPSULE_LOCAL_BARS) it runs the synthetic market and the page says so.
#
# R6/T7 live data:
#   scripts/demo.sh --live [--symbols @file|A,B] [--interval 5m] [--history N]
#          The universe (default @fixtures/local/live/u30.txt) with N (default 120) settled
#          bars of the interval from REST (unadjusted), then ONE live Matriks connection
#          (variables from the environment or ./.env.local, never printed). The page shows
#          real symbols; capsules are made live. This machine only: --live with --lan or
#          --https-host is refused (real data does not go to the local network; a private
#          proxy such as Tailscale to 127.0.0.1 is your own choice, docs/matriks/LIVE.md).
#          Ctrl-C closes the connection cleanly (DISCONNECT) and prints a summary.
#          Local control (this machine, header X-Qp-Live-Control: $QP_LIVE_CONTROL, printed):
#            curl -s localhost:8390/live/stats                      # counts, no values
#            curl -s -X POST -H "X-Qp-Live-Control: $T" 'localhost:8390/live/drop?secs=20'
#          Only the orchestrator opens a live connection, one at a time (lock file).
# Requirements: rustup target wasm32-unknown-unknown, wasm-pack (cargo install wasm-pack).
set -euo pipefail

live=0
port=8390
prev=""
for a in "$@"; do
  case "$a" in
    --live) live=1 ;;
    --lan|--https-host|--https-host=*) lan_like="$a" ;;
  esac
  [ "$prev" = "--port" ] && port="$a"
  case "$a" in --port=*) port="${a#--port=}" ;; esac
  prev="$a"
done
if [ "$live" = 1 ] && [ -n "${lan_like:-}" ]; then
  echo "demo.sh: --live with $lan_like is refused: real market data stays on this machine (127.0.0.1)." >&2
  exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
target_dir="${CARGO_TARGET_DIR:-$root/target}"
case "$target_dir" in /*) ;; *) target_dir="$root/$target_dir" ;; esac

pkg=shells/web/pkg
wasm_pack="$(command -v wasm-pack || echo "$HOME/.cargo/bin/wasm-pack")"
[ -x "$wasm_pack" ] || { echo "demo.sh: wasm-pack not found (cargo install wasm-pack)" >&2; exit 2; }

stale=1
if [ -f "$pkg/qp_wasm_bg.wasm" ] && [ "${QP_DEMO_REBUILD:-0}" != 1 ]; then
  newer="$(find crates -name '*.rs' -newer "$pkg/qp_wasm_bg.wasm" -print -quit 2>/dev/null || true)"
  [ -z "$newer" ] && stale=0
fi
if [ "$stale" = 1 ]; then
  echo "== building the shell core (wasm-pack, profile wasm, feature replica, stripped)"
  # R4/T6: without the function-name section (22% of the raw file: 5.4 -> 4.2 MB raw,
  # 1.45 -> 1.23 MB gzip); the default (view) build keeps the profile as it is.
  CARGO_PROFILE_WASM_STRIP=true \
  "$wasm_pack" build crates/qp-wasm --target web --profile wasm --no-opt \
    --out-dir ../../shells/web/pkg -- --features replica
  rm -f "$pkg/.gitignore" "$pkg/package.json" "$pkg/README.md"
fi
# RP3/M13: pre-compressed package files; `qp demo` serves <file>.br / <file>.gz by the
# request's Accept-Encoding (and the file itself otherwise). gzip -9 always; brotli -q 11
# only when the `brotli` tool exists (otherwise a leftover .br is removed). Skipped for a
# file whose compressed copy is newer than it; the host never serves an older copy.
have_br=0
command -v brotli >/dev/null 2>&1 && have_br=1
for f in "$pkg/qp_wasm_bg.wasm" "$pkg/qp_wasm.js"; do
  if [ ! -f "$f.gz" ] || [ "$f" -nt "$f.gz" ]; then
    gzip -9 -n -c "$f" >"$f.gz.tmp" && mv "$f.gz.tmp" "$f.gz"
  fi
  if [ "$have_br" = 1 ]; then
    if [ ! -f "$f.br" ] || [ "$f" -nt "$f.br" ]; then
      brotli -q 11 -c "$f" >"$f.br.tmp" && mv "$f.br.tmp" "$f.br"
    fi
  else
    rm -f "$f.br"
  fi
done
raw="$(wc -c <"$pkg/qp_wasm_bg.wasm" | tr -d ' ')"
gz="$(wc -c <"$pkg/qp_wasm_bg.wasm.gz" | tr -d ' ')"
br=null
[ -f "$pkg/qp_wasm_bg.wasm.br" ] && br="$(wc -c <"$pkg/qp_wasm_bg.wasm.br" | tr -d ' ')"
js="$(cat shells/web/main.js "$pkg/qp_wasm.js" | wc -c | tr -d ' ')"
jsgz="$(cat shells/web/main.js "$pkg/qp_wasm.js" | gzip -9 -c | wc -c | tr -d ' ')"
printf '{"wasm_raw":%s,"wasm_gzip":%s,"wasm_br":%s,"js_raw":%s,"js_gzip":%s}\n' "$raw" "$gz" "$br" "$js" "$jsgz" >"$pkg/sizes.json"
echo "== sizes: $(cat "$pkg/sizes.json")"
[ "${QP_DEMO_PACK_ONLY:-0}" = 1 ] && exit 0

cargo build -q -p qp-cli --release
if [ "$live" = 1 ]; then
  if [ -z "${MATRIKS_TOKEN_SERVICE_URL:-}" ] && [ -f ./.env.local ]; then
    set -a
    # shellcheck disable=SC1091
    . ./.env.local >/dev/null 2>&1
    set +a
  fi
  if [ -z "${QP_LIVE_CONTROL:-}" ]; then
    QP_LIVE_CONTROL="$(od -An -N16 -tx1 /dev/urandom | tr -d ' \n')"
  fi
  export QP_LIVE_CONTROL
  echo "== live control secret (this machine only): $QP_LIVE_CONTROL"
  # In the background SIGINT does not reach qp; Ctrl-C asks it to close the connection.
  "$target_dir/release/qp" demo --web shells/web "$@" &
  pid=$!
  quit() {
    curl -s -m 15 -X POST -H "X-Qp-Live-Control: $QP_LIVE_CONTROL" \
      "http://127.0.0.1:$port/live/quit" >/dev/null 2>&1 || kill -TERM "$pid" 2>/dev/null || true
  }
  trap quit INT TERM
  while kill -0 "$pid" 2>/dev/null; do wait "$pid" || true; done
  exit 0
fi
exec "$target_dir/release/qp" demo --web shells/web "$@"
