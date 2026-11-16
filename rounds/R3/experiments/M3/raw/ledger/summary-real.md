# M3 — `ledger` (real)

Kurallar: tema `reference.kind_direction`, sonuç `reference.atr_barrier`, sıralayıcı `m3.ledger`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 0.09 | 0 | 0 | 4 | 0.930 | 4.03 | — | 0.090 | 1 | 0.18 |
| havuz | balanced | 2.45 | 1 | 7 | 31 | 0.333 | 1.47 | — | 0.867 | 20 | 6.14 |
| havuz | all | 2.98 | 1 | 9 | 33 | 0.315 | 1.50 | — | 0.890 | 41 | 7.31 |
| real-1day | important | 0.04 | 0 | 0 | 3 | 0.968 | 5.75 | — | 0.000 | 1 | 0.07 |
| real-1day | balanced | 1.21 | 1 | 3 | 9 | 0.405 | 1.20 | — | 0.672 | 7 | 3.00 |
| real-1day | all | 1.30 | 1 | 3 | 9 | 0.386 | 1.17 | — | 0.691 | 7 | 3.21 |
| real-1hour | important | 0.32 | 0 | 1 | 4 | 0.766 | 2.06 | — | 0.135 | 1 | 0.64 |
| real-1hour | balanced | 7.89 | 7 | 15 | 31 | 0.016 | 0.62 | — | 0.998 | 20 | 19.88 |
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
| havuz | 233 | 0.991 | 6353 | 0.507 | 1413 | 0.453 | 0.512 | 0.538 |
| real-1day | 77 | 1.000 | 2565 | 0.536 | 187 | 0.503 | 0.546 | 0.497 |
| real-1hour | 156 | 0.987 | 3788 | 0.488 | 1226 | 0.445 | 0.493 | 0.542 |

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
