#!/usr/bin/env bash
# R4/H-M8910 — launch fresh reader agents (`claude -p`) for M8, M9, M10 (SPEC §0.3).
# Each reader is a new, isolated process that sees only its prompt and the images listed
# for it (seeded codes, never a design name), in a fresh temporary directory holding only
# those images, with --restricted (file tools confined to that directory),
# --strict-mcp-config (no MCP servers), the Read tool only, dontAsk permissions and no
# session persistence. Hidden keys (raw/keys/, raw/key.json, critics' key.json) are never
# copied there. Every prompt is checked against the forbidden-word list (`m8910
# check-prompt`) before it is sent.
#
# Usage:
#   run_readers.sh m8  <face>       <F|H> <page 1|2> <n_agents>   persona readers, M8 page
#   run_readers.sh m9  <feed>       <F|H> -          <n_agents>   persona readers, M9
#   run_readers.sh m10 <transition> <F|H> -          <n_agents>   persona readers, M10
#   run_readers.sh critic -         -     -          <n_agents>   M8 critics c1..cn
#
# Environment:
#   SEED_BASE  first seed (default 1); reader k (1-based) uses SEED_BASE + k - 1
#              (M10: the order of the sequences; critics: the letters and order)
#   MODEL      reader model alias (default sonnet)
#   EXP_ROOT   experiments root (default rounds/R4/experiments)
#   DRY_RUN=1  write the prompt, image list and meta; do not call claude
#
# Output per reader (SPEC §7): <root>/<M>/raw/readers/<persona>_…_r<k>/ (critics:
# <root>/M8/raw/critics/c<k>/): prompt.txt (exact prompt sent), answer.json (raw stdout of
# `claude -p --output-format json`), stderr.txt, meta.json (no design name). A reader that
# already has a non-empty answer.json is skipped (no re-spend). Score with
# `m8910 score --exp m8|m9|m10`.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ $# -ne 5 ]; then
  echo "usage: run_readers.sh <m8|m9|m10|critic> <design|-> <F|H|-> <page|-> <n_agents>" >&2
  exit 2
fi
exp="$1"
design="$2"
persona="$3"
page="$4"
n="$5"
seed_base="${SEED_BASE:-1}"
model="${MODEL:-sonnet}"
root="${EXP_ROOT:-$(cd "$here/../.." && pwd)}"

case "$exp" in m8 | m9 | m10 | critic) ;; *) echo "experiment must be m8, m9, m10 or critic" >&2; exit 2 ;; esac
if [ "$exp" != critic ]; then
  case "$design" in [a-z]*) ;; *) echo "bad design name: $design" >&2; exit 2 ;; esac
  case "$design" in *[!a-z0-9_-]*) echo "bad design name: $design" >&2; exit 2 ;; esac
  case "$persona" in F | H) ;; *) echo "persona must be F or H" >&2; exit 2 ;; esac
fi
if [ "$exp" = m8 ]; then
  case "$page" in 1 | 2) ;; *) echo "page must be 1 or 2" >&2; exit 2 ;; esac
fi
case "$n" in '' | *[!0-9]*) echo "n_agents must be a number" >&2; exit 2 ;; esac

(cd "$here" && cargo build --release --quiet)
bin="$(cd "$here/../../../../.." && pwd)/target/harness-m8910/release/m8910"
if [ "${DRY_RUN:-0}" != "1" ] && ! command -v claude >/dev/null 2>&1; then
  echo "claude CLI not found" >&2
  exit 1
fi

k=1
while [ "$k" -le "$n" ]; do
  seed=$((seed_base + k - 1))
  if [ "$exp" = critic ]; then
    run="$("$bin" m8-critic --root "$root" --k "$k")"
  else
    run="$("$bin" reader-setup --root "$root" --exp "$exp" --design "$design" --persona "$persona" \
      --page "${page/-/1}" --reader "$k" --seed "$seed" --model "$model")"
  fi
  if [ -s "$run/answer.json" ]; then
    echo "skip $run (already answered)"
    k=$((k + 1))
    continue
  fi
  template="$run/prompt.template.txt"
  "$bin" check-prompt "$template"
  tmpbase="${TMPDIR:-/tmp}"
  tmp="$(mktemp -d "${tmpbase%/}/rdr.XXXXXX")"
  tmp="$(cd "$tmp" && pwd -P)"
  # Copy exactly the listed images under their listed names (images.json: [[src, name], …]).
  node -e '
    const fs = require("fs"), path = require("path");
    const [list, dir] = process.argv.slice(1);
    for (const [src, name] of JSON.parse(fs.readFileSync(list, "utf8"))) {
      if (name.includes("/") || name.startsWith(".")) throw new Error("bad image name " + name);
      fs.copyFileSync(src, path.join(dir, name));
    }' "$run/images.json" "$tmp"
  # The only substitution: the absolute path of the reader's private directory.
  sed "s#@DIR@#$tmp#g" "$template" > "$run/prompt.txt"
  if [ "${DRY_RUN:-0}" = "1" ]; then
    echo "dry run: $run/prompt.txt ($(ls "$tmp" | wc -l | tr -d ' ') images)"
  else
    echo "reader $k ($exp ${persona}) ..."
    set +e
    (cd "$tmp" && claude -p --model "$model" --tools Read --allowedTools Read --restricted \
      --strict-mcp-config --permission-mode dontAsk --no-session-persistence \
      --output-format json < "$run/prompt.txt") \
      > "$run/answer.json" 2> "$run/stderr.txt"
    status=$?
    set -e
    if [ "$status" -ne 0 ]; then
      echo "reader $k failed (exit $status); see $run/stderr.txt" >&2
    fi
  fi
  rm -rf "$tmp"
  k=$((k + 1))
done
