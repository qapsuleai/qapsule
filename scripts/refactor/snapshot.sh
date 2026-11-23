#!/usr/bin/env bash
# Behaviour snapshot for refactoring rounds (RG1, RF1: "Değişmez: davranış değişmez"; K87).
#
# Usage: scripts/refactor/snapshot.sh <out-dir> [--no-wasm] [--no-x86] [--no-gallery]
#
# The round's deliberately added tests are named in $QP_NEW_TESTS (default
# rounds/RF1/experiments/new-tests.txt); they stay out of tests.names / tests.count.
#
# Builds the release `qp` and regenerates every observable output the round must keep
# byte-identical, then writes <out-dir>/manifest.sha256 (one "sha256  path" line per
# output, sorted). Outputs:
#   fixtures/   hashes of the committed goldens, explains, digests, protocol fixtures,
#               views (the files themselves must not change)
#   schema/     `qp schema export`
#   render/     `qp render` of the sample (R0 golden)
#   views/      the 12 R2/R3 views: render PNG, `qp explain`, `qp scene`
#   compute/    `qp compute` rsi(14), macd; digest example (host, r3, x86_64, wasm32)
#   conform/    `qp conform` stdout over inproc, serve, mcp; views transcripts
#   replica/    `qp replica-tape` lag 0/25, viewer lag 0/25
#   replay/     `qp replay` line stream (verify.sh step 16 controls)
#   mcp/        MCP `tools/list` + `initialize` of every `qp mcp serve` surface (the RD1
#               adapter `qp mcp --init`, dumped as mcp/adapter.jsonl in the RG1 baseline,
#               was removed in RD2/A62)
#   drive/      `qp drive` (shell parity script) replica and viewer modes
#   look/       `qp look list`, every `look show` and `look css`
#   cli/        `--help` of every command and subcommand, `qp --version`
#   scorer/     R5B scorer tests (agents/test_score.py, harness test_score.py)
#   tests.names every test's name (last path segment), sorted
#   conform/replies/ the actual reply stream of every transcript over `qp serve`
#   mcp/calls/  fixed `tools/call` requests on every surface
#   tests.count total number of `cargo test --workspace` tests (from `--list`)
#   design/     RD2: `scripts/design-check` tables (P1–P13, H1–H3 per look and scheme) and
#               `scripts/design-contract --check`
#   gallery/    RD2: the core part of `scripts/design-gallery` (default fixture, looks tavek,
#               oltan, r4): every chart PNG `qp render` draws for a mockup slot, the looks'
#               generated variables, the P9 layout dumps and the run manifest (browser
#               screenshots of the mockups are left out: they are HTML, not core output)
# Browser goldens (fixtures/golden/demo, rd2-* included) are compared by verify.sh step 18
# against the committed files; their hashes are under fixtures/.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
out="${1:?usage: snapshot.sh <out-dir> [--no-wasm] [--no-x86] [--no-gallery]}"
shift
want_wasm=1
want_x86=1
want_gallery=1
for a in "$@"; do
  case "$a" in
    --no-wasm) want_wasm=0 ;;
    --no-x86) want_x86=0 ;;
    --no-gallery) want_gallery=0 ;;
    *) echo "unknown flag $a" >&2; exit 2 ;;
  esac
