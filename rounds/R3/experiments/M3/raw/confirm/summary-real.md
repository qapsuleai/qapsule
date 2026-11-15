# M3 — `confirm` (real)

Kurallar: tema `reference.kind_direction`, sonuç `reference.atr_barrier`, sıralayıcı `confirm`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 0.11 | 0 | 0 | 4 | 0.916 | 3.69 | — | 0.142 | 3 | 0.21 |
| havuz | balanced | 0.84 | 0 | 3 | 12 | 0.625 | 1.83 | — | 0.707 | 8 | 1.75 |
| havuz | all | 2.98 | 1 | 9 | 33 | 0.315 | 1.50 | — | 0.890 | 41 | 7.31 |
| real-1day | important | 0.04 | 0 | 0 | 3 | 0.960 | 5.14 | — | 0.000 | 1 | 0.09 |
| real-1day | balanced | 0.36 | 0 | 1 | 5 | 0.741 | 1.97 | — | 0.271 | 4 | 0.75 |
| real-1day | all | 1.30 | 1 | 3 | 9 | 0.386 | 1.17 | — | 0.691 | 7 | 3.21 |
| real-1hour | important | 0.39 | 0 | 1 | 4 | 0.723 | 1.88 | — | 0.215 | 3 | 0.75 |
| real-1hour | balanced | 2.98 | 3 | 6 | 12 | 0.113 | 0.76 | — | 0.935 | 8 | 6.14 |
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
| havuz | 290 | 0.962 | 1961 | 0.469 | 5748 | 0.504 | 0.512 | 0.458 |
| real-1day | 98 | 0.990 | 674 | 0.493 | 2057 | 0.543 | 0.546 | 0.447 |
| real-1hour | 192 | 0.948 | 1287 | 0.456 | 3691 | 0.482 | 0.493 | 0.466 |

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
