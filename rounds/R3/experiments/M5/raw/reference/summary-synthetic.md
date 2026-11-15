# M5 — `reference` (synthetic)

Kurallar: tema `reference.kind_direction`, sonuç `reference.atr_barrier`, sıralayıcı `reference.families`.

## Özet

| Veri | Kapanan | Gerçekleşen | Geçersiz | Süresi dolan | İsabet | Süresi dolan payı | maks \|p(25)−p(son)\| | maks \|p(50)−p(son)\| | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| synthetic-s1 | 5183 | 2468 | 2298 | 417 | 0.476 | 0.080 | 0.247 | 0.267 | 0.167 | 0.097 | 0.469 | 0.450 | 0.019 | geçti |
| synthetic-s2 | 5131 | 2421 | 2279 | 431 | 0.472 | 0.084 | 0.245 | 0.225 | 0.145 | 0.049 | 0.465 | 0.451 | 0.014 | geçti |
| synthetic-s3 | 5027 | 2344 | 2243 | 440 | 0.466 | 0.088 | 0.241 | 0.251 | 0.071 | 0.071 | 0.459 | 0.453 | 0.006 | geçti |

## 1. Kararlılık (anahtar başına)

| Veri | Anahtar | n | p(25) | p(50) | p(100) | p(200) | p(son) | ilk yarı | ikinci yarı | fark |
|---|---|---|---|---|---|---|---|---|---|---|
| synthetic-s1 | `flow/flow.institutional@1` | 2206 | 0.200 | 0.180 | 0.280 | 0.350 | 0.447 | 0.425 | 0.466 | -0.042 |
| synthetic-s1 | `extreme/extreme.rsi@1` | 382 | 0.520 | 0.520 | 0.570 | 0.535 | 0.547 | 0.517 | 0.572 | -0.055 |
| synthetic-s1 | `breakout/breakout.range@1` | 1465 | 0.520 | 0.460 | 0.440 | 0.465 | 0.486 | 0.488 | 0.484 | 0.004 |
| synthetic-s1 | `flow/flow.volume_surge@1` | 64 | 0.440 | 0.500 | — | — | 0.469 | 0.444 | 0.486 | -0.042 |
| synthetic-s1 | `reversal/reversal.rsi_divergence@1` | 291 | 0.440 | 0.460 | 0.430 | 0.435 | 0.426 | 0.424 | 0.428 | -0.003 |
| synthetic-s1 | `continuation/continuation.ma_stack@1` | 704 | 0.600 | 0.520 | 0.550 | 0.520 | 0.482 | 0.494 | 0.472 | 0.021 |
| synthetic-s1 | `squeeze/squeeze.bandwidth@1` | 71 | 0.920 | 0.960 | — | — | 0.972 | 0.944 | 0.981 | -0.037 |
| synthetic-s2 | `flow/flow.institutional@1` | 2272 | 0.200 | 0.220 | 0.300 | 0.425 | 0.445 | 0.438 | 0.453 | -0.015 |
| synthetic-s2 | `extreme/extreme.rsi@1` | 386 | 0.640 | 0.620 | 0.570 | 0.565 | 0.516 | 0.559 | 0.471 | 0.088 |
| synthetic-s2 | `breakout/breakout.range@1` | 1383 | 0.640 | 0.580 | 0.540 | 0.500 | 0.468 | 0.480 | 0.456 | 0.024 |
| synthetic-s2 | `reversal/reversal.rsi_divergence@1` | 290 | 0.480 | 0.480 | 0.460 | 0.475 | 0.459 | 0.458 | 0.459 | -0.001 |
| synthetic-s2 | `flow/flow.volume_surge@1` | 48 | 0.520 | — | — | — | 0.438 | 0.500 | 0.364 | 0.136 |
| synthetic-s2 | `continuation/continuation.ma_stack@1` | 678 | 0.400 | 0.500 | 0.530 | 0.530 | 0.499 | 0.550 | 0.457 | 0.092 |
| synthetic-s2 | `squeeze/squeeze.bandwidth@1` | 74 | 0.960 | 0.960 | — | — | 0.959 | 0.958 | 0.960 | -0.002 |
| synthetic-s3 | `flow/flow.institutional@1` | 2169 | 0.200 | 0.180 | 0.360 | 0.360 | 0.431 | 0.439 | 0.422 | 0.017 |
| synthetic-s3 | `extreme/extreme.rsi@1` | 366 | 0.440 | 0.600 | 0.570 | 0.545 | 0.546 | 0.554 | 0.540 | 0.014 |
| synthetic-s3 | `breakout/breakout.range@1` | 1381 | 0.680 | 0.480 | 0.480 | 0.525 | 0.469 | 0.470 | 0.469 | 0.001 |
| synthetic-s3 | `flow/flow.volume_surge@1` | 61 | 0.480 | 0.480 | — | — | 0.492 | 0.500 | 0.485 | 0.015 |
| synthetic-s3 | `reversal/reversal.rsi_divergence@1` | 311 | 0.200 | 0.260 | 0.410 | 0.455 | 0.441 | 0.441 | 0.440 | 0.001 |
| synthetic-s3 | `continuation/continuation.ma_stack@1` | 664 | 0.520 | 0.520 | 0.450 | 0.440 | 0.488 | 0.448 | 0.522 | -0.074 |
| synthetic-s3 | `squeeze/squeeze.bandwidth@1` | 75 | 0.920 | 0.940 | — | — | 0.947 | 0.923 | 0.959 | -0.036 |

