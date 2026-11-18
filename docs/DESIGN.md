# Görsel dil (R5, M16; K7, K18, K39, K43, K46, K52, A31)

Bu belge Qapsule'ün görsel dilini anlatıyor: seçilen dil `oltan`, ikinci aday `zemre`, elenen
yönler ve neden elendikleri, kararın hangi ölçümle verildiği ve bu ölçümün sınırları. Sayılar
kaynaklardan aynen alındı:

- `rounds/R5/experiments/M16/RESULTS.md` ve `run/tables.md`: kör ölçüm, seçim.
- Aday tasarım notları: `r5/m16-{oltan,zemre,bertan,kivra}` dallarında
  `rounds/R5/experiments/M16/<ad>/DESIGN.md`.
- `rounds/R5/experiments/harness/glance/SPEC.md`: düzenek, §G ve Ek G1–G7.
- `rounds/R5/experiments/D/RESULTS.md`: yoğunluk sınaması.
- `rounds/R5/experiments/M18/RESULTS.md` ve `M20/RESULTS.md`: hikâye çizelgesi, açılmış kapsül.
- `rounds/R5/RESEARCH.md`.

Kaydın yapısı `docs/GRAMMAR.md` §9a'da, kapsülün dili (`marla`) `docs/LANGUAGE.md`'de.

> Ajan okuyucu insan değildir. Buradaki seçim bir ön elemedir (SPEC §G0). Son söz Arkin'in
> telefondaki karşılaştırmasında ve insan testi v3'tedir (`rounds/R5/human-test/`).

## Kısaca

