# Araştırma: Domain protokolü için işbirliği, veri akışı ve patch sistemleri

Round: RD1. Tarih: 2026-11-14. Yöntem: tüm bulgular çevrimiçi resmi dokümanlardan WebFetch/WebSearch ile okundu; hafızadan yazılmadı. Fetch edilemeyen sayfalar ve doğrulanamayan noktalar ilgili yerde "DOĞRULANMADI" diye işaretli. "Bizim tasarım" yorumları araştırmacının çıkarımıdır, kaynak iddiası değildir.

Test edilen çalışma kararları:
- (a) Doküman başına tek otorite + sıralı operation log, her yazma origin ile kayıtlı.
- (b) Değişiklikler semantik operasyon, genel doküman diff'i (JSON Patch vb.) değil.
- (c) Motor hem sunucuda hem cihazda çalışır, doküman devredilebilir.

---------------------------------------------------------------------------

## 1. LSP (Language Server Protocol) 3.17 / 3.18

**Kaynaklar**
- https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/ (3.17.x, "önceki sürüm"; sayfada yayın tarihi yok; ~407k karakterin ilk 200k'sı okundu)
- https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/ (3.18.x "Current", 3.19 "Upcoming"; yalnızca ilk 100k karakter okundu)

**Çözdüğü problem.** N editör x M dil için tek protokol; editör (istemci) ile dil sunucusu arasında JSON-RPC üzerinden, aynı belge hakkında tutarlı bir görüş.

**Model.**
- `initialize` ilk mesajdır; yanıttan önce istemci başka istek/bildirim göndermemelidir; yalnızca bir kez gönderilir. Yetenekler `InitializeParams` (istemci) ve `InitializeResult` (sunucu) içinde karşılıklı beyan edilir; bilinmeyen yetenekler yok sayılmalıdır. Pozisyon kodlaması da müzakere edilir (`positionEncodings`, UTF-16 zorunlu varsayılan).
- Versiyonlu doküman: `TextDocumentItem.version` her değişiklikte (undo/redo dahil) artar; `VersionedTextDocumentIdentifier.version` ardışık olmak zorunda değildir. "Versiyon numarası, verilen tüm içerik değişiklikleri uygulandıktan SONRAKİ sürümü gösterir."
- Sahiplik: belge "open" iken içeriğin sahibi istemcidir; sunucu Uri'den içeriği okumaya çalışmamalıdır. Kapanınca "master" tekrar Uri'nin gösterdiği yerdedir. Yani LSP'de bile **tek otorite açıkça bildirilmiş**: açık belgede otorite istemci.
- Incremental sync: `TextDocumentSyncKind` None/Full/Incremental. Incremental'da açılışta tam içerik, sonra yalnızca `range`+`text` değişiklikleri. Bir bildirimdeki birden çok değişiklik sırayla uygulanır ("c1 S durumunda, c2 S' durumunda hesaplanır"). Mirror için bildirimler alınma sırasıyla uygulanır. İstemci, güvenilir sonuç için bir istek göndermeden önce belge durumunu senkronlamalıdır.
- İstek/bildirim: her isteğe yanıt zorunlu, bildirime yok. `$/cancelRequest {id}`; iptal edilen istek de yine yanıt döndürmelidir. `$/` ile başlayan bildirimler yok sayılabilir, desteklenmeyen `$/` istek `MethodNotFound` döner.
- Hata kodları: JSON-RPC (-32700 ParseError, -32600, -32601, -32602, -32603), `ServerNotInitialized` -32002, `UnknownErrorCode` -32001, `RequestFailed` -32803 (sözdizimi doğru ama istek başarısız), `ServerCancelled` -32802 (yalnız sunucu iptalini açıkça destekleyen isteklerde), `ContentModified` -32801 (belge normal koşul dışında değişti; sunucu henüz işlenmemiş mesajlardaki değişiklikler için göndermemeli), `RequestCancelled` -32800.
- `$/progress` (3.15'ten): token (istek id'sinden ayrı), `begin` (zorunlu `title`) / `report` / `end`. İstemci `workDoneToken` ile, sunucu `window/workDoneProgress/create` ile başlatır; sunucu `workDoneProgress` yeteneğini beyan etmelidir. Kısmi sonuçlar `partialResultToken` + `$/progress` ile akar; tüm sonuç n bildirimle raporlanır ve son yanıt boş olur; hata durumunda kısmi sonuç yalnız `RequestCancelled` ise kullanılabilir.
- 3.18'den ilgili: `WorkspaceEdit.changeAnnotations` (3.16'dan beri; `label`, `needsConfirmation`, `description`; edit'ler `annotationId` ile bağlanır), 3.18'de `metadataSupport` / `WorkspaceEditMetadata` eklendi (tanımı okunan kısımda yok, DOĞRULANMADI) ve `SnippetTextEdit` (@since 3.18). 3.18'de iptal/hata değişikliği okunan kısımda görünmedi.

**Bizim tasarımda karşılığı.**
- `initialize` = doküman oturumu açılışı + capability negotiation (hangi operasyon aileleri, hangi şema sürümü, hangi veri akışı modları).
- Versiyonlu belge = doküman `revision` sayısı; her operasyon "şu revision'a göre" uygulanır.
- `$/cancelRequest` + `RequestCancelled` + `ContentModified` = ajan uzun bir isteği (ör. analiz, hesaplama) başlatır, doküman arada değişirse `ContentModified` benzeri hata.
- `changeAnnotations` = bizim "origin + gerekçe + onay gerekir" alanı (ajan yazımı insan onayı bekleyebilir).
- `$/progress` = uzun hesap/backfill ilerlemesi.

**AL.**
1. Capability negotiation: iki taraf yetenek beyan eder, bilinmeyeni yok say, tek `initialize`.
2. Açık sahiplik kuralı ("açıkken otorite istemcide, kapanınca Uri'de") fikri: bizim devir (handover) için model; belgenin hangi uçta "master" olduğu açıkça bir durumdur.
3. Versiyon = "değişiklikler uygulandıktan sonraki sürüm"; ardışık olması şart değil ama monoton.
4. Ayrılmış hata aileleri: `ContentModified` (bayat bağlam, yeniden dene), `RequestCancelled`, `RequestFailed`.
5. Progress'i istek kimliğinden ayrı token ile taşımak; kısmi sonuç akışı.
6. Annotation fikri (etiketli, onay isteyen değişiklik grubu).

**ALMA, neden.**
- Metin aralığı (range) tabanlı incremental değişiklik: bizim domainde adres uzayı metin konumu değil, varlık kimlikleri (çizim, gösterge, pane). Konum tabanlı adresleme bayatlamaya çok açık.
- "Sunucu belgeyi Uri'den okuyamaz" gibi iki-uçlu senkron tasarımı: LSP'de tek yazar var (istemci); bizde aynı belgeye birden çok yazar (insan + iki ajan) var. LSP çoklu eşzamanlı yazar problemini çözmez; ContentModified sadece "yeniden dene" der.
- Eski sürüm uyumsuzluğunda sessiz yok sayma: LSP bunu tolere eder; bizim hesap/replay determinizmimiz için bilinmeyen operasyon sessizce atlanmamalı (reddedilmeli).

---------------------------------------------------------------------------

## 2. CRDT sistemleri: Automerge, Yjs, Loro

**Kaynaklar (okunan)**
- Automerge README https://github.com/automerge/automerge (Automerge 3, Rust çekirdek)
- Automerge 2.0 blog (Ocak 2023) https://automerge.org/blog/automerge-2/
- Automerge 3.0 blog (Temmuz 2025) https://automerge.org/blog/automerge-3/
- https://automerge.org/docs/hello/ ; https://automerge.org/docs/reference/under-the-hood/storage/ ; https://automerge.org/docs/reference/under-the-hood/merge-rules/ ; https://automerge.org/docs/reference/concepts/
- Binary format spec https://automerge.org/automerge-binary-format-spec/
- WebSearch ile sync protokolü bilgisi (R paket vignette'i ve arXiv 2012.00472 atfı; ikincil kaynak, resmi automerge.org sync sayfası 404 verdi)
- Yjs: https://docs.yjs.dev/api/document-updates ; https://docs.yjs.dev/api/undo-manager ; https://github.com/yjs/yjs
- Benchmark: https://github.com/dmonad/crdt-benchmarks (Yjs 13.6.11, Loro 0.10.1, Automerge 2.1.10, i5-8400, Node 20.5.0)
- Loro: https://github.com/loro-dev/loro (README; "Loro 1.0 is out"). loro.dev doküman sayfaları 403 verdi.

**Çözdükleri problem.** Merkezi sunucu olmadan (veya sunucu çökse bile) çoklu kopyanın, offline düzenleme sonrası, sıra bağımsız birleşip aynı duruma yakınsaması.

**Model.**
- Automerge: JSON benzeri doküman; her operasyon (actorId, counter) kimliğiyle etiketli; değişiklikler (change) git benzeri bir DAG oluşturur. Change hash = change chunk'ın SHA-256'sı; actor = rastgele 128-bit; change içeriği: bağımlılık hash'leri, actor, seq, startOp, zaman (ms), **mesaj (serbest metin)**, sütunsal operasyonlar; DEFLATE sıkıştırma. "Heads" = DAG'ın uçları (eşzamanlı düzenlemede birden çok head olabilir). Doküman chunk'ı tüm geçmişi sütunsal biçimde saklar. Sync protokolü yalnız karşıda eksik olan change'leri değiştirir, transport'tan bağımsızdır, kaydedilebilir sync state tutar; güvenilir sıralı akış varsayar (ikincil kaynak).
- Birleşme kuralları (resmi merge-rules sayfası): map'te farklı key'lere eşzamanlı yazma ikisi de kalır; aynı key'e eşzamanlı set'te "rastgele ama tüm düğümlerde aynı" kazanan; liste elemanları ID ile adreslenir, eşzamanlı insert'ler ikisi de kalır; delete vs update'te update kazanır; sayaçlar toplanır. Sayfa move operasyonunu ve başka semantik değişmezleri kapsamıyor.
- Yjs: update'ler ikili (Uint8Array), "commutative, associative, idempotent", yani herhangi sırada ve tekrar uygulanabilir. State vector = her client için bir sonraki beklenen clock; `encodeStateAsUpdate(doc, sv)` yalnız eksik farkı verir; `mergeUpdates` update'leri birleştirir ama GC yapmaz (boyut küçültmek için Y.Doc'a yüklemek gerekir). `UndoManager`: varsayılan olarak yalnızca origin'i `null` olan yerel değişiklikleri izler, `trackedOrigins` ile **origin bazlı** seçilebilir, `captureTimeout` (varsayılan 500 ms) ile adımlar birleşir; izlenmeyen origin undo'dan atlanır. Tombstone'lar tamamen silinemez (sıralamayı bozar); yapılar birleştirilerek, silinmiş içerik soyularak azaltılır; `doc.gc` kapatılırsa eski içerik geri yüklenebilir.
- Loro: JSON'u işbirlikli ve sürüm kontrollü yapar (Fugue metin, movable tree, movable list, time travel, shallow snapshot). README'de sayısal iddia ve peer id/undo bilgisi yok.

**Yayınlanan sayılar.**
- Automerge blog (Ryzen 9 7900X, ~260k tuş vuruşu, Kleppmann trace): 0.14 ~500 s ve ~1.1 GB; 2.0.1 1816 ms ve 44.5 MB; Yjs 1074 ms ve 10.1 MB. Dosya: düz metin 107,121 B; Automerge 2.0 tam geçmişle 129,062 B (~%30 ek yük).
- crdt-benchmarks B4 (259,778 op): Yjs 13.6.11 doc 159,929 B, parse 39 ms; Loro 0.10.1 doc 258,228 B, parse 13 ms; Automerge 2.1.10 doc 129,116 B, parse 1,805 ms. (Süreler: Yjs 5.7 s, Loro 3.1 s, Automerge 14.3 s; bellek sütunu wasm'da 0 diye raporlanıyor, güvenilmez.) Not: bu benchmark zaman damgasız, tek metin dokümanıdır.
- Automerge 3.0: bellek >10x düştü; 700 MB'lık bir örnek ~1.3 MB oldu; çok büyük geçmişli bir doküman 17 saatte yüklenemezken 9 sn'de yüklendi; dosya formatı 2 ile aynı.

**Bizim tasarımda karşılığı.** Automerge change = bizim "operasyon grubu (commit)"; actor id + message = origin + gerekçe; heads = revision; sync = ajan offline dönünce "benim bildiğim revision'dan sonrasını ver".

**AL.**
1. **Provenance modeli**: her değişiklikte actor, sıra numarası, zaman, serbest mesaj, bağımlılık hash'i. İçerik adreslemeli hash (SHA-256) zinciri, log bütünlüğü ve "aynı operasyon iki kez gelirse idempotent" için çok yararlı.
2. **Idempotent + bağımlılık işaretli operasyonlar**: Yjs'in "tekrar uygulanabilir" özelliği, ağ yeniden denemesi için doğrudan lazım.
3. **State vector / heads ile fark senkronu** (ajan "şu revision'a kadar biliyorum" der).
4. **Origin bazlı undo** (Yjs `trackedOrigins`): insanın undo'su ajanın yazdığını geri almamalı; undo kapsamı origin ile seçilir.
5. **Geçmiş sıkıştırma ve kontrol noktaları** (Automerge snapshot/incremental chunk, Yjs `mergeUpdates`): log sonsuz büyümesin.
6. Sütunsal/RLE kodlama fikri: operasyon logunun diskte küçük tutulması için (ölçülmeli).

**ALMA, neden.**
- Aynı key'e eşzamanlı yazmada "rastgele ama deterministik" kazanan: finans grafiği için semantik olarak yanlış olabilir (iki ajan aynı çizgiyi farklı fiyata taşırsa, hangisinin kazandığı anlamsız). Biz çakışmayı açık, kural-temelli ve kullanıcıya görünür yapmak istiyoruz.
- Eşzamanlı liste insert'ünde "ikisi de kalır" (ör. iki ajan aynı göstergeyi ekler → yinelenen). Domain değişmezleri (tekil gösterge, geçerli zaman aralığı, hesap grafiği döngüsüz) CRDT'nin birleşme kuralıyla garanti edilemez; sonradan onarım gerekir.
- Tombstone / tam geçmiş maliyeti: Yjs ve Automerge tüm geçmişi (veya tombstone'ları) taşır; hesap motorunun belleği ve cihazdaki doküman boyutu için gereksiz.
- Byte-seviyesi toplam sıralama olmadığı için aynı girdi → bayt bayt aynı çıktı garantisi yok (özellikle farklı peer, rastgele actor id). Bizim "aynı girdi aynı çıktı" değişmezimizle CRDT'nin actor-id'ye bağlı kazanan seçimi gerilim yaratır (actor id deterministik verilirse çözülür, ama bu bir tasarım yükü).
- Undo CRDT'de çekirdek özellik değil: Automerge README/concepts'te undo bahsi yok (DOĞRULANMADI, "yok" demek değil); Yjs'de UndoManager kütüphane katmanı ve origin/zaman penceresine bağlı.

---------------------------------------------------------------------------

## 3. OT / sunucu-otoriteli sistemler: ShareDB, ProseMirror collab, Replicache/Zero, Figma

**Kaynaklar**
- ShareDB https://share.github.io/sharedb/ ve https://github.com/share/sharedb (README; OT doküman alt sayfaları 404 verdi, `submitOp`/versiyon ayrıntıları DOĞRULANMADI)
- json1 https://github.com/ottypes/json1
- ProseMirror kılavuzu, collab bölümü https://prosemirror.net/docs/guide/#collab
- Replicache https://doc.replicache.dev/concepts/how-it-works (maintenance-mode notu sayfada yok; ayrı doğrulama yapılmadı)
- Zero https://zero.rocicorp.dev/docs/introduction ve https://zero.rocicorp.dev/docs/mutators
- Figma blog (Evan Wallace, 16 Ekim 2019; arşiv kopyası) https://madebyevan.com/figma/how-figmas-multiplayer-technology-works/ (orijinal figma.com adresi fetch edilmedi)

**Model.**
- **ShareDB**: Node sunucusu edit'leri koordine eder ve commit eder; çakışmalar tip eklentileri içindeki OT ile çözülür; "senkron düzenleme API'si, asenkron eventual consistency"; offline değişiklikler bağlanınca senkronlanır; tarihsel sürümlere erişim var; yatay ölçekleme pub/sub ile. json1 (OT tipi, json0'ın yerine): `apply/compose/transform`, move/insert/remove/replace/edit primitifleri; ama README'ye göre "preview" durumunda, fuzzing ~1 milyon iterasyonda yakınsama hatası buluyor ve aynı nesnenin eşzamanlı move'unda sessizce "left kazanır"; conflict bölümü TODO.
- **ProseMirror collab**: merkezi otorite adımların (steps) sırasını belirler; otorite "versiyon = kabul edilen adım sayısı"nı tutar; istemci yalnız kendi versiyonu otoritenin versiyonuyla eşleşirse kabul edilir; reddedilen istemci yerel adımlarını **rebase** eder (uzak adımların position map'lerinden geçirerek) ve yeniden yollar; rebase sonrası anlamsızlaşan adım düşürülebilir (kılavuz bunu "genellikle uygun" der); client ID ile kendi adımını tanır; onay yerine "yeni adımlar var" bildirimi, güvenilmez bağlantıda senkronu sağlar.
- **Replicache**: mutators (adlandırılmış fonksiyonlar) hem istemcide iyimser hem sunucuda yetkili çalışır; push sırayla yeniden çalıştırır ve `lastMutationID` günceller; pull cookie + patch + onaylı mutation id'leri döner; istemci son onaylı sürüme geri sarar, patch'i uygular, bekleyen mutation'ları yeniden oynatır ve sonucu atomik gösterir; "sunucu otoritedir", sunucu sonucu farklıysa o kazanır.
- **Zero**: özel mutator'lar istemcide ve sunucuda iki ayrı implementasyonla çalışır; sunucu çalıştırması yetkili yazmadır; sunucuda hata olursa istemcinin iyimser değişikliği geri alınır; sunucu satırları gelince bekleyen yerel etkiler geri sarılıp yeniden hesaplanır. Sayfa "rebase" kelimesini kullanmıyor.
- **Figma** (resmi yazıdan): clientlar WebSocket ile sunucu kümesine bağlanır. OT'yi "elde edilmek istenene göre fazla" bulmuşlar ve çok sayıdaki durum zor akıl yürütülür; saf CRDT'yi merkezi sunucunun olduğu bir sistemde gereksiz yük bulmuşlar: "sunucu merkezi otorite olduğundan sistemi basitleştirebiliriz". Veri yapıları tek bir CRDT değil, birkaç CRDT'den esinlenmiş birleşim. Çakışma kuralı: sunucu her nesnenin her özelliği için en son gelen değeri tutar (zaman damgası gerekmez, sunucu sırayı belirler); "değişiklikler özellik değeri sınırında atomiktir". İstemci tahmini: yerel edit anında uygulanır; flicker olmasın diye, onaylanmamış kendi edit'leriyle çakışan gelen sunucu değişiklikleri atılır. Nesne oluşturma/silme açık eylemlerdir; silinen nesnenin verisi sunucuda değil silen istemcinin undo tamponunda tutulur (doküman şişmez); istemci kimliği içeren ID'ler (offline'da sunucu ID veremez). Offline: yeniden bağlanınca taze kopya indirilir ve offline edit'ler üzerine yeniden uygulanır. Reparenting: sunucu döngü oluşturan güncellemeyi reddeder, istemci geçici döngüyü görebilir ve ilgili nesneleri sunucu çözene kadar ağaçtan çıkarır. Sıralama: çocuk konumu 0-1 arası kesir (fractional indexing); ebeveyn bağlantısı ve konum tek özellik. Undo: "çok undo, kopyala, ileri redo" belgeyi değiştirmemeli; undo/redo birbirinin geçmişini çalışma anında değiştirir, eski durumu körü körüne geri yüklemez.

**Bizim tasarımda karşılığı.** En yakın model Figma + Replicache/Zero: tek otorite, sıralı log, istemci iyimser tahmin + rebase. ProseMirror "version = kabul edilen adım sayısı" bizim `revision`; reddedilince rebase = bizim ajan operasyonunu güncel revision'a göre yeniden değerlendirmesi.

**AL.**
1. **Sunucunun sırayı belirlemesi sayesinde zaman damgası/vektör saati gerekmez** (Figma, ProseMirror). Operasyon logunun sırası = gerçek sıra.
2. **Özellik-sınırında atomiklik** ve farklı özelliklerin çakışmaması (Figma). Bizim semantik operasyonlarda hedef birim "varlık kimliği + alan".
3. **Onaylanmamış kendi edit'ini koruma** (Figma): iyimser istemci, çakışan gelen değişikliği onay gelene kadar bastırır, flicker olmaz.
4. **Mutation = adlandırılmış, deterministik fonksiyon + argüman** (Replicache/Zero): aynı operasyon istemci ve sunucuda çalışır; `lastMutationID` ile idempotent yeniden gönderim. Bu doğrudan karar (b) ve (c)'yi destekler: operasyonlar veri değil, deterministik çekirdek fonksiyonlarına çağrıdır.
5. **Rebase = geri sar + yeni durum üstüne yeniden oynat** (Replicache), ve "rebase sonrası anlamsızlaşan operasyon düşürülür/reddedilir" (ProseMirror).
6. **Offline dönüşte taze kopya + yeniden uygula** (Figma) bağlantı mantığını basit tutar; bizde de ajan "güncel snapshot + operasyonlarımı yeniden önerdim" akışı.
7. **Silinenin verisi silen tarafın undo tamponunda** fikri (boyut kontrolü) ve **undo'nun geçmişi değiştirmesi**: bizim undo, "ters operasyon üret ve log'a ekle" olmalı; log'dan satır silmek yok.
8. **Kimlik üretimi**: istemci kimliği içeren ID veya içerik adresli ID (offline ajan ID alabilsin).
9. **Döngü/değişmez ihlali sunucuda reddedilir** (Figma): değişmezler otoritede zorlanır.

**ALMA, neden.**
- ShareDB/json1 tarzı genel OT dönüştürme fonksiyonları (transform matrisi): her operasyon çifti için transform yazmak karmaşıklık patlaması; json1 bile "preview" ve yakınsama hataları buluyor. Bizim yaklaşım: semantik operasyon + otorite sırası + gerekirse reddet; transform yok.
- Figma'nın "çakışan gelen sunucu değişikliğini sessiz at" kuralının aynen alınması: bizde ajan yazımı sessizce kaybolmamalı; reddedilen/yenilen operasyon origin'e açık sonuçla (ör. `Superseded`) bildirilmeli.
- Figma'nın "offline'da taze kopya indir, offline edit'leri yeniden uygula" kuralının körü körüne alınması: iki saatlik offline ajan işi güncel durumda anlamsız olabilir; bizim için "yeniden uygula" bir öneri/inceleme adımı olmalı.
- Replicache/Zero'nun bulut ürünü bağımlılığı ve JS/TS mutator runtime'ı; bizim çekirdek Rust ve hesap kodunda ağ yok. Fikir alınır, kütüphane alınmaz. (Replicache bakım durumu doğrulanmadı.)
- ProseMirror'un yalnız belge-ağacı/metin adım modeli: bizim operasyon alfabesi finansal grafik semantiğinde.

---------------------------------------------------------------------------

## 4. Vega ve Vega-Lite (deklaratif görselleştirme veri akışı)

**Kaynaklar**
- https://vega.github.io/vega/docs/api/view/ (changeset `clean` setter'ı v5.13+, `view.data` v5.5+)
- https://vega.github.io/vega/docs/signals/
- https://vega.github.io/vega-lite/docs/selection.html
- https://vega.github.io/vega-lite/docs/parameter.html
(sürüm/tarih sayfalarda açıkça yazılı değil, yalnız yukarıdaki sürüm notları)

**Çözdüğü problem.** Görselleştirmeyi deklaratif spec olarak tanımlayıp, değişen sinyal/veriden yalnız etkilenen kısmın yeniden hesaplanması.

**Model.**
- Reaktif dataflow: operatör grafı; signal'ler değişkenlerdir, event stream'ler (`on: [{events, update|encode, force}]`) tetikler, bağımlı veri/marklar otomatik güncellenir; olay işleyicileri spec sırasıyla yeniden değerlendirilir.
- View API: `view.signal(name, value)` ayarlar ama hemen güncellemez; `view.runAsync()`/`run()` dataflow'u değerlendirir ve render eder; bir sonraki değişiklikten önce `await runAsync`. `view.addSignalListener`, `view.addEventListener`. Veri: `view.insert/remove`, `view.change(name, vega.changeset().insert(...).remove(...))`; "insert, aynı pulse'ta remove ile birleştirilemez" (birleştirmek için `change`); eklenen tuple'lar önceden parse edilmiş olmalı; `view.data(name, values)` = hepsini sil + ekle. Signal listener içinden `run()` çağırmak özyineleme olarak hata verir.
- Vega-Lite: `params` dizisi; değişken param (`value`, `expr`, `bind`) ve seçim param'ı (`select: point|interval`, `on`, `clear`, `resolve: global|union|intersect`, `toggle`, `nearest`, `empty`). Seçimler predicate olarak filtre/koşulda ve ölçek domain'inde kullanılır. Vega'ya derleme sayfada anlatılmıyor (DOĞRULANMADI; yalnız "altta signal" ima ediliyor).

**Bizim tasarımda karşılığı.** Doküman durumu = Vega "spec + veri"; durum ve sinyaller (hover, seçim, viewport) ayrı; changeset = bizim bir transaction'daki operasyon grubu; `runAsync` = commit sonrası tek bir yeniden hesaplama turu.

**AL.**
1. **Batched changeset + tek pulse**: bir ajan/insan işlemi birden çok operasyonu atomik grup olarak verir, motor bir turda yeniden hesaplar.
2. **Kalıcı durum ile geçici etkileşim durumunu ayırmak**: seçim/hover/brush "param/signal"dir, belgenin parçası değil (log'a girmez). Bu protokol için kritik: crosshair/hover log'u şişirmemeli. Seçim ise bazı ajan senaryolarında (örn. "seçili aralığı analiz et") açıkça okunabilir kılınmalı.
3. Adlandırılmış parametre + `bind` fikri: göstergelerin parametreleri (periyot) belge değeri, UI widget'ı bunu bağlar.
4. `resolve` (union/intersect) fikri: çok panelli grafiklerde seçim kapsamı.
5. "Veri değiştir, tek tur çalıştır" ayrımı: akış verisi için delta + tek yeniden hesaplama (bkz. Bölüm 5).

**ALMA, neden.**
- Spec'in tamamının JSON diff ile güncellenmesi ve sinyal ifadelerinin (expression string) belge parçası olması: ifade dili bizim "hesap kodu LLM/ağ/rastgelelik içermez" ve şema Rust tipleri değişmezine aykırı bir ikinci dil getirir.
- Mutable global view nesnesi ve `run()` çağrı disiplini (await etmeyi unutma, listener içinde run hatası): bizde tek komut yolu zorunlu; el ile "run" yok.
- Veri `insert/remove` API'sinde "tuple'lar önceden parse edilmiş olmalı" gibi sözleşmeyi çağırana bırakmak.

---------------------------------------------------------------------------

## 5. Akış/canlı veri protokolleri: Perspective, TradingView datafeed

**Kaynaklar**
- Perspective https://perspective-dev.github.io/ (ana sayfa okundu; alt sayfalar `guide/explanation/table/*` 404 verdi). Not: Perspective sayfasına göre proje **OpenJS Foundation** üyesi, FINOS değil. Arrow, `on_update` delta, `index` upsert, `limit` bu sayfada geçmiyor; bu nedenle bunlar hakkında kaynaklı iddia YOK (DOĞRULANMADI).
- TradingView Charting Library datafeed: https://www.tradingview.com/charting-library-docs/latest/connecting_data/Datafeed-API/ (Streaming sayfası 404 verdi)

**Perspective (okunabilen kısım).** Table = "streaming columnar storage"; View = pivot/aggregate/filter/expression; WebAssembly (tarayıcı) veya Python/Node/Rust (sunucu) çalışma zamanı; DuckDB, ClickHouse, PostgreSQL, Polars için virtual server; WebSocket örnekleri. Table/View ayrımı = veri deposu ile sorgu görünümü ayrı.

**TradingView datafeed API.**
- `onReady` (DatafeedConfiguration), `resolveSymbol` (LibrarySymbolInfo: exchange, timezone, session, pricescale...), `getBars(from, to, countBack)`: barlar **artan zaman sırasında**; günlük/haftalık/aylık için `time` = işlem gününün 00:00:00 UTC'si; bar sayısı (`countBack`) zaman aralığından daha önemli; veri bittiğinde boş dizi + `noData: true`; `nextTime` yalnız from/to destekleyen feed'ler için.
- `subscribeBars(..., onRealtimeCallback, listenerGuid, ...)`: her çağrı **son barı günceller veya daha yeni bir bar ekler; geçmiş barlar değiştirilemez**; son barla aynı `time` olan bar onu tamamen değiştirir (replace/upsert); aynı anda birden çok abonelik olabilir (çözünürlük değişiminde), her aboneye yalnız kendi sembol/çözünürlüğü gönderilmeli, yoksa "time violation"; `unsubscribeBars(listenerGuid)`.
- Geçmişi değiştirmek için `onResetCacheNeededCallback` + `chart.resetData()` (imza belgelenmemiş).

**Bizim tasarımda karşılığı.** Piyasa verisi, doküman operasyon logundan **ayrı** bir akış: snapshot (getBars) + delta (subscribeBars). Doküman yalnız "hangi sembol/çözünürlük/kaynak" bilgisini taşır.

**AL.**
1. **Snapshot + delta ayrımı**: geçmiş yükleme (sayfalama, `countBack` mantığı, `noData`) ile canlı güncellemenin ayrı yolları olması.
2. **Zaman monotonluğu kuralı**: yalnız son bar güncellenir/yeni bar eklenir; aynı `time` = replace (idempotent upsert). Bu, hesap motorunun artımlı güncellenmesini (yalnız son barı yeniden hesapla) mümkün kılar ve determinizmi korur.
3. **İhlal = açık hata** ("time violation" yerine tanımlı hata kodu) ve **geçmişi düzeltme için ayrı açık bir "reset/yeniden yükle" olayı** (revizyon/düzeltme verisi geldiğinde tüm bağımlı hesaplar geçersiz olur).
4. **Abonelik kimliği** (`listenerGuid`) ve çoklu eşzamanlı abonelik, sembol/çözünürlük başına filtreleme.
5. Table/View ayrımı: ham veri deposu ayrı, türetilmiş görünümler (göstergeler) ayrı.

**ALMA, neden.**
- Sıra/yeniden bağlanma/eksik bar garantisi tanımlanmamış bir "fire-and-forget" callback modeli: bizde veri akışı için sıra numarası/sequence ve boşluk tespiti gerekir (bu çıkarım; TradingView dokümanı bunu belgelemiyor).
- Perspective'ten ham alınacak bir şey yok; okunabilen kaynak delta/Arrow davranışını doğrulamadı. Tekrar araştırma önerilir (Öneriler).

---------------------------------------------------------------------------

## 6. JSON Patch (RFC 6902) ve JSON Merge Patch (RFC 7386)

**Kaynaklar**
- https://www.rfc-editor.org/rfc/rfc6902 (Nisan 2013, Standards Track)
- https://www.rfc-editor.org/rfc/rfc7386 (sayfa "obsolete, RFC 7396 ile değiştirildi" diyor; 7396 ayrıca fetch EDİLMEDİ)

**Model.**
- RFC 6902: operasyon dizisi: `add, remove, replace, move, copy, test`; hedef JSON Pointer ile; operasyonlar sırayla uygulanır; hata olursa değerlendirme durur ve patch başarısız sayılır (HTTP PATCH atomik); dizilerde indeks (shift) semantiği, `-` sona ekler; `test` ön koşul olarak kullanılabilir; medya tipi `application/json-patch+json`.
- Merge Patch: hedef-şekilli bir JSON; `null` = silme (null değer atanamaz); diziler tümüyle değiştirilir; objeler özyinelemeli birleşir; `application/merge-patch+json`.

**Bizim tasarımda karşılığı.** "Genel doküman diff" alternatifi (karar b'nin karşısı).

**AL.**
1. `test` ile **ön koşul** fikri: operasyon "şu alan şu değerdeyse uygula" (optimistic concurrency); bizim semantik operasyonlarda `expectedRevision`/`expect` alanı.
2. Atomik toplu uygulama ("hata olursa hiçbiri").
3. Merge Patch'in sadeliği yalnız **şema-bağımsız dış entegrasyon/debug** için (ör. ayarlar nesnesi) düşünülebilir.

**ALMA, neden.**
- Yol (JSON Pointer) adresleme: dizi indeksi eşzamanlı yazma altında kayar (RFC'nin kendisi shift semantiği tanımlıyor); `/panes/1/indicators/0/period` bayat olur, kimlik tabanlı adresleme gerekir. (Çıkarım, RFC'nin indeks semantiğinden.)
- Niyetin kaybı: `replace /x/period 20` hangi kullanıcı niyetinden geldiğini söylemez (ör. "RSI periyodunu 14 yap" vs. şablon uygula); undo, provenance ve ajan açıklaması için semantik gerekir.
- Değişmez doğrulaması yok: patch geçerli JSON'u şemaya aykırı hale getirebilir; her uygulamadan sonra şema+domain doğrulaması gerekir ve hata mesajı kullanıcı dilinde değil.
- Merge Patch: null atanamaz, dizi bütünüyle değişir (eşzamanlı eleman ekleme ezer), metin birleşimi yok.
- Şemanın tek kaynağı Rust tipleri; JSON Pointer yolları tip değişince sessizce kırılır, semantik operasyon ise Rust tipinde derleme zamanında kontrol edilir (çıkarım).

---------------------------------------------------------------------------

## 7. Karşılaştırma: tek otorite + sıralı log vs CRDT vs OT

Senaryo: bir insan + iki ajan eşzamanlı yazıyor; biri offline'a düşüp geri dönüyor (örneğin 2 saat sonra, bu sürede doküman 300 operasyon ilerledi).

| Kriter | Tek otorite + sıralı log (semantik operasyon) | CRDT (Automerge/Yjs/Loro) | OT (ShareDB/json1, ProseMirror) |
|---|---|---|---|
| **Yakınsama** | Trivial: herkes aynı log sırasını uygular. Offline ajan dönünce operasyonlarını **yeniden gönderir**; otorite kabul/reddeder. Yakınsama garanti, ama ajanın offline işi kaybolabilir (reddedilirse). | Matematiksel garanti, offline iş **asla reddedilmez**; ama sonuç domain açısından anlamsız olabilir (çift gösterge, çelişen aralık, rastgele kazanan). | Merkezi sunucu sırası + transform ile yakınsar; json1 gibi tipler hâlâ yakınsama hatası veriyor (README: ~1M fuzz iterasyonunda). ProseMirror'da basit çünkü adım türleri az. |
| **Mesaj boyutu** | Küçük: bir operasyon = tip + kimlik + parametreler. Offline dönüş = kendi operasyonları + otoriteden "revision N'den sonrası". Boyut domain operasyon sayısıyla orantılı, metin karakter sayısına değil. | Change başına metadata (actor, seq, hash, bağımlılıklar) + tam geçmiş. Automerge 2'de ~%30 ek yük (metin trace'inde). Yjs/Automerge senkronunda fark küçük, ama yeni cihaz tam geçmiş (veya snapshot) indirir. Tombstone büyür. | Küçük: op + versiyon. Ama sunucu iç durumu (op logu) saklaması gerekir. |
| **Kod karmaşıklığı** | En düşük: sıra numarası, kabul/ret, rebase = yeniden değerlendirme. Operasyon başına "ön koşul + uygula + ters" yazılır. Karmaşıklık **operasyon alfabesinde** (her operasyonun reddedilme ve ters kuralları). Sunucu tek nokta (HA gerek). | Kütüphane alırsan düşük görünür (Automerge/Yjs olgun), ama domain modelini CRDT tiplerine çevirmek ve değişmezleri sonradan onarmak gerekir; Rust çekirdek + determinizm değişmezi ile entegrasyon (rastgele actor id, kazanan seçimi) tasarım yükü. Kendi yazarsan yüksek. | En yüksek: operasyon çifti başına transform; Figma "fazla" dedi; json1 hâlâ "preview". Kendi operasyon alfabemiz için N² transform. |
| **Provenance ve undo korunumu** | Doğal: log her satırda origin, zaman, gerekçe, `expectedRevision`. Undo = ters operasyonu log'a ekle; origin bazlı undo (Yjs `trackedOrigins` fikri) kolay. Ters operasyon, sonraki başkası değişikliklerle çakışırsa otorite "undo çakıştı" diye reddeder (Figma'nın "undo kendi geçmişini değiştirir" ilkesi). | Provenance iyi: actor + mesaj + hash zinciri (Automerge), Yjs'te origin kavramı; ama değiştirilemez tam geçmiş = gizlilik/silme sorunu ve undo kütüphane katmanı, çoklu yazar altında "kendi değişikliğimi geri al" yalnız origin/scoped. | Op logu vardır (ShareDB tarihsel sürüm); origin/source taşınabilir ama transform sonrası op orijinal niyetten farklılaşır, undo için ters-transform gerekir (karmaşık). |
| **Determinizm** | Tam: aynı başlangıç + aynı sıralı log = aynı durum; hesap motoruyla uyumlu ("aynı girdi aynı çıktı"). Log, hata ayıklama ve altın görsel testleri için yeniden oynatılabilir. | Durum yakınsar, ama aynı yakınsama için tüm replikaların aynı change kümesini görmesi gerekir; ara durumlar replikaya göre farklı olabilir; actor id'ye bağlı eşzamanlı kazanan. Yeniden oynatma deterministik ancak "log sırası" tek değil (DAG). | Sıra otoriteden gelirse deterministik; transform'un kendisi deterministik olmalı (CP1/CP2 koşulları). |
| **Offline dönüş** | Zayıf nokta: ajan kendi base revision'ından işlemişti; dönüşte (1) operasyonları tek tek yeniden değerlendirilir (`test` ön koşullu), (2) çakışanlar `Rejected(reason)` ile geri döner, ajan yeni durumu görüp tekrar önerir. Bunu bir **ilk sınıf akış** yapmak gerekir. | En güçlü: otomatik birleşir, hiçbir şey reddedilmez. | Sunucu offline op'ları transform eder (ShareDB offline destekler), uzun offline'da transform zinciri uzar. |

**Değerlendirme (kararı sınamak).**
- (a) Tek otorite + sıralı log bu senaryoda **savunulabilir**, ama bedeli açık: offline ajanın işi bazen reddedilir. CRDT'nin gerçek avantajı yalnızca "otoriteye hiç ulaşılamazken iki uç da yazmaya devam ediyor ve sonra *reddetmeden* birleşmeli" gerekliyse ortaya çıkar. (c) kararındaki "doküman cihaza devredilebilir" ifadesi bunu tetikler: devir sırasında otorite **taşınmalıdır**, çoğaltılmamalıdır. Aksi halde iki yazar otorite (split brain). Öneri: devir = log'un belirli bir `revision`'ında "lease/epoch" değişimi; eski otorite o revision'dan sonra yazma kabul etmez; epoch log satırına yazılır. Figma/ProseMirror kaynaklarında bu devir modeli yok; kendi tasarımımız gerekecek (çıkarım).
- CRDT'yi **en iyi haliyle** düşünürsek: Automerge/Loro tam geçmiş, hash zinciri, branch/merge sağlar ve offline'da reddetmeden birleşir; hibrit mümkün: yalnız **gerçekten çok yazarlı serbest metin alanlarında** (ör. not/annotation metni) CRDT, geri kalan yapısal değişiklikler otorite+semantik operasyon. Bu hibrit bu araştırmanın bir hipotezidir, doğrulanmamıştır; ayrı bir kısa prototip ölçümü (tek metin alanı, Automerge/Loro/Yjs) önerilir.
- (b) Semantik operasyon: Bölüm 3 ve 6 birlikte destekliyor (Replicache/Zero mutator'ları, JSON Patch'in indeks/niyet zayıflığı). Zayıf yanı: operasyon alfabesi büyür ve her operasyonun ters/rebase/ön koşul kuralı gerekir; **JSON Patch/merge patch yalnız dış sınır (import/export, debug dump) için** tutulabilir.
- (c) Aynı çekirdek iki uçta: Replicache/Zero'nun "aynı mutator iki yerde" modeli bunu doğrular; fark: onlarda sunucu implementasyonu istemciden farklı olabilir ("server can differ"), bizim determinizm değişmezimiz **aynı Rust kodunun** iki uçta aynı sonucu vermesini gerektirir; farklı uç/platformda f64 ve sıralama farkı riski ölçülmeli.

---------------------------------------------------------------------------

## 8. Açık sorular ve öneriler

1. Offline ajan politikası: reddet mi, "öneri" olarak kuyruğa mı al, otomatik rebase dene mi? (Figma: yeniden uygula; ProseMirror: anlamsızsa düşür; Replicache: sunucu sonucu kazanır.) Karar gerekli, `DECISIONS.md`'ye yazılmalı.
2. Devir (handover) protokolü: epoch/lease tasarımı; kaynaklarda hazır model bulunamadı.
3. Etkileşim durumu (hover, crosshair, seçim, viewport) log dışı; hangi kısmı ajanlara okunabilir ve hangisi belgeye yazılır belirlenmeli (Vega param/signal ayrımı model).
4. Canlı veri akışı için `sequence` ve boşluk tespiti sözleşmesi (TradingView bunu belgelemiyor); revizyon (düzeltme) verisi için "geçmiş geçersiz" olayı.
5. Aşağıdakiler doğrulanamadı ve ayrı araştırma ister: Automerge sync mesaj boyutu/bloom filter (resmi sayfa 404); ShareDB `submitOp`/op-log şeması; Perspective delta/Arrow/index davranışı; Loro peer id, undo ve performans; RFC 7396 metni; Figma orijinal sayfa (yalnız arşiv kopyası okundu); LSP 3.18'in metin belge içeriği sağlayıcısı ve `WorkspaceEditMetadata` ayrıntıları.
6. Bu belgeden doğan küçük ölçüm önerisi: operasyon logu başına bayt (tip+kimlik+parametre) ile Automerge change başına bayt karşılaştırması, gerçek bir grafik oturum izinde (kapsam dışı olduğundan yapılmadı).

---------------------------------------------------------------------------

## 9. Başarıyla fetch edilen URL'ler

- https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/ (2 aralık: 0-100k, 100k-200k)
- https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/ (ilk 100k)
- https://automerge.org/docs/hello/
- https://automerge.org/blog/automerge-2/
- https://automerge.org/blog/automerge-3/
- https://automerge.org/docs/reference/under-the-hood/storage/
- https://automerge.org/docs/reference/under-the-hood/merge-rules/
- https://automerge.org/docs/reference/concepts/
- https://automerge.org/automerge-binary-format-spec/
- https://github.com/automerge/automerge
- https://docs.yjs.dev/api/document-updates
- https://docs.yjs.dev/api/undo-manager
- https://github.com/yjs/yjs
- https://github.com/dmonad/crdt-benchmarks
- https://github.com/loro-dev/loro
- https://share.github.io/sharedb/
- https://github.com/share/sharedb
- https://github.com/ottypes/json1
- https://prosemirror.net/docs/guide/#collab
- https://doc.replicache.dev/concepts/how-it-works
- https://zero.rocicorp.dev/docs/introduction
- https://zero.rocicorp.dev/docs/mutators
- https://madebyevan.com/figma/how-figmas-multiplayer-technology-works/
- https://vega.github.io/vega/docs/api/view/
- https://vega.github.io/vega/docs/signals/
- https://vega.github.io/vega-lite/docs/selection.html
- https://vega.github.io/vega-lite/docs/parameter.html
- https://perspective-dev.github.io/
- https://www.tradingview.com/charting-library-docs/latest/connecting_data/Datafeed-API/
- https://www.rfc-editor.org/rfc/rfc6902
- https://www.rfc-editor.org/rfc/rfc7386

Başarısız (404/403): automerge.org/docs/under-the-hood/{storage,merge_rules,sync}/, automerge.org/docs/reference/under-the-hood/sync/, loro.dev/docs/tutorial/{version,get_started}, share.github.io/sharedb/docs/operational-transform, tradingview .../Streaming/, perspective-dev.github.io/guide/explanation/table/*, vega.github.io/vega/docs/api/view/#data (aynı sayfa, farklı soru), zero.rocicorp.dev/docs/introduction ile ilgili değil. WebSearch yalnız Figma ve Automerge sync için ikincil kaynak olarak kullanıldı (cran.r-project.org vignette, liveblocks, hellointerview vb. okunmadı, sadece arama özetleri; iddialar ikincil olarak işaretlendi).
