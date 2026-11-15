# M3 — `reference` (synthetic)

Kurallar: tema `reference.kind_direction`, sonuç `reference.atr_barrier`, sıralayıcı `reference.families`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 0.64 | 0 | 2 | 4 | 0.520 | 1.22 | 6.05 | 0.080 | 2 | 2.24 |
| havuz | balanced | 5.58 | 5 | 9 | 18 | 0.005 | 0.48 | 1.99 | 0.596 | 5 | 14.88 |
| havuz | all | 10.69 | 10 | 16 | 28 | 0.001 | 0.36 | 1.73 | 0.834 | 6 | 26.38 |
| synthetic-s1 | important | 0.66 | 0 | 2 | 4 | 0.504 | 1.20 | 7.46 | 0.104 | 2 | 2.25 |
| synthetic-s1 | balanced | 5.70 | 5 | 9 | 18 | 0.004 | 0.49 | 2.11 | 0.611 | 5 | 15.06 |
| synthetic-s1 | all | 10.88 | 11 | 16 | 28 | 0.000 | 0.38 | 1.86 | 0.842 | 6 | 26.74 |
| synthetic-s2 | important | 0.62 | 0 | 2 | 4 | 0.547 | 1.26 | 5.93 | 0.065 | 2 | 2.20 |
| synthetic-s2 | balanced | 5.52 | 5 | 9 | 17 | 0.007 | 0.49 | 1.96 | 0.586 | 4 | 14.65 |
| synthetic-s2 | all | 10.68 | 10 | 16 | 27 | 0.002 | 0.36 | 1.69 | 0.828 | 6 | 26.27 |
| synthetic-s3 | important | 0.65 | 0 | 2 | 4 | 0.509 | 1.20 | 4.92 | 0.072 | 2 | 2.27 |
| synthetic-s3 | balanced | 5.51 | 5 | 9 | 18 | 0.004 | 0.47 | 1.84 | 0.589 | 4 | 14.93 |
| synthetic-s3 | all | 10.52 | 10 | 15 | 23 | 0.000 | 0.34 | 1.60 | 0.832 | 6 | 26.13 |

Rejim ortalamaları (enstrüman-gün başına doğum; yükselen / düşen / yatay / oynak), havuz:

| Seviye | yükselen | düşen | yatay | oynak | oran |
|---|---|---|---|---|---|
| important | 0.0244 | 0.0265 | 0.0082 | 0.0495 | 6.05 |
| balanced | 0.2103 | 0.2121 | 0.1313 | 0.2606 | 1.99 |
| all | 0.3932 | 0.3968 | 0.2730 | 0.4711 | 1.73 |

Tohumlar (veri kümeleri) arası fark, günlük ortalama:

| Seviye | en çok − en az | göreli |
|---|---|---|
| important | 0.042 | 0.066 |
| balanced | 0.196 | 0.035 |
| all | 0.353 | 0.033 |

## 3. Sonradan gerçekleşenlerin payı (doğumdaki kademe)

| Veri | K1 kapanan | K1 pay | K2 kapanan | K2 pay | K3 kapanan | K3 pay | Taban | K1 − K3 |
|---|---|---|---|---|---|---|---|---|
| havuz | 922 | 0.470 | 7106 | 0.485 | 7313 | 0.458 | 0.471 | 0.011 |
| synthetic-s1 | 318 | 0.500 | 2410 | 0.489 | 2455 | 0.461 | 0.476 | 0.039 |
| synthetic-s2 | 299 | 0.445 | 2370 | 0.481 | 2462 | 0.466 | 0.472 | -0.021 |
| synthetic-s3 | 305 | 0.462 | 2326 | 0.486 | 2396 | 0.448 | 0.466 | 0.014 |

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
| synthetic-s3 | 1702882800000 | 5969 | 0 | 0 | 6554 |
| synthetic-s3 | 1718002800000 | 9224 | 0 | 0 | 3302 |

Hikâye sayısı: synthetic-s1 5225, synthetic-s2 5167, synthetic-s3 5073.
