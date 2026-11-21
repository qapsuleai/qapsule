# Canlı Matriks bağlantısı (R6, RP4)

Qapsule kendi kimliğiyle doğrudan Matriks MQTT'ye bağlanır (K60). Bu belge bağlayıcının
nasıl çalıştığını, nasıl başlatıldığını ve sorun gidermeyi anlatır. Değerler, adresler ve
belirteçler burada yazılmaz; yalnız değişken adları (K61). Ölçümler ve kanıt:
`rounds/R6/REPORT.md`, `rounds/R6/experiments/LIVE-LOG.md`.

## 1. Parçalar

| Parça | Crate | Ağ | Ne yapar |
|---|---|---|---|
| Matriks çözücü | `qp-matriks` | yok | 22 Matriks protosunun bayt eşit kopyası (`proto/`, köken `proto/PROVENANCE.md`), derlemede üretilen prost tipleri (`protox` + `prost-build`, protoc gerekmez); fiyat kartı kitabı (`SymbolBook`, protobuf birleştirme anlamı: **gelmeyen alan değişmedi**, 0 yazılmaz); borsa saati (`MarketClock`, yalnız `MX` kaynağı); bildirim sınıflayıcı (`MULTICONNECT`); seans evresi tablosu (`stockStatus`); ham ileti kaydı (`rawlog`); ortak besleme hattı (`feed::LiveFeed`). |
| Canlı bar üretici | `qp-source::livebar` | yok | Fiyat kartından XIST takvimine hizalı bar (K14): açılış emir toplama günün ilk barına, kapanış seansı son barına; hacim günlük birikimli tutarın farkı; yöntemler `Ticks`, `CardDaily`, `Mixed`, `Trades` (RP4, §6); ek yeri (`resume`); `compare_bars`. |
| Canlı bağlayıcı | `qp-live` | **var** | Belirteç (bellekte, istek sıklığı sınırlı), MQTT 3.1 (`MQIsdp`, WebSocket alt protokolü `mqttv3.1`) WebSocket + TLS (rustls + ring, sertifika doğrulaması açık) üstünde, kendi küçük MQTT kodlayıcısı; uç nokta başına tek bağlantı (süreç içi kayıt + ana ağacın `fixtures/local/live-<uç>.lock` işletim sistemi kilidi; ayrıca belirteç isteyen her süreç için makine çapında `live-identity.lock`); çoklu bağlantı bildiriminde ya da kayan 10 dakikada 2 açıklamasız sunucu kapatmasında dur ve yeniden deneme yok (`.halted` işaret dosyası, K64/A52); üstel geri çekilme (2→60 sn, 6 ardışık hatada dur); günlük maskeleme (`Redactor`); REST istemcisi (`rest::RestClient`: kesin barlar, sembol listesi ve RP4 geçmiş işlemler (`trades_csv`, yalnız düzey 1 sütunlar), aynı servis belirteciyle); işlem ucu `/trade` kendi kilidiyle (§6); sahte sunucu (`mock` özelliği). |
| CLI ve gösterim | `qp-cli` | var | `qp live record|inspect|symbols|bars|compare|trades`, `qp live reset` (ağ yok; yalnız insan), `qp replay --raw`, `qp demo --live`; RP4 ağsız: `qp sim`, `qp demo --sim` (§8). |

`qp-wasm` (cihaz paketi) bu crate'lerin hiçbirine bağlanmaz; hesap çekirdeği (`qp-compute`)
ağ ve saat bilmez (K3).

## 2. Ortam değişkenleri

`.env.local` (Arkin girer; dosya açılmaz, yazdırılmaz). Betikler (`scripts/live.sh`,
`scripts/demo.sh --live`) değişken ortamda yoksa dosyayı kendileri yükler.

| Değişken | Zorunlu | Not |
|---|---|---|
| `MATRIKS_TOKEN_SERVICE_URL`, `MATRIKS_API_USERNAME`, `MATRIKS_API_PASSWORD`, `MATRIKS_SERVICE_TYPE_CODE` | evet | Belirteç |
| `MATRIKS_MQTT_MODE` | evet | `delayed` ya da `realtime`; kodu değiştirmez |
| `MATRIKS_MQTT_DL_HOST` / `MATRIKS_MQTT_RT_HOST` | kipe göre | Tam adres (`wss://…/market`) ya da yalnız ana makine (o zaman `wss://` ve `/market` eklenir) |
| `MATRIKS_HARDWARE_ID` | hayır | Boşsa `qapsule-<makine adı>` (belirleyici, rastgele değil). Önek `qapsule-` değilse uyarı basılır (değer basılmaz). |