done
case "$out" in /*) ;; *) out="$root/$out" ;; esac
rm -rf "$out"
mkdir -p "$out"

target_dir="${CARGO_TARGET_DIR:-$root/target}"
case "$target_dir" in /*) ;; *) target_dir="$root/$target_dir" ;; esac
qp="$target_dir/release/qp"

# One snapshot at a time per target dir (worktrees share CARGO_TARGET_DIR, K56): the
# release `qp` and the wasm package are built and copied to a private place under a lock.
priv=""
lock="$target_dir/refactor-snapshot.lock"
mkdir -p "$target_dir"
until mkdir "$lock" 2>/dev/null; do sleep 2; done
trap 'rm -rf "$lock" "$priv"' EXIT
priv="$(mktemp -d)"
cargo build -q -p qp-cli --release
cp "$target_dir/release/qp" "$priv/qp"
qp="$priv/qp"

# --- committed fixtures -----------------------------------------------------------
mkdir -p "$out/fixtures"
git ls-files fixtures/golden fixtures/compute fixtures/protocol fixtures/views fixtures/schema \
  fixtures/bars fixtures/oracle fixtures/design | LC_ALL=C sort | while read -r f; do
  shasum -a 256 "$f"
done >"$out/fixtures/committed.sha256"

# --- schema -----------------------------------------------------------------------
"$qp" schema export --out "$out/schema" >/dev/null

# --- render, views ----------------------------------------------------------------
mkdir -p "$out/render" "$out/views"
"$qp" render fixtures/bars/sample.json -o "$out/render/sample.png"
while read -r stem sel; do
  [ -n "$stem" ] || continue
  # shellcheck disable=SC2086
  "$qp" render --transcript "fixtures/views/$stem.json" $sel --out "$out/views/$stem.png"
  # shellcheck disable=SC2086
  "$qp" explain --transcript "fixtures/views/$stem.json" $sel >"$out/views/$stem.explain.txt"
  # shellcheck disable=SC2086
  "$qp" scene --transcript "fixtures/views/$stem.json" $sel >"$out/views/$stem.scene.json"
done <<'EOF'
v0-r0 --view r0
v1-three-panels --view three
v2-bollinger-band --view boll
v3-fan --view fan
v4-log --view logv
v5-conditional --view cond
d1-capsule --present fnd-overbought --surface phone --width 390 --height 300
d2-chart --present fnd-overbought --surface desktop
c2-capsule --present fnd.SYN03.breakout-up.144 --surface phone --width 390 --height 300
c3-chart --present fnd.SYN03.breakout-up.144 --surface desktop
c4-band --present fnd.SYN01.squeeze-neutral.245 --surface desktop
c5-points --present fnd.SYN01.reversal-up.195 --surface desktop
EOF

# --- compute ----------------------------------------------------------------------
mkdir -p "$out/compute"
"$qp" compute fixtures/bars/sample.json --study rsi --param period=14 -o "$out/compute/rsi.json"
"$qp" compute fixtures/bars/sample.json --study macd -o "$out/compute/macd.json"
cargo run -q --release -p qp-compute --example digest >"$out/compute/digest.host.txt"
cargo run -q --release -p qp-compute --example digest -- r3 >"$out/compute/digest-r3.host.txt"
if [ "$want_x86" = 1 ] && [ "$(uname -s)" = Darwin ] && [ "$(uname -m)" = arm64 ] \
  && rustup target list --installed 2>/dev/null | grep -q '^x86_64-apple-darwin$'; then
  cargo run -q --release -p qp-compute --example digest --target x86_64-apple-darwin \
    >"$out/compute/digest.x86_64.txt"
fi
wp="$(command -v wasm-pack || echo "$HOME/.cargo/bin/wasm-pack")"
if [ "$want_wasm" = 1 ] && command -v node >/dev/null 2>&1 && [ -x "$wp" ]; then
  pkg="$priv/pkg"
  "$wp" build crates/qp-wasm --target nodejs --profile wasm --no-opt --out-dir "$pkg" >/dev/null 2>&1
  node -e "console.log(require('$pkg/qp_wasm.js').compute_digest())" >"$out/compute/digest.wasm32.txt"
fi

# --- conformance ------------------------------------------------------------------
mkdir -p "$out/conform"
for t in inproc serve mcp; do
  "$qp" conform fixtures/protocol/conformance --transport "$t" >"$out/conform/$t.txt"
done
"$qp" conform fixtures/views >"$out/conform/views.txt"
# the authority's actual replies (every output line) for every transcript over `qp serve`
python3 scripts/refactor/serve_dump.py "$qp" "$out/conform/replies" fixtures/protocol/conformance/*.json fixtures/views/*.json

# --- replica, replay --------------------------------------------------------------
mkdir -p "$out/replica" "$out/replay"
"$qp" replica-tape --lag 0 --out "$out/replica/tape0.json" >/dev/null
"$qp" replica-tape --lag 25 --out "$out/replica/tape25.json" >/dev/null
"$qp" replica-tape --viewer --lag 0 --out "$out/replica/viewer0.json" >/dev/null
"$qp" replica-tape --viewer --lag 25 --out "$out/replica/viewer25.json" >/dev/null
cat >"$out/replay/controls.json" <<'JSON'
[{"at":5,"control":{"kind":"speed","steps_per_s":10}},
 {"at":50,"control":{"kind":"seek","date":"2023-07-03"}},
 {"at":120,"control":{"kind":"seek","step":30}}]
JSON
"$qp" replay --source synthetic --steps 300 --controls "$out/replay/controls.json" \
  --out "$out/replay/stream.jsonl"

# --- MCP tool definitions ---------------------------------------------------------
mkdir -p "$out/mcp"
rpc='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"rg1","version":"0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
for s in thin task query mixed; do
  printf '%s\n' "$rpc" | "$qp" mcp serve --surface "$s" >"$out/mcp/serve-$s.jsonl" 2>/dev/null
done
scripts/refactor/mcp_calls.sh "$qp" "$out/mcp/calls"

# --- drive (shell parity, shell-less) ---------------------------------------------
mkdir -p "$out/drive"
"$qp" drive --script shells/web/tests/parity-script.json --out "$out/drive/replica.txt"
"$qp" drive --mode viewer --script shells/web/tests/parity-script.json --out "$out/drive/viewer.txt"

# --- looks ------------------------------------------------------------------------
mkdir -p "$out/look"
"$qp" look list >"$out/look/list.txt"
cut -f1 "$out/look/list.txt" | while read -r l; do
  "$qp" look show "$l" >"$out/look/$l.json"
  "$qp" look css "$l" >"$out/look/$l.css"
done

# --- command line interface (`--help` of every command and subcommand) -------------
mkdir -p "$out/cli"
help_tree() {
  # $1: file stem, rest: the command words
  local stem="$1"; shift
  "$qp" "$@" --help >"$out/cli/$stem.txt" 2>&1 || true
  awk '/^Commands:/{f=1;next} /^$/{f=0} f && $1 != "help" {print $1}' "$out/cli/$stem.txt" \
    | while read -r sub; do help_tree "$stem.$sub" "$@" "$sub"; done
}
help_tree qp
"$qp" --version >"$out/cli/version.txt"

# --- R5B scorer tests -------------------------------------------------------------
mkdir -p "$out/scorer"
(cd agents && python3 test_score.py >"$out/scorer/agents.txt" 2>&1) || echo "FAIL" >>"$out/scorer/agents.txt"
(cd rounds/R5B/experiments/harness && python3 test_score.py >"$out/scorer/harness.txt" 2>&1) \
  || echo "FAIL" >>"$out/scorer/harness.txt"
# unittest prints its wall time ("Ran N tests in 0.012s"); keep the count, drop the time.
for f in "$out"/scorer/*.txt; do
  sed -E 's/^(Ran [0-9]+ tests?) in [0-9.]+s$/\1/' "$f" >"$f.tmp" && mv "$f.tmp" "$f"
done

# --- design (RD2) ------------------------------------------------------------------
mkdir -p "$out/design"
scripts/design-check >"$out/design/check.txt" 2>&1 || echo "FAIL" >>"$out/design/check.txt"
scripts/design-contract --check >"$out/design/contract.txt" 2>&1 || echo "FAIL" >>"$out/design/contract.txt"
if [ "$want_gallery" = 1 ]; then
  # design-gallery runs `$CARGO_TARGET_DIR/debug/qp`: point it at the private release build.
  mkdir -p "$priv/gt/debug" "$out/gallery"
  ln -s "$qp" "$priv/gt/debug/qp"
  CARGO_TARGET_DIR="$priv/gt" node scripts/design-gallery --no-build --looks tavek,oltan,r4 \
    --out "$priv/gallery" >/dev/null
  (cd "$priv/gallery" && cp -R charts vars "$out/gallery/" && cp layout-*.json manifest.json "$out/gallery/")
fi

# --- test count -------------------------------------------------------------------
cargo test -q --workspace -- --list 2>/dev/null | grep ': test$' >"$priv/tests.list" || true
# test names by their last path segment (module moves change the prefix, not the name).
# Tests the round adds on purpose (characterisation tests, the extension-point test) are
# named one per line in $QP_NEW_TESTS: they are left out of tests.names/tests.count, so
# no baseline test can disappear unseen, and every listed name must exist (else FAIL).
sed -E 's/: test$//; s/.*:://' "$priv/tests.list" | LC_ALL=C sort >"$priv/tests.all"
allow="${QP_NEW_TESTS:-rounds/RF1/experiments/new-tests.txt}"
if [ -f "$allow" ]; then
  { grep -v '^#' "$allow" || true; } | sed '/^$/d' | LC_ALL=C sort >"$priv/allow"
  missing="$(LC_ALL=C comm -23 "$priv/allow" <(LC_ALL=C sort -u "$priv/tests.all"))"
  [ -z "$missing" ] || { echo "new-tests.txt names missing tests: $missing" >&2; echo "MISSING: $missing" >"$out/tests.allow-missing"; }
  python3 - "$priv/tests.all" "$priv/allow" >"$out/tests.names" <<'PY'
import collections, sys
names = [l.rstrip("\n") for l in open(sys.argv[1])]
allow = collections.Counter(l.rstrip("\n") for l in open(sys.argv[2]))
for n in names:
    if allow[n] > 0:
        allow[n] -= 1
    else:
        print(n)
PY
else
  cp "$priv/tests.all" "$out/tests.names"
fi
wc -l <"$out/tests.names" | tr -d ' ' >"$out/tests.count"

# --- manifest ---------------------------------------------------------------------
(cd "$out" && find . -type f ! -name manifest.sha256 | LC_ALL=C sort | sed 's|^\./||' \
  | while read -r f; do shasum -a 256 "$f"; done) >"$out/manifest.sha256"
echo "snapshot: $(wc -l <"$out/manifest.sha256" | tr -d ' ') files -> $out/manifest.sha256"
