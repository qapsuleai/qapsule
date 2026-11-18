### Deterministik kapı (§G7; açık sahnenin boyama listesi + DOM; kapsül ve kanıt derinliği)

| tasarım | metin karşıtlığı en kötü / eşik (öğe) | metin dışı en kötü (rol) | en küçük yazı px | dokunma < 44 (DOM) | renk: geçemeyen çift (açık/koyu) | yetkisizde rakam (Ek G7e) | sahne metni ort. karakter | ekran karakteri ort. |
|---|---|---|---|---:|---|---|---:|---:|
| `tesir` | 1.65 / 4.5 (`Bu kanıt şu yetkiyi gerektirir:`, 14 px) | 2.32 (up, line) | 11 | 72 | 3/4 | 0 (8 görüntü) | 144.0 | 484.5 |
| `nobra` | 1.65 / 4.5 (`This evidence requires the entitle`, 14 px) | 2.32 (up, line) | 11 | 72 | 3/4 | 0 (8 görüntü) | 87.1 | 618.2 |
| `r4` | 1.65 / 4.5 (`This evidence requires the entitle`, 14 px) | 2.32 (up, line) | 11 | 72 | 5/6 | 0 (8 görüntü) | 80.6 | 317.6 |
| `kestim` | 1.2 / 4.5 (`This evidence requires the entitle`, 14 px) | 1.21 (focus/neutral, rect) | 11 | 68 | 10/10 | 0 (8 görüntü) | 75.4 | 312.4 |

Uygulanmayan eşikler (hiçbir aday, `r4` dahil, geçemedi; §G9.1): ['text_contrast', 'nontext_contrast', 'touch', 'colour']

### K52 aynı ekranda: kapsül derinliği (k52open.mjs, Ek G7c; 3 tekrarın ortancası; ms)

| tasarım | kip | çerçeve | tik p95 | sahne boyama p95 (oynatma n / tekerlek n) | dururken uzun kare | tekerlek kipi | tekerlekte kare p99 | tekerlekte uzun kare payı | ilk kapsül | PHONE_BIAS ile tik / sahne | geçti |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `tesir` | `phone-jit-est` | [390, 670] | 1 | 0 (None / None) | 0 | ['scroll'] | 18 | 0 ([0, 0, 0]) | 308 | — | True |
| `tesir` | `phone` | [390, 670] | 1 | 0 (None / None) | 0.002 | ['scroll'] | 19 | 0 ([0, 0, 0]) | 337 | 1.6 / 0 | True |
| `nobra` | `phone-jit-est` | [390, 560] | 1 | 3 (37 / None) | 0 | ['zoom'] | 18 | 0 ([0, 0, 0]) | 321 | — | True |
| `nobra` | `phone` | [390, 560] | 1 | 11 (37 / None) | 0.015 | ['zoom'] | 19 | 0 ([0, 0, 0.003]) | 348 | 1.6 / 20.4 | True |
| `r4` | `phone-jit-est` | [390, 300] | 1 | 2 (37 / None) | 0 | ['zoom'] | 18 | 0 ([0, 0, 0]) | 336 | — | True |
| `r4` | `phone` | [390, 300] | 1 | 4 (37 / None) | 0.002 | ['zoom'] | 19 | 0 ([0, 0.003, 0]) | 354 | 1.6 / 7.4 | True |

### Birincil: kapsül derinliğinde `evidence` + `evidence_visitor` (F 2 : H 1; birim = görüntü × persona × tekrar × model; %95 önyükleme)

| tasarım | hepsi | yetkili (`live_event`) | yetkisiz (`visitor`) | F | H | birim (F/H) |
|---|---|---|---|---|---|---|
| `tesir` | 0.968 [0.949, 0.987] | 0.942 [0.902, 0.973] | 0.993 [0.980, 1.000] | 0.980 | 0.943 | 50/50 |
| `nobra` | 0.848 [0.812, 0.884] | 0.698 [0.667, 0.733] | 1.000 [1.000, 1.000] | 0.853 | 0.837 | 50/49 |
| `r4` | 0.732 [0.679, 0.784] | 0.964 [0.947, 0.982] | 0.500 [0.500, 0.500] | 0.750 | 0.697 | 50/50 |
| `kestim` | 0.726 [0.673, 0.777] | 0.951 [0.929, 0.973] | 0.500 [0.500, 0.500] | 0.750 | 0.677 | 50/50 |

### İkincil: kanıt derinliğinde aynı kümeler (F 2 : H 1; %95)

| tasarım | hepsi | yetkili | yetkisiz |
|---|---|---|---|
| `tesir` | 0.952 [0.915, 0.981] | 0.929 [0.899, 0.960] | 0.972 [0.917, 1.000] |
| `nobra` | 0.975 [0.954, 0.993] | 0.963 [0.935, 0.991] | 0.986 [0.958, 1.000] |
| `r4` | 0.972 [0.954, 0.991] | 0.944 [0.917, 0.972] | 1.000 [1.000, 1.000] |
| `kestim` | 0.972 [0.947, 0.993] | 0.972 [0.944, 1.000] | 0.972 [0.931, 1.000] |

