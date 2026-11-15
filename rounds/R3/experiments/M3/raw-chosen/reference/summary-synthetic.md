# M3 — `reference` (synthetic)

Kurallar: tema `kinddir`, sonuç `barrier`, sıralayıcı `reference.families`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 1.57 | 1 | 3 | 7 | 0.196 | 0.78 | 3.35 | 0.190 | 3 | 7.55 |
| havuz | balanced | 4.60 | 4 | 7 | 14 | 0.011 | 0.48 | 1.84 | 0.520 | 5 | 15.24 |
| havuz | all | 8.68 | 8 | 13 | 23 | 0.000 | 0.36 | 1.69 | 0.787 | 6 | 24.60 |
| synthetic-s1 | important | 1.61 | 1 | 3 | 7 | 0.176 | 0.77 | 3.46 | 0.199 | 3 | 7.65 |
| synthetic-s1 | balanced | 4.73 | 4 | 7 | 14 | 0.009 | 0.48 | 1.93 | 0.544 | 5 | 15.53 |
| synthetic-s1 | all | 8.88 | 9 | 13 | 23 | 0.000 | 0.37 | 1.82 | 0.801 | 6 | 24.97 |
| synthetic-s2 | important | 1.55 | 1 | 3 | 6 | 0.211 | 0.79 | 3.18 | 0.183 | 3 | 7.49 |
| synthetic-s2 | balanced | 4.57 | 4 | 8 | 11 | 0.016 | 0.48 | 1.81 | 0.512 | 5 | 15.03 |
| synthetic-s2 | all | 8.64 | 8 | 13 | 19 | 0.000 | 0.36 | 1.68 | 0.783 | 6 | 24.48 |
| synthetic-s3 | important | 1.55 | 1 | 3 | 6 | 0.200 | 0.79 | 3.35 | 0.188 | 3 | 7.50 |
| synthetic-s3 | balanced | 4.49 | 4 | 7 | 12 | 0.009 | 0.46 | 1.75 | 0.504 | 4 | 15.17 |
| synthetic-s3 | all | 8.52 | 8 | 12 | 18 | 0.000 | 0.34 | 1.54 | 0.777 | 6 | 24.37 |

Rejim ortalamaları (enstrüman-gün başına doğum; yükselen / düşen / yatay / oynak), havuz:

| Seviye | yükselen | düşen | yatay | oynak | oran |
|---|---|---|---|---|---|
| important | 0.0611 | 0.0639 | 0.0276 | 0.0923 | 3.35 |
| balanced | 0.1723 | 0.1698 | 0.1138 | 0.2093 | 1.84 |
| all | 0.3206 | 0.3191 | 0.2239 | 0.3779 | 1.69 |

Tohumlar (veri kümeleri) arası fark, günlük ortalama:

| Seviye | en çok − en az | göreli |
|---|---|---|
| important | 0.060 | 0.038 |
| balanced | 0.240 | 0.052 |
| all | 0.358 | 0.041 |

## 3. Sonradan gerçekleşenlerin payı (doğumdaki kademe)

| Veri | K1 kapanan | K1 pay | K2 kapanan | K2 pay | K3 kapanan | K3 pay | Taban | K1 − K3 |
|---|---|---|---|---|---|---|---|---|
| havuz | 2254 | 0.481 | 4400 | 0.448 | 5845 | 0.462 | 0.460 | 0.019 |
| synthetic-s1 | 763 | 0.477 | 1505 | 0.455 | 1972 | 0.478 | 0.470 | -0.001 |
| synthetic-s2 | 752 | 0.496 | 1472 | 0.435 | 1945 | 0.458 | 0.457 | 0.038 |
| synthetic-s3 | 739 | 0.471 | 1423 | 0.453 | 1928 | 0.449 | 0.454 | 0.022 |

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
