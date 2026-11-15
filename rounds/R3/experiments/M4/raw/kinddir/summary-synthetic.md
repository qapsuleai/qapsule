# M4 — `kinddir` (synthetic)

Kurallar: tema `kinddir`, sonuç `reference.atr_barrier`, sıralayıcı `reference.families`.

## 1, 4. Azalma ve kaybolan bilgi

| Veri | Olgu | Hikâye | Azalma | Yok sayılan | Motorun düşürdüğü | Tür/yön farklı (tür) | Düşen aile (hikâye) |
|---|---|---|---|---|---|---|---|
| episodes-s1 | 6760 | 3749 | 0.445 | 990 | 0 | 243 (243) | 12 (12) |
| episodes-s2 | 7002 | 3815 | 0.455 | 959 | 0 | 265 (265) | 21 (21) |
| episodes-s3 | 6811 | 3687 | 0.459 | 972 | 0 | 266 (266) | 12 (12) |
| synthetic-s1 | 8405 | 4338 | 0.484 | 806 | 0 | 1370 (1370) | 176 (167) |
| synthetic-s2 | 8318 | 4257 | 0.488 | 798 | 0 | 1341 (1341) | 199 (192) |
| synthetic-s3 | 8259 | 4185 | 0.493 | 759 | 0 | 1382 (1382) | 187 (180) |

## 2–3. Etiketli bölüm kümesi: yanlış birleştirme ve bölünme

| Veri | Bölüm (olgulu) | Etiketli olgu payı | Aynı hikâye çifti | Yanlış birleştirme oranı | Karışık hikâye | Arka planla karışık | Aynı bölüm çifti | Bölünme oranı |
|---|---|---|---|---|---|---|---|---|
| episodes-s1 | 316 (316) | 0.528 | 1486 | 0.000 | 0 | 84 | 17142 | 0.913 |
| episodes-s2 | 325 (325) | 0.540 | 1633 | 0.000 | 0 | 70 | 19522 | 0.916 |
| episodes-s3 | 322 (322) | 0.521 | 1604 | 0.000 | 0 | 68 | 16783 | 0.904 |

Şablon başına bölünme (tüm etiketli kümeler toplamı):

| Şablon | Bölüm | Aynı bölüm çifti | Bölünen | Oran |
|---|---|---|---|---|
| breakout_volume_continuation | 122 | 4372 | 3722 | 0.851 |
| squeeze_breakout | 124 | 9976 | 9013 | 0.903 |
| pair_same | 220 | 5862 | 4642 | 0.792 |
| pair_opposite | 266 | 6644 | 5338 | 0.803 |
| extreme_enter_exit | 109 | 5255 | 5230 | 0.995 |
| divergence_breakdown | 122 | 21338 | 20779 | 0.974 |
