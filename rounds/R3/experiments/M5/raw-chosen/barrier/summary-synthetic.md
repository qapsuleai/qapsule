# M5 — `barrier` (synthetic)

Kurallar: tema `kinddir`, sonuç `barrier`, sıralayıcı `reference.families`.

## Özet

| Veri | Kapanan | Gerçekleşen | Geçersiz | Süresi dolan | İsabet | Süresi dolan payı | maks \|p(25)−p(son)\| | maks \|p(50)−p(son)\| | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| synthetic-s1 | 4240 | 1992 | 1898 | 350 | 0.470 | 0.083 | 0.291 | 0.271 | 0.151 | 0.081 | 0.470 | 0.446 | 0.024 | geçti |
| synthetic-s2 | 4169 | 1904 | 1903 | 362 | 0.457 | 0.087 | 0.225 | 0.205 | 0.095 | 0.040 | 0.458 | 0.454 | 0.004 | geçti |
| synthetic-s3 | 4090 | 1858 | 1867 | 365 | 0.454 | 0.089 | 0.282 | 0.230 | 0.070 | 0.070 | 0.456 | 0.453 | 0.003 | geçti |

## 1. Kararlılık (anahtar başına)

| Veri | Anahtar | n | p(25) | p(50) | p(100) | p(200) | p(son) | ilk yarı | ikinci yarı | fark |
|---|---|---|---|---|---|---|---|---|---|---|
| synthetic-s1 | `flow/flow.institutional@1` | 1309 | 0.160 | 0.180 | 0.300 | 0.370 | 0.451 | 0.417 | 0.484 | -0.067 |
| synthetic-s1 | `extreme/extreme.rsi@1` | 388 | 0.480 | 0.520 | 0.570 | 0.520 | 0.531 | 0.503 | 0.554 | -0.051 |
| synthetic-s1 | `breakout/breakout.range@1` | 1502 | 0.520 | 0.460 | 0.450 | 0.465 | 0.489 | 0.490 | 0.489 | 0.001 |
| synthetic-s1 | `flow/flow.volume_surge@1` | 45 | 0.400 | — | — | — | 0.356 | 0.444 | 0.296 | 0.148 |
| synthetic-s1 | `reversal/reversal.rsi_divergence@1` | 287 | 0.440 | 0.420 | 0.420 | 0.425 | 0.425 | 0.415 | 0.433 | -0.018 |
| synthetic-s1 | `continuation/continuation.ma_stack@1` | 643 | 0.640 | 0.520 | 0.540 | 0.515 | 0.454 | 0.472 | 0.440 | 0.031 |
| synthetic-s1 | `squeeze/squeeze.bandwidth@1` | 66 | 0.360 | 0.420 | — | — | 0.455 | 0.188 | 0.540 | -0.353 |
| synthetic-s2 | `flow/flow.institutional@1` | 1352 | 0.200 | 0.220 | 0.330 | 0.385 | 0.425 | 0.421 | 0.430 | -0.009 |
| synthetic-s2 | `extreme/extreme.rsi@1` | 390 | 0.600 | 0.580 | 0.510 | 0.510 | 0.477 | 0.513 | 0.440 | 0.072 |
| synthetic-s2 | `breakout/breakout.range@1` | 1420 | 0.640 | 0.580 | 0.550 | 0.510 | 0.475 | 0.486 | 0.465 | 0.021 |
| synthetic-s2 | `reversal/reversal.rsi_divergence@1` | 289 | 0.560 | 0.460 | 0.440 | 0.465 | 0.446 | 0.437 | 0.456 | -0.019 |
| synthetic-s2 | `flow/flow.volume_surge@1` | 28 | 0.480 | — | — | — | 0.464 | 0.500 | 0.429 | 0.071 |
| synthetic-s2 | `continuation/continuation.ma_stack@1` | 617 | 0.440 | 0.520 | 0.520 | 0.495 | 0.483 | 0.523 | 0.450 | 0.073 |
| synthetic-s2 | `squeeze/squeeze.bandwidth@1` | 73 | 0.480 | 0.380 | — | — | 0.384 | 0.478 | 0.340 | 0.138 |
| synthetic-s3 | `flow/flow.institutional@1` | 1277 | 0.200 | 0.180 | 0.340 | 0.340 | 0.410 | 0.410 | 0.411 | -0.001 |
| synthetic-s3 | `extreme/extreme.rsi@1` | 374 | 0.400 | 0.560 | 0.520 | 0.525 | 0.521 | 0.533 | 0.512 | 0.020 |
| synthetic-s3 | `breakout/breakout.range@1` | 1412 | 0.680 | 0.500 | 0.480 | 0.525 | 0.472 | 0.479 | 0.467 | 0.012 |
| synthetic-s3 | `flow/flow.volume_surge@1` | 39 | 0.480 | — | — | — | 0.538 | 0.500 | 0.565 | -0.065 |
| synthetic-s3 | `reversal/reversal.rsi_divergence@1` | 308 | 0.160 | 0.260 | 0.440 | 0.465 | 0.442 | 0.457 | 0.427 | 0.030 |
| synthetic-s3 | `continuation/continuation.ma_stack@1` | 608 | 0.520 | 0.460 | 0.430 | 0.425 | 0.477 | 0.438 | 0.513 | -0.075 |
| synthetic-s3 | `squeeze/squeeze.bandwidth@1` | 72 | 0.280 | 0.380 | — | — | 0.347 | 0.308 | 0.370 | -0.062 |

