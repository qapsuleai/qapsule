# M4 — `kinddir` (synthetic)

Kurallar: tema `kinddir`, sonuç `barrier`, sıralayıcı `reference.families`.

## 1, 4. Azalma ve kaybolan bilgi

| Veri | Olgu | Hikâye | Azalma | Yok sayılan | Motorun düşürdüğü | Tür/yön farklı (tür) | Düşen aile (hikâye) |
|---|---|---|---|---|---|---|---|
| episodes-s1 | 6760 | 3760 | 0.444 | 847 | 0 | 241 (241) | 10 (10) |
| episodes-s2 | 7002 | 3832 | 0.453 | 844 | 0 | 264 (264) | 20 (20) |
| episodes-s3 | 6811 | 3686 | 0.459 | 833 | 0 | 264 (264) | 10 (10) |
| synthetic-s1 | 8405 | 4283 | 0.490 | 694 | 0 | 1422 (1422) | 198 (188) |
| synthetic-s2 | 8318 | 4203 | 0.495 | 682 | 0 | 1393 (1393) | 220 (212) |
| synthetic-s3 | 8259 | 4133 | 0.500 | 649 | 0 | 1429 (1429) | 211 (203) |

## 2–3. Etiketli bölüm kümesi: yanlış birleştirme ve bölünme

| Veri | Bölüm (olgulu) | Etiketli olgu payı | Aynı hikâye çifti | Yanlış birleştirme oranı | Karışık hikâye | Arka planla karışık | Aynı bölüm çifti | Bölünme oranı |
|---|---|---|---|---|---|---|---|---|
| episodes-s1 | 316 (316) | 0.528 | 1405 | 0.000 | 0 | 110 | 17530 | 0.920 |
| episodes-s2 | 325 (325) | 0.540 | 1592 | 0.000 | 0 | 94 | 19767 | 0.919 |
| episodes-s3 | 322 (322) | 0.521 | 1572 | 0.000 | 0 | 92 | 17254 | 0.909 |

Şablon başına bölünme (tüm etiketli kümeler toplamı):

| Şablon | Bölüm | Aynı bölüm çifti | Bölünen | Oran |
|---|---|---|---|---|
| breakout_volume_continuation | 122 | 4496 | 3879 | 0.863 |
| squeeze_breakout | 124 | 10710 | 9674 | 0.903 |
| pair_same | 220 | 5888 | 4718 | 0.801 |
| pair_opposite | 266 | 6675 | 5416 | 0.811 |
| extreme_enter_exit | 109 | 5296 | 5282 | 0.997 |
| divergence_breakdown | 122 | 21486 | 21013 | 0.978 |
