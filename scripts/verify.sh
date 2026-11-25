#!/usr/bin/env bash
# Qapsule verification harness (R0/T6, extended in R1/T7).
#
# One command, from a clean clone: runs every check, prints a PASS/FAIL/SKIP summary
# and exits 0 iff no step failed. All steps run even if an earlier one fails.
#
# Steps:
#   1 fmt        cargo fmt --all -- --check
#   2 clippy     cargo clippy --workspace --all-targets -- -D warnings
#   3 test       cargo test --workspace
#   4 schema     qp schema export (>= 7 *.schema.json, two exports identical)
#   5 fixtures   fixtures/schema/*.example.json validate against the exported schema
#   6 golden     qp render twice: byte-identical, and identical to fixtures/golden/sample.png
#   7 views      R2 views (fixtures/views/*.json, built only from protocol messages): the
#                transcripts pass `qp conform`; each view is rendered twice from its
#                transcript, byte-identical and equal to its golden (V0:
#                fixtures/golden/sample.png, others: fixtures/golden/views/*.png);
#                `qp explain` output equals fixtures/views/*.explain.txt
#   8 wasm       cargo build -p qp-wasm --target wasm32-unknown-unknown --profile wasm (+ sizes)
#   9 hygiene    no secret-looking tracked files; .gitignore has a .env rule
#  10 browser    shells/web-spike/run.sh (headless Chromium vs golden), optional
#  11 compute    qp compute: RSI(14) and MACD on the sample, outputs valid StudyOutput
#  12 digest     cross-target compute digest == fixtures/compute/digest.txt on the host
#                (aarch64 or x86_64), x86_64 via Rosetta on Apple silicon if available,
#                and wasm32 in Node (same tool rule as the browser step); RL1: also
#                fixtures/compute/digest-rl1.txt (stdev, percentrank) on the host, x86_64
#                and wasm32-wasip1 under node:wasi
#  13 protocol   fixtures/protocol/valid/* validate and fixtures/protocol/invalid/* are
#                rejected by the generated protocol schemas; transcripts and init validate
#  14 conform    qp/0 conformance transcripts pass in-process, over `qp serve` and over MCP
#  15 replica    authority <-> replica (RP2/T8, PROTOCOL §19.6): `qp replica-tape` with the
#                replica's data in step (digests equal after every step) and 25 events
#                behind (no wrong or early claim); the tapes replayed by the WASM replica in
#                Node reproduce every digest (same tool rule as the browser step); (RP3/M14)
#                the viewer variant (`qp replica-tape --viewer`, PROTOCOL §22) at lag 0 and
#                25: 0 violations, every final claim shown final at the end, never subscribed
#                to the other instrument, and at lag 0 the open slice's digests equal the
#                authority's after every step
#  16 replay     R4/T2 (PROTOCOL §21): `qp replay` (synthetic market, 30 instruments, hourly
#                path, recorded controls with a forward and a backward seek) twice: the two
#                line streams are byte-identical, no host notice was raised, no error line,
#                and after the last backward seek (`reset: true`) the tool's connection is
#                back (`welcome`) and notifications still arrive (the stream is not empty)
#  17 shell      R4/T5 architecture test of the web shell (shells/web/ARCH.md): no domain
#                logic in its JavaScript (no arithmetic, sorting, filtering, formatting, text
#                building), only allowed core methods, no external address; plus a self-test
#  18 demo       R4/T5 in headless Chromium (same tool rule as the browser step): the shell
#                built with wasm-pack, `qp demo` (synthetic, paused); the page's requests go
#                only to its own origin; its digests (document, faces, open scene) equal the
#                shell-less `qp drive` run; screenshots at a fixed replay moment twice: page
#                screenshots byte-identical, core-painted scenes equal fixtures/golden/demo
#                (screens.mjs paints with `?paint=ref`, the deterministic reference);
#                the settings page and URL settings (shells/web/tests/settings.mjs, R4/S2);
#                (R5/G) the comparison gallery at phone size: same content per topic, a pick
#                and a note, the copied summary (shells/web/tests/gallery.mjs, VERIFY 15/18);
#                (RP3/S) every paint path opens with `?paint=` and paints, `gpu` reports
#                its fallback where WebGPU has no adapter, the default path is `canvas`,
#                and the semantic reads (digests, feed, open capsule, the authority's
#                tap_at/hit and screen answers) are byte-identical on every path
#                (tests/paint-paths.mjs); the default `canvas` path's scenes against the
#                reference with tolerance (tests/paint-canvas.mjs, Chromium, DPR 1)
#  19 arch       RG1 (docs/ENGINEERING.md): scripts/arch-check --self-test (known violations
#                are caught, the ratchet works) and scripts/arch-check (layers, size budgets,
#                names, READMEs, lib.rs/mod.rs; debt only within scripts/arch-baseline.json);
#                scripts/refactor/moved.py --self-test (the move check's fingerprints, K-8)
#  20 config     RF1/A2 (K-9): scripts/arch-check --config-diff prints the diff of
#                scripts/arch-config.json and scripts/arch-baseline.json against main (merge
#                base; on main $QP_CONFIG_BASE or the round's baseline commit) and fails on a
#                new exception, generated file or loosened budget/baseline value that cites
#                no real, new decision (a K<n> row under docs/decisions/ or an existing
#                rounds/<R>/DECISION.md), or any other loosening without rule_changes;
#                an explicit base that is HEAD or its descendant fails; skipped when git
#                has no such base (shallow clone)
#  21 design     RD2/S1: scripts/design-check: P1–P13 (../refs/reports/VISUAL.md §5) and R5's
#                hard thresholds for every registered non-legacy look in both schemes (legacy
#                looks are printed, never failing), and the value tables of
#                docs/design/CONTRACT.md equal the look record (scripts/design-contract --check)
#
# Environment:
#   QP_VERIFY_BROWSER=1  force the browser step (fails if tools are missing)
#   QP_VERIFY_BROWSER=0  skip the browser step
#   unset                run the browser step iff node and wasm-pack are available
#
# Portable: macOS bash 3.2 (no associative arrays, no mapfile) and Linux GNU tools.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

