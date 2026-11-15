# M3 — `confirm` (synthetic)

Kurallar: tema `reference.kind_direction`, sonuç `reference.atr_barrier`, sıralayıcı `confirm`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 1.16 | 1 | 3 | 9 | 0.327 | 0.98 | 2.56 | 0.195 | 4 | 3.18 |
| havuz | balanced | 4.51 | 4 | 8 | 18 | 0.024 | 0.56 | 2.29 | 0.549 | 5 | 11.34 |
| havuz | all | 10.69 | 10 | 16 | 28 | 0.001 | 0.36 | 1.73 | 0.834 | 6 | 26.38 |
| synthetic-s1 | important | 1.13 | 1 | 3 | 6 | 0.340 | 1.00 | 2.89 | 0.202 | 3 | 3.08 |
| synthetic-s1 | balanced | 4.63 | 4 | 8 | 18 | 0.029 | 0.58 | 2.51 | 0.571 | 5 | 11.58 |
| synthetic-s1 | all | 10.88 | 11 | 16 | 28 | 0.000 | 0.38 | 1.86 | 0.842 | 6 | 26.74 |
| synthetic-s2 | important | 1.27 | 1 | 3 | 6 | 0.284 | 0.92 | 2.52 | 0.198 | 4 | 3.33 |
| synthetic-s2 | balanced | 4.26 | 4 | 8 | 15 | 0.033 | 0.57 | 2.22 | 0.525 | 4 | 10.88 |
| synthetic-s2 | all | 10.68 | 10 | 16 | 27 | 0.002 | 0.36 | 1.69 | 0.828 | 6 | 26.27 |
| synthetic-s3 | important | 1.08 | 1 | 3 | 9 | 0.356 | 1.02 | 2.25 | 0.184 | 2 | 3.12 |
| synthetic-s3 | balanced | 4.65 | 4 | 8 | 15 | 0.011 | 0.53 | 2.11 | 0.549 | 4 | 11.56 |
| synthetic-s3 | all | 10.52 | 10 | 15 | 23 | 0.000 | 0.34 | 1.60 | 0.832 | 6 | 26.13 |

Rejim ortalamaları (enstrüman-gün başına doğum; yükselen / düşen / yatay / oynak), havuz:

| Seviye | yükselen | düşen | yatay | oynak | oran |
|---|---|---|---|---|---|
| important | 0.0395 | 0.0400 | 0.0278 | 0.0710 | 2.56 |
| balanced | 0.1721 | 0.1699 | 0.1004 | 0.2299 | 2.29 |
| all | 0.3932 | 0.3968 | 0.2730 | 0.4711 | 1.73 |

Tohumlar (veri kümeleri) arası fark, günlük ortalama:

| Seviye | en çok − en az | göreli |
|---|---|---|
| important | 0.191 | 0.165 |
| balanced | 0.396 | 0.088 |
| all | 0.353 | 0.033 |

## 3. Sonradan gerçekleşenlerin payı (doğumdaki kademe)

| Veri | K1 kapanan | K1 pay | K2 kapanan | K2 pay | K3 kapanan | K3 pay | Taban | K1 − K3 |
|---|---|---|---|---|---|---|---|---|
| havuz | 1699 | 0.543 | 4747 | 0.471 | 8895 | 0.458 | 0.471 | 0.084 |
| synthetic-s1 | 556 | 0.558 | 1646 | 0.482 | 2981 | 0.458 | 0.476 | 0.100 |
| synthetic-s2 | 621 | 0.531 | 1419 | 0.454 | 3091 | 0.468 | 0.472 | 0.063 |
| synthetic-s3 | 522 | 0.540 | 1682 | 0.474 | 2823 | 0.448 | 0.466 | 0.092 |

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
| synthetic-s3 | 1702882800000 | 5969 | 0 | 0 | 6555 |
| synthetic-s3 | 1718002800000 | 9224 | 0 | 0 | 3302 |

Hikâye sayısı: synthetic-s1 5225, synthetic-s2 5167, synthetic-s3 5073.
