#!/usr/bin/env bash
# R3/H — launch fresh reader agents (`claude -p`) for the M1 reading test and the M2
# comprehension test. Each reader is a new, isolated process that sees only:
#   M1: the task, the kind / direction / strength / phase vocabulary, the legend text (only
#       in the `legend` condition) and the sheet image(s) of ONE alphabet;
#   M2: the task, the vocabulary and the 24 sentences of ONE realizer in ONE language.
# The reader runs in a fresh temporary directory that holds only the sheet images
# (M1) or nothing (M2), with --restricted (file tools confined to that directory),
# --strict-mcp-config (no MCP servers), the Read tool only (M1) or no tools (M2), and no
# session persistence. The answer key and other alternatives are never copied there.
#
# Usage:
#   run_readers.sh [m1] <alt> <nolegend|legend|learn> <n_agents>
#   run_readers.sh m2   <alt> <tr|en> <n_agents>
#
# Environment:
#   SEED_BASE  first sheet seed (default 1); reader k (1-based) gets seed SEED_BASE + k - 1
#   MODEL      reader model alias (default sonnet)
#   DRY_RUN=1  write sheets and prompts, do not call claude
#
# Output per reader: <experiments>/<M1|M2>/raw/<alt>/readers/<cond|lang>_s<seed>_r<k>/
#   prompt.txt   the exact prompt sent
#   answer.json  raw stdout of `claude -p --output-format json` (result, usage, cost)
#   stderr.txt   raw stderr
#   meta.json    experiment, alt, condition/lang, seed, reader id, model
# A reader directory that already has a non-empty answer.json is skipped (no re-spend).
# Score afterwards with: harness m1 score --alt <alt>  /  harness m2 score --alt <alt>
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exp="$(cd "$here/.." && pwd)"

experiment=m1
if [ "${1:-}" = "m1" ] || [ "${1:-}" = "m2" ]; then
  experiment="$1"
  shift
fi
if [ $# -ne 3 ]; then
  echo "usage: run_readers.sh [m1] <alt> <nolegend|legend|learn> <n> | run_readers.sh m2 <alt> <tr|en> <n>" >&2
  exit 2
fi
alt="$1"
cond="$2"
n="$3"
seed_base="${SEED_BASE:-1}"
model="${MODEL:-sonnet}"

case "$alt" in
  *[!a-z0-9_-]* | "") echo "bad alternative name: $alt" >&2; exit 2 ;;
esac
case "$experiment:$cond" in
  m1:nolegend | m1:legend | m1:learn | m2:tr | m2:en) ;;
  *) echo "bad condition '$cond' for $experiment" >&2; exit 2 ;;
esac
case "$n" in
  '' | *[!0-9]*) echo "n_agents must be a number" >&2; exit 2 ;;
esac

cargo build --release --quiet --manifest-path "$here/Cargo.toml"
bin="$here/target/release/harness"
if [ "${DRY_RUN:-0}" != "1" ] && ! command -v claude >/dev/null 2>&1; then
  echo "claude CLI not found" >&2
  exit 1
fi

if [ "$experiment" = "m1" ]; then
  raw="$exp/M1/raw/$alt"
else
  raw="$exp/M2/raw/$alt"
fi

k=1
while [ "$k" -le "$n" ]; do
  seed=$((seed_base + k - 1))
  tag="${cond}_s${seed}"
  run="$raw/readers/${tag}_r${k}"
  if [ -s "$run/answer.json" ]; then
    echo "skip $run (already answered)"
    k=$((k + 1))
    continue
  fi
  tmpbase="${TMPDIR:-/tmp}"
  tmp="$(mktemp -d "${tmpbase%/}/r3h-reader.XXXXXX")"
  tmp="$(cd "$tmp" && pwd -P)"
  if [ "$experiment" = "m1" ]; then
    "$bin" m1 sheets --alt "$alt" --seed "$seed" --cond "$cond" >/dev/null
    cp "$raw/sheets/$tag/"*.png "$tmp/"
    tools="Read"
  else
    "$bin" m2 sheets --alt "$alt" --lang "$cond" --seed "$seed" >/dev/null
    tools=""
  fi
  mkdir -p "$run"
  # The only substitution: the absolute path of the reader's private directory.
  sed "s#@DIR@#$tmp#g" "$raw/prompts/$tag.txt" > "$run/prompt.txt"
  if [ "$experiment" = "m1" ]; then
    printf '{\n  "experiment": "m1",\n  "alt": "%s",\n  "condition": "%s",\n  "seed": %s,\n  "reader": "r%s",\n  "model": "%s"\n}\n' \
      "$alt" "$cond" "$seed" "$k" "$model" > "$run/meta.json"
  else
    printf '{\n  "experiment": "m2",\n  "alt": "%s",\n  "lang": "%s",\n  "seed": %s,\n  "reader": "r%s",\n  "model": "%s"\n}\n' \
      "$alt" "$cond" "$seed" "$k" "$model" > "$run/meta.json"
  fi
  if [ "${DRY_RUN:-0}" = "1" ]; then
    echo "dry run: $run/prompt.txt"
  else
    echo "reader r$k ($experiment $alt $cond seed $seed) ..."
    args=(-p --model "$model" --tools "$tools" --restricted --strict-mcp-config
      --permission-mode dontAsk --no-session-persistence --output-format json)
    if [ -n "$tools" ]; then
      args+=(--allowedTools "$tools")
    fi
    set +e
    (cd "$tmp" && claude "${args[@]}" < "$run/prompt.txt") \
      > "$run/answer.json" 2> "$run/stderr.txt"
    status=$?
    set -e
    if [ "$status" -ne 0 ]; then
      echo "reader r$k failed (exit $status); see $run/stderr.txt" >&2
    fi
  fi
  rm -rf "$tmp"
  k=$((k + 1))
done
