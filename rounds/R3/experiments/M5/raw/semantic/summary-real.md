# M5 — `semantic` (real)

Kurallar: tema `reference.kind_direction`, sonuç `semantic`, sıralayıcı `reference.families`.

## Özet

| Veri | Kapanan | Gerçekleşen | Geçersiz | Süresi dolan | İsabet | Süresi dolan payı | maks \|p(25)−p(son)\| | maks \|p(50)−p(son)\| | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| real-1day | 2718 | 1360 | 1118 | 240 | 0.500 | 0.088 | 0.166 | 0.166 | 0.107 | 0.062 | 0.484 | 0.260 | 0.223 | geçti |
| real-1hour | 4966 | 2396 | 1903 | 667 | 0.482 | 0.134 | 0.216 | 0.116 | 0.067 | 0.066 | 0.465 | 0.285 | 0.180 | geçti |

## 1. Kararlılık (anahtar başına)

| Veri | Anahtar | n | p(25) | p(50) | p(100) | p(200) | p(son) | ilk yarı | ikinci yarı | fark |
|---|---|---|---|---|---|---|---|---|---|---|
| real-1day | `extreme/extreme.rsi@1` | 340 | 0.520 | 0.520 | 0.410 | 0.370 | 0.415 | 0.374 | 0.456 | -0.081 |
| real-1day | `breakout/breakout.range@1` | 1299 | 0.360 | 0.380 | 0.380 | 0.425 | 0.487 | 0.523 | 0.452 | 0.071 |
| real-1day | `reversal/reversal.rsi_divergence@1` | 229 | 0.600 | 0.520 | 0.430 | 0.415 | 0.437 | 0.426 | 0.446 | -0.020 |
| real-1day | `flow/flow.volume_surge@1` | 230 | 0.360 | 0.460 | 0.490 | 0.515 | 0.522 | 0.532 | 0.510 | 0.022 |
| real-1day | `continuation/continuation.ma_stack@1` | 523 | 0.360 | 0.360 | 0.450 | 0.515 | 0.526 | 0.528 | 0.524 | 0.004 |
| real-1day | `squeeze/squeeze.bandwidth@1` | 97 | 0.960 | 0.960 | — | — | 0.948 | 0.955 | 0.943 | 0.011 |
| real-1hour | `breakout/breakout.range@1` | 1897 | 0.440 | 0.380 | 0.390 | 0.415 | 0.453 | 0.465 | 0.440 | 0.025 |
| real-1hour | `extreme/extreme.rsi@1` | 669 | 0.520 | 0.540 | 0.520 | 0.440 | 0.453 | 0.464 | 0.441 | 0.022 |
| real-1hour | `reversal/reversal.rsi_divergence@1` | 365 | 0.440 | 0.460 | 0.460 | 0.420 | 0.422 | 0.415 | 0.429 | -0.014 |
| real-1hour | `flow/flow.volume_surge@1` | 918 | 0.480 | 0.480 | 0.440 | 0.435 | 0.489 | 0.460 | 0.524 | -0.064 |
| real-1hour | `continuation/continuation.ma_stack@1` | 947 | 0.280 | 0.380 | 0.430 | 0.430 | 0.496 | 0.499 | 0.494 | 0.005 |
| real-1hour | `squeeze/squeeze.bandwidth@1` | 170 | 0.880 | 0.920 | 0.930 | — | 0.947 | 0.928 | 0.966 | -0.038 |

## 2. Duyarlılık

| Veri | Çeşit | Genel oran değişimi | Ortak kapsül | Hüküm uyuşması | Kapsama | Anahtar başına en büyük \|değişim\| |
|---|---|---|---|---|---|---|
| real-1day | h-20% | -0.020 | 2709 | 0.959 | 0.997 | 0.035 |
| real-1day | h+20% | 0.019 | 2695 | 0.968 | 0.992 | 0.037 |
| real-1day | k-10% | 0.018 | 2692 | 0.965 | 0.990 | 0.058 |
| real-1day | k+10% | -0.005 | 2630 | 0.977 | 0.968 | 0.028 |
| real-1hour | h-20% | -0.039 | 4942 | 0.933 | 0.995 | 0.065 |
| real-1hour | h+20% | 0.018 | 4929 | 0.966 | 0.993 | 0.041 |
| real-1hour | k-10% | 0.017 | 4933 | 0.970 | 0.993 | 0.030 |
| real-1hour | k+10% | -0.013 | 4830 | 0.970 | 0.973 | 0.041 |

## 3. İleriye bakma

| Veri | Kesim t | Karşılaştırılan (t ≤ T) | Farklı olay | Farklı hüküm | Karnesi farklı anahtar | Gelecekte değişen | Koruma ihlali |
|---|---|---|---|---|---|---|---|
| real-1day | 1584514800000 | 1611 | 0 | 0 | 0 | 8355 | 0 |
| real-1day | 1654671600000 | 3505 | 0 | 0 | 0 | 4781 | 0 |
| real-1day | 1724914800000 | 5431 | 0 | 0 | 0 | 2272 | 0 |
| real-1hour | 1747303200000 | 3127 | 0 | 0 | 0 | 14013 | 0 |
| real-1hour | 1763107200000 | 6438 | 0 | 0 | 0 | 9237 | 0 |
| real-1hour | 1778749200000 | 9677 | 0 | 0 | 0 | 4534 | 0 |
