# M3 — `rarity` (synthetic)

Kurallar: tema `reference.kind_direction`, sonuç `reference.atr_barrier`, sıralayıcı `rarity`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 1.02 | 1 | 2 | 5 | 0.381 | 1.03 | 1.42 | 0.185 | 2 | 2.61 |
| havuz | balanced | 4.02 | 4 | 7 | 15 | 0.030 | 0.55 | 2.05 | 0.526 | 5 | 10.51 |
| havuz | all | 10.69 | 10 | 16 | 28 | 0.001 | 0.36 | 1.73 | 0.834 | 6 | 26.38 |
| synthetic-s1 | important | 1.05 | 1 | 2 | 5 | 0.362 | 0.99 | 1.65 | 0.186 | 2 | 2.65 |
| synthetic-s1 | balanced | 4.11 | 4 | 7 | 15 | 0.036 | 0.58 | 2.30 | 0.535 | 4 | 10.69 |
| synthetic-s1 | all | 10.88 | 11 | 16 | 28 | 0.000 | 0.38 | 1.86 | 0.842 | 6 | 26.74 |
| synthetic-s2 | important | 1.03 | 1 | 2 | 5 | 0.380 | 1.02 | 1.50 | 0.194 | 2 | 2.63 |
| synthetic-s2 | balanced | 3.81 | 4 | 7 | 11 | 0.040 | 0.57 | 2.08 | 0.513 | 4 | 9.96 |
| synthetic-s2 | all | 10.68 | 10 | 16 | 27 | 0.002 | 0.36 | 1.69 | 0.828 | 6 | 26.27 |
| synthetic-s3 | important | 0.99 | 1 | 2 | 5 | 0.402 | 1.06 | 1.11 | 0.175 | 2 | 2.56 |
| synthetic-s3 | balanced | 4.14 | 4 | 7 | 13 | 0.016 | 0.50 | 1.76 | 0.530 | 5 | 10.86 |
| synthetic-s3 | all | 10.52 | 10 | 15 | 23 | 0.000 | 0.34 | 1.60 | 0.832 | 6 | 26.13 |

Rejim ortalamaları (enstrüman-gün başına doğum; yükselen / düşen / yatay / oynak), havuz:

| Seviye | yükselen | düşen | yatay | oynak | oran |
|---|---|---|---|---|---|
| important | 0.0357 | 0.0301 | 0.0330 | 0.0427 | 1.42 |
| balanced | 0.1502 | 0.1481 | 0.0961 | 0.1971 | 2.05 |
| all | 0.3932 | 0.3968 | 0.2730 | 0.4711 | 1.73 |

Tohumlar (veri kümeleri) arası fark, günlük ortalama:

| Seviye | en çok − en az | göreli |
|---|---|---|
| important | 0.062 | 0.061 |
| balanced | 0.327 | 0.081 |
| all | 0.353 | 0.033 |

## 3. Sonradan gerçekleşenlerin payı (doğumdaki kademe)

| Veri | K1 kapanan | K1 pay | K2 kapanan | K2 pay | K3 kapanan | K3 pay | Taban | K1 − K3 |
|---|---|---|---|---|---|---|---|---|
| havuz | 1598 | 0.540 | 4448 | 0.478 | 9295 | 0.457 | 0.471 | 0.083 |
| synthetic-s1 | 537 | 0.547 | 1534 | 0.472 | 3112 | 0.466 | 0.476 | 0.082 |
| synthetic-s2 | 530 | 0.543 | 1389 | 0.475 | 3212 | 0.459 | 0.472 | 0.085 |
| synthetic-s3 | 531 | 0.529 | 1525 | 0.485 | 2971 | 0.445 | 0.466 | 0.084 |

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
