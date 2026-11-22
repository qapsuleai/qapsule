# RD2 E1 — seçilen yönde ölçümler (bir kez, K82)

Yazan: RD2 E1 alt ajanı, 2026-11-23. Dal `rd2/e1`, ölçülen kod `a966a581` (`rd2/candidate`
birleşimi; bugünkü `main` `61bc3938` bu kodun üstüne yalnız doküman ekliyor, kod aynı).
"Önce" tabanı: RD2 yönünden önceki `main` `9aae35f7` (`git archive`, aynı makinede derlendi).
Makine: Apple M4 Max, macOS, Node 24.3, Playwright (Chromium + WebKit 26.5). Bütün veri sentetik
(SYN*); gerçek veri kullanılmadı. Derleme klasörü `target/wt/rd2-e1` (iş sonunda silindi).

Kabul ölçütleri: GOAL 10 (karşıtlık ve renk körlüğü iki profilde), 11 (K52, K17, WASM boyutu);
VERIFY 5 ve 13.

## Özet

| ölçüm | `tavek` (yoğun) | `tavek-tel` (dokunma) | sonuç |
|---|---|---|---|
| 1a. R5 düzeneği `compute` (§G7 + Ek G5): metin, işaret, en küçük yazı, dokunma, ΔE00 | metin 5,08; işaret 3,07; en küçük 11; ΔE00 en küçük 11,45 | metin 5,08; işaret 3,07; en küçük 13; dokunma < 44: 0; ΔE00 11,45 | **GEÇTİ** (yoğunda dokunma sayılmaz, aşağıda) |
| 1b. Bütün öğeler (R5 `S/CONTRAST.md` yöntemi) | eşik altı **332** (metin 4,5'in altı 152, metin dışı 3'ün altı 180); en kötü metin **2,41**, metin dışı **2,41** | eşik altı **312**; en kötü metin **2,41**, metin dışı **2,41** | **KALDI** — tek neden gri rol (`neutral` = `invalid` = `closed`) |
| 2. K52 (`calibrate.mjs`, 3 tekrar ortanca) | — (telefon ölçümü) | JIT açık tahmini bütçe içinde; JIT kapalı uzun kare %0,8 ≤ %10 | **GEÇTİ** |
| 3. K17 (`sweep_bench`, 15 tekrar) | yerel ortanca en çok 9,36 ms (< 50); WASM en çok 18,59 ms (< 150); özetler aynı | | **GEÇTİ**, etkilenmedi |
| 4. WASM boyutu | kabuk paketi gzip 2.431.842 B: RG1 tabanının %3,0 altında, önceki `main`'e göre +%0,95; varsayılan `qp_wasm.wasm` gzip +%3,09 | | **GEÇTİ** (hedef ≤ +%5) |
| 5. Önce/sonra görseli | `before-after.png` (1992×1688, 799 KB) | | hazır |

