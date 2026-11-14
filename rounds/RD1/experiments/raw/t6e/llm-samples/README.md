# T6e model samples

Collected by the orchestrator (RD1). Each sample is a fresh Claude Code sub-agent
(`general-purpose`, model alias `haiku` = small or `sonnet` = large, default temperature)
told: "Use the Read tool exactly once to read <the prompt file>; do exactly what it says;
use no other tools; reply only with the requested JSON object." The prompt file is the text
after `=== PROMPT START ===` in `../llm-prompt.md` (7 143 bytes).

Measurement caveats (honest limits):
- Durations are sub-agent wall clock including one Read tool round trip and agent start-up;
  a direct API call would be faster. Treat them as upper bounds.
- Token counts: only the sub-agent total is reported by the harness (~49–51 k), dominated by
  the agent system prompt; it is NOT the cost of a direct API call. For cost, use prompt size
  (~2 k input tokens for 7.1 KB) and output size (bytes/~3.5) instead.
- The raw text is the sub-agent's final reply, copied verbatim.
