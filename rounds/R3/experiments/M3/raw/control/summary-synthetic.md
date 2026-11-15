# M3 — `control` (synthetic)

Kurallar: tema `reference.kind_direction`, sonuç `reference.atr_barrier`, sıralayıcı `control.round_robin`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 3.56 | 3 | 5 | 9 | 0.002 | 0.38 | 1.77 | 0.453 | 4 | 8.80 |
| havuz | balanced | 7.13 | 7 | 10 | 19 | 0.001 | 0.37 | 1.77 | 0.706 | 6 | 17.56 |
| havuz | all | 10.69 | 10 | 16 | 28 | 0.001 | 0.36 | 1.73 | 0.834 | 6 | 26.38 |
| synthetic-s1 | important | 3.62 | 4 | 5 | 9 | 0.002 | 0.40 | 1.76 | 0.454 | 4 | 8.94 |
| synthetic-s1 | balanced | 7.25 | 7 | 11 | 19 | 0.000 | 0.39 | 1.80 | 0.715 | 5 | 17.76 |
| synthetic-s1 | all | 10.88 | 11 | 16 | 28 | 0.000 | 0.38 | 1.86 | 0.842 | 6 | 26.74 |
| synthetic-s2 | important | 3.56 | 3 | 5 | 9 | 0.004 | 0.38 | 1.81 | 0.451 | 4 | 8.74 |
| synthetic-s2 | balanced | 7.12 | 7 | 10 | 18 | 0.002 | 0.37 | 1.76 | 0.702 | 5 | 17.52 |
| synthetic-s2 | all | 10.68 | 10 | 16 | 27 | 0.002 | 0.36 | 1.69 | 0.828 | 6 | 26.27 |
| synthetic-s3 | important | 3.51 | 3 | 5 | 8 | 0.000 | 0.35 | 1.72 | 0.455 | 4 | 8.71 |
| synthetic-s3 | balanced | 7.01 | 7 | 10 | 15 | 0.000 | 0.34 | 1.73 | 0.702 | 6 | 17.38 |
| synthetic-s3 | all | 10.52 | 10 | 15 | 23 | 0.000 | 0.34 | 1.60 | 0.832 | 6 | 26.13 |

Rejim ortalamaları (enstrüman-gün başına doğum; yükselen / düşen / yatay / oynak), havuz:

| Seviye | yükselen | düşen | yatay | oynak | oran |
|---|---|---|---|---|---|
| important | 0.1297 | 0.1343 | 0.0901 | 0.1591 | 1.77 |
| balanced | 0.2606 | 0.2672 | 0.1802 | 0.3181 | 1.77 |
| all | 0.3932 | 0.3968 | 0.2730 | 0.4711 | 1.73 |

Tohumlar (veri kümeleri) arası fark, günlük ortalama:

| Seviye | en çok − en az | göreli |
|---|---|---|
| important | 0.118 | 0.033 |
| balanced | 0.238 | 0.033 |
| all | 0.353 | 0.033 |

## 3. Sonradan gerçekleşenlerin payı (doğumdaki kademe)

| Veri | K1 kapanan | K1 pay | K2 kapanan | K2 pay | K3 kapanan | K3 pay | Taban | K1 − K3 |
|---|---|---|---|---|---|---|---|---|
| havuz | 5112 | 0.465 | 5113 | 0.473 | 5116 | 0.477 | 0.471 | -0.012 |
| synthetic-s1 | 1727 | 0.466 | 1728 | 0.479 | 1728 | 0.484 | 0.476 | -0.018 |
| synthetic-s2 | 1708 | 0.466 | 1711 | 0.464 | 1712 | 0.485 | 0.472 | -0.019 |
| synthetic-s3 | 1677 | 0.462 | 1674 | 0.477 | 1676 | 0.461 | 0.466 | 0.001 |

## 4. İleriye bakma

Sonuç: **geçti**.

| Veri | Kesim t | Karşılaştırılan | Farklı | Kademe farklı | Gelecekte değişen |
|---|---|---|---|---|---|
| synthetic-s1 | 1687762800000 | 2785 | 0 | 0 | 9997 |
| synthetic-s1 | 1702882800000 | 6035 | 0 | 0 | 6746 |
| synthetic-s1 | 1718002800000 | 9327 | 0 | 0 | 3454 |
| synthetic-s2 | 1687762800000 | 2858 | 0 | 0 | 9793 |
| synthetic-s2 | 1702882800000 | 6149 | 0 | 0 | 6502 |
| synthetic-s2 | 1718002800000 | 9384 | 0 | 0 | 3267 |
| synthetic-s3 | 1687762800000 | 2764 | 0 | 0 | 9762 |
| synthetic-s3 | 1702882800000 | 5969 | 0 | 0 | 6555 |
| synthetic-s3 | 1718002800000 | 9224 | 0 | 0 | 3302 |

Hikâye sayısı: synthetic-s1 5225, synthetic-s2 5167, synthetic-s3 5073.
