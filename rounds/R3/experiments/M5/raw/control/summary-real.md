# M5 — `control` (real)

Kurallar: tema `reference.kind_direction`, sonuç `control.always_realized`, sıralayıcı `reference.families`.

## Özet

| Veri | Kapanan | Gerçekleşen | Geçersiz | Süresi dolan | İsabet | Süresi dolan payı | maks \|p(25)−p(son)\| | maks \|p(50)−p(son)\| | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| real-1day | 3672 | 3672 | 0 | 0 | 1.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 1.000 | 1.000 | 0.000 | geçti |
| real-1hour | 6442 | 6442 | 0 | 0 | 1.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 1.000 | 1.000 | 0.000 | geçti |

## 1. Kararlılık (anahtar başına)

| Veri | Anahtar | n | p(25) | p(50) | p(100) | p(200) | p(son) | ilk yarı | ikinci yarı | fark |
|---|---|---|---|---|---|---|---|---|---|---|
| real-1day | `extreme/extreme.rsi@1` | 429 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| real-1day | `breakout/breakout.range@1` | 1915 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| real-1day | `reversal/reversal.rsi_divergence@1` | 245 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| real-1day | `flow/flow.volume_surge@1` | 256 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| real-1day | `continuation/continuation.ma_stack@1` | 730 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| real-1day | `squeeze/squeeze.bandwidth@1` | 97 | 1.000 | 1.000 | — | — | 1.000 | 1.000 | 1.000 | 0.000 |
| real-1hour | `extreme/extreme.rsi@1` | 895 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| real-1hour | `breakout/breakout.range@1` | 2694 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| real-1hour | `reversal/reversal.rsi_divergence@1` | 409 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| real-1hour | `flow/flow.volume_surge@1` | 988 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| real-1hour | `continuation/continuation.ma_stack@1` | 1282 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| real-1hour | `squeeze/squeeze.bandwidth@1` | 174 | 1.000 | 1.000 | 1.000 | — | 1.000 | 1.000 | 1.000 | 0.000 |

## 2. Duyarlılık

| Veri | Çeşit | Genel oran değişimi | Ortak kapsül | Hüküm uyuşması | Kapsama | Anahtar başına en büyük \|değişim\| |
|---|---|---|---|---|---|---|
| real-1day | h-20% | 0.000 | 3672 | 1.000 | 1.000 | 0.000 |
| real-1day | h+20% | 0.000 | 3672 | 1.000 | 1.000 | 0.000 |
| real-1hour | h-20% | 0.000 | 6442 | 1.000 | 1.000 | 0.000 |
| real-1hour | h+20% | 0.000 | 6442 | 1.000 | 1.000 | 0.000 |

## 3. İleriye bakma

| Veri | Kesim t | Karşılaştırılan (t ≤ T) | Farklı olay | Farklı hüküm | Karnesi farklı anahtar | Gelecekte değişen | Koruma ihlali |
|---|---|---|---|---|---|---|---|
| real-1day | 1584514800000 | 1660 | 0 | 0 | 0 | 8168 | 0 |
| real-1day | 1654671600000 | 3632 | 0 | 0 | 0 | 4528 | 0 |
| real-1day | 1724914800000 | 5635 | 0 | 0 | 0 | 2140 | 0 |
| real-1hour | 1747303200000 | 3130 | 0 | 0 | 0 | 13404 | 0 |
| real-1hour | 1763107200000 | 6456 | 0 | 0 | 0 | 8850 | 0 |
| real-1hour | 1778749200000 | 9736 | 0 | 0 | 0 | 4309 | 0 |