**Bulgu (ölçüt 10'u bozuyor):** `tavek` ve `tavek-tel` gri rolü koyu temada `#616369`, açıkta
`#8C8E94`. Yüzeye karşı karşıtlığı koyu 2,41–2,71, açık 2,61–2,93 (zemine göre değişiyor). R5'te
`oltan` aynı rol için 3,75 / 3,46 kullanıyordu. Bu renk hikâye çizelgesinde kapanmış hikâyelerin
şeridinde ve 11/13 px etiketinde, kapanmış işaretin dolgusunda (yüz, açılmış kapsül, kanıt)
kullanılıyor. Depodaki `design-check` (H1, H2) bunu yakalamıyor, çünkü H2 yalnız eksen, ızgara ve
denetim sınırı gibi kabuk rollerini ölçüyor, gri anlam rolünü ölçmüyor (H2 en kötü 3,49, `axis`).
Düzeltmeyi bu iş yapmadı (yazma kapsamı yalnız `E1/`). Öneriler aşağıda.

## 1. Sert eşikler

### 1a. R5 düzeneği, `glance.py compute` (§G7 + Ek G5)

Yöntem: `rounds/R5/experiments/harness/glance` hiç değiştirilmeden kullanıldı: `probe` → `states`
→ `shoot --kinds face,feed,evidence,capsule --voice marla` → `compute`. Kapsam: 2 dil × 2 tema,
402×874 @1×, sekiz yüz durumu, akış, kanıt (okur ve ziyaretçi), kapsül derinliği. Toplam 156 çekim.
Hiçbir aşama model çağırmıyor. Okuyucu ve eleştirmen aşamaları (`derive`, `read`, `critic`,
`score`) koşulmadı, bu işin kapsamında değiller. Denetim için `oltan` aynı koşuda ölçüldü.
Düzeneğin birim testleri geçti (`python3 -m unittest`, 56 test).

| tasarım | metin (en kötü / eşik) | işaret 1.4.11 (≥ 3, kapanmış dahil) | en küçük yazı | dokunma < 44 | ΔE00 en küçük / eşik | geçti |
|---|---|---|---|---:|---|---|
| `tavek` | **5,08** / 4,5 (13 px, açık, ziyaretçi kanıtı: "Bu kanıt şu yetkiyi gerektirir:") | **3,07** (koyu, `closed_realized` yüzü, `cap.SYN23…106`) | **11** (≥ 11 yoğun ✓; "Bu cihazda sınandı") | 180 (kabuk; yoğunda eşik yok) | **11,45** / 10 (açık, gri koşul, `up ↔ neutral`) | düzenek "False" diyor, tek nedeni dokunma |
| `tavek-tel` | **5,08** / 4,5 (16 px, aynı metin, en) | **3,07** (koyu, `state_live` yüzü) | **13** (≥ 13 dokunma ✓) | **0** | **11,45** / 10 | **True** |
| `oltan` (denetim) | 5,24 | 3,46 | 13 | 0 | 10,53 (koyu `up ↔ valid`) | True |

ΔE00, beş koşulun her birinde en küçük çift (17 renk kodlu çift, tema başına; başarısız 0; işaret
20/24 px, eşik 10). İki profilin değerleri aynı, çünkü renkler aynı:

| tema | normal | gri | protan | deutan | tritan |
|---|---|---|---|---|---|
| koyu | 27,67 (`up↔valid`) | 12,43 (`down↔neutral`) | 14,53 (`down↔neutral`) | 13,50 (`up↔down`) | 21,07 (`up↔valid`) |
| açık | 28,10 (`up↔neutral`) | **11,45** (`up↔neutral`) | 16,77 (`down↔valid`) | 12,43 (`up↔down`) | 19,49 (`up↔neutral`) |

Bilerek paylaşılan roller (`valid` = `text`, `invalid` = `closed` = `neutral`) Ek G5'teki gibi
sayılmadı.

`tavek`'te dokunma hedefi < 44 olan 180 öğenin hepsi kabuğun DOM düğmeleri: sekmeler
`#feed`/`#settings`/`#metrics` 36,2 px, `#back`/`#toevidence` 28,2 px. Görev kuralına göre 44 px
yalnız dokunma profilinde zorunlu. Yoğun profil 402 px telefon görünümünde çekildi. Bu yüzden bu
sayı kapı değil, kayıt. `tavek-tel`'de 0.

### 1b. Bütün öğeler (`S/CONTRAST.md` yöntemi, R5/fix47)

Yöntem: `rounds/R5/experiments/S/contrast/probe.mjs --look <ad>` ve `analyze.py`, değiştirilmeden.
Kapsam: akış yüzleri dört adımda (48 sahne), adım 200'de açılan 7 kapsül üç derinlikte (açılmış
kapsül, kanıt, grafik + hikâye çizelgesi; 84 sahne), kabuk DOM'u (44 blok), denetimler, yerel onay
kutuları. 2 tema × 2 dil, sayfa hatası 0. `analyze.py` eşik altı listeyi 30 satırda kesiyor.
Tam sayım için aynı betiğin `[:30]` kesmesi kaldırılmış bir kopyası scratch'te koşuldu; kopya
depoya girmedi, çıktıları `evidence/contrast-*.md` dosyalarında.

| yüzey | tema | en kötü metin (`tavek` / `-tel`) | en kötü metin dışı | eşik altı (`tavek` / `-tel`) |
|---|---|---|---|---|
| yüz (akış) | koyu | 6,46 / 6,46 (`muted`, 11 / 13 px) | **2,71** dolgu, gri | 4 / 4 |
| yüz (akış) | açık | 6,40 / 6,40 | **2,93** dolgu, gri | 4 / 4 |
| açılmış kapsül | koyu | 6,46 / 6,46 | **2,71** dolgu, gri | 6 / 6 |
| açılmış kapsül | açık | 5,84 / 5,84 (`on_accent`) | **2,93** dolgu, gri | 6 / 6 |
| kanıt derinliği | koyu | **2,41** / 2,41 (gri, "düşüş eğilimi…") | **2,41** dikdörtgen, gri | 78 / 68 |
| kanıt derinliği | açık | **2,61** / 2,61 | **2,61** dikdörtgen, gri | 78 / 66 |
| hikâye çizelgesi | koyu | **2,41** / 2,41 | **2,41** dikdörtgen, gri | 78 / 68 |
| hikâye çizelgesi | açık | **2,61** / 2,61 | **2,61** dikdörtgen, gri | 78 / 66 |
| kabuk (DOM) | koyu | 6,46 / 6,46 (`trust`) | 3,54 `select` | 0 / 0 |
| kabuk (DOM) | açık | 5,84 / 5,84 (`mark`) | 3,52 `select` | 0 / 0 |

