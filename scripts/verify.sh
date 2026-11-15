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
#                and wasm32 in Node (same tool rule as the browser step)
#  13 protocol   fixtures/protocol/valid/* validate and fixtures/protocol/invalid/* are
#                rejected by the generated protocol schemas; transcripts and init validate
#  14 conform    qp/0 conformance transcripts pass in-process, over `qp serve` and over MCP
#  15 replica    authority <-> replica (RP2/T8, PROTOCOL §19.6): `qp replica-tape` with the
#                replica's data in step (digests equal after every step) and 25 events
#                behind (no wrong or early claim); the tapes replayed by the WASM replica in
#                Node reproduce every digest (same tool rule as the browser step)
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

# R2 views: one line per view, "<transcript stem> <qp render/explain view selection>".
VIEWS_LIST='v0-r0 --view r0
v1-three-panels --view three
v2-bollinger-band --view boll
v3-fan --view fan
v4-log --view logv
v5-conditional --view cond
d1-capsule --present fnd-overbought --surface phone --width 390 --height 300
d2-chart --present fnd-overbought --surface desktop'

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
  ran="$(uname -m)"
  if [ "$(uname -s)" = Darwin ] && [ "$(uname -m)" = arm64 ] \
    && rustup target list --installed 2>/dev/null | grep -q '^x86_64-apple-darwin$' \
    && arch -x86_64 /usr/bin/true 2>/dev/null; then
    got="$(cargo run -q --release -p qp-compute --example digest --target x86_64-apple-darwin)"
    echo "x86_64 (Rosetta): $got"
    [ "$got" = "$want" ] || fail "x86_64 digest $got != $want"
    ran="$ran, x86_64"
  fi
  local have_tools=1
  command -v node >/dev/null 2>&1 || have_tools=0
  if ! command -v wasm-pack >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/wasm-pack" ]; then have_tools=0; fi
  if [ "$mode" = 0 ] || { [ "$mode" != 1 ] && [ "$have_tools" = 0 ]; }; then
    echo "wasm32: skipped (QP_VERIFY_BROWSER=${mode:-unset}, tools: $have_tools)"
    note "$ran: $want; wasm32 skipped"
    return 0
  fi
  local wp pkg="$target_dir/wasm-bench/pkg"
  wp="$(command -v wasm-pack || echo "$HOME/.cargo/bin/wasm-pack")"
  "$wp" build crates/qp-wasm --target nodejs --profile wasm --no-opt --out-dir "$pkg"
  got="$(node -e "console.log(require('$pkg/qp_wasm.js').compute_digest())")"
  echo "wasm32 (node $(node --version)): $got"
  [ "$got" = "$want" ] || fail "wasm32 digest $got != $want"
  note "$ran, wasm32: $want"
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

step_replica() {
  build_qp
  local d="$tmp/replica" mode="${QP_VERIFY_BROWSER:-}" have_tools=1 wp pkg="$target_dir/wasm-replica/pkg"
  mkdir -p "$d"
  "$qp" replica-tape --lag 0 --out "$d/tape0.json" || fail "replica in step: digests differ or a claim is wrong"
  "$qp" replica-tape --lag 25 --out "$d/tape25.json" || fail "lagging replica: a wrong or early claim"
  command -v node >/dev/null 2>&1 || have_tools=0
  if ! command -v wasm-pack >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/wasm-pack" ]; then have_tools=0; fi
  if [ "$mode" = 0 ] || { [ "$mode" != 1 ] && [ "$have_tools" = 0 ]; }; then
    echo "wasm32 replica: skipped (QP_VERIFY_BROWSER=${mode:-unset}, tools: $have_tools)"
    note "native authority = native replica (lag 0, 25); wasm32 skipped"
    return 0
  fi
  wp="$(command -v wasm-pack || echo "$HOME/.cargo/bin/wasm-pack")"
  "$wp" build crates/qp-wasm --target nodejs --profile wasm --no-opt --out-dir "$pkg" -- --features replica
  node crates/qp-wasm/tests/replica_tape.mjs "$pkg" "$d/tape0.json" "$d/tape25.json" \
    || fail "wasm32 replica digests differ from native"
  note "native authority = native replica = wasm32 replica (lag 0, 25)"
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