Şans düzeyi (soru başına 1/k): e_rests 0,20 · e_who 0,333 · valid 0,20 · numbers 0,333.

### Soru başına doğruluk — kapsül derinliği (haiku, F+H; Wilson %95)

| tasarım | `e_rests` neye dayanıyor (yetkili) | `e_who` kim doğruladı (yetkili; anahtar cihaz) | `e_who` (yetkisiz; anahtar sunucu) | `valid` | `numbers` neden rakam yok (yetkisiz) | `e_who` gevşek (sunucu ∪ cihaz; bilgi) |
|---|---|---|---|---|---|---|
| `tesir` | 1.00 [0.93, 1.00] n=48 | 0.94 [0.83, 0.98] n=48 | 1.00 [0.93, 1.00] n=48 | 0.85 [0.73, 0.93] n=48 | 0.98 [0.89, 1.00] n=48 | 1.00 [0.96, 1.00] n=96 |
| `nobra` | 1.00 [0.93, 1.00] n=48 | 0.08 [0.03, 0.20] n=48 | 1.00 [0.92, 1.00] n=47 | 0.98 [0.89, 1.00] n=48 | 1.00 [0.92, 1.00] n=47 | 1.00 [0.96, 1.00] n=95 |
| `r4` | 1.00 [0.93, 1.00] n=48 | 1.00 [0.93, 1.00] n=48 | 1.00 [0.93, 1.00] n=48 | 0.83 [0.70, 0.91] n=48 | 0.00 [0.00, 0.07] n=48 | 1.00 [0.96, 1.00] n=96 |
| `kestim` | 1.00 [0.93, 1.00] n=48 | 1.00 [0.93, 1.00] n=48 | 1.00 [0.93, 1.00] n=48 | 0.77 [0.63, 0.87] n=48 | 0.00 [0.00, 0.07] n=48 | 1.00 [0.96, 1.00] n=96 |

Cevap dağılımı — kapsül derinliği (haiku; kod: sayı)

| tasarım | e_who yetkili | e_who yetkisiz | numbers | e_rests |
|---|---|---|---|---|
| `tesir` | {'device': 45, 'server': 3} | {'server': 48} | {'no_license': 47, 'unk': 1} | {'price': 48} |
| `nobra` | {'server': 44, 'device': 4} | {'server': 47} | {'no_license': 47} | {'price': 44, 'volume': 4} |
| `r4` | {'device': 48} | {'server': 48} | {'no_reason': 48} | {'price': 48} |
| `kestim` | {'device': 48} | {'server': 48} | {'no_reason': 48} | {'price': 48} |

### Soru başına doğruluk — kanıt derinliği (haiku, F+H; Wilson %95)

| tasarım | `e_rests` neye dayanıyor (yetkili) | `e_who` kim doğruladı (yetkili; anahtar cihaz) | `e_who` (yetkisiz; anahtar sunucu) | `valid` | `numbers` neden rakam yok (yetkisiz) | `e_who` gevşek (sunucu ∪ cihaz; bilgi) |
|---|---|---|---|---|---|---|
| `tesir` | 1.00 [0.86, 1.00] n=23 | 1.00 [0.86, 1.00] n=23 | 1.00 [0.86, 1.00] n=24 | 0.70 [0.49, 0.84] n=23 | 0.96 [0.80, 0.99] n=24 | 1.00 [0.92, 1.00] n=47 |
| `nobra` | 1.00 [0.86, 1.00] n=24 | 1.00 [0.86, 1.00] n=24 | 1.00 [0.86, 1.00] n=24 | 0.83 [0.64, 0.93] n=24 | 0.96 [0.80, 0.99] n=24 | 1.00 [0.93, 1.00] n=48 |
| `r4` | 1.00 [0.86, 1.00] n=24 | 1.00 [0.86, 1.00] n=24 | 1.00 [0.86, 1.00] n=24 | 0.75 [0.55, 0.88] n=24 | 1.00 [0.86, 1.00] n=24 | 1.00 [0.93, 1.00] n=48 |
| `kestim` | 1.00 [0.86, 1.00] n=24 | 1.00 [0.86, 1.00] n=24 | 1.00 [0.86, 1.00] n=24 | 0.88 [0.69, 0.96] n=24 | 0.92 [0.74, 0.98] n=24 | 1.00 [0.93, 1.00] n=48 |

Cevap dağılımı — kanıt derinliği (haiku; kod: sayı)

