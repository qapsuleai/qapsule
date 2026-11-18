# R5/D — yoğunluk sınaması: `oltan` R2'nin altı görünümünde

Dal `r5/d-density` (`r5/m16-oltan` + `main`), 2026-11-18. Kabul ölçütü 5 (`rounds/R5/GOAL.md`,
"Kapsam kararı"): M16'nın seçtiği görsel dil `oltan` (görünüş kaydı `OLTAN`,
`crates/qp-grammar/src/look.rs`) R2'nin altı çok panelli görünümüne uygulandı ve okunabilirliği
ölçüldü; başvuru aynı görünümler `r4` ile. Etkileşim yok (R7).

> Ajan okuyucu insan değildir; buradaki okuma sonuçları ön elemedir. Hesapla ölçülenler sahne
> tarifinden (element düzeyinde) gelir ve deterministiktir.

## Özet

- **Okunuyor.** Kör okuyucu (haiku ve sonnet, F ve H personaları) altı görünümdeki olgu sorularında
  `oltan` ile `r4`'ü **ayırt edemedi**: eşleşmiş görünümlerde haiku 0,996 [0,977, 0,999] ↔ 1,000
  [0,984, 1,000] (v0–v1), sonnet 0,979 [0,955, 0,990] ↔ 0,986 [0,965, 0,995] (v0–v3). Hiçbir kesimde
  (boy, tema, persona, boy × tema) `oltan` aşağıda değil. Okuma tavanda: bu sorular dilin farkını
  ölçecek kadar zor değil (aşağıda "Sınırlar").
- **Karşıtlık ve yazı boyu yoğun ekrana taşınıyor.** R2'nin altı görünümünde `oltan`'ın bütün metin
  öğeleri ≥ 4,5:1 (en kötü 5,67, son değer etiketi ve kapanış), en küçük yazı 13 px (`r4` 11 px;
  son değer etiketi 3,08, kapanış 3,57, açıklama 2,74 ile kalıyor). 112 sahnede (7 görünüm × 2
  görünüş × 2 tema × 4 boy) metin çakışması 0, çerçeveden taşma 0, 2 px'ten yakın metin çifti 0.
- **Kırıldığı yerler** (ayrıntı aşağıda): (1) **seri paleti s1–s8 gri tonlamada ayrılmıyor**
  (28 çiftin 22–23'ü ΔE00 < 10; beş koşulun hepsinde ayrı kalan ilk seri sayısı açıkta 2, koyuda 1)
  ve açıklama seriyi yalnız renkle eşliyor; (2) **seri rengi açıklama metninde 4,5:1'i tutmuyor**
  (açık `s2` 3,94; R2 görünümlerinde değil, 8 serili sınama görünümünde); (3) **açıklama kaybı**:
  gramer sığmayan açıklamayı düşürür; telefonda 18 açıklamadan 10'u, 320 px'te 5'i, 640 × 360
  çeyrek ekranda 13'ü (`r4` 14) kalıyor — 14 px açıklama yazısı `r4`'ün 11 px'ine göre bir açıklama
  daha yitiriyor; (4) **hiyerarşi**: `oltan`'ın `muted` rengi neredeyse metin kadar koyu (açık
  14,35:1 ↔ metin 18,72:1), bu yüzden eksen yazıları ve 70/30 kuralları veri çizgisinden daha
  yüksek karşıtlıkla çiziliyor (açık: kural 14,35, RSI çizgisi 6,42); (5) eksen çizgisi 2,88/2,95 ile
  1.4.11'in altında (`r4` de: 2,25/2,71).
- Maliyet **2,98 $** (bütçe 6 $). Okuyucu koşusu kullanım sınırına takıldı: 768 çağrının 354'ü
  cevapsız döndü ve puanlamadan çıkarıldı; v3–v5'in haiku kapsamı ve v4–v5'in sonnet kapsamı eksik
  (aşağıda kapsam tablosu ve devam komutu).

## Düzenek

