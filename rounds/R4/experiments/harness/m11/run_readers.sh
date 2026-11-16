#!/usr/bin/env bash
# R4/H-M11 — launch fresh persona reader agents (`claude -p`) for the M11 reading test
# (SPEC §0.3, §5(b)). Each reader is a new, isolated process that sees only:
#   the persona description (SPEC §1), the task, the four answer options, the style's
#   one-sentence explanation (only in the `legend` condition) and ONE sheet image whose
#   file name is a seeded code (never the style name).
# The reader runs in a fresh temporary directory that holds only that image, with
# --restricted (file tools confined to that directory), --strict-mcp-config (no MCP
# servers), the Read tool only, dontAsk permissions and no session persistence. The hidden
# key (raw/keys/, raw/key.json) and other sheets are never copied there. Every prompt is
# checked against the forbidden-word list (`m11 check-prompt`) before it is sent.
#
# Usage:
#   run_readers.sh <style> <F|H> <nolegend|legend> <n_agents>
#
# Environment:
#   SEED_BASE  first sheet seed (default 1); reader k (1-based) sees seed SEED_BASE + k - 1
#   ALPHABET   siluet (default, the product alphabet) or rozet
#   MODEL      reader model alias (default sonnet)
#   M11_ROOT   M11 root (default rounds/R4/experiments/M11)
#   DRY_RUN=1  write the sheet, prompt and meta; do not call claude
#
# Output per reader: <root>/raw/readers/<persona>_<cond>_<code>_r<k>/
#   prompt.txt   the exact prompt sent
#   answer.json  raw stdout of `claude -p --output-format json` (result, usage, cost)
#   stderr.txt   raw stderr
#   meta.json    experiment, code, persona, condition, seed, reader, model, alphabet
#                (no style name: `m11 score` resolves it through raw/keys/<code>.json)
# A reader directory that already has a non-empty answer.json is skipped (no re-spend).
# Score afterwards with: m11 score
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ $# -ne 4 ]; then
  echo "usage: run_readers.sh <style> <F|H> <nolegend|legend> <n_agents>" >&2
  exit 2
fi
style="$1"
persona="$2"
cond="$3"
n="$4"
seed_base="${SEED_BASE:-1}"
alphabet="${ALPHABET:-siluet}"
model="${MODEL:-sonnet}"
root="${M11_ROOT:-$(cd "$here/../.." && pwd)/M11}"

case "$style" in
  [a-z]*) ;;
  *) echo "bad style name: $style" >&2; exit 2 ;;
esac
case "$style" in
  *[!a-z0-9_-]*) echo "bad style name: $style" >&2; exit 2 ;;
esac
case "$persona" in F | H) ;; *) echo "persona must be F or H" >&2; exit 2 ;; esac
case "$cond" in nolegend | legend) ;; *) echo "condition must be nolegend or legend" >&2; exit 2 ;; esac
case "$alphabet" in siluet | rozet) ;; *) echo "ALPHABET must be siluet or rozet" >&2; exit 2 ;; esac
case "$n" in '' | *[!0-9]*) echo "n_agents must be a number" >&2; exit 2 ;; esac

cargo build --release --quiet --manifest-path "$here/Cargo.toml"
bin="$here/target/release/m11"
if [ "${DRY_RUN:-0}" != "1" ] && ! command -v claude >/dev/null 2>&1; then
  echo "claude CLI not found" >&2
  exit 1
fi
raw="$root/raw"

k=1
while [ "$k" -le "$n" ]; do
  seed=$((seed_base + k - 1))
  code="$("$bin" sheets --root "$root" --style "$style" --alphabet "$alphabet" \
    --seed "$seed" --persona "$persona" --cond "$cond")"
  run="$raw/readers/${persona}_${cond}_${code}_r${k}"
  if [ -s "$run/answer.json" ]; then
    echo "skip $run (already answered)"
    k=$((k + 1))
    continue
  fi
  template="$raw/prompts/${persona}_${cond}_${code}.txt"
  # The template is what the reader reads apart from its directory path.
  "$bin" check-prompt "$template"
  tmpbase="${TMPDIR:-/tmp}"
  tmp="$(mktemp -d "${tmpbase%/}/rdr.XXXXXX")"
  tmp="$(cd "$tmp" && pwd -P)"
  cp "$raw/screens/$code.png" "$tmp/"
  mkdir -p "$run"
  # The only substitution: the absolute path of the reader's private directory.
  sed "s#@DIR@#$tmp#g" "$template" > "$run/prompt.txt"
  printf '{\n  "experiment": "m11",\n  "code": "%s",\n  "persona": "%s",\n  "condition": "%s",\n  "seed": %s,\n  "reader": "r%s",\n  "model": "%s",\n  "alphabet": "%s"\n}\n' \
    "$code" "$persona" "$cond" "$seed" "$k" "$model" "$alphabet" > "$run/meta.json"
  if [ "${DRY_RUN:-0}" = "1" ]; then
    echo "dry run: $run/prompt.txt (image $code.png)"
  else
    echo "reader r$k ($persona $cond, sheet $code) ..."
    set +e
    (cd "$tmp" && claude -p --model "$model" --tools Read --allowedTools Read --restricted \
      --strict-mcp-config --permission-mode dontAsk --no-session-persistence \
      --output-format json < "$run/prompt.txt") \
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
