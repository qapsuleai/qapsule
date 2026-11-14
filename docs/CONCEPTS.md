# Kavram Sözlüğü

Projede herkes (insan ve ajan) aynı kelimeyi aynı anlamda kullanır. Kodda İngilizce
karşılık kullanılır; dokümanda Türkçesi.

| Kavram | Kodda | Tanım |
|--------|-------|-------|
| Seri | `Series` | Zamana bağlı sayısal veri. Bar, indikatör çıktısı, akış. |
| Seri kaynağı | `SeriesSource` | Serinin geldiği yer: dosya, Matriks yayını, NATS, yerel hesap. Çekirdek kaynağın türünü bilmez. |
| Hesap | `Study` | Seriden seri üreten deterministik fonksiyon. Akışkan, çok çıktılı, parametreli. |
| Olgu | `Fact` | Motorun tespit ettiği, tartışmasız, sayısal olay. "RSI(14) 70'i yukarı kesti, bar #1842." Kanıt verisini taşır. |
| Bulgu | `Finding` | Olgulardan çıkarılan iddia. İddia + dayandığı olgular + güven + geçersizlik koşulu + yazar (ajan/insan). |
| Kapsül | `Capsule` | Tek bir bulgunun insana sunulan, canlı, açılabilir, taşınabilir hali. |
| Anlam işareti | `Glyph` | Kapsülün tek bakışlık görseli. Veriyi değil anlamı çizer. Bir kez öğrenilir, her yerde aynı okunur. |
| Görünüm | `View` | Görsel gramerle yazılmış gösterim tarifi: paneller, katmanlar (işaret + veri + kodlama), ölçekler, vurgu. |
| Görsel gramer | `grammar` | Görünümleri kuran yapı taşları kümesi. |
| Derinlik | `Depth` | Anlam zoom'unun seviyesi: 0 cümle, 1 kapsül, 2 işaretli grafik, 3 tam çalışma alanı. |
| Doküman | `Document` | Seri bağları, görünümler, bulgular, kapsüller, çizimler ve niyetlerin tamamı. |
| Komut | `Command` | Dokümanı değiştiren tek yol. İnsan ve ajan aynı komutları kullanır. |
| Niyet | `Intent` | Kullanıcının bıraktığı kalıcı istek. "Yükseliş bozulursa haber ver." |
| Karne | `Ledger` | Bulguların sonradan gerçekle karşılaştırılmış kaydı. |
| Enstrüman | `Instrument` | İşlem gören varlık. Hangi borsada, hangi takvimle işlem gördüğünü taşır. |
| Takvim | `Calendar` | Borsanın saat dilimi, seans saatleri ve tatilleri. Bar hizası buna göre yapılır. |
| Gösterim saat dilimi | `display_tz` | Zamanın kullanıcıya hangi saat diliminde gösterileceği. Bar hizasını etkilemez. |
| Isınma | `warmup` | Bir hesabın ilk geçerli değerini üretmesi için gereken bar sayısı. Bu barlarda çıktı boştur (`null`); çıktı uzunluğu her zaman bar sayısına eşittir. |
| Hesap çıktısı | `StudyOutput` | Bir hesabın bir bar serisi üzerindeki sonucu: bar zamanları ve adlandırılmış çıktılar, her biri bar sayısı uzunluğunda; ısınma `null`. |
| Bar olayı | `BarEvent` | Seri kaynağının ürettiği adım: yeni bar ya da son barın düzeltilmesi (canlı bar). Hesabın akışkan ve "son barı düzelt" kiplerine birebir karşılık gelir. |
| Parametre taraması | `sweep` | Bir hesabın tek çağrıda çok sayıda parametre kümesiyle çalıştırılması; her sonuç tek tek çalıştırmayla bit-eşittir. |
| Köken | `Provenance` | Bir öğeyi kimin, neye dayanarak, ne zaman ürettiği. |
| Alan protokolü | `qp/0` | Motor, arayüzler ve ajanlar arasındaki konuşma; arayüz ve ajan aynı mesajları kullanır (`docs/PROTOCOL.md`). |
| Otorite | `authority` | Bir dokümanın tek yazma noktası: komutları sıraya koyar, günlüğe yazar, değişimi yayar. Belge başına tektir. |
| Kimlik | `Principal` | Konuşan kişi, ajan ya da besleme; rolü otoritenin yapılandırmasındadır. |
| Rol | `Role` | Kimliğin yetkisini belirler: `owner`, `editor`, `proposer`, `viewer`, `feed`, `host`. |
| Bağlantı | `connection` | Bir istemcinin otoriteyle konuştuğu kanal; `hello` ile başlar. |
| Yüzey | `Surface` | Bağlantının sunum ortamı: masaüstü, telefon, saat, ses, ekransız. |
| Profil | `Profile` | Otoritenin yüzeye göre seçtiği sunum derinliği ve veri sınırları (yetenek uzlaşması). |
| İşlem | `Op` | Komuttaki istek, anlam düzeyinde ("RSI ekle", "bulguyu reddet"). |
| Değişim | `ChangeOp` | İşlemin otoritede açıldığı ilkel, tam ve yeniden oynatılabilir hali. |
| Günlük | `LogEntry` / log | Uygulanmış girdilerin sıralı, boşluksuz listesi; her girdide yazar, onaylayan, istek, değişim ve ters değişim. Denetim izidir. |
| Öneri | `Proposal` | Onay bekleyen yazı: `bekliyor → kabul | ret | süresi doldu`. Dokümanın içinde durur. |
| Sorgu | `Query` | Dokümanı değiştirmeyen anlamsal okuma. |
| Abonelik | `Topic` | Bir bulgu, koşul, öneri, dikkat ya da değişim için haber isteği. |
| Girdi | `Gesture` | Dokunma, kaydırma, yakınlaştırma; motor yorumlar, dokümanı değiştirmez. |
| Varlık durumu | presence | Kalıcı olmayan oturum durumu: bakılan aralık, odak, dikkat yönergeleri. Günlüğe girmez. |
| Dikkat yönergesi | `AttentionDirective` | Bir kişinin nereye, hangi sırayla bakmasının istendiği. |
| Anlatım | `Narrative` | Bir bulgu hakkında sıralı, zamanlı adımlar; her adım bir şeyi gösterir ve bir cümle söyler. Dokümanda yaşar. |
| Sunucu | presenter | Bulgudan görünümü deterministik olarak kuran motor parçası; ajan anlam düzeyinde konuşur, sunucu gramere çevirir (K21-1). |
| Devir | hand-over / `epoch` | Otoritenin bir yerden başka yere (sunucu ↔ cihaz) taşınması; her devirde `epoch` artar. |
| Uygunluk dökümü | `Transcript` | Dilden ve taşımadan bağımsız "girdi mesajları → beklenen çıktı mesajları" senaryosu. |

