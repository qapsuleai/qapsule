# M3 — `ledger` (synthetic)

Kurallar: tema `reference.kind_direction`, sonuç `reference.atr_barrier`, sıralayıcı `m3.ledger`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 0.46 | 0 | 2 | 8 | 0.726 | 2.08 | 1.37 | 0.130 | 3 | 1.19 |
| havuz | balanced | 6.28 | 6 | 10 | 20 | 0.002 | 0.46 | 1.98 | 0.677 | 5 | 15.93 |
| havuz | all | 10.69 | 10 | 16 | 28 | 0.001 | 0.36 | 1.73 | 0.834 | 6 | 26.38 |
| synthetic-s1 | important | 0.44 | 0 | 1 | 8 | 0.760 | 2.30 | 1.33 | 0.117 | 3 | 1.14 |
| synthetic-s1 | balanced | 6.35 | 6 | 10 | 20 | 0.002 | 0.48 | 2.19 | 0.681 | 5 | 16.05 |
| synthetic-s1 | all | 10.88 | 11 | 16 | 28 | 0.000 | 0.38 | 1.86 | 0.842 | 6 | 26.74 |
| synthetic-s2 | important | 0.42 | 0 | 2 | 5 | 0.773 | 2.27 | 1.17 | 0.138 | 1 | 1.03 |
| synthetic-s2 | balanced | 6.06 | 6 | 9 | 15 | 0.004 | 0.44 | 2.01 | 0.665 | 5 | 15.43 |
| synthetic-s2 | all | 10.68 | 10 | 16 | 27 | 0.002 | 0.36 | 1.69 | 0.828 | 6 | 26.27 |
| synthetic-s3 | important | 0.53 | 0 | 2 | 7 | 0.644 | 1.72 | 1.91 | 0.134 | 2 | 1.38 |
| synthetic-s3 | balanced | 6.42 | 6 | 10 | 17 | 0.000 | 0.46 | 1.73 | 0.685 | 5 | 16.29 |
| synthetic-s3 | all | 10.52 | 10 | 15 | 23 | 0.000 | 0.34 | 1.60 | 0.832 | 6 | 26.13 |

Rejim ortalamaları (enstrüman-gün başına doğum; yükselen / düşen / yatay / oynak), havuz:

| Seviye | yükselen | düşen | yatay | oynak | oran |
|---|---|---|---|---|---|
| important | 0.0162 | 0.0178 | 0.0130 | 0.0169 | 1.37 |
| balanced | 0.2356 | 0.2442 | 0.1463 | 0.2896 | 1.98 |
| all | 0.3932 | 0.3968 | 0.2730 | 0.4711 | 1.73 |

Tohumlar (veri kümeleri) arası fark, günlük ortalama:

| Seviye | en çok − en az | göreli |
|---|---|---|
| important | 0.109 | 0.236 |
| balanced | 0.353 | 0.056 |
| all | 0.353 | 0.033 |

## 3. Sonradan gerçekleşenlerin payı (doğumdaki kademe)

| Veri | K1 kapanan | K1 pay | K2 kapanan | K2 pay | K3 kapanan | K3 pay | Taban | K1 − K3 |
|---|---|---|---|---|---|---|---|---|
| havuz | 625 | 0.608 | 8480 | 0.480 | 6236 | 0.446 | 0.471 | 0.162 |
| synthetic-s1 | 192 | 0.630 | 2839 | 0.483 | 2152 | 0.454 | 0.476 | 0.176 |
| synthetic-s2 | 197 | 0.609 | 2736 | 0.478 | 2198 | 0.451 | 0.472 | 0.158 |
| synthetic-s3 | 236 | 0.589 | 2905 | 0.480 | 1886 | 0.431 | 0.466 | 0.158 |

## 4. İleriye bakma

Sonuç: **geçti**.

| Veri | Kesim t | Karşılaştırılan | Farklı | Kademe farklı | Gelecekte değişen |
|---|---|---|---|---|---|
| synthetic-s1 | 1687762800000 | 2785 | 0 | 0 | 9997 |
| synthetic-s1 | 1702882800000 | 6035 | 0 | 0 | 6747 |
| synthetic-s1 | 1718002800000 | 9327 | 0 | 0 | 3454 |
| synthetic-s2 | 1687762800000 | 2858 | 0 | 0 | 9793 |
| synthetic-s2 | 1702882800000 | 6149 | 0 | 0 | 6502 |
| synthetic-s2 | 1718002800000 | 9384 | 0 | 0 | 3267 |
| synthetic-s3 | 1687762800000 | 2764 | 0 | 0 | 9762 |
| synthetic-s3 | 1702882800000 | 5969 | 0 | 0 | 6554 |
| synthetic-s3 | 1718002800000 | 9224 | 0 | 0 | 3302 |

Hikâye sayısı: synthetic-s1 5225, synthetic-s2 5167, synthetic-s3 5073.