Varlık denetimi (değer yazdırmaz): `scripts/live-env.sh`.

## 3. Nasıl başlatılır

```sh
scripts/live-env.sh                                   # değişkenler var mı
scripts/live.sh record --symbols THYAO,GARAN,ASELS --seconds 120   # ham kayıt
scripts/live.sh inspect fixtures/local/live/raw/<dosya>.qpraw       # toplu istatistik
scripts/live.sh symbols                               # evren listeleri fixtures/local/live/u30|u100|utum.txt
scripts/live.sh bars --symbols @fixtures/local/live/u30.txt --period 1min --start 2026-11-19 --unadjusted
scripts/demo.sh --live [--symbols @fixtures/local/live/u30.txt] [--interval 5m] [--history 120]
```

Canlı gösterim yalnız bu makinede açılır: `--live` ile `--lan` ya da `--https-host` reddedilir
(gerçek veri yerel ağa açılmaz). Tailscale üzerinden açmak isteyen kendi vekilini kurar ve
sorumluluğu alır.

Gösterimin denetimi (yalnız 127.0.0.1, konsolda basılan denetim sırrı başlıkta):
`GET /live/stats` (toplu sayılar), `POST /live/drop?secs=N` (bağlantıyı bilerek kes, N sn sonra
yeniden aç: bayat → onarım), `POST /live/quit` (DISCONNECT ile kapat).

## 4. Akış

```
Matriks (market uç noktası, tek bağlantı)
  │ MQTT PUBLISH (protobuf), QoS 0, sıra numarası yok
  ▼
qp-live LiveConnection ── LiveSink ──► rawlog (fixtures/local/live/raw/*.qpraw, varış zamanı + konu + bayt)
  │                                  └► Hub kanalı
  ▼
qp-matriks LiveFeed  (aynı hat `qp replay --raw` ile kayıttan da çalışır)
  │ SymbolBook (alan varlığı) → CardTick (zaman = updateDate) → LiveBars (K14) → DataEvent (§18.4, yerel seq)
  │ borsa zamanı → host_clock ; stockStatus → seans evresi ; bağlantı olayları
  ▼
ReplayHost (otorite; ilk çözüm = REST geçmişi, gecikmeye göre kesilmiş) → kapsül motoru kesinleşen barda
  ▼
qp demo (SSE, görüntüleyici: yalnız açık enstrümanın verisi; piyasa durumu çerçevesi)
```

## 5. Kurallar

- **Alan varlığı:** proto2 isteğe bağlı alanlar `Option<T>`; gelmeyen alan durum üzerinde değişmez.
  İlk ileti (abonelik ya da yeniden abonelik sonrası) tam görüntüdür.
- **Zaman:** bar zamanı fiyat kartının `updateDate`'i (borsa zamanı, `+03`). Gecikmeli kipte varış ondan
  ~15 dk sonradır. Gösterimin saati borsa zamanıdır; REST geçmişi `şimdi − gecikme`'ye kadar
  tamamlanmış barlarla kesilir (REST gecikmeli değildir).
- **Bayat:** bağlantı koparsa bütün canlı beslemeler bayat; bayatken veri olayı ve canlı kapsül
  üretilmez; yeniden bağlanınca kopma aralığı REST'ten yeniden çözülür, sonra canlı devam eder.
- **Çoklu bağlantı:** Matriks bildirimi gelirse bağlantı kapanır, yeniden denenmez, uç nokta
  `.halted` işaretiyle kapalı kalır; yalnız bir insan notuyla açılır. Önce Arkin'e haber verilir.
  Piyasa ucunda bildirim konusu (`mx/messenger`) SUBACK 0x80 ile reddedilir (R6, CONNECTIVITY §8):
  orada çoklu bağlantı **ileti olarak gelmez**, en çok bağlantının sunucu tarafından kapanması olarak
  görünür. Bu yüzden aşağıdaki kural var.
