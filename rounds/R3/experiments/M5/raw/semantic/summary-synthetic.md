# M5 — `semantic` (synthetic)

Kurallar: tema `reference.kind_direction`, sonuç `semantic`, sıralayıcı `reference.families`.

## Özet

| Veri | Kapanan | Gerçekleşen | Geçersiz | Süresi dolan | İsabet | Süresi dolan payı | maks \|p(25)−p(son)\| | maks \|p(50)−p(son)\| | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| synthetic-s1 | 5045 | 2423 | 2103 | 519 | 0.480 | 0.103 | 0.164 | 0.166 | 0.090 | 0.045 | 0.474 | 0.302 | 0.172 | geçti |
| synthetic-s2 | 4990 | 2350 | 2088 | 552 | 0.471 | 0.111 | 0.233 | 0.153 | 0.053 | 0.078 | 0.467 | 0.302 | 0.165 | geçti |
| synthetic-s3 | 4915 | 2288 | 2063 | 564 | 0.466 | 0.115 | 0.182 | 0.133 | 0.103 | 0.061 | 0.460 | 0.304 | 0.156 | geçti |

## 1. Kararlılık (anahtar başına)

| Veri | Anahtar | n | p(25) | p(50) | p(100) | p(200) | p(son) | ilk yarı | ikinci yarı | fark |
|---|---|---|---|---|---|---|---|---|---|---|
| synthetic-s1 | `flow/flow.institutional@1` | 2191 | 0.600 | 0.660 | 0.550 | 0.500 | 0.494 | 0.478 | 0.510 | -0.032 |
| synthetic-s1 | `extreme/extreme.rsi@1` | 352 | 0.360 | 0.420 | 0.440 | 0.425 | 0.389 | 0.423 | 0.362 | 0.061 |
| synthetic-s1 | `flow/flow.volume_surge@1` | 62 | 0.440 | 0.520 | — | — | 0.484 | 0.462 | 0.500 | -0.038 |
| synthetic-s1 | `reversal/reversal.rsi_divergence@1` | 278 | 0.440 | 0.380 | 0.390 | 0.450 | 0.446 | 0.422 | 0.467 | -0.045 |
| synthetic-s1 | `breakout/breakout.range@1` | 1501 | 0.520 | 0.440 | 0.380 | 0.425 | 0.470 | 0.474 | 0.467 | 0.008 |
| synthetic-s1 | `continuation/continuation.ma_stack@1` | 594 | 0.640 | 0.540 | 0.540 | 0.500 | 0.476 | 0.475 | 0.477 | -0.002 |
| synthetic-s1 | `squeeze/squeeze.bandwidth@1` | 67 | 0.840 | 0.880 | — | — | 0.896 | 0.812 | 0.922 | -0.109 |
| synthetic-s2 | `flow/flow.institutional@1` | 2236 | 0.720 | 0.640 | 0.540 | 0.565 | 0.487 | 0.497 | 0.478 | 0.020 |
| synthetic-s2 | `extreme/extreme.rsi@1` | 351 | 0.400 | 0.380 | 0.370 | 0.395 | 0.422 | 0.382 | 0.462 | -0.080 |
| synthetic-s2 | `breakout/breakout.range@1` | 1432 | 0.560 | 0.540 | 0.460 | 0.480 | 0.450 | 0.466 | 0.435 | 0.032 |
| synthetic-s2 | `reversal/reversal.rsi_divergence@1` | 276 | 0.440 | 0.480 | 0.440 | 0.445 | 0.417 | 0.419 | 0.414 | 0.005 |
| synthetic-s2 | `continuation/continuation.ma_stack@1` | 581 | 0.520 | 0.540 | 0.490 | 0.525 | 0.478 | 0.525 | 0.440 | 0.084 |
| synthetic-s2 | `flow/flow.volume_surge@1` | 41 | 0.520 | — | — | — | 0.463 | 0.571 | 0.350 | 0.221 |
| synthetic-s2 | `squeeze/squeeze.bandwidth@1` | 73 | 0.840 | 0.860 | — | — | 0.767 | 0.826 | 0.740 | 0.086 |
| synthetic-s3 | `flow/flow.institutional@1` | 2153 | 0.480 | 0.600 | 0.570 | 0.490 | 0.467 | 0.488 | 0.447 | 0.041 |
| synthetic-s3 | `extreme/extreme.rsi@1` | 334 | 0.440 | 0.260 | 0.330 | 0.385 | 0.392 | 0.371 | 0.410 | -0.039 |
| synthetic-s3 | `breakout/breakout.range@1` | 1422 | 0.640 | 0.480 | 0.470 | 0.475 | 0.458 | 0.453 | 0.462 | -0.008 |
| synthetic-s3 | `flow/flow.volume_surge@1` | 61 | 0.440 | 0.480 | — | — | 0.492 | 0.531 | 0.448 | 0.083 |
| synthetic-s3 | `reversal/reversal.rsi_divergence@1` | 297 | 0.280 | 0.380 | 0.470 | 0.475 | 0.458 | 0.479 | 0.438 | 0.041 |
| synthetic-s3 | `continuation/continuation.ma_stack@1` | 576 | 0.600 | 0.520 | 0.440 | 0.415 | 0.476 | 0.429 | 0.518 | -0.090 |
| synthetic-s3 | `squeeze/squeeze.bandwidth@1` | 72 | 0.800 | 0.840 | — | — | 0.833 | 0.833 | 0.833 | 0.000 |

