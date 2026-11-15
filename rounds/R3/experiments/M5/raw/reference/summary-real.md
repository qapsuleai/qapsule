# M5 — `reference` (real)

Kurallar: tema `reference.kind_direction`, sonuç `reference.atr_barrier`, sıralayıcı `reference.families`.

## Özet

| Veri | Kapanan | Gerçekleşen | Geçersiz | Süresi dolan | İsabet | Süresi dolan payı | maks \|p(25)−p(son)\| | maks \|p(50)−p(son)\| | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| real-1day | 2829 | 1545 | 1157 | 127 | 0.546 | 0.045 | 0.193 | 0.133 | 0.064 | 0.064 | 0.529 | 0.424 | 0.106 | geçti |
| real-1hour | 5170 | 2548 | 2200 | 422 | 0.493 | 0.082 | 0.065 | 0.139 | 0.139 | 0.057 | 0.475 | 0.442 | 0.033 | geçti |

## 1. Kararlılık (anahtar başına)

| Veri | Anahtar | n | p(25) | p(50) | p(100) | p(200) | p(son) | ilk yarı | ikinci yarı | fark |
|---|---|---|---|---|---|---|---|---|---|---|
| real-1day | `extreme/extreme.rsi@1` | 365 | 0.360 | 0.420 | 0.560 | 0.585 | 0.553 | 0.575 | 0.532 | 0.043 |
| real-1day | `breakout/breakout.range@1` | 1283 | 0.440 | 0.460 | 0.460 | 0.460 | 0.524 | 0.554 | 0.495 | 0.058 |
| real-1day | `reversal/reversal.rsi_divergence@1` | 243 | 0.600 | 0.640 | 0.530 | 0.495 | 0.514 | 0.509 | 0.520 | -0.011 |
| real-1day | `flow/flow.volume_surge@1` | 230 | 0.440 | 0.480 | 0.510 | 0.515 | 0.530 | 0.532 | 0.529 | 0.003 |
| real-1day | `continuation/continuation.ma_stack@1` | 611 | 0.440 | 0.420 | 0.490 | 0.525 | 0.535 | 0.538 | 0.533 | 0.005 |
| real-1day | `squeeze/squeeze.bandwidth@1` | 97 | 1.000 | 1.000 | — | — | 1.000 | 1.000 | 1.000 | 0.000 |
| real-1hour | `extreme/extreme.rsi@1` | 730 | 0.480 | 0.380 | 0.380 | 0.485 | 0.519 | 0.522 | 0.517 | 0.005 |
| real-1hour | `breakout/breakout.range@1` | 1861 | 0.520 | 0.380 | 0.440 | 0.435 | 0.471 | 0.493 | 0.449 | 0.044 |
| real-1hour | `reversal/reversal.rsi_divergence@1` | 398 | 0.480 | 0.520 | 0.460 | 0.430 | 0.415 | 0.429 | 0.400 | 0.029 |
| real-1hour | `flow/flow.volume_surge@1` | 925 | 0.440 | 0.440 | 0.410 | 0.400 | 0.445 | 0.424 | 0.470 | -0.046 |
| real-1hour | `continuation/continuation.ma_stack@1` | 1083 | 0.440 | 0.480 | 0.500 | 0.445 | 0.502 | 0.512 | 0.493 | 0.020 |
| real-1hour | `squeeze/squeeze.bandwidth@1` | 173 | 1.000 | 1.000 | 0.980 | — | 0.988 | 0.976 | 1.000 | -0.024 |

## 2. Duyarlılık

| Veri | Çeşit | Genel oran değişimi | Ortak kapsül | Hüküm uyuşması | Kapsama | Anahtar başına en büyük \|değişim\| |
|---|---|---|---|---|---|---|
| real-1day | h-20% | -0.015 | 2827 | 0.973 | 0.999 | 0.029 |
| real-1day | h+20% | 0.012 | 2822 | 0.979 | 0.998 | 0.029 |
| real-1day | k-10% | 0.002 | 2796 | 0.959 | 0.988 | 0.026 |
| real-1day | k+10% | -0.015 | 2695 | 0.952 | 0.953 | 0.039 |
| real-1hour | h-20% | -0.026 | 5158 | 0.953 | 0.998 | 0.041 |
| real-1hour | h+20% | 0.011 | 5160 | 0.977 | 0.998 | 0.016 |
| real-1hour | k-10% | 0.010 | 5126 | 0.955 | 0.991 | 0.029 |
| real-1hour | k+10% | -0.010 | 5002 | 0.958 | 0.968 | 0.020 |

## 3. İleriye bakma

| Veri | Kesim t | Karşılaştırılan (t ≤ T) | Farklı olay | Farklı hüküm | Karnesi farklı anahtar | Gelecekte değişen | Koruma ihlali |
|---|---|---|---|---|---|---|---|
| real-1day | 1584514800000 | 1584 | 0 | 0 | 0 | 8341 | 0 |
| real-1day | 1654671600000 | 3468 | 0 | 0 | 0 | 4735 | 0 |
| real-1day | 1724914800000 | 5374 | 0 | 0 | 0 | 2260 | 0 |
| real-1hour | 1747303200000 | 3072 | 0 | 0 | 0 | 13945 | 0 |
| real-1hour | 1763107200000 | 6318 | 0 | 0 | 0 | 9190 | 0 |
| real-1hour | 1778749200000 | 9524 | 0 | 0 | 0 | 4509 | 0 |
