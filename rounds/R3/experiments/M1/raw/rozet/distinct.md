# M1 distinctness — `rozet`

Main metric (weakest `K` pair in the worst condition): **2.55** — `continuation.down` vs `breakout.down` at `16/light/gray`.

Lowest `K` mean over conditions: 4.06. Primitives outside the box: 0.

## Weakest pair per channel (worst condition)

| channel | ΔE | condition | a | b |
|---|---:|---|---|---|
| confidence | 1.49 | 16/dark/gray | `flow.up|final|s0.6|cnone|n1|f1|-` | `flow.up|final|s0.6|c0.3|n1|f1|-` |
| confirmations | 2.17 | 24/light/protan | `flow.down|final|s0.6|c0.6|n1|f1|-` | `flow.down|final|s0.6|c0.6|n2|f1|-` |
| direction | 2.57 | 24/light/gray | `breakout.up|final|s0.6|c0.6|n1|f1|-` | `breakout.down|final|s0.6|c0.6|n1|f1|-` |
| freshness | 10.50 | 16/light/gray | `extreme.down|final|s0.6|c0.6|n1|f1|-` | `extreme.down|final|s0.6|c0.6|n1|f0.3|-` |
| phase | 2.02 | 16/light/gray | `flow.down|final|s0.6|c0.6|n1|f1|-` | `flow.down|failed|s0.6|c0.6|n1|f1|-` |
| server | 1.65 | 16/light/protan | `flow.neutral|final|s0.6|c0.6|n1|f1|-` | `flow.neutral|final|s0.6|c0.6|n1|f1|srv` |
| strength | 2.27 | 24/dark/gray | `extreme.down|final|s0.2|c0.6|n1|f1|-` | `extreme.down|final|s0.9|c0.6|n1|f1|-` |

## Per condition (weakest pair / mean, mean ΔE76)

