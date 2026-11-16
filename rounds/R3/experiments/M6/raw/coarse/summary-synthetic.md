# M6 — `coarse` (synthetic)

Güven yolu `coarse`. Izgara 2001 nokta (±%300), gerçekçi saldırgan 2000 aday, pencere ±3 bar.

## 1. Kâhin eşdeğerlik saldırısı (genişlik / doğal yayılım; küçük = kötü)

| Veri | Hedef | Medyan | p10 | En kötü | Olgulu medyan | Olgusuz medyan | Kapsayan medyan | Bit medyan | Bit p90 | Bit en çok | Çok parçalı | Doymuş | Gerçek tutulmadı |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | 8406 | 1.6869 | 0.3975 | 0.000734 | 1.7492 | 0.2975 | 1.6869 | 0.00 | 1.33 | 10.41 | 0 | 7234 | 0 |
| synthetic-s1 | 2782 | 1.6628 | 0.3913 | 0.003166 | 1.7428 | 0.2952 | 1.6628 | 0.00 | 1.35 | 8.30 | 0 | 2397 | 0 |
| synthetic-s2 | 2820 | 1.7151 | 0.4371 | 0.011595 | 1.7705 | 0.2967 | 1.7151 | 0.00 | 1.19 | 6.43 | 0 | 2431 | 0 |
| synthetic-s3 | 2804 | 1.6507 | 0.3860 | 0.000734 | 1.7225 | 0.2975 | 1.6507 | 0.00 | 1.37 | 10.41 | 0 | 2406 | 0 |

## 2. Gerçekçi saldırgan (sonsal p5–p95 genişliği / yayılım)

| Veri | Hedef | Tutulan aday medyan | p10 | en az | ≥20 tutulan | Sonsal medyan | p10 | en kötü | Önsel medyan | Kapsama |
|---|---|---|---|---|---|---|---|---|---|---|
| havuz | 8406 | 33 | 0 | 0 | 4834 | 0.586 | 0.213 | 0.066 | 0.549 | 0.835 |
| synthetic-s1 | 2782 | 35 | 1 | 0 | 1639 | 0.622 | 0.220 | 0.073 | 0.551 | 0.838 |
| synthetic-s2 | 2820 | 33 | 1 | 0 | 1608 | 0.574 | 0.215 | 0.099 | 0.567 | 0.825 |
| synthetic-s3 | 2804 | 31 | 0 | 0 | 1587 | 0.571 | 0.206 | 0.066 | 0.526 | 0.843 |

## 3–5. Gecikme, dürüstlük, fayda

| Veri | İleti | Gecikme <0 / 0 / 1 / 2 / 3 / 4 / 5–9 / ≥10 | Ort. | En çok | trust=server | Metinde rakam | Kodda rakam | Ekte rakam | TR ifade | EN ifade | Olgu | Fayda |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | 21901 | 0 / 0 / 21901 / 0 / 0 / 0 / 0 / 0 | 1.00 | 1 | 1.000 | 0 | 0 | 0 | 1.000 | 1.000 | 8136 | 0.843 |
| synthetic-s1 | 7387 | 0 / 0 / 7387 / 0 / 0 / 0 / 0 / 0 | 1.00 | 1 | 1.000 | 0 | 0 | 0 | 1.000 | 1.000 | 2692 | 0.847 |
| synthetic-s2 | 7251 | 0 / 0 / 7251 / 0 / 0 / 0 / 0 / 0 | 1.00 | 1 | 1.000 | 0 | 0 | 0 | 1.000 | 1.000 | 2730 | 0.846 |
| synthetic-s3 | 7263 | 0 / 0 / 7263 / 0 / 0 / 0 / 0 / 0 | 1.00 | 1 | 1.000 | 0 | 0 | 0 | 1.000 | 1.000 | 2714 | 0.838 |