- **Açıklamasız sunucu kapatması (K64, A52):** sunucu açık bağlantıyı bildirim vermeden kapatırsa
  (WebSocket kapanışı, sunucudan MQTT DISCONNECT, akış sonu, TCP sıfırlama / kopma; kod:
  `DisconnectReason::ServerClosed`) bu bir açıklamasız kapatmadır. İlki geri çekilmeyle yeniden
  bağlanır; **kayan 10 dakika içinde ikincisinde** bağlayıcı durur (`HALTED (unexplained server
  close)`), yeniden denemez, `fixtures/local/live-<uç>.halted` işaretine nedeni yazar
  (`halted=unexplained-server-close`, çoklu bağlantıda `halted=multiconnect`) ve
  `live-<uç>.halted.log`'a bir satır ekler. Yeni süreçler de işaret durdukça bağlanmaz (belirteç bile
  istemez). Sayılmayanlar: bizim kapatmamız (durdurma, `/live/drop`, belirteç yenileme) ve yerel ağ /
  yol hataları — `Io` (ör. R6'daki `os error 49`), TCP/TLS/WebSocket el sıkışma hatası
  (`ConnectFailed`), sessizlik (`KeepaliveTimeout`, `ConnackTimeout`), protokol hatası, CONNACK
  reddi (bunların kendi kuralları var: 6 ardışık hatada dur, CONNACK 4/5'te bir kez yeni belirteç).
  Ayrım kapanışı kimin yaptığına bakar; bir ara cihazın gönderdiği TCP sıfırlaması sunucununkinden
  ayırt edilemez ve sayılır (yanlış durma bir insan sıfırlamasına, kaçan ikinci oturum başkasının
  verisine mal olur). Pencere bağlantı nesnesi başınadır (yeniden açılış ya da yeni süreç boş başlar).
  Saat enjekte edilir (`ConnectOptions::clock`; testler 10 dakika beklemez).
- **Sıfırlama yalnız insan işidir:** `.halted` işareti elle silinmez. Bir insan nedenini denetler
  (aynı kimlikle başka oturum var mı, Quantex tarafında etki var mı), sonra
  `scripts/live.sh reset --endpoint <uç> --note "<kim, neyi denetledi>"` çalıştırır. Not boşsa
  hiçbir şey değişmez; uçbirimde uç anahtarını yazarak onay ister, uçbirim yoksa `--i-am-human`
  ister. Not ve zaman `live-<uç>.halted.log`'a eklenir. **Ajan bu komutu kendi başına çalıştırmaz,
  `--i-am-human` bayrağını kendisi vermez; durma görürse durur ve Arkin'e bildirir.** Durmuş bir
  `qp demo --live` süreci sıfırlamadan sonra yeniden başlatılır (süreç içi yasak sürer).
- **Geç katılan tam kopya:** `mode=replica` istemcisi sonradan bağlandığında başlangıç çerçevesi ilk
  geçmişi ve o ana kadar uygulanan bütün veri olaylarını taşır; serisi baştan bağlı istemcininkiyle
  bayt eşittir (RP4/T5 testleri: `livehost::tests::late_full_replica_*`,
  `tests/demo_live.rs::late_replica_gets_the_live_bars_of_an_early_one`).

## 6. İşlem ucu (RP4, K66)

Kesin gün içi bar için işlem (trade) akışı. Kod ve sahte sunucu testleri hazır; gerçek
bağlantı ilk iş gününde (işlem akışı yalnız anlık uçta yayınlanır, K67).

| Konu | Kural | Kodda |
|---|---|---|
| Uç nokta | Piyasa ucuyla aynı ana makine, yol `/trade`; kip `MATRIKS_MQTT_MODE`'a göre (RT/DL değişkeni). Ayrı değişken yok. | `qp_live::LiveConfig::trade_endpoint` |
| Kilit | Anahtar `<kip>-trade` → kendi süreç içi kilidi ve makine kilidi `fixtures/local/live-<kip>-trade.lock`; piyasa ucunun kilidinden bağımsız. Uç nokta başına tek bağlantı ve çoklu bağlantı durması aynen geçerli. Aynı `AuthClient` (aynı kimlik) ile, kimlik kilidini tutan süreç açar (R6 B2). | `LiveConnection::open` |
| Konular | `mx/trade/<SEMBOL>` (düzey 1, eksiz), özel işlemler `mx/tradeex/<SEMBOL>`. `@lvl0` kurulabilir. **`@lvl2` hiç kurulmaz ve abone olunmaz:** konu kurucu reddeder, bağlantı da yerelde reddeder (`SubscribeRejected`, sunucuya gitmez). Fiyat kartının `mx/symbol/<SEMBOL>@lvl2` konusu etkilenmez. | `qp_matriks::{trade_topic, trade_topic_at, trade_topics}`, `qp_live::is_level2_trade_topic` |
| Alanlar | Yalnız düzey 1: sembol, fiyat, adet, yön (`activeBidOrAsk`), zaman, işlem no (`orderNo`), `isTradeEx`. **Alıcı ve satıcı (alan 7, 8) çözülmez**: hiçbir yapıya girmez, saklanmaz, günlüğe yazılmaz. Ham kayıt için `sanitize_trade_payload` yükü bu iki alan olmadan yeniden kodlar. | `qp_matriks::trade` |
| Fiyat | Telde 32 bitlik kayan sayı; en kısa ondalık yazımıyla `f64`'e genişletilir (`12.34f32` → `12.34`), böylece REST metninden okunan fiyata bit eşit olur. | `widen_price` |
| Zaman | Belgeye göre gün başından nanosaniye (İstanbul), **doğrulanmadı**. Birim seçenek: `ns_from_midnight` (varsayılan; tarih varış zamanının yerel tarihi), `epoch_ms`, `epoch_us`, `epoch_ns`, `auto` (her iletide varışa en yakın okuma). Hangi birimin kullanıldığı ve varıştan 60 sn'den uzak işlem sayısı sayaçta (`FeedStats.trades`). | `TradeTimeUnit` |
| İlk durum | **Yok.** İzlenen her sembol bir boşlukla başlar (`watch_trades(sembol, geçmişin_başı)`; izlenmeyen sembolde yerel gece yarısı). Boşluk REST işlem geçmişinden doldurulana kadar sembol bayattır; o arada gelen canlı işlemler bekletilir, barlar kesinleşmez. | `LiveFeed::watch_trades` |
| Bar | `BarMethod::Trades`: XIST takvimine hizalı (K14), açılış emir toplama işlemi günün ilk barına, kapanış seansı işlemleri son bara (kart kuralıyla aynı). Hacim Σ fiyat·adet (TL) ya da Σ adet; barın adedi ayrıca tutulur. Özel işlemler varsayılan olarak bara girmez (`SpecialTrades::Include` seçeneği; kesin karar M27 ölçümünde). Kart bu kipte bar kurmaz; günlük alanlar, seans evresi ve saat için kalır. | `qp_source::livebar` kural 8, `LiveFeed::with_trades` |
| Kopma | İşlem bağlantısı koparsa bütün izlenen semboller bayat; boşluk "işlemlerin tam olduğu son an"dan açılır. Yeniden bağlanınca `pending_trade_repairs` boşlukları verir (bağlantı kapalıyken boş), `repair_trades(&dyn TradeHistory)` ya da `apply_trade_history` doldurur: geçmiş ∪ bekletilenler, işlem no ile tekilleştirilir, borsa sırasıyla (zaman, işlem no) uygulanır; sonuç baştan işlemle kurulan barla bit eşittir (test). Başarısız istek boşluğu açık bırakır. | `LiveFeed::{pending_trade_repairs, apply_trade_history, repair_trades}` |
| Aynı hat | Kart ve işlem aynı `LiveFeed`'den geçer; işlem ucunun bağlantı kayıtları `push_from(FeedSource::Trade, …)` ile girer (piyasa durumunu değiştirmez). | `FeedSource` |
| Topach | `GET /dumrul/v2/topach` yanıtının biçimi bilinmiyor; `parse_topach` herhangi bir JSON'dan konu kalıplarını, veri tipini, uç nokta yolunu ve anlık/gecikmeli ipucunu toplar, ana makine adresini atar. Ağ çağrısı yazılmadı (`RestClient`'e eklenecek). | `qp_live::topach` |

Orkestratörün bağlaması gerekenler: `RestClient::trades` → `TradeHistory` uyarlayıcısı (her
`RestTrade` bir `qp_matriks::Trade`'e; fiyat metinden `f64`), onarım öncesi `advance_clock`
ile borsa zamanı (sessiz sembolün boşluğu "şimdi"ye kadar uzansın), ham kayıtta
`sanitize_trade_payload`.

Barındırıcı tarafı RP4/T4'te hazır (benzetim bunu kullanır, §8): `livehost::MatriksFeed::with_trades`
(işlem girdili `LiveFeed`), `LiveIn::Trade` (işlem ucunun kayıtları, `FeedSource::Trade`),
`HubSink::for_trades` (işlem ucunun bağlantısı için ham kayıt + hub), `LiveRuntime::watch_trades`
ve `set_trade_history`: işlem bağlantısı açılınca çalışma zamanı saati borsa zamanına götürür ve
bütün boşlukları geçmişten doldurur (`fill_trade_gaps`; şimdilik hub iş parçacığında, eşzamanlı).
Bağlanmayanlar: `qp demo --live`'da ikinci (işlem) bağlantısının açılması ve REST uyarlayıcısı.
Bilinen sınır: piyasa ucu koparsa yapılan REST bar onarımı `MatriksFeed`'i yenisiyle değiştirir;
işlem girdili kipte yeni beslemenin izlenen sembolleri ve işlem bağlantısı durumu taşınmaz
(canlı işlem bağlamasında ele alınmalı).

## 7. Sorun giderme

| Belirti | Neden / ne yapılır |
|---|---|
| `… another qp process holds the Matriks identity` | Belirteç isteyen başka bir `qp` süreci (record, symbols, bars, demo --live) çalışıyor; kimlik kilidi `fixtures/local/live-identity.lock` (ana ağaç). Aynı kimlikle eşzamanlı belirteç istenmez. |
| `lock: endpoint … is locked by another process (pid N)` | Aynı makinede başka bir Qapsule süreci bağlı. O süreci kapatın; ikinci bağlantı açmayın. |
| `HALTED (multi-connect …)` | Aynı kimlikle başka bir oturum aynı uç noktaya bağlandı. Yeniden denemeyin; Arkin'e bildirin. `fixtures/local/live-<uç>.halted` dosyası notla sıfırlanana kadar bağlantı açılmaz. |
| `HALTED (unexplained server close: … 2 times within 10 min …)` | Sunucu bağlantıyı 10 dakikada iki kez açıklamasız kapattı (piyasa ucunda çoklu bağlantının görünebildiği tek biçim). Yeniden denemeyin; Arkin'e bildirin. Aynı kimlikle başka bir oturum (Quantex, başka makine) var mı bakılır. Açmak: bir insan `scripts/live.sh reset --endpoint <uç> --note "…"`. |
| `endpoint <uç> halted: …` (açılışta) | Önceki bir süreç durmuş; işaret dosyası duruyor. Belirteç istenmedi, bağlantı açılmadı. Yukarıdaki iki satıra bakın; işaret dosyasının `halted=` satırı nedeni söyler, `live-<uç>.halted.log` geçmişi. |
| `disconnected: server closed the connection` (tek) | Açıklamasız kapatma 1/2: geri çekilip yeniden bağlanır. 10 dakika içinde ikincisi durdurur. |
| `disconnected: i/o error: … (os error 49)` ya da `connect failed` / `no PINGRESP within keepalive` | Yerel ağ ya da yol hatası; açıklamasız kapatma sayılmaz; geri çekilmeyle döner (6 ardışık hatada `TooManyFailures`). |
| `reset …: --note is required` / `--i-am-human` | Sıfırlama notsuz ya da onaysız reddedildi; hiçbir şey değişmedi. Yalnız bir insan, nedeni denetledikten sonra. |
| `mode=replica` sayfası sonradan açılınca yalnız geçmiş görünüyor | RP4/T5'te ağsız üç düzeyde yeniden üretilemedi (başlangıç çerçevesi bütün olayları taşıyor, seri bayt eşit). Yeniden görülürse sayfanın gerçekten `mode=replica` olduğunu (varsayılan görüntüleyicidir), tarayıcı konsolunda hata olmadığını ve `/live/stats` olay sayısını not edin. |
| `subscription refused mx/timestamp (code 0x80)` | Gecikmeli uçta borsa saati konusu yok (ölçüldü). Saat `updateDate`'ten ve takvimden kurulur. |
| `subscription refused mx/symbol/<SYM>@lvl2` | Sembol işlem görmüyor ya da yetki yok. |
| `subscription refused mx/trade/<SYM>@lvl2 (code 0x80)` | Düzey 2 işlem konusu yerelde reddedildi (K66); sunucuya gitmedi. Konuyu eksiz kurun (`trade_topics`). |
| İşlem sayacında `far_from_arrival` yüksek | İşlem zaman birimi yanlış olabilir; `TradeTimeUnit::Auto` ile ölçüp birimi sabitleyin. |
| `retrying in …` | Kopma; ölçülü geri çekilme. 6 ardışık hatada durur (`TooManyFailures`). |
| `token request failed: HTTP 401` | Servis hesabı değişkenleri yanlış. |
| `missing: MATRIKS_…` | `.env.local`'a o değişkeni girin. |

## 8. Hızlandırılmış benzetim (RP4/T4, K65)

Borsa kapalıyken (hafta sonu) canlı yolu gerçek veriyle çalıştırmak için. Bağlantı, belirteç ve
ağ yok; yalnız yerel önbellek. Ayrı bir hat değildir: kod `crates/qp-cli/src/sim.rs`.

```
fixtures/local/trades/<SYM>/<başlangıç>_<bitiş>.csv.gz   (düzey 1, borsa zamanı; qp live trades)
  │ parse_rest_csv → Trade, işlem no ile tekilleştirilir, borsa sırası (zaman, sembol, işlem no)
  ▼ her işlem → işlem ucunun göndereceği ileti (TradeMessage, mx/trade/<SYM>), varış = işlemin zamanı
LiveIn::Trade ──► LiveRuntime (benzetim saati; LiveIn::Clock) ──► LiveFeed::with_trades
  ──► ReplayHost::live (otorite) ──► kapsül motoru ──► (gösterimde) görüntüleyici
```

| Konu | Kural |
|---|---|
| Aynı yol | `qp demo --live` ile aynı türler: `livehost::host_from_history` (barındırıcı), `LiveRuntime`, `MatriksFeed::with_trades`, `LiveIn::Trade`. Test: `sim::tests::simulation_path_is_the_live_path` aynı işlemleri sahte sunucunun işlem ucundan gerçek bir `LiveConnection` + `HubSink::for_trades` ile duvar saatindeki çalışma zamanına verir ve benzetimle **bayt eşit** veri olayları bekler (VERIFY 8). |
| İlk çözüm (ısınma) | Başlangıçtan (ilk günün yerel gece yarısı) önce tamamlanmış son `--history` bar: sembol başına önbellekteki REST barlarından (`fixtures/local/live/rest/<SYM>.<period>.*.unadj.json`, canlı REST kaynağıyla aynı `normalize_rest`) ya da — önbellekte orada daha az bar varsa — önceki günlerin önbellekteki işlemlerinden aynı bar kurucuyla (`BarMethod::Trades`). Hangisinin alındığı sayılır (`history_from_rest`, `history_from_trades`). Geçmişi olmayan sembol dışarıda kalır. |
| Başlangıç saati (RP4/T6) | `--at HH:MM` (`qp sim`) / `--sim-at HH:MM` (`qp demo --sim`): benzetim ilk günün o saatinde başlar; o günün daha önceki işlemleri ısınmaya girer (aynı bar kurucuyla). Ör. `--at 11:00` ile önceki günü önbellekte olmayan semboller 10:00–11:00 barlarıyla (5 dk'da 12 bar) başlar. Varsayılan gece yarısı (önceki davranış, bayt eşit). |
| Bellek (RP4/T6) | Günün işlemleri sembol başına bir kez ve sıkışık tutulur (`sim::Packed`, işlem başına 40 bayt; gönderim sırası okunurken birleştirilir); gösterim barındırıcısının kaydettiği veri olayları da sıkışık (`ReplayHost` içinde, olay başına ≈ 70 bayt). Ölçüm: `rounds/RP4/experiments/T6.md`. |
| İşlem bağlantısı | Başlangıçta işlem ucu "açılır"; başlangıçtan itibaren boşluk önbellekten (`CacheHistory`, canlıda REST) doldurulur, sonra işlemler canlı gibi gelir. |
| Saat | Borsa zamanı. Çalışma zamanının "şimdi"si benzetim saatidir (`LiveRuntime::use_sim_clock`); benzetim onu her işlemin zamanına ve XIST takviminin her bar kapanışına, seans açılış ve kapanışına ve gün sonuna götürür (`sim::schedule`): işlemsiz sembolün barı da kapanır, günler sırayla geçer (kapanış → piyasa kapalı → hafta sonu/tatil atlanır → ertesi işlem günü açılış). |
| Seans evresi | Fiyat kartı olmadığı için sembol başına `stockStatus` yok. Piyasa durumu (açık / kapalı / tatil) XIST takviminden gelir (`MarketSource::Calendar`), canlıda kart sessizken olduğu gibi; sembol evreleri boştur (`phases` 0). Açılış ve kapanış emir toplama işlemleri bar kuralıyla (K14) ilk ve son bara girer. |
| Hız | `--speed N`: seans içinde N× gerçek zaman; kapalı aralıklar (gece, hafta sonu, açılış öncesi) en çok 2 sn sürer. `--speed max`: beklemesiz. Hız yalnız gönderme anını değiştirir; aynı önbellek ve bağımsız değişkenler her hızda **bayt eşit** akış verir (test: `same_input_twice_gives_a_byte_equal_stream_at_any_speed`). Hız sahte saatle sınanır (`pace_follows_the_speed_factor_on_a_fake_clock`). |
| Çıktı | Standart çıktıya yalnız toplu sayılar ve akışın SHA-256 özeti (iki koşuyu veri basmadan karşılaştırmak için). `--out` yalnız `fixtures/local/` altına (başka yer reddedilir): her satır bir veri olayı (`ev`), kapsül doğumu/durum değişimi (`capsule`, `state`, `rev`), piyasa durumu değişimi (`market`) ya da otoritenin iletisi. |
| Kural | `--rule r3` (bugünkü `EngineConfig::r3`). Adlandırılmış kural kayıtları (`sim::RULES`) kalibrasyonla eklenecek; başka ad reddedilir. |
| Gizlilik | Gerçek veri: `qp demo --sim` yalnız 127.0.0.1; `--lan`, `--https-host` ve başka `--bind` reddedilir (qp ve `scripts/demo.sh`; test `tests/demo_sim.rs`, VERIFY 17). |

### Arkin için: izlemek

```sh
# 1) Önbellek var mı (işlemler; REST barları isteğe bağlı)
ls fixtures/local/trades | head; ls fixtures/local/live/rest | head

# 2) Tarayıcıda izle: 30 sembol, iki gün, seansta 60× (5 dk'lık bar 5 sn'de), kapalı saatler 2 sn
scripts/demo.sh --sim --symbols @fixtures/local/live/u30.txt --start 2026-11-17 --end 2026-11-18 --speed 60
#    → http://localhost:8390/  (konsolda basılan denetim belirteciyle sayılar:)
curl -s -H "X-Qp-Live-Control: $QP_LIVE_CONTROL" localhost:8390/live/stats

# 3) Tarayıcısız, en hızlı: 100 sembol, bir gün (toplu sayılar + özet)
target/release/qp sim --symbols @fixtures/local/live/u100.txt --start 2026-11-18 --speed max

# 4) Olay akışını dosyaya (yalnız fixtures/local altına) ve iki koşuyu karşılaştır
mkdir -p fixtures/local/sim
target/release/qp sim --start 2026-11-18 --out fixtures/local/sim/a.jsonl
target/release/qp sim --start 2026-11-18 --out fixtures/local/sim/b.jsonl
cmp fixtures/local/sim/a.jsonl fixtures/local/sim/b.jsonl && echo bayt-eşit
```

Durdurmak: Ctrl-C. Ölçüm (100 sembol, bir gün, `max`): `rounds/RP4/experiments/T4.md`.
Ölçek (580 sembol, 1×, 11:00'den, bir görüntüleyici): `rounds/RP4/experiments/T6.md`:

```sh
scripts/demo.sh --sim --symbols @fixtures/local/live/utum.txt --start 2026-11-20 --sim-at 11:00 --speed 1
```
Not: önbellek inerken iki koşunun özeti farklı çıkar (girdi değişti); karşılaştırmayı inişi biten
günlerde yapın.
