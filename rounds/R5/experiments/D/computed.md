### Metin karşıtlığı, öğe başına (en kötü; yedi görünüm × dört boy)

| öğe | `oltan` light | `oltan` dark | `r4` light | `r4` dark |
|---|---|---|---|---|
| eksen değeri (y) | 14,35 / 4,5 (13 px) | 9,77 / 4,5 (13 px) | 5,78 / 4,5 (11 px) | 5,27 / 4,5 (11 px) |
| zaman etiketi (x) | 14,35 / 4,5 (14 px) | 9,77 / 4,5 (14 px) | 5,78 / 4,5 (11 px) | 5,27 / 4,5 (11 px) |
| son değer etiketi | 5,69 / 4,5 (13 px) | 5,67 / 4,5 (13 px) | 3,08 / 4,5 (11 px) **✗** | 5,27 / 4,5 (11 px) |
| başlık + kapanış | 5,69 / 4,5 (15 px) | 5,67 / 4,5 (15 px) | 3,57 / 4,5 (16 px) **✗** | 5,37 / 4,5 (16 px) |
| açıklama (lejant) | 3,94 / 4,5 (14 px) **✗** | 6,66 / 4,5 (14 px) | 2,74 / 4,5 (11 px) **✗** | 3,89 / 4,5 (11 px) **✗** |

### Metin dışı karşıtlık, öğe başına (en kötü; WCAG 1.4.11 ≥ 3)

| öğe | `oltan` light | `oltan` dark | `r4` light | `r4` dark |
|---|---|---|---|---|
| ızgara | 1,23 | 1,23 | 1,20 | 1,24 |
| eksen çizgisi | 2,88 **✗** | 2,95 **✗** | 2,25 **✗** | 2,71 **✗** |
| mum: yukarı | 5,69 | 10,97 | 3,57 | 6,24 |
| mum: aşağı | 10,22 | 5,67 | 4,23 | 5,37 |
| seri çizgisi | 3,94 | 5,67 | 2,74 **✗** | 3,89 |
| histogram | 5,69 | 5,67 | 3,57 | 5,37 |
| eşik kuralı (70/30) | 14,35 | 9,77 | 5,78 | 5,27 |
| bant/alan dolgusu (saydam) | 1,26 | 1,29 | 1,16 | 1,30 |
| son değer kutusu | 5,69 | 5,67 | 3,08 | 5,27 |

Izgara: en sık değer (kesişen ızgara çizgisinin üstündeki ölçüm 1,00 okur). Izgara ve saydam dolgu WCAG 1.4.11'in kapsamı dışında sayıldı (süs/arka plan); bilgi için yazıldı.

### En küçük yazı, metin yoğunluğu, açıklama kaybı (boy başına; yedi görünüm, iki tema aynı)

| boy | look | en küçük yazı | metin öğesi (ort.) | karakter / 10 000 px² (ort.) | metin mürekkep alanı (ort.) | çakışma | taşma | 2 px'ten yakın | açıklama: gösterilen / yazılan |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| phone | `oltan` | 13 px | 12,4 | 3,61 | 2,7 % | 0 | 0 | 0 | 10 / 18 |
| phone | `r4` | 11 px | 12,4 | 3,61 | 2,5 % | 0 | 0 | 0 | 10 / 18 |
| desk | `oltan` | 13 px | 20,0 | 1,58 | 1,2 % | 0 | 0 | 0 | 18 / 18 |
| desk | `r4` | 11 px | 20,0 | 1,58 | 1,0 % | 0 | 0 | 0 | 18 / 18 |
| narrow | `oltan` | 13 px | 11,1 | 4,83 | 3,6 % | 0 | 0 | 0 | 5 / 18 |
| narrow | `r4` | 11 px | 11,1 | 4,83 | 3,4 % | 0 | 0 | 0 | 5 / 18 |
| quad | `oltan` | 13 px | 13,0 | 4,16 | 3,2 % | 0 | 0 | 0 | 13 / 18 |
| quad | `r4` | 11 px | 13,1 | 4,23 | 2,8 % | 0 | 0 | 0 | 14 / 18 |

Açıklama sayısı: v0 1, v1 1, v2 2, v3 3, v4 0, v5 2, s8 9 (yedi görünümün toplamı 18). Gramer başlık satırına sığmayan açıklamayı soldakilerle birlikte düşürür (`compile.rs`, "the right-most are kept").

Açıklama kaybı, görünüm başına (gösterilen/yazılan; `oltan` | `r4`):

| görünüm | phone | desk | narrow | quad |
|---|---|---|---|---|
| v0 | 1·1 / 1 | 1·1 / 1 | 0·0 / 1 | 1·1 / 1 |
| v1 | 1·1 / 1 | 1·1 / 1 | 1·1 / 1 | 1·1 / 1 |
| v2 | 1·1 / 2 | 2·2 / 2 | 0·0 / 2 | 2·2 / 2 |
| v3 | 3·3 / 3 | 3·3 / 3 | 2·2 / 3 | 3·3 / 3 |
| v4 | 0·0 / 0 | 0·0 / 0 | 0·0 / 0 | 0·0 / 0 |
| v5 | 2·2 / 2 | 2·2 / 2 | 1·1 / 2 | 2·2 / 2 |
| s8 | 2·2 / 9 | 9·9 / 9 | 1·1 / 9 | 4·5 / 9 |

