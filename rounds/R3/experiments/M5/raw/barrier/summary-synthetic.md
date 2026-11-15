# M5 — `barrier` (synthetic)

Kurallar: tema `reference.kind_direction`, sonuç `barrier`, sıralayıcı `reference.families`.

## Özet

| Veri | Kapanan | Gerçekleşen | Geçersiz | Süresi dolan | İsabet | Süresi dolan payı | maks \|p(25)−p(son)\| | maks \|p(50)−p(son)\| | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| synthetic-s1 | 5181 | 2430 | 2336 | 415 | 0.469 | 0.080 | 0.296 | 0.276 | 0.146 | 0.096 | 0.469 | 0.450 | 0.020 | geçti |
| synthetic-s2 | 5130 | 2366 | 2329 | 435 | 0.461 | 0.085 | 0.250 | 0.230 | 0.140 | 0.035 | 0.462 | 0.452 | 0.011 | geçti |
| synthetic-s3 | 5034 | 2285 | 2317 | 432 | 0.454 | 0.086 | 0.280 | 0.249 | 0.059 | 0.064 | 0.455 | 0.457 | -0.002 | geçti |

## 1. Kararlılık (anahtar başına)

| Veri | Anahtar | n | p(25) | p(50) | p(100) | p(200) | p(son) | ilk yarı | ikinci yarı | fark |
|---|---|---|---|---|---|---|---|---|---|---|
| synthetic-s1 | `flow/flow.institutional@1` | 2236 | 0.160 | 0.180 | 0.310 | 0.360 | 0.456 | 0.434 | 0.477 | -0.042 |
| synthetic-s1 | `extreme/extreme.rsi@1` | 388 | 0.480 | 0.520 | 0.570 | 0.520 | 0.531 | 0.503 | 0.554 | -0.051 |
| synthetic-s1 | `breakout/breakout.range@1` | 1502 | 0.520 | 0.460 | 0.450 | 0.465 | 0.489 | 0.490 | 0.489 | 0.001 |
| synthetic-s1 | `flow/flow.volume_surge@1` | 66 | 0.480 | 0.500 | — | — | 0.455 | 0.500 | 0.421 | 0.079 |
| synthetic-s1 | `reversal/reversal.rsi_divergence@1` | 285 | 0.440 | 0.420 | 0.420 | 0.425 | 0.425 | 0.411 | 0.436 | -0.025 |
| synthetic-s1 | `continuation/continuation.ma_stack@1` | 638 | 0.640 | 0.520 | 0.540 | 0.515 | 0.451 | 0.471 | 0.436 | 0.036 |
| synthetic-s1 | `squeeze/squeeze.bandwidth@1` | 66 | 0.360 | 0.420 | — | — | 0.455 | 0.188 | 0.540 | -0.353 |
| synthetic-s2 | `flow/flow.institutional@1` | 2300 | 0.200 | 0.220 | 0.310 | 0.430 | 0.450 | 0.445 | 0.456 | -0.011 |
| synthetic-s2 | `extreme/extreme.rsi@1` | 390 | 0.600 | 0.580 | 0.510 | 0.510 | 0.477 | 0.513 | 0.440 | 0.072 |
| synthetic-s2 | `breakout/breakout.range@1` | 1420 | 0.640 | 0.580 | 0.550 | 0.510 | 0.475 | 0.486 | 0.465 | 0.021 |
| synthetic-s2 | `reversal/reversal.rsi_divergence@1` | 286 | 0.560 | 0.460 | 0.430 | 0.460 | 0.441 | 0.429 | 0.452 | -0.023 |
| synthetic-s2 | `flow/flow.volume_surge@1` | 48 | 0.520 | — | — | — | 0.438 | 0.500 | 0.364 | 0.136 |
| synthetic-s2 | `continuation/continuation.ma_stack@1` | 613 | 0.440 | 0.520 | 0.510 | 0.490 | 0.480 | 0.518 | 0.448 | 0.070 |
| synthetic-s2 | `squeeze/squeeze.bandwidth@1` | 73 | 0.480 | 0.380 | — | — | 0.384 | 0.478 | 0.340 | 0.138 |
| synthetic-s3 | `flow/flow.institutional@1` | 2203 | 0.200 | 0.180 | 0.370 | 0.365 | 0.429 | 0.439 | 0.419 | 0.020 |
| synthetic-s3 | `extreme/extreme.rsi@1` | 374 | 0.400 | 0.560 | 0.520 | 0.525 | 0.521 | 0.533 | 0.512 | 0.020 |
| synthetic-s3 | `breakout/breakout.range@1` | 1412 | 0.680 | 0.500 | 0.480 | 0.525 | 0.472 | 0.479 | 0.467 | 0.012 |
| synthetic-s3 | `flow/flow.volume_surge@1` | 60 | 0.480 | 0.480 | — | — | 0.483 | 0.500 | 0.469 | 0.031 |
| synthetic-s3 | `reversal/reversal.rsi_divergence@1` | 307 | 0.160 | 0.260 | 0.440 | 0.460 | 0.440 | 0.453 | 0.427 | 0.027 |
| synthetic-s3 | `continuation/continuation.ma_stack@1` | 606 | 0.520 | 0.460 | 0.440 | 0.425 | 0.477 | 0.441 | 0.509 | -0.068 |
| synthetic-s3 | `squeeze/squeeze.bandwidth@1` | 72 | 0.280 | 0.380 | — | — | 0.347 | 0.308 | 0.370 | -0.062 |

