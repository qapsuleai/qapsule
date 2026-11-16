# M4 — `control` (synthetic)

Kurallar: tema `control.singleton`, sonuç `reference.atr_barrier`, sıralayıcı `reference.families`.

## 1, 4. Azalma ve kaybolan bilgi

| Veri | Olgu | Hikâye | Azalma | Yok sayılan | Motorun düşürdüğü | Tür/yön farklı (tür) | Düşen aile (hikâye) |
|---|---|---|---|---|---|---|---|
| episodes-s1 | 6760 | 6760 | 0.000 | 0 | 0 | 0 (0) | 0 (0) |
| episodes-s2 | 7002 | 7002 | 0.000 | 0 | 0 | 0 (0) | 0 (0) |
| episodes-s3 | 6811 | 6811 | 0.000 | 0 | 0 | 0 (0) | 0 (0) |
| synthetic-s1 | 8405 | 8405 | 0.000 | 0 | 0 | 0 (0) | 0 (0) |
| synthetic-s2 | 8318 | 8318 | 0.000 | 0 | 0 | 0 (0) | 0 (0) |
| synthetic-s3 | 8259 | 8259 | 0.000 | 0 | 0 | 0 (0) | 0 (0) |

## 2–3. Etiketli bölüm kümesi: yanlış birleştirme ve bölünme

| Veri | Bölüm (olgulu) | Etiketli olgu payı | Aynı hikâye çifti | Yanlış birleştirme oranı | Karışık hikâye | Arka planla karışık | Aynı bölüm çifti | Bölünme oranı |
|---|---|---|---|---|---|---|---|---|
| episodes-s1 | 316 (316) | 0.528 | 0 | — | 0 | 0 | 22166 | 1.000 |
| episodes-s2 | 325 (325) | 0.540 | 0 | — | 0 | 0 | 24858 | 1.000 |
| episodes-s3 | 322 (322) | 0.521 | 0 | — | 0 | 0 | 21365 | 1.000 |

Şablon başına bölünme (tüm etiketli kümeler toplamı):

| Şablon | Bölüm | Aynı bölüm çifti | Bölünen | Oran |
|---|---|---|---|---|
| breakout_volume_continuation | 122 | 4890 | 4890 | 1.000 |
| squeeze_breakout | 124 | 13169 | 13169 | 1.000 |
| pair_same | 220 | 6555 | 6555 | 1.000 |
| pair_opposite | 266 | 7298 | 7298 | 1.000 |
| extreme_enter_exit | 109 | 7125 | 7125 | 1.000 |
| divergence_breakdown | 122 | 29352 | 29352 | 1.000 |
