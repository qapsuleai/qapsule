# M1 distinctness — `iz`

Main metric (weakest `K` pair in the worst condition): **7.55** — `breakout.up` vs `flow.up` at `16/light/protan`.

Lowest `K` mean over conditions: 10.56. Primitives outside the box: 0.

## Weakest pair per channel (worst condition)

| channel | ΔE | condition | a | b |
|---|---:|---|---|---|
| confidence | 0.64 | 24/light/protan | `reversal.up|final|s0.6|c0.3|n1|f1|-` | `reversal.up|final|s0.6|c0.8|n1|f1|-` |
| confirmations | 1.19 | 16/light/protan | `continuation.down|final|s0.6|c0.6|n1|f1|-` | `continuation.down|final|s0.6|c0.6|n2|f1|-` |
| direction | 8.50 | 16/light/gray | `flow.up|final|s0.6|c0.6|n1|f1|-` | `flow.down|final|s0.6|c0.6|n1|f1|-` |
| freshness | 3.32 | 24/light/protan | `flow.up|final|s0.6|c0.6|n1|f1|-` | `flow.up|final|s0.6|c0.6|n1|f0.3|-` |
| phase | 3.58 | 24/light/protan | `flow.up|candidate|s0.6|c0.6|n1|f1|-` | `flow.up|final|s0.6|c0.6|n1|f1|-` |
| server | 1.97 | 24/light/gray | `reversal.down|final|s0.6|c0.6|n1|f1|-` | `reversal.down|final|s0.6|c0.6|n1|f1|srv` |
| strength | 2.21 | 32/light/protan | `flow.up|final|s0.2|c0.6|n1|f1|-` | `flow.up|final|s0.9|c0.6|n1|f1|-` |

## Per condition (weakest pair / mean, mean ΔE76)

