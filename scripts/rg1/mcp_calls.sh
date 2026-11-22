#!/usr/bin/env bash
# RG1 snapshot helper: a fixed set of MCP `tools/call` requests per tool surface over
# `qp mcp serve` (synthetic, paused); writes every reply line. Usage: mcp_calls.sh <qp> <out-dir>
set -euo pipefail
qp="$1"; out="$2"; mkdir -p "$out"
init='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"rg1","version":"0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}'
call() { printf '{"jsonrpc":"2.0","id":%s,"method":"tools/call","params":{"name":"%s","arguments":%s}}\n' "$1" "$2" "$3"; }
{ printf '%s\n' "$init"; call 10 query_describe '{}'; call 11 query_capsules '{}'; call 12 replay_status '{}';
  call 13 replay_step '{"n":20}'; call 14 query_capsules '{}'; call 15 query_document '{}'; } \
  | "$qp" mcp serve --surface thin >"$out/thin.jsonl" 2>/dev/null
{ printf '%s\n' "$init"; call 10 whats_new '{}'; call 11 glossary '{}'; call 12 find '{"meaning":"breakout"}'; } \
  | "$qp" mcp serve --surface task >"$out/task.jsonl" 2>/dev/null
{ printf '%s\n' "$init"; call 10 ask '{"question":"overview"}'; } \
  | "$qp" mcp serve --surface query >"$out/query.jsonl" 2>/dev/null
{ printf '%s\n' "$init"; call 10 overview '{}'; call 11 find '{"meaning":"breakout"}'; } \
  | "$qp" mcp serve --surface mixed >"$out/mixed.jsonl" 2>/dev/null
