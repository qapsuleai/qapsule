# M5 — `semantic` (synthetic)

Kurallar: tema `kinddir`, sonuç `semantic`, sıralayıcı `reference.families`.

## Özet

| Veri | Kapanan | Gerçekleşen | Geçersiz | Süresi dolan | İsabet | Süresi dolan payı | maks \|p(25)−p(son)\| | maks \|p(50)−p(son)\| | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| synthetic-s1 | 4142 | 1935 | 1823 | 384 | 0.467 | 0.093 | 0.163 | 0.195 | 0.090 | 0.045 | 0.460 | 0.287 | 0.172 | geçti |
| synthetic-s2 | 4102 | 1868 | 1810 | 424 | 0.455 | 0.103 | 0.270 | 0.190 | 0.070 | 0.075 | 0.450 | 0.294 | 0.156 | geçti |
| synthetic-s3 | 4036 | 1843 | 1787 | 406 | 0.457 | 0.101 | 0.182 | 0.162 | 0.072 | 0.069 | 0.449 | 0.292 | 0.158 | geçti |

## 1. Kararlılık (anahtar başına)

| Veri | Anahtar | n | p(25) | p(50) | p(100) | p(200) | p(son) | ilk yarı | ikinci yarı | fark |
|---|---|---|---|---|---|---|---|---|---|---|
| synthetic-s1 | `flow/flow.institutional@1` | 1297 | 0.600 | 0.660 | 0.540 | 0.480 | 0.465 | 0.446 | 0.483 | -0.037 |
| synthetic-s1 | `extreme/extreme.rsi@1` | 353 | 0.360 | 0.420 | 0.440 | 0.425 | 0.388 | 0.423 | 0.360 | 0.063 |
| synthetic-s1 | `flow/flow.volume_surge@1` | 44 | 0.400 | — | — | — | 0.432 | 0.389 | 0.462 | -0.073 |
| synthetic-s1 | `reversal/reversal.rsi_divergence@1` | 285 | 0.440 | 0.380 | 0.380 | 0.440 | 0.442 | 0.415 | 0.465 | -0.049 |
| synthetic-s1 | `breakout/breakout.range@1` | 1501 | 0.520 | 0.440 | 0.380 | 0.425 | 0.470 | 0.474 | 0.467 | 0.008 |
| synthetic-s1 | `continuation/continuation.ma_stack@1` | 595 | 0.640 | 0.540 | 0.540 | 0.495 | 0.477 | 0.474 | 0.480 | -0.007 |
| synthetic-s1 | `squeeze/squeeze.bandwidth@1` | 67 | 0.840 | 0.880 | — | — | 0.896 | 0.812 | 0.922 | -0.109 |
| synthetic-s2 | `flow/flow.institutional@1` | 1348 | 0.720 | 0.640 | 0.520 | 0.525 | 0.450 | 0.460 | 0.439 | 0.021 |
| synthetic-s2 | `extreme/extreme.rsi@1` | 351 | 0.400 | 0.380 | 0.370 | 0.395 | 0.422 | 0.382 | 0.462 | -0.080 |
| synthetic-s2 | `breakout/breakout.range@1` | 1432 | 0.560 | 0.540 | 0.460 | 0.480 | 0.450 | 0.466 | 0.435 | 0.032 |
| synthetic-s2 | `reversal/reversal.rsi_divergence@1` | 282 | 0.440 | 0.480 | 0.440 | 0.445 | 0.422 | 0.424 | 0.420 | 0.005 |
| synthetic-s2 | `continuation/continuation.ma_stack@1` | 588 | 0.520 | 0.540 | 0.490 | 0.525 | 0.481 | 0.526 | 0.444 | 0.082 |
| synthetic-s2 | `flow/flow.volume_surge@1` | 27 | 0.440 | — | — | — | 0.444 | 0.538 | 0.357 | 0.181 |
| synthetic-s2 | `squeeze/squeeze.bandwidth@1` | 74 | 0.840 | 0.860 | — | — | 0.757 | 0.826 | 0.725 | 0.101 |
| synthetic-s3 | `flow/flow.institutional@1` | 1284 | 0.480 | 0.600 | 0.510 | 0.450 | 0.438 | 0.459 | 0.417 | 0.042 |
| synthetic-s3 | `extreme/extreme.rsi@1` | 335 | 0.400 | 0.260 | 0.330 | 0.380 | 0.391 | 0.368 | 0.410 | -0.041 |
| synthetic-s3 | `breakout/breakout.range@1` | 1422 | 0.640 | 0.480 | 0.470 | 0.475 | 0.458 | 0.453 | 0.462 | -0.008 |
| synthetic-s3 | `flow/flow.volume_surge@1` | 38 | 0.560 | — | — | — | 0.579 | 0.562 | 0.591 | -0.028 |
| synthetic-s3 | `reversal/reversal.rsi_divergence@1` | 304 | 0.360 | 0.400 | 0.470 | 0.480 | 0.464 | 0.490 | 0.439 | 0.051 |
| synthetic-s3 | `continuation/continuation.ma_stack@1` | 580 | 0.600 | 0.500 | 0.430 | 0.405 | 0.474 | 0.424 | 0.520 | -0.096 |
| synthetic-s3 | `squeeze/squeeze.bandwidth@1` | 73 | 0.840 | 0.860 | — | — | 0.836 | 0.840 | 0.833 | 0.007 |

