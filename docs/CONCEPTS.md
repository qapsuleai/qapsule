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
| Bar olayı | `BarEvent` | Seri kaynağının ürettiği adım: yeni bar ya da son barın düzeltilmesi (canlı bar). Hesabın akışkan ve "son barı düzelt" kiplerine birebir karşılık gelir. Alan düzeyindeki genişlemesi veri olayıdır (`DataEvent`). |
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
| Abonelik | `Topic` | Bir bulgu, koşul, öneri, dikkat, değişim ya da bir görünümün verisi (`view`: görünümün okuduğu bir serinin barları değişti, `view_changed`) için haber isteği. |
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
| Seçici | selector (`SeriesSel`, `PanelSel`, `LayerSel`…) | Anlamla başvurunun seçilen biçimi: kimlik yazılacak alana konan, öğeyi özellikleriyle tarif eden nesne (`{"study":"rsi","params":{"period":14}}`, `{"shows":{"study":"macd"}}`, `{"kind":"breakout","latest":true}`). Tek eşleşme kullanılır; çok eşleşme `ambiguous`; günlükte kimlik saklanır (`docs/PROTOCOL.md` §17). |
| Nokta serisi | `SeriesData::Points` | Bar olmayan, `t`/`v` noktalarından oluşan seri (ör. tahmin yelpazesinin bantları). Son bardan sonraki noktalar ardışık seans yuvalarına yerleşir (`docs/GRAMMAR.md` §3). |

### Veri düzlemi (RP2)

Piyasa verisi protokolün içinden akmaz (K23); kaynağından kendi biçiminde gelir. Ayrıntı
`docs/PROTOCOL.md` §18.

