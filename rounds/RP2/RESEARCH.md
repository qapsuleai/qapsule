# RP2 — Araştırma: veri düzlemi ile anlam düzleminin tutarlılığı (T6)

Tarih: 2026-11-15. Görev: `rounds/RP2/TASKS.md` T6. Bu belge **araştırma ve tasarım
adaylarıdır**; hiçbir ölçüm sonucu içermez. Ölçüm T7'de, burada önerilen düzenekle
yapılır. Sıralama (§6) açıkça bir **hipotezdir**.

Kaynaklar bu oturumda web aramasıyla künyeleri üzerinden doğrulandı; doğrulanamayan ya da
yalnızca ikincil kaynaktan görülen yerler **[DOĞRULANMADI]** ya da **[ikincil]** diye
işaretli. Künyesi bellekten yazılan ve bu oturumda ayrıca aranmayan klasikler
**[bellekten]** diye işaretli. Hiçbir kaynağın sonucu "ölçüm" diye aktarılmadı.

Okunan depo belgeleri: `NORTH_STAR.md`, `docs/CONCEPTS.md`, `docs/ARCHITECTURE.md`,
`docs/PROTOCOL.md` (§2, §4.3–4.5, §8, §11, §15), `DECISIONS.md` (K18, K21–K26, Ö1–Ö9),
`rounds/RD1/experiments/T6d-handover.md`, `docs/MATRIKS_PROTOCOL.md` §9–§11,
`rounds/RP2/GOAL.md`, `TASKS.md`, `VERIFY.md`, `crates/qp-schema/src/types.rs` (`Fact`).

---

## 1. Problem

### 1.1 Durum

K23 ile piyasa verisi protokolden çıktı. Otorite (sunucu) ve her yansı (cihaz) barları
**kendi** besleme bağlantısından alır (Matriks: MQTT üzerinde ikili protobuf;
`SymbolMessage` `REFRESH` = tam anlık görüntü, `UPDATE` = yalnızca değişen alanlar;
`TradeMessage` işlem başına; QoS 0, temiz oturum; `docs/MATRIKS_PROTOCOL.md` §9, §11).
İki uç aynı motor kodunu çalıştırır ve aynı girdiden bit-eşit sonuç üretir (Ö3, T6d).
Protokol yalnızca **anlam** taşır: olgular ("RSI(14) 70'i yukarı kesti, bar #1842"),
bulgular, kapsül durum değişimleri (`invalidated`), `condition_met`, `intent_triggered`.