## 2. Duyarlılık

| Veri | Çeşit | Genel oran değişimi | Ortak kapsül | Hüküm uyuşması | Kapsama | Anahtar başına en büyük \|değişim\| |
|---|---|---|---|---|---|---|
| synthetic-s1 | h-20% | -0.018 | 5157 | 0.964 | 0.995 | 0.055 |
| synthetic-s1 | h+20% | 0.012 | 5117 | 0.978 | 0.987 | 0.018 |
| synthetic-s1 | k-10% | 0.013 | 5135 | 0.961 | 0.991 | 0.054 |
| synthetic-s1 | k+10% | -0.006 | 4969 | 0.955 | 0.959 | 0.023 |
| synthetic-s2 | h-20% | -0.018 | 5100 | 0.964 | 0.994 | 0.025 |
| synthetic-s2 | h+20% | 0.015 | 5065 | 0.976 | 0.987 | 0.021 |
| synthetic-s2 | k-10% | 0.010 | 5069 | 0.955 | 0.988 | 0.014 |
| synthetic-s2 | k+10% | -0.010 | 4906 | 0.957 | 0.956 | 0.027 |
| synthetic-s3 | h-20% | -0.024 | 4997 | 0.957 | 0.994 | 0.028 |
| synthetic-s3 | h+20% | 0.013 | 4953 | 0.978 | 0.985 | 0.040 |
| synthetic-s3 | k-10% | 0.005 | 4957 | 0.958 | 0.986 | 0.015 |
| synthetic-s3 | k+10% | -0.011 | 4816 | 0.959 | 0.958 | 0.078 |

## 3. İleriye bakma

| Veri | Kesim t | Karşılaştırılan (t ≤ T) | Farklı olay | Farklı hüküm | Karnesi farklı anahtar | Gelecekte değişen | Koruma ihlali |
|---|---|---|---|---|---|---|---|
| synthetic-s1 | 1687762800000 | 2803 | 0 | 0 | 0 | 11696 | 0 |
| synthetic-s1 | 1702882800000 | 6071 | 0 | 0 | 0 | 7686 | 0 |
| synthetic-s1 | 1718002800000 | 9352 | 0 | 0 | 0 | 3862 | 0 |
| synthetic-s2 | 1687762800000 | 2871 | 0 | 0 | 0 | 11836 | 0 |
| synthetic-s2 | 1702882800000 | 6184 | 0 | 0 | 0 | 7785 | 0 |
| synthetic-s2 | 1718002800000 | 9425 | 0 | 0 | 0 | 3831 | 0 |
| synthetic-s3 | 1687762800000 | 2795 | 0 | 0 | 0 | 11674 | 0 |
| synthetic-s3 | 1702882800000 | 5998 | 0 | 0 | 0 | 7789 | 0 |
| synthetic-s3 | 1718002800000 | 9246 | 0 | 0 | 0 | 3825 | 0 |
