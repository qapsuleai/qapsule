# R5 — Araştırma: yeni nesil kapsül arayüzü (T1)

Tarih: 2026-11-17. Kabul ölçütü 1 (`rounds/R5/GOAL.md` Kapsam 1). Amaç rakiplerin ne yaptığını değil,
**neyin işe yaradığına dair kanıtı** toplamak ve her fikri M16–M21 düzeneğine girecek biçimde
ölçülebilir kılmak.

## 0. Yöntem ve kaynak durumu

- Kaynaklar WebSearch ve WebFetch ile bu oturumda arandı. Her kaynağın yanında durumu yazılıdır:
  - **[A]** sayfa bu oturumda açılıp okundu (birincil metin ya da resmi belge);
  - **[Ö]** yalnızca arama sonucundaki özet/abstract okundu; tam metin açılmadı. Bu kaynaklardaki
    sayılar abstract düzeyindedir; düzenekte eşik olarak kullanılmadan önce makaleden doğrulanmalı;
  - **[Y]** yerel deneme (scratchpad'de; fontTools 4.68, fontdue 0.9.5 — depodaki sürüm).
- Rakip uygulama incelemesi yapılmadı (görev tanımı gereği).
- Açılamayan/şüpheli: Apple HIG sayfaları JavaScript ile yüklendiği için WebFetch boş döndü; aynı
  içerik Apple'ın belge JSON uç noktasından (`developer.apple.com/tutorials/data/design/human-interface-guidelines/<sayfa>.json`)
  indirilip okundu [A]. Wickens–Dixon "0,70 eşiği"nin yılı kaynaklarda tutarsız (2005/2007) [Ö].
  `prefers-reduced-motion` kullanım oranı için güvenilir birincil veri bulunamadı (blog tahminleri
  %5–30; kullanılmadı).

---

## 1. Özet / kart dili (mobil finans ve haber)

Bulgular:

1. **Başlık önden yüklenir.** NN/g göz izleme çalışmaları: okur listelerde ilk birkaç kelimeye
   bakar; "ilk 11 karakter / ilk 2 kelime" bir kural-ı şeyhtir, kesin eşik değildir. Konu kelimesi
   başa gelmelidir (BBC örnekleri: "Italy buries…"). — NN/g "What Is Microcontent?"
   https://www.nngroup.com/articles/microcontent-how-to-write-headlines-page-titles-and-subject-lines/ (1998, güncel) [Ö];
   NN/g "World's best headlines: BBC" https://www.nngroup.com/articles/worlds-best-headlines-bbc-news/ [Ö].
   → Kapsül başlığı **nesne + olay** ile başlamalı ("148,90 dibi kırıldı…"), "Önemli bir…" gibi
   içeriksiz bir açılışla değil (K49 ile aynı yön).
2. **Kısa dikkat penceresi.** Saat kullanımında saat/komplikasyon bakışları tüm etkileşimlerin yarısı,
   ortalama ≈ 3,9 s (yazarın basın açıklaması; makalede doğrulanmadı); bildirimlere bakmak
   etkileşimlerin %17'si. — Pizza, Brown, McMillan, Lampinen, "Smartwatch in vivo", CHI 2016,
   https://doi.org/10.1145/2858036.2858522 [Ö]. Fitness izleyicileri için "5 saniyen var" tasarım
   çalışması: https://research.utwente.nl/en/publications/you-have-5-seconds-designing-glanceable-feedback-for-physical-act/ [Ö].
   → GOAL'daki "beş saniye" personası literatürle uyumlu; düzenekte **gösterim süresi**
   kavramı (aşağıda §14) bunun ajan karşılığıdır.
3. **Apple Live Activities kılavuzu** (glanceable yüzeyler için en somut resmi yazılı ölçüt) [A]:
   "Focus on important information that people need to see at a glance… doesn't need to display
   everything"; "Use large, heavier-weight text — a medium weight or higher. Use small text
   sparingly"; "Alert people only for essential updates"; tanımlı başı ve sonu olan, ≤ 8 saatlik
   olaylar; olay bitince hemen sonlandır, kilit ekranında en çok 4 saat kalır, çoğu durumda 15–30 dk
   özet yeter. — https://developer.apple.com/design/human-interface-guidelines/live-activities (2026 sürümü, JSON'dan okundu).
4. **"Expressive" tasarımın ölçülmüş kazancı ve sınırı.** Google M3 Expressive: 46 çalışma, 18 000+
   katılımcı; göz izleme gözlükleriyle 10 uygulama, rastgele sıra; kilit öğeyi "4 kata kadar daha
   hızlı" fark etme (e-posta "Gönder" düğmesi); 45+ yaşta sabitlenme süresi farkı kapandı. **Aynı
   kaynakta sınırlar:** "güçlü bir azınlık daha sakin sürümleri tercih etti"; bankacılık benzeri
   arayüzler yerleşik kalıplara uymalı; dağınık yerleşim ve etiketsiz eylemler kullanılabilirliği
   düşürdü; alışılmamışlık tercih ve kullanılabilirliği düşürdü. Google'ın kendi çalışması,
   bağımsız doğrulanmadı. — https://design.google/library/expressive-material-design-google-research (2025, IO25) [A].
   → Çıkarım: **vurgu (boyut, renk, biçim) yalnız tek bir anahtar öğede** işe yarıyor; "her şey
   ifadeli" değil. Bu, K7 (işaret en hızlı okunan öğe) ile örtüşür.
5. **Kart mı liste mi:** NN/g kartları heterojen içeriği taramaya uygun, homojen öğeleri
   karşılaştırmaya daha az uygun bulur (https://www.nngroup.com/articles/cards-component/ [Ö]).
   Akış homojen kapsüllerden oluşur; kart çerçevesi zorunlu değildir (GOAL'ın "alışılmışın dışı"
   serbestliğiyle uyumlu).

## 2. Veri görselleştirmede anlam kodlama

1. **Önceden dikkat çeken özellikler** (renk tonu, yönelim, boyut, eğrilik, yoğunluk, hareket)
   paralel ve hızlı işlenir; birleşik hedefler (ör. "kırmızı **ve** kare") genelde sıçramaz; özellikler
   arasında girişim olur, görsel sistem bazı özellikleri önceler (ton > biçim). — Healey, Booth, Enns,
   ACM TOMACS 1995, https://healey.csc.ncsu.edu/publications/15885-harnessing-preattentive-processes-for-multivariate-data-visualization [Ö].
   → **Tek anlam = tek ayırt edici kanal + yedek kanal.** Yön (yukarı/aşağı) biçimle (üçgen yönü),
   sonuç (tuttu/tutmadı) biçimle (dolu/boş, ✓/✗), geçerlilik parlaklık/doygunlukla; aynı anlamı
   iki kanalda yineleyen kodlama renk körü okurda da çalışır. Birleşik arama gerektiren kodlama
   ("yeşil boş üçgen = tutmadı yukarı") tek bakışta okunmaz.
2. **Renk tek başına anlam taşımaz:** Apple HIG — "Offer visual indicators, like distinct shapes or
   icons, in addition to color…"; red-green ve blue-orange çiftleri sorunlu. —
   https://developer.apple.com/design/human-interface-guidelines/accessibility [A]. WCAG 2.2 SC 1.4.1.
3. **İşaret boyutu renk ayrımını değiştirir.** Algılanan renk farkı işaret boyutu küçüldükçe azalır;
   uzun ince işaretler (çubuk, çizgi) noktalardan daha ayırt edilebilir. Büyük, düzgün alanlarda
   ölçülen ΔE eşikleri küçük işaretlere aktarılamaz. — Szafir, "Modeling Color Difference for
   Visualization Design", IEEE VIS 2017 / TVCG 24(1), https://cmci.colorado.edu/visualab/VisColors/ [Ö].
   → Kapanmış işaret küçük ve solukken iki kez kaybeder (karşıtlık + boyut). ΔE eşiği işaret
   boyutuna göre seçilmeli (§8).
4. **Hareket çevrede renkten ve biçimden güçlüdür.** Basit hareketli simgeler (moticons) bildirimde
   renk ve biçim kodlarından iyi, özellikle çevresel görüşte. — Bartram, Ware, Calvert, IJHCS 58,
   2003, https://www.interruptions.net/literature/Bartram-IJHCS03.pdf [Ö]. → "Canlı" kapsülün
   işaretinde küçük, yavaş bir hareket aday kanaldır (M21); ama dikkat dağıtma maliyeti de aynı
   makalenin konusudur → yalnız durum değişiminde ve kısa.
5. **Makine okur için konum en güvenilir kanal.** EncQA vb.: eksene hizalı konum en doğru, alan ve
   açıklık (lightness) en zayıf okunan kodlama VLM'lerde. — Keith-Norambuena, "Toward a Machine
   Bertin", arXiv 2602.01527, 2026, https://arxiv.org/html/2602.01527v1 [A].

## 3. Tek bakış (glanceable) okunabilirlik

1. **Saatte grafik karşılaştırma eşikleri:** çubuk ≈ 245–285 ms, halka ≈ 159 ms, radyal çubuk ≈ 1548 ms
   (iki veri noktası karşılaştırma). — Blascheck, Besançon, Bezerianos, Lee, Isenberg, "Glanceable
   Visualization", IEEE InfoVis 2018, https://hal.archives-ouvertes.fr/hal-01851306 [Ö]. 65+ yaş
   tekrarında farklar veri boyutuyla büyüyor, 75+ yaşta belirgin: https://arxiv.org/abs/2403.12343 (2024) [Ö].
   → Basit, hizalı biçimler çok hızlı; "yaratıcı" radyal biçimler 5–6 kat yavaş. 40+ persona için
   yaş etkisi gerçek.
2. **Bakışlık okunabilirlik yazı tipine bağlı.** Hümanist sans, kare grotesk'e göre toplam bakış
   süresini %10,6 azalttı (erkek alt grubu, araç ekranı). — Reimer ve ark., Ergonomics 2014 [Ö];
   yaş × yazı tipi × boyut × kutupluluk etkileşimi: Dobres ve ark., Ergonomics 59(10), 2016 [Ö];
   sekiz sans-serif ailenin bakışlık eşikleri: Sawyer, Dobres, Chahine, Reimer, "The Great Typography
   Bake-Off", Ergonomics 2020, https://readabilitymatters.org/articles/the-great-typography-bake-off [Ö].
   MIT AgeLab proje sayfası: https://agelab.mit.edu/transportation-and-livable-communities/projects/glance-based-legibility [Ö].
   → Tek aralıklı (kare, geniş) yazı başlıkta zayıf aday; açık iç boşluklu (open aperture), hümanist
   eğilimli aileler öne çıkar (Atkinson Hyperlegible Next, IBM Plex Sans, Inter, Source Sans 3).
3. **Okuma hızında yazı tipi etkisi kişiseldir.** Kişinin en hızlı ve en yavaş yazı tipi arasında %35
   fark, anlama korunarak; **tercih hızı öngörmüyor.** — Wallace ve ark., ACM TOCHI 2022,
   https://jeffhuang.com/papers/Readability_TOCHI22.pdf [Ö]. → (a) Eleştirmenin "beğenisi" okunabilirlik
   ölçüsü değildir; (b) tek bir "en iyi aile" iddiası zayıf — yazı ailesi ayardan değiştirilebilir
   tasarım kaydı olmalı (T3 ile uyumlu).
4. **Mobilde dikkat ekranın üstünde.** Mobil arayüzlerde göz izleme verisi üst bölgeye güçlü eğilim
   gösterir; klasik masaüstü dikkat modelleri mobilde farklı. — Leiva ve ark., "Understanding Visual
   Saliency in Mobile User Interfaces", MobileHCI 2020, https://userinterfaces.aalto.fi/mobile-saliency [Ö]
   (veri kümesi açık). Tasarım için önem modeli: Fosco ve ark. UMSI, arXiv 2008.02912 (2020) [Ö].
   → Deterministik, ajan dışı bir "ilk bakış" vekili olarak saliency/önem haritası düzeneğe girebilir.
5. **Kalabalık ölçüsü.** Feature Congestion / Subband Entropy / Edge Density görüntüden hesaplanan
   kalabalık ölçüleridir; kalabalık arttıkça yeni bir öğenin dikkat çekmesi zorlaşır. — Rosenholtz,
   Li, Nakano, Journal of Vision 7(2):17, 2007, https://dspace.mit.edu/handle/1721.1/37593 [Ö].
   → "Metin yoğunluğu" ölçüsünün görüntü tabanlı, modelsiz tamamlayıcısı.

## 4. Yazı tipleri (açık lisanslı adaylar)

### 4.1 Ölçüm [Y]

Dosyalar `github.com/google/fonts` deposunun `ofl/<aile>/` klasöründen 2026-11-17'de indirildi.
Türkçe tamlık: ğĞşŞıİçÇöÖüÜâÂîÎûÛ (18 harf) cmap'te var mı. `tnum`: GSUB'da özellik var mı.
"Varsayılan rakam tablo mu": cmap'teki 0–9 genişlikleri eşit mi (fontdue için belirleyici, aşağıya bak).
woff2: fontTools ile tam dosyanın woff2'ye çevrilmiş boyutu. "Statik alt küme": değişken dosyadan tek
ağırlık örneği + Latin/Latin-1/Latin Ext-A + gerekli simgeler, `kern`/`locl` korunarak, rakamlar
cmap'te `tnum` glifine yeniden eşlenmiş (fontdue'ya hazır biçim).

| Aile | Lisans (dosya) | RFN | Değişken eksenler (varsayılan) | Türkçe 18/18 | ₺ | Varsayılan rakam tablo | `tnum` | Tam ttf | Tam woff2 | Statik alt küme (1 ağırlık) |
|---|---|---|---|---|---|---|---|---|---|---|
| Inter 4.001 | [OFL](https://github.com/google/fonts/blob/main/ofl/inter/OFL.txt) | yok | opsz 14–32 (14), wght 100–900 (400) | ✓ | ✓ | ✗ (orantılı) | ✓ | 876 576 | 350 276 | 38 840 (400), 38 972 (600) |
| IBM Plex Sans 3.201 | [OFL](https://github.com/google/fonts/blob/main/ofl/ibmplexsans/OFL.txt) | **"Plex"** | wght 100–700 (400), wdth 75–100 | ✓ | ✓ | **✓** | — (gerek yok) | 537 244 | 229 956 | 56 296 (400), 56 344 (600) |
| Atkinson Hyperlegible Next 2.001 | [OFL](https://github.com/google/fonts/blob/main/ofl/atkinsonhyperlegiblenext/OFL.txt) | yok | wght 200–800 (400) | ✓ | **✗** | ✗ | ✓ | 114 552 | 48 116 | 35 648 (400), 35 620 (700) |
| Geist 1.800 | [OFL](https://github.com/google/fonts/blob/main/ofl/geist/OFL.txt) | yok | wght 100–900 (400) | ✓ | **✗** | ✗ | ✓ | 169 056 | 69 688 | 29 308 (400), 29 352 (600) |
| Source Sans 3 3.052 | [OFL](https://github.com/google/fonts/blob/main/ofl/sourcesans3/OFL.txt) | **"Source"** | wght 200–900 (**200**) | ✓ | ✓ | ✓ | — (pnum var) | 646 340 | 169 480 | ölçülmedi |
| Figtree 2.002 | [OFL](https://github.com/google/fonts/blob/main/ofl/figtree/OFL.txt) | yok | wght 300–900 (**300**) | ✓ | **✗** (‰ da yok) | ✗ | ✓ | 62 712 | 27 668 | 26 908 (400), 26 892 (600) |
| Public Sans 2.001 | [OFL](https://github.com/google/fonts/blob/main/ofl/publicsans/OFL.txt) | yok | wght 100–900 (**100**) | ✓ | ✓ | ✗ | ✓ | 103 316 | 42 016 | 30 984 (400), 31 064 (600) |
| Noto Sans 2.015 | [OFL](https://github.com/google/fonts/blob/main/ofl/notosans/OFL.txt) | yok | wght 100–900 (400), wdth 62,5–100 | ✓ | ✓ | ✓ | ✓ | 2 049 096 | 888 888 | ölçülmedi |
| Roboto Flex 3.200 | [OFL](https://github.com/google/fonts/blob/main/ofl/robotoflex/OFL.txt) | yok | 13 eksen (opsz, wght 100–1000, GRAD, wdth…) | ✓ | ✓ | ✓ | — | 1 787 292 | 791 864 | ölçülmedi |
| Space Grotesk 2.000 | [OFL](https://github.com/google/fonts/blob/main/ofl/spacegrotesk/OFL.txt) | yok | wght 300–700 (**300**) | ✓ | ✓ | ✗ | ✓ | 136 676 | 49 248 | ölçülmedi |
| Recursive 1.085 | [OFL](https://github.com/google/fonts/blob/main/ofl/recursive/OFL.txt) | yok | MONO, CASL, wght 300–1000 (**300**), slnt, CRSV | ✓ | ✓ | ✓ | — | 2 379 132 | 718 604 | ölçülmedi |
| JetBrains Mono 2.211 | [OFL](https://github.com/google/fonts/blob/main/ofl/jetbrainsmono/OFL.txt) | yok | wght 100–800 (400) | ✓ | **✗** | ✓ (tek aralık) | — | 187 208 | 71 592 | ölçülmedi |
| Geist Mono 1.701 | [OFL](https://github.com/google/fonts/blob/main/ofl/geistmono/OFL.txt) | yok | wght 100–900 (400) | ✓ | **✗** | ✓ | — | 171 948 | 70 980 | ölçülmedi |
| *IBM Plex Mono Regular 2.3 (bugünkü, `crates/qp-render/assets/fonts/`)* | OFL (depoda) | "Plex" | statik | ✓ | ✓ | ✓ | — | 135 580 | 39 184 | — |

Boyutlar bayttır. Hiçbir adayda Türkçe harf eksiği yok; ₺ eksik olanlarda para birimi "TL" yazımına ya
da ikinci bir aileden tek glif almaya muhtaçtır (CLDR `tr` sembolü ₺; §10).

### 4.2 fontdue ile uygunluk — ölçülmüş tuzaklar [Y]

1. **fontdue 0.9.5 değişken yazı tipini yalnız varsayılan örnekle çizer.** fontdue, ttf-parser 0.25'i
   `variable-fonts` özelliği olmadan derler ve değişim API'si sunmaz. Deneme: Inter değişken dosyası
   ile fontTools'la üretilmiş Inter wght=400 statik örneğinin 64 px "H" mürekkep toplamı aynı
   (173 639); wght=600 örneği 236 799. Figtree değişken (varsayılan 300) "H" = 119 286, statik 400 =
   155 372. → **Varsayılanı 100–300 olan aileler (Figtree, Public Sans, Source Sans 3, Space Grotesk,
   Recursive) değişken dosyayla fontdue'da ince çizilir.** Çekirdek yolu için her ağırlık ayrı statik
   örnek olmalı; "tek değişken dosya, çok ağırlık" kazancı fontdue yolunda yoktur.
2. **fontdue şekillendirme yapmaz** (cmap → glif); `tnum` gibi OpenType özellikleri uygulanmaz.
   Varsayılan rakamları orantılı olan ailelerde (Inter, Geist, Atkinson Next, Figtree, Public Sans,
   Space Grotesk) rakamlar fontdue'da hizasız çıkar (Inter: "1" 26,0 px, "0" 40,4 px @64 px). Çözüm
   denendi: statik örnekte cmap'te 0–9'u `tnum` glifine yeniden eşlemek → 10/10 rakam eşit genişlik
   (Inter 41,5 px). IBM Plex Sans, Noto Sans, Source Sans 3, Roboto Flex'te varsayılan rakamlar zaten
   tablo genişliğinde.
3. **Aralama (kerning) farkı:** fontdue yalnız eski `kern` tablosunu okur
   (`fontdue-0.9.5/src/font.rs:253`); adayların **hiçbirinde** `kern` tablosu yok, hepsinde aralama
   GPOS'ta. Tarayıcı `canvas` metni GPOS aralamasını varsayılan uygular → `ref` (fontdue) ile `canvas`
   arasında metin genişliği farkı doğar. Önlem adayları: `canvas` tarafında
   `ctx.fontKerning = "none"` (kabukta biçem kararı değil, eşitlik ayarı) ya da derleme betiğinde
   GPOS çiftlerinden `kern` tablosu üretmek. İki yol arasındaki metin farkı T3/M16'da ölçülmeli.
4. **woff2 fontdue'ya verilemez** (sıkıştırma çözücü yok); iki yolun aynı dosyayı kullanması için
   statik **ttf** alt küme hem çekirdeğe gömülür hem `FontFace` ile `canvas`'a verilir. Statik alt
   küme ağırlık başına ≈ 27–56 KB; iki ağırlık ≈ 55–113 KB (bugünkü tek Plex Mono 136 KB'tan küçük).
5. **Ayrılmış Yazı Tipi Adı (RFN):** IBM Plex ("Plex") ve Source Sans 3 ("Source") OFL'de RFN taşır;
   alt küme/örnekleme/cmap değişikliği "Modified Version" sayılır ve bu adla dağıtılamaz (OFL 1.1
   madde 3). Bu iki aile seçilirse dosya yeniden adlandırılmalı ya da değiştirilmeden kullanılmalı.
   Inter, Atkinson Next, Geist, Figtree, Public Sans, Noto Sans'ta RFN yok.
6. `locl` TRK: Atkinson Next, Geist, Figtree, Public Sans, Source Sans 3, Space Grotesk, JetBrains Mono
   Türkçe yerel değişkenleri taşır (çoğunlukla "fi" bağının i noktasını korumak için). fontdue bağ
   yapmadığından bu yolda sorun yok; `canvas`'ta `lang="tr"` önemlidir.

## 5. Hareketin anlam taşıması

1. **Animasyonlu geçişler grafik algısını iyileştirebilir**; nesne izleme görevlerinde yardımcı;
   karmaşık geçişlerde aşamalama (staging), mümkünse ortak ölçek (eksen yeniden ölçeklemesi algıyı
   bozar). — Heer & Robertson, "Animated Transitions in Statistical Data Graphics", IEEE InfoVis 2007,
   https://homes.cs.washington.edu/~jheer/files/2007-AnimatedTrans-InfoVis.pdf [Ö].
2. **Kademeli (staggered) animasyonun izlemeye beklenen faydası çıkmadı.** — Chevalier, Dragicevic,
   Franconeri, "The Not-so-Staggering Effect of Staggered Animations on Visual Tracking", IEEE TVCG
   2014 [Ö, yalnız künye]. Animasyonun arayüzdeki rolleri: Chevalier ve ark., AVI 2016,
   https://hal-enac.archives-ouvertes.fr/hal-01322962 [Ö].
3. **Değişim körlüğü:** bir değişiklik ancak yerel geçici sinyal (transient) dikkati yakalarsa görülür;
   ekran yenilenmesi/kesme gibi büyük geçiciler değişimi maskeler. — Rensink, O'Regan, Clark,
   Psychological Science 8, 1997, https://www2.psych.ubc.ca/~rensink/flicker/abs.97.3a.html [Ö];
   Rensink ve ark. 2000 https://philarchive.org/archive/RENOFT-2 [Ö].
   → Canlı güncellemede **bütün kartı yeniden çizmek** (ya da akışı yeniden sıralamak) değişimi
   gizler; değişen öğede **yerel** kısa bir geçici sinyal (işaret nabzı, rakam vurgusu) fark
   edilmeyi sağlar. Hareketi azalt ayarında aynı bilgi kalıcı bir durum işaretiyle ("yeni",
   "güncellendi 2 dk önce") verilmelidir (bilgi kaybı 0).
4. **Apple HIG Motion** [A]: "Add motion purposefully… Don't add motion for the sake of adding motion";
   "Make motion optional… avoid using it as the only way to communicate important information";
   "Aim for brevity and precision in feedback animations." —
   https://developer.apple.com/design/human-interface-guidelines/motion.
5. **WCAG 2.3.3 Animation from Interactions** (AAA): etkileşimle tetiklenen, zorunlu olmayan hareket
   kapatılabilmeli. `prefers-reduced-motion` ayarını açmayan ama hareketten rahatsız olan kullanıcılar
   da var (web.dev). — https://www.w3.org/TR/WCAG22/#animation-from-interactions [Ö].
6. **Belirsizliği hareketle göstermek** (§9, HOPs) hareketin bilgi taşıdığı nadir kanıtlı örnektir;
   ancak hareketi azalt ayarında statik eşdeğer (nicelik nokta grafiği) gerekir.

## 6. Dokunsal ve sesli geri bildirim

1. **Apple HIG Playing haptics** [A]: sistem titreşimlerini belgelenmiş anlamlarıyla kullan;
   tutarlı neden–sonuç ilişkisi kur ("If a haptic doesn't reinforce a cause-and-effect relationship,
   it can be confusing and seem gratuitous"); dokunsalı görsel ve işitsel geri bildirimi
   **tamamlayıcı** kullan; uygulamalarda kısa, ayrık olaylara eşlik eden titreşim tercih et. —
   https://developer.apple.com/design/human-interface-guidelines/playing-haptics.
2. **Titreşim desenleri (tactons) öğrenilebilir ama tanıma oranı tasarıma çok bağlı:** dokunmatik
   ekranda genel tanıma 0,69 ± 0,15, bazı birleşimler > %90; karmaşık ve çeşitli desenler basit,
   özdeş desenlerden iyi tanındı. — MDPI Sensors 21(15):5024, 2021,
   https://doaj.org/article/19e159e2326a411c845b118b6abb7749 [Ö]; Omata & Kuramoto 2020 (hece
   tabanlı bildirim titreşimi), https://www.scitepress.org/Papers/2020/100641/100641.pdf [Ö].
3. Kapsam notu: yerel kabuk R8, ses R9. Web kabuğunda iOS Safari Vibration API'yi desteklemiyor
   (MDN uyumluluk tablosu; bu oturumda açılmadı) → R5'te dokunsal yalnız **çekirdekte anlam
   kaydı** olarak (ör. `durum_değişimi: olumsuz_geçiş → uyarı`) tanımlanıp kabuğun yerine
   getirmesine bırakılabilir; ölçümü R8'e kalır.

## 7. Saat ve kilit ekranı yüzeyleri

- Live Activities (§1.3) [A]: tanımlı başlangıç–bitiş, ≤ 8 saat; orta ve üstü ağırlıkta büyük yazı;
  "yalnız gerekli güncellemelerde uyar"; aynı güncelleme için hem Live Activity hem push gönderme;
  bitince hemen sonlandır, özet 15–30 dk. watchOS Smart Stack'te de görünür.
- Kapsülle eşleşme: canlı kapsülün yaşam döngüsü (doğdu → güçlendi → geçersiz/sonuçlandı) Live
  Activity modeline doğal olarak oturur; ama bir kapsül günlerce canlı kalabilir (8 saat sınırı) →
  Live Activity yalnız **seans içi** olaylar için (ör. "eşik bugün test ediliyor"); uzun ömürlü
  kapsül widget'a. Bu R8 konusu; R5'te kapalı kapsül yüzünün **en küçük biçimi** (bir cümle + işaret,
  ≈ 160×72 pt kompakt alan) tasarım kaydı olarak tanımlanabilir.
- Saat bakışı ≈ 3,9 s (§1.2) [Ö].

## 8. Erişilebilirlik

1. **WCAG 2.2 (Öneri, geçerli standart):** 1.4.3 metin karşıtlığı ≥ 4.5:1 (büyük metin — 18 pt ya da
   14 pt kalın — ≥ 3:1); 1.4.11 grafik nesneler ve arayüz bileşenleri ≥ 3:1; 2.5.8 hedef boyutu
   (AA) ≥ 24×24 CSS px ya da 24 px çaplı dairelerin kesişmediği aralık; 2.5.5 (Enhanced) 44×44.
   — https://www.w3.org/TR/WCAG22/ ; https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html [A].
   Apple HIG: en küçük kontrol 44×44 pt; WCAG AA değerleri 4.5:1 / 3:1; karşıtlık hem açık hem koyu
   temada sağlanmalı; "Increase Contrast" ayarında daha yüksek karşıtlık [A].
2. **WCAG 3 durumu:** son Çalışma Taslağı 2026-03-03 (https://www.w3.org/TR/2026/WD-wcag-3.0-20260303/,
   https://www.w3.org/WAI/news/2026-03-03/wcag3 [Ö]); karşıtlık testi "keşif" düzeyinde, "The contrast
   algorithm used in WCAG 3 is yet to be determined"; **APCA 2023'te taslaktan çıkarıldı**; en erken
   2030. — Adrian Roselli, "WCAG3 Contrast as of April 2026",
   https://adrianroselli.com/2026/04/wcag3-contrast-as-of-april-2026.html [A]. Apple HIG APCA'yı "popüler
   ölçü" olarak anar ama değerleri WCAG AA'dan verir [A].
   → **Karar girdisi:** eşikler WCAG 2.x oranıyla ölçülür (zorunlu); APCA Lc yalnız bilgi amaçlı ek sütun
   olabilir, kabul ölçütü olamaz.
3. **Renk körlüğü benzetimi:** Brettel, Viénot, Mollon 1997 (JOSA A 14) ve Viénot, Brettel, Mollon 1999
   (Color Res. Appl. 24(4)) dikromatlar için; Machado, Oliveira, Fernandes 2009 (IEEE TVCG 15(6),
   https://www.inf.ufrgs.br/~oliveira/pubs_files/CVD_Simulation/CVD_Simulation.html [Ö]) anomal
   trikromasi dahil birleşik model. Açık kaynak uygulamaların karşılaştırması: **tritanopi için yalnız
   Brettel 1997 geçerli**; protan/deutan için üçü de sağlam, Viénot hafif önde; Machado dikromasi için
   daha düşük sırada, anomal trikromasi için ilkesel seçenek. — DaltonLens, "Review of Open Source
   Color Blindness Simulations", 2021, https://daltonlens.org/opensource-cvd-simulation/ [A].
   → Düzenek: protan ve deutan **Viénot 1999** (sRGB doğrusal uzayda), tritan **Brettel 1997**;
   anomal (şiddet 0,6) için Machado 2009 ek sütun. Hepsi deterministik matris işlemi.
4. **ΔE2000 eşikleri:** büyük alanlarda JND ≈ 1–2; görselleştirmede küçük işaretler için gereken fark
   boyutla ters orantılı büyür (Szafir 2017, §2.3) [Ö]. ColorBrewer renklerini değerlendiren bir
   çalışma **ΔE00 = 10**'u "algısal olarak güvenli en küçük uzaklık" olarak kullanmış; bütün paletin
   ayrımı ikili eşiklerden tam çıkarılamaz (EuroVis 2024 kısa makalesi) [Ö]. Colorgorical (Gramazio,
   Laidlaw, Schloss, IEEE TVCG 2017, https://connorgr.github.io/pdf/gramazio-2016-ccd.pdf [Ö])
   CIEDE2000'i ayrım puanı olarak kullanır.
   → Önerilen düzenek kuralı: anlam renkleri (yukarı, aşağı, nötr, kapanmış, seçili) her çift için
   normal görüşte ve üç benzetimde **ΔE00 ≥ 10**; küçük işaretler (≤ 12 pt) için ≥ 15 hedef
   (sezgisel; Szafir modelinden doğrulanmalı). Ayrıca **gri tonlamada** (Y = göreli parlaklık)
   anlam renkleri arasındaki parlaklık karşıtlığı ≥ 1,5:1 ve her biri zeminle ≥ 3:1 — renk tamamen
   kaybolduğunda biçim kanalı devrede kalmalı (WCAG 1.4.1 ruhu).
5. **Renk körlüğü yaygınlığı** ≈ erkeklerin %8'i, kadınların %0,5'i (Kuzey Avrupa kökenli nüfus; yaygın
   kabul gören değer, bu oturumda birincil kaynak açılmadı). Türkiye için ayrı ölçüm aranmadı.

## 9. Belirsizlik ve güven gösterimi (karne, örnek sayısı)

1. **Ayrık gösterim küçük ekranda daha iyi:** nicelik nokta grafiği (quantile dotplot) yoğunluk
   grafiğine göre olasılık tahminlerinin varyansını ≈ 1,15 kat azalttı ve daha güvenli tahmin sağladı
   (otobüs varış tahmini, mobil). — Kay, Kola, Hullman, Munson, "When(ish) is My Bus?", CHI 2016,
   https://idl.uw.edu/papers/when-ish-is-my-bus [Ö]. 50 sonuçlu nokta grafiğiyle kararların beklenen
   kazancı optimumun %97'si: Fernandes ve ark., CHI 2018, https://idl.uw.edu/papers/uncertainty-bus [Ö].
   → **Karne "10 kez; 7'sinde tuttu" ve 10 noktalık sıra (●●●●●●●○○○)** biçimi, yüzde ve çubuktan
   daha iyi okunması beklenen aday. Nokta sayısı = gerçek örnek sayısı olduğunda "az örnek" kendini
   görsel olarak söyler (3 nokta ≠ 30 nokta).
2. **Doğal frekanslar** ("10 kişiden 7'si") olasılık/yüzdeye göre Bayesçi akıl yürütmeyi iyileştirir;
   etki sayısal okuryazarlığa bağlı. — Hoffrage, Krauss, Martignon, Gigerenzer, Frontiers in Psychology
   2015, https://pmc.ncbi.nlm.nih.gov/articles/PMC4604268 [Ö]; sayısal okuryazarlık bağımlılığı:
   Judgment and Decision Making 2009 [Ö].
3. **Yalnız çıkarımsal belirsizliği göstermek etkiyi abartır:** güven aralığı gösterilen katılımcılar
   tahmin aralığı gösterilenlere göre etkiyi büyük sandı ve daha çok ödemeye razı oldu; ikisini birlikte
   göstermek daha doğru algı sağladı. — Hofman, Goldstein, Hullman, CHI 2020,
   https://mucollective.northwestern.edu/files/2020-Visualizing-Inferential-Uncertainty.pdf [Ö];
   Zhang ve ark., PNAS 2023 [Ö]. → Karnede tutma oranı yanında **sonuçların dağılımı** (ne kadar
   sürdü, ne kadar gitti) da gösterilmeli; tek yüzde aşırı güven doğurur.
4. **Belirsizlikte görsel kanalı bastırmak** (value-suppressing palettes): belirsizlik yüksekken veriye
   kanalın daha dar aralığını ayırmak, insanların belirsizliğe daha çok ağırlık vermesini sağladı. —
   Correll, Moritz, Heer, CHI 2018, https://www.tableau.com/research/publications/value-suppressing-uncertainty-palettes [Ö].
   → **Az örnekli kapsülün işareti doygunluğu/kontrastı kısılmış** (ama ≥ 3:1) çizilebilir: güven
   kanalı ayrı bir rozet yerine işaretin kendisinde. Bu M16'da "yeni kanal" adayı.
5. **HOPs** (olası sonuçların animasyonu) iki–üç niceliğin sıralamasında hata çubuklarından ve keman
   grafiklerinden çok daha doğru. — Hullman, Resnick, Adar, PLOS ONE 2015,
   https://doi.org/10.1371/journal.pone.0142444 [Ö]. Kapsül yüzü için ağır; açılmış kapsülde aday.

## 10. Somut dil ve sayısal okuryazarlık

1. **Sözel olasılık ifadeleri çok geniş yorumlanır:** ≈ 1700 kişilik ankette "likely" için %90'lık
   aralık %55–90; "real possibility" ≈ %20–80; "always" bile %90–100. Öneri: yüzde ve zaman aralığı
   söyle. — Mauboussin & Mauboussin, HBR 2018, (özet) https://www.jasoncollins.blog/posts/how-likely-is-likely [Ö].
   → Yasak belirsiz ifade listesine (M17) "muhtemelen, büyük ihtimalle, güçlü, önemli, kritik,
   olası" gibi sözel olasılık/önem sözcükleri girmeli; yerine **sayı + pencere** ("son 10 olayda 7",
   "sonraki 10 işlem günü").
2. **Göreli ve mutlak:** göreli değişim ("%40 daha sık") mutlak değişime göre etkiyi büyük gösterir
   (Gigerenzer risk iletişimi literatürü; bu oturumda birincil kaynak açılmadı). → Kapsül karnesi
   mutlak sayıyla ("10'da 7") söylenir; "ortalamadan %40 iyi" türü göreli ifade yasak listesine.
3. **Türkçe sayı biçimi (CLDR `tr`)**: ondalık ",", binlik "."; yüzde deseni **`%#,##0`** (yüzde işareti
   önde: "%5"); para deseni `¤#,##0.00`, TRY sembolü **₺**. — CLDR JSON
   https://github.com/unicode-org/cldr-json/blob/main/cldr-json/cldr-numbers-full/main/tr/numbers.json ve
   `currencies.json` [A]. → Yazı tipinde ₺ yoksa (Atkinson Next, Geist, Figtree, JetBrains Mono) tasarım
   kaydı "TL" ya da yedek glif kararı vermeli.

## 11. "Ajanların da okuduğu arayüz"

1. **VLM'ler grafiği insandan farklı okur:** CharXiv'de insan %80,5 / GPT-4o %47,1; ufak grafik
   değişiklikleriyle açık modellerde %34,5'e varan düşüş; VLAT'ta GPT-4 insanların 16. yüzdeliğinde,
   eğilimde iyi, **tek değer okumada zayıf** (insanın tersi); OCR'a çok dayanır, yama (patch) tabanlı
   görür, insan "sıçrama" etkileri aktarılmayabilir; bozulmuş görüntüde **kesin ama uydurma değerler**
   verir. Metin tabanlı grafik tanımı görsel girdiden açıkça iyi (DePlot +%29,4). — Keith-Norambuena,
   arXiv 2602.01527 (2026-02) [A].
2. **Erişilebilirlik ağacı ajanın API'sidir; `canvas` bu ağaçta opaktır.** A11y-CUA (CHI 2026): Claude
   Sonnet 4.5 varsayılan %78,3, yalnız klavye %41,7, büyüteçli görünüm %28,3 başarı. — Mohanbabu ve
   ark., https://arxiv.org/abs/2602.09310 [Ö]. Yapısal bileşen arayüzüyle %75,3'e karşı WebArena temeli
   %26,4: CI4A, https://arxiv.org/pdf/2601.14790 [Ö].
   → Bizim gösterim `canvas`'a boyuyor (K51) → ekran okuyucu ve ajan için **sahne tarifinden
   üretilen paralel anlamsal katman** gerekir (kapsül cümlesi, işaret anlamı, karne — hepsi zaten
   çekirdekte deterministik). Ajan yüzeyi R5B'de; R5'te ölçülebilir küçük adım: aynı sahneden
   çıkan erişilebilir metnin yüzle **eşdeğerliği** (aynı olgular, aynı sayılar).
3. Yüksek çözünürlük/OCR: yoğun ve küçük metinde OCR doğruluğu çözünürlükle güçlü ilişkili
   (VisualOverload, arXiv 2509.25339 [Ö]); ACL 2026 Findings: düşük DPI ve yoğun metinde kırılgan [Ö];
   temiz düzyazıda 72 ve 300 PPI farksız (ReadBench, https://www.answer.ai/posts/2025-06-05-readbench.html [Ö]).
   → Ajan okur, insanın tek bakışta okuyamayacağı küçük metni **okuyabilir** — §14'teki uyarının
   temeli.

## 12. Zaman çizelgesi ve olay işaretlerinin kümelenmesi

1. **Genel bakış + ayrıntı, yakınlaştırma, odak + bağlam** tekniklerinin karşılaştırmalı derlemesi:
   Cockburn, Karlson, Bederson, ACM Computing Surveys 41(1), 2008, https://doi.org/10.1145/1456650.1456652
   (bu oturumda açılmadı; künye bilgisi). Olay türlerinin zaman içindeki yaygınlığını renkli yığınlarla
   özetleyen "temporal summaries": Wang ve ark. 2009, https://www.cs.umd.edu/~ben/papers/Wang2009Temporal.pdf [Ö].
2. Uygulama kılavuzlarında ortak kalıp: yoğun dönemde **sayılı özet işaret** (kümeleme), dokununca
   açılma; zaman aralıklı olay = yatay çubuk ya da kuyruklu simge (ChartIQ belgeleri [Ö]). Bu kanıt değil
   pratiktir; kanıt düzeyi zayıf.
3. VLM ve insan için en güvenilir kanal **konumdur** (§2.5); kümelenen işaretlerde konum bilgisi
   (zaman ekseninde yer) korunmalı, sayı küme rozetinde yazılı olmalı.
4. Değişim körlüğü (§5.3): odak değiştiğinde (başka işarete dokunma) kanıt geometrisinin **animasyonlu
   taşınması** izlemeyi kolaylaştırır (Heer & Robertson, nesne izleme) — M18 × M21 kesişimi.
   → 200 kapsülde: kapanmışlar düşük vurgulu ince iz, canlılar tam işaret; piksel çakışması eşiğinde
   (ör. iki işaret merkezi < 1 işaret çapı) kümeleme. Ölçüm §15'te.

## 13. Bildirim / akış önceliği ve güven

1. **Yanlış alarma eğilimli otomasyon** hem uyma (compliance) hem güvenmeyi (reliance) bozar;
   kaçırmaya eğilimli otomasyon yalnız birini (kaynaklar hangisi olduğunda çelişiyor). "Cry wolf"
   etkisi. — Dixon, Wickens, McCarley, Human Factors 2007, https://interruptions.net/literature/Wickens-HF07.pdf [Ö].
   **≈ 0,70 güvenilirlik** altında kusurlu otomasyonun hiç otomasyondan kötü olduğu "kesişme noktası"
   (Wickens & Dixon meta-analizi; yıl kaynaklarda 2005/2007 tutarsız) [Ö].
   → K53 için doğrudan girdi: karnesi yazı tura (≈ %50) olan kapsül, akışın bütününe olan güveni
   düşürür. Gizleme eşiği **0,5 civarı değil, 0,6–0,7 bandında** sınanmalı (M19 benzetiminde eşik
   adaylarından biri 0,70 olmalı).
2. **Tekrarlanan uyarılar kabulü düşürür:** klinik karar destekte her ek hatırlatma, kabul olasılığını
   %30 azalttı; iş yükü değil tekrar belirleyici. — Ancker ve ark., BMC Med Inform Decis Mak 17:36, 2017,
   https://pmc.ncbi.nlm.nih.gov/articles/PMC5387195 [Ö]. → Aynı hisse için aynı türden birden çok canlı
   kapsül akışta ayrı ayrı değil, **tek satırda birleşik** gösterilmeli (K50/K54 ile uyumlu); aynı
   cümlenin tekrarı 0 (K49).
3. **Bildirimin kabulü içeriğe bağlı; önemli içerik de böler.** 20 kullanıcı, 10 372 bildirim:
   Mehrotra ve ark., "My Phone and Me", CHI 2016, https://discovery.ucl.ac.uk/1502364/1/CHI16.pdf [Ö];
   35 kullanıcı ≈ 70 000 bildirim (içerik odaklı) [Ö].
4. Apple: "Alert people only for essential updates" (§7) [A].

## 14. Dil modeliyle ajan değerlendirmesinin sınırları (ölçüm düzeneği için)

1. **Konum yanlılığı:** yargıç, iki cevap yer değiştirince kararını değiştirebilir; skorlar
   birbirine yakınken çelişki daha sık. Önlem: iki sırayı da koş, ancak ikisinde de kazanan kazanır,
   çelişki = beraberlik (Balanced Position Calibration); karar öncesi kanıt yazdır (Multiple Evidence
   Calibration). — Zheng ve ark., NeurIPS 2023 D&B, https://arxiv.org/abs/2306.05685 [Ö]; Wang ve ark.,
   "Large Language Models are not Fair Evaluators", ACL 2024, https://aclanthology.org/2024.acl-long.511.pdf [Ö].
2. **Uzunluk/ayrıntı yanlılığı:** yargıç uzun cevapları kayırır; uzunluk farkı regresyonla
   denetlendiğinde insan sıralamasıyla korelasyon 0,94 → 0,98. Liste varlığı da yanlılık. — Dubois ve
   ark., "Length-Controlled AlpacaEval", 2024, https://arxiv.org/abs/2404.04475 [Ö]. → Görsel karşılığı:
   **daha çok metin içeren ekran kayırılır** (R4'te gözlendi, K46). Metin miktarı her ekran için
   ölçülüp sonuçla birlikte raporlanmalı ve regresyonla denetlenmeli.
3. **Kendini kayırma:** yargıç kendi ürettiğini tanır ve kayırır; tanıma yeteneğiyle doğrusal ilişkili;
   "bu senin" etiketi bile tercihi değiştirir. — Panickssery, Bowman, Feng, 2024,
   https://arxiv.org/abs/2404.13076 [Ö]; perpleksite denetimi kayırmayı gidermiyor:
   https://arxiv.org/pdf/2604.06996 (2026) [Ö]. → Tasarımı üreten model ailesi ile okuyan aynıysa
   (bizde aynı) **yazar bilgisini gizlemek yetmeyebilir**; ikinci aile yoksa sınır olarak yazılmalı.
4. **Çok kipli yargıç:** MLLM'ler ikili karşılaştırmada insanla uyumlu, **puanlama ve toplu sıralamada
   belirgin uyumsuz**; benmerkezci, konum ve uzunluk yanlılıkları ve halüsinasyon. — Chen ve ark.,
   "MLLM-as-a-Judge", ICML 2024, https://arxiv.org/abs/2402.04788 [Ö]. Yargıç hatalarında "cevaba
   çapalanma" (%14,1) saf algı hatasından (%9,4) fazla: Park ve ark., ICML 2026 slaytları,
   https://icml.cc/media/icml-2026/Slides/61108.pdf [Ö]. → **Mutlak puan değil ikili karşılaştırma**;
   yargıca okuyucunun cevabı gösterilmeden önce kendi gözlemi yazdırılmalı.
5. **Sentetik kullanıcı geçerliliği:** LLM ajanları gerçek kullanıcıların tereddüt, geri dönme ve
   bırakma davranışını üretmiyor (UXAgent, CHI 2025 [Ö]); sezgisel değerlendirmede sorunların %73–77'sini
   buldu ama arayüz kalıplarını tanımada zorlandı (Zhong ve ark., arXiv 2507.02306 [Ö]); dalkavukluk
   eğilimi. → GOAL'daki "ajan insan değildir; ön elemedir" sınırı literatürle uyumlu.
6. **Algı farkları** (§11): VLM'ler insanın sıçrama etkilerini paylaşmayabilir, alan ve açıklık
   kodlamasını zayıf okur, küçük metni insandan iyi okuyabilir → ajan okur **renk/açıklık hiyerarşisini
   eksik, metni fazla** değerlendirir. Bu, bulanık/küçültülmüş "kısma sınaması"nı zorunlu kılar.

---

## 15. Aday fikir listesi

Her satırdaki "nasıl ölçülür" düzeneğe (T2) doğrudan girdi olacak biçimde yazılmıştır. "Hesap" =
deterministik betik; "okuyucu" = kör ajan okuyucu; "ikili" = ikili kör karşılaştırma + Bradley–Terry.

| # | Fikir | Kanıt | Beklenen kazanç | Konu | Nasıl ölçülür |
|---|---|---|---|---|---|
| F1 | **Başlık önden yüklü:** cümle nesne/olayla başlar (ilk 3 kelimede eşik adı ya da fiyat) | NN/g microcontent [Ö]; K49 | Bakışta "ne oldu" sorusu doğru | M17 | Hesap: her şablon çıktısında ilk 3 kelimede {fiyat, eşik adı, olay fiili} var mı (%). Okuyucu: 402 pt, tek görüntü, "ne oldu?" tek cümle; doğruluk |
| F2 | **Sözel olasılık/önem sözcükleri yasak; yerine sayı + pencere** | Mauboussin 2018 [Ö] | Yorum varyansı düşer; K49 | M17 | Hesap: yasak liste taraması (TR+EN) = 0; her karne cümlesinde bir sayı ve bir süre ifadesi var (%100) |
| F3 | **Karne doğal frekansla: "10 olayda 7"** + örnek sayısı kadar nokta | Kay 2016, Fernandes 2018, Hoffrage 2015 [Ö] | "Geçmişte kaç kez tuttu" doğru ve az örnek fark edilir | M17, M20 | Okuyucu: "kaç kez tuttu, kaç örnek?" iki sayı cevabı; doğru oran. "Bu karne güvenilir mi?" sorusunda 3 örnekli kapsül için "az örnek" cevabı oranı |
| F4 | **Güveni işaretin kendisinde taşımak** (az örnek / yazı tura → doygunluk kısık, ≥ 3:1 korunur; VSUP) | Correll 2018 [Ö] | Rozet eklemeden güven okunur; tek bakış sınırı (K43) korunur | M16 | Hesap: kısık işaretin karşıtlığı ≥ 3:1. Okuyucu ikili: aynı içerik, kısık/kısıksız; "hangisine daha çok güvenirsin" cevabının örnek sayısıyla uyumu |
| F5 | **Tek anlam = tek ana kanal + biçim yedeği** (yön: üçgen yönü; sonuç: dolu/boş; geçerlilik: parlaklık) | Healey 1995 [Ö], Apple HIG [A] | Renk körü ve gri tonlamada bilgi kaybı 0 | M16, M18 | Hesap: işaret glifleri renksiz (tek renk) render edilip glifler arası piksel farkı (IoU < 0,7) ; okuyucu: gri tonlama + 3 CVD benzetimi görüntüsünde yön/sonuç sorusu, doğruluk normal görüşle eşit olmalı |
| F6 | **Anlam renkleri ΔE00 eşiği:** normalde ve Viénot (protan, deutan) + Brettel (tritan) benzetimlerinde her çift ≥ 10; ≤ 12 pt işarette ≥ 15 | Szafir 2017 [Ö], DaltonLens [A] | Renk ayrımı garanti | M16 | Hesap: renk kayıtlarından çift × benzetim tablosu; en küçük ΔE00 raporu; eşik altı = eleme |
| F7 | **Gri tonlama ayrımı:** anlam renklerinin göreli parlaklık karşıtlığı çift başına ≥ 1,5:1, zeminle ≥ 3:1 | WCAG 1.4.1/1.4.11 [A] | Renksiz ekranda (gece modu, gri filtre) de okunur | M16 | Hesap: Y tablosu, açık/koyu tema |
| F8 | **Kapanmış işaret ≥ 3:1** ve küçük boyda bile (A31) | WCAG 1.4.11 [A]; Szafir [Ö] | Erişilebilirlik borcu kapanır | M16 | Hesap: her işaret durumunun gövde (yalnız köşe değil) karşıtlığı; en kötü piksel kümesi |
| F9 | **Tek vurgu ilkesi:** ekranda yalnız işaret en yüksek görsel ağırlıkta (boyut × karşıtlık) | M3 Expressive [A] (tek anahtar öğede 4× hız); Live Activities [A] | İlk bakış işarete gider (K7) | M16 | Hesap: saliency modeli (ör. açık bir mobil saliency modeli) ya da basit "görsel ağırlık = alan × ΔL" sıralaması; işaret 1. mi. Kısma sınaması: bulanık+küçük görüntüde "ilk ne görüyorsun" → işaret cevabı oranı |
| F10 | **Hümanist, açık iç boşluklu orantılı aile** başlıkta; rakamlar tablo | Reimer 2014, Sawyer 2020 [Ö] | Bakış süresi ≈ %10 kısalır (araç ekranı verisi) | M16 | Okuyucu: 1 s'lik "gösterim" vekili olarak 0,5× küçültülmüş görüntüde başlık okuma doğruluğu (aile karşılaştırması, ikili). Hesap: en küçük yazı boyu ≥ 11 pt; x-yüksekliği/em |
| F11 | **Yazı ailesi değiştirilebilir kayıt** (tek "en iyi" aile yok) | Wallace 2022 [Ö] | İnsan testinde kişisel tercih denenebilir | M16, T3 | Kayıt testi: `?look=` ile aile değişince düzen taşması 0 (metin kutusu taşma sayısı) |
| F12 | **fontdue'ya hazır statik ttf alt küme** (ağırlık başına; rakam cmap'te tnum; aralama eşitliği) | §4.2 [Y] | Paket ≈ 55–113 KB (2 ağırlık); `ref`=`canvas` metni | M16 | Hesap: paket boyutu önce/sonra; 0–9 genişlik eşitliği; `ref` ve `canvas` metin genişliği farkı (px, her öğe) ≤ 0,5 px |
| F13 | **Yerel geçici sinyal** ile durum değişimi (değişen öğede kısa nabız; tüm kartı yeniden çizme yok); hareketi azalt'ta kalıcı "yeni/güncellendi" etiketi | Rensink 1997 [Ö]; Bartram 2003 [Ö]; HIG Motion [A] | Değişim fark edilir; bilgi kaybı 0 | M21 | Okuyucu: önce/sonra iki kare + **ara kare** dizisi; "ne değişti?" cevabı. Hareketi azalt: son kare tek başına aynı soruyu cevaplatmalı (bilgi kaybı = cevap farkı). Hesap: değişen piksel alanı / ekran (yerellik) |
| F14 | **Açılma geçişinde süreklilik** (işaret yerinden büyür, ortak ölçek, aşamalı) | Heer & Robertson 2007 [Ö] | "Bu açılan şey hangi kapsüldü" izlenir | M21 | Okuyucu: yalnız ara kareler (son ekran gösterilmeden) + "hangi kapsül açıldı" çoktan seçmeli (R4 dersi). Hesap: geçiş süresi ≤ 300 ms, K52 bütçesi |
| F15 | **Aynı hisse birleşik satır**: birden çok kapsül tek satırda "3 olay: 2 canlı" | Ancker 2017 [Ö] (tekrar → kabul ↓) | Akışta tekrar ve yorgunluk azalır | M19, M18 | Benzetim: gün başına akış satırı sayısı, satır başına tekrar eden tür sayısı = 0 |
| F16 | **Gizleme eşiği 0,6–0,7 bandında sınanır** (0,70 aday), az örnekli gizlenmez | Wickens & Dixon [Ö] | Akışın genel güvenilirliği > 0,7 | M19 | Benzetim (geleceğe bakmadan): kalanların sonradan tutma oranı, gizlenenlerinki, boş gün payı; eşik 0,55/0,60/0,70 × min örnek 5/10 |
| F17 | **Kanıt ekranında tutma oranı + sonuç dağılımı** (yalnız yüzde değil) | Hofman 2020 [Ö] | Aşırı güven azalır | M20 | Okuyucu: "bu olay tutarsa ne kadar sürer/gider?" sorusunda aralık cevabı oranı; "kesin mi?" sorusuna "hayır" oranı |
| F18 | **Kümelenmiş işaret: konum korunur, sayı rozette**; kapanmışlar ince iz; canlılar tam işaret | Wang 2009 [Ö]; Machine Bertin [A] (konum) | 200 kapsülde canlı bulunur | M18 | Görev: 5/50/200 kapsül; "şu an canlı olan hangisi" doğruluğu ve (insan testinde) süre; hesap: piksel çakışması oranı, K52 |
| F19 | **Paralel anlamsal katman:** sahneden üretilen erişilebilir metin (cümle, işaret anlamı, karne) | A11y-CUA [Ö]; Machine Bertin [A] (metin > görüntü) | Ekran okuyucu ve ajan aynı olguyu okur | M20 (R5B'ye hazırlık) | Hesap: erişilebilir metin ile yüz metni aynı sayıları içeriyor (küme eşitliği), yetkisiz okuyucuda rakam yok (K33) |
| F20 | **Kompakt yüz** (≈ 160×72 pt: bir cümle + işaret) tasarım kaydı | Live Activities [A]; Pizza 2016 [Ö] | Saat/kilit ekranına hazır yüz | M16 (R8 hazırlık) | Hesap: kompakt kayıtta metin taşma 0, yazı ≥ orta ağırlık; okuyucu: kompakt görüntüde "ne oldu" doğruluğu |
| F21 | **Dokunsal anlam kaydı** (doğdu / olumsuz geçiş / sonuçlandı → sistem deseni adı), görsel ile eşzamanlı | Apple HIG haptics [A]; tacton çalışmaları [Ö] | Tutarlı neden–sonuç; R8'e hazır | M21 (kayıt), ölçüm R8 | Hesap: her durum değişiminin tek bir desen adı var, desen–anlam eşlemesi birebir (çakışma 0) |
| F22 | **Türkçe sayı biçimi CLDR'den** (%5, 1.234,56 ₺) ve ₺ glif kararı | CLDR [A] | Yerel doğruluk | M16, M17 | Hesap: biçim birim testleri (CLDR desenleriyle); seçilen ailede ₺ glif var mı ya da "TL" kararı |

## 16. İşe yaramayan / elenen fikirler

| Fikir | Neden elendi |
|---|---|
| **Radyal/dairesel özgün grafik biçimleri** tek bakış yüzünde | Saatte radyal çubuk ≈ 1548 ms, çubuk ≈ 250 ms (Blascheck 2018) [Ö]; özgünlük okunabilirlik pahasına. "Alışılmışın dışı" yön bunu deneyebilir ama ölçümle; varsayılan değil |
| **Tek değişken yazı tipi dosyasıyla çok ağırlık** (çekirdek yolu) | fontdue 0.9.5 yalnız varsayılan örneği çizer [Y]; kazanç yalnız `canvas`'ta olur, iki yol ayrışır (sahne tek doğruluk kaynağı ilkesine aykırı) |
| **woff2'yi doğrudan gömmek** | fontdue okuyamaz; iki yol aynı dosyayı kullanamaz [Y] |
| **APCA'yı kabul eşiği yapmak** | WCAG 3'ten 2023'te çıkarıldı; algoritma "henüz belirlenmedi", en erken 2030 (Roselli 2026) [A]. Bilgi sütunu olarak kalabilir |
| **Machado 2009'u dikromasi benzetiminde tek yöntem yapmak** | Tritanopide zayıf; protan/deutanda Viénot/Brettel daha iyi (DaltonLens 2021) [A] |
| **Kademeli (staggered) çoklu animasyon** (akışta kartların sırayla gelmesi) | İzlemeye beklenen faydası çıkmadı (Chevalier 2014) [Ö, künye]; süs, ölçülebilir kazanç yok, K52 maliyeti var |
| **Uzun süren/sürekli hareket** (sürekli nabız atan canlı işaret) | Hareket çevrede güçlü ama dikkat dağıtır (Bartram 2003) [Ö]; Apple: kısa ve kesin [A]; hareketi azalt'ta bilgi taşıyamaz. Yalnız durum değişiminde kısa nabız (F13) kalır |
| **Sözel güven etiketleri** ("güçlü sinyal", "yüksek olasılık") | Sözel olasılıkların yorum aralığı çok geniş (Mauboussin 2018) [Ö]; K49 ile çelişir |
| **Yalnız yüzde ile karne** ("%70 başarı") | Örnek sayısını gizler; doğal frekans ve dağılım daha iyi (Hoffrage 2015, Hofman 2020) [Ö] |
| **Her şeyi "ifadeli" yapmak** (çok renk, çok boyut, kart başına farklı biçim) | M3E'nin kazancı tek anahtar öğede; dağınık yerleşim ve etiketsiz eylemler kullanılabilirliği düşürdü; bankacılıkta yerleşik kalıplar [A] |
| **Eleştirmenin mutlak 1–10 puanı** | MLLM'ler puanlamada insanla uyumsuz, ikilide uyumlu (Chen 2024) [Ö]; tercih hız öngörmüyor (Wallace 2022) [Ö] |
| **Live Activity'yi uzun ömürlü kapsül için kullanmak** | Apple: ≤ 8 saat, tanımlı başı–sonu olan olaylar [A]; R8 kapsamı |
| **Web kabuğunda titreşim** | iOS Safari Vibration API'yi desteklemiyor (MDN; açılmadı); kabuk ince kalmalı; ölçüm R8'e |
| **IBM Plex / Source Sans 3'ü alt kümeyle aynı adla dağıtmak** | RFN ("Plex", "Source") — değiştirilmiş sürüm aynı adı taşıyamaz (OFL 1.1 §3). Elenmez ama yeniden adlandırma şartı |

## 17. Ölçüm düzeneği için uyarılar (ajan okuyucu yanlılıkları ve karşı önlemler)

| # | Risk | Kaynak | Karşı önlem (T2'ye somut girdi) |
|---|---|---|---|
| U1 | **Konum yanlılığı** (A/B sırası kararı değiştirir) | Zheng 2023, Wang 2024 [Ö] | Her çift iki sırada; ikisinde de aynı kazanan = galibiyet, çelişki = beraberlik. Çelişki oranı (conflict rate) tasarım çifti başına raporlanır; > %30 ise o çiftin sonucu "ayırt edilemedi" |
| U2 | **Metin/uzunluk yanlılığı** (çok metinli ekran kayırılır; R4 K46) | Dubois 2024 [Ö] | Her ekran için metin karakter sayısı ve metin alanı payı hesaplanır; BT modeline ortak değişken olarak eklenir (uzunluk denetimli sıralama) ya da en azından kazananla metin miktarının korelasyonu raporlanır |
| U3 | **Ajan küçük metni insandan iyi okur** (OCR), açıklık/alan hiyerarşisini zayıf okur | Machine Bertin [A], VisualOverload [Ö] | Gerçek boyut (402 pt @1×) + **kısma sınaması** (Gauss bulanıklığı σ ≈ 2–3 px ve 0,5× küçültme) zorunlu; ana soru bulanık görüntüde sorulur. Hesaplanan karşıtlık/boyut ölçüleri ajan puanından **önce** eleme kapısıdır |
| U4 | **Kendini kayırma** (tasarımı da okuyucuyu da aynı aile üretiyor) | Panickssery 2024 [Ö] | Okuyucuya yazar/"eski–yeni" bilgisi verilmez; adaylar nötr kimlikle (rastgele harf); mümkünse ikinci model ailesi, değilse sınır olarak yazılır |
| U5 | **Mutlak puan güvenilmez**, ikili daha iyi | Chen 2024 [Ö] | Yalnız ikili karşılaştırma + Bradley–Terry; bootstrap %95 güven aralığı; aralıklar örtüşüyorsa kazanan ilan edilmez |
| U6 | **Cevaba çapalanma** (yargıç okuyucunun cevabına kapılır) | Park 2026 [Ö] | Eleştirmen önce kendi gözlemini (bir cümle) yazar, sonra ölçütle seçer; okuyucu cevapları eleştirmene gösterilmez; anlama sorularının doğruluğu **deterministik anahtarla** puanlanır (dil modeli değil) |
| U7 | **Dalkavukluk / iyimserlik** (ajan "her şey anlaşılır" der) | sentetik kullanıcı literatürü [Ö] | Açık uçlu "anlaşılır mı" sorusu yok; yalnız olgu soruları (hangi eşik, kaç, ne zaman, kaç kez tuttu, bana ne yapmamı söylüyor → "hiçbir şey"); bozuk tasarım sınaması sona konamazsa düzenek geçersiz |
| U8 | **Ajan davranışı insan değildir** (tereddüt, bırakma yok) | UXAgent 2025 [Ö] | Dokunma sayısı görevleri hesapla (en kısa yol uzunluğu) ölçülür; ajan sonucu ön elemedir, insan testi v3 son sözdür |
| U9 | **Tercih ≠ performans** | Wallace 2022 [Ö] | "Hangisini beğendin" ikili sonucu ile anlama doğruluğu ayrı raporlanır; seçimde anlama doğruluğu önceliklidir |
| U10 | **Durağan son kare geçişi ölçmez** (R4 dersi) | GOAL; Heer & Robertson [Ö] | M21 sorusu yalnız ara karelerden çözülebilir olmalı; kontrol: aynı soru yalnız son kareyle sorulduğunda doğruluk şans düzeyinde olmalı (sorunun geçerliliği sınaması) |
| U11 | **Tek tur rastlantısallık** | genel | Her çift × persona en az 3 tekrar (farklı rastgele sıra tohumu); tutarlılık (aynı çifte aynı karar oranı) raporlanır; sıcaklık sabit ve kayıtlı |
| U12 | **Okuyucu ve yüz arasında metin kaçağı** (dosya adı, alt yazı, HTML başlığı) | genel | Görüntüler meta verisiz PNG; dosya adları rastgele; okuyucu istemi tasarım adını içermez (körlük kayıttan doğrulanabilir) |

Ek öneri: **deterministik vekiller** (karşıtlık, ΔE00 × benzetim, en küçük yazı, hedef boyutu, metin
yoğunluğu, görsel ağırlık sıralaması, Rosenholtz tarzı kalabalık ölçüsü) ajan okuyucudan önce koşulan
**eleme kapısı** olsun; ajan yalnız kapıdan geçen adayları sıralasın. Böylece ajan yanlılıkları
ancak ikinci aşamada etkili olur ve her eleme gerekçesi yeniden üretilebilir.

## 18. Ek: deneme ayrıntıları [Y]

- İndirme: `https://raw.githubusercontent.com/google/fonts/main/ofl/<aile>/<dosya>.ttf` ve aynı klasörde
  `OFL.txt` (13 ailenin hepsi HTTP 200).
- fontTools betikleri: cmap Türkçe kapsama, rakam genişlikleri, GSUB özellikleri, `fvar` eksenleri,
  woff2 boyutu; `varLib.instancer` ile statik örnek + `subset` + cmap `tnum` yeniden eşleme.
- fontdue: 64 px "H", "1", "0", "ğ", "İ" rasterleştirme; mürekkep toplamı ve ilerleme genişliği.
- Gerçek piyasa verisi kullanılmadı. Geçici dosyalar scratchpad'dedir ve iş sonunda silinir; depoya
  yazı tipi dosyası eklenmedi (seçim M16'nındır).
