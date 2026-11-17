# R5 — Dil ve Tasarım

## Neden

R4 ilk kullanılabilir gösterimi verdi, RP3 onu telefonda akıcı yaptı (K51: `canvas`,
tik + boyama ≈ 2 ms). Çalışıyor; ama henüz iyi değil. Arkin'in kullanım izlenimi:

- **Dil boş kalabiliyor.** "Önemli bir eşik" ne demek? Hangi eşik, kaç lira? (K49)
- **Ayrıntı cümlesi hâlâ iç dilde** (bar sayısı, "teyit", "aile"; A33) ve yüzle çelişebiliyor.
- **Grafik tek kapsülü gösteriyor.** Aynı hissede birden çok kapsül var; grafik onların
  hepsinin hikâyesi olmalı (K50, K54).
- **Akış eskiyi öne koyabiliyor**; karnesi yazı tura olan kapsüller güveni düşürüyor (K53).
- **Görsel hiyerarşi zayıf:** her şey aynı ağırlıkta tek aralıklı yazı; işaret küçük;
  kapanmış işaretin karşıtlığı erişilebilirlik eşiğinin altında (A31).

Bu round kapsül deneyiminin dilini ve görsel dilini kurar. Anlaşılabilirlik ve arayüz
deneyimi bu ürünün asıl farkıdır (`NORTH_STAR.md`: "İnsan grafiğe bakmaz; grafik insana
haber verir"). 2026–2030 için tasarlıyoruz: bugünün alışkanlıklarını kopyalamak yetmez.

## Kapsam kararı (K55)

Bu round **kapsül deneyimidir**: akış, kapsülün yüzü, açılmış kapsül, hikâye çizelgesi
(grafik derinliği), kanıt ekranı, sözlük, ayarlar. TradingView benzeri profesyonel
çalışma alanı (çizim araçları, çok panel, gösterge yönetimi, yerleşimler) **R7'dedir**.

Ama görsel dil ikisine de hizmet etmek zorunda. Bu yüzden seçilen dil var olan çok
panelli görünümlere (R2'nin altı görünümü) uygulanır ve ölçülür: **yoğunluk sınaması**.
Etkileşimi yapılmaz; yalnızca "bu dil yoğun ekranda da okunuyor mu" sorusu cevaplanır.

## Değişmezler

- **Tasarım gramerde yaşar** (Ö44): renk, yazı, boşluk, hareket değerleri çekirdekteki
  tasarım kayıtlarıdır; kabukta biçem kararı yoktur. Kabuk ince kalır (mimari testi).
- **Sahne tarifi tek doğruluk kaynağıdır**; `ref` ve `canvas` aynı sahneyi boyar.
- **Dil deterministiktir:** cümleler şablon dilbilgisinden gelir; dil modeli ürün
  yolunda yoktur (K3). Dil modeli yalnızca ölçüm düzeneğinde okuyucu ve eleştirmen olabilir.
- **Gözlem, tavsiye değil** (`docs/CAPSULE.md` §8). Hiçbir cümle "al", "sat", "fırsat" demez.
- **Dürüstlük:** karne, "az örnekli", "sunucu doğruladı / bu cihazda sınandı" ayrımı
  gizlenmez. Lisans kuralı (K33, K34) aynen geçerli; yetkisiz okuyucu rakam görmez.
- **Performans bütçesi (K52)** her alternatif için ölçülür; bütçeyi aşan tasarım seçilmez.
- **İki dil** (Türkçe, İngilizce), açık ve koyu tema, hareketi azalt ayarı.
- Yazı tipleri gömülüdür, lisansı açıktır (OFL ya da eşdeğeri; lisans dosyası depoda);
  sayfa dış ağa istek yapmaz.
- Hesap özeti ve protokol dökümlerinin anlamı değişmez. **Altın görseller bu round'da
  değişebilir** (tasarım round'u): var olanlar silinmez, `v1` olarak kalır; yeni dil `v2`
  altınlarını ekler; değişen her şey gerekçesiyle listelenir.

## Kapsam

1. **Araştırma: yeni nesil arayüz.** Güncel kaynaklardan (2026): mobil finans ve haber
   uygulamalarında özet/kart dili, veri görselleştirmede anlam kodlama, tek bakış
   okunabilirliği (glanceable) araştırması, değişken yazı tipleri, hareketin anlam
   taşıması, dokunsal ve sesli geri bildirim, saat ve kilit ekranı yüzeyleri, erişilebilirlik
   (WCAG 2.2, renk körlüğü), "ajanların da okuduğu arayüz" fikri. Rakiplerin ne yaptığı
   değil, neyin işe yaradığına dair kanıt aranır. Çıktı: aday fikir listesi, her biri için
   beklenen kazanç ve nasıl ölçüleceği.
2. **Kapsül dili** (K49, A33, A30): başlık, ayrıntı cümlesi, karne cümlesi, sözlük.
3. **Görsel dil:** yazı, renk, boşluk, biçim, işaretin yeri ve büyüklüğü, hareket.
4. **Hikâye çizelgesi** (K50, K54): grafik derinliği enstrümanın bütün kapsüllerini gösterir.
5. **Akış** (K53): canlı olan önce; yazı tura karneliler gizli; boş akış durumu.
6. **Açılmış kapsül ve kanıt ekranı;** güven ayrımının (sunucu / cihaz) anlatımı;
   yetkisiz okuyucunun yüzü (A32).
7. **Telefonda karşılaştırma:** finalistler gösterimde ayardan ve adres parametresiyle
   seçilir; ayrıca yan yana **galeri sayfası**.
8. **İnsan testi v3** paketi (v2 koşulmadı; yeni tasarımla güncellenir).
9. **Küçük işler:** `cpu` boyama yolunun kaldırılması (K51); ölçüm ekranındaki
   "tik + geçiş" bölme hatası (tek veri karesinde anlamsız değer); `scripts/clean.sh` (K56).

## Kapsam dışı

- Profesyonel çalışma alanı etkileşimi (R7). Gerçek canlı veri (R6). Yerel uygulama
  kabuğu (R8). Dil modeliyle anlatım ve ses (R9). Ajan yüzeyi (R5B).
- Yeni dedektör ya da yeni anlam türü. Altı tür ve yedi dedektör aynı kalır; yalnızca
  nasıl anlatıldıkları ve gösterildikleri değişir.

## Ölçümle karar verilecek konular (K39)

Her konu için **düzenek ve puanlama kuralı alternatiflerden önce** yazılır ve commit
edilir. Alternatifler ayrı alt ajanlarca, ayrı dallarda, birbirini görmeden, en iyi
halleriyle kurulur. Okuyucu ve eleştirmen ajanlar tasarımı kimin yaptığını ve hangisinin
"eski" olduğunu bilmez. Bugünkü tasarım (`r4`) her ölçümde başvuru olarak yarışır.

| # | Konu | Alternatif (en az) | Ölçüm |
|---|------|--------------------|-------|
| M16 | **Görsel dil** (yazı ailesi ve ölçeği, renk ve anlam renkleri, boşluk, köşe ve yüzey, işaretin yeri ve boyu, açık/koyu) | 4 yön; biri bilinçli olarak "alışılmışın dışı" olmalı (K18) | Tek bakış okuması (aşağıda); eleştirmen puanı; hesapla: karşıtlık (WCAG 1.4.3 ve 1.4.11, her öğe), en küçük yazı boyu, dokunma hedefi (≥ 44 pt), metin yoğunluğu, gri tonlamada ve üç renk körlüğü benzetiminde anlam renklerinin ayrımı (ΔE); yoğunluk sınaması; K52 bütçesi; paket boyutuna etki |
| M17 | **Kapsül dili** (başlık + ayrıntı + karne cümlesi yapısı; sözlüğün nasıl açıldığı) | 3 cümle yapısı | Anlama soruları: ne oldu, hangi eşik ve kaç, ne zaman, hâlâ geçerli mi, geçmişte kaç kez tuttu, bana ne yapmamı söylüyor (doğru cevap: hiçbir şey). Hesapla: yasak belirsiz ifade listesi (deterministik tarama, 0 olmalı), aynı cümlenin tekrarı (0), uzunluk, iç terim (0), Türkçe ve İngilizce eşdeğerliği. Yedi dedektör × yaşam döngüsü aşamaları × iki dil tablosu eksiksiz |
| M18 | **Hikâye çizelgesi** (kapsüllerin grafikte gösterimi: ömür, sonuç, seçili vurgusu, kümeleme, odak değiştirme) | 3 | Görevler: "bu hissede son üç ayda ne oldu", "hangileri tuttu", "şu an canlı olan hangisi", "şu işaretin kanıtı ne". Doğruluk ve dokunma sayısı. Görünen aralıkta 5 / 50 / 200 kapsülle okunabilirlik ve K52 bütçesi. Kapanmışlar görünürken canlı olanın bulunma süresi |
| M19 | **Akış önceliği ve gizleme eşiği** (K53) | 3 eşik tanımı × 2 sıralama | Gerçek veride (varsa `fixtures/local/`, yoksa sentetik) geriye dönük benzetim, geleceğe bakmadan: akışta kalan kapsül sayısı (gün başına), kalanların sonradan tutma oranı, gizlenenlerinki, boş akış günlerinin payı, eşiğin örnek sayısına duyarlılığı. Persona görevleri: "son baktığımdan beri ne değişti", "elimdekinde olumsuz bir şey var mı" |
| M20 | **Açılmış kapsül ve kanıt ekranı** (bilginin sırası, sözlüğün yeri, güven ayrımı, yetkisiz görünüm) | 2 | "Bu iddia neye dayanıyor", "bunu kim doğruladı", "neden rakam göremiyorum" soruları; dokunma sayısı |
| M21 | **Hareket** (açılma geçişi, canlı güncelleme, durum değişimi: doğdu, güçlendi, geçersiz) | 2 + hareketsiz başvuru | Ara karelerden sorulan süreklilik sorusu (R4 dersi: son ekrandan çözülebilen soru geçişi ölçmez); durum değişiminin fark edilmesi; K52 bütçesi; hareketi azalt ayarında bilgi kaybı (0 olmalı) |

### Tek bakış okuması (R4'ün dersi)

R4'te persona ajanları görüntüyü büyütüp uzun uzun okuyabildi; ölçüm metin ağırlıklı
tasarımı kayırdı (K46). Bu round'da düzenek bunu engeller:

- **Gerçek boyut:** görüntü telefonun fiziksel çözünürlüğünde değil, okurun gördüğü
  boyuta indirgenmiş verilir (ör. 402 pt genişlik, 1×); büyütme yok, tek görüntü, tek tur.
- **Kısma sınaması:** aynı ekranın bulanıklaştırılmış ve küçültülmüş kopyasında "ilk
  ne görüyorsun, en önemli şey ne" sorusu. Hiyerarşi bulanıkta da okunmalı.
- **Kısa cevap:** okuyucu en çok bir cümle yazar; uzun akıl yürütme istenmez.
- **İkili kör karşılaştırma:** aynı içerik iki tasarımda yan yana, sıra rastgele;
  eleştirmen önceden yazılmış ölçütle hangisinin daha iyi olduğunu söyler. Sonuçlar
  ikili karşılaştırma sıralamasıyla (ör. Bradley–Terry ya da Elo) birleştirilir; güven
  aralığı raporlanır. Fark aralığın içindeyse "ayırt edilemedi" yazılır, kazanan ilan edilmez.
- **İki persona:** geleceğin yatırımcısı (teknik analiz bilmiyor, beş saniyesi var,
  telefonda) ve alışkın kullanıcı (40+, terminal alışkanlığı, ayrıntıya güvenir).
- **Düzeneğin kendi sınaması:** bilinçli olarak bozulmuş bir tasarım (karşıtlık düşük,
  hiyerarşi ters) yarışa gizlice eklenir; düzenek onu sona koyamıyorsa düzenek geçersizdir.
- Okuyucu olarak başka bir model ailesi de kullanılabiliyorsa (`docs/MODELS.md`; anahtar
  `.env.local`'da varsa) ikinci bir aile eklenir ve aileler arası uyum raporlanır. Yoksa
  bu bir sınır olarak yazılır; anahtar istenmez, round bunun için durmaz.

**Sınır:** ajan insan değildir. Bu ölçümler ön elemedir. Her konuda ilk iki aday
gösterimde seçilebilir kalır; son söz Arkin'in telefonundaki karşılaştırma ve insan testi v3'tür.

## Tasarım sınırları

- **Tek bakış** (K43): kapalı kapsül bir cümle, bir işaret, en çok iki küçük ek bilgi.
- **İşaret önemlidir** (K7): anlam işareti ekranın en hızlı okunan öğesi olmalı; bugün
  küçük ve soluk. Kapanmış işaretin karşıtlığı ≥ 3:1 (A31).
- **Somut dil** (K49): eşik adıyla ve değeriyle; beklenti kendisiyle; terim dokununca açıklanır.
- **Renk tek başına anlam taşımaz:** yön, sonuç ve geçerlilik biçimle de okunur.
- **Sayılar hizalı okunur** (tablo rakamları); para ve yüzde biçimi yerele göre.
- Yazı ailesi serbesttir: tek aralıklı zorunlu değil. Seçilen aile Türkçe karakterleri
  (ğ, ş, ı, İ) ve tablo rakamlarını tam taşımalı.
- Yeni görsel fikir serbesttir (K18): kapsülün yaşını, gücünü, güvenini taşıyan yeni
  kanallar, işaretin canlılığı, akışın zaman duygusu. Her yeni kanal ölçülür; ölçülemeyen
  süs eklenmez.

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | Araştırma: `rounds/R5/RESEARCH.md` güncel kaynaklarla (bağlantılı), aday fikirler, her birinin kanıtı ve ölçüm planı; işe yaramayan ve elenen fikirler dahil. |
| 2 | Düzenek önce: M16–M21 için düzenek ve puanlama kuralı alternatiflerden önce commit edilmiş (commit sırası kanıt). Bozuk tasarım sınaması düzeneği doğrulamış. Okuyucu ve eleştirmenlerin kör olduğu kayıtlardan anlaşılıyor. |
| 3 | M17 dil: yasak belirsiz ifade taraması 0; tekrar 0; iç terim 0; her dedektör kendi eşiğini adıyla ve değeriyle söylüyor; karne beklentinin kendisini yazıyor; ayrıntı cümlesi yüzün yapı taşlarından kuruluyor ve yüzle çelişmiyor (A33 kapandı). Sözlük deterministik, iki dilde, her terim dokununca açılıyor. Dedektör × aşama × dil tablosu `docs/LANGUAGE.md`'de. |
| 4 | M16 görsel dil: en az dört yön kuruldu ve ölçüldü; seçilen dil tasarım kayıtları olarak çekirdekte; kabukta biçem kararı yok (mimari testi). Bütün öğelerde karşıtlık eşikleri tutuyor (kapanmış işaret dahil); anlam renkleri gri tonlamada ve renk körlüğü benzetimlerinde ayrılıyor. `docs/DESIGN.md` dili, gerekçeyi ve elenenleri anlatıyor. |
| 5 | Yoğunluk sınaması: seçilen dil R2'nin altı görünümüne uygulandı; okunabilirlik ölçüldü; R7 için açık kalan sorular yazıldı. |
| 6 | M18 hikâye çizelgesi: grafik derinliği enstrümanın görünen aralıktaki bütün kapsüllerini gösteriyor; seçili vurgulu, kanıt geometrisi yalnız seçili için; başka işarete dokununca odak taşınıyor; uzaklaşınca kümeleniyor; kapanmışlar varsayılan görünür ve kapatılabilir (K54). Liste grafik açılınca sunucudan isteniyor (K48); protokol eki şartnamede, dökümleri var. 200 kapsülde K52 bütçesi tutuyor. |
| 7 | M19 akış: canlı ve yeni olan önce; yazı tura eşiği önceden yazılı, geleceğe bakmıyor; gizlenenler karneye işliyor; az örnekli gizlenmiyor, söyleniyor; gizlenenler grafikte ve en açık hassasiyette işaretli görülebiliyor (K53). Benzetim sonuçları tabloda. Boş akış durumu tasarlanmış ("şu an anlatacak bir şey yok" dürüstçe). |
| 8 | M20: açılmış kapsül ve kanıt ekranı seçildi; "sunucu doğruladı" ile "bu cihazda sınandı" ayrımı anlaşılıyor; yetkisiz okuyucu için şablonlu rakamsız yüz var (A32) ve girişimsizlik testleri geçiyor. |
| 9 | M21 hareket: seçildi; hareketi azalt ayarında bilgi kaybı yok; ara kare ölçümü yapılmış. |
| 10 | Performans: seçilen tasarım `canvas` yolunda kalibre benzetimde (`PHONE` ve JIT açık tahmini) K52 bütçesini tutuyor; önce/sonra tablosu; paket boyutu (yazı tipleri dahil) önce/sonra. |
| 11 | Telefonda karşılaştırma: her konuda ilk iki aday Ayarlar'dan ve adres parametresiyle seçilebiliyor. `scripts/demo.sh --gallery` (ya da eşdeğeri) sentetik veriyle bir galeri sayfası açıyor: aynı içerik, adaylar alt alta, telefonda kaydırılarak karşılaştırılıyor; her adayın altında "bunu seçerdim" ve kısa not alanı; sonuç tek dokunuşla kopyalanıyor. |
| 12 | İnsan testi v3: `rounds/R5/human-test/` altında yeni tasarımla görev listesi (yaklaşık yirmi dakika, beş ila on kişi), gözlem formu, puanlama; A21 (alfabe) dahil hangi açık kararları nasıl kapatacağı yazılı. |
| 13 | Küçük işler: `cpu` yolu kaldırıldı; ölçüm ekranı bölme hatası giderildi; `scripts/clean.sh` yazıldı, `--dry-run` var, round sonunda çalıştırıldı. |
| 14 | Altınlar: `v1` altınlar yerinde; `v2` altınlar eklendi; değişen dökümler ve görseller gerekçeli listelendi. Hesap özeti değişmedi. Dökümler üç taşımada geçiyor. |
| 15 | `REPORT.md`, `RESEARCH.md`, `AUDIT.md`; `docs/DESIGN.md`, `docs/LANGUAGE.md`, `docs/FACE.md` güncel; `STATUS.md`, `ROADMAP.md`; önerilen kararlar `DECISIONS.md`'de ayrı bölümde ("Önerilen (R5)"). Raporun sonunda Arkin için: galeriyi telefonda nasıl açarım, neye bakarım. Disk hijyeni yapıldı, kalan boyut raporda (K56). |
