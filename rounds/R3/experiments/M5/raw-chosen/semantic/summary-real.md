# M5 — `semantic` (real)

Kurallar: tema `kinddir`, sonuç `semantic`, sıralayıcı `reference.families`.

## Özet

| Veri | Kapanan | Gerçekleşen | Geçersiz | Süresi dolan | İsabet | Süresi dolan payı | maks \|p(25)−p(son)\| | maks \|p(50)−p(son)\| | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| real-1day | 2582 | 1279 | 1095 | 208 | 0.495 | 0.081 | 0.166 | 0.166 | 0.107 | 0.062 | 0.477 | 0.257 | 0.220 | geçti |
| real-1hour | 4438 | 2125 | 1788 | 525 | 0.479 | 0.118 | 0.216 | 0.116 | 0.067 | 0.071 | 0.459 | 0.274 | 0.185 | geçti |

## 1. Kararlılık (anahtar başına)

| Veri | Anahtar | n | p(25) | p(50) | p(100) | p(200) | p(son) | ilk yarı | ikinci yarı | fark |
|---|---|---|---|---|---|---|---|---|---|---|
| real-1day | `extreme/extreme.rsi@1` | 341 | 0.520 | 0.520 | 0.410 | 0.370 | 0.416 | 0.374 | 0.459 | -0.085 |
| real-1day | `breakout/breakout.range@1` | 1299 | 0.360 | 0.380 | 0.380 | 0.425 | 0.487 | 0.523 | 0.452 | 0.071 |
| real-1day | `reversal/reversal.rsi_divergence@1` | 233 | 0.560 | 0.520 | 0.410 | 0.410 | 0.433 | 0.414 | 0.451 | -0.036 |
| real-1day | `flow/flow.volume_surge@1` | 85 | 0.320 | 0.320 | — | — | 0.412 | 0.346 | 0.515 | -0.169 |
| real-1day | `continuation/continuation.ma_stack@1` | 527 | 0.360 | 0.360 | 0.450 | 0.515 | 0.526 | 0.525 | 0.526 | -0.000 |
| real-1day | `squeeze/squeeze.bandwidth@1` | 97 | 0.960 | 0.960 | — | — | 0.948 | 0.955 | 0.943 | 0.011 |
| real-1hour | `breakout/breakout.range@1` | 1897 | 0.440 | 0.380 | 0.390 | 0.415 | 0.453 | 0.465 | 0.440 | 0.025 |
| real-1hour | `extreme/extreme.rsi@1` | 669 | 0.520 | 0.540 | 0.520 | 0.440 | 0.453 | 0.464 | 0.441 | 0.022 |
| real-1hour | `reversal/reversal.rsi_divergence@1` | 376 | 0.440 | 0.460 | 0.460 | 0.415 | 0.420 | 0.415 | 0.426 | -0.012 |
| real-1hour | `flow/flow.volume_surge@1` | 366 | 0.480 | 0.480 | 0.440 | 0.410 | 0.459 | 0.409 | 0.518 | -0.109 |
| real-1hour | `continuation/continuation.ma_stack@1` | 960 | 0.280 | 0.380 | 0.430 | 0.425 | 0.496 | 0.499 | 0.493 | 0.006 |
| real-1hour | `squeeze/squeeze.bandwidth@1` | 170 | 0.880 | 0.920 | 0.930 | — | 0.947 | 0.928 | 0.966 | -0.038 |

## 2. Duyarlılık

| Veri | Çeşit | Genel oran değişimi | Ortak kapsül | Hüküm uyuşması | Kapsama | Anahtar başına en büyük \|değişim\| |
|---|---|---|---|---|---|---|
| real-1day | h-20% | -0.020 | 2577 | 0.958 | 0.998 | 0.036 |
| real-1day | h+20% | 0.018 | 2565 | 0.970 | 0.993 | 0.035 |
| real-1day | k-10% | 0.016 | 2556 | 0.967 | 0.990 | 0.082 |
| real-1day | k+10% | -0.004 | 2494 | 0.976 | 0.966 | 0.028 |
| real-1hour | h-20% | -0.036 | 4414 | 0.934 | 0.995 | 0.065 |
| real-1hour | h+20% | 0.018 | 4414 | 0.965 | 0.995 | 0.042 |
| real-1hour | k-10% | 0.015 | 4403 | 0.970 | 0.992 | 0.030 |
| real-1hour | k+10% | -0.012 | 4304 | 0.970 | 0.970 | 0.041 |

## 3. İleriye bakma

| Veri | Kesim t | Karşılaştırılan (t ≤ T) | Farklı olay | Farklı hüküm | Karnesi farklı anahtar | Gelecekte değişen | Koruma ihlali |
|---|---|---|---|---|---|---|---|
| real-1day | 1584514800000 | 1592 | 0 | 0 | 0 | 8061 | 0 |
| real-1day | 1654671600000 | 3440 | 0 | 0 | 0 | 4684 | 0 |
| real-1day | 1724914800000 | 5329 | 0 | 0 | 0 | 2236 | 0 |
| real-1hour | 1747303200000 | 3011 | 0 | 0 | 0 | 13537 | 0 |
| real-1hour | 1763107200000 | 6151 | 0 | 0 | 0 | 8969 | 0 |
| real-1hour | 1778749200000 | 9270 | 0 | 0 | 0 | 4422 | 0 |