Soru: otorite "RSI 70'i kesti" dediğinde yansı **aynı veriye** baktığını nasıl bilir?
Ürünün şartı (`VERIFY.md` #10): **yanlış ya da erken gösterilen bulgu 0**; bulgu
kanıtıyla, doğru zamanda gösterilir (Vazgeçilmez: "kapsül her zaman kanıtına açılabilir";
K26: cümle olgunun zamanını ve bugünkü geçerliliğini yansıtır).

### 1.2 Belirleyicilik yetmez — çünkü girdiler aynı değil

Aynı kod aynı girdiden aynı biti verir; ama iki ucun **girdisi** aynı değildir:

1. **Besleme farkı.** QoS 0 ve (bildiğimiz kadarıyla) sıra numarasız akışta düşen bir
   `UPDATE` sessizce kaybolur. Bar içi en yüksek fiyatı taşıyan tek bir tik düşerse
   iki ucun barı farklıdır. Matriks akışında sıra numarası olup olmadığı belgelenmemiş
   (`MATRIKS_PROTOCOL.md` §13 açık sorular) **[DOĞRULANMADI]**.
2. **Gecikme farkı.** Yansının beslemesi otoritenin anlam mesajından geç kalırsa yansı
   iddiayı, iddianın dayandığı bar kendisinde yokken alır.
3. **Canlı bar.** Son bar saniyede onlarca kez değişir; bar içinde bir kesişim belirip
   kaybolabilir. İki ucun canlı barı, tiklerin geliş anına göre **neredeyse hiçbir zaman**
   bit-eşit değildir.
4. **Geçmiş düzeltmesi.** Veri sağlayıcısı geçmiş barı düzeltir; bölünme/temettü tüm
   geçmişi ölçekler.
5. **Hesap başlangıcı farkı (gözden kaçması kolay).** RSI'ın Wilder yumuşatması
   özyinelemelidir: bar *k*'nin bar *i*'deki etkisi kabaca (13/14)^(i−k) ile söner.
   Bu etkinin f64'ün bağıl duyarlılığı 2⁻⁵³'ün altına inmesi için
   53·ln2 / ln(14/13) ≈ **496 bar** gerekir (hesap, ölçüm değil). Yani otorite 10.000
   bardan, yansı "son 500 bar"dan başlarsa RSI değerleri **aynı bar için farklı bitler**
   verir; eşik yakınında yüklem bile farklı çıkabilir. Bağ (K23) yalnız "ne gösteriliyor"u
   değil **"hesap nereden başlıyor"u** da sabitlemelidir.
6. **Bar kimliği.** "Bar #1842" sıra numarası bağın penceresine göredir; iki uçta pencere
   farklı başlıyorsa aynı numara farklı bardır. Kimlik **bar açılış zamanı** (`t`, K14
   takvim hizası) olmalı; sıra numarası ikincil.
7. **Otoritenin kendisi de yanılabilir.** Otoritenin beslemesi de tik kaçırabilir.
   "Doğru"nun tanımı otoritenin dediği değil, sağlayıcının **kesinleşmiş** barıdır (§1.3).
8. **Protokol kanalının gecikmesi ve sırası.** `change` olaylarında `seq` var ve boşluk
   tespit ediliyor (§4.3); ama anlam mesajı ile veri ayrı kanallardan geldiği için
   kanallar **arası** sıra garantisi yok.
9. **Yeniden bağlanma.** Besleme yeniden bağlanınca `REFRESH` gelir ama arada kaçan
   tikler gelmez (Matriks'te geri doldurma yok, §11); protokol yeniden bağlanınca anlık
   görüntü ya da günlük gelir.

Not: `Fact` tipi bugün zaten `evidence: BTreeMap<String, f64>` taşıyor
(`crates/qp-schema/src/types.rs`), ama kanıtın **hangi veriye** dayandığını (bağ, bar
aralığı, özet) taşımıyor. Yaklaşım B bu alanı kullanır; dondurulmuş kanıt (K23) eksik
olan ikinci yarıdır.

### 1.3 Tanımlar (sözlük adayları)

Aşağıdaki adlar `docs/CONCEPTS.md` için **adaydır**; sözlük bu görevde değiştirilmedi.

| Aday kavram | Kodda (öneri) | Tanım |
|-------------|---------------|-------|
| Gerçek bar | true bar | Sağlayıcının o bar için kesinleşmiş değeri (Matriks'te REST tarihsel bar uçları). Ölçümde hakemin bildiği değer. Düzeltme gelirse gerçek bar değişir; her değişiklik bir **veri dönemi** açar. |
| Veri konumu | `DataPos` | Bir ucun bir bağ için o anda elinde olan verinin yeri: `(bağ, son kapanmış bar zamanı t_W, canlı bar zamanı, canlı bar düzeltme sayacı, veri dönemi)`. Çok bağlı iddiada konum bağ başına bir bileşenli **vektördür**. |
| Veri filigranı | data watermark | "t_W ve öncesindeki barlar bu uçta kapanmıştır ve artık (düzeltme hariç) değişmeyecektir" beyanı. Akış işlemedeki filigranın bar düzeyindeki karşılığı. |
| Veri özeti | `DataDigest` | Barların kanonik baytlarının (t i64 LE + o,h,l,c,v f64 bitleri LE) özeti. İki biçimi var: **özet zinciri** `h_i = H(h_{i−1} ‖ bar_i)` (önekin tamamına bağlanır) ve **aralık özeti** `H(bar_a ‖ … ‖ bar_b)` (yalnızca kanıt penceresine). |
| Hesap başlangıcı | compute anchor | Hesabın ilk barı (bağın çözümlediği ilk bar zamanı). Aynı biti üretmek için iki uçta aynı olmalıdır (§1.2/5). |
| Kanıt başvurusu | `EvidenceRef` | Bir olgunun dayandığı veri: bağ, `t_from..t_to`, aralık özeti, hesap başlangıcı, kanıt değerleri. Dondurulmuş kanıtla (K23) **aynı nesne**. |
| Geçici olgu | provisional fact | Canlı (kapanmamış) bar üzerinde doğru olan olgu. Bar kapanınca **kesinleşir** ya da **geri çekilir**. Her zaman "geçici" etiketiyle gösterilir. |
| Kesinleşme | seal / final | Olgunun dayandığı barların hepsi filigranın gerisinde ve otorite (tercihen sağlayıcının kesinleşmiş barına karşı) olguyu onaylamış. |
| Uyuşmazlık | dispute | Yansının elinde kanıt aralığı var ama yansının verisi otoritenin kanıtını doğrulamıyor. |
| Görüntü günlüğü | display log | Yansının kullanıcıya ne gösterdiğinin zaman damgalı kaydı (göster, gizle, etiket değiştir). Ölçümün tek girdisi. |
| Hakem | oracle | Ölçüm düzeneğinde gerçek barları ve her iki ucun veri konumunu bilen, iddiaların doğruluğunu ve görünürlüğünü hesaplayan bileşen. |

**Gösterim hataları.** Bir iddia *c* için: `ev(c)` = dayandığı bar aralığı ve bağlar;
`τ_show` = yansının *c*'yi (belli bir etiketle) göstermeye başladığı sanal zaman.

| Aday kavram | Tanım (ölçülebilir) |
|-------------|---------------------|
| **Erken gösterim** | `τ_show` anında yansının **görünen verisi** `ev(c)`'yi kapsamıyor (en az bir kanıt barı yansıda yok ya da canlı barın kanıt sürümüne yetişilmemiş). Kullanıcı kanıtı göremediği bir iddia görür. |
| **Çelişkili gösterim** | `τ_show` anında *c*'nin yüklemi yansının görünen verisi üzerinde yanlış (ekrandaki RSI 69,8 iken "70'i kesti"). Kullanıcı açısından en ağır hata. |
| **Yanlış gösterim** | *c* "kesin" etiketiyle gösterilmiş ama gerçek barlar üzerinde (hakem) yanlış; ya da "geçici" etiketiyle gösterilmiş ve o anki gerçek canlı bar üzerinde yanlış. |
| **Bayat gösterim** | *c* gösterilmeye devam ediyor ama hakeme göre *c* geri çekildi/geçersizleşti ve aradan sınır Δ_stale'den fazla geçti. |
| **Kaçırılan** | *c* gerçek barlar üzerinde kesin doğru, ama Δ_miss içinde hiç gösterilmedi. |
| **Gösterme gecikmesi** | `τ_show − τ_true`; `τ_true` = *c*'nin gerçek veride doğru olduğu ilk sanal an (canlı bar için tikin dünyada olduğu an, kesin için barın kapandığı an). |
| **Yanıp sönme** | Bir iddianın göster/gizle geçiş sayısı. Geçici olgularda bir ölçüde kaçınılmaz; UX ölçüsüdür. |

"Yanlış ya da erken gösterilen bulgu 0" şartını şöyle okumayı öneriyoruz:
**erken = 0, çelişkili = 0, yanlış = 0**; geçici etiketle gösterilip bar kapanmadan geri
çekilen olgu **yanlış sayılmaz** ama ayrı sayılır ("geri çekilen geçici"). Bu okuma bir
**ürün kararıdır** ve Arkin'in onayını ister (Açık soru S1).

**Tutarlılık düzeyleri** (adaylar; yaklaşımlar bunlara göre sınıflanır):

| Düzey | Ad (aday) | Garanti |
|-------|-----------|---------|
| T0 | Sonunda tutarlı | Beslemeler durulunca iki uç aynı şeyi gösterir. Arada her şey olabilir. |
| T1 | Kanıta tutarlı | Gösterilen her iddia yansının o anki görünen verisi üzerinde doğrudur (erken = çelişkili = 0). Otoriteyle aynı veriye bakıldığı garanti değildir. |
| T2 | Konuma tutarlı | T1 + yansı, iddiayı ancak kanıt aralığındaki verisi otoritenin verisiyle **aynı** olduğunda (özet eşit) "doğrulandı" diye gösterir. |
| T3 | Kesin tutarlı | T2 + yalnızca kesinleşmiş (filigran gerisinde, sağlayıcının kesin barına karşı onaylanmış) iddialar "kesin" gösterilir. |

Akış işlemedeki "olay zamanı / işlem zamanı" ayrımı burada şöyle eşlenir: olay zamanı =
bar zamanı `t`; işlem zamanı = sanal saat (düzenekte) ya da duvar saati (üründe).
Otoritenin saati yoktur (§11); ölçümde zaman tohumlu zamanlayıcıdan gelir.

---

## 2. Literatür

### 2.1 Akış işleme: olay zamanı, filigran, noktalama, ilerleme

| Kaynak | Ne söylüyor | Bize ne verir |
|--------|-------------|---------------|
| Tucker, Maier, Sheard, Fegaras, "Exploiting Punctuation Semantics in Continuous Data Streams", *IEEE TKDE* 15(3):555–568, 2003. doi:10.1109/TKDE.2003.1198390 | Akışa gömülü **noktalama** (punctuation): "bu alt kümeden artık öğe gelmeyecek". Engelleyici işleçler sonsuz akışta sonlu alt akışlarla çalışır; "tut" ve "yay" değişmezleri. | Anlam kanalına gömülü **veri işareti** (`data_mark`, Yaklaşım E): "t_W'ye kadar barlar kapandı, özeti şu". Yansı tamponladığı iddiaları bu işaretle serbest bırakır. |
| Akidau ve ark., "MillWheel: Fault-Tolerant Stream Processing at Internet Scale", *PVLDB* 6(11), 2013 **[bellekten]** | Düşük filigran: "bundan eski olay zamanlı veri gelmeyecek" tahmini; tam-bir-kez işleme. | Filigran bir **beyandır**, tahmin olabilir; geç gelen veri ayrı yolla işlenir. |
| Akidau ve ark., "The Dataflow Model: A Practical Approach to Balancing Correctness, Latency, and Cost in Massive-Scale, Unbounded, Out-of-Order Data Processing", *PVLDB* 8(12):1792–1803, 2015. doi:10.14778/2824032.2824076 | Pencere / tetikleyici / filigran / izin verilen gecikme; **erken, zamanında, geç** tetiklemeler; birikim kipleri: atan, biriken, **biriken ve geri çeken** (retraction) **[ikincil: kip tanımları Akidau'nun "Streaming 102" yazısından]**. | Geçici olgu = "erken tetikleme"; kesin olgu = "filigranda tetikleme"; düzeltme = "geç veri + geri çekme". Ürün dilimizdeki **geçici / kesin / geri çekildi** üçlüsü bu modelin doğrudan karşılığı. |
| Akidau, Begoli, Chernyak, Hueske, Knight, Knowles, Mills, Sotolongo, "Watermarks in Stream Processing Systems: Semantics and Comparative Analysis of Apache Flink and Google Cloud Dataflow", *PVLDB* 14(12):3135–3147, 2021. doi:10.14778/3476311.3476389 | Filigranın anlamı; Flink ve Dataflow'un anlamca eşdeğer, uygulamaca çok farklı filigranları. | Filigranın **kaynaktan** üretilmesi ve işleçler boyunca yayılması; bizde filigranı kim üretir sorusu: her uç **kendi** beslemesinden. İki filigran farklıdır; yansının filigranı otoritenin iddiasını serbest bırakma ölçütüdür. |
| Murray, McSherry, Isaacs, Isard, Barham, Abadi, "Naiad: A Timely Dataflow System", *SOSP* 2013 **[bellekten]** | Kısmi sıralı mantıksal zaman damgaları ve **ilerleme izleme** (bir zamanın "tamamlandığını" bilme). | Çok bağlı iddiada konum bir vektördür; "tamamlandı" bilgisi bağ başınadır, kısmi sıralıdır. |
| McSherry, Murray, Isaacs, Isard, "Differential Dataflow", *CIDR* 2013 | Değişiklikleri (+/−) kısmi sıralı zamanlarla taşıyarak artımlı ve yinelemeli hesap. | Düzeltme = geçmiş bir zamanda bir (−eski, +yeni) farkı; sonraki olguların yeniden değerlendirilmesi. Artımlı canlı yolla (T5) aynı dil. |

**Çıkarım.** Akış işleme "doğru zamanda göster" sorusunu **tek** bir işleme hattında
çözer: filigran kaynaktan aşağı akar. Bizde **iki bağımsız kaynak** (iki besleme) ve
**bir anlam kanalı** var; filigran bir uçtan diğerine akmaz. Literatürden alınacak şey
kavram dilidir (filigran, noktalama, erken/geç tetikleme, geri çekme); iki kaynaklı
doğrulama kısmı literatürde hazır değil (§3'te özgün kısım).

### 2.2 Dağıtık veritabanlarında okuma tutarlılığı

| Kaynak | Ne söylüyor | Bize ne verir |
|--------|-------------|---------------|
| Terry, Demers, Petersen, Spreitzer, Theimer, Welch, "Session Guarantees for Weakly Consistent Replicated Data", *PDIS* 1994, s. 140–149. doi:10.1109/PDIS.1994.331722 | Dört oturum garantisi: yazdığını oku, tekdüze okuma, okumayı izleyen yazma, tekdüze yazma; sunucu tarafında değil, oturumun taşıdığı **yazma kümesi/sürüm vektörü** ile. | "Yansı, iddianın dayandığı veriyi görmeden iddiayı göstermez" = **okumayı izleyen gösterim**; iddia kendi bağımlılık konumunu taşır, yansı yetişene kadar bekler (Yaklaşım A). Tekdüze okuma: yansının gösterdiği konum geri gitmemeli (düzeltme hariç, açık etiketle). |
| Hunt, Konar, Junqueira, Reed, "ZooKeeper: Wait-free Coordination for Internet-scale Systems", *USENIX ATC* 2010 **[okuma yolu ayrıntıları ikincil: ders notları]** | Okumalar takipçide yerel; her cevap sunucunun gördüğü son `zxid` ile; istemci gördüğü en büyük `zxid`'den geriye düşmez; `sync` ile güncellik. | Konum belirteci taşıyan istemci + "yetişmeden sunma" kuralı. A'nın en sade öncülü. |
| Corbett ve ark., "Spanner: Google's Globally-Distributed Database", *OSDI* 2012 **[bellekten]** | Zaman damgasında anlık görüntü okuma; kopya, güvenli zaman t_safe ≥ t olmadan okumaya cevap vermez. | "Konum t'de oku, t_safe'e kadar bekle" = yansının filigranı ≥ iddianın konumu olmadan göstermemesi. TrueTime'ın belirsizlik aralığının karşılığı bizde **besleme gecikmesi** — bekleme süresi buradan gelir. |
| Lamport, "Time, Clocks, and the Ordering of Events in a Distributed System", *CACM* 21(7), 1978 **[bellekten]**; Parker ve ark., "Detection of Mutual Inconsistency in Distributed Systems", *IEEE TSE* 1983 **[bellekten]** | Önce-olur ilişkisi; sürüm vektörleriyle uyuşmazlık tespiti. | Çok bağlı iddianın konumu bir sürüm vektörü; karşılaştırma bileşen bazında. |
| Lloyd, Freedman, Kaminsky, Andersen, "Don't Settle for Eventual: Scalable Causal Consistency for Wide-Area Storage with COPS", *SOSP* 2011 **[bellekten]** | Nedensel tutarlılık: bir yazı, bağımlılıkları görünür olmadan görünür olmaz. | Bulgu, dayandığı olgular yansıda doğrulanmadan gösterilmez; olgu, dayandığı veri yansıda yokken gösterilmez — **iki katlı bağımlılık**. |
| Shapiro, Preguiça, Baquero, Zawirski, "Conflict-free Replicated Data Types", *SSS* 2011 **[bellekten]** | Birleşmesi garanti veri tipleri. | Alınmadı: anlam düzlemi tek otoriteli (Ö2); veri düzleminde iki kaynak birleştirilmez, karşılaştırılır. |

### 2.3 İçerik adresli kayıtlar ve özet zincirleri

| Kaynak | Ne söylüyor | Bize ne verir |
|--------|-------------|---------------|
| Merkle, "A Digital Signature Based on a Conventional Encryption Function", *CRYPTO '87*, 1987 **[bellekten]** | Özet ağacı: kökle bütüne bağlanma, O(log n) kanıt. | Aralık özeti ve farkı ikiye bölerek bulma (§3 E). |
| Haber, Stornetta, "How to Time-Stamp a Digital Document", *J. Cryptology* 3(2), 1991 **[bellekten]** | Belgeleri özet zincirine bağlayarak geriye dönük değiştirilemez zaman damgası. | Dondurulmuş kanıt: "bu iddia o gün şu veriye dayanıyordu". |
| Crosby, Wallach, "Efficient Data Structures for Tamper-Evident Logging", *USENIX Security* 2009, s. 317–334 | Özet zincirinde denetim doğrusal; ağaç biçimli **geçmiş ağacı** ile O(log n) denetim ve arama. | Özet zinciri (A) uzun geçmişte farkın **nerede** olduğunu söylemez; ağaç biçimi söyler. |
| Laurie, Langley, Kasper, RFC 6962 "Certificate Transparency", 2013 **[bellekten]** | Yalnız eklenen Merkle günlüğü; tutarlılık ve içerme kanıtı. | Eklenen bar dizisinin özeti için hazır yapı. |
| Todd, "Merkle Mountain Range" (bitcoin-dev e-posta listesi, 2016) **[ikincil; resmi şartname bulunamadı]** | Ekleme sırasıyla büyüyen, O(log n) "dağ tepesi" saklayan Merkle yapısı; ekleme var olanı değiştirmez. | Bar dizisi için ideal: yeni bar O(1) amortize ekleme, kanıt O(log n). Canlı bar ayrı tutulur (dağa yalnız kapanınca girer). |
| Haeberlen, Aditya, Rodrigues, Druschel, "Accountable Virtual Machines", *OSDI* 2010 (öncülü PeerReview, SOSP 2007) | Belirleyici bir yürütmenin girdileri özet zincirli bir günlükte tutulur; denetçi girdileri yeniden oynatarak çıktıyı doğrular; sapma = hata kanıtı. | **Doğrudan modelimiz:** motor belirleyici, olgu bir çıktı; girdiye (veri aralığı + hesap başlangıcı) taahhüt edilirse herkes yeniden hesaplayıp doğrulayabilir. Ağır doğrulanabilir hesap (SNARK vb.) gerekmez: yeniden hesap ucuz (K4). |

### 2.4 Piyasa verisi uygulaması

| Kaynak | Ne söylüyor | Bize ne verir |
|--------|-------------|---------------|
| FIX oturum protokolü: `MsgSeqNum`(34), `ResendRequest`(35=2), `SequenceReset`(35=4) + `GapFillFlag`(123), `PossDupFlag`(43); kaynak: OnixS FIXT1.1 sözlüğü "Message Recovery", Cboe ve CME belgeleri **[ikincil: borsa/üretici belgeleri]** | Her mesajda sıra numarası; boşlukta yeniden gönderim isteği; gönderilemeyenler için "boşluk doldurma"; beklenenden küçük numara (PossDup değilse) ciddi hata. | Anlam kanalında zaten `seq` var (§4.3). Eksik olan veri kanalında konum. "Boşluk doldurma" fikri: otorite "şu aralıkta senin için iddia yok" diyebilmeli (§3 E, `data_mark`). |
| Nasdaq MoldUDP64 v1.00 (2009), https://www.nasdaqtrader.com/content/technicalsupport/specifications/dataproducts/moldudp64.pdf | UDP çoklu yayın; paket başına oturum + sıra numarası + mesaj sayısı; boşlukta ayrı **yeniden istek sunucusuna** tekli istek. | Ana akış ile kurtarma kanalının ayrılması: yansı uyuşmazlıkta veriyi **kendi kaynağının** tarihsel ucundan (REST) yeniden ister, protokolden değil (K23). |
| Nasdaq TotalView-ITCH 5.0 (ve aynı biçimi kullanan GLIMPSE, BX şartnameleri) | Mesajda takip numarası ve gece yarısından beri nanosaniye zaman damgası; GLIMPSE ile anlık görüntü + artımlı akış. **[ana PDF okunmadı; alan yerleşimi kardeş şartnamelerden]** | Anlık görüntü + artımlı akışın birleşme noktası: anlık görüntünün hangi sıra numarasına kadarki durumu temsil ettiği yazılır. |
| CME MDP 3.0 kurtarma belgeleri (CME müşteri wiki: "MDP 3.0 – Recovery Services", "Market Data Snapshot – Full Recovery", "Channel Reset") | Enstrüman başına `RptSeq`(83) ile boşluk tespiti; paket sırası ayrı; döngüsel anlık görüntü akışında `LastMsgSeqNumProcessed`(369) ile artımlı akışla hizalama; "doğal tazelenme" kurtarmanın yerini tutmaz; kanal sıfırlamada `RptSeq` 1'e döner; sıkıştırılmış (conflated) TCP akışında `RptSeq` = 0. | (1) **Enstrüman başına** konum (bizde bağ başına). (2) Anlık görüntü, hangi artımlı konuma kadar olduğunu söyler — bizde yeniden bağlanan yansı için `REFRESH` hangi bar konumuna denk? (3) "Doğal tazelenme" uyarısı: Matriks'in kümülatif alanları (`dailyHigh`, `dailyQuantity`) bir sonraki güncellemede kısmen kendini onarır, ama bar içi en yüksek/en düşük kaybolabilir. (4) Sıkıştırmalı akışta tek tik sırası yoktur — sıkıştırma, tik düzeyinde konumu anlamsız kılar; konum bar düzeyinde olmalı. |

**Matriks'e özgü iki gözlem** (depo belgesinden çıkarım; R6'da doğrulanmalı):

- `SymbolMessage` kümülatif alanlar taşıyor (`dailyHigh`, `dailyLow`, `dailyQuantity`,
  `volume`). Bar hacmi = kümülatif miktarın bar başı/sonu farkı ise, barların hacim
  toplamı kümülatif değere eşit olmalı; tutmuyorsa **kayıp var**. Benzer şekilde
  `dailyHigh` gün içinde görülen barların en yükseğini aşarsa bir tik kaçmıştır. Bu,
  sıra numarası olmayan akışta **kayıp tespiti** sağlar (düzeltmez).
- `TradeMessage.price` 32 bit `float`, `SymbolMessage` fiyatları `double`
  (`MATRIKS_PROTOCOL.md` §12/5). Barı hangi mesajdan kurduğu iki uçta farklıysa bit
  farkı **kalıcıdır**. Bar kurma kuralı bağın parçası olmalı.

### 2.5 Belirleyici benzetim, kilit adım ve geri sarma

| Kaynak | Ne söylüyor | Bize ne verir |
|--------|-------------|---------------|
| Bettner, Terrano, "1500 Archers on a 28.8: Network Programming in Age of Empires and Beyond", *GDC* 2001 / Gamasutra | Kilit adım (lockstep): eşler yalnızca **girdileri** paylaşır, herkes aynı benzetimi koşar; aynı kodun aynı sonucu vermediği durumlara karşı uyarı; geç katılma zor. **[eşzamanlılık kaybı tespiti (durum sağlaması) ayrıntısı bu oturumda doğrulanamadı]** | Bizim mimarimizin oyun karşılığı: veri = girdi, motor = benzetim. Oyunlarda sık kullanılan "periyodik durum sağlaması ile sapma tespiti" **[DOĞRULANMADI: bu kaynakta]** Yaklaşım A/E'nin özüdür. |
| Cannon, GGPO (geri sarmalı ağ kodu), Game Developer yazısı 2012; MIT lisansıyla açıldı 2019 **[ikincil]** | Uzak girdiyi tahmin et, hemen göster; gerçek girdi farklı gelirse geri sar ve yeniden oynat. | **Karşı örnek:** "hemen göster, sonra düzelt" oyunda kabul edilir, finansal iddiada edilmez (erken/yanlış gösterim). Ama **geçici** etiketli gösterim, açıkça etiketlenmiş bir tahmin olarak bu fikrin güvenli hali. Düzenekte "saf gösterim" tabanı olarak kullanılır. |
| Zhou ve ark., "FoundationDB: A Distributed Unbundled Transactional Key Value Store", *SIGMOD* 2021. doi:10.1145/3448016.3457559; https://apple.github.io/foundationdb/testing.html | Tüm kümenin tek iş parçacıklı, **belirleyici benzetimi**; hatalar tohumlu enjekte edilir; her hata tohumla yeniden üretilir. | Düzenek (§5) bu ilkeyle: gecikme, sıra bozulması, düşme ve düzeltme tohumlu bir zamanlayıcıdan; gerçek saat yok; her başarısızlık tohumuyla yeniden oynatılır. |

### 2.6 Hafif doğrulanabilir hesap: girdiye taahhüt

Ağır doğrulanabilir hesap (ör. Pinocchio, SNARK'lar) **[bellekten]** burada gereksiz:
doğrulayıcı (yansı ya da sonraki denetçi) aynı motoru çalıştırır ve hesap ucuzdur (K4,
K19). Gereken yalnızca **girdiye taahhüt**: olgu, hangi girdiden hesaplandığını (bağ +
hesap başlangıcı + kanıt aralığının özeti + motor sürümü) söylerse, herkes yeniden
hesaplayıp aynı biti beklemeyi ya da yüklemi sınamayı seçebilir. AVM (§2.3) bunun
günlük düzeyindeki örneği. Bu, dondurulmuş kanıtın (K23) ve canlı tutarlılığın aynı
mekanizma olabileceğini gösterir.

### 2.7 Ortak boşluk → özgün kısım (K18)

Hiçbir kaynak şunları birlikte vermiyor:

1. **İki bağımsız veri kaynağı + bir anlam kanalı.** Akış işlemede filigran tek hat
   boyunca akar; veritabanında okuyan, yazının **kendisini** alır. Bizde yansı iddianın
   dayandığı veriyi otoriteden değil **kendi kaynağından** alır ve kaynaklar aynı olmayabilir.
2. **Bit eşitliği ile yüklem doğruluğu ayrımı.** Otoritenin ve yansının değerleri bit
   düzeyinde farklıyken iddia her iki veride de doğru olabilir (bölünme sonrası RSI;
   farklı hesap başlangıcı). Kullanıcı için önemli olan **ekranda gördüğü kanıt üzerinde
   iddianın doğru olması** (T1); denetim için önemli olan **aynı veri** (T2/T3).
3. **Geçici iddia.** Akış işlemede erken tetikleme bir aşağı akış sonucudur; bizde
   kullanıcıya **etiketli** gösterilen, geri çekilebilen bir iddiadır ve K26'ya göre
   cümlesi zamanını söylemek zorundadır.
4. **Dondurulmuş kanıt ile canlı tutarlılığın tek nesne olması.**

---

## 3. Aday yaklaşımlar

Her yaklaşım savunucusunun yazacağı en iyi haliyle. Ortak varsayımlar:

- Bar kimliği `t` (bar açılış zamanı, K14); sıra numarası yardımcı.
- Bağ (T2'nin tasarımı) hesap başlangıcını ve bar kurma kuralını sabitler (§1.2/5–6).
  Bu yapılmazsa **hiçbir** yaklaşım bit-eşitlik vaat edemez; B, C, D yüklem düzeyinde
  çalışmaya devam eder, A sürekli uyuşmazlık verir.
- Özet işlevi: kanonik bar baytları (t i64 LE, o/h/l/c/v f64 bitleri LE, NaN kanonik).
  Telde 128 bit (32 onaltılık) önerilir; kriptografik bir özetin kısaltması (ör.
  BLAKE3 ya da SHA-256) dondurulmuş kanıtın kasıtlı değiştirmeye karşı da sınanmasını
  sağlar. Hangi kütüphane: **açık**, ölçülmeden seçilmez (CLAUDE.md).
- JSON örneklerindeki alan adları öneridir; şemanın tek kaynağı `qp-schema`.

### A. Konum damgası (veri konumu + önek özeti; yansı yetişmeden uygulamaz)

**Savunucunun sözü.** "Veritabanları bunu kırk yıldır çözüyor: okuma bir konumda yapılır,
kopya o konuma yetişmeden cevap vermez (ZooKeeper `zxid`, Spanner `t_safe`, Terry'nin
oturum garantileri). Veriden türeyen her anlam mesajına otoritenin o andaki veri konumunu
ve oraya kadarki verinin özetini iliştir. Yansı aynı konuma ulaşıp aynı özeti
hesaplayınca uygular. Basit, genel, iddianın türünden bağımsız: olgu, `invalidated`,
`condition_met`, niyet — hepsi aynı kuralla."

**Mesaj ekleri.** Veriden türeyen her günlük girdisine ve olayına `data_pos`:

```json
{"type":"notify","sub":"s1","event":{"kind":"change","seq":812,"author":"qp",
 "data_pos":[{"binding":"px","sealed_t":1700000000000,"sealed_digest":"9f2c…(32 hex)",
              "live_t":1700000060000,"live_rev":41,"live_digest":"07ab…",
              "data_epoch":0}],
 "changes":[{"op":"put_fact","fact":{"id":"f-1842","kind":"cross_above", "...":"..."}}]}}
```

`condition_met`, `intent_triggered`, `finding_state` olaylarına aynı `data_pos`.
`sealed_digest` = özet zinciri `h_W` (hesap başlangıcından son kapanmış bara kadar).
`live_*` yalnızca iddia canlı bara dayanıyorsa.

**Yansının algoritması.**

1. Her bağ için özet zincirini artımlı tut: bar kapanınca `h ← H(h ‖ bar)` (O(1)).
   Canlı bar için `H(h_W ‖ live_bar)` her güncellemede (O(1)).
2. Anlam mesajını, bağ başına `local.sealed_t ≥ msg.sealed_t` olana kadar **beklet**
   (seq sırası korunur: önceki beklerken sonraki uygulanmaz — tekdüze okuma).
3. Yetişince `local h(sealed_t) == msg.sealed_digest` ise uygula ve göster. Canlı bara
   dayanıyorsa canlı bar özeti de eşit olmalı — pratikte nadiren eşit olur (§1.2/3),
   bu yüzden **en iyi hali:** canlı bara dayanan iddiayı bar kapanana kadar "bekliyor"
   tut, kapanınca kapanmış bar özetini karşılaştır.
4. Özet farklı → uyuşmazlık: iddiayı gösterme; `data_dispute` gönder (otorite tarafı
   kayıt); kendi kaynağından aralığı yeniden çek (REST); yeniden karşılaştır.
5. Zaman aşımı (yansı beslemesi gelmiyor): "veri bekleniyor" durumu; iddia gösterilmez.

**Maliyet.** Mesaj başına ~120–160 B JSON (bağ başına; tahmin, ölçülmedi). Yansıda
bar başına bir özet (~100 ns mertebesi beklenir, ölçülmedi). Otoritede de aynı.

**Hata kipleri ve zayıf noktalar.**

- **Önek özeti çok katı.** 3 ay önceki bir barın düzeltilmesi sonraki **bütün**
  özetleri değiştirir; yansı düzeltmeyi almadıysa bütün yeni iddialar durur. Bölünmede
  tüm geçmiş değişir; iddia (RSI ölçekten bağımsızdır) doğru olsa bile gösterilmez
  ("yanlış engelleme").
- **Canlı barda neredeyse hiç eşleşmez** → canlı iddialar bar kapanışına kadar gecikir
  (1 dakikalık barda ≤ 60 s; günlük barda gün sonu). Ürün için kabul edilemez olabilir.
- Uyuşmazlıkta **hangi barın** farklı olduğunu söylemez (zincir doğrusal; Crosby &
  Wallach 2009) → E ile birleştirilmezse kurtarma tüm aralığın yeniden çekilmesidir.
- Kanıt değeri taşımadığı için **verisi olmayan** istemciye (ses, saat, ajan) bir şey
  göstermez; onlar için ayrı yol gerekir.
- Erken = 0 ve çelişkili = 0 güçlü biçimde sağlanır (özet eşitse veri aynı, motor aynı
  → yüklem aynı). Bedel gecikme ve kaçırılan iddiadır.

### B. Kanıt değerleri (otorite değerleri gönderir; yansı kendi hesabıyla karşılaştırır)

**Savunucunun sözü.** "İddiayı neyin doğruladığını biliyoruz: birkaç sayı. RSI(14)
bar t₁'de 69,8, bar t₂'de 71,37, eşik 70. Bütün geçmişin özetini taşımak, bin barlık bir
düzeltmeyi bütün iddiaları durdurmaya çevirir. Sayıları gönder; yansı kendi verisinde
aynı barlarda aynı sayıları (ya da en azından yüklemi) görürse göster. `Fact.evidence`
zaten var; eksik olan yalnızca hangi barlar ve hangi seri olduğu."

**Mesaj ekleri.** `Fact` (ve `condition_met`/`intent_triggered`) kanıt noktalarını taşır:

```json
{"op":"put_fact","fact":{"id":"f-1842","kind":"cross_above","series":"rsi14",
 "bar":1842,"t":1700000000000,"text":"RSI(14) 70'i yukarı kesti",
 "evidence":{"level":70.0,"prev":69.8,"value":71.37},
 "evidence_at":{"binding":"px","anchor_t":1690000000000,
   "points":[{"series":"rsi14","out":"rsi","t":1699999940000,"v":69.8},
             {"series":"rsi14","out":"rsi","t":1700000000000,"v":71.37}],
   "bar_state":"closed","live_rev":null},
 "provenance":{"...":"..."}}}
```

**Yansının algoritması.**

1. Kanıt noktalarındaki her `t` yansıda var ve (kapanmış bar için) kapanmış mı? Değilse
   **beklet** (bu, A'nın konum kuralının yerel hali: yalnızca kanıt barları).
2. Yansı aynı serinin aynı çıktısını o barlarda hesaplar.
   - **B-katı:** değerler bit-eşitse göster.
   - **B-yüklem (savunucunun tercihi):** dedektörün yüklemi yansının verisinde doğruysa
     göster; değerler farklıysa ekranda **yansının** değerleri görünür (kanıt ile ekran
     hiçbir zaman çelişmez), otoritenin değerleri "kaynakta" açılımında saklanır.
3. Yüklem yansıda yanlış → uyuşmazlık: gösterme; `data_dispute` bildir.
4. Canlı bar: `bar_state: "live"` iddiası, yansının canlı barında yüklem doğruysa
   "geçici" etiketle gösterilir; yansının canlı barı yüklemi kaybedince gizlenir.

**Maliyet.** Olgu başına ~200–300 B (tahmin). Olgular seyrektir (bar başına değil olay
başına). Yansıda doğrulama, zaten hesapladığı değerleri okumaktır (O(nokta sayısı)).

**Hata kipleri.**

- **Kanıt noktaları yetmeyebilir.** Bulgunun geçersizlik koşulu ya da çok olgulu bir
  desen (ör. "üç dip") pek çok noktaya dayanır; nokta listesi büyür.
- Değerler eşit, **görünen geri kalan veri** farklı olabilir (mum gövdesi farklı, ama
  RSI aynı). Kullanıcı açısından T1 sağlanır, T2 sağlanmaz; denetim "aynı veriye
  dayanıyordu" diyemez.
- B-katı, hesap başlangıcı ve f32/f64 farkında **sürekli** uyuşmazlık verir; B-yüklem
  eşik çok yakınken (69,99999 / 70,00001) iki uçta farklı karar verir — bu doğru
  davranıştır (yansının verisinde iddia gerçekten yanlış) ama "kaçırılan" sayılır.
- Verisi olmayan istemciye (ses, ajan) değerleri **otoritenin sözü** olarak verebilir —
  A'dan üstün (AI-native testi 1: ekransız).
- Dondurulmuş kanıt için yalnız değerler yetmez: "veri değişti mi?" sorusu değerlerden
  değil verinin özetinden cevaplanır.

### C. Yansı kendi verisiyle çalışır, sapmayı sonradan bildirir

**Savunucunun sözü.** "Yansı zaten aynı motoru ve aynı dedektörleri çalıştırıyor. Erken
gösterim, yansının **kendi** verisinde olmayan bir iddiayı göstermesidir; o halde yansı
yalnızca kendi verisinden hesapladığı olguyu göstersin. Tanım gereği erken = 0 ve
çelişkili = 0. Otoritenin olgularıyla karşılaştırma arka planda yapılır; fark varsa
yansı sapmayı bildirir ve otorite kayda geçer. Gecikme en düşük: tik geldiği anda."

**Mesaj ekleri.** Otorite → yansı ek yok (olgular zaten `change` ile geliyor; yansı
onları **göstermek için değil karşılaştırmak için** kullanır). Yansı → otorite:

```json
{"type":"report","kind":"divergence","binding":"px","t":1700000000000,
 "local":{"fact":"cross_above","series":"rsi14","v":71.37},
 "authority":{"fact":null},"local_pos":{"sealed_t":1700000000000,"sealed_digest":"…"}}
```

(Yeni istemci → otorite mesaj türü; v0'da yok. Günlüğe girmez, varlık durumu gibi.)

**Yansının algoritması.** Kendi beslemesinden artımlı hesap + dedektör; olguyu yerel
olarak üret ve göster (canlı barda "geçici"). Bulgular (ajanın yazdığı iddialar) yerel
olarak üretilemez: bir bulgu, dayandığı olgular **yansıda da** varsa gösterilir; yoksa
"kanıt yerelde yok" durumunda bekler. Geçersizlik koşulları yerel veride değerlendirilir.
Otoritenin olgu kümesiyle bar kapanışından Δ sonra karşılaştır; fark → `divergence`.

**Maliyet.** Protokolde neredeyse sıfır; sapma raporları seyrek. Yansıda dedektör
maliyeti (ucuz).

**Hata kipleri.**

- **İki kullanıcı farklı şey görür.** Otorite "kesti" der, yansı demez (ya da tersi).
  Bulgunun **kapsül durumu** belgede otoritenindir (K21-2); yansı yerel değerlendirmeyle
  `invalidated` derse belge ile ekran ayrışır. Tek otorite ilkesi görüntüde delinir.
- Yansının beslemesi kötüyse yansı **otoritenin hiç söylemediği** bir olgu gösterir:
  hakeme göre yanlış. T1 sağlanır, T3 sağlanmaz.
- Ajanların ve karnenin dayandığı olgular otoritededir; yansının "kendi olgusu" karneye
  girmez → kullanıcı karnede olmayan bir olgu görmüş olur.
- Ekransız/verisiz istemcilere bir şey vermez.
- Güçlü yanı: gecikme en düşük, ağ kesintisinde de çalışır (T6d B+ ile aynı ruh).

### D. Kanıt zarfı + görüntü durumları (önerilen özgün tasarım; A+B+C'nin birleşimi)

**Fikir.** Bir iddianın gösterilmesi için iki ayrı soru var ve ikisi farklı mekanizma
ister:

1. *Kullanıcı ekranda kanıtı görüyor mu ve iddia o kanıt üzerinde doğru mu?* (T1) — bu
   **yansının kendi verisinde yüklem** sorusudur (C'nin gücü, B-yüklem).
2. *Yansının kanıtı otoritenin kanıtıyla aynı veri mi?* (T2) — bu **yalnızca kanıt
   penceresinin aralık özeti** sorusudur (A'nın gücü, ama önek değil pencere).

Ve bir üçüncü boyut: iddianın **olgunluğu** — geçici (canlı bar) / kesin (filigran
gerisinde) / geri çekildi. Görüntü bir **durum makinesi** olur; her durum kullanıcıya
dürüst bir şey söyler; hiçbir durum erken ya da çelişkili değildir.

**Mesaj ekleri.** Olgu bir **kanıt başvurusu** taşır; bu nesne dondurulmuş kanıtla
(K23, `GOAL.md` "Dondurulmuş kanıt") **aynıdır**:

```json
{"op":"put_fact","fact":{"id":"f-1842","kind":"cross_above","series":"rsi14",
 "bar":1842,"t":1700000000000,"text":"RSI(14) 70'i yukarı kesti",
 "evidence":{"level":70.0,"prev":69.8,"value":71.37},
 "evidence_ref":{
   "binding":"px","anchor_t":1690000000000,
   "from_t":1699999160000,"to_t":1700000000000,
   "digest":"5d1e…(32 hex, kanıt penceresinin aralık özeti)",
   "maturity":"provisional","live_rev":41,
   "engine":"qp-compute@<sürüm>"},
 "provenance":{"...":"..."}}}
```

- `from_t..to_t` = **kanıt penceresi**: kullanıcının kanıtı görmek için bakması gereken
  barlar (dedektör söyler; RSI kesişiminde ör. son 14 bar + kesişim barı). Hesabın bütün
  geçmişi değil.
- `maturity`: `provisional` (canlı bar) | `final` (kesinleşti).
- Ek olaylar: `fact_final {fact, digest}` (kesinleşti; canlı bar kapanınca, varsa
  sağlayıcının kesin barına karşı onaylandıktan sonra) ve `fact_retracted {fact, reason}`
  (bar kapanmadan yüklem kayboldu ya da düzeltme iddiayı düşürdü). İkisi de günlüğe
  girer (`qp` yazarlı girdi; geri çekilen iddia da iz olarak kalır, öneri gibi).
- `condition_met` ve `intent_triggered` aynı `evidence_ref`'i taşır.

**Yansının durum makinesi** (iddia başına):

```
            ┌────────────── mesaj geldi ─────────────┐
            ▼                                         │
      [bekliyor]  ── yerel veri pencereyi kapsadı ──►  yüklemi yerelde sına
   (gösterilmez,                                       │
    istenirse "sunucu                     ┌────────────┼──────────────────────┐
    bir şey buldu,                        ▼            ▼                      ▼
    veri geliyor")              yüklem doğru,   yüklem doğru,          yüklem yanlış
                                özet eşit       özet farklı            (yerel veride)
                                    │               │                        │
                                    ▼               ▼                        ▼
                            [doğrulandı]     [kanıtla tutarlı,       [uyuşmazlık]
                             (geçici/kesin    veri farklı]            iddia GÖSTERİLMEZ;
                              etiketiyle)     yerel değerlerle,       yalnız "kaynaklar
                                              "veri kaynağınız        farklı" işareti;
                                              sunucudan farklı"       aralığı yeniden çek,
                                              notuyla                 dispute bildir
```

- **Canlı bar (`provisional`):** pencere kapsandı = yansının canlı barı var ve
  `local_rev`'den bağımsız olarak yüklem **yansının şu anki canlı barında** doğru. Özet
  karşılaştırılmaz (canlı bar özeti neredeyse hiç eşleşmez). Gösterim: "geçici —
  bar henüz kapanmadı" (K26). Yansının canlı barı yüklemi kaybedince **yerel olarak**
  gizlenir — otoritenin `fact_retracted`'ını beklemeden. Böylece çelişkili = 0.
- **Kapanış:** yansı kendi kapanmış barında yüklemi ve pencere özetini sınar;
  `fact_final` gelince özetini karşılaştırır → `doğrulandı (kesin)` ya da
  `kanıtla tutarlı, veri farklı` ya da `uyuşmazlık`.
- **Düzeltme:** otorite düzeltmeyi kendi beslemesinden alınca etkilenen iddiaları
  yeniden değerlendirir: yüklem hâlâ doğru → yeni `digest` ile `fact_final`
  (veri dönemi +1); değilse `fact_retracted`. Pencere özeti kullanıldığı için **kanıt
  penceresine dokunmayan** düzeltme iddiayı etkilemez (A'nın önek kırılganlığı yok);
  ama hesap değeri değişebilir — yüklem yerelde yeniden sınanır.
- **Verisi olmayan istemci** (ses, saat, ajan): `evidence` değerleri ve `maturity` ile
  otoritenin sözü olarak aktarılır ("sunucunun verisine göre, geçici").

**Neden "yanlış/erken = 0"a götürmesi beklenir (hipotez).** Erken: pencere yerelde
yokken gösterim yok. Çelişkili: gösterim her zaman yerel yükleme bağlı, yerel veri
değişince yerel olarak gizlenir. Yanlış (kesin): "kesin" etiketi yalnızca
`fact_final` + yerel özet eşitliği ile; `fact_final` otoritede kesinleşmeden (S3)
gelmez. Kalan risk: otoritenin kesinleşme ölçütünün kendisi yanlışsa (iki besleme
de aynı tiki kaçırdıysa) — bu, sağlayıcının kesin barına karşı onaylamayla kapatılır
(Açık soru S3).

**Maliyet.** Olgu başına ~300–400 B (tahmin); `fact_final` ~120 B. Yansıda pencere
özeti kesinleşmede bir kez (pencere ~15–50 bar). Durum makinesi iddia başına.

**Hata kipleri.**

- Karmaşıklık: iki ek olay, beş görüntü durumu; ürün dili bunları anlaşılır kılmalı.
- **Kanıt penceresi ≠ hesap bağımlılığı.** RSI değeri pencere dışı geçmişe (≈496 bar)
  bağlıdır; pencere özeti eşit ama değerler bit-farklı olabilir. D bunu bilerek kabul
  eder: kullanıcıya gösterilen kanıt penceresi ve yüklem esastır; tam bit eşitliği
  için ek olarak `anchor_t` + hesap başlangıcından önek özeti (A) istenebilir
  ("tam taahhüt" kipi; denetimde).
- Pencereyi her dedektör doğru söylemeli (R3'te dedektör sözleşmesinin parçası).
- Geçici iddialar yanıp sönebilir; histerezis ürün kararıdır (ör. geçici etiketi en az
  N tik sürmeden gösterme) — bu gecikme ekler, ölçülmeli.

### E. Anlam kanalında veri işareti + Merkle ile fark bulma (destek mekanizması)

**Fikir.** Noktalama (Tucker ve ark. 2003) ve MoldUDP64/CME'nin "anlık görüntü hangi
konuma kadar" fikrini anlam kanalına taşı. Otorite, bağ başına, bar kapanışlarında
seyrek bir işaret yayar (abonelikle, `changes` değil ayrı konu):

```json
{"type":"notify","sub":"s7","event":{"kind":"data_mark","binding":"px",
 "sealed_t":1700000000000,"bars":10000,"root":"c4…(MMR kökü)","data_epoch":0,
 "facts_through":1700000000000}}
```

- `root`: kapanmış barların Merkle dağ silsilesi (MMR) kökü; yalnız eklenir, kapanan bar
  O(1) amortize eklenir (Todd 2016 [ikincil]; RFC 6962 benzeri).
- `facts_through`: "bu konuma kadar senin için başka olgu yok" — FIX'in **boşluk
  doldurması**. Yansı, otoritenin bir iddiayı **söylemediğini** de bilir; C'nin
  "otorite bu olguyu söylemedi mi, henüz mi söylemedi?" belirsizliğini kapatır.
- Kök farklıysa yansı `{"kind":"data_proof","binding":"px","range":[a,b]}` sorgusuyla
  alt ağaç özetlerini ister, ikiye bölerek **farklı barı** O(log n) adımda bulur
  (Merkle ağaçlarıyla anti-entropi; veri yine protokolden gelmez, yalnız özetler).
  Sonra yalnız o barı kendi kaynağından yeniden çeker.

**Kullanımı.** Tek başına bir gösterim kuralı değil; A ya da D'ye eklenir: yeniden
bağlanma (yansı yüzlerce iddiayı tek kökle toplu doğrular), düzeltmenin yerini bulma,
proaktif sapma tespiti (henüz iddia yokken). Maliyet: bar kapanışı başına bağ başına
~150 B (1 dakikalık barda ihmal edilebilir; tahmin).

### N. Saf gösterim (kontrol tabanı; aday değil)

Mesaj gelince hemen göster (GGPO'nun geri sarmasız hali). Düzeneğin senaryolarının
**gerçekten** erken/çelişkili gösterim ürettiğini kanıtlamak için gerekli: N'de bu
sayılar sıfırsa senaryo zayıftır. T7'de her senaryo önce N ile koşulur.

### Özet tablo

| | Mesaj eki | Yansı ne zaman gösterir | Erken | Çelişkili | Canlı bar | Eski düzeltme | Verisiz istemci | Dondurulmuş kanıt |
|---|---|---|---|---|---|---|---|---|
| A | `data_pos` (önek özeti) her türevde | konum yetişti + önek özeti eşit | 0 | 0 | kapanışa kadar bekler | **tüm** sonraki iddiaları durdurur | bir şey yok | önek özeti (katı) |
| B | `evidence_at` (kanıt noktaları) | kanıt barları var + değer/yüklem tutuyor | 0 | 0 (yüklem kipi) | yerel yüklemle geçici | yalnız etkilenen | otoritenin değerleri | yetersiz (veri özeti yok) |
| C | yansı → `divergence` | yerel dedektör buldu | 0 | 0 | en hızlı | yerel yeniden hesap | bir şey yok | yok |
| D | `evidence_ref` + `fact_final` + `fact_retracted` | durum makinesi | 0 (hip.) | 0 (hip.) | yerel yüklemle geçici, kapanışta özet | yalnız pencereye dokunan | değerler + olgunluk | **aynı nesne** |
| E | `data_mark` + `data_proof` sorgusu | (destek) | – | – | – | farkın yerini O(log n) bulur | – | kök ile toplu |

---

## 4. Ölçüm planı

### 4.1 Ölçüler

| # | Ölçü | Tanım (§1.3) | Hedef |
|---|------|--------------|-------|
| M1 | Erken gösterim sayısı | `τ_show`'da yansının görünen verisi kanıtı kapsamıyor | 0 |
| M2 | Çelişkili gösterim sayısı (ve süresi) | gösterim süresince herhangi bir anda yüklem yansının görünen verisinde yanlış | 0 |
| M3 | Yanlış gösterim sayısı | "kesin" etiketli iddia hakeme göre yanlış; "geçici" etiketli iddia o anki gerçek canlı barda yanlış | 0 |
| M4 | Geri çekilen geçici sayısı | geçici gösterilip bar kapanmadan gizlenen | raporlanır (yanlış sayılmaz — S1) |
| M5 | Kaçırılan | hakeme göre kesin doğru, Δ_miss içinde gösterilmedi | en az |
| M6 | Gösterme gecikmesi | `τ_show − τ_true`, p50/p95/p99; geçici ve kesin ayrı | en az |
| M7 | Bayat gösterim süresi | hakeme göre geri çekildikten/geçersizleştikten sonra gösterimde kalınan süre | ≤ Δ_stale |
| M8 | Yanlış engelleme | hakeme göre doğru ve yansının verisi pencerede doğru, ama gösterilmedi (ör. A'da bölünme) | en az |
| M9 | Mesaj baytı | iddia başına ve saniye başına ek bayt (JSON ham ve deflate) | raporlanır |
| M10 | Yansı işlemci maliyeti | tik başına ve kapanış başına ek iş (özet, durum makinesi) | raporlanır; T5 bütçesine göre |
| M11 | Düzeltme davranışı | düzeltmenin dünyaya girişinden yansıdaki etiketin değişmesine kadar süre; etkilenmeyen iddialarda gereksiz değişim sayısı | raporlanır |
| M12 | Yanıp sönme | iddia başına göster/gizle geçişi | raporlanır |
| M13 | Uyuşmazlık süresi | iddianın "uyuşmazlık"/"veri farklı" durumunda kaldığı süre | raporlanır |

Her ölçü senaryo × tohum (öneri: senaryo başına ≥ 100 tohum) için; özet tabloda
toplam, en kötü tohum ve o tohumun kimliği (yeniden oynatma için).

### 4.2 Senaryolar (düzeneğin içermesi gerekenler)

Hepsi tohumlu; sanal zaman; parametreler senaryo adının yanında.

| # | Senaryo | Neyi sınar |
|---|---------|-----------|
| S1 | **İdeal**: gecikme yok, kayıp yok | Kimlik; bütün yaklaşımlar N ile aynı göstermeli; taban gecikme |
| S2 | **Yansı beslemesi geride** (sabit 0,5 s / 5 s / 60 s) ve protokol hızlı | Erken gösterim (N'de > 0 olmalı) |
| S3 | **Protokol geride**, besleme hızlı | Gecikme; C'nin üstünlüğü; A/D'de bekleme yok |
| S4 | **Protokol titremesi + sıra bozulması** (yeniden bağlanma etrafında yinelenen/sırasız teslim) | `seq` tamponu, tekdüzelik, yinelenen mesaj |
| S5 | **Besleme sıra bozulması** (bar içi tikler karışık sırada) | Canlı barda farklı ara durumlar; kapanışta aynı olmalı mı? (`last` sırası!) |
| S6 | **Besleme kaybı** (QoS 0; tek tik düşer — bar içi en yükseği taşıyan) ve RSI eşiğe yakın | Kalıcı bar farkı; uyuşmazlık; yanlış (yansı tarafında) |
| S7 | **Canlı bar titremesi**: eşik etrafında salınım, 60 tik/s, kesişim bar içinde belirip kaybolur | Geçici iddia, yanıp sönme, geri çekme |
| S8 | **Kapanış yarışı**: kesişimi kapanıştan hemen önceki son tik belirler; bir uç o tiki kaçırır | Kesinleşme doğruluğu |
| S9 | **Geçmiş bar düzeltmesi** (a) kanıt penceresi içinde, iddiayı düşürür; (b) pencere içinde, iddia ayakta; (c) pencere dışında, 1.000 bar önce | Geri çekme; A'nın önek kırılganlığı; M8, M11 |
| S10 | **Bölünme / ölçek düzeltmesi** (tüm geçmiş ×0,5) | RSI ölçekten bağımsız → yüklem ayakta, bütün özetler değişir; A ile B/D'yi ayırır |
| S11 | **Yansı yeniden bağlanması** (besleme `REFRESH` + protokol anlık görüntüsü/günlük) kesişimin ortasında; arada kaçan tikler geri gelmez | Toplu doğrulama, E'nin değeri |
| S12 | **Farklı hesap başlangıcı / pencere** (otorite 10.000 bar, yansı son 500 bar) | §1.2/5; bağ sabitlemezse ne olur — bilinçli olarak "bağ hatası" senaryosu |
| S13 | **Yinelenen teslim** (besleme ve protokol) | İdempotentlik |
| S14 | **Otoritenin beslemesi hatalı**, yansınınki doğru | Otorite yanlış iddia eder; hangi yaklaşım yanlışı **göstermez**? Kesinleşmede sağlayıcı barına karşı onaylamanın değeri |
| S15 | **Kalıcı kaynak farkı** (yansı fiyatı f32 `TradeMessage`'dan, otorite f64 `SymbolMessage`'dan kuruyor) | Bit eşitliğinin hiç olmadığı durumda B-katı/A'nın çöküşü, B-yüklem/D'nin davranışı |
| S16 | **Çok bağlı iddia** (iki enstrüman; biri geride) | Konum vektörü |
| S17 | **Yük**: 1.000 tik/s, 50 bağ, 500 hesap | M9, M10 |
| S18 | **Bulgu ve geçersizlik**: bir bulgu bir olguya dayanıyor, sonra geçersizlik koşulu canlı barda gerçekleşip kaybolur | `finding_state`'in de aynı kurala tabi olması; bayat gösterim |
| S19 | **Besleme kesintisi** (yansı 2 dk veri alamaz, sonra devam) | Bekleme durumunun kullanıcıya dürüstlüğü; zaman aşımı |

### 4.3 Düzenek arayüzü önerisi (yaklaşımdan bağımsız)

İlke: düzenek yaklaşımların iç yapısını bilmez; yaklaşımlar birbirini görmez (T7a–c
ayrı dallarda). Düzenek **dünya**yı, **beslemeleri**, **kanalı**, **zamanlayıcıyı** ve
**hakemi** sağlar; yaklaşım iki adaptör yazar (otorite ve yansı). Ölçü yalnızca
**görüntü günlüğünden** ve **kanal baytlarından** hesaplanır.

Bileşenler (Rust biçiminde taslak; kod bu görevde yazılmadı, adlar öneridir):

```text
// 1. Dünya: kaynağın gerçek olay akışı (tohumlu, sanal zamanlı)
World::generate(seed, ScenarioSpec) -> Vec<(VTime, WorldEvent)>
WorldEvent = Tick   { binding, bar_t, fields: FieldUpdate }   // yalnız değişen alan (T3 biçimi)
           | Close  { binding, bar_t }
           | Correct{ binding, bar_t, bar: Bar }              // veri dönemi +1
           | Rescale{ binding, factor, from_t }               // bölünme
           | Refresh{ binding }                               // kaynağın tam anlık görüntüsü

// 2. Besleme modeli: dünyayı bir alıcıya (otorite ya da yansı) teslim eder
FeedModel { seed, delay: Dist, jitter: Dist, reorder_p, drop_p, dup_p,
            outages: Vec<(VTime, VTime)>, conflation: Option<ms>, price_repr: F64|F32 }
FeedModel::deliver(&World) -> Vec<(VTime, SourceEvent)>       // T3'ün olay biçimi

// 3. Protokol kanalı: opak baytlar, iki yön
Channel { seed, delay: Dist, jitter: Dist, reorder_p, dup_p, disconnects: Vec<VTime> }
// mesajlar opak Vec<u8>; düzenek boyutunu ölçer, içeriğine bakmaz

// 4. Yaklaşımın yazdığı iki adaptör
trait AuthorityAdapter {
    fn on_source(&mut self, now: VTime, ev: &SourceEvent) -> Vec<Bytes>;   // yansıya giden
    fn on_replica_msg(&mut self, now: VTime, msg: &[u8]) -> Vec<Bytes>;
    fn on_timer(&mut self, now: VTime) -> Vec<Bytes>;
    fn settled_bars(&mut self, binding, range) ;  // isteğe bağlı: sağlayıcının kesin barı (hakemden değil, "REST" benzetimi)
}
trait ReplicaAdapter {
    fn on_source(&mut self, now: VTime, ev: &SourceEvent) -> Vec<Bytes>;   // otoriteye giden
    fn on_authority_msg(&mut self, now: VTime, msg: &[u8]) -> Vec<Bytes>;
    fn on_timer(&mut self, now: VTime) -> Vec<Bytes>;
    fn refetch(&mut self, now: VTime, binding, range) -> FetchTicket;      // kendi kaynağından tarihsel çekme (gecikmeli)
    fn display(&self) -> &DisplayState;                                    // düzenek her adımdan sonra okur
    fn visible_data(&self, binding) -> &[Bar];                             // ekrandaki veri (hakem için)
}

// 5. Görüntü durumu: yaklaşımdan bağımsız iddia anahtarıyla
ClaimKey { detector: "rsi_cross", params, binding, bar_t, direction }       // kimlik değil anlam
DisplayItem { key: ClaimKey, label: Provisional|Final|Disputed|Pending|SourceDiffers,
              shown_values: Map<String,f64>, evidence: {binding, from_t, to_t} }
// Düzenek ardışık DisplayState'lerden görüntü günlüğünü (göster/gizle/etiket) çıkarır.

// 6. Zamanlayıcı: tek iş parçacıklı, öncelik kuyruğu (VTime, eş-bozucu sıra no)
// gerçek saat yok; her olay sırası tohumdan; aynı tohum → aynı günlük (bayt düzeyinde)

// 7. Hakem: dünyayı, iki beslemenin teslim ettiğini ve görüntü günlüğünü bilir
Oracle::truth(key, vtime) -> {holds_final: bool, holds_live: bool, true_since: VTime}
Oracle::check(display_log, replica_visible_data_log) -> Metrics (M1–M13)
```

Kararlar ve gerekçeler:

- **İddia anahtarı anlamdır, kimlik değil** (`ClaimKey`): hakem otoritenin kimlik
  atamasını bilmeden iddiayı gerçek veride sınar. Dedektör R3'e kadar yok; düzenek
  **bir başvuru dedektörü** (RSI(14) 70/30 kesişimi; isteğe bağlı SMA kesişimi) ve bir
  başvuru geçersizlik koşulu sağlar; yaklaşımlar aynı `qp-compute` hesabını kullanır.
- **Görünen veri düzeneğe açıktır** (`visible_data`): M1/M2 yansının *ekranındaki* veriye
  göre tanımlı; hakem bunu yaklaşımın sözüne değil, yansının gerçek veri durumuna göre
  hesaplar.
- **Sağlayıcının kesin barı** (`settled_bars`, `refetch`) dünyadan türeyen, gecikmeli
  ayrı bir kaynak olarak benzetilir (Matriks REST karşılığı); hakemin bilgisini
  sızdırmaz.
- **Kanal opaktır**: bayt sayımı adil olsun ve yaklaşımlar kendi kodlamalarını seçebilsin
  (ama JSON kanonik, Ö4 — ölçüm JSON ile).
- **Taban N** düzenekle birlikte gelir; her senaryonun N'de hata ürettiği ilk denetimdir.
- **Belirleyicilik denetimi**: aynı tohumla iki koşu → görüntü günlüğü baytları aynı;
  denetçi (T11) kendi senaryosunu yalnız `ScenarioSpec` yazarak ekleyebilmeli.

---

## 5. Uygulamayla ilgili notlar (tasarım girdisi; kapsam dışı işler "Öneriler"de)

- Veri bağı (T2) en az şunları sabitlemeli, yoksa hiçbir yaklaşım T2 düzeyine çıkamaz:
  enstrüman, periyot, **hesap başlangıcı** (`anchor_t` ya da "şu tarihten itibaren"),
  **bar kurma kuralı** (hangi mesaj, hangi alan, f64), takvim. "Son N bar" bir
  **gösterim** penceresi olabilir ama hesap başlangıcı olmamalı (S12).
- Dondurulmuş kanıt başvurusu (GOAL kapsam 1) ile D'nin `evidence_ref`'i aynı tip
  olursa iki iş birleşir; T2'nin bu belgeyi okuması önerilir.
- Kanıt doğrulama sorgusu (`VERIFY.md` #9) D'de doğal: "bu `evidence_ref`'in aralığını
  şimdiki veriyle özetle ve karşılaştır; farklıysa farklı barları (E) listele".
- `condition_met`/`intent_triggered` olaylarındaki `bar` alanı bugün sıra numarası
  (§8.1); `t` eklenmesi gerekir.

---

## 6. Geçici sıralama — HİPOTEZ (T7'de ölçülecek)

Aşağıdaki sıralama **ölçüm değildir**; T7 düzeneği bunu çürütebilir.

| Sıra | Yaklaşım | Beklenti | Gerekçe | Çürütülürse neye bakılır |
|------|----------|----------|---------|--------------------------|
| 1 | **D** (kanıt zarfı + görüntü durumları), **E ile** | M1=M2=M3=0 bütün senaryolarda; geçicide gecikme ≈ C; kesinde ≈ A'dan küçük; M8 düşük (S9c, S10'da A'dan iyi); bayt A'dan biraz fazla ama olay başına | T1 yerel yüklemle, T2 pencere özetiyle, T3 `fact_final` ile; dondurulmuş kanıtla aynı nesne; verisiz istemciye değer verir; düzeltmeye orantılı tepki | S8/S14'te M3 > 0 çıkarsa kesinleşme ölçütü (sağlayıcı barına onay) yetersiz; S7'de M12 çok yüksekse histerezis gerekir |
| 2 | **B-yüklem** | M1=M2=0; M3 S14'te > 0 olabilir (otoritenin yanlış iddiası yansının verisinde de doğruysa gösterilir); denetimde "aynı veri" diyemez | En basit ikinci; mevcut `Fact.evidence` üzerine | Kanıt noktaları çok olgulu desende büyürse bayt |
| 3 | **A** (+E) | M1=M2=M3=0 güçlü; ama M5/M8 yüksek (S9c, S10, S15) ve canlı iddialar kapanışa kadar gecikir (M6) | Doğruluk en sağlam, kullanılabilirlik en zayıf | Canlı bar yüksek gecikmesi ürün için kabul edilirse A sadeliğiyle öne geçer |
| 4 | **C** | M1=M2=0 (tanım gereği), gecikme en düşük; M3 S6/S15'te > 0 (yansının hatalı beslemesi); otoriteyle görüntü ayrışması | Tek otorite ilkesini görüntüde deler; karneyle ilişkisiz olgu gösterir | Beslemeler pratikte nadiren ayrışıyorsa C'nin bedeli küçük kalır — ölçülmeli |
| – | **N** | Bütün hata ölçüleri > 0 | Taban | N'de hata yoksa senaryo düzeltilir |

D'nin parçaları ayrı ayrı da ölçülmeli (ablasyon): D − E (veri işareti yok), D − özet
(yalnız yüklem = B-yüklem + olgunluk), D + önek özeti (tam taahhüt). Hangi parçanın
hangi hatayı kapattığı böyle görülür.

---

## 7. Açık sorular (Arkin'e)

- **S1.** "Geçici" etiketle gösterilip bar kapanmadan geri çekilen olgu "yanlış" mı
  sayılır? Öneri: hayır, ayrı sayılır (M4); ama ürün dili "geçici"yi açıkça söylemeli
  (K26). Hayır denirse canlı barda iddia hiç gösterilmez → gecikme bar periyodu kadar.
- **S2.** Uyuşmazlıkta yansı aralığı **kendi** kaynağından (kullanıcının kendi
  yetkisiyle) yeniden çekebilir mi? (Lisans açısından sorun beklemiyoruz; K23 verinin
  protokolden akmasını yasaklıyor, kullanıcının kendi çekmesini değil.)
- **S3.** "Doğru"nun tanımı: sağlayıcının kesinleşmiş barı (REST) mı? Otorite
  kesinleşmeyi ona karşı mı onaylamalı? Bu, kesin iddiayı REST gecikmesi kadar geciktirir.
- **S4.** Matriks akışında sıra numarası ya da yeniden gönderim var mı (R6)? Yoksa kayıp
  tespiti kümülatif alanlarla (§2.4) yapılır; bu bilinçli bir zayıflıktır.
- **S5.** Özet işlevi ve uzunluğu (128 bit önerisi); kriptografik mi (dondurulmuş
  kanıtın kasıtlı değiştirmeye karşı da sınanması) yoksa hızlı bütünlük özeti mi?

## Öneriler (kapsam dışı; yapılmadı)

- `docs/CONCEPTS.md` için §1.3'teki aday kavramlar (veri konumu, veri filigranı, veri
  özeti, hesap başlangıcı, kanıt başvurusu, geçici olgu, kesinleşme, uyuşmazlık,
  görüntü günlüğü, hakem; erken/çelişkili/yanlış/bayat gösterim) — seçilen yaklaşımla
  birlikte sözlüğe girmeli.
- R3 dedektör sözleşmesi: her dedektör yüklemini, **kanıt penceresini** ve canlı barda
  geçici/kesin ayrımını bildirmeli.
- R6: Matriks bar kurma kuralı (hangi mesajdan, f32/f64) bağın parçası olarak
  sabitlenmeli; kümülatif alanlarla kayıp tespiti denenmeli.
- `condition_met`/`intent_triggered` olaylarına bar zamanı (`t`).
