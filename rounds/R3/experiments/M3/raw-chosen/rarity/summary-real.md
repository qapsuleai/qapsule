# M3 — `rarity` (real)

Kurallar: tema `kinddir`, sonuç `barrier`, sıralayıcı `rarity`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 0.39 | 0 | 1 | 8 | 0.758 | 2.21 | — | 0.500 | 4 | 0.96 |
| havuz | balanced | 1.87 | 1 | 6 | 25 | 0.449 | 1.64 | — | 0.832 | 15 | 5.07 |
| havuz | all | 2.71 | 1 | 8 | 27 | 0.321 | 1.45 | — | 0.879 | 38 | 7.15 |
| real-1day | important | 0.18 | 0 | 1 | 4 | 0.849 | 2.57 | — | 0.158 | 3 | 0.46 |
| real-1day | balanced | 0.73 | 0 | 2 | 9 | 0.549 | 1.45 | — | 0.477 | 4 | 1.97 |
| real-1day | all | 1.22 | 1 | 3 | 9 | 0.393 | 1.17 | — | 0.673 | 7 | 3.18 |
| real-1hour | important | 1.32 | 1 | 3 | 8 | 0.360 | 1.07 | — | 0.700 | 4 | 3.15 |
| real-1hour | balanced | 6.86 | 6 | 12 | 25 | 0.008 | 0.56 | — | 0.997 | 15 | 18.69 |
| real-1hour | all | 9.22 | 9 | 15 | 27 | 0.002 | 0.51 | — | 0.999 | 38 | 24.51 |

Rejim ortalamaları (enstrüman-gün başına doğum; yükselen / düşen / yatay / oynak), havuz:

| Seviye | yükselen | düşen | yatay | oynak | oran |
|---|---|---|---|---|---|
| important | — | — | — | — | — |
| balanced | — | — | — | — | — |
| all | — | — | — | — | — |

Tohumlar (veri kümeleri) arası fark, günlük ortalama:

| Seviye | en çok − en az | göreli |
|---|---|---|
| important | — | — |
| balanced | — | — |
| all | — | — |

## 3. Sonradan gerçekleşenlerin payı (doğumdaki kademe)

| Veri | K1 kapanan | K1 pay | K2 kapanan | K2 pay | K3 kapanan | K3 pay | Taban | K1 − K3 |
|---|---|---|---|---|---|---|---|---|
| havuz | 1046 | 0.520 | 3979 | 0.474 | 2242 | 0.525 | 0.496 | -0.004 |
| real-1day | 388 | 0.564 | 1213 | 0.488 | 1070 | 0.550 | 0.524 | 0.015 |
| real-1hour | 658 | 0.494 | 2766 | 0.469 | 1172 | 0.502 | 0.481 | -0.008 |

## 4. İleriye bakma

Sonuç: **geçti**.

| Veri | Kesim t | Karşılaştırılan | Farklı | Kademe farklı | Gelecekte değişen |
|---|---|---|---|---|---|
| real-1day | 1584514800000 | 1589 | 0 | 0 | 5362 |
| real-1day | 1654671600000 | 3432 | 0 | 0 | 3578 |
| real-1day | 1724914800000 | 5320 | 0 | 0 | 1789 |
| real-1hour | 1747303200000 | 2984 | 0 | 0 | 9545 |
| real-1hour | 1763107200000 | 6100 | 0 | 0 | 6603 |
| real-1hour | 1778749200000 | 9223 | 0 | 0 | 3281 |

Hikâye sayısı: real-1day 2678, real-1hour 4605.