## 2. Duyarlılık

| Veri | Çeşit | Genel oran değişimi | Ortak kapsül | Hüküm uyuşması | Kapsama | Anahtar başına en büyük \|değişim\| |
|---|---|---|---|---|---|---|
| synthetic-s1 | h-20% | -0.020 | 4217 | 0.961 | 0.995 | 0.092 |
| synthetic-s1 | h+20% | 0.013 | 4199 | 0.976 | 0.990 | 0.022 |
| synthetic-s1 | k-10% | 0.013 | 4191 | 0.954 | 0.988 | 0.091 |
| synthetic-s1 | k+10% | -0.011 | 4071 | 0.958 | 0.960 | 0.061 |
| synthetic-s2 | h-20% | -0.020 | 4146 | 0.963 | 0.994 | 0.082 |
| synthetic-s2 | h+20% | 0.017 | 4134 | 0.971 | 0.992 | 0.123 |
| synthetic-s2 | k-10% | 0.017 | 4106 | 0.955 | 0.985 | 0.047 |
| synthetic-s2 | k+10% | -0.011 | 3975 | 0.954 | 0.953 | 0.027 |
| synthetic-s3 | h-20% | -0.020 | 4068 | 0.961 | 0.995 | 0.056 |
| synthetic-s3 | h+20% | 0.014 | 4050 | 0.978 | 0.990 | 0.061 |
| synthetic-s3 | k-10% | 0.008 | 4019 | 0.956 | 0.983 | 0.023 |
| synthetic-s3 | k+10% | -0.016 | 3898 | 0.958 | 0.953 | 0.033 |

## 3. İleriye bakma

| Veri | Kesim t | Karşılaştırılan (t ≤ T) | Farklı olay | Farklı hüküm | Karnesi farklı anahtar | Gelecekte değişen | Koruma ihlali |
|---|---|---|---|---|---|---|---|
| synthetic-s1 | 1687762800000 | 2622 | 0 | 0 | 0 | 10888 | 0 |
| synthetic-s1 | 1702882800000 | 5669 | 0 | 0 | 0 | 7144 | 0 |
| synthetic-s1 | 1718002800000 | 8743 | 0 | 0 | 0 | 3573 | 0 |
| synthetic-s2 | 1687762800000 | 2686 | 0 | 0 | 0 | 11009 | 0 |
| synthetic-s2 | 1702882800000 | 5790 | 0 | 0 | 0 | 7219 | 0 |
| synthetic-s2 | 1718002800000 | 8807 | 0 | 0 | 0 | 3556 | 0 |
| synthetic-s3 | 1687762800000 | 2613 | 0 | 0 | 0 | 10895 | 0 |
| synthetic-s3 | 1702882800000 | 5619 | 0 | 0 | 0 | 7257 | 0 |
| synthetic-s3 | 1718002800000 | 8638 | 0 | 0 | 0 | 3545 | 0 |