### Görsel gramer (R2)

Ayrıntı ve örnekler `docs/GRAMMAR.md`.

| Kavram | Kodda | Tanım |
|--------|-------|-------|
| Panel | `Panel` | Görünümün yatay bir şeridi. Paneller üstten alta yığılır, zaman eksenini paylaşır; her birinin kendi değer ölçeği var. Kalıcı bir **numara kimliği** taşır (konum değildir). |
| Katman | `Layer` | Bir işaret + veri + kodlama kanalları. Görünüm içinde kalıcı kimliği var (`id`); çizim sırası katman sırasıdır. |
| İşaret (gramer) | `Mark` | Katmanın çizim biçimi: `candle`, `bar`, `line`, `area`, `band`, `rule`, `point`, `text`. (Anlam işareti `Glyph` ile karıştırılmamalı.) |
| Ölçek | `Scale` | Veri değerini eksene eşleyen kural: zaman (bar sırası, seans boşlukları kapalı), doğrusal, logaritmik. Kimliği var (`x`, `y<panel>`). |
| Kodlama kanalı | `Encoding` | Kapalı ve tipli kanal kümesi: `x`, `y`, `y2`, `color`, `opacity`, `width`, `size`, `text`. Bilinmeyen kanal reddedilir. |
| Veri başvurusu | `DataRef` | Bir katmanın ya da kanalın verisi: seri alanı (`c`, `upper`), başka bir serinin alanı, olgu ya da bulgu. Gramer veriye her zaman başvuruyla ulaşır, veriyi taşımaz. |
| Köken bağı | `Origin` | Bir katmandaki sayısal sabitin (kural seviyesi, koşul eşiği) nereden geldiği: olgu alanı, adlandırılmış eşik ya da seri değeri. Kökeni olmayan sabit otoritede reddedilir (ilke 6). |
| Adlandırılmış eşik | threshold | Bir hesabın motorca bilinen geleneksel seviyesi (`rsi.overbought` = 70, `macd.zero` = 0). Köken olarak kullanılır; liste `describe`'da. |
| Vurgu | `Emphasis` | Görünümün bir parçasını (bulgu, katman, bar aralığı) öne çıkarma, odaklama ya da karartma. Kalıcısı görünümde; dikkat yönergesi ve anlatım adımı aynı hedefle geçici olarak çizilir. |
| Kalıcı aralık | `x.domain` | Görünümün herkes için sabit yatay aralığı; komutla değişir, günlüğe girer. Bağlantının geçici bakışı (viewport) varlık durumudur. |
| Tema | `theme` | Renklerin ve yazı boyutlarının motor tarafındaki tanımı (`dark`, `light`). Görünüm yalnızca tema adını ve tema renk adlarını (`up`, `down`, `accent`…) taşır. |
| Sahne tarifi | `SceneDesc` | Görünüm + veri derlendikten sonraki, çizimden bağımsız ara biçim: paneller, ölçek eşlemeleri, katmanlar, veri öğeleri ve çizim ilkelleri. Çizici ve anlamsal okuma aynı tarifi okur (tek doğruluk kaynağı). |
| Gramer derleyicisi | `qp-grammar` | Görünüm + veri sağlayıcı → sahne tarifi. Hesap yapmaz; değerleri otoriteden (`qp-compute`) alır. |
| Sahne öğesi | `Item` | Sahne tarifinde veriye bağlı tek eleman (bir mum, bir çizgi noktası, bir çubuk): bar, zaman, değerler, piksel kutusu. |
| Nokta sorgusu | `hit` | Bir pikselde hangi öğenin çizildiğini (panel, katman, bar, zaman, değerler, veri başvurusu) sahne tarifinden veren sorgu. |
| Ayrıntı düzeyi | LOD | Görünen bar sayısı piksel sütunlarını aştığında barların sütun kovalarında birleştirilmesi (mumda ilk/en yüksek/en düşük/son; çizgide kova içi en küçük/en büyük). |
| Derinlik kuralı | depth rule | Bir bulgunun `sentence`, `capsule`, `chart` derinliklerinde ne gösterdiğini belirleyen gramer kuralı; sunucu (presenter) bu kuralları uygular. |
| Yüzey indirgemesi | profile reduction | Yüzey profilinin (`max_points`, `max_glyphs`, `series_data`) derinlik kuralının çıktısını kısması. |
| Doküman özeti | `DocSummary` | `welcome`'da gelen kısa doküman özeti: seriler, görünümler/paneller/katmanlar, olgular, bulgular; ajanın ilk mesajında başvurabilmesi için. |
| Anlamla başvuru | semantic reference | Bir öğeye kimlik yerine anlamıyla başvurmak ("RSI(14)'ün paneli", "en son kırılım bulgusu"). Otorite deterministik çözer; belirsizse seçenekleri sıralayan `ambiguous` hatası döner; günlükte çözülmüş kimlik saklanır. |

## Kapsülün yaşam döngüsü

`taslak → canlı → (güncellendi)* → geçersiz | süresi doldu | onaylandı | reddedildi`

- **Canlı:** dayandığı seriler değiştikçe kendini günceller.
- **Geçersiz:** geçersizlik koşulu gerçekleşti; kapsül bunu kendisi ilan eder.
- **Onay / ret:** insan girdisi; aynı dokümana yazılır, karneye işlenir.

## Anlam işaretleri (ilk aday küme)

Kesin liste R3'te tasarlanır. Başlangıç adayları:

| İşaret | Cevapladığı soru |
|--------|------------------|
| Yön | Nereye gidiyor? |
| Güç | Hareket ne kadar kuvvetli? |
| Belirsizlik | Ne kadar emin? |
| Aşırılık | Olağandışı mı? |
| Benzerlik | Daha önce oldu mu, sonra ne oldu? |
| Değişim | Son bakıştan beri ne değişti? |