target_dir="${CARGO_TARGET_DIR:-$root/target}"
case "$target_dir" in
  /*) ;;
  *) target_dir="$root/$target_dir" ;;
esac
qp="$target_dir/release/qp"

tmp="$(mktemp -d 2>/dev/null || mktemp -d -t qp-verify)"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

SKIP_CODE=99
names=()
results=()
times=()
notes=()
note_file="$tmp/.note"

# A step may write a one-line note (e.g. sizes, skip reason) for the summary.
note() { printf '%s' "$*" >"$note_file"; }
skip() {
  note "$*"
  echo "SKIP: $*"
  exit "$SKIP_CODE"
}
fail() {
  note "$*"
  echo "FAIL: $*" >&2
  exit 1
}

run_step() {
  local name="$1" fn="$2" start rc result elapsed msg
  echo
  echo "================================================================"
  echo "== [$(( ${#names[@]} + 1 ))] $name"
  echo "================================================================"
  : >"$note_file"
  start=$SECONDS
  set +e
  (
    set -euo pipefail
    "$fn"
  )
  rc=$?
  set -e
  elapsed=$((SECONDS - start))
  if [ "$rc" -eq 0 ]; then
    result=PASS
  elif [ "$rc" -eq "$SKIP_CODE" ]; then
    result=SKIP
  else
    result=FAIL
  fi
  msg="$(cat "$note_file" 2>/dev/null || true)"
  echo "-- $name: $result (${elapsed}s)"
  names+=("$name")
  results+=("$result")
  times+=("$elapsed")
  notes+=("$msg")
}

# --- steps -----------------------------------------------------------------

step_fmt() { cargo fmt --all -- --check; }

step_clippy() { cargo clippy --workspace --all-targets -- -D warnings; }

step_test() { cargo test --workspace; }

build_qp() {
  if [ ! -f "$tmp/.qp-built" ]; then
    cargo build -p qp-cli --release
    : >"$tmp/.qp-built"
  fi
  [ -x "$qp" ] || fail "qp binary not found at $qp"
}

step_schema() {
  build_qp
  "$qp" schema export --out "$tmp/schema"
  "$qp" schema export --out "$tmp/schema2"
  local n
  n="$(find "$tmp/schema" -maxdepth 1 -name '*.schema.json' | wc -l | tr -d ' ')"
  echo "exported $n schema files:"
  (cd "$tmp/schema" && ls -1 ./*.schema.json | sed 's|^\./|  |')
  [ "$n" -ge 7 ] || fail "expected >= 7 *.schema.json, got $n"
  diff -r "$tmp/schema" "$tmp/schema2" || fail "two schema exports differ"
  echo "two exports are identical"
  note "$n schemas, deterministic"
}

# capsule -> Capsule, bar_series / bar-series -> BarSeries
type_name_for() {
  printf '%s\n' "$1" | awk -F'[_-]' '{ s = ""; for (i = 1; i <= NF; i++) s = s toupper(substr($i, 1, 1)) substr($i, 2); print s }'
}

step_fixtures() {
  build_qp
  [ -d "$tmp/schema" ] || "$qp" schema export --out "$tmp/schema"
  local f base ty count=0 bad=0
  for f in fixtures/schema/*.example.json; do
    [ -e "$f" ] || fail "no fixtures/schema/*.example.json found"
    base="$(basename "$f" .example.json)"
    ty="$(type_name_for "$base")"
    echo "-- $f as $ty"
    if "$qp" schema validate "$f" --type "$ty" --schema-dir "$tmp/schema"; then
      count=$((count + 1))
    else
      echo "INVALID: $f ($ty)" >&2
      bad=$((bad + 1))
    fi
  done
  [ "$bad" -eq 0 ] || fail "$bad fixture(s) invalid"
  note "$count fixtures valid"
}

step_golden() {
  build_qp
  "$qp" render fixtures/bars/sample.json -o "$tmp/a.png"
  "$qp" render fixtures/bars/sample.json -o "$tmp/b.png"
  cmp "$tmp/a.png" "$tmp/b.png" || fail "two renders differ"
  echo "two renders are byte-identical"
  if ! cmp "$tmp/a.png" fixtures/golden/sample.png; then
    mkdir -p "$target_dir/golden-actual"
    cp "$tmp/a.png" "$target_dir/golden-actual/qp-render-sample.png"
    fail "render differs from fixtures/golden/sample.png (actual: $target_dir/golden-actual/qp-render-sample.png)"
  fi
  echo "render matches fixtures/golden/sample.png"
  note "byte-identical to golden"
}

# R2 views (R3: c2–c5 kapsül görünümleri): one line per view, "<transcript stem> <qp render/explain view selection>".
VIEWS_LIST='v0-r0 --view r0
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
c5-points --present fnd.SYN01.reversal-up.195 --surface desktop'

step_views() {
  build_qp
  if ! "$qp" conform fixtures/views >"$tmp/views-conform.txt"; then
    cat "$tmp/views-conform.txt" >&2
    fail "a view transcript fails qp conform"
  fi
  echo "view transcripts pass qp conform"
  local f stem sel golden d="$tmp/views" n=0 bad=0
  for f in fixtures/views/*.json; do
    "$qp" schema validate "$f" --type Transcript >/dev/null || fail "$f is not a valid Transcript"
  done
  for f in fixtures/views/docs/*.json; do
    "$qp" schema validate "$f" --type Document >/dev/null || fail "$f is not a valid Document"
  done
  echo "view transcripts and documents validate"
  mkdir -p "$d"
  while read -r stem sel; do
    [ -n "$stem" ] || continue
    n=$((n + 1))
    if [ "$stem" = v0-r0 ]; then
      golden=fixtures/golden/sample.png
    else
      golden="fixtures/golden/views/$stem.png"
    fi
    # $sel is a list of arguments (no spaces inside any of them): split on purpose.
    # shellcheck disable=SC2086
    "$qp" render --transcript "fixtures/views/$stem.json" $sel --out "$d/$stem.a.png"
    # shellcheck disable=SC2086
    "$qp" render --transcript "fixtures/views/$stem.json" $sel --out "$d/$stem.b.png"
    # shellcheck disable=SC2086
    "$qp" explain --transcript "fixtures/views/$stem.json" $sel >"$d/$stem.explain.txt"
    if ! cmp -s "$d/$stem.a.png" "$d/$stem.b.png"; then
      echo "$stem: two renders differ" >&2
      bad=$((bad + 1))
    elif ! cmp -s "$d/$stem.a.png" "$golden"; then
      mkdir -p "$target_dir/golden-actual"
      cp "$d/$stem.a.png" "$target_dir/golden-actual/$stem.png"
      echo "$stem: render differs from $golden (actual: $target_dir/golden-actual/$stem.png)" >&2
      bad=$((bad + 1))
    elif ! cmp -s "$d/$stem.explain.txt" "fixtures/views/$stem.explain.txt"; then
      diff "fixtures/views/$stem.explain.txt" "$d/$stem.explain.txt" >&2 || true
      echo "$stem: explain differs from fixtures/views/$stem.explain.txt" >&2
      bad=$((bad + 1))
    else
      echo "$stem: two renders byte-identical and equal to $golden; explain matches"
    fi
  done <<EOF
$VIEWS_LIST
EOF
  [ "$bad" -eq 0 ] || fail "$bad of $n view(s) failed"
  note "$n views x2 byte-identical to goldens (V0 = sample.png)"
}

step_wasm() {
  cargo build -p qp-wasm --target wasm32-unknown-unknown --profile wasm
  local w="$target_dir/wasm32-unknown-unknown/wasm/qp_wasm.wasm" raw gz
  [ -f "$w" ] || fail "wasm output not found at $w"
  raw="$(wc -c <"$w" | tr -d ' ')"
  gz="$(gzip -9 -c "$w" | wc -c | tr -d ' ')"
  echo "wasm: $w"
  echo "wasm size: raw=$raw bytes, gzip -9=$gz bytes"
  note "raw $raw B, gzip -9 $gz B"
}

step_hygiene() {
  if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    skip "not a git repository"
  fi
  local hits
  # scripts/qapsule-gh.env is tracked on purpose (.gitignore exception, decided by Arkin):
  # account settings for qgh/preflight. It is allowed only while it holds exactly the
  # known non-secret keys.
  if git ls-files --error-unmatch scripts/qapsule-gh.env >/dev/null 2>&1; then
    if grep -v -E '^(#.*|QAPSULE_GH_USER=[A-Za-z0-9_-]+|QAPSULE_GH_REPO=[A-Za-z0-9_./-]+|QAPSULE_SSH_ALIAS=[A-Za-z0-9_.-]+|QAPSULE_REMOTE_URL=git@[A-Za-z0-9_.-]+:[A-Za-z0-9_./-]+\.git|)$' scripts/qapsule-gh.env; then
      fail "scripts/qapsule-gh.env has unexpected content"
    fi
  fi
  hits="$(git ls-files | grep -v -x 'scripts/qapsule-gh.env' | grep -i -E '\.env$|token|secret' || true)"
  if [ -n "$hits" ]; then
    echo "tracked files that look like secrets:" >&2
    printf '%s\n' "$hits" >&2
    fail "secret-looking files are tracked"
  fi
  echo "no secret-looking tracked files"
  [ -f .gitignore ] || fail ".gitignore missing"
  grep -q -E '^[[:space:]]*(/|\*\*/)?\.env[[:space:]]*$' .gitignore || fail ".gitignore has no .env rule"
  echo ".gitignore has a .env rule"
}

