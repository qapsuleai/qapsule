# M3 — `control` (real)

Kurallar: tema `reference.kind_direction`, sonuç `reference.atr_barrier`, sıralayıcı `control.round_robin`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 0.99 | 0 | 3 | 11 | 0.509 | 1.55 | — | 0.730 | 9 | 2.44 |
| havuz | balanced | 1.99 | 1 | 6 | 22 | 0.390 | 1.52 | — | 0.839 | 19 | 4.87 |
| havuz | all | 2.98 | 1 | 9 | 33 | 0.315 | 1.50 | — | 0.890 | 41 | 7.31 |
| real-1day | important | 0.43 | 0 | 1 | 3 | 0.621 | 1.39 | — | 0.308 | 4 | 1.07 |
| real-1day | balanced | 0.86 | 1 | 2 | 6 | 0.476 | 1.23 | — | 0.551 | 4 | 2.13 |
| real-1day | all | 1.30 | 1 | 3 | 9 | 0.386 | 1.17 | — | 0.691 | 7 | 3.21 |
| real-1hour | important | 3.46 | 3 | 6 | 11 | 0.022 | 0.56 | — | 0.961 | 9 | 8.45 |
| real-1hour | balanced | 6.91 | 6 | 12 | 22 | 0.010 | 0.54 | — | 0.997 | 19 | 16.89 |
| real-1hour | all | 10.37 | 10 | 18 | 33 | 0.002 | 0.54 | — | 0.999 | 41 | 25.29 |

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
| havuz | 2668 | 0.523 | 2666 | 0.503 | 2665 | 0.509 | 0.512 | 0.014 |
| real-1day | 944 | 0.549 | 943 | 0.548 | 942 | 0.541 | 0.546 | 0.007 |
| real-1hour | 1724 | 0.509 | 1723 | 0.478 | 1723 | 0.491 | 0.493 | 0.018 |

## 4. İleriye bakma

Sonuç: **geçti**.

| Veri | Kesim t | Karşılaştırılan | Farklı | Kademe farklı | Gelecekte değişen |
|---|---|---|---|---|---|
| real-1day | 1584514800000 | 1580 | 0 | 0 | 5435 |
| real-1day | 1654671600000 | 3463 | 0 | 0 | 3552 |
| real-1day | 1724914800000 | 5370 | 0 | 0 | 1740 |
| real-1hour | 1747303200000 | 3072 | 0 | 0 | 9539 |
| real-1hour | 1763107200000 | 6317 | 0 | 0 | 6493 |
| real-1hour | 1778749200000 | 9520 | 0 | 0 | 3219 |

Hikâye sayısı: real-1day 2834, real-1hour 5174.
