# M4 — `reference` (synthetic)

Kurallar: tema `reference.kind_direction`, sonuç `reference.atr_barrier`, sıralayıcı `reference.families`.

## 1, 4. Azalma ve kaybolan bilgi

| Veri | Olgu | Hikâye | Azalma | Yok sayılan | Motorun düşürdüğü | Tür/yön farklı (tür) | Düşen aile (hikâye) |
|---|---|---|---|---|---|---|---|
| episodes-s1 | 6760 | 3988 | 0.410 | 990 | 0 | 0 (0) | 0 (0) |
| episodes-s2 | 7002 | 4080 | 0.417 | 959 | 0 | 0 (0) | 0 (0) |
| episodes-s3 | 6811 | 3951 | 0.420 | 972 | 0 | 0 (0) | 0 (0) |
| synthetic-s1 | 8405 | 5225 | 0.378 | 806 | 0 | 0 (0) | 0 (0) |
| synthetic-s2 | 8318 | 5167 | 0.379 | 798 | 0 | 0 (0) | 0 (0) |
| synthetic-s3 | 8259 | 5073 | 0.386 | 760 | 0 | 0 (0) | 0 (0) |

## 2–3. Etiketli bölüm kümesi: yanlış birleştirme ve bölünme

| Veri | Bölüm (olgulu) | Etiketli olgu payı | Aynı hikâye çifti | Yanlış birleştirme oranı | Karışık hikâye | Arka planla karışık | Aynı bölüm çifti | Bölünme oranı |
|---|---|---|---|---|---|---|---|---|
| episodes-s1 | 316 (316) | 0.528 | 1005 | 0.000 | 0 | 84 | 17142 | 0.941 |
| episodes-s2 | 325 (325) | 0.540 | 1085 | 0.000 | 0 | 70 | 19522 | 0.944 |
| episodes-s3 | 322 (322) | 0.521 | 1056 | 0.000 | 0 | 68 | 16783 | 0.937 |

Şablon başına bölünme (tüm etiketli kümeler toplamı):

| Şablon | Bölüm | Aynı bölüm çifti | Bölünen | Oran |
|---|---|---|---|---|
| breakout_volume_continuation | 122 | 4372 | 3974 | 0.909 |
| squeeze_breakout | 124 | 9976 | 9198 | 0.922 |
| pair_same | 220 | 5862 | 5144 | 0.878 |
| pair_opposite | 266 | 6644 | 5886 | 0.886 |
| extreme_enter_exit | 109 | 5255 | 5230 | 0.995 |
| divergence_breakdown | 122 | 21338 | 20869 | 0.978 |
