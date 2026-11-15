# M3 — `confirm` (real)

Kurallar: tema `kinddir`, sonuç `barrier`, sıralayıcı `confirm`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 0.30 | 0 | 1 | 10 | 0.822 | 2.72 | — | 0.523 | 5 | 0.82 |
| havuz | balanced | 0.91 | 0 | 3 | 12 | 0.605 | 1.79 | — | 0.729 | 8 | 2.18 |
| havuz | all | 2.71 | 1 | 8 | 27 | 0.321 | 1.45 | — | 0.879 | 38 | 7.15 |
| real-1day | important | 0.09 | 0 | 0 | 3 | 0.919 | 3.58 | — | 0.126 | 3 | 0.23 |
| real-1day | balanced | 0.38 | 0 | 1 | 5 | 0.723 | 1.91 | — | 0.295 | 4 | 0.88 |
| real-1day | all | 1.22 | 1 | 3 | 9 | 0.393 | 1.17 | — | 0.673 | 7 | 3.18 |
| real-1hour | important | 1.19 | 1 | 3 | 10 | 0.398 | 1.19 | — | 0.656 | 5 | 3.40 |
| real-1hour | balanced | 3.21 | 3 | 6 | 12 | 0.089 | 0.73 | — | 0.957 | 8 | 7.90 |
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
| havuz | 789 | 0.517 | 1640 | 0.479 | 4838 | 0.499 | 0.496 | 0.018 |
| real-1day | 200 | 0.610 | 636 | 0.503 | 1835 | 0.522 | 0.524 | 0.088 |
| real-1hour | 589 | 0.486 | 1004 | 0.464 | 3003 | 0.485 | 0.481 | 0.000 |

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
