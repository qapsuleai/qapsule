# M6 — `control` (synthetic)

Güven yolu `control.echo`. Izgara 2001 nokta (±%300), gerçekçi saldırgan 2000 aday, pencere ±3 bar.

## 1. Kâhin eşdeğerlik saldırısı (genişlik / doğal yayılım; küçük = kötü)

| Veri | Hedef | Medyan | p10 | En kötü | Olgulu medyan | Olgusuz medyan | Kapsayan medyan | Bit medyan | Bit p90 | Bit en çok | Çok parçalı | Doymuş | Gerçek tutulmadı |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | 8406 | 1.1712 | 0.1971 | 0.000734 | 1.2195 | 0.3562 | 1.1742 | 0.00 | 2.34 | 10.41 | 56 | 5416 | 0 |
| synthetic-s1 | 2782 | 1.1557 | 0.2033 | 0.003166 | 1.1918 | 0.3562 | 1.1559 | 0.00 | 2.30 | 8.30 | 16 | 1797 | 0 |
| synthetic-s2 | 2820 | 1.1797 | 0.1935 | 0.003787 | 1.2294 | 0.4162 | 1.1849 | 0.00 | 2.36 | 8.04 | 17 | 1797 | 0 |
| synthetic-s3 | 2804 | 1.1770 | 0.1942 | 0.000734 | 1.2345 | 0.3177 | 1.1793 | 0.00 | 2.36 | 10.41 | 23 | 1822 | 0 |

## 2. Gerçekçi saldırgan (sonsal p5–p95 genişliği / yayılım)

| Veri | Hedef | Tutulan aday medyan | p10 | en az | ≥20 tutulan | Sonsal medyan | p10 | en kötü | Önsel medyan | Kapsama |
|---|---|---|---|---|---|---|---|---|---|---|
| havuz | 8406 | 120 | 4 | 0 | 6432 | 1.083 | 0.296 | 0.049 | 0.549 | 0.889 |
| synthetic-s1 | 2782 | 127 | 5 | 0 | 2138 | 1.077 | 0.293 | 0.095 | 0.551 | 0.891 |
| synthetic-s2 | 2820 | 117 | 5 | 0 | 2164 | 1.101 | 0.309 | 0.049 | 0.567 | 0.898 |
| synthetic-s3 | 2804 | 119 | 4 | 0 | 2130 | 1.071 | 0.281 | 0.075 | 0.526 | 0.878 |

## 3–5. Gecikme, dürüstlük, fayda

| Veri | İleti | Gecikme <0 / 0 / 1 / 2 / 3 / 4 / 5–9 / ≥10 | Ort. | En çok | trust=server | Metinde rakam | Kodda rakam | Ekte rakam | TR ifade | EN ifade | Olgu | Fayda |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | 8136 | 0 / 8136 / 0 / 0 / 0 / 0 / 0 / 0 | 0.00 | 0 | 1.000 | 0 | 0 | 0 | 1.000 | 1.000 | 8136 | 1.000 |
| synthetic-s1 | 2692 | 0 / 2692 / 0 / 0 / 0 / 0 / 0 / 0 | 0.00 | 0 | 1.000 | 0 | 0 | 0 | 1.000 | 1.000 | 2692 | 1.000 |
| synthetic-s2 | 2730 | 0 / 2730 / 0 / 0 / 0 / 0 / 0 / 0 | 0.00 | 0 | 1.000 | 0 | 0 | 0 | 1.000 | 1.000 | 2730 | 1.000 |
| synthetic-s3 | 2714 | 0 / 2714 / 0 / 0 / 0 / 0 / 0 / 0 | 0.00 | 0 | 1.000 | 0 | 0 | 0 | 1.000 | 1.000 | 2714 | 1.000 |