| size | theme | vision | K min | K mean | K weakest pair | direction min | phase min | strength min | confirmations min | confidence min | freshness min | server min |
|---:|---|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 16 | light | normal | 3.74 | 7.88 | continuation.up / reversal.up | 7.92 | 2.84 | 3.33 | 2.25 | 2.38 | 11.43 | 1.65 |
| 16 | light | deutan | 2.78 | 5.19 | continuation.up / breakout.up | 4.45 | 2.15 | 2.48 | 2.26 | 2.39 | 10.93 | 1.66 |
| 16 | light | protan | 2.66 | 4.59 | continuation.up / breakout.up | 3.30 | 2.30 | 2.38 | 2.25 | 2.35 | 10.81 | 1.65 |
| 16 | light | tritan | 3.88 | 9.04 | breakout.up / flow.up | 7.99 | 2.83 | 3.42 | 2.25 | 2.37 | 11.49 | 1.65 |
| 16 | light | gray | 2.55 | 4.06 | continuation.down / breakout.down | 2.58 | 2.02 | 2.37 | 2.25 | 2.35 | 10.50 | 1.65 |
| 24 | light | normal | 4.16 | 8.74 | continuation.up / breakout.up | 8.79 | 3.06 | 3.54 | 2.17 | 2.50 | 12.31 | 2.00 |
| 24 | light | deutan | 3.08 | 5.96 | continuation.up / breakout.up | 4.80 | 2.43 | 2.71 | 2.17 | 2.51 | 11.79 | 2.00 |
| 24 | light | protan | 2.97 | 5.35 | continuation.up / breakout.up | 3.53 | 2.55 | 2.55 | 2.17 | 2.48 | 11.66 | 2.00 |
| 24 | light | tritan | 4.27 | 9.82 | continuation.up / breakout.up | 8.68 | 3.08 | 3.61 | 2.17 | 2.49 | 12.27 | 2.00 |
| 24 | light | gray | 2.57 | 4.67 | breakout.up / breakout.down | 2.57 | 2.25 | 2.54 | 2.17 | 2.48 | 11.29 | 2.00 |
| 32 | light | normal | 4.81 | 9.61 | breakout.up / flow.up | 9.48 | 3.21 | 3.78 | 2.20 | 2.47 | 13.09 | 2.21 |
| 32 | light | deutan | 3.64 | 6.70 | breakout.up / flow.up | 5.91 | 2.55 | 2.93 | 2.20 | 2.49 | 12.62 | 2.22 |
| 32 | light | protan | 3.46 | 6.05 | breakout.up / flow.up | 4.57 | 2.65 | 2.75 | 2.19 | 2.45 | 12.50 | 2.21 |
| 32 | light | tritan | 4.65 | 10.62 | breakout.up / flow.up | 9.29 | 3.22 | 3.79 | 2.19 | 2.46 | 12.91 | 2.21 |
| 32 | light | gray | 3.43 | 5.26 | breakout.up / breakout.down | 3.43 | 2.41 | 2.75 | 2.19 | 2.45 | 12.07 | 2.21 |
| 16 | dark | normal | 4.02 | 9.56 | continuation.up / reversal.up | 7.64 | 3.28 | 4.29 | 2.42 | 1.54 | 16.03 | 1.72 |
| 16 | dark | deutan | 2.86 | 6.26 | continuation.up / extreme.up | 5.07 | 2.58 | 3.04 | 2.42 | 1.51 | 15.16 | 1.71 |
| 16 | dark | protan | 3.04 | 5.11 | continuation.down / reversal.down | 4.27 | 2.61 | 3.08 | 2.42 | 1.51 | 14.85 | 1.71 |
| 16 | dark | tritan | 4.07 | 11.02 | continuation.up / reversal.up | 7.64 | 3.28 | 4.39 | 2.42 | 1.52 | 15.96 | 1.72 |
| 16 | dark | gray | 2.64 | 4.27 | breakout.up / breakout.down | 2.64 | 2.44 | 2.37 | 2.42 | 1.49 | 14.65 | 1.71 |
| 24 | dark | normal | 4.70 | 10.03 | continuation.up / breakout.up | 8.22 | 3.13 | 3.93 | 2.19 | 1.94 | 15.70 | 1.93 |
| 24 | dark | deutan | 3.32 | 6.87 | continuation.up / breakout.up | 5.69 | 2.59 | 2.89 | 2.19 | 1.92 | 14.86 | 1.93 |
| 24 | dark | protan | 3.69 | 5.72 | continuation.up / breakout.up | 4.38 | 2.61 | 2.82 | 2.20 | 1.93 | 14.35 | 1.93 |
| 24 | dark | tritan | 4.74 | 11.38 | continuation.up / breakout.up | 8.17 | 3.13 | 3.99 | 2.19 | 1.93 | 15.60 | 1.93 |
| 24 | dark | gray | 2.78 | 4.91 | breakout.up / breakout.down | 2.78 | 2.44 | 2.27 | 2.19 | 1.92 | 14.12 | 1.93 |
| 32 | dark | normal | 5.05 | 10.48 | breakout.up / flow.up | 8.59 | 3.23 | 4.23 | 2.26 | 2.07 | 15.32 | 2.08 |
| 32 | dark | deutan | 3.85 | 7.37 | breakout.up / flow.up | 6.14 | 2.73 | 3.25 | 2.26 | 2.06 | 14.76 | 2.07 |
| 32 | dark | protan | 4.02 | 6.18 | breakout.down / flow.down | 5.13 | 2.73 | 3.08 | 2.26 | 2.07 | 14.27 | 2.08 |
| 32 | dark | tritan | 5.08 | 11.77 | breakout.up / flow.up | 8.51 | 3.23 | 4.28 | 2.26 | 2.07 | 15.31 | 2.08 |
| 32 | dark | gray | 3.48 | 5.37 | breakout.up / flow.down | 3.53 | 2.70 | 2.65 | 2.26 | 2.06 | 13.94 | 2.08 |