step_browser() {
  local mode="${QP_VERIFY_BROWSER:-}"
  case "$mode" in
    0) skip "QP_VERIFY_BROWSER=0" ;;
    1) ;;
    *)
      command -v node >/dev/null 2>&1 || skip "node not found (set QP_VERIFY_BROWSER=1 to force)"
      if ! command -v wasm-pack >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/wasm-pack" ]; then
        skip "wasm-pack not found (set QP_VERIFY_BROWSER=1 to force)"
      fi
      ;;
  esac
  shells/web-spike/run.sh
  note "headless Chromium matches golden"
}

step_compute() {
  build_qp
  local out="$tmp/compute"
  mkdir -p "$out"
  [ -d "$tmp/schema" ] || "$qp" schema export --out "$tmp/schema"
  "$qp" compute fixtures/bars/sample.json --study rsi --param period=14 -o "$out/rsi.json"
  "$qp" compute fixtures/bars/sample.json --study macd -o "$out/macd.json"
  local f
  for f in rsi macd; do
    "$qp" schema validate "$out/$f.json" --type StudyOutput --schema-dir "$tmp/schema" \
      || fail "$f output is not a valid StudyOutput"
  done
  # Warmup is null (K13): RSI(14) starts with exactly 14 nulls; MACD has 3 outputs.
  grep -q '"rsi":\[\(null,\)\{14\}[0-9]' "$out/rsi.json" || fail "RSI(14) warmup is not 14 nulls"
  for k in macd signal histogram; do
    grep -q "\"$k\":\[" "$out/macd.json" || fail "MACD output $k missing"
  done
  note "rsi(14), macd: valid StudyOutput"
}

