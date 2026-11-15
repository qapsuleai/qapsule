# M5 — `barrier` (real)

Kurallar: tema `kinddir`, sonuç `barrier`, sıralayıcı `reference.families`.

## Özet

| Veri | Kapanan | Gerçekleşen | Geçersiz | Süresi dolan | İsabet | Süresi dolan payı | maks \|p(25)−p(son)\| | maks \|p(50)−p(son)\| | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| real-1day | 2671 | 1399 | 1138 | 134 | 0.524 | 0.050 | 0.140 | 0.132 | 0.062 | 0.045 | 0.518 | 0.430 | 0.088 | geçti |
| real-1hour | 4596 | 2209 | 1990 | 397 | 0.481 | 0.086 | 0.158 | 0.138 | 0.148 | 0.071 | 0.476 | 0.434 | 0.043 | geçti |

## 1. Kararlılık (anahtar başına)

| Veri | Anahtar | n | p(25) | p(50) | p(100) | p(200) | p(son) | ilk yarı | ikinci yarı | fark |
|---|---|---|---|---|---|---|---|---|---|---|
| real-1day | `extreme/extreme.rsi@1` | 368 | 0.360 | 0.400 | 0.520 | 0.545 | 0.500 | 0.530 | 0.471 | 0.060 |
| real-1day | `breakout/breakout.range@1` | 1319 | 0.400 | 0.480 | 0.480 | 0.485 | 0.519 | 0.548 | 0.492 | 0.056 |
| real-1day | `reversal/reversal.rsi_divergence@1` | 237 | 0.640 | 0.600 | 0.520 | 0.480 | 0.506 | 0.500 | 0.512 | -0.012 |
| real-1day | `flow/flow.volume_surge@1` | 92 | 0.400 | 0.420 | — | — | 0.511 | 0.436 | 0.622 | -0.185 |
| real-1day | `continuation/continuation.ma_stack@1` | 558 | 0.440 | 0.400 | 0.470 | 0.520 | 0.532 | 0.527 | 0.537 | -0.011 |
| real-1day | `squeeze/squeeze.bandwidth@1` | 97 | 0.640 | 0.700 | — | — | 0.680 | 0.659 | 0.698 | -0.039 |
| real-1hour | `extreme/extreme.rsi@1` | 745 | 0.320 | 0.340 | 0.330 | 0.425 | 0.478 | 0.469 | 0.488 | -0.019 |
| real-1hour | `breakout/breakout.range@1` | 1897 | 0.520 | 0.440 | 0.470 | 0.460 | 0.477 | 0.499 | 0.455 | 0.044 |
| real-1hour | `reversal/reversal.rsi_divergence@1` | 391 | 0.480 | 0.580 | 0.500 | 0.480 | 0.453 | 0.485 | 0.420 | 0.065 |
| real-1hour | `flow/flow.volume_surge@1` | 387 | 0.480 | 0.460 | 0.430 | 0.420 | 0.465 | 0.426 | 0.511 | -0.085 |
| real-1hour | `continuation/continuation.ma_stack@1` | 1009 | 0.360 | 0.440 | 0.450 | 0.420 | 0.491 | 0.504 | 0.478 | 0.026 |
| real-1hour | `squeeze/squeeze.bandwidth@1` | 167 | 0.600 | 0.600 | 0.570 | — | 0.575 | 0.585 | 0.565 | 0.021 |

## 2. Duyarlılık

| Veri | Çeşit | Genel oran değişimi | Ortak kapsül | Hüküm uyuşması | Kapsama | Anahtar başına en büyük \|değişim\| |
|---|---|---|---|---|---|---|
| real-1day | h-20% | -0.015 | 2670 | 0.968 | 1.000 | 0.062 |
| real-1day | h+20% | 0.015 | 2665 | 0.977 | 0.998 | 0.072 |
| real-1day | k-10% | 0.012 | 2629 | 0.955 | 0.984 | 0.072 |
| real-1day | k+10% | -0.010 | 2552 | 0.955 | 0.955 | 0.113 |
| real-1hour | h-20% | -0.031 | 4581 | 0.947 | 0.997 | 0.107 |
| real-1hour | h+20% | 0.015 | 4578 | 0.971 | 0.996 | 0.084 |
| real-1hour | k-10% | 0.009 | 4555 | 0.954 | 0.991 | 0.054 |
| real-1hour | k+10% | -0.017 | 4441 | 0.951 | 0.966 | 0.084 |

## 3. İleriye bakma

| Veri | Kesim t | Karşılaştırılan (t ≤ T) | Farklı olay | Farklı hüküm | Karnesi farklı anahtar | Gelecekte değişen | Koruma ihlali |
|---|---|---|---|---|---|---|---|
| real-1day | 1584514800000 | 1592 | 0 | 0 | 0 | 8179 | 0 |
| real-1day | 1654671600000 | 3436 | 0 | 0 | 0 | 4691 | 0 |
| real-1day | 1724914800000 | 5325 | 0 | 0 | 0 | 2262 | 0 |
| real-1hour | 1747303200000 | 2984 | 0 | 0 | 0 | 13675 | 0 |
| real-1hour | 1763107200000 | 6100 | 0 | 0 | 0 | 9071 | 0 |
| real-1hour | 1778749200000 | 9227 | 0 | 0 | 0 | 4461 | 0 |
