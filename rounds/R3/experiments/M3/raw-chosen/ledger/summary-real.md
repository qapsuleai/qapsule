# M3 — `ledger` (real)

Kurallar: tema `kinddir`, sonuç `barrier`, sıralayıcı `m3.ledger`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.

## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)

| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |
|---|---|---|---|---|---|---|---|---|---|---|---|
| havuz | important | 0.01 | 0 | 0 | 3 | 0.989 | 10.06 | — | 0.000 | 1 | 0.03 |
| havuz | balanced | 2.71 | 1 | 8 | 27 | 0.321 | 1.45 | — | 0.879 | 38 | 7.15 |
| havuz | all | 2.71 | 1 | 8 | 27 | 0.321 | 1.45 | — | 0.879 | 38 | 7.15 |
| real-1day | important | 0.01 | 0 | 0 | 2 | 0.988 | 9.26 | — | 0.000 | 1 | 0.04 |
| real-1day | balanced | 1.22 | 1 | 3 | 9 | 0.393 | 1.17 | — | 0.673 | 7 | 3.18 |
| real-1day | all | 1.22 | 1 | 3 | 9 | 0.393 | 1.17 | — | 0.673 | 7 | 3.18 |
| real-1hour | important | 0.01 | 0 | 0 | 3 | 0.994 | 14.72 | — | 0.000 | 1 | 0.02 |
| real-1hour | balanced | 9.22 | 9 | 15 | 27 | 0.002 | 0.51 | — | 0.999 | 38 | 24.51 |
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
| havuz | 35 | 0.514 | 7232 | 0.496 | 0 | — | 0.496 | — |
| real-1day | 30 | 0.567 | 2641 | 0.523 | 0 | — | 0.524 | — |
| real-1hour | 5 | 0.200 | 4591 | 0.481 | 0 | — | 0.481 | — |

## 4. İleriye bakma

Sonuç: **geçti**.

| Veri | Kesim t | Karşılaştırılan | Farklı | Kademe farklı | Gelecekte değişen |
|---|---|---|---|---|---|
| real-1day | 1584514800000 | 1589 | 0 | 0 | 5362 |
| real-1day | 1654671600000 | 3432 | 0 | 0 | 3578 |
| real-1day | 1724914800000 | 5320 | 0 | 0 | 1788 |
| real-1hour | 1747303200000 | 2984 | 0 | 0 | 9545 |
| real-1hour | 1763107200000 | 6100 | 0 | 0 | 6603 |
| real-1hour | 1778749200000 | 9223 | 0 | 0 | 3281 |

Hikâye sayısı: real-1day 2678, real-1hour 4605.