| tasarım | e_who yetkili | e_who yetkisiz | numbers | e_rests |
|---|---|---|---|---|
| `tesir` | {'device': 23} | {'server': 24} | {'no_license': 23, 'no_reason': 1} | {'price': 23} |
| `nobra` | {'device': 24} | {'server': 24} | {'no_license': 23, 'no_reason': 1} | {'price': 24} |
| `r4` | {'device': 24} | {'server': 24} | {'no_license': 24} | {'price': 24} |
| `kestim` | {'device': 24} | {'server': 24} | {'no_license': 22, 'unk': 1, 'no_reason': 1} | {'price': 24} |

### Kısma kopyası (kapsül derinliği, σ 4,5; bilgi; F 2 : H 1, %95)

| tasarım | `throttle` | t_first (işaret 1 / cümle 0,5) | t_dir | t_valid |
|---|---|---|---|---|
| `tesir` | 0.196 [0.172, 0.222] | 0.47 (n=95) | 0.14 (n=95) | 0.00 (n=95) |
| `nobra` | 0.104 [0.080, 0.130] | 0.22 (n=96) | 0.07 (n=96) | 0.02 (n=96) |
| `r4` | 0.120 [0.101, 0.138] | 0.36 (n=94) | 0.00 (n=94) | 0.00 (n=94) |
| `kestim` | 0.107 [0.090, 0.123] | 0.31 (n=95) | 0.00 (n=95) | 0.00 (n=95) |

### İkinci boy (sonnet; kapsül derinliği, tr/açık, 1 tekrar) ve boylar arası uyum

| tasarım | sonnet (F2:H1) | haiku aynı alt küme (tr/açık, tekrar 0) |
|---|---|---|
| `tesir` | 1.000 [1.000, 1.000] | 0.944 [0.889, 1.000] |
| `nobra` | 0.889 [0.778, 1.000] | 0.778 [0.556, 1.000] |
| `r4` | 0.750 [0.500, 1.000] | 0.750 [0.500, 1.000] |
| `kestim` | 0.750 [0.500, 1.000] | 0.750 [0.500, 1.000] |

Uyum (aynı uyaran, persona, soru; ilk tekrar): {'agree': 0.925, 'models': ['claude-haiku-5-5', 'claude-sonnet-5-5'], 'n': 40}

### Dokunma sayısı (taps.mjs, Ek G7b; akıştan cevabın göründüğü ekrana en az dokunuş; tr/açık)

| tasarım | neye dayanıyor (yetkili) | kim doğruladı (yetkili) | neye dayanıyor (yetkisiz) | kim doğruladı (yetkisiz) | neden rakam göremiyorum (yetkisiz) | denenen dokunuş / görülen ekran |
|---|---|---|---|---|---|---|
| `tesir` | 1 (capsule) | 1 (capsule) | 1 (capsule) | 1 (capsule) | 1 (capsule) | arkin: 0 / 1; visitor: 0 / 1 |
| `nobra` | 1 (capsule) | 1 (capsule) | 1 (capsule) | 1 (capsule) | 1 (capsule) | arkin: 0 / 1; visitor: 0 / 1 |
| `r4` | 1 (capsule) | 1 (capsule) | 2 (evidence) | 1 (capsule) | 2 (evidence) | arkin: 0 / 1; visitor: 362 / 3 |

### Eleştirmen (sonnet; 6 çift × 2 durum (kapsül derinliği) × 2 sıra × 3 tekrar; tr/açık)

| tasarım | BT gücü [%95 önyükleme] |
|---|---|
| `tesir` | 1.810 [1.195, 2.463] |
| `nobra` | 2.413 [1.803, 3.351] |
| `r4` | -0.817 [-1.229, -0.456] |
| `kestim` | -3.406 [-3.803, -2.801] |

| çift | bölünme / toplam | çelişki oranı | ayırt edilemedi (> %30) |
|---|---|---|---|
| `kestim|nobra` | 0/6 | 0.0 | hayır |
| `kestim|r4` | 0/6 | 0.0 | hayır |
| `kestim|tesir` | 0/6 | 0.0 | hayır |
| `nobra|r4` | 0/6 | 0.0 | hayır |
| `nobra|tesir` | 2/6 | 0.333 | evet |
| `r4|tesir` | 0/6 | 0.0 | hayır |

Uzunluk yanlılığı (U2): BT ~ ekran karakteri Spearman = 1.0; kesin sonuçlarda daha çok metinli olanın kazanma payı = {'n': 34, 'share': 0.971}

| tasarım | ham kararlar: kazandı / ayırt edemiyorum / kaybetti |
|---|---|
| `tesir` | 28 / 0 / 8 |
| `nobra` | 32 / 0 / 4 |
| `r4` | 12 / 0 / 24 |
| `kestim` | 0 / 0 / 36 |

Tek bakış dışı (birden çok Read) çıkarılan: 6; kesilen cevap: {'tesir': 5, 'nobra': 12, 'r4': 4, 'kestim': 4}

Maliyet (kayıtlardan): okuyucu 0.89 $ (970 çağrı), eleştirmen 1.03 $ (72 çağrı).

