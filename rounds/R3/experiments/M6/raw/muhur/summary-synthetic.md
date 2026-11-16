# M6 — `muhur` (synthetic)

Güven yolu `m6.muhur`. Izgara 2001 nokta (±%300), gerçekçi saldırgan 2000 aday, pencere ±3 bar.

## 1. Kâhin eşdeğerlik saldırısı (genişlik / doğal yayılım; küçük = kötü)

| Veri | Hedef | Medyan | p10 | En kötü | Olgulu medyan | Olgusuz medyan | Kapsayan medyan | Bit medyan | Bit p90 | Bit en çok | Çok parçalı | Doymuş | Gerçek tutulmadı |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | 8406 | 1.5566 | 0.4278 | 0.000734 | 1.6060 | 0.3709 | 1.5742 | 0.00 | 1.22 | 10.41 | 790 | 6948 | 0 |
| synthetic-s1 | 2782 | 1.5439 | 0.4278 | 0.003166 | 1.5902 | 0.3736 | 1.5615 | 0.00 | 1.22 | 8.30 | 272 | 2280 | 0 |
| synthetic-s2 | 2820 | 1.5896 | 0.4434 | 0.011595 | 1.6647 | 0.3934 | 1.6127 | 0.00 | 1.17 | 6.43 | 242 | 2350 | 0 |
| synthetic-s3 | 2804 | 1.5282 | 0.4194 | 0.000734 | 1.5700 | 0.2975 | 1.5648 | 0.00 | 1.25 | 10.41 | 276 | 2318 | 0 |

## 2. Gerçekçi saldırgan (sonsal p5–p95 genişliği / yayılım)

| Veri | Hedef | Tutulan aday medyan | p10 | en az | ≥20 tutulan | Sonsal medyan | p10 | en kötü | Önsel medyan | Kapsama |
|---|---|---|---|---|---|---|---|---|---|---|
| havuz | 8406 | 99 | 4 | 0 | 6225 | 0.851 | 0.281 | 0.066 | 0.549 | 0.853 |
| synthetic-s1 | 2782 | 109 | 4 | 0 | 2118 | 0.854 | 0.280 | 0.066 | 0.551 | 0.851 |
| synthetic-s2 | 2820 | 97 | 4 | 0 | 2078 | 0.858 | 0.292 | 0.089 | 0.567 | 0.854 |
| synthetic-s3 | 2804 | 91 | 3 | 0 | 2029 | 0.843 | 0.271 | 0.074 | 0.526 | 0.854 |

## 3–5. Gecikme, dürüstlük, fayda

| Veri | İleti | Gecikme <0 / 0 / 1 / 2 / 3 / 4 / 5–9 / ≥10 | Ort. | En çok | trust=server | Metinde rakam | Kodda rakam | Ekte rakam | TR ifade | EN ifade | Olgu | Fayda |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | 9503 | 0 / 9503 / 0 / 0 / 0 / 0 / 0 / 0 | 0.00 | 0 | 1.000 | 0 | 0 | 0 | 1.000 | 1.000 | 8136 | 0.826 |
| synthetic-s1 | 3135 | 0 / 3135 / 0 / 0 / 0 / 0 / 0 / 0 | 0.00 | 0 | 1.000 | 0 | 0 | 0 | 1.000 | 1.000 | 2692 | 0.820 |
| synthetic-s2 | 3203 | 0 / 3203 / 0 / 0 / 0 / 0 / 0 / 0 | 0.00 | 0 | 1.000 | 0 | 0 | 0 | 1.000 | 1.000 | 2730 | 0.832 |
| synthetic-s3 | 3165 | 0 / 3165 / 0 / 0 / 0 / 0 / 0 / 0 | 0.00 | 0 | 1.000 | 0 | 0 | 0 | 1.000 | 1.000 | 2714 | 0.826 |
