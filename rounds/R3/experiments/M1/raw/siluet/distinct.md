# M1 distinctness — `siluet`

Main metric (weakest `K` pair in the worst condition): **4.60** — `reversal.down` vs `flow.up` at `16/light/gray`.

Lowest `K` mean over conditions: 6.72. Primitives outside the box: 0.

## Weakest pair per channel (worst condition)

| channel | ΔE | condition | a | b |
|---|---:|---|---|---|
| confidence | 2.41 | 16/light/protan | `extreme.up|final|s0.6|cnone|n1|f1|-` | `extreme.up|final|s0.6|c0.3|n1|f1|-` |
| confirmations | 1.16 | 32/light/protan | `continuation.down|final|s0.6|c0.6|n1|f1|-` | `continuation.down|final|s0.6|c0.6|n2|f1|-` |
| direction | 5.07 | 16/light/gray | `reversal.up|final|s0.6|c0.6|n1|f1|-` | `reversal.down|final|s0.6|c0.6|n1|f1|-` |
| freshness | 4.97 | 16/light/protan | `continuation.up|final|s0.6|c0.6|n1|f1|-` | `continuation.up|final|s0.6|c0.6|n1|f0.3|-` |
| phase | 1.64 | 24/light/gray | `squeeze.neutral|candidate|s0.6|c0.6|n1|f1|-` | `squeeze.neutral|final|s0.6|c0.6|n1|f1|-` |
| server | 2.84 | 16/light/protan | `breakout.up|final|s0.6|c0.6|n1|f1|-` | `breakout.up|final|s0.6|c0.6|n1|f1|srv` |
| strength | 2.37 | 16/light/protan | `continuation.up|final|s0.2|c0.6|n1|f1|-` | `continuation.up|final|s0.9|c0.6|n1|f1|-` |

## Per condition (weakest pair / mean, mean ΔE76)