## 2. Duyarlılık

| Veri | Çeşit | Genel oran değişimi | Ortak kapsül | Hüküm uyuşması | Kapsama | Anahtar başına en büyük \|değişim\| |
|---|---|---|---|---|---|---|
| synthetic-s1 | h-20% | -0.018 | 5155 | 0.963 | 0.995 | 0.092 |
| synthetic-s1 | h+20% | 0.012 | 5111 | 0.977 | 0.986 | 0.017 |
| synthetic-s1 | k-10% | 0.013 | 5137 | 0.956 | 0.992 | 0.053 |
| synthetic-s1 | k+10% | -0.009 | 4981 | 0.958 | 0.961 | 0.061 |
| synthetic-s2 | h-20% | -0.019 | 5100 | 0.965 | 0.994 | 0.082 |
| synthetic-s2 | h+20% | 0.018 | 5063 | 0.972 | 0.987 | 0.130 |
| synthetic-s2 | k-10% | 0.017 | 5066 | 0.954 | 0.988 | 0.049 |
| synthetic-s2 | k+10% | -0.009 | 4904 | 0.957 | 0.956 | 0.027 |
| synthetic-s3 | h-20% | -0.023 | 5002 | 0.958 | 0.994 | 0.056 |
| synthetic-s3 | h+20% | 0.014 | 4955 | 0.980 | 0.984 | 0.059 |
| synthetic-s3 | k-10% | 0.010 | 4962 | 0.958 | 0.986 | 0.023 |
| synthetic-s3 | k+10% | -0.012 | 4832 | 0.958 | 0.960 | 0.032 |

## 3. İleriye bakma

| Veri | Kesim t | Karşılaştırılan (t ≤ T) | Farklı olay | Farklı hüküm | Karnesi farklı anahtar | Gelecekte değişen | Koruma ihlali |
|---|---|---|---|---|---|---|---|
| synthetic-s1 | 1687762800000 | 2822 | 0 | 0 | 0 | 11860 | 0 |
| synthetic-s1 | 1702882800000 | 6117 | 0 | 0 | 0 | 7770 | 0 |
| synthetic-s1 | 1718002800000 | 9430 | 0 | 0 | 0 | 3906 | 0 |
| synthetic-s2 | 1687762800000 | 2890 | 0 | 0 | 0 | 11988 | 0 |
| synthetic-s2 | 1702882800000 | 6247 | 0 | 0 | 0 | 7866 | 0 |
| synthetic-s2 | 1718002800000 | 9508 | 0 | 0 | 0 | 3874 | 0 |
| synthetic-s3 | 1687762800000 | 2817 | 0 | 0 | 0 | 11809 | 0 |
| synthetic-s3 | 1702882800000 | 6070 | 0 | 0 | 0 | 7880 | 0 |
| synthetic-s3 | 1718002800000 | 9333 | 0 | 0 | 0 | 3868 | 0 |
