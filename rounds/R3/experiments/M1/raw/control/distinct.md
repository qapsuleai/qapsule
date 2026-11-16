# M1 distinctness — `control`

Main metric (weakest `K` pair in the worst condition): **0.00** — `continuation.up` vs `continuation.down` at `16/light/normal`.

Lowest `K` mean over conditions: 0.69. Primitives outside the box: 0.

## Weakest pair per channel (worst condition)

| channel | ΔE | condition | a | b |
|---|---:|---|---|---|
| confidence | 0.00 | 16/light/normal | `continuation.up|final|s0.6|cnone|n1|f1|-` | `continuation.up|final|s0.6|c0.3|n1|f1|-` |
| confirmations | 0.00 | 16/light/normal | `continuation.up|final|s0.6|c0.6|n1|f1|-` | `continuation.up|final|s0.6|c0.6|n2|f1|-` |
| direction | 0.00 | 16/light/normal | `continuation.up|final|s0.6|c0.6|n1|f1|-` | `continuation.down|final|s0.6|c0.6|n1|f1|-` |
| freshness | 0.00 | 16/light/normal | `continuation.up|final|s0.6|c0.6|n1|f1|-` | `continuation.up|final|s0.6|c0.6|n1|f0.3|-` |
| phase | 0.00 | 16/light/normal | `continuation.up|candidate|s0.6|c0.6|n1|f1|-` | `continuation.up|final|s0.6|c0.6|n1|f1|-` |
| server | 0.00 | 16/light/normal | `continuation.up|final|s0.6|c0.6|n1|f1|-` | `continuation.up|final|s0.6|c0.6|n1|f1|srv` |
| strength | 0.00 | 16/light/normal | `continuation.up|final|s0.2|c0.6|n1|f1|-` | `continuation.up|final|s0.9|c0.6|n1|f1|-` |

## Per condition (weakest pair / mean, mean ΔE76)

| size | theme | vision | K min | K mean | K weakest pair | direction min | phase min | strength min | confirmations min | confidence min | freshness min | server min |
|---:|---|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 16 | light | normal | 0.00 | 1.95 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 16 | light | deutan | 0.00 | 1.63 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 16 | light | protan | 0.00 | 1.72 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 16 | light | tritan | 0.00 | 1.64 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 16 | light | gray | 0.00 | 0.69 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 24 | light | normal | 0.00 | 2.12 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 24 | light | deutan | 0.00 | 1.83 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 24 | light | protan | 0.00 | 1.91 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 24 | light | tritan | 0.00 | 1.75 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 24 | light | gray | 0.00 | 0.75 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 32 | light | normal | 0.00 | 2.21 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 32 | light | deutan | 0.00 | 1.89 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 32 | light | protan | 0.00 | 1.95 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 32 | light | tritan | 0.00 | 1.88 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 32 | light | gray | 0.00 | 0.82 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 16 | dark | normal | 0.00 | 1.32 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 16 | dark | deutan | 0.00 | 1.32 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 16 | dark | protan | 0.00 | 1.27 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 16 | dark | tritan | 0.00 | 1.15 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 16 | dark | gray | 0.00 | 1.01 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 24 | dark | normal | 0.00 | 1.33 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 24 | dark | deutan | 0.00 | 1.34 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 24 | dark | protan | 0.00 | 1.29 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 24 | dark | tritan | 0.00 | 1.17 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 24 | dark | gray | 0.00 | 1.02 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 32 | dark | normal | 0.00 | 1.56 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 32 | dark | deutan | 0.00 | 1.57 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 32 | dark | protan | 0.00 | 1.51 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 32 | dark | tritan | 0.00 | 1.37 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| 32 | dark | gray | 0.00 | 1.20 | continuation.up / continuation.down | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