| | `oltan` (seçilen) | `zemre` (ikinci) | `bertan` (elendi) | `kivra` (elendi) | `r4` (başvuru) |
|---|---|---|---|---|---|
| Yön | işaret önde / saat yüzeyi | zaman şeridi (alışılmışın dışı, K18) | sakin editoryal | yoğun ve keskin | bugünkü dil |
| Sert eşikler (§G7 + Ek G5) | geçti | geçti | **kaldı: renk (Ek G5)** | geçti | kaldı: 5 eşik |
| Tam boy okuma, aşağı kalmama kapısı (üst sınır ≥ 0,866) | 0,865 [0,847, 0,882] | 0,862 [0,844, 0,880] | 0,855 [0,835, 0,874] | **0,775 [0,740, 0,809]** | 0,866 |
| **Birincil: kısma kopyası** (F2:H1, %95) | **0,471 [0,445, 0,497]** | 0,279 [0,257, 0,301] | 0,345 [0,319, 0,370] | 0,243 [0,225, 0,260] | 0,318 [0,295, 0,343] |
| Eleştirmen BT | **2,15** | 1,24 | 1,30 | 1,79 | −1,20 |
| Paket (WASM brotli, `r4`'e göre) | +27,8 KB | +25,4 KB | +29,0 KB | +71,6 KB | 1 038,7 KB |

Bozuk tasarım birincil ölçüde 0,183 ile, BT'de −5,29 ile kesin son çıktı; koşu geçerli (§G8).
Maliyet 17,02 $.

## 1. Seçilen dil: `oltan`, işaret önde / saat yüzeyi

Seçim: `?look=oltan&face=oltan` (koyu varsayılan; `&theme=light`, `&lang=en`). Kayıt
`crates/qp-grammar/src/look.rs` içindeki `OLTAN`, yüz `crates/qp-grammar/src/faces/oltan.rs`.
Değerler `r5/m16-oltan` dalının ölçülen sürümünden alındı (`0abd4e5`). Bütünleştirmede değişen
değer olursa bu belge de güncellenir. Kabukta biçem kararı yok: `shells/web/style.css` ve
`arch-check.mjs` değişmedi, kabuk görünüşün değerlerini CSS özel özellikleri olarak alıyor
(GRAMMAR §9a).

### 1.1 İlke

Ekranın kahramanı anlam işaretidir: yüzün en büyük ve en yüksek karşıtlıklı öğesi o. Saat
kadranındaki bir komplikasyon gibi, göz önce işarete düşer; yönü, türü ve geçerliliği oradan
okur (K7). Cümle ikinci sıradadır ama rahat okunur. Bulanık ve yarım boy kopyada yüz bir renkli
işarete ve gri bir metin bloğuna iner; işaret tek başına ayırt edilir. Dayanak RESEARCH F9'dur
(tek vurgu: ekranda yalnız işaret en yüksek görsel ağırlıkta), M3 Expressive ve Live
Activities örnekleri de aynı yönü gösteriyor.

```
┌──────────────────────────────────────────────┐   yüz 390 × 226 CSS px (r4/vadrel/marla: 259)
│ ┌────────────┐ SYN20 son 20 günün en          │
│ │            │ yüksek fiyatını (227,08)       │  başlık: 19 px yarı kalın, ≤ 5 satır,
│ │   işaret   │ aştı · bugün, hâlâ geçerli     │  176 px sütunda
│ │  120 px    │                                │
│ └────────────┘                                │
│ Benzer 20 durumun 11'inde fiyat 10 işlem      │  karne: 16 px normal, muted, ≤ 2 satır
│ günü içinde yükseldi.                         │
└──────────────────────────────────────────────┘
```

Tek bakışın içeriği (K43): bir cümle, bir işaret ve bir ek bilgi (karne). Başlık satırı boş
kalıyor, enstrüman başlığın ilk sözcüğü. Güven işareti ("Sunucu doğruladı") sağ altta, rozet
sağ üstte. Başlık sütunu 176 px. `marla`'nın 1 062 başlığının en uzunu bu sütunda 5 satır
tutuyor, 328 karnenin hepsi ≤ 2 satır (ölçüldü). İşaret 128 px olunca sütun 168 px'e iniyor ve
başlık 6 satıra çıkıyordu; bu yüzden 120 px seçildi.

### 1.2 Yazı

| | |
|---|---|
| Aile | **Atkinson Hyperlegible Next** 2.001 (Braille Institute; düşük görüş için tasarlandı: açık biçimler, 0/O ve 1/l/I ayrımı, eğik çizgili sıfır). Kaynak `google/fonts` `ofl/atkinsonhyperlegiblenext/`. |
| Ağırlık | Başlık 600, geri kalan her şey 400. İki statik örnek (fontTools `instancer`, wght 400 ve 600). fontdue değişken yazı tipinin yalnız varsayılan örneğini çiziyor (RESEARCH §4.2-1). |
| Lisans | SIL OFL 1.1, **Ayrılmış Yazı Tipi Adı yok**. Değiştirilmiş sürüm aynı adla dağıtılabilir. Lisans metni depoda (`OFL-AtkinsonHyperlegibleNext.txt`). |
| Tablo rakamı | Ailenin varsayılan rakamları orantılı, fontdue da `tnum` uygulamıyor. Bu yüzden cmap'te 0–9 `tnum` gliflerine yeniden eşlendi: 10/10 rakam 632 birim. |
| Alt küme | Latin + Latin-1 + Latin Ext-A + birkaç işaret. `kern`, `locl`, `tnum` ve `liga` korunur; tuval `fontKerning = 'none'` ile çiziyor, `ref` ve `canvas` aynı yerleşimi veriyor. |
| Türkçe | ğĞşŞıİçÇöÖüÜâÂîÎûÛ, 18 harfin hepsi var. **`₺` yok**: `marla` metni `₺` kullanmıyor. Para birimi gerekirse "TL" yazımı ya da başka bir aileden tek glif gerekir (RESEARCH §4.1). |
| Dosyalar | Regular 36 768 B, SemiBold 36 804 B, toplam 73 572 B (WASM'a gömülü). |
| Gerekçe | RESEARCH §3.2 ve F10: hümanist, iç boşluğu açık aile bakışta daha hızlı okunuyor. 40 yaş üstü persona için de uygun. |

**Tip ölçeği** (boy px / satır yüksekliği, boyun katı olarak):

| rol | ne | `oltan` | `r4` |
|---|---|---|---|
| `headline` | başlık cümlesi | 19 / 1,27, **600** | 20 |
| `title` | sahnenin üst satırı, yüzün ek bilgisi | 15 / 1,3 | 16 |
| `body` | kart cümlesi, grafik notu | 16 / 1,375 | 12 |
| `label` | tarih etiketi, açıklama, karne satırı | 14 / 1,3 | 11 |
| `figure` | eksen değeri, son değer etiketi | 13 / 1,3 | 11 |
| `note` | kanıt etiketi | 13 / 1,3 | 9 |

Kabuk: metin 16 / 1,4, küçük 14, tablo 14, en küçük 13, güven işareti 13; bölüm başı 600.
**Hiçbir yerde 13 px'ten küçük yazı yok**; eşik 11, öneri 13.

### 1.3 Renk rolleri ve parlaklık merdiveni

Anlam renkleri ayrı **ışıklık basamaklarına** yerleşiyor (CIE L\*). Böylece renk tonu kaybolsa
da (gri tonlama, protan, deutan, tritan) ayrım kalıyor:

| tema | merdiven (L\*) |
|---|---|
| koyu | gri (yönsüz / artık geçerli değil / tutmadı) **47** < aşağı **59** < yukarı **80** < mürekkep (gerçekleşti, `tg5` plakası) **97** |
| açık | gri **57** > yukarı **43** > aşağı **28** > mürekkep **6** |

Merdivenin kuralları:

- **Gri, yön yok demek.** `neutral` = `invalid` = `closed`. Renk yalnız yön taşıyor. Yönsüz,
  artık geçerli olmayan ve tutmamış işaret aynı gri; aralarındaki farkı biçim (✗ ↔ çubuklu
  halka, rozet) ve sözcük taşıyor. Gerekçe: dört basamaktan fazlası ΔE00 ≥ 10 ile ne koyu temaya
  (L\* 40–100) ne açık temaya (L\* 0–58) sığıyor.
- **Açık temada aşağı koyu bordo.** Protanopide kırmızı kararıyor. Aşağı açık tutulsaydı protan
  benzetiminde koyu yeşille çakışıyordu (ΔE 7,8, denendi).
- **Geçerli olmayan başlık `muted` renginde yazılıyor.** `invalid` 3,75:1 ile metin eşiğinin
  altında kalırdı. Geçerlilik üç yoldan okunuyor: işaretin rengi ve rozeti, sözcük, cümlenin
  rengi. (R5/fix47, A46) Açılmış kapsül (`tesir`) de aynı kuralı izliyor
  (`open::tesir::NO_LONGER_HEADLINE`); önceden `invalid` ile yazıyordu (3,75 / 3,46, 19 px yarı
  kalın: 600'ü kalın saymayan tanımla eşik altı). Açık temada `muted` metne yakın (14,4 ↔ 18,7);
  ayrım orada işaret, rozet ve sözcükte.

| rol | koyu | açık |
|---|---|---|
| sayfa (kabuk zemini) | `#060608` | `#EEEEEA` (sayfa kenarı `#C8C8C2`) |
| yüz / karo (`bg`, `surface`, `card`) | `#121317` | `#FFFFFF` |
| metin, `valid`, mürekkep | `#F6F6F2` (17,1:1) | `#111216` (18,7:1) |
| `muted` (karne, geçersiz başlık) | `#B9BCC4` (9,8:1) | `#272A32` (14,4:1) |
| yukarı | `#4EE094` (11,0:1) | `#107642` (5,7:1) |
| aşağı | `#F6584A` (5,7:1) | `#840E16` (10,2:1) |
| gri: `neutral` = `invalid` = `closed` | `#6E7076` (3,75:1) | `#888A8E` (3,46:1) |
| `axis` (grafik eksen çizgisi; R5/fix47) | `#626670` (3,23:1; önce `#5C606A` 2,95) | `#8E9096` (3,19:1; önce `#96989E` 2,88) |
| `grid` (ızgara, süs) | `#24262C` (1,23:1) | `#E8E8E4` (1,23:1) |
| `chrome.control` (denetim sınırı; R5/fix47) | `#6E7076` (kabukta 4,09, kartta 3,75) | `#7E8084` (kabukta 3,40, kartta 3,96) |
| vurgu, odak, `s1` | `#8AAAFF` | `#2850DC` |
| `warn` (kanıt notu) | `#FFC850` (12,1:1) | `#965A00` (5,6:1) |

Oranlar yüz zeminine göre ve tasarımcının kendi hesabı (düzeneğin `gl/color.py` işlevleriyle).
Düzeneğin ölçtüğü değerler §1.6'da.

### 1.4 İşaretin yeri, boyu ve tabanları

- **Boy:** yüzde 120 px (`r4` 32), kartta 64, grafikte 32, boyu verilmemiş katmanda 28. Yeri
  `leading`: cümlenin solunda, başlık bloğuyla (5 × 24 px) aynı yükseklikte. İşaret ile cümle
  arası 14 px. Bu boyda ΔE00 için küçük işaret eşiği (15) uygulanmıyor, eşik 10.
- **İşaret tabanları** (GRAMMAR §9a, T3c; iki temada aynı):

| alan | değer | anlamı |
|---|---|---|
| `closed_alpha` | **255** | Kapanmış işaret solmuyor. `tg5`'in "tutmadı" izi `closed` renginde, en opak çizgisi bu tabanda; gerçekleşti plakası opak mürekkep. |
| `fresh_alpha_min` | **255** | Açık işaret de tazelikle solmuyor. |
| `plate_alpha_min` | **61** | Güven plakası taze bir işaretinkinin (0,24) altına inmiyor. |
| `grey_plate_alpha_max` (R5/fix47) | **49** koyu / **29** açık | Gri işaretin (kapanmış, artık geçerli değil, yönsüz) plakası bundan koyu olmuyor: gri çizgiler plakanın üstünde de ≥ 3:1 (3,09 / 3,08; 61'de 2,93 / 2,70). Renkli plakalar değişmedi. |

- **Tazelik solması yok, yaş cümleyle söyleniyor.** Alfabeler eski işareti saydamlıkla
  soldururdu (opaklık 0,3 + 0,7·tazelik); 0,3 opaklıkta hiçbir renk 3:1'e ulaşamaz. Gösterimdeki
  kapanmış kapsüllerin tazeliği 0,06–0,29 arasında ölçüldü. Taban olmadan koyu temada
  gerçekleşti plakası 2,9–5,1:1'e, tutmadı izi 1,5–1,9:1'e düşüyordu.
- **255'e nasıl gelindi.** ≥ 3:1 için yetecek en küçük açık taban koyu temada 214, açıkta 232
  (bağlayan rol `neutral`). İlk denemede 216/234 kullanıldı ve düzeneğin `compute`'u koyu temada
  renk kapısında kaldı: solmuş `down` griye kayıyordu (gri ΔE00 4,2). Kapanmış tabanı ilk
  denemede 154/196 idi; solmuş gerçekleşti plakası açık temada aşağı rengiyle çakıştı (gri ΔE00
  0,75). 255 merdiveni her tazelikte koruyor. Tazelik artık cümleden okunuyor ("bugün, hâlâ
  geçerli").
- Sahnenin öğe değerleri gerçek tazeliği taşımayı sürdürüyor; yalnız çizim solmuyor.
- **Sınama:** `glyph::outcome::tg5::tests::oltan_signs_keep_3_to_1_at_freshness_zero`. Tazelik
  0'da `tg5`'in her işareti ≥ 3:1: iki alfabe, iki tema, 6 tür × 3 yön × 4 evre.

### 1.5 Boşluk, yüzey, hareket, kabuk boyları

| | `oltan` | `r4` |
|---|---|---|
| Boşluk ölçeği (`--qp-space-0..5`) | 4, 8, 12, 16, 20, 28 | 4, 6, 8, 12, 16, 24 |
| Çizim kenarı (sol, sağ, üst, alt); panel arası | 8, 72, 28, 24; 10 | 8, 72, 34, 24; 8 |
| Yüzey | **kadran karoları**: köşe 14, telefon çerçevesi 32, rozet 10, çizgi 1, **gölge yok**. Koyu: karo `#121317` neredeyse siyah sayfada; açık: beyaz karo sıcak gri sayfada | 6, 18, 8; düz |
| Hareket | süre × 0,85, `cubic-bezier(0.2, 0, 0, 1)` (kısa ve kesin, HIG Motion), kabuk 140 ms | 1, `linear`, 0 |
| **Sekme** | **48 px** (iç boşluk 8, alt çizgi 3) | 0 (= içerik; ölçümde 33 px) |
| Düğme / seçim / alan | **44 px** | 0 |
| Alt oynatma çubuğu | 60 px (≥ denetim + 12) | 48 |
| Yüz metni iç boşluğu, en düşük yükseklik | 14 × 14, 72 | 14 × 12, 60 |

Kural: **dokunma hedefi ≥ 44 CSS px** (Apple HIG). Sekme bunun üstünde, 48.

### 1.6 Sert eşikler

Düzeneğin `compute` adımı (§G7 + Ek G5), M16 koşusu, 2 dil × 2 tema:

| eşik | kural | `oltan` | `r4` |
|---|---|---|---|
| Metin karşıtlığı (WCAG 1.4.3) | ≥ 4,5:1 (büyük metin 3:1), en kötü öğe | **6,33** | 2,75 (`karnesi yazı tura`, 11 px) |
| İşaret karşıtlığı (1.4.11, kapanmış dahil, A31) | ≥ 3:1, işaretin en opak ilkeli | **3,46** (tutmadı izi, açık) | 2,06 |
| Bütün öğeler (R5/fix47; yüz, akış, açılmış kapsül, kanıt, hikâye çizelgesi, kabuk) | metin 4,5:1 (600 kalın sayılmadan; ≥ 24 px 3:1), metin dışı 3:1 bitişik renge karşı; ızgara, ayraç ve saydam bant süs | eşik altı **0**; en kötü metin 5,67, metin dışı **3,07** (açık, seçili hikâye satırındaki kapanmış şerit); `rounds/R5/experiments/S/CONTRAST.md` | — |
| En küçük yazı | ≥ 11 CSS px | **13** | 10 |
| Dokunma hedefi | ≥ 44 CSS px; eşiğin altındaki hedef sayısı | **0** | 108 |
| Anlam renkleri ΔE00 | **beş koşulda** ≥ 10 (normal, gri, protan, deutan, tritan; CVD'de Viénot/Brettel ile Machado'nun küçüğü), işaret ≤ 16 px ise ≥ 15; tema başına | en kötü **10,53** (koyu `up ↔ valid`) | 0,50 (koyu `down ↔ invalid`) |
| K52 | JIT açık tahmini bağlayıcı; JIT kapalı kaydırmada uzun kare ≤ %10 | geçti | geçti |

**Bilerek paylaşılan roller** ölçüme çift olarak girmedi (normal görüşte ΔE00 < 5), raporda
listelendi:

- `valid` = `text`
- `invalid` = `closed` = `neutral`

Bu rollerin taşıdığı anlam biçimle ayrılıyor. Gri kopyada `dir` ve `valid` doğruluğu 1,00
çıktı.

Tasarımcının kendi tablosundan, tazelik 1'de en küçük ΔE00 değerleri:

- koyu: 10,5 (yukarı ↔ mürekkep, gri koşul);
- açık: 11,7 (yukarı ↔ aşağı, deutan).

Kayıttan sınanan alt küme `look::tests::oltan_meets_the_hard_thresholds`. Kapsadığı şeyler:

- boylar ve dokunma hedefleri;
- bütün kabuk metin çiftleri;
- kapanmış işaret tabanı;
- merdivenin her basamağı ≥ 3:1 ve gri koşulda ΔE00 ≥ 10.

### 1.7 K52 ve paket boyu

K52 ölçümü yüz akışında yapıldı (3 tekrarın ortancası, ms):

| | JIT açık: tik / yüz / sahne p95 | ilk kapsül | kaydırma kare p99 | JIT kapalı: tik / yüz / sahne p95 | kaydırma p99 | uzun kare payı |
|---|---|---:|---:|---|---:|---:|
| `oltan` | 1 / 1 / 2 | 200 | 19 | 1,6 / 2 / 7,4 | 19 | 0 |
| `r4` | 1 / 1 / 2 | 200 | 19 | 1,6 / 2 / 5,6 | 19 | 0 |

Paket boyu (`pkg/sizes.json`; yazı tipleri WASM'a gömülü):

| | wasm ham | gzip | brotli | yazı tipi dosyaları |
|---|---:|---:|---:|---:|
| `r4` | 5 342 433 | 1 566 385 | 1 038 710 | 135 580 (Plex Mono) |
| `oltan` | 5 499 388 | 1 639 746 | 1 066 543 (**+27 833**) | 209 152 (Plex Mono + 73 572 Atkinson) |

Not: `canvas` yolu açılışta gömülü **bütün** yazı tiplerini `FontFace` ile yüklüyor. Her yeni
görünüşün yazı tipi, görünüş seçilmese de herkesin açılışına ekleniyor. Öneri: yalnız seçili
görünüşün dosyaları yüklensin.

### 1.8 Neden `oltan`

§G9 kuralı adım adım uygulandı:

1. Sert eşiklerde `bertan` ve `r4` elendi. Hiçbir eşikten feragat edilmedi; her eşiği en az bir
   aday geçti.
2. Aşağı kalmama kapısında `kivra` elendi.
3. Birincil ölçüde `oltan` tek başına ilk grupta kaldı: aralığı `zemre`'ninkiyle örtüşmüyor.
   Sonuç: **kazanan, ayırt edildi.**

Üstünlüğün kaynağı soru başına puanlarda görülüyor (kısma kopyası):

| soru | `oltan` | `bertan` | `r4` | `zemre` | `kivra` | bozuk |
|---|---|---|---|---|---|---|
| `t_first` (işaret önce) | **0,83** | 0,54 | 0,61 | 0,52 | 0,58 | 0,45 |
| `t_dir` (yön) | **0,41** | 0,29 | 0,21 | 0,20 | 0,12 | 0,09 |
| `t_valid` (ham) | 0,16 | 0,25 | 0,15 | 0,13 | 0,03 | 0,01 |

Okuyucu ilk olarak işareti görüyor (K7) ve bulanık kopyada yönü seçebiliyor. Geçerlilik
(`t_valid`) bulanıkta her adayda zayıf, en iyisi `bertan`. Eleştirmenin tercihi de aynı yönde:
`oltan` en üstte (BT 2,15).

Zayıf yanı: akışta canlı olanı bulma (`feed/f_live`) `oltan`'da 0,65 [0,45, 0,81] (n 23).
Bozuk tasarım dışındaki en düşük değer bu (`r4` 0,92, `kivra` 1,00). Küçük örneklem, seçime
girmiyor, ama insan testinde bakılmalı.

## 2. İkinci aday: `zemre`, zaman şeridi

Seçim: `?look=zemre&face=zemre`. Görünüşü ve yüzü gösterimde seçilebilir kalıyor.

**Yön** (alışılmışın dışı, K18). Kaynak cümle: "İnsan grafiğe bakmaz; grafik insana haber
verir." Yüz iki soruyu ayrı ayrı cevaplıyor:

1. *Ne oldu, hangi yöne, hâlâ geçerli mi?* Büyük işaret (yüzde 56 px, kartta 48), kalın başlık
   ve soluk karne.
2. *Zaman neresinde?* Altta bir **ömür şeridi**. Penceresi saat ya da bar sayısı değil,
   iddianın kendi ömrü (`horizon_bars`): ömür + ömrün dörtte biri kadar öncesi. Sağ kenar
   *şimdi*, dikey mürekkep çizgi iddianın doğduğu an. Yer formülü
   `(doğum − pencere başı) / (pencere − 1)`: şimdi doğan 1,0, ömrünün sonunda 0,2, ömrünü aşmış
   olan 0,2'nin solunda.

Her yüzün sağ kenarı aynı x'te (390 − 64 px) ve son fiyat etiketi orada duruyor. Bu yüzden
akışta yukarıdan aşağı bir **"şimdi sütunu"** oluşuyor: doğum çizgisi bu sütuna yakın olan
iddia taze, sola kaymış olan ömrünü doldurmuş.

**Yeni kanallar ve nasıl sınandıkları:**

| kanal | sınama |
|---|---|
| Yaş (doğum çizgisinin şeritteki yeri) | Saf işlev `faces::zemre::birth_fraction`; birim testleri yaşla kesin azaldığını sınıyor. |
| Şimdi sütunu | Sahnelerde `life` panelinin sağ kenarı sabit. |
| Kapanmış işaret solmuyor | `closed_alpha` 255. |
| Açık işaret tazelikle solmayı sürdürüyor | Taban `fresh_alpha_min` koyu 172, açık 180; tazelik 0'da da ≥ 3:1. |

**Görsel kararlar:**

- **Kâğıt ve mürekkep.** Varsayılan açık tema: zemin `#f6f3ec`, metin `#1c1a17`. Kart çerçevesi
  ve gölge yok; yüzler günlüğün satırları gibi alt alta duruyor.
- **Anlam renkleri mavi / koyu zinober / mürekkep / gri**, yeşil–kırmızı değil. Gerekçe:
  yeşil–kırmızı çifti protan ve deutan okurda çöküyor (RESEARCH §2.2, §8.3). Bu renkler de
  ışıklık ekseninde aralıklı: açıkta mürekkep ≈ 10, zinober ≈ 22, mavi ≈ 41, gri ≈ 51; koyuda
  gri ≈ 47, zinober ≈ 59, mavi ≈ 73, mürekkep ≈ 92.
- **İki aile.** Atkinson Hyperlegible Next 400 ve 700; fiyat ve eksen için IBM Plex Mono.

**Neden ikinci.** `bertan` ve `kivra` elendikten sonra iki aday kaldı. `zemre` birincil ölçüde
`oltan`'ın arkasında, aralıkları ayrık. Bunun dışındaki ölçümleri:

- bütün sert eşikleri geçti (en kötü renk çifti açık temada `down ↔ neutral` 10,43 / 10);
- aşağı kalmama kapısını geçti (0,862);
- gri kopyada 1,000.

Dürüst not:

- **`zemre`'nin birincil puanı (0,279) başvuru `r4`'ün (0,318) altında.** `t_first` 0,52:
  büyük işaret ilk göze çarpan öğe olmaya yetmiyor, şerit ve kalın başlık onunla yarışıyor.
- Ömür şeridi "ne zaman" sorusunda (`face/when`) bir üstünlük getirmedi: `zemre` 0,69, `oltan`
  0,70, `r4` 0,69.
- Kanalın asıl sorusu ("iki yüzden hangisi daha yeni?") M16'da sorulmadı, ajan okuyucuyla
  ölçülmedi.
- Yüz başına en çok metin onda: ortalama 170 karakter, öteki adaylar 130–133.
- JIT kapalı kaydırmada %2,1 uzun kare var. Eşiğin (%10) altında, ama adaylar arasında sıfır
  olmayan tek değer.
- Renk kapısı sonucu Ek G5'e bağlı (§4.2): eski sahne tablosuyla `zemre` kalırdı.
- Eleştirmende `oltan|zemre` çiftinin çelişki oranı 0,33: eleştirmen bu ikisini ayırt edemedi.

## 3. Elenenler

### 3.1 `bertan`: sakin editoryal (Ek G5 renk kapısı)

**Yön:** gazete sayfası; bir cümle baskın, gerisi sessiz.

- Sıcak kâğıt zemin (açıkta yüz `#FFFDF8`), kutu yerine ince çizgiler, gölge yok, köşe 6.
- İşaret yüzde 64 px, kendi sütununda.
- Yazı tipi **Bertan Sans**: Source Sans 3'ün 400 ve 600 alt kümesi. Source Sans 3 "Source"
  Ayrılmış Yazı Tipi Adı taşıdığı için yeniden adlandırıldı (OFL madde 3). `₺` var, rakamlar
  zaten tablo genişliğinde.
- Ölçek: başlık 22, gövde 16, en küçük 13.

**Neden elendi.** Ek G5 renk kapısında kaldı. Kapı, anlam renklerini görünüş kaydının
rollerinden ölçüyor:

- açık tema: `up ↔ invalid` ve `up ↔ closed` gri koşulda 7,28 / 10;
- koyu tema: `up ↔ invalid`/`closed` deutan koşulda 9,35, `down ↔ invalid`/`closed` gri
  koşulda 7,51.

Yeşil yukarı ile gri "artık geçerli değil" aynı ışıklığa düşüyor.

**Açıkça yazılması gereken:** Ek G5 uygulanmasaydı `bertan` ikinci olurdu.

- Ek G5'ten önceki sahne tablosuyla (`colour_scene`) renk kapısını geçiyordu.
- Aşağı kalmama kapısını geçerdi: üst sınırı 0,874 ≥ 0,866.
- Birincil ölçüde `oltan` yine tek başına ilk grupta olurdu, kazanan değişmezdi.
- İlk iki **`oltan`, `bertan`** olurdu: `bertan` 0,345 > `zemre` 0,279. `bertan` `r4`'ün (0,318)
  üstünde kalan tek öteki aday ve bulanıkta geçerliliği en iyi okutan tasarım (`t_valid` 0,25).

İlk ikinin ikincisi bu yüzden Ek G5'e bağlı. Ek G5 ölçümden önce yazıldı ve kural uygulandı.
`valid` rolünün ve işaretin kullandığı rollerin renk çiftlerine girmesinin doğru kapsam olup
olmadığı açık bir karar (M16 RESULTS "Açık sorular 1"; bir karar satırı önerildi).

### 3.2 `kivra`: yoğun ve keskin (tam boy okuma alt sınırı)

**Yön:** terminal tablosu.

- Her kapsül sabit ızgarada kısa bir satır; yüz 390 × 161 px. Ekranda ≈ 4,5 yüz sığıyor
  (`r4`: ≈ 2,5).
- İşaret 40 px.
- Yazı tipi IBM Plex Sans Condensed 400 ve 600. Değiştirilmeden kullanıldı, bu yüzden "Plex"
  adı korundu.
- Ölçek: başlık 19, metin 14, en küçük 12. Köşe 2 px, satırlar arası 2 px.
- Anlam renkleri bir açıklık merdiveni. Eskilik, `oltan` gibi, saydamlıkla değil cümleyle
  söyleniyor ("11 işlem günü önce").

**Neden elendi.** Aşağı kalmama kapısında kaldı: tam boy okumada 0,775 [0,740, 0,809], üst
sınırı başvurunun 0,866'sının altında. Metnin söylediğini açıkça kaybediyor.

- Soru başına: `dir` 0,86, `valid` 0,82, `kind` 0,67, `when` 0,53. Öteki adaylarda `dir` ve
  `valid` 0,98–0,99, `when` 0,69–0,70.
- Gri kopyada `few_cases` durumunda 0/48: yoğun satırda az örnekli kapsülün yönü ve geçerliliği
  okunmuyor.
- Birincil ölçü 0,243, bozuk tasarımın hemen üstünde. Eşleştirilmiş fark +0,060 [0,040, 0,080].
- Paket en ağırı: +71,6 KB brotli. Değiştirilmemiş Condensed dosyaları 111 936 + 112 232 B.
- Eleştirmen yine de onu ikinci sıraya koydu (BT 1,79). Ajan eleştirmen yoğunluğu ve keskinliği
  beğeniyor, okuyucu yoğun satırda yanılıyor.

### 3.3 `r4`: başvuru (sert eşikler)

Bugünkü dil: IBM Plex Mono, işaret 32 px, rol boyları 9–20 px (ölçülen en küçük yazı 10 px). Her ölçümde yarıştı ve
kazanabilirdi; kazansaydı varsayılan değişmezdi.

- **Beş sert eşikte kaldı:**
  - metin karşıtlığı 2,75 ("karnesi yazı tura", 11 px);
  - işaret karşıtlığı 2,06;
  - en küçük yazı 10 px;
  - 108 dokunma hedefi < 44 px (sekmeler 33 px, güven yazısı 10 px);
  - renk: koyu temada `down ↔ invalid` 0,50.
- Tam boy okumada en yüksek (0,866), çünkü ajan küçük metni okuyor.
- Birincil ölçüde üçüncü (0,318); eleştirmen BT −1,20.
- `?look=r4` bayt eşit kalıyor. `v1` altınları değişmedi.

### 3.4 Görünüşün tek başına çözemediği

- Grafik derinliği M16 ölçümünde oturumda hâlâ `r4` görünümüyle kuruluyordu (LOOK_RECIPE,
  "kayıt dışı"). Yoğunluk sınaması (§5) `oltan`'ı R2 görünümlerine `qp render` ile doğrudan
  uyguladı.
- `oltan` görünüşü varsayılan yüz `vadrel` ile birlikte seçilirse, `vadrel` geçersiz başlığı
  `invalid` ile yazıyor (3,75:1, metin eşiğinin altında). Görünüş `face=oltan` ile birlikte
  tasarlandı.

## 4. Ölçüm yöntemi ve sınırları

### 4.1 Yöntem (SPEC §G, kurallar adaylardan önce commit edildi)

**Görüntü:**

- 402 × 874 CSS px, DPR 1 (iPhone 17 Pro'nun pt boyutu), büyütme yok.
- `canvas` yolu, hareket kapalı. TR + EN × açık + koyu.
- Aynı içerik: 8 yüz durumu ve akış; `--voice marla` (Ek G4-1).
- 216 görüntüde hedef kapsül ve oynatma adımı altı derlemede aynı.

**Kör okuyucu:**

- haiku, F ve H personaları. Tek görüntü, tek `Read`, kırpma ve büyütme yok, cevap başına en çok
  bir cümle.
- İstemlerde tasarım adı yok; `check_prompt` denetimi her istemde çalışıyor.
- Tek bakış dışı 18 kayıt puanlamadan çıkarıldı.

**Birincil ölçü, kısma kopyası:**

- Görüntünün Gauss bulanık (σ 4,5 CSS px) ve 0,5× küçültülmüş kopyası.
- σ, başvuru yüzlerinde ana cümlenin okunamadığı en küçük denenen değer: σ 2,5'te ajan cümleyi
  harfi harfine okuyordu (benzerlik 0,99); σ 4,5'te okuyamadı (0,18 / 0,42).
- Sorular: ilk göze çarpan öğe, yön, geçerlilik. `t_valid` sınıf dengeli puanlanıyor (Ek G3).
- 6 tekrar, F2:H1 ağırlık, %95 önyükleme.

**Seçim kuralı (§G9):**

1. Sert eşikler.
2. Tam boy okumada aşağı kalmama kapısı.
3. Birincil ölçü. Aralığı en iyiyle örtüşen herkes ilk grupta.
4. Eleştirmen BT (sonnet, ikili kör, iki sıra × 3 tekrar).
5. Tam boy okuma.

Tek aday kalmazsa "ayırt edilemedi" yazılır.

**Bozuk tasarım** her koşuda gizli adla yarışıyor. Hiyerarşisi ters, karşıtlığı düşük, işareti
minik ve soluk. Birincil ölçüde ve BT'de kesin son çıkmazsa koşu geçersiz sayılıyor.

**Gri kopya** (Ek G5-3) "biçimle de okunur" sorusuna bakıyor. Birincilden ayrı raporlanıyor.

### 4.2 Ek G4 ve Ek G5: ölçümden önce yapılan düzeltmeler

İkisi de M16 okumalarından **önce**, ayrı commit'lerle düzenekte yapıldı (Ek G4 `2ea1ab6`,
Ek G5 `85338e2`). İkisini de aday ajanların raporları tetikledi.

- **Ek G4 (uygulama hataları; kural değişmedi).** SPEC "bir aday ajanın raporu (M16
  alternatifi)" diyor. `bertan`'ın tasarım notu, ilk sürümünün önerilerinin Ek G4 ile
  karşılandığını yazıyor: tema içi renk tablosu, `--voice`, en opak çizgi. Üç düzeltme:
  1. `voice` adaylara aktarılmıyordu; okunan metin ürünün o anki varsayılanına bağlıydı.
  2. Anlam rengi çiftleri temaları karıştırıyordu: açık tema rengiyle koyu tema rengi
     karşılaştırılabiliyordu.
  3. İşaret rengi gövde değil alfa ağırlıklı en büyük alandı. Büyük ve saydam bir güven
     plakası gövdenin yerine geçebiliyordu (adayın raporunda ≈ 1,3:1).
- **Ek G5 (renk kapısı rollerden; K52 JIT'siz madde kapı).** SPEC yine "bir aday ajanın
  raporu" diyor ve adı vermiyor.
  - Sorun: Ek G4-3'ten sonra `siluet` alfabesinin metin renginde çizdiği kenar "gövde"
    sayılıyordu. Yönlü ve yönsüz işaretler aynı mürekkebe ölçülüyor, tablo yön rengini
    karşılaştırmıyordu.
  - Bu sorunu `zemre`'nin ve `oltan`'ın tasarım notları tarif ediyor. `zemre` kendi `compute`
    koşusunda bu yüzden renk kapısından kalıyordu; `oltan` "yeşil `up` kapıda ölçülmüyor" diye
    not düşmüştü.
  - Düzeltme: renk kapısı artık görünüş kaydının anlam rolleriyle ölçülüyor. Roller `up`,
    `down`, `neutral`, `valid`, `invalid`, `closed` ve işaretin kullandığı roller; tema başına,
    beş koşulda.
  - Sonuç: **Ek G5 iki adayın kapı sonucunu çevirdi.** `zemre` kalırken geçti, `bertan` geçerken
    kaldı.

### 4.3 Sınırlar

- **Ajan insan değildir.** Küçük metni insandan iyi okuyor, renk ve açıklık hiyerarşisini zayıf
  okuyor (RESEARCH §14.6). Tam boy okuma bu yüzden tasarımları ayırmıyor (bozuk tasarım bile
  0,837 alıyor) ve metin ağırlıklı tasarımı kayırıyor (R4'ün hatası, K46). Bu yüzden **birincil ölçü kısma kopyası**:
  metin okunmuyor, anlam görsel hiyerarşiyle taşınmalı.
- **Tek model ailesi.** Tasarımları ve okuyucuları aynı aile üretiyor; kendini kayırma (U4) yazar
  bilgisi gizlenerek azaltılıyor ama giderilemiyor. İkinci aile kullanılmadı (anahtar yok).
  haiku–sonnet uyumu 0,861 (n 576).
- **Bozuk tasarım sınaması düzeneğin kendisini sınıyor.** M16'da bozuk tasarım her adaydan
  eşleştirilmiş farkla ayrıldı. `kivra` ile fark en küçüktü: +0,060 [0,040, 0,080]. M18 ve
  M20'de bozuk kayıt `r4`'ün üstüne kurulduğu için `r4`'ten ayrılmadı. Sonraki ölçümde bozuk
  kayıt bir adayın üstüne kurulmalı.
- **Türkçe doğruluk İngilizceden düşük.** Kısma kopyasında her tasarımda, bozuk dahil, böyle.
  `oltan`: tr/açık 0,411, tr/koyu 0,422, en/açık 0,542, en/koyu 0,512. Okuyucu Türkçede daha
  sık "bilmiyorum" diyor. Bunun dil farkından mı model davranışından mı geldiği ayrılamadı.
  Sıralama dört birleşimin hepsinde aynı: `oltan` önde, bozuk sonda.
- **Mutlak doğruluk düşük** (birincil 0,18–0,47; birim şansı ≈ 0,36). Ölçü sıralama içindir.
- **Gri kopya** tam boyda metni okuduğu için üst sınıra dayanıyor (0,98–1,00). Anlamın yalnız
  biçimle taşındığını tek başına kanıtlamıyor.
- **Yetkisiz yüzde rakam sorusu** (`numbers`) bütün tasarımlarda şansa yakın (0,29–0,50). Bu
  tasarımdan bağımsız bir soru ya da metin sorunu.
- **Sapmalar.** Adaylar ayrı derlemelerle çekildi; içerik eşitliği denetlendi. Kanıt ve hareket
  ekranı M16'da çekilmedi.

## 5. Yoğunluk sınaması: `oltan` R2'nin altı görünümünde

Sınama `rounds/R5/experiments/D/RESULTS.md`'de (kabul ölçütü 5). `oltan`, R2'nin altı çok
panelli görünümüne `qp render` ile uygulandı. Başvuru aynı görünümlerin `r4` ile çizimi. Ayrıca
sekiz serili bir sınama görünümü (`s8`) kullanıldı. Boylar: telefon 402 × 600, masaüstü
1280 × 720, dar 320 × 480, çeyrek 640 × 360.

> **Okuyucu kapsamı tamamlandı (2026-11-18).** Kullanım sınırı yüzünden cevapsız kalan 354 çağrı aynı tohum ve
> istemlerle yeniden koşuldu; altı görünümde de kapsam tam (haiku 40/40, sonnet 24/24). Sonuç değişmedi: `oltan`
> 0,983 [0,971; 0,990], `r4` 0,983 [0,971; 0,990] — ayırt edilemedi (D/RESULTS ek, `readers.md`).

**Taşınanlar:**

- **Okuma.** Okuyucu `oltan` ile `r4`'ü ayırt edemedi:
  - haiku v0–v1: 0,996 [0,977, 0,999] ↔ 1,000 [0,984, 1,000];
  - sonnet v0–v3: 0,979 [0,955, 0,990] ↔ 0,986 [0,965, 0,995].

  Hiçbir kesimde `oltan` aşağıda değil. Olgu soruları tavanda; dilin farkını ölçecek kadar zor
  değiller.
- **Karşıtlık ve yazı boyu yoğun ekrana taşınıyor.**
  - R2 görünümlerinde bütün metin ≥ 4,5:1 (en kötü 5,67).
  - En küçük yazı her boyda 13 px. `r4` 11 px; son değer etiketi 3,08, kapanış 3,57, açıklama
    2,74 ile kalıyor.
  - 112 sahnede çakışma 0, taşma 0.
- **Yukarı ve aşağı gri koşulda da ayrılıyor.** Mum ve histogramda yukarı–aşağı gri ΔE00 13,2
  (açık) / 16,5 (koyu); `r4`'te 4,7 / 4,0. Parlaklık merdiveni grafiğe de taşınıyor.

**Kırılanlar:**

1. **Seri paleti s1–s8 gri koşulda ayrılmıyor.** 28 çiftin 22–23'ü ΔE00 < 10. Beş koşulun
   hepsinde ayrı kalan ilk seri sayısı açıkta 2, koyuda 1. Merdiven yalnız anlam renklerine
   uygulandı; seri paleti parlaklıkça düz. Açıklama seriyi yalnız renkle eşliyor, yani "renk
   tek başına anlam taşımaz" ilkesi çok serili panelde tutmuyor.
2. **Seri rengi metinde 4,5:1'i tutmuyor.** Açık temada `s2` 3,94; `s4` 4,64 ve `s6` 4,67
   sınırda. Bu yalnız dört ve üstü seride ortaya çıkıyor.
3. **Açıklama kaybı.** Gramer sığmayan açıklamayı sessizce düşürüyor. 18 açıklamadan kalanlar:
   telefonda 10, 320 px'te 5, çeyrekte 13 (`r4` 14).
4. **Hiyerarşi ters dönüyor.** `muted` yüzde karne için koyu seçildi (açık 14,35:1, metin
   18,72:1). Grafikte eksen yazıları ve 70/30 kuralları aynı rolü kullanıyor, bu yüzden veri
   çizgisinden daha yüksek karşıtlıkla çiziliyor: açıkta kural 14,35, RSI çizgisi 6,42. Yüzdeki
   "işaret önde" ilkesinin grafikteki karşılığı "veri önde" olurdu.
5. **Eksen çizgisi 1.4.11'in altında:** 2,88 / 2,95. `r4` de altında (2,25 / 2,71).
   (R5/fix47: `oltan`'da 3,19 / 3,23'e çıkarıldı; `r4` değişmedi, v1 altınları.)
6. **Mum telefonda 1 px.** Yön yalnız renkle okunuyor. Bu gramerden geliyor, iki görünüşte aynı.

**R7 için açık sorular** (D/RESULTS §4):

1. Seri paleti: parlaklık basamaklı palet mi (en çok 3–4 seri taşır), yoksa renge ek bir kanal
   mı (desen, kalınlık, uçta ad, dokununca vurgu)? Panel başına seri sınırı ne olmalı? Koyu
   temada `s1 = accent = focus` çakışması kalsın mı?
2. Açıklama: düşürmek yerine ikinci satır mı, kısaltma mı, çizginin ucu mu, panel mi? Kayıp
   bugün sessiz; "+7" gibi bir işaret gerekmez mi?
3. Açıklama metni nötr renkte yazılıp yanına bir renk örneği mi konmalı? Böylece palet 3:1'e,
   metin 4,5:1'e ayrı ayrı ayarlanır.
4. Grafik çerçevesi için `muted`'dan ayrı bir rol mü (`chrome`/`rule`)? Hedef: yazıda ≥ 4,5,
   eksende ≥ 3, ama veri çizgisinin altında. Eksen çizgisi 3:1'e çıkarılmalı.
5. Yoğun boyda en küçük panel ne olmalı? Çalışma alanı için daha sıkı bir yazı ölçeği gerekir
   mi?
6. Mum yoğunluğu: görünür barları kısıtlamak mı (gövde ≥ 3 px için ≲ 80 bar), çizgi grafiğe
   düşmek mi, içi boş/dolu biçim mi?
7. Çok katmanlı panelde son değer etiketini hangi seri alır? v2'de etiket kapanışı değil üst
   bandı gösteriyor.
8. Masaüstü koyu tema iki görünüşte de en zayıf okuma hücresi (~0,93–0,94). İnsan testinde
   bakılmalı.
9. Ölçümün kendisi: daha zor görevler ve insan testi gerekiyor.

## 6. Hikâye çizelgesi ve açılmış kapsül: görsel ilkeler

İki konu da `look=r4` ile ölçüldü (M18, M20). Seçilen görünüşle renk ve karşıtlık kapısı yeniden
ölçülmeli. M18 ve M20'de metin karşıtlığı, metin dışı karşıtlık, dokunma ve renk eşiklerini
`r4` dahil hiçbir tasarım geçemedi, bu yüzden uygulanmadı (E6). Ortak nedenler `r4`'ün kendi
öğeleri:

- grafiğin `up` renkli son fiyat yazısı (3,57:1);
- kabuk düğmeleri (< 44 px);
- gri koşulda `up ↔ closed` (0,13).

**Hikâye çizelgesi** (M18; varsayılan `pavka`, ikinci `korda`; `?story=`). Grafik derinliğinde
görünen aralıktaki bütün hikâyeler gösteriliyor.

- **Şerit:** her hikâye doğum barından kapanış barına (açıksa son bara) bir yatay şerit.
  `pavka` fiyatı üstte ince bir bağlam şeridine indiriyor, çerçeveyi hikâyelere veriyor.
- **Sıralama:** canlılar önce, kendi şeritlerinde, ok başıyla bitiyor. Kapanmışlar altta ve
  soluk.
- **Solukluk saydamlık değil.** Kapanmışlar `closed` rolüyle çiziliyor, ≥ 3:1. `korda`'da
  solukluk çizgi kalınlığıyla veriliyor, renk tam opak kalıyor.
- **Renk tek başına anlam taşımıyor.** Biçimler:

  | anlam | biçim |
  |---|---|
  | yön (doğumda) | ▲ yukarı, ▼ aşağı, ◆ yönsüz |
  | sonuç (şeridin ucunda) | ● gerçekleşti, ✕ tutmadı, \| ya da ○ süresi doldu |
  | yazı tura (K53) | kesik ya da noktalı şerit |
  | artık geçerli değil | ince şerit ve içi boş ok (`korda`'da çift ray) |

- **Seçili hikâye:** odak rengi, kalın şerit ve fiyata çıkan ince bir kılavuz. Kanıt geometrisi
  yalnız onun.
- **Özet satırı lejant işi görüyor:** `205 hikâye ▶ 1 canlı ● 83 tuttu ✕ 121 tutmadı`.
- **Kalabalık:** önce şeritler inceliyor, sonra kümeleniyor (`+21` çipleri).
- **Kapanmışlar** varsayılan görünür ve kapatılabilir (K54).
- **Ölçüm:** birincil `chart` doğruluğu `pavka` 0,987, `korda` 0,928, `r4` 0,411. Kapanmışlar
  görünürken canlıyı bulma `pavka` 1,00, `korda` 0,96, `selen` 0,52.
- **Not:** JIT kapalı sürüklemede `pavka`'nın kare p99'u 27–28 ms (`korda` 21). Kullanılabilir
  ama daha ağır; insan testinde gözlenmeli.

**Açılmış kapsül** (M20; varsayılan `tesir`, ikinci `nobra`; `?open=`).

- **`tesir`: tek, kaydırılabilir bir sütun.** Sıra yukarıdan aşağı iddia → dayanak → kim
  doğruladı → karne. Her blok bir soruyu cevaplıyor.
  - İddia, yüzdeki cümlenin aynısı: açmak okurun bildiği hiçbir şeyi değiştirmiyor.
  - Dayanak, eşiği adıyla ve değeriyle veriyor; yanında kanıtın küçük resmi var.
  - "Kim doğruladı" satırının rengi `accent` / `valid` / `muted` / `warn`, ama anlamı sözcük
    taşıyor. Satırın tamamı dokunma hedefi.
  - Yetkisiz okurda dayanak şablonlu bir satıra dönüşüyor ("Kilitli: flow/SYN03 yetkisi").
    Verisi olmayan okur sahne yerine metin sütunu görüyor; rakam yok.
- **`nobra`: soru kartları.** Her kartta soru satırı ve tek satırlık cevap var; ayrıntı dokununca
  açılıyor. Yetkisiz okur için ayrı bir "Neden rakam göremiyorum?" kartı var.
- **Ölçüm:**
  - Birincil `tesir` 0,968, `nobra` 0,848, `r4` 0,732.
  - Üç soruya da tek dokunuşla ulaşılıyor. `r4`'te "neden rakam yok" iki dokunuş.
  - Yetkisizde rakam: hepsinde 0.
- **Karar bir anahtara bağlı.** Kararı `e_who` sorusunun sıkı anahtarı belirliyor. Gevşek
  anahtarla (sunucu ∪ cihaz) varsayılan `nobra` olurdu (M20 "Açık soru 1").

## 7. Telefonda nasıl karşılaştırılır

`scripts/demo.sh --gallery --lan` komutu sentetik dünyayla gösterimi ve galeriyi açar. Galerinin
adresi `/gallery.html`; telefondan açılabilir adres ekrana yazılır. `--lan` yerine
`--https-host` da kullanılabilir.

Galeride her konunun adayları alt alta, aynı içerikle duruyor. Konular dil, görünüş, hikâye
çizelgesi, akış, hareket ve açılmış kapsül; hepsi aynı oynatma adımında, 402 px genişlikte.
Görünüş konusunda her aday üç çekimle gösteriliyor: aynı yüz, açılmış kapsül ve grafik. Bakılacak şeyler:

- İşaret ilk göze çarpan şey mi?
- Yazı rahat okunuyor mu?
- Açık ve koyu temada renkler ayrılıyor mu?

Her adayın altında "bunu seçerdim" düğmesi ve kısa bir not alanı var (konu başına tek seçim).
Sonuç en altta tek dokunuşla kopyalanıyor. Bir seçimi paylaşmak için `?pick=look.oltan,...`
kullanılabilir.

Gösterimin kendisinde görünüş Ayarlar → Görünüş'ten ya da adresle seçiliyor:

- `?look=oltan&face=oltan`
- `?look=zemre&face=zemre`
- `?look=r4` (bugünkü)

Bunlara `&theme=light` ve `&lang=en` eklenebilir. Hareket ve dokunma ancak gösterimde
denenebiliyor; galeri son kareyi gösteriyor.