| size | theme | vision | K min | K mean | K weakest pair | direction min | phase min | strength min | confirmations min | confidence min | freshness min | server min |
|---:|---|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 16 | light | normal | 10.70 | 21.85 | breakout.up / flow.up | 18.19 | 5.98 | 3.05 | 1.19 | 1.05 | 4.30 | 4.65 |
| 16 | light | deutan | 8.24 | 15.91 | breakout.up / flow.up | 12.17 | 4.70 | 2.49 | 1.20 | 0.77 | 3.70 | 4.22 |
| 16 | light | protan | 7.55 | 14.27 | breakout.up / flow.up | 10.22 | 4.23 | 2.38 | 1.19 | 0.70 | 3.52 | 4.08 |
| 16 | light | tritan | 11.01 | 23.77 | breakout.up / flow.up | 13.08 | 6.42 | 2.85 | 1.19 | 1.10 | 4.23 | 4.45 |
| 16 | light | gray | 7.82 | 10.56 | breakout.up / flow.up | 8.50 | 4.46 | 2.39 | 1.19 | 0.73 | 3.57 | 2.07 |
| 24 | light | normal | 11.06 | 22.65 | breakout.up / flow.up | 18.55 | 4.78 | 2.92 | 1.24 | 0.97 | 4.08 | 4.43 |
| 24 | light | deutan | 8.44 | 16.66 | breakout.up / flow.up | 12.58 | 3.89 | 2.34 | 1.24 | 0.71 | 3.48 | 3.86 |
| 24 | light | protan | 7.71 | 14.97 | breakout.up / flow.up | 10.76 | 3.58 | 2.22 | 1.24 | 0.64 | 3.32 | 3.75 |
| 24 | light | tritan | 11.40 | 24.28 | breakout.up / flow.up | 13.60 | 4.93 | 2.68 | 1.24 | 1.01 | 3.95 | 4.34 |
| 24 | light | gray | 8.01 | 11.11 | breakout.up / flow.up | 9.05 | 3.72 | 2.24 | 1.24 | 0.67 | 3.36 | 1.97 |
| 32 | light | normal | 11.81 | 23.37 | breakout.up / flow.up | 19.01 | 5.58 | 2.88 | 1.27 | 0.99 | 4.24 | 5.11 |
| 32 | light | deutan | 9.01 | 17.23 | breakout.up / flow.up | 13.05 | 4.41 | 2.32 | 1.27 | 0.72 | 3.61 | 4.55 |
| 32 | light | protan | 8.26 | 15.49 | breakout.up / flow.up | 11.28 | 4.03 | 2.21 | 1.27 | 0.65 | 3.44 | 4.46 |
| 32 | light | tritan | 12.07 | 24.83 | breakout.up / flow.up | 13.93 | 5.66 | 2.60 | 1.27 | 1.05 | 4.08 | 4.79 |
| 32 | light | gray | 8.55 | 11.49 | breakout.up / flow.up | 9.47 | 4.21 | 2.22 | 1.27 | 0.69 | 3.48 | 2.20 |
| 16 | dark | normal | 13.11 | 26.62 | continuation.up / extreme.up | 20.85 | 8.68 | 4.21 | 1.33 | 1.19 | 5.89 | 6.53 |
| 16 | dark | deutan | 10.26 | 20.39 | breakout.up / extreme.up | 15.86 | 6.31 | 3.33 | 1.32 | 0.94 | 5.06 | 6.60 |
| 16 | dark | protan | 10.15 | 17.38 | breakout.down / extreme.down | 12.57 | 5.48 | 3.27 | 1.31 | 0.85 | 5.09 | 6.26 |
| 16 | dark | tritan | 13.34 | 29.10 | continuation.up / extreme.up | 16.38 | 8.99 | 4.18 | 1.33 | 1.22 | 5.88 | 5.58 |
| 16 | dark | gray | 9.54 | 13.74 | breakout.down / extreme.down | 10.76 | 4.98 | 3.19 | 1.31 | 0.90 | 5.13 | 4.51 |
| 24 | dark | normal | 13.51 | 26.21 | breakout.up / flow.up | 21.04 | 7.24 | 3.58 | 1.43 | 1.11 | 5.50 | 6.48 |
| 24 | dark | deutan | 10.73 | 20.39 | breakout.up / flow.up | 15.83 | 5.56 | 2.87 | 1.43 | 0.89 | 4.74 | 6.56 |
| 24 | dark | protan | 10.46 | 17.49 | continuation.down / extreme.down | 12.76 | 4.94 | 2.92 | 1.42 | 0.82 | 4.61 | 6.19 |
| 24 | dark | tritan | 13.72 | 28.50 | breakout.up / flow.up | 17.26 | 7.44 | 3.56 | 1.43 | 1.12 | 5.47 | 5.60 |
| 24 | dark | gray | 10.29 | 14.18 | breakout.down / extreme.down | 11.13 | 4.62 | 2.89 | 1.43 | 0.84 | 4.64 | 4.55 |
| 32 | dark | normal | 13.94 | 26.03 | breakout.up / flow.up | 20.77 | 7.48 | 3.50 | 1.47 | 1.08 | 5.31 | 6.43 |
| 32 | dark | deutan | 10.90 | 20.38 | breakout.up / extreme.up | 15.89 | 5.66 | 2.83 | 1.47 | 0.87 | 4.58 | 6.51 |
| 32 | dark | protan | 10.57 | 17.54 | breakout.down / flow.down | 12.90 | 5.05 | 2.89 | 1.47 | 0.83 | 4.47 | 6.14 |
| 32 | dark | tritan | 14.14 | 28.21 | breakout.up / flow.up | 17.28 | 7.66 | 3.49 | 1.47 | 1.08 | 5.28 | 5.56 |
| 32 | dark | gray | 10.45 | 14.39 | breakout.down / extreme.down | 11.45 | 4.69 | 2.87 | 1.47 | 0.86 | 4.51 | 4.50 |