step_digest() {
  local want got ran="" mode="${QP_VERIFY_BROWSER:-}"
  want="$(tr -d '[:space:]' <fixtures/compute/digest.txt)"
  got="$(cargo run -q --release -p qp-compute --example digest)"
  echo "host $(uname -m): $got (expected $want)"
  [ "$got" = "$want" ] || fail "host digest $got != $want"
  # R3: the new studies (Highest, Lowest, Atr) have their own digest; the old one is unchanged.
  local want3 got3
  want3="$(tr -d '[:space:]' <fixtures/compute/digest-r3.txt)"
  got3="$(cargo run -q --release -p qp-compute --example digest -- r3)"
  echo "host $(uname -m) r3 studies: $got3 (expected $want3)"
  [ "$got3" = "$want3" ] || fail "host r3 digest $got3 != $want3"
  # RL1: the compute-language kernels (Stdev, PercentRank) have their own digest too,
  # compared on every architecture this host can run (wasm32 via wasm32-wasip1 + node:wasi).
  local wantl gotl
  wantl="$(tr -d '[:space:]' <fixtures/compute/digest-rl1.txt)"
  gotl="$(cargo run -q --release -p qp-compute --example digest -- rl1)"
  echo "host $(uname -m) rl1 kernels: $gotl (expected $wantl)"
  [ "$gotl" = "$wantl" ] || fail "host rl1 digest $gotl != $wantl"
  ran="$(uname -m)"
  if [ "$(uname -s)" = Darwin ] && [ "$(uname -m)" = arm64 ] \
    && rustup target list --installed 2>/dev/null | grep -q '^x86_64-apple-darwin$' \
    && arch -x86_64 /usr/bin/true 2>/dev/null; then
    got="$(cargo run -q --release -p qp-compute --example digest --target x86_64-apple-darwin)"
    echo "x86_64 (Rosetta): $got"
    [ "$got" = "$want" ] || fail "x86_64 digest $got != $want"
    gotl="$(cargo run -q --release -p qp-compute --example digest --target x86_64-apple-darwin -- rl1)"
    echo "x86_64 (Rosetta) rl1 kernels: $gotl"
    [ "$gotl" = "$wantl" ] || fail "x86_64 rl1 digest $gotl != $wantl"
    ran="$ran, x86_64"
  fi
  if [ "$mode" != 0 ] && command -v node >/dev/null 2>&1 \
    && rustup target list --installed 2>/dev/null | grep -q '^wasm32-wasip1$'; then
    cargo build -q --release -p qp-compute --example digest --target wasm32-wasip1
    gotl="$(node --no-warnings -e "const {WASI}=require('node:wasi');const w=new WASI({version:'preview1',args:['digest','rl1'],env:{}});WebAssembly.instantiate(require('fs').readFileSync(process.argv[1]),w.getImportObject()).then(({instance})=>w.start(instance))" "$target_dir/wasm32-wasip1/release/examples/digest.wasm")"
    echo "wasm32-wasip1 (node $(node --version)) rl1 kernels: $gotl"
    [ "$gotl" = "$wantl" ] || fail "wasm32 rl1 digest $gotl != $wantl"
  else
    echo "wasm32 rl1 kernels: skipped (QP_VERIFY_BROWSER=${mode:-unset}, node or wasm32-wasip1 missing)"
  fi
  local have_tools=1
  command -v node >/dev/null 2>&1 || have_tools=0
  if ! command -v wasm-pack >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/wasm-pack" ]; then have_tools=0; fi
  if [ "$mode" = 0 ] || { [ "$mode" != 1 ] && [ "$have_tools" = 0 ]; }; then
    echo "wasm32: skipped (QP_VERIFY_BROWSER=${mode:-unset}, tools: $have_tools)"
    note "$ran: $want; wasm32 skipped; rl1 $wantl"
    return 0
  fi
  local wp pkg="$target_dir/wasm-bench/pkg"
  wp="$(command -v wasm-pack || echo "$HOME/.cargo/bin/wasm-pack")"
  "$wp" build crates/qp-wasm --target nodejs --profile wasm --out-dir "$pkg"
  got="$(node -e "console.log(require('$pkg/qp_wasm.js').compute_digest())")"
  echo "wasm32 (node $(node --version)): $got"
  [ "$got" = "$want" ] || fail "wasm32 digest $got != $want"
  note "$ran, wasm32: $want; rl1 $wantl"
}

