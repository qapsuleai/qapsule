# M3 — `rarity` (synthetic)

Kurallar: tema `kinddir`, sonuç `barrier`, sıralayıcı `rarity`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 1.07 | 1 | 3 | 5 | 0.364 | 1.00 | 1.42 | 0.186 | 3 | 3.59 |
| havuz | balanced | 5.29 | 5 | 9 | 17 | 0.013 | 0.47 | 1.78 | 0.594 | 6 | 16.23 |
| havuz | all | 8.68 | 8 | 13 | 23 | 0.000 | 0.36 | 1.69 | 0.787 | 6 | 24.60 |
| synthetic-s1 | important | 1.12 | 1 | 3 | 5 | 0.347 | 0.97 | 1.79 | 0.187 | 2 | 3.67 |
| synthetic-s1 | balanced | 5.39 | 5 | 9 | 17 | 0.020 | 0.48 | 1.89 | 0.619 | 5 | 16.25 |
| synthetic-s1 | all | 8.88 | 9 | 13 | 23 | 0.000 | 0.37 | 1.82 | 0.801 | 6 | 24.97 |
| synthetic-s2 | important | 1.07 | 1 | 3 | 5 | 0.371 | 1.01 | 1.49 | 0.191 | 2 | 3.54 |
| synthetic-s2 | balanced | 5.06 | 5 | 8 | 12 | 0.011 | 0.49 | 1.77 | 0.574 | 5 | 15.59 |
| synthetic-s2 | all | 8.64 | 8 | 13 | 19 | 0.000 | 0.36 | 1.68 | 0.783 | 6 | 24.48 |
| synthetic-s3 | important | 1.03 | 1 | 2 | 5 | 0.376 | 1.02 | 1.08 | 0.179 | 3 | 3.57 |
| synthetic-s3 | balanced | 5.42 | 5 | 8 | 13 | 0.007 | 0.44 | 1.65 | 0.590 | 6 | 16.84 |
| synthetic-s3 | all | 8.52 | 8 | 12 | 18 | 0.000 | 0.34 | 1.54 | 0.777 | 6 | 24.37 |

Rejim ortalamaları (enstrüman-gün başına doğum; yükselen / düşen / yatay / oynak), havuz:

| Seviye | yükselen | düşen | yatay | oynak | oran |
|---|---|---|---|---|---|
| important | 0.0378 | 0.0317 | 0.0343 | 0.0449 | 1.42 |
| balanced | 0.1943 | 0.1903 | 0.1361 | 0.2418 | 1.78 |
| all | 0.3206 | 0.3191 | 0.2239 | 0.3779 | 1.69 |

Tohumlar (veri kümeleri) arası fark, günlük ortalama:

| Seviye | en çok − en az | göreli |
|---|---|---|
| important | 0.089 | 0.083 |
| balanced | 0.360 | 0.068 |
| all | 0.358 | 0.041 |

## 3. Sonradan gerçekleşenlerin payı (doğumdaki kademe)

| Veri | K1 kapanan | K1 pay | K2 kapanan | K2 pay | K3 kapanan | K3 pay | Taban | K1 − K3 |
|---|---|---|---|---|---|---|---|---|
| havuz | 1686 | 0.447 | 6058 | 0.463 | 4755 | 0.462 | 0.460 | -0.015 |
| synthetic-s1 | 575 | 0.452 | 2050 | 0.466 | 1615 | 0.481 | 0.470 | -0.029 |
| synthetic-s2 | 557 | 0.449 | 1931 | 0.460 | 1681 | 0.456 | 0.457 | -0.007 |
| synthetic-s3 | 554 | 0.440 | 2077 | 0.462 | 1459 | 0.449 | 0.454 | -0.009 |

## 4. İleriye bakma

Sonuç: **geçti**.

| Veri | Kesim t | Karşılaştırılan | Farklı | Kademe farklı | Gelecekte değişen |
|---|---|---|---|---|---|
| synthetic-s1 | 1687762800000 | 2607 | 0 | 0 | 9344 |
| synthetic-s1 | 1702882800000 | 5639 | 0 | 0 | 6312 |
| synthetic-s1 | 1718002800000 | 8720 | 0 | 0 | 3230 |
| synthetic-s2 | 1687762800000 | 2672 | 0 | 0 | 9133 |
| synthetic-s2 | 1702882800000 | 5758 | 0 | 0 | 6047 |
| synthetic-s2 | 1718002800000 | 8772 | 0 | 0 | 3091 |
| synthetic-s3 | 1687762800000 | 2584 | 0 | 0 | 9115 |
| synthetic-s3 | 1702882800000 | 5594 | 0 | 0 | 6106 |
| synthetic-s3 | 1718002800000 | 8616 | 0 | 0 | 3084 |

Hikâye sayısı: synthetic-s1 4283, synthetic-s2 4203, synthetic-s3 4133.
