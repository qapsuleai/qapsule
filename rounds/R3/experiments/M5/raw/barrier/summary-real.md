# M5 — `barrier` (real)

Kurallar: tema `reference.kind_direction`, sonuç `barrier`, sıralayıcı `reference.families`.

## Özet

| Veri | Kapanan | Gerçekleşen | Geçersiz | Süresi dolan | İsabet | Süresi dolan payı | maks \|p(25)−p(son)\| | maks \|p(50)−p(son)\| | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| real-1day | 2802 | 1474 | 1183 | 145 | 0.526 | 0.052 | 0.140 | 0.157 | 0.057 | 0.045 | 0.520 | 0.426 | 0.094 | geçti |
| real-1hour | 5126 | 2451 | 2220 | 455 | 0.478 | 0.089 | 0.158 | 0.138 | 0.148 | 0.072 | 0.474 | 0.434 | 0.041 | geçti |

## 1. Kararlılık (anahtar başına)

| Veri | Anahtar | n | p(25) | p(50) | p(100) | p(200) | p(son) | ilk yarı | ikinci yarı | fark |
|---|---|---|---|---|---|---|---|---|---|---|
| real-1day | `extreme/extreme.rsi@1` | 368 | 0.360 | 0.400 | 0.520 | 0.545 | 0.500 | 0.530 | 0.471 | 0.060 |
| real-1day | `breakout/breakout.range@1` | 1319 | 0.400 | 0.480 | 0.480 | 0.485 | 0.519 | 0.548 | 0.492 | 0.056 |
| real-1day | `reversal/reversal.rsi_divergence@1` | 236 | 0.640 | 0.600 | 0.520 | 0.485 | 0.508 | 0.504 | 0.512 | -0.008 |
| real-1day | `flow/flow.volume_surge@1` | 231 | 0.400 | 0.500 | 0.520 | 0.515 | 0.532 | 0.551 | 0.510 | 0.042 |
| real-1day | `continuation/continuation.ma_stack@1` | 551 | 0.440 | 0.380 | 0.480 | 0.530 | 0.537 | 0.537 | 0.537 | -0.000 |
| real-1day | `squeeze/squeeze.bandwidth@1` | 97 | 0.640 | 0.700 | — | — | 0.680 | 0.659 | 0.698 | -0.039 |
| real-1hour | `extreme/extreme.rsi@1` | 745 | 0.320 | 0.340 | 0.330 | 0.425 | 0.478 | 0.469 | 0.488 | -0.019 |
| real-1hour | `breakout/breakout.range@1` | 1897 | 0.520 | 0.440 | 0.470 | 0.460 | 0.477 | 0.499 | 0.455 | 0.044 |
| real-1hour | `reversal/reversal.rsi_divergence@1` | 388 | 0.480 | 0.580 | 0.500 | 0.485 | 0.456 | 0.490 | 0.422 | 0.068 |
| real-1hour | `flow/flow.volume_surge@1` | 928 | 0.480 | 0.480 | 0.450 | 0.435 | 0.458 | 0.450 | 0.468 | -0.018 |
| real-1hour | `continuation/continuation.ma_stack@1` | 1002 | 0.360 | 0.440 | 0.450 | 0.420 | 0.492 | 0.504 | 0.480 | 0.024 |
| real-1hour | `squeeze/squeeze.bandwidth@1` | 166 | 0.600 | 0.600 | 0.570 | — | 0.572 | 0.585 | 0.560 | 0.026 |

## 2. Duyarlılık

| Veri | Çeşit | Genel oran değişimi | Ortak kapsül | Hüküm uyuşması | Kapsama | Anahtar başına en büyük \|değişim\| |
|---|---|---|---|---|---|---|
| real-1day | h-20% | -0.016 | 2800 | 0.968 | 0.999 | 0.062 |
| real-1day | h+20% | 0.016 | 2793 | 0.976 | 0.997 | 0.072 |
| real-1day | k-10% | 0.011 | 2760 | 0.955 | 0.985 | 0.072 |
| real-1day | k+10% | -0.010 | 2687 | 0.955 | 0.959 | 0.113 |
| real-1hour | h-20% | -0.032 | 5111 | 0.945 | 0.997 | 0.104 |
| real-1hour | h+20% | 0.015 | 5100 | 0.972 | 0.995 | 0.086 |
| real-1hour | k-10% | 0.009 | 5085 | 0.954 | 0.992 | 0.054 |
| real-1hour | k+10% | -0.017 | 4968 | 0.950 | 0.969 | 0.081 |

## 3. İleriye bakma

| Veri | Kesim t | Karşılaştırılan (t ≤ T) | Farklı olay | Farklı hüküm | Karnesi farklı anahtar | Gelecekte değişen | Koruma ihlali |
|---|---|---|---|---|---|---|---|
| real-1day | 1584514800000 | 1609 | 0 | 0 | 0 | 8502 | 0 |
| real-1day | 1654671600000 | 3499 | 0 | 0 | 0 | 4793 | 0 |
| real-1day | 1724914800000 | 5424 | 0 | 0 | 0 | 2296 | 0 |
| real-1hour | 1747303200000 | 3101 | 0 | 0 | 0 | 14190 | 0 |
| real-1hour | 1763107200000 | 6383 | 0 | 0 | 0 | 9382 | 0 |
| real-1hour | 1778749200000 | 9637 | 0 | 0 | 0 | 4579 | 0 |
