# M5 — `control` (synthetic)

Kurallar: tema `reference.kind_direction`, sonuç `control.always_realized`, sıralayıcı `reference.families`.

## Özet

| Veri | Kapanan | Gerçekleşen | Geçersiz | Süresi dolan | İsabet | Süresi dolan payı | maks \|p(25)−p(son)\| | maks \|p(50)−p(son)\| | maks \|p(100)−p(son)\| | maks \|p(200)−p(son)\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| synthetic-s1 | 6713 | 6713 | 0 | 0 | 1.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 1.000 | 1.000 | 0.000 | geçti |
| synthetic-s2 | 6646 | 6646 | 0 | 0 | 1.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 1.000 | 1.000 | 0.000 | geçti |
| synthetic-s3 | 6607 | 6607 | 0 | 0 | 1.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 1.000 | 1.000 | 0.000 | geçti |

## 1. Kararlılık (anahtar başına)

| Veri | Anahtar | n | p(25) | p(50) | p(100) | p(200) | p(son) | ilk yarı | ikinci yarı | fark |
|---|---|---|---|---|---|---|---|---|---|---|
| synthetic-s1 | `flow/flow.institutional@1` | 2686 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s1 | `extreme/extreme.rsi@1` | 435 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s1 | `breakout/breakout.range@1` | 2302 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s1 | `flow/flow.volume_surge@1` | 80 | 1.000 | 1.000 | — | — | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s1 | `reversal/reversal.rsi_divergence@1` | 293 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s1 | `continuation/continuation.ma_stack@1` | 844 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s1 | `squeeze/squeeze.bandwidth@1` | 73 | 1.000 | 1.000 | — | — | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s2 | `flow/flow.institutional@1` | 2726 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s2 | `extreme/extreme.rsi@1` | 445 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s2 | `breakout/breakout.range@1` | 2196 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s2 | `flow/flow.volume_surge@1` | 72 | 1.000 | 1.000 | — | — | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s2 | `reversal/reversal.rsi_divergence@1` | 299 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s2 | `continuation/continuation.ma_stack@1` | 832 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s2 | `squeeze/squeeze.bandwidth@1` | 76 | 1.000 | 1.000 | — | — | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s3 | `flow/flow.institutional@1` | 2710 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s3 | `extreme/extreme.rsi@1` | 427 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s3 | `breakout/breakout.range@1` | 2165 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s3 | `flow/flow.volume_surge@1` | 81 | 1.000 | 1.000 | — | — | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s3 | `reversal/reversal.rsi_divergence@1` | 319 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s3 | `continuation/continuation.ma_stack@1` | 830 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 0.000 |
| synthetic-s3 | `squeeze/squeeze.bandwidth@1` | 75 | 1.000 | 1.000 | — | — | 1.000 | 1.000 | 1.000 | 0.000 |

## 2. Duyarlılık

| Veri | Çeşit | Genel oran değişimi | Ortak kapsül | Hüküm uyuşması | Kapsama | Anahtar başına en büyük \|değişim\| |
|---|---|---|---|---|---|---|
| synthetic-s1 | h-20% | 0.000 | 6713 | 1.000 | 1.000 | 0.000 |
| synthetic-s1 | h+20% | 0.000 | 6713 | 1.000 | 1.000 | 0.000 |
| synthetic-s2 | h-20% | 0.000 | 6646 | 1.000 | 1.000 | 0.000 |
| synthetic-s2 | h+20% | 0.000 | 6646 | 1.000 | 1.000 | 0.000 |
| synthetic-s3 | h-20% | 0.000 | 6607 | 1.000 | 1.000 | 0.000 |
| synthetic-s3 | h+20% | 0.000 | 6607 | 1.000 | 1.000 | 0.000 |

## 3. İleriye bakma

| Veri | Kesim t | Karşılaştırılan (t ≤ T) | Farklı olay | Farklı hüküm | Karnesi farklı anahtar | Gelecekte değişen | Koruma ihlali |
|---|---|---|---|---|---|---|---|
| synthetic-s1 | 1687762800000 | 3106 | 0 | 0 | 0 | 11610 | 0 |
| synthetic-s1 | 1702882800000 | 6609 | 0 | 0 | 0 | 7623 | 0 |
| synthetic-s1 | 1718002800000 | 10153 | 0 | 0 | 0 | 3788 | 0 |
| synthetic-s2 | 1687762800000 | 3162 | 0 | 0 | 0 | 11687 | 0 |
| synthetic-s2 | 1702882800000 | 6696 | 0 | 0 | 0 | 7705 | 0 |
| synthetic-s2 | 1718002800000 | 10165 | 0 | 0 | 0 | 3778 | 0 |
| synthetic-s3 | 1687762800000 | 3073 | 0 | 0 | 0 | 11643 | 0 |
| synthetic-s3 | 1702882800000 | 6521 | 0 | 0 | 0 | 7716 | 0 |
| synthetic-s3 | 1718002800000 | 10045 | 0 | 0 | 0 | 3787 | 0 |