## 2. Duyarlılık

| Veri | Çeşit | Genel oran değişimi | Ortak kapsül | Hüküm uyuşması | Kapsama | Anahtar başına en büyük \|değişim\| |
|---|---|---|---|---|---|---|
| synthetic-s1 | h-20% | -0.024 | 5000 | 0.955 | 0.991 | 0.090 |
| synthetic-s1 | h+20% | 0.013 | 4956 | 0.975 | 0.982 | 0.045 |
| synthetic-s1 | k-10% | 0.019 | 4979 | 0.970 | 0.987 | 0.039 |
| synthetic-s1 | k+10% | -0.015 | 4875 | 0.971 | 0.966 | 0.104 |
| synthetic-s2 | h-20% | -0.026 | 4966 | 0.953 | 0.995 | 0.058 |
| synthetic-s2 | h+20% | 0.017 | 4921 | 0.970 | 0.986 | 0.024 |
| synthetic-s2 | k-10% | 0.019 | 4942 | 0.970 | 0.990 | 0.057 |
| synthetic-s2 | k+10% | -0.015 | 4823 | 0.974 | 0.967 | 0.059 |
| synthetic-s3 | h-20% | -0.024 | 4883 | 0.949 | 0.993 | 0.053 |
| synthetic-s3 | h+20% | 0.020 | 4819 | 0.967 | 0.980 | 0.068 |
| synthetic-s3 | k-10% | 0.016 | 4860 | 0.972 | 0.989 | 0.057 |
| synthetic-s3 | k+10% | -0.018 | 4748 | 0.972 | 0.966 | 0.059 |

## 3. İleriye bakma

| Veri | Kesim t | Karşılaştırılan (t ≤ T) | Farklı olay | Farklı hüküm | Karnesi farklı anahtar | Gelecekte değişen | Koruma ihlali |
|---|---|---|---|---|---|---|---|
| synthetic-s1 | 1687762800000 | 2850 | 0 | 0 | 0 | 11843 | 0 |
| synthetic-s1 | 1702882800000 | 6111 | 0 | 0 | 0 | 7796 | 0 |
| synthetic-s1 | 1718002800000 | 9456 | 0 | 0 | 0 | 3908 | 0 |
| synthetic-s2 | 1687762800000 | 2887 | 0 | 0 | 0 | 12008 | 0 |
| synthetic-s2 | 1702882800000 | 6240 | 0 | 0 | 0 | 7854 | 0 |
| synthetic-s2 | 1718002800000 | 9508 | 0 | 0 | 0 | 3865 | 0 |
| synthetic-s3 | 1687762800000 | 2826 | 0 | 0 | 0 | 11838 | 0 |
| synthetic-s3 | 1702882800000 | 6067 | 0 | 0 | 0 | 7884 | 0 |
| synthetic-s3 | 1718002800000 | 9351 | 0 | 0 | 0 | 3851 | 0 |