step_protocol() {
  build_qp
  [ -d "$tmp/schema" ] || "$qp" schema export --out "$tmp/schema"
  local f b ty good=0 bad=0
  for f in fixtures/protocol/valid/*.json; do
    b="$(basename "$f")"; ty="$(type_name_for "${b%%.*}")"
    if "$qp" schema validate "$f" --type "$ty" --schema-dir "$tmp/schema" >/dev/null; then
      good=$((good + 1))
    else
      echo "valid example rejected: $f ($ty)" >&2; bad=$((bad + 1))
    fi
  done
  for f in fixtures/protocol/invalid/*.json; do
    b="$(basename "$f")"; ty="$(type_name_for "${b%%.*}")"
    if "$qp" schema validate "$f" --type "$ty" --schema-dir "$tmp/schema" >/dev/null 2>&1; then
      echo "invalid example accepted: $f ($ty)" >&2; bad=$((bad + 1))
    else
      good=$((good + 1))
    fi
  done
  for f in fixtures/protocol/conformance/*.json; do
    "$qp" schema validate "$f" --type Transcript --schema-dir "$tmp/schema" >/dev/null \
      || { echo "transcript invalid: $f" >&2; bad=$((bad + 1)); }
  done
  "$qp" schema validate fixtures/protocol/init/thyao.json --type SessionInit --schema-dir "$tmp/schema" >/dev/null \
    || { echo "init invalid" >&2; bad=$((bad + 1)); }
  [ "$bad" -eq 0 ] || fail "$bad protocol fixture check(s) failed"
  note "$good examples as expected; transcripts and init valid"
}

step_conform() {
  build_qp
  local t n
  n="$(find fixtures/protocol/conformance -name '*.json' | wc -l | tr -d ' ')"
  for t in inproc serve mcp; do
    "$qp" conform fixtures/protocol/conformance --transport "$t" || fail "conformance failed over $t"
  done
  note "$n transcripts x inproc, serve, mcp"
}

step_replay() {
  build_qp
  local d="$tmp/replay"
  mkdir -p "$d"
  cat >"$d/controls.json" <<'JSON'
[{"at":5,"control":{"kind":"speed","steps_per_s":10}},
 {"at":50,"control":{"kind":"seek","date":"2023-07-03"}},
 {"at":120,"control":{"kind":"seek","step":30}}]
JSON
  "$qp" replay --source synthetic --steps 300 --controls "$d/controls.json" --out "$d/a.jsonl" \
    || fail "qp replay raised host notices"
  "$qp" replay --source synthetic --steps 300 --controls "$d/controls.json" --out "$d/b.jsonl" \
    || fail "qp replay raised host notices"
  cmp -s "$d/a.jsonl" "$d/b.jsonl" || fail "two replay runs differ"
  local last after welcome notes
  ! grep -q '"msg":{"type":"error"' "$d/a.jsonl" || fail "qp replay wrote an error line"
  last="$(grep -n '"reset":true' "$d/a.jsonl" | tail -1 | cut -d: -f1)"
  [ -n "$last" ] || fail "no backward seek (reset) in the replay stream"
  after="$(tail -n +"$((last + 1))" "$d/a.jsonl")"
  welcome="$(printf '%s\n' "$after" | grep -c '"msg":{"type":"welcome"' || true)"
  notes="$(printf '%s\n' "$after" | grep -c '"msg":{"type":"notify"' || true)"
  [ "$welcome" -ge 1 ] || fail "no reconnection (welcome) after the last backward seek"
  [ "$notes" -ge 1 ] || fail "empty stream after the last backward seek"
  note "two runs byte-identical ($(wc -l <"$d/a.jsonl" | tr -d ' ') lines; $notes notifications after the last backward seek)"
}

step_replica() {
  build_qp
  local d="$tmp/replica" mode="${QP_VERIFY_BROWSER:-}" have_tools=1 wp pkg="$target_dir/wasm-replica/pkg"
  mkdir -p "$d"
  "$qp" replica-tape --lag 0 --out "$d/tape0.json" || fail "replica in step: digests differ or a claim is wrong"
  "$qp" replica-tape --lag 25 --out "$d/tape25.json" || fail "lagging replica: a wrong or early claim"
  "$qp" replica-tape --viewer --lag 0 --out "$d/viewer0.json" \
    || fail "viewer in step: a violation, a digest differs or a final claim is not shown"
  "$qp" replica-tape --viewer --lag 25 --out "$d/viewer25.json" \
    || fail "lagging viewer: a wrong or early claim, or a final claim not shown"
  command -v node >/dev/null 2>&1 || have_tools=0
  if ! command -v wasm-pack >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/wasm-pack" ]; then have_tools=0; fi
  if [ "$mode" = 0 ] || { [ "$mode" != 1 ] && [ "$have_tools" = 0 ]; }; then
    echo "wasm32 replica: skipped (QP_VERIFY_BROWSER=${mode:-unset}, tools: $have_tools)"
    note "native authority = native replica (lag 0, 25), viewer (lag 0, 25); wasm32 skipped"
    return 0
  fi
  wp="$(command -v wasm-pack || echo "$HOME/.cargo/bin/wasm-pack")"
  "$wp" build crates/qp-wasm --target nodejs --profile wasm --out-dir "$pkg" -- --features replica
  node crates/qp-wasm/tests/replica_tape.mjs "$pkg" "$d/tape0.json" "$d/tape25.json" \
    || fail "wasm32 replica digests differ from native"
  note "native authority = native replica = wasm32 replica (lag 0, 25); viewer (lag 0, 25)"
}

step_shell() {
  if [ "${QP_VERIFY_BROWSER:-}" = 0 ] && ! command -v node >/dev/null 2>&1; then
    skip "node not found and QP_VERIFY_BROWSER=0"
  fi
  command -v node >/dev/null 2>&1 || fail "node not found (the architecture test needs node)"
  node shells/web/arch-check.mjs || fail "the web shell breaks an architecture rule"
  note "no domain logic in shells/web; self-test passes"
}

step_demo() {
  local mode="${QP_VERIFY_BROWSER:-}" wp d="$tmp/demo" f b bad=0 n=0
  case "$mode" in
    0) skip "QP_VERIFY_BROWSER=0" ;;
    1) ;;
    *)
      command -v node >/dev/null 2>&1 || skip "node not found (set QP_VERIFY_BROWSER=1 to force)"
      if ! command -v wasm-pack >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/wasm-pack" ]; then
        skip "wasm-pack not found (set QP_VERIFY_BROWSER=1 to force)"
      fi
      ;;
  esac
  build_qp
  wp="$(command -v wasm-pack || echo "$HOME/.cargo/bin/wasm-pack")"
  "$wp" build crates/qp-wasm --target web --profile wasm \
    --out-dir ../../shells/web/pkg -- --features replica
  rm -f shells/web/pkg/.gitignore shells/web/pkg/package.json shells/web/pkg/README.md
  printf '{"wasm_raw":%s,"wasm_gzip":%s}\n' "$(wc -c <shells/web/pkg/qp_wasm_bg.wasm | tr -d ' ')" \
    "$(gzip -9 -c shells/web/pkg/qp_wasm_bg.wasm | wc -c | tr -d ' ')" >shells/web/pkg/sizes.json
  (cd shells/web && { [ -d node_modules ] || npm ci --no-audit --no-fund; })
  if ! (cd shells/web && node -e "const fs=require('fs');process.exit(fs.existsSync(require('playwright').chromium.executablePath())?0:1)"); then
    (cd shells/web && npx playwright install chromium)
  fi
  mkdir -p "$d"
  (cd shells/web && node tests/parity.mjs "$qp" "$d/parity") || fail "parity or network test failed"
  (cd shells/web && node tests/settings.mjs "$qp") || fail "settings page test failed"
  (cd shells/web && node tests/gallery.mjs "$qp") || fail "gallery test failed"
  (cd shells/web && node tests/paint-paths.mjs "$qp" --browsers chromium) || fail "paint path test failed"
  (cd shells/web && node tests/paint-canvas.mjs "$qp" --browsers chromium) || fail "canvas path screenshot test failed"
  (cd shells/web && node screens.mjs "$qp" "$d/a" && node screens.mjs "$qp" "$d/b") || fail "screenshots failed"
  for f in "$d"/a/*.png; do
    b="$(basename "$f")"
    n=$((n + 1))
    case "$b" in
      *.page.png)
        # browser chrome: bounded antialias tolerance (shells/web/tests/pagecmp.mjs)
        (cd shells/web && node tests/pagecmp.mjs "$f" "$d/b/$b" 2>/dev/null) ||
          { echo "$b: two runs differ" >&2; bad=$((bad + 1)); }
        continue
        ;;
    esac
    cmp -s "$f" "$d/b/$b" || { echo "$b: two runs differ" >&2; bad=$((bad + 1)); continue; }
    if ! cmp -s "$f" "fixtures/golden/demo/$b"; then
      mkdir -p "$target_dir/golden-actual/demo"
      cp "$f" "$target_dir/golden-actual/demo/$b"
      echo "$b: differs from fixtures/golden/demo/$b (actual: $target_dir/golden-actual/demo/$b)" >&2
      bad=$((bad + 1))
    fi
  done
  for f in fixtures/golden/demo/*.png; do
    [ -f "$d/a/$(basename "$f")" ] || { echo "$(basename "$f"): golden not produced" >&2; bad=$((bad + 1)); }
  done
  # (R5/S) The v2 goldens: R5's presentation defaults (screens.mjs `V2`), the same checks.
  for f in "$d"/a/v2/*.png; do
    b="v2/$(basename "$f")"
    n=$((n + 1))
    case "$b" in
      *.page.png)
        (cd shells/web && node tests/pagecmp.mjs "$f" "$d/b/$b" 2>/dev/null) ||
          { echo "$b: two runs differ" >&2; bad=$((bad + 1)); }
        continue
        ;;
    esac
    cmp -s "$f" "$d/b/$b" || { echo "$b: two runs differ" >&2; bad=$((bad + 1)); continue; }
    if ! cmp -s "$f" "fixtures/golden/demo/$b"; then
      mkdir -p "$target_dir/golden-actual/demo/v2"
      cp "$f" "$target_dir/golden-actual/demo/$b"
      echo "$b: differs from fixtures/golden/demo/$b (actual: $target_dir/golden-actual/demo/$b)" >&2
      bad=$((bad + 1))
    fi
  done
  for f in fixtures/golden/demo/v2/*.png; do
    [ -f "$d/a/v2/$(basename "$f")" ] || { echo "v2/$(basename "$f"): golden not produced" >&2; bad=$((bad + 1)); }
  done
  [ "$bad" -eq 0 ] || fail "$bad screenshot check(s) failed"
  note "same origin only; browser = shell-less digests; settings page; gallery; paint paths (semantic reads identical); canvas within tolerance; $n images x2 (scenes identical, pages within chrome tolerance), scenes = goldens (v1 pinned to the pre-R5 designs, v2 = R5 defaults with story pinned to pavka, both with clock off; p4 = the presentation of RP4, rows with their time, look pinned to oltan; rd2 = the presentation now, tavek/tavek-tel)"
}

step_arch() {
  command -v python3 >/dev/null 2>&1 || fail "python3 not found (scripts/arch-check needs it)"
  scripts/arch-check --self-test || fail "arch-check self-test failed"
  scripts/arch-check || fail "architecture rules broken (scripts/arch-check)"
  python3 scripts/refactor/moved.py --self-test || fail "moved.py self-test failed"
  note "self-tests pass (arch-check, moved.py); no error outside the debt baseline"
}

step_config() {
  command -v python3 >/dev/null 2>&1 || fail "python3 not found (scripts/arch-check needs it)"
  # A shallow clone has no merge base: the fallback would compare HEAD with itself and
  # judge nothing, so say SKIP instead of passing (CI clones with full history, K93).
  [ "$(git rev-parse --is-shallow-repository 2>/dev/null)" != "true" ] \
    || skip "shallow clone: no base revision for the config diff"
  local rc=0
  scripts/arch-check --config-diff || rc=$?
  [ "$rc" -ne 2 ] || skip "no base revision for the config diff (shallow clone?)"
  [ "$rc" -eq 0 ] || fail "config diff: a loosening cites no decision, or the base is not a commit (see above)"
  note "config/baseline diff against main printed; only tightening or cited loosening"
}

step_design() {
  command -v python3 >/dev/null 2>&1 || fail "python3 not found (scripts/design-contract needs it)"
  scripts/design-check || fail "design check failed (scripts/design-check)"
  note "non-legacy looks pass P1-P13 and H1-H3; contract tables equal the look record"
}

# --- run -------------------------------------------------------------------

total_start=$SECONDS
run_step fmt step_fmt
run_step clippy step_clippy
run_step test step_test
run_step schema step_schema
run_step fixtures step_fixtures
run_step golden step_golden
run_step views step_views
run_step wasm step_wasm
run_step hygiene step_hygiene
run_step browser step_browser
run_step compute step_compute
run_step digest step_digest
run_step protocol step_protocol
run_step conform step_conform
run_step replica step_replica
run_step replay step_replay
run_step shell step_shell
run_step demo step_demo
run_step arch step_arch
run_step config step_config
run_step design step_design
total=$((SECONDS - total_start))

failed=0
echo
echo "================================================================"
echo "== summary"
echo "================================================================"
printf '%-3s %-10s %-6s %6s  %s\n' "#" "step" "result" "time" "note"
i=0
while [ "$i" -lt "${#names[@]}" ]; do
  printf '%-3s %-10s %-6s %5ss  %s\n' "$((i + 1))" "${names[$i]}" "${results[$i]}" "${times[$i]}" "${notes[$i]}"
  [ "${results[$i]}" = FAIL ] && failed=$((failed + 1))
  i=$((i + 1))
done
echo "total: ${total}s"

if [ "$failed" -gt 0 ]; then
  echo "verify: $failed step(s) FAILED"
  exit 1
fi
echo "verify: OK"
