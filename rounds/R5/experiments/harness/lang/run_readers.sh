#!/usr/bin/env bash
# R5/T2 M17 — run the blind readers and critics of a planned run (SPEC §3.5–§3.6).
#
# Each run directory (<run>/readers/* and <run>/critics/*) holds prompt.txt and meta.json
# (blind code only; <run>/key.json is never copied). Every prompt is checked for
# forbidden words (`langh check-prompt`) before it is sent. Each reader is a fresh
# `claude -p` process in a new empty temporary directory, no tools, one turn, no MCP, no
# session persistence; the text is in the prompt (SPEC §3.4: neutral plain text).
# A directory that already has a non-empty answer.json is skipped (no re-spend).
#
# Usage:  run_readers.sh <run-dir>
# Env:    MODEL (default haiku), JOBS (parallel processes, default 5), DRY_RUN=1 (check
#         prompts only, no call), CARGO_TARGET_DIR (default <repo>/target/wt/t2-lang)
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# The main tree (also from a git worktree): the parent of the common git directory.
repo="$(dirname "$(git -C "$here" rev-parse --path-format=absolute --git-common-dir)")"
run="${1:?usage: run_readers.sh <run-dir>}"
run="$(cd "$run" && pwd)"
model="${MODEL:-haiku}"
jobs="${JOBS:-5}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo/target/wt/t2-lang}"
(cd "$here/tool" && cargo build --release --offline --quiet)
bin="$CARGO_TARGET_DIR/release/langh"

if [ "${DRY_RUN:-0}" != "1" ] && ! command -v claude >/dev/null 2>&1; then
  echo "claude CLI not found" >&2
  exit 1
fi

one() {
  local dir="$1"
  if [ -s "$dir/answer.json" ]; then
    return 0
  fi
  "$bin" check-prompt "$dir/prompt.txt"
  if [ "${DRY_RUN:-0}" = "1" ]; then
    return 0
  fi
  local tmpbase="${TMPDIR:-/tmp}"
  local tmp
  tmp="$(mktemp -d "${tmpbase%/}/r5rdr.XXXXXX")"
  set +e
  (cd "$tmp" && claude -p --model "$model" --tools "" --max-turns 1 --strict-mcp-config \
    --permission-mode dontAsk --no-session-persistence --output-format json \
    < "$dir/prompt.txt") > "$dir/answer.json" 2> "$dir/stderr.txt"
  local st=$?
  set -e
  rm -rf "$tmp"
  [ -s "$dir/stderr.txt" ] || rm -f "$dir/stderr.txt"
  if [ "$st" -ne 0 ]; then
    echo "failed ($st): $dir" >&2
    mv "$dir/answer.json" "$dir/answer.failed.json" 2>/dev/null || true
  fi
}
export -f one
export bin model

find "$run/readers" "$run/critics" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort \
  | xargs -P "$jobs" -I{} bash -c 'one "$@"' _ {}
echo "done: $(find "$run" -name answer.json -size +0 | wc -l | tr -d ' ') answered"