| | |
|---|---|
| Görünümler | `fixtures/views/v0-r0` … `v5-conditional` (R2'nin altı görünümü) + **`s8`**: yalnız bu sınama için v1'in üç paneline sekiz SMA çizgisi (5…150) eklenmiş görünüm (s1–s8 paleti ve dokuz açıklama aynı başlık satırında). `s8` fikstür değildir; `render.py` içinde kuruluyor, okuyucuya gösterilmedi. |
| Çizici | `qp render` (deterministik `ref`), `qp scene --roles` (sahne tarifi, metin rolleri). `qp` bu dalın sürüm derlemesi. |
| Tema | `qp render`'da tema bayrağı yok: dökümün kopyasında `put_view` görünümüne `"theme"` yazıldı (kopya depo dışında). |
| Boylar | telefon **402 × 600**, masaüstü **1280 × 720** (okurun gördüğü boy, 1×; R7'nin hedefi); sınama boyları **320 × 480** (en dar yaygın telefon) ve **640 × 360** (1280 × 720 masaüstünün 2 × 2 bölünmüş çeyreği). |
| Hesap | `measure.py`: düzeneğin `gl/compute.py` (`scene_prims`, `background_at`, `_anchor`) ve `gl/color.py` (`contrast`, `text_threshold`, `pair_de`: normal, gri, protan, deutan, tritan; CVD'de Viénot/Brettel ile Machado'nun küçüğü) işlevleriyle. Metin kutusu: çizicinin kullandığı yazı tipi dosyasıyla (`crates/qp-render/assets/fonts`) FreeType mürekkep kutusu (Pillow, `Layout.BASIC`; fontdue kerning uygulamaz). Tablolar `summarize.py` → [`computed.md`](computed.md). |
| Okuyucu | `readers_d.py`: düzeneğin `gl/readers.py` koşturucusu (`claude -p --tools Read`, tek görüntü rastgele adla, tek bakış, cevap başına tek cümle, `check_prompt` tasarım adlarını dışarıda tutar) ve `gl/stats.wilson`. Görünüm başına üç olgu sorusu, seçenekler çağrı başına tohumla karışık, "bilmiyorum" son seçenek; anahtar görünüm verisinden (`*.explain.txt`, `docs/base.json`, `docs/growth.json`) çağrılardan önce yazıldı. 6 görünüm × 2 görünüş × 2 tema × 2 boy (telefon, masaüstü) × 2 persona; haiku 5 tekrar, sonnet 3 tekrar. Tablolar → [`readers.md`](readers.md). |
| "Ayırt edilemedi" | iki görünüşün Wilson %95 aralıkları örtüşüyorsa (SPEC §G kuralı). |
| Örnekler | [`samples/`](samples/) (sentetik veri, her biri ≤ 46 KB): v1 telefon `oltan` açık/koyu ve `r4` açık, v2 telefon `oltan` açık, `s8` telefon `oltan` koyu, `s8` çeyrek `oltan` açık. |

Ekran görüntüleri ve okuyucu kayıtları depo dışında (scratchpad); depoya yalnız betikler, tablolar
ve altı küçük örnek girdi.

## 1. Hesapla ölçülenler

### Metin karşıtlığı, öğe başına (en kötü; 7 görünüm × 4 boy)

| öğe | `oltan` açık | `oltan` koyu | `r4` açık | `r4` koyu |
|---|---|---|---|---|
| eksen değeri (y) | 14,35 / 4,5 (13 px) | 9,77 / 4,5 (13 px) | 5,78 / 4,5 (11 px) | 5,27 / 4,5 (11 px) |
| zaman etiketi (x) | 14,35 / 4,5 (14 px) | 9,77 / 4,5 (14 px) | 5,78 / 4,5 (11 px) | 5,27 / 4,5 (11 px) |
| son değer etiketi | 5,69 / 4,5 (13 px) | 5,67 / 4,5 (13 px) | 3,08 / 4,5 (11 px) **✗** | 5,27 / 4,5 (11 px) |
| başlık + kapanış | 5,69 / 4,5 (15 px) | 5,67 / 4,5 (15 px) | 3,57 / 4,5 (16 px) **✗** | 5,37 / 4,5 (16 px) |
| açıklama (lejant) | 3,94 / 4,5 (14 px) **✗** (`s8`: SMA10 = `s2`) | 6,66 / 4,5 (14 px) | 2,74 / 4,5 (11 px) **✗** | 3,89 / 4,5 (11 px) **✗** |

R2'nin altı görünümünde açıklamalar `s1` ya da `up` renginde: `oltan` en kötü 5,69 (açık), geçiyor.
Açık `oltan` paletinde metin olarak 4,5'in altında kalan tek renk `s2` (3,94); `s4` 4,64, `s6` 4,67
sınırda. `r4` açıkta `s1` 3,08, `s2` 3,68, `s4` 3,30, `s6` 2,74.

### Metin dışı karşıtlık (en kötü; WCAG 1.4.11 ≥ 3)

| öğe | `oltan` açık | `oltan` koyu | `r4` açık | `r4` koyu |
|---|---|---|---|---|
| ızgara (bilgi) | 1,23 | 1,23 | 1,20 | 1,24 |
| eksen çizgisi | 2,88 **✗** | 2,95 **✗** | 2,25 **✗** | 2,71 **✗** |
| mum: yukarı | 5,69 | 10,97 | 3,57 | 6,24 |
| mum: aşağı | 10,22 | 5,67 | 4,23 | 5,37 |
| seri çizgisi | 3,94 | 5,67 | 2,74 **✗** | 3,89 |
| histogram | 5,69 | 5,67 | 3,57 | 5,37 |
| eşik kuralı (70/30) | 14,35 | 9,77 | 5,78 | 5,27 |
| bant/alan dolgusu (saydam; bilgi) | 1,26 | 1,29 | 1,16 | 1,30 |
| son değer kutusu | 5,69 | 5,67 | 3,08 | 5,27 |

### En küçük yazı, metin yoğunluğu, çakışma, açıklama kaybı (açık tema; koyu aynı)

| boy | look | en küçük yazı | metin öğesi (ort.) | karakter / 10 000 px² | metin mürekkep alanı | çakışma | taşma | < 2 px | açıklama gösterilen / yazılan |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| telefon 402 | `oltan` | 13 px | 12,4 | 3,61 | 2,7 % | 0 | 0 | 0 | 10 / 18 |
| telefon 402 | `r4` | 11 px | 12,4 | 3,61 | 2,5 % | 0 | 0 | 0 | 10 / 18 |
| masaüstü 1280 | `oltan` | 13 px | 20,0 | 1,58 | 1,2 % | 0 | 0 | 0 | 18 / 18 |
| masaüstü 1280 | `r4` | 11 px | 20,0 | 1,58 | 1,0 % | 0 | 0 | 0 | 18 / 18 |
| dar 320 | `oltan` | 13 px | 11,1 | 4,83 | 3,6 % | 0 | 0 | 0 | 5 / 18 |
| dar 320 | `r4` | 11 px | 11,1 | 4,83 | 3,4 % | 0 | 0 | 0 | 5 / 18 |
| çeyrek 640 × 360 | `oltan` | 13 px | 13,0 | 4,16 | 3,2 % | 0 | 0 | 0 | 13 / 18 |
| çeyrek 640 × 360 | `r4` | 11 px | 13,1 | 4,23 | 2,8 % | 0 | 0 | 0 | 14 / 18 |

Açıklama kaybının görünüm başına dökümü `computed.md`'de: telefonda v2 (Bollinger) 2'den 1, `s8`
9'dan 2 (yalnız SMA150 ve RSI14); 320 px'te v0 ve v2 açıklamasız, `s8` 9'dan 1; çeyrekte `s8`
`oltan` 4, `r4` 5. Çakışma 0'ın nedeni gramer: başlık satırındaki açıklamalar görünüşün ölçülmüş
genişlik tablosuyla (`advances.rs`) yerleştiriliyor ve sığmayan, solundakilerle birlikte düşürülüyor
(`compile.rs`, "the right-most are kept"). Yani yoğunluk çakışma olarak değil **bilgi kaybı** olarak
ortaya çıkıyor.

### Seri paleti s1–s8 (28 çift; ΔE00)

| look / tema | metin dışı < 3 | en küçük ΔE00: normal / gri / prot / deut / trit | ΔE00 < 10 çift: normal / gri / prot / deut / trit | beş koşulda da ≥ 10 kalan ilk k seri |
|---|---|---|---|---|
| `oltan` açık | yok | 13,9 / 0,2 / 0,4 / 4,4 / 5,8 | 0 / 22 / 2 / 3 / 5 | 2 |
| `oltan` koyu | yok | 17,7 / 0,4 / 4,8 / 0,6 / 6,0 | 0 / 23 / 3 / 4 / 4 | 1 |
| `r4` açık | s6 2,74 | 19,3 / 1,8 / 7,5 / 8,2 / 5,0 | 0 / 12 / 1 / 1 / 5 | 1 |
| `r4` koyu | yok | 20,1 / 1,1 / 6,7 / 9,2 / 6,0 | 0 / 13 / 1 / 1 / 4 | 3 |

En kötü `oltan` çiftleri: açık `s4–s6` 0,2 (gri), `s2–s6` 0,4 (protan), `s1–s8` 0,5 (gri); koyu
`s1–s5` 0,4 (gri), `s1–s3` 0,6 (deutan), `s4–s8` 0,7 (gri). Bütün liste `computed.md`'de.

### Anlam renkleri ve hiyerarşi (kayıttan)

| look / tema | up–down ΔE00 (normal / gri / prot / deut / trit) | metin/zemin | muted/zemin | metin–muted ΔE00 | eksen/zemin |
|---|---|---:|---:|---:|---:|
| `oltan` açık | 60,4 / 13,2 / 20,5 / 11,7 / 50,0 | 18,72 | 14,35 | 7,8 | 2,88 |
| `oltan` koyu | 71,8 / 16,5 / 26,3 / 14,7 / 63,8 | 17,13 | 9,77 | 14,7 | 2,95 |
| `r4` açık | 63,0 / 4,7 / 20,5 / 23,6 / 61,3 | 15,80 | 5,78 | 22,6 | 2,25 |
| `r4` koyu | 59,5 / 4,0 / 19,8 / 28,8 / 61,0 | 11,74 | 5,27 | 20,1 | 2,71 |

Mum/histogram yukarı–aşağı ayrımı `oltan`'da gri tonlamada `r4`'ten çok daha iyi (13,2/16,5 ↔ 4,7/4,0):
M16'nın parlaklık merdiveni grafiğe de taşınıyor.

### Veri işaretinin boyu ve yelpaze tonları

- Mum gövdesi telefon, 320 ve çeyrek boyda **1 px = fitil** (200 bar / ~320 px çizim alanı); masaüstünde
  3 px gövde, 1 px fitil. İki görünüşte aynı (gramer). Telefonda mum yönü yalnız renkle (ve 1 px'lik
  bir çizginin uzunluğuyla) okunuyor; `oltan` açıkta yukarı (L\* 43) ve aşağı (L\* 28) iki koyu renk,
  benim gözümle 1 px'te tek koyu doku gibi duruyor (ölçüm değil; ΔE00 normal 60,4).
- v3 yelpazesi (aynı renk, üç saydamlık): komşu tonlar arası ΔE00 `oltan` açık 10,4 / 10,8 / 10,7
  (gri 5,3 / 7,4 / 9,1), koyu 9,6 / 10,7 / 11,6; `r4` açık 9,4 / 9,4 / 8,6 (gri 3,5 / 4,6 / 5,2). Okuyucu
  "kaç ton" sorusunda `oltan` 22/24, `r4` 23/24 (sonnet).

## 2. Kör okuyucu

Kapsam (puanlanan çağrı / planlanan). Kullanım sınırına takılan çağrılar (cevap: "You've hit your
5-hour limit") ve `Read`'den sonra sınıra takılanlar puanlamadan çıkarıldı; yanlış sayılmadı:

| görünüm | haiku `oltan` | haiku `r4` | sonnet `oltan` | sonnet `r4` |
|---|---:|---:|---:|---:|
| v0 | 40/40 | 40/40 | 24/24 | 24/24 |
| v1 | 40/40 | 39/40 | 24/24 | 24/24 |
| v2 | 34/40 | 5/40 | 24/24 | 24/24 |
| v3 | 0/40 | 0/40 | 24/24 | 24/24 |
| v4 | 2/40 | 0/40 | 15/24 | 3/24 |
| v5 | 0/40 | 0/40 | 4/24 | 0/24 |

Karşılaştırma yalnız iki görünüşte de kapsamı ≥ %90 olan görünümlerde yapıldı (kapsam eşit değilken
toplam oran zor görünümü bir tarafa yığar):

| okuyucu, kesim | `oltan` [Wilson %95] | `r4` [Wilson %95] | sonuç |
|---|---|---|---|
| haiku, v0–v1 | 0,996 [0,977, 0,999] (n 240) | 1,000 [0,984, 1,000] (n 237) | ayırt edilemedi |
| sonnet, v0–v3 | 0,979 [0,955, 0,990] (n 288) | 0,986 [0,965, 0,995] (n 288) | ayırt edilemedi |
| sonnet, telefon | 1,000 [0,979, 1,000] | 1,000 [0,975, 1,000] | ayırt edilemedi |
| sonnet, masaüstü | 0,964 [0,924, 0,984] | 0,973 [0,932, 0,989] | ayırt edilemedi |
| sonnet, koyu | 0,968 [0,927, 0,986] | 0,972 [0,931, 0,989] | ayırt edilemedi |
| sonnet, masaüstü × koyu | 0,933 [0,853, 0,971] | 0,944 [0,866, 0,978] | ayırt edilemedi |
| sonnet, persona F / H | 0,994 / 0,970 | 0,973 / 1,000 | ayırt edilemedi (ikisi de) |
| sonnet, v4 (kapsam eksik) | 1,000 [0,921, 1,000] (n 45) | 1,000 [0,701, 1,000] (n 9) | ayırt edilemedi |

Bütün kesimler (haiku dahil, her boy × tema, her persona, her görünüm) `readers.md`'de; hiçbirinde
bir taraf ayırt edilmedi.

Yanlışlar az ve belirli:

- **Masaüstü, koyu** iki görünüşte de en zayıf hücre (sonnet `oltan` 0,933, `r4` 0,944): v1'de panel
  sayısı (bir `oltan` cevabı "4"; 70/30 kuralları koyu temada parlak ve uzun, bir okuyucu ayrı bölme
  saydı olabilir — tek kayıt, kanıt değil) ve "bilmiyorum".
- **v3 ton sayısı:** `oltan` masaüstünde 2 kez "5" (sonnet), `r4` 1 kez.
- **v2 "fiyat bandın neresinde"** (haiku): `oltan` 7 "bilmiyorum" ("üst yarıda ama orta çizgiye yakın
  mı emin değilim"); `r4` kapsamı 5 çağrı, karşılaştırılamaz. Soru sınırda (kapanış 124,69, orta
  124,97, üst 128,51); dilden çok soruya bağlı.
- **v2 son değer etiketi kapanışı değil üst bandı gösteriyor** (128,51; iki görünüşte aynı, gramer).
  Okuyucular kapanışı başlıktaki "C 124.69"dan okudu (sonnet 24/24 iki görünüşte).

## 3. `oltan`'ın yoğun ekranda kırıldığı yerler

1. **Seri paleti çok seriyi taşımıyor.** Normal görüşte s1–s8 ayrı (en küçük ΔE00 13,9/17,7), ama gri
   tonlamada 28 çiftin 22–23'ü, renk körlüğü benzetimlerinde 2–5'i ΔE00 < 10. M16 merdiveni yalnız anlam
   renklerine (up/down/gri/mürekkep) uygulandı; seri paleti parlaklıkça düz (çoğu L\* 50–75 koyuda,
   35–55 açıkta). Açıklama seriyi yalnız renkle eşliyor (çizgi biçimi, uçta ad yok) → çok serili
   panelde "renk tek başına anlam taşımaz" (GOAL tasarım sınırı) tutmuyor. `r4` de tutmuyor (gride
   12–13 çift); `oltan` gride daha kötü, normal görüşte biraz daha sıkı.
2. **Seri rengi metin olarak.** Palet 3:1 (çizgi) için seçilmiş ama açıklama metni de aynı renkle
   yazılıyor; açık temada `s2` 3,94 < 4,5 (`s4`, `s6` 4,6'da sınırda). R2'nin altı görünümü yalnız `s1`
   kullandığı için orada görünmüyor; dört ve üstü seride çıkıyor.
3. **Açıklama yazısı dar sütunda taşmıyor, düşüyor.** 14 px Atkinson açıklama (`r4` 11 px mono)
   telefonda `r4` ile aynı sayıda açıklama bırakıyor (10/18), 640 × 360 çeyrekte bir eksik (13 ↔ 14).
   Asıl kayıp iki görünüşte de büyük: 402 px'te 8 SMA'nın yalnız biri adlı; 320 px'te Bollinger ve
   SMA20 açıklamasız. Okur adı olmayan çizgi görüyor.
4. **Çerçeve veriyle yarışıyor.** `oltan`'ın `muted` rengi yüzde karne yazısının rahat okunması için
   koyu (açık 14,35:1, metinle ΔE00 7,8); grafik çerçevesi (eksen yazıları, 70/30 kuralları) aynı rolü
   kullandığı için açık temada kurallar neredeyse siyah (14,35:1) ve RSI çizgisinden (`s1`, 6,42) iki kat
   karşıtlıklı; koyuda 9,77 ↔ 8,20. Yoğun ekranda hiyerarşi tersine dönüyor (yüzde istenen "işaret önde"
   ilkesinin grafikteki karşılığı "veri önde" olurdu). `r4`'te oran benzer (5,78 ↔ 2,74) ama mutlak
   değer düşük.
5. **Eksen çizgisi 1.4.11'in altında** (2,88 / 2,95). Küçük ama kayıtta tek satırla düzelir (R7'de).
6. **1 px mumlar** (gramer, iki görünüş): telefonda gövde = fitil; yön yalnız renkte. `oltan`'ın gri
   ayrımı iyi (13,2) ama açık temada iki koyu renk.
7. Eğik çizgili sıfır (Atkinson Hyperlegible Next) 13 px eksen yazısında yoğun görünüyor (gözlem);
   okuyucu sayı okumada hata yapmadı (son değer ve RSI sorularında yanlış yok denecek kadar az).

Kırılmayanlar: metin karşıtlığı (R2 görünümlerinde her öğe ≥ 5,67), en küçük yazı 13 px her boyda,
çakışma/taşma 0, mum ve histogram yönünün gri ayrımı (`r4`'ün üç katı), son değer kutusu (5,67–5,69).

## 4. R7 için açık sorular

1. **Seri paleti:** çok serili panel için parlaklık basamaklı bir palet mi (gri ayrım ≥ 10 en çok 3–4
   seri taşır), yoksa renge ek kanal mı (çizgi deseni/kalınlık, çizginin ucunda ad etiketi, dokununca
   vurgu)? Panel başına kaç seri sınır olmalı? `s1 = accent = focus` (koyu) çakışması kalsın mı?
2. **Açıklama yeri ve kaybı:** başlık satırına sığmayan açıklama düşürülmek yerine ikinci satıra mı
   iner, kısaltılır mı (SMA150 → 150), çizginin sağ ucuna mı taşınır, yoksa bir açıklama paneline mi
   (dokununca)? Kayıp bugün sessiz: en azından "+7" gibi bir işaret gerekmez mi?
3. **Seri renginin metinde kullanımı:** açıklama metni nötr renkte yazılıp yanına renk örneği (çizgi
   parçası) mı konmalı? Böylece palet 3:1 ile, metin 4,5:1 ile ayrı ayarlanır.
4. **Çerçeve rolü:** grafik çerçevesi (eksen yazısı, eşik kuralları, ızgara) için `muted`'dan ayrı bir
   rol (`chrome`/`rule`) mi? Hedef: çerçeve ≥ 4,5 (yazı) / ≥ 3 (eksen) ama veri çizgisinden düşük.
   Eksen çizgisini 3:1'e çıkarmak.
5. **Yoğun boy:** R7 çalışma alanında panel başına en küçük boy ne (320 px genişlik ve 640 × 360 çeyrek
   ölçüldü)? Bu boylarda 13–14 px yazı ölçeği korunacak mı, yoksa çalışma alanı için ayrı (daha sıkı)
   bir yazı ölçeği mi (`r4` 11 px'in bıraktığı yer bir açıklama)?
6. **Mum yoğunluğu:** 200 bar / ~320 px'te mum 1 px. Telefonda görünür bar sayısını kısıtlamak mı
   (gövde ≥ 3 px için ≲ 80 bar), çizgi/alan grafiğine düşmek mi, yoksa yön için biçim (içi boş/dolu) mu?
7. **Son değer etiketi kuralı:** v2'de etiket kapanışı değil üst bandı gösteriyor (gramer; iki
   görünüş). Çok katmanlı panelde hangi seri son değer etiketini alır, kaç etiket gösterilir, etiketler
   çakışırsa ne olur?
8. **Koyu tema masaüstü** iki görünüşte de en zayıf okuma hücresi (sonnet ~0,93–0,94); eşik kurallarının
   koyu temadaki parlaklığı bunun nedeni mi? İnsan testinde ayrıca bakılmalı.
9. **Ölçümün kendisi:** olgu soruları tavanda; R7 için daha zor görevler (iki seriyi kesişme noktasında
   ayırt et, renk körlüğü benzetimli görüntüde seri bul, kısma kopyasında panel sırası) ve insan testi.

## Sınırlar ve devam

- **Kullanım sınırı:** okuyucu koşusunun ortasında hesabın 5 saatlik sınırı doldu (11:50'de açılıyor);
  354 çağrı cevapsız. Haiku v3–v5 ve sonnet v4–v5 kapsamı eksik. Eksikler aynı kod adlarıyla tamamlanır
  (var olan kayıt yeniden çağrılmaz):
  `python3 rounds/R5/experiments/D/readers_d.py run --work <iş klasörü> --model haiku --reps 5` ve
  `--model sonnet --reps 3`, sonra `score`. Tahmini ek maliyet ≈ 0,3 $ (haiku) + ≈ 0,7 $ (sonnet).
- Olgu soruları kolay: tek görüntü, tam boy, düz okuma; okuma tavanda. Kısma/yarım boy kopyası
  (M16'nın birincil ölçüsü) bu görevin tarifinde yoktu, yapılmadı.
- Tek model ailesi (haiku, sonnet). Ajan insan değildir.
- `s8` ve iki sınama boyu benim eklediğim sınamalar; R2'nin altı görünümü değişmedi.
- Okuyucu yalnız Türkçe istemle (görünümlerde çevrilecek sözcük yok).

## Maliyet

| kalem | çağrı | $ |
|---|---:|---:|
| haiku okuyucu (puanlanan; 2 deneme çağrısı dahil, aynı kod adı) | 200 | 0,16 |
| sonnet okuyucu (puanlanan; 2 deneme dahil) | 214 | 2,74 |
| cevapsız (kullanım sınırı; haiku 280, sonnet 74) | 354 | 0,08 |
| **toplam** | 768 kayıt (+ 89 sınır yoklaması, aynı kayıtların üstüne, ≈ 0 $) | **2,98** |

## Dosyalar

`render.py` (çizim + sahne dökümü), `measure.py` (hesap), `summarize.py` → `computed.md`,
`readers_d.py` (okuyucu ve puanlama) → `readers.md`, `samples/`.