### Seri paleti s1–s8: ΔE00 ayrımı (28 çift; normal, gri, protan, deutan, tritan)

| look / tema | s1–s8 karşıtlık < 3 | en küçük ΔE00: normal / gri / prot / deut / trit | ΔE00 < 10 çift sayısı: normal / gri / prot / deut / trit | ilk k seride en kötü koşulda ΔE00 ≥ 10 kalan en büyük k |
|---|---|---|---|---|
| `oltan` light | yok | 13,9 / 0,2 / 0,4 / 4,4 / 5,8 | 0 / 22 / 2 / 3 / 5 | 2 |
| `oltan` dark | yok | 17,7 / 0,4 / 4,8 / 0,6 / 6,0 | 0 / 23 / 3 / 4 / 4 | 1 |
| `r4` light | s6 2,74 | 19,3 / 1,8 / 7,5 / 8,2 / 5,0 | 0 / 12 / 1 / 1 / 5 | 1 |
| `r4` dark | yok | 20,1 / 1,1 / 6,7 / 9,2 / 6,0 | 0 / 13 / 1 / 1 / 4 | 3 |

En kötü çiftler (en kötü koşulda ΔE00 < 10; `oltan`):

- light: s4-s6 0,2 (gray); s2-s6 0,4 (protan); s1-s8 0,5 (gray); s1-s3 0,7 (gray); s3-s8 1,2 (gray); s3-s7 1,8 (gray); s1-s7 2,5 (gray); s7-s8 2,9 (gray); s5-s6 3,7 (gray); s4-s5 3,9 (gray); s5-s8 4,0 (gray); s1-s5 4,5 (gray); s2-s4 4,6 (gray); s3-s5 5,2 (gray); s1-s4 6,7 (tritan); s5-s7 6,9 (gray); s6-s8 7,7 (gray); s4-s8 7,9 (gray); s1-s6 8,2 (gray); s3-s6 8,4 (tritan); s2-s5 8,5 (gray); s3-s4 9,1 (gray)
- dark: s1-s5 0,4 (gray); s1-s3 0,6 (deutan); s4-s8 0,7 (gray); s2-s4 1,4 (gray); s3-s5 1,7 (gray); s2-s8 2,0 (gray); s6-s8 2,2 (gray); s4-s6 2,8 (gray); s3-s7 3,2 (gray); s2-s6 4,2 (gray); s5-s7 4,9 (gray); s1-s7 5,2 (gray); s1-s2 5,4 (gray); s2-s5 5,7 (gray); s1-s4 6,7 (gray); s4-s5 7,1 (gray); s1-s8 7,4 (gray); s2-s3 7,4 (gray); s5-s8 7,8 (gray); s3-s4 8,8 (gray); s3-s8 9,4 (gray); s1-s6 9,6 (gray); s5-s6 9,9 (gray)

### Anlam renkleri ve hiyerarşi (görünüş kaydından)

| look / tema | up–down ΔE00 (normal/gri/prot/deut/trit) | metin/zemin | muted/zemin | metin–muted ΔE00 | eksen/zemin | ızgara/zemin |
|---|---|---:|---:|---:|---:|---:|
| `oltan` light | 60,4 / 13,2 / 20,5 / 11,7 / 50,0 | 18,72 | 14,35 | 7,8 | 2,88 | 1,23 |
| `oltan` dark | 71,8 / 16,5 / 26,3 / 14,7 / 63,8 | 17,13 | 9,77 | 14,7 | 2,95 | 1,23 |
| `r4` light | 63,0 / 4,7 / 20,5 / 23,6 / 61,3 | 15,80 | 5,78 | 22,6 | 2,25 | 1,20 |
| `r4` dark | 59,5 / 4,0 / 19,8 / 28,8 / 61,0 | 11,74 | 5,27 | 20,1 | 2,71 | 1,24 |

### Veri işaretinin boyu: mum gövdesi ve fitili (v1, px)

| boy | `oltan`: gövde / fitil | `r4`: gövde / fitil |
|---|---|---|
| phone | 1 / 1 | 1 / 1 |
| desk | 3 / 1 | 3 / 1 |
| narrow | 1 / 1 | 1 / 1 |
| quad | 1 / 1 | 1 / 1 |

### Yelpaze (v3) iç içe üç ton: komşu tonlar arası ΔE00 (normal / gri / prot / deut / trit)

| look / tema | zemin → dış | dış → orta | orta → iç |
|---|---|---|---|
| `oltan` light | 10,4 / 5,3 / 10,1 / 10,5 / 7,2 | 10,8 / 7,4 / 10,4 / 11,1 / 9,1 | 10,7 / 9,1 / 10,3 / 11,2 / 10,3 |
| `oltan` dark | 9,6 / 7,0 / 9,7 / 9,6 / 8,3 | 10,7 / 9,3 / 10,9 / 10,5 / 10,1 | 11,6 / 10,8 / 11,8 / 11,3 / 11,3 |
| `r4` light | 9,4 / 3,5 / 8,3 / 8,8 / 10,1 | 9,4 / 4,6 / 8,8 / 8,7 / 9,3 | 8,6 / 5,2 / 8,5 / 7,8 / 7,9 |
| `r4` dark | 12,7 / 7,3 / 12,4 / 13,3 / 11,4 | 13,0 / 10,0 / 12,3 / 13,2 / 12,0 | 13,3 / 12,0 / 12,4 / 13,7 / 12,8 |