| size | theme | vision | K min | K mean | K weakest pair | direction min | phase min | strength min | confirmations min | confidence min | freshness min | server min |
|---:|---|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 16 | light | normal | 7.16 | 21.10 | reversal.up / flow.up | 18.39 | 2.62 | 3.26 | 1.25 | 4.06 | 6.98 | 2.85 |
| 16 | light | deutan | 5.61 | 12.69 | extreme.up / flow.up | 12.19 | 2.08 | 2.58 | 1.24 | 2.87 | 5.51 | 2.85 |
| 16 | light | protan | 5.14 | 10.63 | extreme.up / flow.up | 7.98 | 1.93 | 2.37 | 1.20 | 2.41 | 4.97 | 2.84 |
| 16 | light | tritan | 7.37 | 23.80 | extreme.up / flow.up | 14.17 | 2.63 | 3.33 | 1.25 | 4.64 | 7.51 | 2.85 |
| 16 | light | gray | 4.60 | 6.72 | reversal.down / flow.up | 5.07 | 1.71 | 2.46 | 1.22 | 2.58 | 5.16 | 2.85 |
| 24 | light | normal | 7.72 | 22.20 | extreme.up / flow.up | 19.92 | 2.48 | 3.52 | 1.21 | 4.15 | 7.04 | 3.02 |
| 24 | light | deutan | 6.05 | 13.58 | extreme.up / flow.up | 13.22 | 1.97 | 2.76 | 1.21 | 2.93 | 5.53 | 3.02 |
| 24 | light | protan | 5.59 | 11.56 | extreme.up / flow.up | 8.98 | 1.85 | 2.55 | 1.16 | 2.46 | 4.99 | 3.01 |
| 24 | light | tritan | 7.56 | 24.68 | extreme.up / flow.up | 16.17 | 2.35 | 3.46 | 1.22 | 4.73 | 7.49 | 3.01 |
| 24 | light | gray | 5.11 | 7.32 | squeeze.neutral / flow.neutral | 5.66 | 1.64 | 2.64 | 1.19 | 2.63 | 5.15 | 3.01 |
| 32 | light | normal | 7.81 | 22.87 | extreme.up / flow.up | 20.73 | 2.49 | 3.61 | 1.21 | 4.15 | 7.03 | 3.12 |
| 32 | light | deutan | 6.16 | 14.13 | extreme.up / flow.up | 13.31 | 1.99 | 2.82 | 1.20 | 2.92 | 5.50 | 3.12 |
| 32 | light | protan | 5.72 | 12.13 | extreme.up / flow.up | 9.30 | 1.87 | 2.61 | 1.16 | 2.44 | 4.97 | 3.11 |
| 32 | light | tritan | 7.49 | 25.18 | extreme.up / flow.up | 16.97 | 2.31 | 3.50 | 1.21 | 4.75 | 7.42 | 3.12 |
| 32 | light | gray | 5.45 | 7.66 | squeeze.neutral / flow.neutral | 5.77 | 1.67 | 2.70 | 1.18 | 2.59 | 5.15 | 3.12 |
| 16 | dark | normal | 8.48 | 25.00 | extreme.up / flow.up | 25.65 | 2.98 | 4.13 | 1.24 | 3.87 | 8.82 | 3.14 |
| 16 | dark | deutan | 6.91 | 16.45 | extreme.up / flow.up | 16.22 | 2.52 | 3.33 | 1.22 | 2.98 | 7.23 | 3.06 |
| 16 | dark | protan | 6.53 | 13.79 | reversal.down / flow.down | 10.64 | 2.65 | 3.25 | 1.19 | 2.98 | 6.73 | 3.10 |
| 16 | dark | tritan | 8.46 | 27.99 | extreme.up / flow.up | 22.58 | 2.98 | 4.14 | 1.24 | 4.31 | 9.10 | 3.18 |
| 16 | dark | gray | 5.87 | 9.17 | reversal.up / reversal.down | 5.87 | 2.60 | 3.30 | 1.18 | 3.09 | 6.83 | 3.03 |
| 24 | dark | normal | 9.07 | 25.19 | extreme.up / flow.up | 26.22 | 2.80 | 4.26 | 1.27 | 4.12 | 8.58 | 3.13 |
| 24 | dark | deutan | 7.41 | 16.87 | extreme.up / flow.up | 17.00 | 2.32 | 3.46 | 1.26 | 3.15 | 7.02 | 3.09 |
| 24 | dark | protan | 7.13 | 14.22 | reversal.down / flow.down | 11.36 | 2.51 | 3.27 | 1.23 | 3.15 | 6.53 | 3.11 |
| 24 | dark | tritan | 9.03 | 28.12 | extreme.up / flow.up | 23.34 | 2.75 | 4.27 | 1.25 | 4.61 | 8.88 | 3.15 |
| 24 | dark | gray | 6.57 | 9.71 | reversal.down / flow.up | 6.68 | 2.39 | 3.32 | 1.20 | 3.26 | 6.63 | 3.07 |
| 32 | dark | normal | 9.29 | 25.13 | extreme.up / flow.up | 26.37 | 2.89 | 4.26 | 1.37 | 4.11 | 8.48 | 3.11 |
| 32 | dark | deutan | 7.59 | 16.98 | extreme.up / flow.up | 17.39 | 2.39 | 3.46 | 1.35 | 3.13 | 6.96 | 3.09 |
| 32 | dark | protan | 7.43 | 14.36 | reversal.down / flow.down | 11.31 | 2.58 | 3.26 | 1.32 | 3.09 | 6.40 | 3.11 |
| 32 | dark | tritan | 9.24 | 27.99 | extreme.up / flow.up | 23.53 | 2.83 | 4.26 | 1.32 | 4.62 | 8.81 | 3.12 |
| 32 | dark | gray | 6.69 | 9.91 | reversal.up / reversal.down | 6.69 | 2.46 | 3.30 | 1.26 | 3.24 | 6.51 | 3.08 |
