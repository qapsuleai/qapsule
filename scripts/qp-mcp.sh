#!/usr/bin/env bash
# R5B/T0: start the Qapsule MCP server for an MCP client (.mcp.json, docs/AGENTS.md).
# Builds `qp` once if missing. Principal and surface come from the environment
# (QP_MCP_PRINCIPAL, default `claude`; QP_MCP_SURFACE, default: the chosen surface).
# Holds no secret; the server listens on stdio (or 127.0.0.1 with --http).
set -euo pipefail
cd "$(dirname "$0")/.."
QP="${CARGO_TARGET_DIR:-target}/release/qp"
if [ ! -x "$QP" ]; then
  cargo build -q -p qp-cli --release >&2
fi
args=(mcp serve "--principal" "${QP_MCP_PRINCIPAL:-claude}")
[ -n "${QP_MCP_SURFACE:-}" ] && args+=("--surface" "$QP_MCP_SURFACE")
exec "$QP" "${args[@]}" "$@"
