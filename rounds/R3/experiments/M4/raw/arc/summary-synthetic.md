# M4 — `arc` (synthetic)

Kurallar: tema `arc`, sonuç `reference.atr_barrier`, sıralayıcı `reference.families`.

## 1, 4. Azalma ve kaybolan bilgi

| Veri | Olgu | Hikâye | Azalma | Yok sayılan | Motorun düşürdüğü | Tür/yön farklı (tür) | Düşen aile (hikâye) |
|---|---|---|---|---|---|---|---|
| episodes-s1 | 6760 | 2962 | 0.562 | 1003 | 0 | 1428 (1427) | 0 (0) |
| episodes-s2 | 7002 | 3022 | 0.568 | 987 | 0 | 1488 (1485) | 0 (0) |
| episodes-s3 | 6811 | 2857 | 0.581 | 986 | 0 | 1509 (1508) | 0 (0) |
| synthetic-s1 | 8405 | 3626 | 0.569 | 799 | 0 | 2554 (2551) | 0 (0) |
| synthetic-s2 | 8318 | 3589 | 0.569 | 810 | 0 | 2515 (2513) | 0 (0) |
| synthetic-s3 | 8259 | 3558 | 0.569 | 775 | 0 | 2501 (2495) | 0 (0) |

## 2–3. Etiketli bölüm kümesi: yanlış birleştirme ve bölünme

| Veri | Bölüm (olgulu) | Etiketli olgu payı | Aynı hikâye çifti | Yanlış birleştirme oranı | Karışık hikâye | Arka planla karışık | Aynı bölüm çifti | Bölünme oranı |
|---|---|---|---|---|---|---|---|---|
| episodes-s1 | 316 (316) | 0.528 | 2834 | 0.000 | 0 | 70 | 17057 | 0.834 |
| episodes-s2 | 325 (325) | 0.540 | 3090 | 0.000 | 0 | 66 | 19401 | 0.841 |
| episodes-s3 | 322 (322) | 0.521 | 3158 | 0.000 | 0 | 74 | 16698 | 0.811 |

Şablon başına bölünme (tüm etiketli kümeler toplamı):

| Şablon | Bölüm | Aynı bölüm çifti | Bölünen | Oran |
|---|---|---|---|---|
| breakout_volume_continuation | 122 | 4281 | 3084 | 0.720 |
| squeeze_breakout | 124 | 9971 | 8575 | 0.860 |
| pair_same | 220 | 5715 | 3068 | 0.537 |
| pair_opposite | 266 | 6616 | 3914 | 0.592 |
| extreme_enter_exit | 109 | 5255 | 5003 | 0.952 |
| divergence_breakdown | 122 | 21318 | 20430 | 0.958 |
