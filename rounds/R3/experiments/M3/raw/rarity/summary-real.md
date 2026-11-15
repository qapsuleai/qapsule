# M3 — `rarity` (real)

Kurallar: tema `reference.kind_direction`, sonuç `reference.atr_barrier`, sıralayıcı `rarity`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 0.32 | 0 | 1 | 7 | 0.780 | 2.26 | — | 0.399 | 3 | 0.71 |
| havuz | balanced | 2.01 | 1 | 6 | 30 | 0.434 | 1.68 | — | 0.840 | 14 | 5.04 |
| havuz | all | 2.98 | 1 | 9 | 33 | 0.315 | 1.50 | — | 0.890 | 41 | 7.31 |
| real-1day | important | 0.17 | 0 | 1 | 4 | 0.856 | 2.67 | — | 0.159 | 3 | 0.40 |
| real-1day | balanced | 0.80 | 0 | 2 | 9 | 0.531 | 1.43 | — | 0.511 | 4 | 2.02 |
| real-1day | all | 1.30 | 1 | 3 | 9 | 0.386 | 1.17 | — | 0.691 | 7 | 3.21 |
| real-1hour | important | 0.97 | 1 | 3 | 7 | 0.448 | 1.21 | — | 0.583 | 3 | 2.07 |
| real-1hour | balanced | 7.33 | 7 | 14 | 30 | 0.008 | 0.62 | — | 0.997 | 14 | 18.30 |
| real-1hour | all | 10.37 | 10 | 18 | 33 | 0.002 | 0.54 | — | 0.999 | 41 | 25.29 |

Rejim ortalamaları (enstrüman-gün başına doğum; yükselen / düşen / yatay / oynak), havuz:

| Seviye | yükselen | düşen | yatay | oynak | oran |
|---|---|---|---|---|---|
| important | — | — | — | — | — |
| balanced | — | — | — | — | — |
| all | — | — | — | — | — |

Tohumlar (veri kümeleri) arası fark, günlük ortalama:

| Seviye | en çok − en az | göreli |
|---|---|---|
| important | — | — |
| balanced | — | — |
| all | — | — |

## 3. Sonradan gerçekleşenlerin payı (doğumdaki kademe)

| Veri | K1 kapanan | K1 pay | K2 kapanan | K2 pay | K3 kapanan | K3 pay | Taban | K1 − K3 |
|---|---|---|---|---|---|---|---|---|
| havuz | 868 | 0.647 | 4551 | 0.485 | 2580 | 0.514 | 0.512 | 0.134 |
| real-1day | 379 | 0.636 | 1373 | 0.516 | 1077 | 0.552 | 0.546 | 0.083 |
| real-1hour | 489 | 0.656 | 3178 | 0.471 | 1503 | 0.486 | 0.493 | 0.170 |

## 4. İleriye bakma

Sonuç: **geçti**.

| Veri | Kesim t | Karşılaştırılan | Farklı | Kademe farklı | Gelecekte değişen |
|---|---|---|---|---|---|
| real-1day | 1584514800000 | 1580 | 0 | 0 | 5435 |
| real-1day | 1654671600000 | 3463 | 0 | 0 | 3552 |
| real-1day | 1724914800000 | 5370 | 0 | 0 | 1740 |
| real-1hour | 1747303200000 | 3072 | 0 | 0 | 9539 |
| real-1hour | 1763107200000 | 6317 | 0 | 0 | 6493 |
| real-1hour | 1778749200000 | 9520 | 0 | 0 | 3219 |

Hikâye sayısı: real-1day 2834, real-1hour 5174.