Kanıt derinliği ve hikâye çizelgesi aynı grafik sahnesini ölçüyor; R5 tablosunda da böyleydi.
Bu yüzden toplamda aynı öğe iki kez sayılıyor.

Eşik altındaki her öğe gri rolde, `tavek-tel`'deki iki öğe dışında:

- **Hikâye çizelgesi ve kanıt:** kapanmış hikâye etiketleri. Metin 11 / 13 px. Karşıtlık koyu
  3,07, açık 3,28 (36'şar öğe); en kötü 2,41 / 2,61. Kapanmış şerit dikdörtgenleri, çizgileri,
  daireleri 2,41 / 2,61.
- **Yüz ve açılmış kapsül:** kapanmış ya da geçersiz işaretin gri dolgusu (adım 200,
  `cap.SYN02…127`, `cap.SYN12…129`). Karşıtlık koyu 2,71, açık 2,93.
- **Yalnız `tavek-tel`:** koyu temada `down` dikdörtgeni (`#D45C5F`), 2,50 (2 öğe), hikâye
  çizelgesinde bitişik renge karşı.

Kabuk DOM'u eşiklerin hepsini geçiyor.

R5'te `oltan`'ın aynı ölçümü eşik altı 0 vermişti (en kötü metin 5,67, metin dışı 3,07). 1a'daki
işaret kapısı (3,07) yalnız işaret katmanının en opak ilkelini ölçüyor. Bu yüzden gri dolguyu ve
hikâye çizelgesini görmüyor. Ölçüt 10'u bozan sonuç 1b'den geliyor.

### Depodaki denetim (`scripts/design-check`, H1–H3), karşılaştırma için

`tavek` ve `tavek-tel` iki temada PASS. H1: 6,16 / 5,84. H2: 3,49 / 3,80 (`axis`). H3: 12,0 /
10,7 (köken grisi). Tam çıktı `evidence/design-check.txt`. Bu kontrol gri anlam rolünü yüzeye
karşı ölçmüyor; 1b'deki açığın kaynağı bu.

## 2. K52 — telefon bütçesi

Yöntem: `rounds/R5/experiments/harness/glance/k52.mjs` (R5 §G7.6, Ek G5-4). Betik
`calibrate.mjs` `runCalibrated` ile `?look=` ekleyerek iki yapılandırmada ölçer:

- `phone-jit-est`: JIT açık tahmini, bağlayıcı;
- `phone`: kalibre, WebKit JIT kapalı, DPR 3, `PHONE_BIAS` uygulanıyor.

Ayarlar: 3 tekrarın ortancası, `--voice marla`. Koşu sırasında makinede başka ağır iş yoktu.
DESIGN §1.7'deki tablo biçimi (ms):

| | JIT açık: tik / yüz / sahne p95 | ilk kapsül | kaydırma kare p99 | JIT kapalı: tik / yüz / sahne p95 | kaydırma p99 | uzun kare payı (JIT kapalı) | ilk kapsül (JIT kapalı) |
|---|---|---:|---:|---|---:|---:|---:|
| `tavek-tel` | 1 / 1 / 3 | 215 | 19 | 3,2 / 2 / 14,8 | 25 | 0,008 | 217 |
| `oltan` (taban, aynı koşu) | 1 / 1 / 3 | 212 | 19 | 1,6 / 2 / 12,95 | 19 | 0 | 171 |

Kapılar:

- **JIT açık:** tik, yüz ve sahne p95 ≤ 8; ilk kapsül ≤ 1500; kaydırma p99 ≤ 33,4. `tavek-tel`
  hepsini geçiyor.
- **JIT kapalı:** "kullanılabilir" kapısı uzun kare ≤ %10. Ölçülen %0,8, geçiyor.

JIT kapalıyken `tavek-tel` `oltan`'dan biraz daha pahalı: tik p95 1,6 → 3,2; sahne 12,95 → 14,8;
kaydırma p99 19 → 25. Bütçe içinde kalıyor. JIT kapalı sayılar kapı değil, kayıt.

R5'teki `oltan` JIT kapalı sahne p95 değeri 7,4 idi. Bugünkü 12,95'in farkı, R5'ten bu yana
değişen sahne içeriğinden ve ölçüm gününden geliyor. Karşılaştırma aynı koşudaki `oltan` ile
yapıldı. Ham kayıtlar `evidence/k52-*.json`.

## 3. K17 — hesap bütçesi

Hesap yolu değişmedi: `git diff 0d78c440 a966a581 -- crates/qp-compute` boş. `0d78c440`, RG1
tutanağının commit'i. Standart tezgâh bir kez koşuldu: `sweep_bench --reps 15 --profiles none`,
`scripts/rg1-perf.sh` ile aynı. WASM tezgâhı da bir kez koşuldu (`wasm-sweep-bench.sh`'ın adımları,
çıktı klasörü scratch'te).

| çalışma | RG1 tutanağı ortanca (ms) | şimdi ortanca (ms) | fark | özet (aynı) | WASM şimdi (ms) |
|---|---:|---:|---:|---|---:|
| sma | 4,56 | 4,76 | +%4,4 | `d9a393c2f397ae9e` | 12,66 |
| ema | 4,15 | 3,91 | −%5,8 | `f9be271e91103539` | 11,20 |
| rsi | 9,70 | 9,13 | −%5,9 | `9515526d5288453b` | 18,59 |
| macd | 8,46 | 7,70 | −%9,0 | `d5a6c407256992e6` | 10,57 |
| bollinger | 9,08 | 9,36 | +%3,1 | `7e683c082fb95dc9` | 11,76 |

WASM `compute_digest` `7b1e99d40889f83c` (K19'daki değer).

K17 hedefleri: yerel < 50 ms, WASM < 150 ms. En kötü değerler 9,36 / 18,59. Farklar iki yönde de
var. RG1 aynı makinede ±%7 makine durumu gürültüsü ölçmüştü; farklar bu gürültünün içinde. Kod aynı
olduğu için **etkilenmedi**.

## 4. WASM boyutu

Yöntem: kabuk paketi `scripts/rg1-perf.sh` gibi derlendi (wasm-pack web, profil `wasm`,
`--no-opt`, `--features replica`, soyulmamış). RG1 tabanı bu yöntemle ölçülmüştü. Ayrıca
`scripts/demo.sh`'ın sunduğu soyulmuş paket ve varsayılan `qp_wasm.wasm` (verify adım 8) ölçüldü.
Sıkıştırma `gzip -9` ve `brotli -q 11`. "Önce" sütunu `9aae35f7`, aynı makinede aynı komutlarla
derlendi.

| paket | önce (`main` `9aae35f7`) ham / gzip / brotli | şimdi (`a966a581`) ham / gzip / brotli | fark gzip | fark brotli |
|---|---|---|---:|---:|
| kabuk, `replica`, soyulmamış (RG1 yöntemi) | 8.731.379 / 2.408.953 / 1.648.679 | 8.788.005 / **2.431.842** / 1.665.529 | +22.889 (**+%0,95**) | +16.850 (+%1,02) |
| kabuk, `replica`, soyulmuş (`demo.sh`, sunulan) | 6.890.093 / 2.057.034 / 1.390.588 | 6.941.415 / 2.076.904 / 1.404.891 | +19.870 (+%0,97) | +14.303 (+%1,03) |
| varsayılan `qp_wasm.wasm` (verify adım 8) | 2.282.026 / 746.730 / 556.814 | 2.345.845 / 769.827 / 574.408 | +23.097 (**+%3,09**) | +17.594 (+%3,16) |

- **RG1 tabanına göre** (2.506.216 B gzip): kabuk paketi 2.431.842 B, yani −74.374 B (−%2,97).
  VERIFY 5 geçiyor.
- **GOAL'ın ≤ +%5 hedefi:** kabuk paketi +%0,95, varsayılan modül +%3,09. İkisi de geçiyor.
- **Bugünkü `main` (`61bc3938`):** kodu `a966a581` ile aynı, boyutları da aynı.

Varsayılan modüldeki büyüme ham +63.819 B. Kaynağı gömülü `tavek` yazı tipi ve kayıtları olmalı;
bu iş bunu ayrıca ölçmedi.

## 5. Önce/sonra görseli

`before-after.png`: `scripts/design-gallery --looks tavek --anchors c1,c3`, sentetik fikstür
`fixtures/design/desk.json` (SYN20), koyu tema. Üst satır Ç1 masaüstü 1440×900 (DPR 2'den
küçültüldü), soldan sağa `oltan` ve `tavek`. Alt satır Ç3 telefon 390×844 (DPR 3'ten küçültüldü),
`oltan` ve `tavek-tel`. Etiketli, 1992×1688 px, 799 KB.

## Kapsanmayanlar

- **Okuyucu ve eleştirmen aşamaları:** R5 düzeneğinin model gerektiren aşamaları (`read`,
  `critic`, `score`, `broken-check`) ve `derive` koşulmadı. Sert eşikler bunlara bağlı değil.
  Bozuk tasarım (`kestim`) yamalı derlemesi de koşulmadı; yalnız düzenek kendi sınamasında
  gerekiyor.
- **Yoğun profilin görünüm boyu:** `tavek` 402×874 telefon görünümünde ölçüldü, çünkü iki düzenek
  de bu boyu sabit kullanıyor. Masaüstü kabuğun (Ç1) DOM'u bu düzeneklerin kapsamında değil.
  Ç1 için yalnız depodaki `design-check` (sahne ve kayıt düzeyi) var.
- **Kademe 2–3 maketleri:** `docs/design/mockups` HTML'leri hiçbir düzenekle ölçülmedi.
- **Grafik derinliği K52:** `k52chart.mjs` (5/50/200 hikâye) ve `k52open.mjs` koşulmadı. K52
  yalnız yüz akışında, DESIGN §1.7 gibi ölçüldü.
- **Hareketi azalt:** `reduced_motion_loss` (§G7.7) ölçülmedi.
- **Seçilebilir alfabe `rozet`:** ölçülmedi. Gösterim alfabesi ve `tg5` ölçüldü.

## Öneriler (kapsam dışı, yapılmadı)

1. `tavek` gri rolünü (`neutral` = `invalid` = `closed`) yüzeye karşı en az 3:1'e çıkar. R5'te
   `oltan` için yapılan buydu (koyu `#6E7076` 3,75; açık `#888A8E` 3,46). Gri ladder testini ve
   ΔE00 tablosunu yeniden koş, çünkü açık temadaki en küçük değer zaten bu rolde (`up ↔ neutral`
   gri 11,45).
2. Kapanmış hikâye etiketlerini (11/13 px metin) gri değil `muted` rengiyle çiz; metin eşiği 4,5.
3. `design-check` H2'ye gri anlam rolünü yüzey ve sayfa zeminine karşı ekle. H1'e gri rolle çizilen
   metin rollerini ekle. Böylece bu açık 1b koşulmadan da görünür.
4. `tavek-tel` koyu temada `down` (`#D45C5F`) hikâye çizelgesinde bitişik renge karşı 2,50. Hangi
   bitişiklik olduğunu `evidence/contrast-tavek-tel.md`'den bul.

## Kanıt dosyaları (`evidence/`)

- `computed.json`, `computed.md`: 1a.
- `contrast-tavek.md`, `contrast-tavek-tel.md`: 1b, tam liste.
- `design-check.txt`
- `k52-tavek-tel.json`, `k52-oltan.json`
- `k17.txt`
- `wasm-sizes.txt`: "main" satırları `9aae35f7`.
- `designs.json`

## Düzeltme sonrası (RD2 gri düzeltmesi, ölçüt 10)

Yazan: RD2 gri düzeltmesi alt ajanı, 2026-11-23, dal `rd2/fix-grey`. Yalnız 1b (bütün öğeler)
yeniden koşuldu: aynı `probe.mjs --look <ad>` ve aynı `analyze.py`, kesmesi kaldırılmış kopya
(scratch'te; ek olarak eşik altı satırına zemin rengini yazıyor). Kabuk paketi ve `qp` bu dalın
koduyla derlendi. 1a, K52, K17 ve WASM yeniden koşulmadı. Çıktılar
`evidence/contrast-tavek-after.md`, `evidence/contrast-tavek-tel-after.md`.

**Nerede kalıyordu (önce koşusunun zemin dökümü):** gri rol, metin dışı öğe olarak vurgu
şeridinin (`surface.strip_alpha` 40, `accent`) üstünde 2,41 / 2,61, şeridin üstündeki kanıt
bandında da eşik altı ve kendi plakasında (`grey_plate_alpha_max` 40 / 29) 2,71 / 2,93 idi.
Metin olarak kapanmış hikâye etiketleri (`korda`, `closed` rengi) zeminde 3,07 / 3,28,
şeritte 2,41 / 2,61 idi. `tavek-tel`'deki `down` öğesi, değer çizgisinin 2×1 px noktası,
kendi hacim çubuğunun (`down` %35) şerit üstündeki hâline karşı 2,50 idi.

| | önce eşik altı (`tavek` / `-tel`) | önce en kötü metin / metin dışı | sonra eşik altı | sonra en kötü metin / metin dışı |
|---|---|---|---|---|
| `tavek` | 332 | 2,41 / 2,41 | **0** | 5,48 (açık, `bg` rengi yazı `up` etiketinde) / 3,30 (koyu, gri dolgu, yüz) |
| `tavek-tel` | 312 | 2,41 / 2,41 | **0** | 5,48 / **3,04** (koyu, `down` noktası kendi hacim çubuğunda, şeritte) |

İki temada, yüz, açılmış kapsül, kanıt, hikâye çizelgesi ve kabuk yüzeylerinin hepsinde eşik
altı 0; sayfa hatası 0. Gri rolün en kötüsü artık 3,30 / 3,31 (yüzde, kendi plakasında).

**Değişen değerler** (yalnız `crates/qp-grammar/src/look/tavek.rs`; `tavek-tel` renkleri
`..TAVEK` ile aynı; eski görünüşler değişmedi):

- **Gri rol** (`neutral` = `invalid` = `closed`): koyuda açıldı (L\* 42 → 47; zeminde 3,72),
  açıkta koyulaştı (L\* 59 → 55; zeminde 3,74).
- **Koyu `down`:** biraz daha açık ve sıcak (L\* 54,5 → 58,8; HSL S 60, L 59, P3 içinde).
  Gri açılınca gri ↔ aşağı ΔE00'ı korumak için.
- **Açık `up` ve `down`:** biraz koyulaştı (L\* 47,3 → 44,4; 32,6 → 30,8). Gri koyulaşınca
  gri ↔ yukarı ve yukarı ↔ aşağı ΔE00'ı korumak için.
- **Koyu `axis`:** bir basamak açıldı (kanıt bandı + şerit üstünde 2,99 → 3,07).
- **`surface.strip_alpha`** 40 → 16 ve **`story.focus_row_alpha`** 40 → 16 (`oltan`'daki R5
  düzeltmesiyle aynı yol: gri şeritler bandın üstünde de 3:1).
- **Koyu `grey_plate_alpha_max`** 40 → 32.
- **Yeni görünüş değeri `story.closed_label`** (`korda`'nın kapanmış hikâye etiketinin rolü):
  `r4` ve ondan türeyenlerde `closed` (çıktı aynı), `tavek`'te `muted`. Etiket metin olduğu
  için 4,5:1 istiyor; gri rol bunu tutamıyor (merdivende yeri yok, aşağıda).

**Merdiven (L\*), düzeltme sonrası:** koyu gri 47,3 < aşağı 58,8 < yukarı 72,6 < mürekkep
91,6; açık gri 55,2 > yukarı 44,4 > aşağı 30,8 > mürekkep 8,8. `design-check` H3 en kötü:
koyu 11,3 (`up ↔ down`, gri koşul), açık anlam rollerinde 10,7 (`grey ↔ up`, gri koşul).
Gri rolü metin eşiğine (4,5) çıkarmak koyu temada griyi aşağının, açık temada yukarının
ışıklığına taşıyordu; ΔE00 ≥ 10 ile ikisi birden sığmadı. Bu yüzden etiket `muted`.

**`design-check`:** H1 ve H2 artık rolü çizildiği her zemine karşı ölçüyor (`bg`, `surface`,
kart, şerit, seçili satır bandı, kanıt bandı zeminde ve şeritte, gri rolün kendi plakası,
`up`/`down`'ın kendi hacim çubuğu zeminde ve şeritte); gri rol H2'de, kapanmış hikâye
etiketi H1'de. Eski değerlerle `tavek` ve `tavek-tel` dört satırda kaldı (H1 2,13 / 2,36;
H2 2,13 / 2,36, gri, kanıt bandı + şerit). Yeni değerlerle PASS: H1 5,27 / 5,26; H2 3,04 /
3,06; H3 11,3 / 10,7.
