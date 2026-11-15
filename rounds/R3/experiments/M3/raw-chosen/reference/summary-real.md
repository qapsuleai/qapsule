# M3 — `reference` (real)

Kurallar: tema `kinddir`, sonuç `barrier`, sıralayıcı `reference.families`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 0.20 | 0 | 1 | 8 | 0.880 | 3.42 | — | 0.473 | 5 | 0.73 |
| havuz | balanced | 1.12 | 0 | 3 | 14 | 0.541 | 1.66 | — | 0.743 | 9 | 3.32 |
| havuz | all | 2.71 | 1 | 8 | 27 | 0.321 | 1.45 | — | 0.879 | 38 | 7.15 |
| real-1day | important | 0.05 | 0 | 0 | 2 | 0.956 | 4.80 | — | 0.173 | 3 | 0.19 |
| real-1day | balanced | 0.51 | 0 | 2 | 6 | 0.650 | 1.68 | — | 0.355 | 4 | 1.50 |
| real-1day | all | 1.22 | 1 | 3 | 9 | 0.393 | 1.17 | — | 0.673 | 7 | 3.18 |
| real-1hour | important | 0.84 | 0 | 2 | 8 | 0.549 | 1.53 | — | 0.548 | 5 | 3.09 |
| real-1hour | balanced | 3.76 | 3 | 7 | 14 | 0.061 | 0.68 | — | 0.975 | 9 | 11.27 |
| real-1hour | all | 9.22 | 9 | 15 | 27 | 0.002 | 0.51 | — | 0.999 | 38 | 24.51 |

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
| havuz | 526 | 0.477 | 2463 | 0.497 | 4278 | 0.499 | 0.496 | -0.022 |
| real-1day | 105 | 0.562 | 1013 | 0.512 | 1553 | 0.529 | 0.524 | 0.033 |
| real-1hour | 421 | 0.456 | 1450 | 0.486 | 2725 | 0.482 | 0.481 | -0.026 |

## 4. İleriye bakma

Sonuç: **geçti**.

| Veri | Kesim t | Karşılaştırılan | Farklı | Kademe farklı | Gelecekte değişen |
|---|---|---|---|---|---|
| real-1day | 1584514800000 | 1589 | 0 | 0 | 5362 |
| real-1day | 1654671600000 | 3432 | 0 | 0 | 3578 |
| real-1day | 1724914800000 | 5320 | 0 | 0 | 1789 |
| real-1hour | 1747303200000 | 2984 | 0 | 0 | 9545 |
| real-1hour | 1763107200000 | 6100 | 0 | 0 | 6603 |
| real-1hour | 1778749200000 | 9223 | 0 | 0 | 3281 |

Hikâye sayısı: real-1day 2678, real-1hour 4605.
