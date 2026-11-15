# M4 — `arc` (synthetic)

Kurallar: tema `arc`, sonuç `barrier`, sıralayıcı `reference.families`.

## 1, 4. Azalma ve kaybolan bilgi

| Veri | Olgu | Hikâye | Azalma | Yok sayılan | Motorun düşürdüğü | Tür/yön farklı (tür) | Düşen aile (hikâye) |
|---|---|---|---|---|---|---|---|
| episodes-s1 | 6760 | 2968 | 0.561 | 881 | 0 | 1509 (1508) | 0 (0) |
| episodes-s2 | 7002 | 3042 | 0.566 | 878 | 0 | 1565 (1562) | 0 (0) |
| episodes-s3 | 6811 | 2861 | 0.580 | 871 | 0 | 1588 (1584) | 0 (0) |
| synthetic-s1 | 8405 | 3660 | 0.565 | 736 | 0 | 2575 (2571) | 0 (0) |
| synthetic-s2 | 8318 | 3606 | 0.566 | 730 | 0 | 2546 (2543) | 0 (0) |
| synthetic-s3 | 8259 | 3573 | 0.567 | 701 | 0 | 2552 (2546) | 0 (0) |

## 2–3. Etiketli bölüm kümesi: yanlış birleştirme ve bölünme

| Veri | Bölüm (olgulu) | Etiketli olgu payı | Aynı hikâye çifti | Yanlış birleştirme oranı | Karışık hikâye | Arka planla karışık | Aynı bölüm çifti | Bölünme oranı |
|---|---|---|---|---|---|---|---|---|
| episodes-s1 | 316 (316) | 0.528 | 2764 | 0.000 | 0 | 89 | 17449 | 0.842 |
| episodes-s2 | 325 (325) | 0.540 | 3042 | 0.000 | 0 | 88 | 19740 | 0.846 |
| episodes-s3 | 322 (322) | 0.521 | 3108 | 0.000 | 0 | 84 | 17027 | 0.817 |

Şablon başına bölünme (tüm etiketli kümeler toplamı):

| Şablon | Bölüm | Aynı bölüm çifti | Bölünen | Oran |
|---|---|---|---|---|
| breakout_volume_continuation | 122 | 4377 | 3213 | 0.734 |
| squeeze_breakout | 124 | 10644 | 9087 | 0.854 |
| pair_same | 220 | 5765 | 3201 | 0.555 |
| pair_opposite | 266 | 6623 | 4025 | 0.608 |
| extreme_enter_exit | 109 | 5340 | 5098 | 0.955 |
| divergence_breakdown | 122 | 21467 | 20678 | 0.963 |