| Kavram | Kodda | Tanım |
|--------|-------|-------|
| Veri düzlemi | data plane | Bar verisinin ve canlı fiyatın aktığı yol: kaynaktan (Matriks, NATS, dosya, sentetik) motora, kendi biçiminde. Protokol mesajı değildir, günlüğe girmez. |
| Anlam düzlemi | meaning plane | Protokolün taşıdığı her şey: komutlar, değişimler, olgular, bulgular, kapsül durumları, koşul olayları. Veri taşımaz; veriye başvurur. |
| Veri bağı | `DataBinding` (`source.kind = "bind"`) | Bir serinin verisini taşımak yerine onu tarif eden bağ: enstrüman (sembol), periyot ve pencere; isteğe bağlı besleme adı. Dokümanda durur; her kullanıcının kendi veri yetkisiyle çözülür. |
| Çapa | `anchor_t` | Bağlı serinin bar 0'ının zamanı. Otorite bağı ilk çözdüğünde dokümana yazar; sonra her çözüm (otorite, her yansı) oradan başlar. Bar numaraları ve hesap başlangıcı her yerde aynı olur. |
| Pencere | `Window` | Bağın hangi barları istediği: son N bar (`{"last": n}`), bir zamandan bu yana (`{"from_t": t}`) ya da kapalı aralık (`{"from_t": a, "to_t": b}`). |
| Besleme | feed (`FeedKey`) | Aynı sembol ve periyodun tek veri akışı. Aynı beslemeye bağlı birden çok seri veriyi paylaşır. |
| Veri sağlayıcı | `SourceProvider` | Barındırıcının (sunucu, cihaz kabuğu) motora verdiği, bağı bir kişinin veri yetkisiyle çözen arayüz: yetki denetimi, ilk barlar, sonra veri olayları. (Gramer derleyicisinin `DataProvider`'ı başka bir şeydir: derleyiciye otoritenin değerlerini veren okuma arayüzü.) |
| Bağ durumu | `BindingState` | Bir bağın bir kişi için çözüm sonucu: `ready` (barlar hazır), `pending` (bekleniyor), `no_entitlement` (yetki yok), `no_data` (kaynakta veri yok), `stale` (sıra boşluğu ya da boyut sınırı; yeniden çözülene kadar değer verilmez). Çözülemeyen bağ belgenin geri kalanını durdurmaz. |
| Veri olayı | `DataEvent` | Veri kaynağının alan düzeyindeki adımı: yeni bar (`bar`) ya da var olan bir barın **yalnızca değişen alanları** (`patch`): son barda canlı güncelleme, daha eski barda düzeltme. Her olay beslemenin kaynak sıra numarasını (`seq`) taşır. `BarEvent`'in genişlemesidir. |
| Veri konumu | `DataPosition` | Bir beslemenin bir tarafta (otorite ya da yansı) ulaştığı yer: bar sayısı, son bar zamanı, uygulanan olay sayısı ve o ana kadarki verinin özeti. İki tarafın aynı veriye bakıp bakmadığı bununla karşılaştırılır. |
| Veri özeti | `data_digest` | Bir bar aralığının içerik özeti: barların zaman ve değerlerinin bit düzeyindeki baytları üzerinden belirleyici bir karma (v0: FNV-1a 64; kriptografik değil: düzeltmeyi yakalar, kasıtlı sahteciliği değil). |
| Kanıt başvurusu | `EvidenceRef` | Bir olgunun dayandığı veriyi gösteren dondurulmuş başvuru: hangi seri ve bağ, hangi zaman aralığı, kaç bar ve o aralığın veri özeti. Otorite olgu eklenirken damgalar. "Bu iddia o gün şu veriye dayanıyordu" sonradan sınanabilir. |
| Kanıt doğrulama | `verify_evidence` | Bir olgu ya da bulgunun kanıt başvurularını bugünkü veriye karşı sınayan sorgu: veri özeti aynı mı, olgunun kanıt değerleri yeniden hesapta aynı mı; değilse farkı bildirir. |
| Veri yetkisi | entitlement | Bir kişinin bir beslemeyi görme hakkı (borsa veri lisansı). Rol yetkisinden (`Role`) ayrıdır; veri sağlayıcı denetler. |

### Tutarlılık (RP2)

Otorite ile yansının aynı veriye bakıp bakmadığı ve yansının ne zaman neyi gösterdiği.
Ayrıntı `docs/PROTOCOL.md` §19; seçimin gerekçesi `rounds/RP2/experiments/T7-COMPARE.md`.

| Kavram | Kodda | Tanım |
|--------|-------|-------|
| Yansı | `Replica` | Otoritenin dokümanını `document` + `change` olaylarından birebir izleyen, bağlı serileri **kendi** veri sağlayıcısından ve kendi yetkisiyle çapadan çözen, otoriteyle **aynı motor kodunu** çalıştıran taraf (cihaz, tarayıcı). Dokümana yazmaz; kendi motor girdisi üretmez. |
| İddia | claim (`ClaimRef`) | Otoritenin veriden çıkardığı anlam: bir koşulun bir barda doğru olması (`condition`), bir niyetin tetiklenmesi (`intent`), bir kapsülün geçersizleşmesi (`capsule`). Kimliği: neyin iddiası olduğu + bar zamanı `t`. |
| İddia kanıtı | `ClaimEvidence` | Otoritenin veri kaynaklı her anlam mesajına eklediği kanıt: iddianın bar serisi, bar zamanı `t`, kanıt değerleri, olgunluk, temizlik ve çapa. Veri değildir (bar taşımaz); iddianın dayandığı birkaç sayıdır. |
| Kanıt değeri | `EvidenceValue` | İddianın dayandığı bir işlenenin (koşuldaki serinin ana çıktısı) `t`'deki ve bir önceki bardaki değeri (`value`, `prev`). Yansı kendi verisinde aynı yüklemi bu barlarda sınar. |
| Olgunluk | `Maturity` | İddia barının bir taraftaki durumu: `provisional` (geçici: bar o tarafın verisinde henüz kapanmadı, canlı bar) ya da `final` (kesin: bar kapandı; ondan sonra bar var ya da pencere kapalı). Yansıda gösterilen olgunluk iki tarafınkinin küçüğüdür. |
| Temizlik | `clean` | Bir tarafın verisinin iddia barına kadar onarılmamış sıra boşluğu (kaybolmuş olay) içermemesi. Kesin gösterim için iki taraftan en az birinin temiz olması gerekir. |
| İddia durumu | `ClaimState` | Otoritenin bir iddia hakkındaki son sözü: `asserted` (ileri sürüyor; geçici ya da kesin) ya da `withdrawn` (geri çekti: bar kapandığında ya da düzeltmeden sonra yüklem artık tutmuyor). |
| İddia aboneliği | `claims` konusu | İddiaların yaşam döngüsünü (`claim` olayı: geçici, kesin, geri çekildi, temizlik değişti) kanıtıyla taşıyan abonelik konusu. Yansının dinlediği konu. |
| Görüntü kuralı | display rule | Yansının bir iddiayı ne zaman gösterdiği: otorite ileri sürüyor, yansının kendi verisi `t`'yi aynı çapayla kapsıyor ve iddianın yüklemi **kendi verisinde** tutuyor; veri her değiştiğinde aynı adımda yeniden sınanır. Kesin için kendi barı kapalı + otorite kesin + iki taraftan biri temiz. Gösterilen değerler yansının kendi değerleridir. |
| Yansı görüntü durumu | `DisplayState` | Yansının her iddia için gösterdiği: `pending` (bekliyor: otorite bir şey buldu, kendi verisi henüz kapsamıyor ya da doğrulamıyor), `shown` (gösteriliyor: geçici ya da kesin, kendi değerleriyle), `disputed` (tartışmalı: temiz ve yeniden çekilmiş kendi verisi iddiayı tutmuyor; iddia olarak gösterilmez). Belirleyicidir. |
| Kesin bar çekme | settled fetch | Barındırıcının yansı (ya da otorite) için veri sağlayıcının kesinleşmiş barlarını (Matriks REST karşılığı) çekmesi; anlaşmazlıkta ve boşluk onarımında kullanılır. Veri düzlemindedir. |
| Doküman özeti / görünüm özeti | `document_digest` / `view_digest` | Dokümanın kanonik JSON baytlarının ve bir görünümün sahne tarifinin (`SceneDesc`) FNV-1a 64 özeti. Otorite ile yansının aynı dokümanı ve aynı görüntüyü ürettiği bununla karşılaştırılır. |

### Kapsül (R3)

Çerçeve `docs/CAPSULE.md`; uygulama ayrıntısı aynı dosyanın §9'u; şema `qp-schema::capsule`.

| Kavram | Kodda | Tanım |
|--------|-------|-------|
| Olgu dedektörü | `Detector` (`qp-facts`) | Bar serisinden (ve varsa lisanslı akıştan) tipli olgu üreten deterministik akışkan fonksiyon. Toplu ve akışkan çalıştırma bit düzeyinde aynıdır; açık barda verilen sonuç (`push` ya da canlı düzeltmede `update_last`: "bar şimdi kapansaydı") adaydır; bar kesinleşince olgu olur. |
| Anlam türü | `MeaningKind` | Olgunun altı anlamından biri: devam (`continuation`), dönüş (`reversal`), kırılım (`breakout`), sıkışma (`squeeze`), aşırılık (`extreme`), akış (`flow`). İşaretin şekli bunu kodlar. |
| Yön | `Direction` | `up`, `down`, `neutral` (belirsiz). |
| Olgu anlamı | `FactMeaning` | Olgunun `meaning` alanı: tür, yön, kanıt ailesi, zaman türü, dedektör ve sürümü, enstrüman, güç, lisans, geometri, koşul, sunucu olgusu bayrağı. |
| Kanıt ailesi | `EvidenceFamily` | Olgunun dayandığı veri ailesi: fiyat, hacim, akış, türev, haber/temel. Aynı aileden birden çok olgu tek teyit sayılır. |
| Teyit sayısı | `confirmations` | Bir hikâyeyi destekleyen **farklı** kanıt ailesi sayısı. Bulguda ve işarette görünür. |
| Lisans sınıfı | `License`, `LicenseClass` | Olgunun dayandığı verinin sınıfı: `public` (kullanıcının kendi bar yetkisiyle yansıda yeniden hesaplanır) ya da `licensed` (yalnız sunucuda hesaplanır; yetkisize rakamsız yorum). Gereken veri yetkisinin adını taşır. |
| Zaman türü | `TimeKind` | Kapsülün dili ve ömrü: olay (`event`, geçmiş zaman + ne zaman), durum (`state`, şimdiki zaman, koşul sürdükçe), beklenti (`expectation`, koşullu). |
| Geometri | `Geometry` | Olgunun `chart` derinliğinde çizilen kanıtı: seviye, bant, işaretli noktalar. Değer taşımaz, olgunun kanıt anahtarlarına başvurur; kanıt gizlenince geometrinin sayıları da gizlenir. |
| Anlam koşulu | `MeaningCondition` | Beklentinin ya da durumun makinece sınanabilir koşulu (konu, karşılaştırma, kanıt anahtarı, kaç bar içinde). Cümlenin yuvasıdır, serbest metin değildir. |
| Hikâye | `Story` | Kapsülün `story` alanı: aynı enstrüman ve tema için tek kapsülün evrilen kaydı: tema, tür, yön, aşama, aileler, güç, önem ve kademe, doğum barı, ömür (bar), karne anahtarı, doğumdaki karne, güven yolu, geçişler. |
| Tema | `theme` | Hangi olguların aynı hikâyeye ait olduğunu belirleyen anahtar (tema kimliği kuralı, M4). |
| Aşama | `Stage` | Hikâyenin yaşam döngüsündeki yeri: `candidate` (aday), `born` (doğdu), `strengthened` (güçlendi), `weakened` (zayıfladı), `confirmed` (teyit edildi), `realized` (gerçekleşti), `invalidated` (geçersiz), `expired` (süresi doldu), `retracted` (geri çekildi). Son dördü kapatır. |
| Geçiş | `Transition` | Hikâyede bir aşamaya giriş: bar, zaman, neden olan olgular, makinece okunur neden (`new_family`, `outcome_hit`, `horizon`, `data_corrected`…). Dökümde görünür. |
| Aday | `Stage::Candidate` | Açık barda yerel veriden görülen oluşum. Yalnız o enstrümanı izleyen yansıda; otorite günlüğüne ve karneye girmez; bar kapanınca kesinleşir ya da kaybolur. |
| Kesinleşme | finalize | Bar kapanınca otoritenin hikâyeye kimlik vermesi ve günlüğe yazması. |
| Eskime | aging | Hikâyenin bar sayısıyla ölçülen doğal ömrü (`horizon_bars`), açık geçersizlik koşulu ve sonuç. Saatle ölçülmez. |
| Sonuç kuralı | outcome rule (M5) | Tür başına "gerçekleşti" tanımı ve ufku (bar). Yalnız hikâyenin kapanış barına kadarki veriyi okur. |
| Karne istatistiği | `LedgerStat` | Bir karne anahtarı (tür + dedektör + sürüm) için kapanmış kapsül sayısı ve gerçekleşenler. "Geri çekildi" sayılmaz. |
| Doğumdaki karne | `ledger_at_birth` | Kapsülün doğduğu barda görünen karne: yalnız o bardan **önce** kapanmış kapsüllerden (ileriye bakma yok). |
| İleriye bakma | look-ahead | Bir değerin, ait olduğu andan sonraki veriye bağlı olması. Karne ve sonuç kuralında yasaktır; test: gelecek veri karıştırılınca geçmiş karne değişmez. |
| Önem | `importance`, `tier` | Sıralama kuralının (M3) hikâyeye verdiği puan ve kademe (1 en önemli … 3). Hassasiyet kademeye göre süzer. |
| Hassasiyet | `Sensitivity` | Kullanıcının seçtiği dikkat bütçesi: `important` (yalnız önemliler, kademe 1), `balanced` (dengeli, 1–2), `all` (hepsi). Sistem kendiliğinden değiştirmez. |
| Kullanıcı grubu | `UserGroup` | `habituated` (alışkın; varsayılan `all`) ya da `future` (geleceğin yatırımcısı; varsayılan `important`). |
| Kitle | `Audience` | Dokümandaki kişi başına kapsül tercihleri: grup, hassasiyet, izleme listesi, pozisyonlar. Pozisyon ve niyetle ilgili kapsüller hassasiyetten bağımsız gelir. |
| Sunucu olgusu | server fact (`FactMeaning::server`) | Lisanslı veriden yalnız sunucuda hesaplanan olgu. Yansı kendi verisinde sınayamaz. |
| Güven yolu | `Trust` | Kapsülün kullanıcıya nasıl doğrulanarak geldiği: `replica` (yansı kendi verisinde sınar, §19) ya da `server` ("sunucu tarafından doğrulandı" işaretiyle, K38; tasarım M6). |
| Nitel yorum | qualitative comment | Lisanslı olgunun yetkisize giden rakamsız cümlesi (üstünde, altında, arttı, azaldı). Ham değerin geri çıkarılamayacağı biçimde kurulur. |
| Geri çıkarma saldırısı | inversion attack | Art arda gelen nitel yorumlardan ham değerin aralığını daraltma denemesi. Güven yolunun ölçüsüdür (M6). |
| İtiraz | `dispute` | Yansının, otoritenin kesin iddiasını kendi temiz ve yeniden çekilmiş verisinde tutmadığını otoriteye bildirmesi (A20, M7). |
| Geri çekme | retraction | Otoritenin, dayandığı veri hatalı çıktığı için bir iddiayı/kapsülü düşürmesi (K37). Karneye yazılmaz. |
| İşaret alfabesi | glyph alphabet (M1) | Anlam işaretlerinin çizim kuralları: şekil = tür, yön, büyüklük = güç, doluluk = karne, çizgi biçimi = aşama, solukluk = tazelik, teyit işaretleri = aile sayısı, sunucu işareti. |
| İşaret tarifi | `GlyphSpec` | Bir anlam işaretinin motorca üretilmiş parametreleri (yukarıdaki kanallar). Gramerde `glyph` işaretinin verisidir. |
| İşaret aşaması | `GlyphPhase` | Çizgi biçiminin kodladığı aşama: aday, kesin, gerçekleşti, gerçekleşmedi, geri çekildi. |
| Gerçekleştirici | realizer (M2) | Hikâyeden cümleyi üreten deterministik dil parçası (şablon dilbilgisi; Türkçe ve İngilizce). |
| Okuyucu | `Reader` (`qp_capsule::sentence`) | Cümleyi ve kanıtı okuyan kişinin veri yetkileri (§18.3 biçimi). Okuyucu bir olgunun rakamlarını görmeye yetkili değilse cümle hiç rakam içermez (`redacted`), kanıt değerleri ve veri özeti ona gitmez. |
| Okuyucuya göre izdüşüm | projection (`qp-session`) | Otoritenin bir belgeyi, değişimi ya da kapsülü belli bir kişiye göndermeden önce onun veri yetkisine göre süzmesi: yetkisi olmayan lisanslı olgu ve yalnız ona dayanan hikâye hiç gitmez; bar yetkisi olmayana kanıt değerleri ve veri özeti çıkarılır. Saklanan doküman ve günlük değişmez. |
| Açık dünya | public world (`qp-session`, `public.rs`) | Lisanslı akış okunurken otoritenin tuttuğu ikinci kapsül dünyası: yalnız açık lisans sınıfındaki olgularla beslenen ikinci motorun (**açık-yalnız motor**) yazdığı doküman ve günlük; otoritenin günlüğüyle girdi girdiye eşleşir. Lisanslı akışların hepsine yetkili olmayan okuyucu (**açık görünüm okuyucusu**) kapsülle ilgili her şeyi buradan alır; böylece aldığı, mühür iletileri dışında, lisanslı veriden bağımsızdır (girişimsizlik, R3/K1). Saklanmaz, gönderilmez; otoritenin girdilerinin fonksiyonudur. |
| Kapsül motoru (otoritede) | capsule runner (`qp-session`) | Otoritenin, açıkça açılmış bar serilerinde (`init.capsules`) her kesinleşen barda dedektörleri ve kapsül motorunu (`EngineConfig::r3`) koşturup olguları, bulguları, kapsülleri ve karne satırlarını motor girdisi (`qp`) olarak günlüğe yazması. Her kesinleşme turu tam bir motor girdisi yazar (boş da olsa): günlük sırası lisanslı bir hikâyenin varlığını söylemez. |
| Mühür | `qp_capsule::trust::Muhur` (M6 seçimi) | Lisanslı akış için güven yolu: ham değer gelir gelmez işaret + ikili büyüklük basamağına indirgenir; ileti yalnız "olağan düzeyinin belirgin biçimde üzerinde" der, `trust = server` ve makbuz taşır. Yetkisiz okuyucu lisanslı akış hikâyesini **yalnız** mühür iletileriyle görür. |
| Sunucu yorumu | `ServerComment` | Mühürün yetkisize giden iletisi (protokolde): enstrüman, bar, zaman, kodlar, TR/EN cümle, `trust = server`, ekler (gereken yetki, neye dayandığı, makbuz). Rakam içermez. |
| Kapsül konusu | `capsules` topic | Abonenin kitlesine (izleme listesi, hassasiyet; pozisyon ve niyet geçişi) göre süzülmüş hikâye geçişleri; yetkisiz aboneye lisanslı hikâye yerine sunucu yorumları. |
| Kitle işlemleri | `set_audience`, `set_sensitivity` | Kişinin kendi kapsül tercihlerini (grup, hassasiyet, izleme listesi, pozisyonlar) dokümana yazan işlemler; başkasınınkini yalnız `owner` yazar. Sistem bunları kendiliğinden yazmaz. |
| İlgili kapsül | `relevant` | Okuyucunun pozisyonu ya da etkin niyetiyle ilgili enstrümanın kapsülü; hassasiyet kademesinden bağımsız geçer (§9.8). |
| Aday okuması | `Replica::candidates` | Yansının açık bardaki adayları kendi verisinden hesaplaması; otoriteye, günlüğe ve karneye hiç girmez. |
| Kart paneli | `card` paneli (`qp_grammar::depth::capsule_card_view`) | Kapsül derinliğinde işaret, cümle ve karne satırını taşıyan panel: bütün katmanları panele göre konumlanır; ızgara ve eksen çizilmez (`docs/GRAMMAR.md` §18). |
| Panele göre konum | `Position::Panel` | Bir katman öğesinin konumunun veri değeriyle değil panelin kesriyle ve piksel kaydırmasıyla verilmesi (`{"panel":0.5,"px":-13}`). Sayı taşımaz, kökeni gerekmez. |

### Gösterim (R4)

Çerçeve `rounds/R4/GOAL.md`; kabuk sınırı `docs/ARCHITECTURE.md` "Kabuk sınırı (R4)";
yeniden oynatma `docs/PROTOCOL.md` §21; kapsülün yüzü `docs/FACE.md`.

| Kavram | Kodda | Tanım |
|--------|-------|-------|
| Yeniden oynatma | replay (`qp_source::replay`) | Geçmiş barları (yerel gerçek veri ya da sentetik pazar) canlı akış gibi, **alan düzeyinde veri olayları** (`DataEvent`) ve saat girdisi olarak sıralı veren belirleyici kaynak (K44). Aynı veri + aynı denetim girdileri → bayt düzeyinde aynı olay ve mesaj akışı. R6'da yalnız kaynak değişir. |
| Oynatma saati | replay clock | Yeniden oynatmanın ürettiği sanal zaman; barındırıcı onu otoriteye `clock` olarak verir. Duvar saati yalnız **ne zaman** gönderileceğini belirler, **ne** gönderileceğini değil. |
| Oynatma konumu | `ReplayCursor` | Yeniden oynatmanın ulaştığı yer: sanal zaman ve her beslemede verilmiş olay sayısı. Belirleyicidir; konum + veri → sonraki olaylar. |
| Oynatma denetimi | `replay` mesajı (`ReplayControl`) | Barındırıcıya giden protokol mesajı: hız, duraklat, sürdür, adım, tarihe git, durum. Dokümana yazmaz, günlüğe girmez; geriye gitmek oturumu baştan kurup o konuma kadar belirleyici olarak ilerletmektir. Ajan aynı mesajlarla sürer. |
| Gösterim | demo (`qp demo`, `scripts/demo.sh`) | Tek makinede çalışan yerel barındırıcı (otorite + yeniden oynatma + statik sayfa) ve web kabuğu. Dış ağa istek yapmaz. |
| Kabuk | shell (`shells/web/`) | İnce istemci: sahne tarifini boyar, ham girdiyi protokole iletir, protokolden geleni gösterir. Alan mantığı (hesap, tespit, sıralama, cümle kurma, yerleşim) taşımaz. |
| Düzen öğesi | chrome | Kabuğa ait, alan bilgisi taşımayan arayüz parçası: liste kaydırma, sekme, ayar sayfası, düğme. Neyi hangi sırayla göstereceğini protokolden alır. |
| Kapsülün yüzü | face (`Depth::Capsule` yüz kuralı) | Kapalı kapsülün tek bakışlık hali: bir başlık cümlesi, bir işaret ve en çok iki ek bilgi (K43). Gramerle (derinlik kuralı) kurulur; kabuk yalnız boyar. |
| Başlık cümlesi | `headline` (yüz cümlesi) | Kapalı yüzdeki tek cümle: ne oldu, ne zaman, hâlâ geçerli mi. Gündelik dil; iç terim ("aile", "teyit", "bar") yok. Şablon dilbilgisinden, TR ve EN. |
| Ayrıntı cümlesi | `detail` | Kapsül açılınca gelen cümleler: kanıt, karnenin tam hali, koşul, ne olursa ne olur. |
| Ek bilgi | face chip (`FaceChip`) | Yüzdeki en çok iki küçük bilgi (ör. kısaltılmış karne, "az örnekli", ne zaman). Dürüstlük bilgisi gizlenmez, kısaltılır. |
| Süre ifadesi | elapsed phrase (`qp_capsule::realize::elapsed`) | Bar farkını periyoda ve borsa takvimine göre gündelik süreye çeviren ifade: "bugün", "dün", "6 işlem günü önce", "2 saat önce", "geçen hafta". Takvimden (`qp-calendar`) hesaplanır; hafta sonu ve tatil işlem günü sayılmaz. |
| Geçerlilik | validity (`Validity`) | Kapsülün iddiasının **bugün** hâlâ geçerli olup olmadığı: `valid` (geçerli), `no_longer` (durum koşulu artık tutmuyor, hikâye henüz kapanmadı), `closed` (hikâye kapandı: gerçekleşti, tutmadı, süresi doldu, geri çekildi). Başlık ve işaret aynı şeyi söyler (K42). |
| Durum koşulu | state condition | Durumun (`state`) **şu an** sürüp sürmediğini söyleyen koşul ("RSI 70'in üstünde"). Geçerliliği belirler. |
| Beklenen çözüm | expected resolution | Hikâyenin nasıl kapanacağını söyleyen sonuç kuralı (bariyer, ufuk). Durum koşulundan ayrıdır (K42/A28): durum bitmiş, hikâye hâlâ açık olabilir. |
| Karne yönü | directional ledger key | Karne anahtarına yönün eklenmesi: `<tür>/<yön>/<dedektör>@<sürüm>` (K42/A23). Yukarı ve aşağı kırılım ayrı sayılır. |
| Olumsuz geçiş | negative transition (`Stage::is_negative`) | Hikâyenin `weakened` (zayıfladı), `invalidated` (geçersiz) ya da `retracted` (geri çekildi) geçişi. Pozisyon ve niyetle ilgili kapsülde yalnız bunlar hassasiyetten bağımsız gelir; doğum seviyeye tabidir (K42/A22). |
| Kapsül seçicisi | `CapsuleSel` | Kapsülü nitelikleriyle tarif eden seçici (`instrument`, `meaning`, `direction`, `stage`, `open`, `born_bar`, `series`, `first`/`latest`); sıra son geçiş barı. `capsule` sorgusu kimlik yerine alır (PROTOCOL §17.1, R4). |
| Başlık yapı taşları | headline parts (`qp_capsule::realize::parts`) | Kısa başlık cümlesinin parçaları: olay adı, ne zaman (süre ifadesi), geçerlilik, yön, kısaltılmış karne; iki dilde, iç terimsiz, yetkisize rakamsız. Başlığı yüz tasarımı (M8) bunlardan kurar. |
| Akış | feed | Kapsüllerin çekirdekçe seçilmiş ve sıralanmış listesi (kişinin kitlesine göre). Sırayı ve gruplamayı çekirdek verir; kabuk yalnız alt alta dizer (düzen M9). |
| Açılma | open (depth transition) | Bir kapsülün derinlikler arasında geçişi: cümle → kapsül → işaretli grafik. Kabuktan `input` ile istenir; hangi öğelerin ortak kaldığını çekirdek belirler (M10). |
| Bakış durumu | view state (`ViewState`, presence) | Bağlantı başına derinlik, açık kapsül, o derinlikteki görünüm, grafik aralığı ve kabuğun bildirdiği çerçeve boyutları. Varlık durumudur; günlüğe girmez, dokümanı değiştirmez (§4.6, §7). Akış listesindeki kaydırma konumu kabuğun düzen öğesidir, bakış durumuna girmez. |
| Sonuç işareti | outcome mark | Kapanmış kapsülün işaretinde "gerçekleşti" ile "tutmadı"yı ayıran kanal (M11). Renksiz de okunur. |
| Ölçüm ekranı | metrics page | Gösterimde kare süresi (durağan, kaydırırken), tik maliyeti, açılış süresi ve paket boyutunu gösteren ve kopyalanabilir özet üreten sayfa. |
| Persona | persona | Ölçümde ekran görüntüsünü okuyan taze alt ajanın rolü: geleceğin yatırımcısı (teknik analiz bilmez, beş saniyesi var) ya da alışkın kullanıcı. İnsan değildir; ön elemedir (K39). |

## Kapsülün yaşam döngüsü

R3'ten itibaren kapsülün hikâye aşamaları `Stage`'dir (yukarıda; `docs/CAPSULE.md` §3).
Eski `CapsuleState` (aşağıdaki) protokol uyumu için kalır: `born…confirmed` → `live`,
`realized` → `live` kapanışı karnede, `invalidated` → `invalidated`, `expired` → `expired`,
`retracted` → `invalidated` + karne yok.

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
