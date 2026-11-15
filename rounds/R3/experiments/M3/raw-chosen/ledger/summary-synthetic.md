# M3 — `ledger` (synthetic)

Kurallar: tema `kinddir`, sonuç `barrier`, sıralayıcı `m3.ledger`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 1.00 | 0 | 4 | 13 | 0.687 | 1.91 | 2.38 | 0.450 | 4 | 3.19 |
| havuz | balanced | 6.51 | 6 | 10 | 23 | 0.001 | 0.46 | 1.97 | 0.696 | 6 | 19.73 |
| havuz | all | 8.68 | 8 | 13 | 23 | 0.000 | 0.36 | 1.69 | 0.787 | 6 | 24.60 |
| synthetic-s1 | important | 1.53 | 0 | 5 | 13 | 0.553 | 1.52 | 2.61 | 0.484 | 4 | 4.88 |
| synthetic-s1 | balanced | 7.52 | 7 | 12 | 23 | 0.000 | 0.46 | 1.98 | 0.747 | 6 | 21.99 |
| synthetic-s1 | all | 8.88 | 9 | 13 | 23 | 0.000 | 0.37 | 1.82 | 0.801 | 6 | 24.97 |
| synthetic-s2 | important | 0.09 | 0 | 0 | 5 | 0.964 | 6.06 | 2.71 | 0.293 | 2 | 0.34 |
| synthetic-s2 | balanced | 5.98 | 6 | 9 | 14 | 0.004 | 0.44 | 2.02 | 0.666 | 5 | 18.47 |
| synthetic-s2 | all | 8.64 | 8 | 13 | 19 | 0.000 | 0.36 | 1.68 | 0.783 | 6 | 24.48 |
| synthetic-s3 | important | 1.36 | 0 | 4 | 10 | 0.542 | 1.43 | 2.42 | 0.422 | 3 | 4.34 |
| synthetic-s3 | balanced | 6.04 | 6 | 10 | 16 | 0.000 | 0.44 | 1.84 | 0.662 | 5 | 18.72 |
| synthetic-s3 | all | 8.52 | 8 | 12 | 18 | 0.000 | 0.34 | 1.54 | 0.777 | 6 | 24.37 |

Rejim ortalamaları (enstrüman-gün başına doğum; yükselen / düşen / yatay / oynak), havuz:

| Seviye | yükselen | düşen | yatay | oynak | oran |
|---|---|---|---|---|---|
| important | 0.0401 | 0.0434 | 0.0189 | 0.0449 | 2.38 |
| balanced | 0.2482 | 0.2502 | 0.1515 | 0.2990 | 1.97 |
| all | 0.3206 | 0.3191 | 0.2239 | 0.3779 | 1.69 |

Tohumlar (veri kümeleri) arası fark, günlük ortalama:

| Seviye | en çok − en az | göreli |
|---|---|---|
| important | 1.442 | 1.448 |
| balanced | 1.540 | 0.236 |
| all | 0.358 | 0.041 |

## 3. Sonradan gerçekleşenlerin payı (doğumdaki kademe)

| Veri | K1 kapanan | K1 pay | K2 kapanan | K2 pay | K3 kapanan | K3 pay | Taban | K1 − K3 |
|---|---|---|---|---|---|---|---|---|
| havuz | 1369 | 0.463 | 7994 | 0.469 | 3136 | 0.437 | 0.460 | 0.027 |
| synthetic-s1 | 688 | 0.471 | 2847 | 0.470 | 705 | 0.468 | 0.470 | 0.003 |
| synthetic-s2 | 59 | 0.356 | 2830 | 0.469 | 1280 | 0.434 | 0.457 | -0.078 |
| synthetic-s3 | 622 | 0.465 | 2317 | 0.468 | 1151 | 0.421 | 0.454 | 0.044 |

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