## 2. Duyarlılık

| Veri | Çeşit | Genel oran değişimi | Ortak kapsül | Hüküm uyuşması | Kapsama | Anahtar başına en büyük \|değişim\| |
|---|---|---|---|---|---|---|
| synthetic-s1 | h-20% | -0.023 | 4114 | 0.956 | 0.993 | 0.090 |
| synthetic-s1 | h+20% | 0.013 | 4099 | 0.974 | 0.990 | 0.045 |
| synthetic-s1 | k-10% | 0.020 | 4080 | 0.969 | 0.985 | 0.068 |
| synthetic-s1 | k+10% | -0.015 | 3996 | 0.969 | 0.965 | 0.104 |
| synthetic-s2 | h-20% | -0.023 | 4084 | 0.958 | 0.996 | 0.058 |
| synthetic-s2 | h+20% | 0.017 | 4068 | 0.969 | 0.992 | 0.037 |
| synthetic-s2 | k-10% | 0.019 | 4052 | 0.971 | 0.988 | 0.068 |
| synthetic-s2 | k+10% | -0.016 | 3961 | 0.973 | 0.966 | 0.054 |
| synthetic-s3 | h-20% | -0.021 | 4013 | 0.953 | 0.994 | 0.055 |
| synthetic-s3 | h+20% | 0.016 | 3983 | 0.972 | 0.987 | 0.067 |
| synthetic-s3 | k-10% | 0.017 | 3983 | 0.971 | 0.987 | 0.055 |
| synthetic-s3 | k+10% | -0.018 | 3882 | 0.971 | 0.962 | 0.058 |

## 3. İleriye bakma

| Veri | Kesim t | Karşılaştırılan (t ≤ T) | Farklı olay | Farklı hüküm | Karnesi farklı anahtar | Gelecekte değişen | Koruma ihlali |
|---|---|---|---|---|---|---|---|
| synthetic-s1 | 1687762800000 | 2662 | 0 | 0 | 0 | 10912 | 0 |
| synthetic-s1 | 1702882800000 | 5692 | 0 | 0 | 0 | 7188 | 0 |
| synthetic-s1 | 1718002800000 | 8795 | 0 | 0 | 0 | 3589 | 0 |
| synthetic-s2 | 1687762800000 | 2684 | 0 | 0 | 0 | 11070 | 0 |
| synthetic-s2 | 1702882800000 | 5807 | 0 | 0 | 0 | 7235 | 0 |
| synthetic-s2 | 1718002800000 | 8857 | 0 | 0 | 0 | 3580 | 0 |
| synthetic-s3 | 1687762800000 | 2640 | 0 | 0 | 0 | 10931 | 0 |
| synthetic-s3 | 1702882800000 | 5650 | 0 | 0 | 0 | 7274 | 0 |
| synthetic-s3 | 1718002800000 | 8702 | 0 | 0 | 0 | 3544 | 0 |
