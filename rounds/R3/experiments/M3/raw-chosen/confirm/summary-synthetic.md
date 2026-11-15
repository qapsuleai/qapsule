# M3 — `confirm` (synthetic)

Kurallar: tema `kinddir`, sonuç `barrier`, sıralayıcı `confirm`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 1.79 | 2 | 4 | 8 | 0.150 | 0.74 | 2.82 | 0.222 | 3 | 6.98 |
| havuz | balanced | 3.78 | 4 | 6 | 14 | 0.031 | 0.55 | 2.15 | 0.474 | 5 | 11.90 |
| havuz | all | 8.68 | 8 | 13 | 23 | 0.000 | 0.36 | 1.69 | 0.787 | 6 | 24.60 |
| synthetic-s1 | important | 1.77 | 2 | 4 | 7 | 0.142 | 0.74 | 2.84 | 0.218 | 3 | 6.87 |
| synthetic-s1 | balanced | 3.97 | 4 | 7 | 14 | 0.031 | 0.56 | 2.15 | 0.502 | 5 | 12.35 |
| synthetic-s1 | all | 8.88 | 9 | 13 | 23 | 0.000 | 0.37 | 1.82 | 0.801 | 6 | 24.97 |
| synthetic-s2 | important | 1.92 | 2 | 4 | 7 | 0.136 | 0.72 | 2.65 | 0.227 | 3 | 7.30 |
| synthetic-s2 | balanced | 3.66 | 3 | 6 | 10 | 0.038 | 0.55 | 2.06 | 0.449 | 5 | 11.76 |
| synthetic-s2 | all | 8.64 | 8 | 13 | 19 | 0.000 | 0.36 | 1.68 | 0.783 | 6 | 24.48 |
| synthetic-s3 | important | 1.69 | 1 | 3 | 8 | 0.173 | 0.77 | 3.03 | 0.221 | 3 | 6.76 |
| synthetic-s3 | balanced | 3.72 | 3 | 6 | 13 | 0.024 | 0.55 | 2.22 | 0.470 | 4 | 11.58 |
| synthetic-s3 | all | 8.52 | 8 | 12 | 18 | 0.000 | 0.34 | 1.54 | 0.777 | 6 | 24.37 |

Rejim ortalamaları (enstrüman-gün başına doğum; yükselen / düşen / yatay / oynak), havuz:

| Seviye | yükselen | düşen | yatay | oynak | oran |
|---|---|---|---|---|---|
| important | 0.0676 | 0.0700 | 0.0359 | 0.1015 | 2.82 |
| balanced | 0.1442 | 0.1383 | 0.0878 | 0.1887 | 2.15 |
| all | 0.3206 | 0.3191 | 0.2239 | 0.3779 | 1.69 |

Tohumlar (veri kümeleri) arası fark, günlük ortalama:

| Seviye | en çok − en az | göreli |
|---|---|---|
| important | 0.233 | 0.130 |
| balanced | 0.318 | 0.084 |
| all | 0.358 | 0.041 |

## 3. Sonradan gerçekleşenlerin payı (doğumdaki kademe)

| Veri | K1 kapanan | K1 pay | K2 kapanan | K2 pay | K3 kapanan | K3 pay | Taban | K1 − K3 |
|---|---|---|---|---|---|---|---|---|
| havuz | 2591 | 0.475 | 2842 | 0.454 | 7066 | 0.457 | 0.460 | 0.018 |
| synthetic-s1 | 844 | 0.480 | 1046 | 0.467 | 2350 | 0.467 | 0.470 | 0.013 |
| synthetic-s2 | 935 | 0.486 | 831 | 0.421 | 2403 | 0.458 | 0.457 | 0.028 |
| synthetic-s3 | 812 | 0.459 | 965 | 0.467 | 2313 | 0.447 | 0.454 | 0.012 |

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
