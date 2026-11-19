# Matriks mesajları: proto kataloğu, ilk durum ve güncelleme, zaman, üst veri, bar, lisans

Durum: RI1 / T2 (keşif; yalnız okuma). Kaynak: `quantex` deposu, HEAD `f3d95aea`
(`rounds/RI1/experiments/MAP.md`). O depoda hiçbir şey çalıştırılmadı ya da yazılmadı; canlı
bağlantı kurulmadı. Bu belge proto tanımlarını kopyalamaz; alanları kendi tablosuyla anlatır.
Örnek mesajlardaki değerler, iç adresler ve gizli değerler taşınmadı.

## 0. Kaynak kısaltmaları ve güven etiketleri

| Kısaltma | Yol (`quantex` deposu kökünden) | Ne |
|---|---|---|
| `P/` | `services/quantex-stream/matriks-adapter/src/proto/` | 22 Matriks protosu. C++ servislerindeki kopyalar bunlarla bayt düzeyinde aynı (yalnız iki `Timestamp.proto` dosya sonu satır sonunda farklı; `diff` ile denetlendi). |
| `MA/` | `services/quantex-stream/matriks-adapter/src/` | Matriks MQTT → çözme → iç NATS (servis kimliği). |
| `MUA/` | `services/quantex-stream/matriks-user-adapter/src/` | Kullanıcı başına Matriks bağlantısı; baytları çözmeden NATS'a geçirir. |
| `PR/`, `FL/`, `DP/` | `services/quantex_indicators_{price,flow,depth}_cpp/src/` | C++ gösterge servisleri (Matriks'e kendileri bağlanır). |
| `AN/` | `services/quantex-anomaly-engine/src/` | Anomali motoru (C++). |
| `SY/` | `services/quantex-stream/sync-service/src/` | Matriks meta ve topach eşitlemesi (PostgreSQL). |
| `SH/` | `services/quantex-stream/shared/src/` | Ortak kitaplık (iç zarf, ölçekleme). |
| `MD/` | `.worktrees/mcp-feat-anve-regenerate-dual-scope/docs/working-docs/MatriksServices/` | Depo içindeki bir worktree'de duran **Matriks servis belgelerinin kopyası** (kod değil; dal ucu `720ba61`). Kodla çelişirse kod esas alınır. |

Etiketler: **[proto]** proto dosyasında yazılı; **[kod]** kodda görülen davranış; **[belge]**
yalnız `MD/` belgesinde; **Varsayım:** kanıtsız yorum; **Bilinmiyor:** koddan anlaşılmıyor,
kime sorulacağı yazılı.

## Özet

1. 22 protonun 13'ü Quantex kodunda bir yerde çözülüyor; 9'u (Meta, Session, Tradedate, QStatus,
   DepthRow/DepthV2, RankedSymbols, RatingScore, Fundratio, ViopAgentList) hiçbir yerde çözülmüyor.
   Akışta **bar protosu yok**; gün içi barlar akıştan üretiliyor, geçmiş REST'ten geliyor.
2. "İlk mesaj en geniş hal, sonra yalnız değişen alanlar" kuralı Matriks belgesinde market
   (`mx/symbol`, `mx/derivative`) için açıkça yazılı. Emir defterinde güncelleme tüm tabloyu
   taşır, işlemde (`mx/trade`) ilk durum hiç yoktur, derinlikte ilk tablo + satır işlemleri vardır.
3. Hiçbir Matriks protosunda **sıra numarası yok**; abonelikler QoS 0. Boşluk algılama ve
   yeniden eşitleme Matriks tarafında görünmüyor. Tek ipucu kullanılmayan `publishReason`
   (UPDATE/REFRESH) alanı.
4. Quantex tüketicileri alan düzeyindeki güncellemeyi **tutarlı korumuyor**: anomali motoru
   ve `matriks-adapter` eksik alanı 0'a çeviriyor; derinliği yalnız anomali motoru satır
   işlemleriyle doğru kuruyor. Qapsule için ders: çözücü alan varlığını (proto2 `has_`) korumalı.
5. Borsa açıklığı `mx/timestamp` (`TimeMessage`) ile gelir: Unix epoch **mikrosaniye** zaman +
   `isBistPPOpen` + `isBistPPHoliday`; açıklık bilgisi yalnız `source = "MX"` iken anlamlı ve yalnız
   **BIST Pay Piyasası** içindir. Seans evresi (açılış emir toplama, sürekli işlem, kapanış…) ise
   sembol başına `stockStatus` kodlarıyla gelir; Quantex bunu kullanmıyor.
6. (Çelişki notu: Qapsule'ün eski prototip belgesi `docs/MATRIKS_PROTOCOL.md` §10
   `TradeMessage.timestamp`'i "epoch µs, doğrulanmış" diye kaydediyor; aşağıdaki yorum
   anomali motorununkidir; hangisinin doğru olduğu S4 ile Matriks'e sorulur.)
   İşlem, derinlik, PGC zaman damgaları **İstanbul gece yarısından bu yana geçen süre** (ms,
   bazen µs/ns) olarak yorumlanıyor (anomali motoru); diğer servisler aynı alanı ham ms sayıyor.
   Fiyat servisi tik zamanı olarak borsa zamanını değil **varış zamanını** kullanıyor.
7. Canlı bar hizası UTC epoch'una göre taban alma: saatlik ve altı İstanbul'la örtüşür, ama
   günlük bar 03:00 İstanbul'da, haftalık Perşembe'de başlar; REST günlük barı ise İstanbul gece
   yarısıdır. K14 ile uyumsuz; canlı hacim birimi (adet) ile geçmiş hacim birimi (TL) karışıyor.
8. Sembol evreni REST meta'dan (`/dumrul/v4/meta/symbols.gz`) günlük eşitlenir; BIST sembollerinde
   para birimi ve lot alanı yok. Fiyat adımı canlı `priceStep` alanında, ayrıca kodda sabit bir tablo var.
9. Lisans denetimi Quantex kodunda **yok**; Matriks aracı abonelikte reddeder (SUBACK 128) ya da REST
   403 döner. Belgedeki sınıflar: anlık/gecikmeli, KD1 / PD1P-PD2-PD2P / AKDE işlem düzeyleri,
   derinlik ilk 10 / kalan 15 kademe, AKDE (PGC, kurum bazlı Prime), AINEWS, takas yetkisi.
   "Gösterimli / gösterimsiz" ayrımı hiçbir yerde yok.
10. Arkin'in 3. bilgisi **kısmen doğrulandı** (mesaj türüne göre değişiyor), 4. bilgisi **kısmen
    doğrulandı** (açıklık ve zaman var; zaman UTC epoch, yerel saat +3 ile türetiliyor; yalnız Pay Piyasası).

---

## 1. Proto kataloğu

"Uç" sütunu Matriks MQTT bağlantı adıdır (yayın tipi başına ayrı bağlantı;
`MUA/services/topicPlan.ts:9-28`). Adres ve kapılar burada yazılmaz.

| # | Proto → mesaj | Konu kalıbı / kullanım | Uç | İlk durum mu güncelleme mi | Quantex'te çözen | İçerik |
|---|---|---|---|---|---|---|
| 1 | `Symbol.proto` → `SymbolMessage` | `mx/symbol/<SYM>@lvl2` (`MA/services/mqtt-client.service.ts:19`, `MUA/services/topicPlan.ts:193`); fiyat servisi son eksiz `mx/symbol/<SYM>` (`PR/services/mqtt_manager.cpp:392`) | market | İlk mesaj tam görüntü, sonra yalnız değişen alanlar [belge] `MD/MQTT/market.md:27-28,165`. Ayrıca `publishReason` UPDATE/REFRESH [proto] `P/Symbol.proto:84-88` | MA `services/protobuf.service.ts:43-44`; PR `services/mqtt_manager.cpp:536`; AN `main.cpp:1554` | Fiyat, hacim, seans durumu, dönem istatistikleri, temel oranlar |
| 2 | `Derivative.proto` → `DerivativeMessage` | `mx/derivative/<SYM>@lvl2` (`MA/services/mqtt-client.service.ts:20`) | market | Symbol ile aynı [belge] `MD/MQTT/market.md:20-28`; `publishReason` `P/Derivative.proto:81-85` | Yalnız MA `services/protobuf.service.ts:47-48` | VİOP vadeli/opsiyon, varant, sertifika |
| 3 | `DepthTable.proto` → `DepthTableMessage` (+ `DepthCell`) | `mx/depth/<SYM>@lvl2` (ilk 10 kademe), `@lvl3` (kalan 15) [belge] `MD/MQTT/derinlik.md:4,24-25`; AN `main.cpp:623`; DP son eksiz `mx/depth/<SYM>` `DP/services/mqtt_manager.cpp:392`; MA `mx/depth/%s` `MA/services/mqtt-client.service.ts:21` | depth | İlk mesaj tam tablo, sonra satır işlemi (I/U/D + satır no). Belge bunu açıkça söylemiyor; AN kodu böyle yorumluyor `AN/ingestion/depth_ingester.hpp:181-182` | MA, DP `engine/depth_processor.cpp:17`, AN `ingestion/depth_ingester.hpp:126` | Derinlik tahtası |
| 4 | `DepthStats.proto` → `DepthStatsMessage` (proto3) | `mx/depthstats/<SYM>` (AN `main.cpp:624`) | depth | Bilinmiyor (her mesaj 5 alan, proto3) | AN `ingestion/depth_ingester.hpp:93` | Derinlik alt toplamları |
| 5 | `DepthRow.proto` → `DepthRowMessage`, `DepthV2Message` | Kodda konu yok | ? | Bilinmiyor | Hiçbiri | Satır tabanlı derinlik (ikinci sürüm) |
| 6 | `TopOfTheBook.proto` → `TopOfTheBookMessage` (+ `Order`) | `mx/booklet/<SYM>` (AN `main.cpp:625`) | depth | İlk mesaj initial; güncellemeler **tüm tabloyu** içerir [belge] `MD/MQTT/emir-defteri.md:4,97` | AN `ingestion/booklet_ingester.hpp:39-104`; MA `booklet` tipini yükler `services/protobuf.service.ts:67-68` ama konu desenlerinde yok | Emir defteri (emir düzeyi) |
| 7 | `Trade.proto` → `TradeMessage` | `mx/trade/<SYM>` (FL `services/mqtt_manager.cpp:397`), `mx/trade/<SYM>@lvl2` (AN `main.cpp:626`), `mx/tradeex/<SYM>` (AN `main.cpp:627`); `@lvl0`/`@lvl2` son ekleri lisans düzeyi [belge] `MD/MQTT/trade.md:28-32` | trade | **İlk durum yok**; her mesaj tek işlem; geçmiş REST'ten [belge] `MD/MQTT/trade.md:4,92-94` | MA, FL `engine/trade_processor.cpp:27-37`, AN `ingestion/trade_ingester.hpp:96-138` | Gerçekleşen işlem |
| 8 | `Pgc.proto` → `PgcMessage` | `mx/pgc/<SYM>` (FL `services/mqtt_manager.cpp:397`; MUA `mx/pgc/+` `services/topicPlan.ts:146`) | trade | İlk görüntü kullanıcıya özel ayna konudan (`mx/user/<sub>/mx/pgc/<SYM>`), sonraki güncellemeler düz konudan [kod yorumu] `FL/services/mqtt_manager.cpp:520-523`; `MUA/services/userPool.ts:503-511` | FL `engine/pgc_processor.cpp:30-33`, AN `ingestion/pgc_ingester.hpp:97-112` | Para giriş-çıkış (kurumsal akış) |
| 9 | `Timestamp.proto` → `TimeMessage` (proto3) | `mx/timestamp` (`MA/services/mqtt-client.service.ts:23`; PR/FL `services/mqtt_manager.cpp:125`; AN `main.cpp:632`; MUA `services/topicPlan.ts:67`) | market | Her mesaj 4 alanın tamamı (durum mesajı; ilk/güncelleme ayrımı yok — Varsayım) | PR/FL/DP `services/market_clock.cpp:112-160`, AN `services/market_clock.hpp:101-170` | Borsa zamanı, Pay Piyasası açık mı, tatil mi |
| 10 | `News.proto` → `NewsMessage` (+ `RelatedNews`, `AiAnalysis`) | `mx/news@<kaynak>`; kaynaklar matriks, kap, aa, int, comment ve `ai*` karşılıkları (`MA/services/mqtt-client.service.ts:30-41`) | news | Her mesaj tek haber; `deleted` bayrağı silmeyi bildirir [proto] `P/News.proto:23` | MA; MUA (çözmeden) `services/topicPlan.ts:117-135`; AN `ingestion/news_ingester.hpp:43-68` | Haber, AI özeti |
| 11 | `Event.proto` → `EventMessage` | `mx/event/prime/symbol`, `mx/event/prime/agent` (`MUA/services/topicPlan.ts:99-110`) | news | Bilinmiyor; içerik JSON metni [proto yorumu] `P/Event.proto:14-17` | Yalnız MUA (çözmeden geçirir) | Prime olayları (formasyon, dedektör) |
| 12 | `Messenger.proto` → `MessengerMessage` | `mx/messenger`; alarm için `mx/user/<sub>/messenger` (aracı kendisi abone eder) `MUA/services/topicPlan.ts:81-92` | arf | Olay | AN `main.cpp:1784-1797`; MUA | Bildirim: ALERT, DISCONNECT/MULTICONNECT, METAUPDATE |
| 13 | `Meta.proto` → `MetaMessage` | MQTT'de kodda kullanılmıyor. Aynı alan kümesi REST meta yanıtında kısa anahtarlarla geliyor (`SY/clients/matriks-meta.client.ts:20-62`) | — | — | Hiçbiri (proto olarak) | Sembol üst verisi |
| 14 | `Session.proto` → `SessionMessage` | Kodda kullanılmıyor; seans saatleri REST'ten (`PR/services/market_clock.cpp:268`) | — | — | Hiçbiri | Gün başına iki seansın başlangıç/bitişi |
| 15 | `Tradedate.proto` → `TradeDateMessage` | Kodda kullanılmıyor | ? | Bilinmiyor | Hiçbiri | Sembolün işlem tarihi |
| 16 | `QStatus.proto` → `QStatus` (proto3) | Kodda kullanılmıyor. **Borsa durumu değildir**: "QStream" sorgu durumu, `queryId` taşır `P/QStatus.proto:1,17-19` | ? | — | Hiçbiri | Sorgu/akış durumu (START, END, PAUSED…) |
| 17 | `ComputedValues.proto` → `ComputedValuesMessage` | `mx/analytics/warrant`, `mx/analytics/option` [belge] `MD/MQTT/opsiyon-varant-greeks.md:22-23` | analytics | Bilinmiyor | MA yükler `services/protobuf.service.ts:71-72`; MUA'da konu açılmamış `services/topicPlan.ts:152-158` | Opsiyon/varant Greek değerleri |
| 18 | `Ranker.proto` → `RankMessage` (+ `Symbol`) | `mx/stats/{high,low,volume}` (`MA/services/mqtt-client.service.ts:24-26`; eşleme `services/protobuf.service.ts:153`) | market (hisse) ya da stats — kodlar çelişiyor: MA ayrı stats ucu açıyor `MA/services/mqtt-client.service.ts:292`, MUA hisse listelerinin market ucunda olduğunu yazıyor `MUA/services/topicPlan.ts:23-26` | Bilinmiyor | MA | Yükselen/düşen/hacimli listeleri |
| 19 | `RankedSymbols.proto` → `RankedSymbolsMessage` | Yükleniyor ama hiçbir konuya eşlenmiyor (`MA/services/protobuf.service.ts:79-80,144-155`) | ? | — | Hiçbiri | Alan adlı sıralı tablo |
| 20 | `RatingScore.proto` → `RatingScoreMessage` | `mx/ratingscore/<SYM>`; teslim `mx/user/<sub>/...` altında; MUA'da devre dışı `MUA/services/topicPlan.ts:71-78` | market | Bilinmiyor | Hiçbiri | Matriks skorları |
| 21 | `Fundratio.proto` → `FundratioMessage` | `mx/fundratio/<SYM>` (yalnız yorumda `MUA/services/topicPlan.ts:11`) | market | Bilinmiyor | Hiçbiri | Hissede fon oranları |
| 22 | `ViopAgentList.proto` → `ViopAgentListMessage` (proto3) | Kodda konu yok | ? | Bilinmiyor | Hiçbiri | VİOP emir başına alıcı/satıcı kurum listesi (adlardan çıkarım) |

Not: 18 protonun sözdizimi proto2'dir (alan varlığı `has_` ile ayırt edilir); `DepthStats`,
`QStatus`, `Timestamp`, `ViopAgentList` proto3'tür ve sayısal/mantıksal alanlarında varlık bilgisi yoktur
(`P/DepthStats.proto:1`, `P/QStatus.proto:2`, `P/Timestamp.proto:1`, `P/ViopAgentList.proto:1`).

---

## 2. Proto başına alanlar

Tip sütunu proto tipidir. "Satır" `P/<dosya>` içindeki satırdır.

### 2.1 `SymbolMessage` (toplam 85 alan, 1–85; `P/Symbol.proto:7-98`)

Yalnız `symbolId` zorunlu; diğerlerinin hepsi isteğe bağlı (bu, alan düzeyinde güncellemeyi mümkün kılar).

| Alan | Tip | Anlamı | Satır |
|---|---|---|---|
| `symbolId` | sint32 (zorunlu) | Matriks sayısal sembol kimliği | 8 |
| `symbolCode`, `stockSymbolCode` | string | Matriks sembol kodu; borsadaki kodu | 9, 52 |
| `updateDate` | string | Son güncelleme anı, ISO metni (proto yorumu `Z` ile UTC diyor; belgedeki örnek `+03` ofsetli) | 11 |
| `bid`, `ask` | double | En iyi alış / satış fiyatı | 12, 13 |
| `bidSize`, `askSize` | int64 | En iyi alış / satış miktarı | 68, 67 |
| `last` | double | Son işlem fiyatı | 16 |
| `lastQuantity` | double | Son işlemin adedi | 59 |
| `open` | double | Günün açılış fiyatı | 54 |
| `low`, `high` | double | **Seans** içi en düşük / en yüksek | 14, 15 |
| `dailyLow`, `dailyHigh` | double | **Gün** içi en düşük / en yüksek | 19, 20 |
| `dayClose` | double | Bir önceki günün kapanışı | 17 |
| `basePrice` | double | Baz fiyat (yorumsuz; Varsayım: fiyat limitlerinin dayanağı) | 49 |
| `limitUp`, `limitDown` | double | Tavan / taban | 33, 34 |
| `priceStep` | double | Fiyat adımı | 48 |
| `fractionCount`, `tradeFraction` | int32 | Fiyat ondalık basamağı; işlem (adet) ondalık basamağı | 18, 51 |
| `quantity` | double | İşlem adedi (lot) | 21 |
| `volume` | double | İşlem hacmi (tutar) | 22 |
| `dailyQuantity`, `dailyVolume` | double | Gün içi toplam lot; günlük işlem hacmi. Belgedeki örnekte `quantity = dailyQuantity` ve `volume = dailyVolume` (Varsayım: ikisi de gün içi birikimli) | 55, 45 |
| `incrementalQuantity` | double, varsayılan 0 | Yorumsuz. Varsayım: son mesajdan bu yana eklenen adet. PR bunu tik hacmi olarak kullanıyor `PR/services/mqtt_manager.cpp:550-551` | 82 |
| `totalTradeCount` | double | Gün içi işlem sayısı | 58 |
| `priceMean` | double | Gün içi ağırlıklı ortalama fiyat | 32 |
| `difference`, `differencePercent` | double | Son fiyat − önceki kapanış; yüzdesi | 23, 24 |
| `weekLow/High/Close`, `monthLow/High/Close`, `yearLow/High/Close`, `prevYearClose`, `threeMonthClose`, `sixMonthClose`, `nineMonthClose`, `fiveYearClose`, `*PriceMean`, `days7DifPer`, `days30DifPer`, `week52DifPer` | double | Dönem istatistikleri (hafta/ay/yıl düşük-yüksek, önceki dönem kapanışları, dönem ortalamaları, fark yüzdeleri) | 25-31, 60-64, 73, 75-77, 94-97 |
| `sessionIsOpen` | bool | Seans açık mı | 46 |
| `openForTrade` | bool (kullanımdan kalkmış) | İşleme açık mı | 47 |
| `stockStatus` | int32 | Sembolün işlem durumu kodu (1 Kapalı, 2 Sürekli İşlem, 4 Açılış Emir Toplama, 5 Kapanış Emir Toplama, 6 Ara, 10 Geçici Durdurma, 26 Seans Dışı… 1–55 ve 99; tam liste proto yorumunda) | 81; liste 107-163 |
| `direction` | int32 | Yorumsuz (Varsayım: son fiyat değişiminin yönü) | 74 |
| `eqPrice`, `eqQuantity`, `eqRemainingBidQuantity`, `eqRemainingAskQuantity` | double | Emir toplama evresinde teorik eşleşme fiyatı, adedi ve kalan alış/satış | 69-72 |
| `publishReason` | enum UPDATE=0 / REFRESH=1, varsayılan UPDATE | Yayın nedeni. Quantex'te hiçbir yerde okunmuyor (arama: `publishReason`, `publish_reason` → yalnız proto) | 84-88 |
| `tradeDate` | string | İşlem tarihi (belge örneğinde GG/AA/YY biçimi) | 53 |
| `period` | string | Yorumsuz (Varsayım: finansal tablo dönemi) | 65 |
| `symbolType`, `symbolGroup`, `actionType` | string | Sembol tipi (S, I, V, F…), grup (A/B/C/D), işlem tipi | 50, 44, 56 |
| Temel veriler: `netProceeds`, `priceProceeds` (F/K), `marketValue`, `marketValueUsd`, `marValBookVal` (PD/DD), `equity`, `capital`, `circulationShare`, `circulationSharePer`, `shiftedNetProceed`, `netDebt`, `shiftedEbitda`, `beta100`, `cashNetDividend`, `dividendYield` | double | Şirket temel verileri ve oranları | 35-43, 66, 78-80, 92-93 |
| `xu030Weight`, `xu050Weight`, `xu100Weight` | double | Endeks ağırlıkları | 89-91 |
| `brutSwap` | int32 (kullanımdan kalkmış) | — | 57 |

Not: Proto dosyasının sonunda Messenger kodları (ALERT, DISCONNECT, MULTICONNECT, METAUPDATE) yorum
olarak yinelenmiş (`P/Symbol.proto:100-105`); Symbol mesajıyla ilişkisi yok.

### 2.2 `DerivativeMessage` (toplam 78 alan; `P/Derivative.proto:7-97`)

Symbol ile ortak alanlar aynı anlamdadır (fiyat, hacim, dönem istatistikleri, `stockStatus`, `eq*`,
`incrementalQuantity`, `publishReason`). Farklı ya da türeve özgü olanlar:

| Alan | Tip | Anlamı | Satır |
|---|---|---|---|
| `updateDate` | string | Yorumdaki biçimde saat dilimi yok (`yyyy-MM-ddTHH:mm:ss`) | 11 |
| `strikePrice`, `maturity`, `underlying`, `optionClass` (P/C) | double, string, string, enum | Kullanım fiyatı, vade (ISO tarih), dayanak, opsiyon sınıfı | 19-27 |
| `settlement`, `settlementPerDif`, `preSettlement` | double | Uzlaşma fiyatı, farkı, önceki uzlaşma | 32-33, 53 |
| `openInterest`, `openInterestChange` | int64, double | Açık pozisyon ve değişimi | 34, 47 |
| `theoricalPrice`, `theoricelPDifPer` | double | Teorik fiyat ve farkı | 35-36 |
| `openForTrade` | bool | İşleme açık mı (burada kullanımdan kalkmamış) | 39 |
| `marketMaker`, `marketMakerAsk/Bid`, `marketMakerAsk/BidClose` | string, double | Piyasa yapıcı (ihraççı) ve kotasyonları | 57-59, 62-63 |
| `initialMargin` | double | Başlangıç teminatı | 74 |
| `riskLevel`, `rate` | string, double | Risk düzeyi; oran (yorumsuz) | 61, 56 |
| `stockStatus` | int32 | İşlem durumu kodu (liste Symbol ile aynı varsayılıyor; bu dosyada liste yok) | 69 |

### 2.3 Derinlik: `DepthTableMessage`, `DepthCell` (`P/DepthTable.proto:7-35`)

| Alan | Tip | Anlamı | Satır |
|---|---|---|---|
| `DepthCell.price`, `.quantity`, `.timestamp`, `.orderCount` | double, int64, int64, int64 (hepsi zorunlu) | Kademe fiyatı, adedi, zamanı, emir sayısı | 8-11 |
| `symbol` | string (zorunlu) | Sembol | 15 |
| `dateSymbol` | string | Tarih+sembol bilgisi | 16 |
| `timestamp` | int64 | Mesaj zamanı (birim için §4) | 17 |
| `asks`, `bids` | tekrarlı `DepthCell` | Satış / alış kademeleri (tam tablo mesajında dolu) | 18-19 |
| `action` | enum I=0, U=1, D=2 | Satır işlemi: ekle / güncelle / sil | 20-25 |
| `bidAsk` | enum A=0, B=1 | İşlemin tarafı (A satış, B alış) | 26-30 |
| `row` | int32 | Etkilenen kademe numarası | 31 |
| `actionPrice`, `actionQuantity`, `actionOrderCount` | double, int64, int64 | Satır işleminin yeni değerleri | 32-34 |

Kademe sayısı: `@lvl2` ilk 10, `@lvl3` kalan 15, toplam 25 [belge] `MD/MQTT/derinlik.md:4`; AN tahtayı
25 kademe tutuyor `AN/state/symbol_state.hpp:55`.

`DepthRowMessage` (`P/DepthRow.proto:7-22`): `date`, `price`, `quantity`, `timestamp` (hepsi zorunlu),
`actionOrderCount`, `bidAsk`; `DepthV2Message` bunların listesi. Kodda kullanılmıyor.

`DepthStatsMessage` (proto3, `P/DepthStats.proto:7-13`): `timestamp` int64, `totalBidWAvg` ve
`totalAskWAvg` (alış/satış ağırlıklı ortalama fiyat), `totalBidQuantity` ve `totalAskQuantity`
(toplam lot; protoda double, belgede long `MD/MQTT/derinlik.md`).

### 2.4 Emir defteri: `TopOfTheBookMessage`, `Order` (`P/TopOfTheBook.proto:7-31`)

| Alan | Tip | Anlamı |
|---|---|---|
| `Order.orderID`, `.timestamp`, `.quantity`, `.price` | int64, int64, double, double | Emir kimliği (belgeye göre çoğunlukla boş/anonim `MD/MQTT/emir-defteri.md:98`), zaman, adet, fiyat |
| `symbol` | string | Sembol |
| `asks`, `bids` | tekrarlı `Order` | Satış / alış emirleri |
| `action`, `bidAsk`, `affectedRow` | enum I/U/D, enum A/B, int32 | Değişen satır ve işlem türü |

### 2.5 İşlem: `TradeMessage` (`P/Trade.proto:8-18`)

| Alan | Tip | Anlamı | Lisans düzeyi [belge] `MD/MQTT/trade.md:39-47` |
|---|---|---|---|
| `symbol` | string (zorunlu) | Sembol | hepsi |
| `orderNo` | string | İşlem numarası | lvl1+ |
| `price` | **float (32 bit)** | İşlem fiyatı | hepsi |
| `quantity` | int64 | Lot | lvl1+ |
| `activeBidOrAsk` | string | Saldırgan taraf: `b` alış, `a` satış | lvl1+ |
| `timestamp` | int64 | İşlem zamanı (birim için §4) | hepsi |
| `buyer`, `seller` | string | Alan / satan kurum kodu | lvl2 (AKDE) |
| `isTradeEx` | bool | Özel işlem mi | — |

Dikkat: `price` 32 bit kayan nokta; Qapsule'ün `f64` kuralıyla birlikte yuvarlama farkı üretebilir
(Varsayım: büyük fiyatlı sembollerde son basamakta).

### 2.6 Kurumsal akış: `PgcMessage` (`P/Pgc.proto:7-13`)

| Alan | Tip | Anlamı |
|---|---|---|
| `timestamp` | int64 | Zaman (§4) |
| `pgc5`, `pgc8`, `pgc10` | double | İlk 5, 8, 10 kurumun net para giriş-çıkışı (TL); negatif çıkış, pozitif giriş [belge] `MD/MQTT/para-giris-cikis.md:30-32,62` |
| `symbol` | string | Sembol; yalnız hisseler ve 4 endeks [belge] `MD/MQTT/para-giris-cikis.md:4` |

### 2.7 Zaman ve açıklık: `TimeMessage` (proto3, `P/Timestamp.proto:7-22`)

| Alan | Tip | Anlamı |
|---|---|---|
| `timestamp` | int64 | Unix epoch **mikrosaniye** [belge] `MD/MQTT/zaman-damgasi.md:22`; AN yorumu aynı `AN/services/market_clock.hpp:96` |
| `source` | string | Kaynak: `MX`, `BISTPP`, `BISTEX`, `BISTVIOP` [belge] `MD/MQTT/zaman-damgasi.md:23` |
| `isBistPPHoliday` | bool | Bugün Pay Piyasası tatil mi; yalnız `source = MX` iken anlamlı [belge] `:24` |
| `isBistPPOpen` | bool | Pay Piyasası açık mı; yalnız `source = MX` iken anlamlı [belge] `:25` |

Proto yorumu iki iç paket tipini ayırıyor: `0xF` paketinde kaynak "MX" ve açıklık/tatil paketten;
`0x1571` paketinde kaynak paketteki borsa (`P/Timestamp.proto:14-22`).

### 2.8 Haber: `NewsMessage` (16 alan; `P/News.proto:8-37`)

`id` (zorunlu), `isFlash`, `deleted`, `timestamp` (int64), `headline`, `content`, `source[]`,
`category[]`, `symbol[]` (ilgili semboller), `language` (varsayılan TR), `attachment[]`, `dailyNewsNo`,
`chainId`, `relatedNews[]` (`id`, `headline`), `tag[]`, `aiAnalysis` (`summary`, `sentiment`,
`importance` int64, `impact`; hepsi zorunlu).

### 2.9 Diğerleri

| Mesaj | Alanlar (kendi özetim) | Kaynak |
|---|---|---|
| `EventMessage` | `id` int64 (zorunlu), `symbol[]`, `category`, `content` (JSON metni) | `P/Event.proto:7-17` |
| `MessengerMessage` | `code` (zorunlu; ALERT, DISCONNECT, MULTICONNECT, METAUPDATE), `from`, `timestamp` int64, `contentType` (varsayılan text/plain; json, md), `subject`, `content` | `P/Messenger.proto:7-28` |
| `MetaMessage` (43 alan) | Kimlik (`symbolId`, `symbolCode`, `deleted`, `modifiedAt` int64), sınıflama (`sectorId/Code`, `exchangeId/Code`, `marketId/Code`, `submarketId/Code`, `symbolType`, `symbolGroup`), işlem (`fraction`, `tradeFraction`, `tradeStatus`, `tradeableStatus`, `multiSession`, `delay`), türev (`underlying`, `optionType`, `optionClass`, `maturity`, `strikePrice`, `contractGroup*`, `multiplier`, `settlementType`, `preSettlementCode`), diğer (`indexes[]`, `issuer`, `openingDate`, `isinCode`, `isProxy`, `cloneId`, `marketMaker`, `ratio`, `fxSymbolId`, `actionType`, `swapType`, `stockSymbolCode`) | `P/Meta.proto:7-51`; `tradeStatus` kod listesi `:53-79` |
| `SessionMessage` | `date` int32, `first_session_start/end`, `second_session_start/end` int32, `exchange_id`, `market_id` (hepsi zorunlu). Sayıların biçimi (HHMM mi, ms mi) **bilinmiyor** — Matriks'e sorulur | `P/Session.proto:1-16` |
| `TradeDateMessage` | `symbol` (zorunlu), `tradedate` string, `timestamp` int64 | `P/Tradedate.proto:8-12` |
| `QStatus` | `status` enum (START, END, PAUSED, RESUMED, CANCELED, INFO, WARNING, ERROR), `queryId`, `statusText` | `P/QStatus.proto:6-20` |
| `ComputedValuesMessage` (16 alan) | `symbol` (zorunlu), `updateDate`, `optionClass`, `strikePrice`, `impliedVolatility`, `instrinsicValue`, `timeValue`, `leverage`, `delta`, `gamma`, `theta`, `vega`, `rho`, `breakEven`, `omega`, `sensitivity` | `P/ComputedValues.proto:7-24` |
| `RankMessage` | `symbols[]`, `bist30[]`, `bist100[]`; her öğe `key` (sembol), `value`, `last`, `priceChange`, `additionalValue`, `ask`, `bid` | `P/Ranker.proto:7-22` |
| `RankedSymbolsMessage` | `field[]` (sütun adları), `lines[]` (`symbol`, `value[]`) | `P/RankedSymbols.proto:7-15` |
| `RatingScoreMessage` | `symbol`, `generalScore`, `sectoralScore`, `valuationScore`, `growthScore`, `profitScore`, `finScore`, `timestamp` | `P/RatingScore.proto:7-16` |
| `FundratioMessage` | `symbol`, `emkfon` (emeklilik fonu oranı %), `yatfon` (yatırım fonu oranı %), `timestamp` | `P/Fundratio.proto:7-12`; anlam `MD/MQTT/fon-orani.md:29-30` |
| `ViopAgentListMessage` | `order_no`, `order_count` int32, `buyers[]`, `sellers[]` | `P/ViopAgentList.proto:7-12` |

---

## 3. İlk durum ile güncelleme

### 3.1 Matriks tarafı (belge ve kod yorumları)

| Akış | Davranış | Kaynak |
|---|---|---|
| market (`mx/symbol`, `mx/derivative`) | Aboneliğin ilk mesajı initial (tam görüntü); ayrı abonelik gerekmez; sonrasında yalnız güncellenen alanlar gelir | [belge] `MD/MQTT/market.md:27-28,165`; [kod yorumu] `MA/services/nats-publisher.service.ts:323-325` |
| booklet | İlk mesaj initial; güncellemeler **tüm tabloyu** taşır, `affectedRow` değişen satırı gösterir | [belge] `MD/MQTT/emir-defteri.md:4,97-99` |
| trade | İlk durum yok; geçmiş işlemler REST'ten | [belge] `MD/MQTT/trade.md:4,92` |
| pgc | İlk görüntü kullanıcıya özel ayna konudan gelir, sonra düz konudan güncelleme | [kod yorumu] `FL/services/mqtt_manager.cpp:520-523` |
| depth | Belge ilk durum için bir şey söylemiyor. AN: `asks` ve `bids` dolu ve satır işlemi alanları boş olan mesaj = ilk tablo; diğerleri satır işlemi | [kod] `AN/ingestion/depth_ingester.hpp:181-182`; `AN/main.cpp:1456-1460` |
| timestamp | Her mesaj dört alanı da taşıyan durum mesajı (Varsayım; proto3 olduğundan "eksik alan" ayırt edilemez) | `P/Timestamp.proto:7-12` |

**Sıra numarası ve boşluk algılama:** 22 protonun hiçbirinde sıra/sürüm numarası yok (hepsi okundu).
Abonelikler QoS 0 ile yapılıyor (`MA/services/mqtt-client.service.ts:242`, `MUA/services/userPool.ts:293`),
yani teslim garantisi yok. Matriks'in boşluk bildirdiği ya da yeniden eşitleme isteği aldığı bir yol
kodda ve belgede görünmüyor. Tek aday `publishReason` = REFRESH (`P/Symbol.proto:84-88`); anlamı yazılı
değil ve kimse okumuyor. Varsayım: yeniden abone olmak yeni bir initial getirir (MA dinamik
aboneliği buna dayandırıyor `MA/services/nats-publisher.service.ts:262-283,323-325`), ama bağlantı
koptuktan sonra yeniden abonelikte initial'ın tekrar gelip gelmediği doğrulanmadı — Matriks'e sorulur.

**Eksik alan "değişmedi" mi?** Belgeye göre evet (market). Proto2 `optional` alanlar varlık bilgisini
taşır (C++ `has_x()`), bu yüzden "alan yok" ile "alan 0" ayrılabilir. Proto3 mesajlarında (Timestamp,
DepthStats) bu ayrım yoktur: `isBistPPOpen = false` ile alanın gönderilmemesi aynı görünür.

### 3.2 Quantex tüketicileri bunu nasıl ele alıyor

| Tüketici | Yöntem | Sonuç | Kaynak |
|---|---|---|---|
| `matriks-adapter` (TS) | protobufjs `toObject(..., defaults: true)` ile çözüyor | Varsayım (kütüphane belgesine dayanır, bu round'da denenmedi): eksik alanlar varsayılan değerle (0, "", false) doldurulur, varlık bilgisi kaybolur | `MA/services/protobuf.service.ts:112-118` |
| | Sayıya çevirmede tanımsız/geçersiz → 0 | Eksik alan 0 olur | `MA/services/nats-publisher.service.ts:830-836` |
| | Sembol başına **süreçteki ilk** mesaj SNAPSHOT, sonrakiler önceki duruma göre fark (DELTA); fark yoksa yayın yok | Kendi önbelleğiyle alan farkı. Önbellek yeniden bağlanmada temizlenmiyor; Matriks'in yeni initial'ı fark olarak gider | `MA/services/nats-publisher.service.ts:459-481,366-368` |
| | Fark yalnız 21 alanda hesaplanıyor (`last`…`eqRemainingAsk`) | `limitUp/Down`, `priceStep`, `sessionOpen`, dönem istatistikleri, temel veriler güncellemelerde hiç yayınlanmıyor; yalnız snapshot'ta | `MA/services/nats-publisher.service.ts:720-758` |
| | Bir alan "önceki değerden farklı ve tanımsız değil" ise farka giriyor; önbellek yeni durumla tümden üzerine yazılıyor | Eksik alan 0 geldiyse farka `0` olarak girer ve önbellek 0'lanır (yukarıdaki Varsayım doğruysa) | `MA/services/nats-publisher.service.ts:726,474` |
| | Kendi sıra numarası: `snapshotSeq` = ilk mesajın duvar saati, `seq` her mesajda +1 | Matriks'in değil Quantex'in sırası; süreç yeniden başlayınca sıfırlanır | `MA/services/nats-publisher.service.ts:411-418` |
| | Yeniden eşitleme: NATS isteği `streamx.symbol.refresh` → önbellekteki durumdan snapshot | İç yeniden eşitleme var; Matriks'ten değil önbellekten | `MA/services/nats-publisher.service.ts:207-319` |
| | Derinlik: her mesaj tam tablo sayılıyor; `action/row` yok sayılıyor | Satır işlemi mesajları boş tablo olarak yayınlanır (Varsayım: yanlış) | `MA/services/nats-publisher.service.ts:777-818` |
| | İşlem: protoda olmayan `side`, `tradeId`, `amount` alanlarını okuyor; `activeBidOrAsk` okunmuyor | Yön her zaman bilinmiyor, işlem no boş | `MA/services/nats-publisher.service.ts:763-772` |
| `matriks-user-adapter` | Çözmeden, olduğu gibi NATS'a geçiriyor; başlıkta konu, biçim, uç, varış zamanı | Birleştirme tüketiciye (mqlaw) kalıyor | `MUA/services/natsPublisher.ts:1-10` |
| Fiyat servisi (C++) | `has_` ile okuyor; durum tutmuyor, her mesajdan bir tik üretiyor | `last` yoksa ama `dayClose` varsa tik fiyatı önceki kapanış olur; ikisi de yoksa mesaj atılır. Hacim: `incrementalQuantity` yoksa birikimli `volume` kullanılır | `PR/services/mqtt_manager.cpp:536-559` |
| Anomali motoru (C++) | `has_x() ? x : 0`, sonra fiyat durumunu tümden üzerine yazıyor | Yalnız `bid` değişen bir güncelleme `last_price`'ı 0 yapar; fiyat ilerletmede `last` 0 ise alış-satış ortası kullanılıyor | `AN/main.cpp:1554-1573,1603-1609`; `AN/state/symbol_state.hpp:27-35` |
| | Derinlik: ilk tablo → yeniden kur; diğerleri satır ekle/güncelle/sil | Matriks modeline uygun tek tüketici | `AN/main.cpp:1456-1460`; `AN/state/symbol_state.hpp:171-300` |
| Derinlik servisi (C++) | Her `DepthTableMessage` tam tablo; geçersizse atılır | Satır işlemleri kaybolur | `DP/engine/depth_processor.cpp:17-63` |

Qapsule için çıkarım (K23): `SeriesSource` çözücüsü proto2 alan varlığını korumalı ve alanı
"gelmedi = değişmedi" diye birleştirmeli; Quantex'in iç NATS yayını (MA) bu anlamı korumuyor.
Rust tarafında proto2 `optional` alanların `Option<T>` olarak çözüldüğü R6'da küçük bir denemeyle
doğrulanmalı (CLAUDE.md: kütüphane davranışı varsayılmaz).

---

## 4. Zaman

### 4.1 Alan alan zaman

| Akış / alan | Birim ve dayanak | Yorumlayan kod | Kaynak |
|---|---|---|---|
| `TimeMessage.timestamp` | Unix epoch **µs** (UTC anı). Belge aynı sayfada "milisaniye hassasiyeti" de diyor | Tüm C++ servisleri µs olarak saklıyor; yalnız ölçüm (metrik) için kullanılıyor | [belge] `MD/MQTT/zaman-damgasi.md:22,67`; `PR/services/market_clock.cpp:122-125`; `PR/main.cpp:368-372` |
| `TradeMessage.timestamp`, `DepthTableMessage.timestamp`, `DepthCell.timestamp`, `PgcMessage.timestamp`, `DepthStatsMessage.timestamp` | AN'ye göre **İstanbul gece yarısından bu yana** geçen süre; çoğunlukla ms, bazı akışlarda µs ya da ns; değer büyüklüğünden birim kestiriliyor, epoch µs ise olduğu gibi | AN: epoch µs'ye çeviriyor | `AN/utils/timestamp_utils.hpp:8-20,48-69`; `AN/ingestion/trade_ingester.hpp:111-112`; `AN/ingestion/depth_ingester.hpp:103,137-138,164` |
| aynı alanlar | Ham değer ms sayılıyor; alan yoksa duvar saati (epoch ms) | FL ve DP: iki farklı ölçek karışabiliyor | `FL/engine/trade_processor.cpp:31`; `FL/engine/pgc_processor.cpp:30`; `DP/engine/depth_processor.cpp:26` |
| `TradeMessage.timestamp` | Belgede tipi "string" | Protoda int64 | `MD/MQTT/trade.md:44` ↔ `P/Trade.proto:14` (çelişki; kod/proto esas) |
| `Order.timestamp` (booklet) | Ham alınıyor, yoksa duvar saati µs | AN | `AN/ingestion/booklet_ingester.hpp:66-71` |
| `SymbolMessage.updateDate` | ISO metin; proto yorumu UTC (`Z`), belge örneği `+03` ofsetli ve milisaniyeli | MA `Date` ile çözüp `tsEvent` yapıyor; yoksa duvar saati | `P/Symbol.proto:11`; `MA/services/nats-publisher.service.ts:893-903` |
| `DerivativeMessage.updateDate` | ISO metin, yorumda saat dilimi yok | Varsayım: JS'te dilimsiz metin yerel saat sayılır; sunucunun saat dilimine bağlı | `P/Derivative.proto:11` |
| `tradeDate` (Symbol/Derivative) | Metin; belge örneğinde GG/AA/YY | Kullanılmıyor | `P/Symbol.proto:53` |
| `NewsMessage.timestamp` | int64; AN ms sayıyor (yoksa duvar saati ms) | AN | `AN/ingestion/news_ingester.hpp:44,67-68` |
| `MetaMessage.modifiedAt` / REST `mA` | Epoch ms | SY artımlı eşitlemede kullanıyor | `SY/database/migrations/001_initial_symbols.sql:126`; `SY/jobs/symbols-sync.job.ts:150-160` |
| Fiyat servisinde tik zamanı | **Varış zamanı** (`now_ms`), borsa zamanı değil | Bar hizası bu zamana göre | `PR/services/mqtt_manager.cpp:533` |
| `matriks-adapter` `tsIngest` | Varış zamanı | — | `MA/services/protobuf.service.ts:125`; `MA/services/nats-publisher.service.ts:435` |

Saat dilimi: C++ kodu İstanbul'u sabit UTC+3 alıyor (`PR/engine/bar_aggregator.hpp:154`,
`AN/services/market_clock.hpp:324-326`, `AN/utils/timestamp_utils.hpp:23`). Ancak fiyat/akış/derinlik
servislerindeki seans saati yedeği `localtime_r` ile **sunucunun yerel saatine** bakıyor
(`PR/services/market_clock.cpp:38-45,312-325`); kapsayıcının saat dilimi İstanbul değilse sapar.
Ayrıca gece yarısına göre çevirme "bugünün UTC gece yarısı − 3 saat" ile yapılıyor
(`AN/utils/timestamp_utils.hpp:29-34,53-54`); İstanbul'da 00:00–03:00 arası bir gün geri kayar
(işlem saatlerini etkilemez).

### 4.2 Borsa açıklığı ve seans evresi

**Açık/kapalı (Pay Piyasası):** `mx/timestamp` → `isBistPPOpen`, `isBistPPHoliday`, yalnız `MX`
kaynağında anlamlı.

- Fiyat/akış/derinlik servisleri: yalnız `MX` kaynağını dinliyor; `BISTPP` kaynağının eski ya da
  çelişkili açıklık değeri taşıyıp açık/kapalı arasında sıçramaya yol açtığı yazılı. Açılış ve kapanış
  geçişlerinde geri çağırma tetikleniyor (`PR/services/market_clock.cpp:127-170`). Mesaj hiç gelmediyse
  REST seans saatleriyle yedek karar (`:25-63`).
- Anomali motoru: `MX` gelene kadar `BISTPP`'yi de kabul ediyor (`AN/services/market_clock.hpp:137-144`)
  — servisler arasında tutarsızlık. Tatilde her zaman kapalı (`:48`).
- Anomali motorunun "rejim"i (CLOSED, PRE_MARKET, OPENING_AUCTION, CONTINUOUS, CLOSING_AUCTION,
  POST_MARKET, LUNCH_BREAK, AFTER_HOURS, HOLIDAY, UNKNOWN; `AN/core/types.hpp:82-92`) borsa mesajından
  değil, açık/kapalı bayrağı + seans saatleri + **sabit pencerelerden** türetiliyor (açılıştan sonraki ve
  kapanıştan önceki 10 dakika "müzayede", açılış öncesi 60 dk, kapanış sonrası 30 dk;
  `AN/services/market_clock.hpp:249-300`).

**Seans evresi (sembol başına):** `SymbolMessage.stockStatus` / `MetaMessage.tradeStatus` kodları
(Kapalı, Sürekli İşlem, Eşleştirme, Açılış Emir Toplama, Kapanış Emir Toplama, Ara, Kapanış Fiyatından
İşlemler, Tek Fiyat Emir Toplama, Geçici Durdurma, Devre kesici sonrası emir toplama, Seans Dışı, VİOP
uzlaşma ve sabit getirili evreleri…; `P/Symbol.proto:107-163`, `P/Meta.proto:53-79`). Bu, borsanın
gerçek evresini sembol düzeyinde verir. Quantex'te: AN çözüyor ama kullanmıyor (`AN/main.cpp:1573`,
`AN/core/types.hpp:202`); MA NATS'a taşıyor (`MA/services/nats-publisher.service.ts:554,749`).
`sessionIsOpen` MA'da snapshot'a giriyor, farka girmiyor (`:555,720-758`).

**Seans takvimi (REST):** `/v1/session-hours` → `BISTPP` anahtarı altında [ilk açılış, kapanış,
ikinci açılış, ikinci kapanış] "SS:DD" metinleri (`PR/services/market_clock.cpp:189-209,268`);
`/v1/holidays?startdate&enddate` → tatil günleri (`PR/services/matriks_client.cpp:476-483`). Belgeye
göre tatil servisi yarım günleri `halfday=true` ile ayrıca verir (`MD/Meta Veriler/tatiller.md`);
Quantex bu parametreyi kullanmıyor (arama: `halfday` yok). `Session.proto` kullanılmıyor.

---

## 5. Sembol evreni ve üst veri

| Bilgi | Nereden | Kaynak |
|---|---|---|
| Abone olunacak semboller | `matriks-adapter`: ortam değişkeni ya da symbol-service (o da PostgreSQL'den) | `MA/index.ts:171-218` |
| Sembol tablosu | sync-service, REST meta `/dumrul/v4/meta/symbols.gz` (sıkıştırılmış JSON, kısa anahtarlar); varsayılan her gün 08:30; artımlı eşitleme `modifiedAt` ile | `SY/clients/matriks-meta.client.ts:311-322,442`; `SY/config/env.ts:53`; `SY/jobs/symbols-sync.job.ts:150-160` |
| Alanlar | Sembol kimliği/kodu, açıklama, tip (S/I/V/F/O/X/M/R/K/E/B/Z/C), borsa, pazar, alt pazar, sektör, ISIN, fiyat ondalığı (`f`), işlem ondalığı (`tF`), çarpan, oran, işlem durumu, çoklu seans, endeks üyelikleri, türev alanları (dayanak, vade, kullanım fiyatı, opsiyon tipi/sınıfı, uzlaşma tipi), piyasa yapıcı | `SY/clients/matriks-meta.client.ts:20-62`; `SY/database/migrations/001_initial_symbols.sql:70-130` |
| Borsa, sektör, pazar, üye (kurum) listeleri | `/meta/exchanges.gz`, `sectors.gz`, `markets.gz`, `members.gz` | `SY/clients/matriks-meta.client.ts:399-421` |
| Para birimi | BIST sembollerinde **alan yok**; yalnız yabancı sembollerde `cur` | `SY/clients/matriks-meta.client.ts:20-62,120` (Varsayım: BIST = TRY) |
| Lot büyüklüğü | **Alan yok** (ne Meta'da ne Symbol'de). `tradeFraction` işlem adedinin ondalık basamağı [belge] `MD/Meta Veriler/symbols.md:52` | Bilinmiyor — Matriks'e sorulur |
| Fiyat adımı | (a) Canlı: `SymbolMessage.priceStep` / `DerivativeMessage.priceStep` (`P/Symbol.proto:48`, `P/Derivative.proto:50`). (b) Matriks'in kimlik doğrulamasız `price-step-v2.json` servisi; aralık başına adım [belge] `MD/Meta Veriler/fiyat-adimlari.md`; Quantex kullanmıyor. (c) DP'de **sabit tablo**: 8 fiyat aralığı, en üst adım 2,50 (`DP/utils/bist_tick_size.hpp:5-14`), yayılım göstergesinde kullanılıyor (`DP/indicators/depth/bid_ask_spread.hpp:28`). Belge 5 aralık sayıyor; sabit tablo ile belge farklı | — |
| Seans takvimi | REST seans saatleri ve tatiller (§4.2) | — |
| Konu kuralları (hangi sembol tipi hangi konu, gerçek zamanlı mı gecikmeli mi) | `/dumrul/v2/topach` → `topic_rules` tablosu, her gün 08:30; adapter symbol-service'ten okur, yoksa sabit varsayılanlar | `SY/jobs/topach-sync.job.ts:24`; `MA/services/topic-resolver.service.ts:79-160` |

Metaveri değişikliği canlı olarak `MessengerMessage.code = METAUPDATE` ile bildiriliyor olabilir
(`P/Messenger.proto:23-28`; `MD/MQTT/bilgilendirme-mesajlari.md` "METAUPDATE"); Quantex bunu dinleyip
eşitleme başlatmıyor (arama: `METAUPDATE` yalnız protolarda).

---

## 6. Bar verisi

**Akışta bar yok.** 22 protonun hiçbiri bar taşımıyor; `SymbolMessage` yalnız günün açılış, yüksek,
düşük, son ve önceki kapanışını taşıyor. Gün içi barları Quantex akıştan üretiyor, geçmişi REST'ten alıyor.

### 6.1 Fiyat servisi (C++) — canlı bar üretimi

| Adım | Ne yapıyor | Kaynak |
|---|---|---|
| Tik kaynağı | `mx/symbol/<SYM>` mesajları (işlem akışı değil); fiyat = `last` (yoksa `dayClose`), hacim = `incrementalQuantity` (yoksa birikimli `volume`), zaman = varış | `PR/services/mqtt_manager.cpp:533-559` |
| Tik birleştirme | Fiyat değişmediyse tikler bir pencere boyunca birikir; değişince hemen yayın | `PR/engine/tick_merger.cpp:15-87` |
| Bar sınırı | `floor(zaman_ms / periyot_ms) × periyot_ms` — **UTC epoch'una göre** | `PR/engine/bar_aggregator.cpp:350-352` |
| Periyotlar | 1m, 5m, 15m, 30m, 1h, 4h, 1d, 1w (bilinmeyen → 5m) | `PR/core/types.hpp:209-235` |
| Boş periyot | Seans penceresi içindeyse önceki kapanışla düz, hacimsiz "taşıma" barı üretiliyor; hafta sonu ve tatil atlanıyor | `PR/engine/bar_aggregator.hpp:27-31`; `PR/engine/bar_aggregator.cpp:59-83,151-174` |
| Geç gelen tik | Önceki bara değil, açık bara ekleniyor | `PR/engine/bar_aggregator.cpp:192-206` |
| Kapanış | Pay Piyasası kapandı bildirimi (`MX`) gelince açık bar kapatılıyor | `PR/engine/calculation_registry.cpp:1947-1958` |

### 6.2 Geçmiş (REST) ile ilişki

- Başlangıçta: `get_history` → taban periyotlarda `/v1/tick/bar.gz`, diğerlerinde `/v1/tick/derived-bar`;
  parametreler `symbol`, `period`, `count`, `end` = bugün (`PR/services/matriks_client.cpp:25-45,312-316`).
  `divide` verilmiyor → **düzeltilmiş** fiyat (`docs/DATA_ACCESS.md`: varsayılan düzeltilmiş).
- Hacim alanı: `volume` (TL tutar), yoksa `totalQuantity` (`PR/services/matriks_client.cpp:91`).
- OHLC tutarsızsa yüksek/düşük düzeltiliyor, sıfır/negatif bar atılıyor, aynı zamanlı barlar tekilleştiriliyor
  (`:93-106,130-143`).
- Boşluk doldurma: `timestamp` + `count` ile "şu andan sonraki N bar" (1 dk'da en çok 1440, diğerlerinde
  1000) (`:174-200`); gelen barlar taşıma barlarının yerine konuyor (`PR/engine/bar_aggregator.cpp:268-314`).
- Tatiller REST'ten yükleniyor (`PR/engine/calculation_registry.cpp:1259-1280`).

### 6.3 Hiza ve Qapsule K14 ile karşılaştırma

| Periyot | Quantex canlı bar başlangıcı (İstanbul) | Matriks REST (R3 ölçümü, `docs/DATA_ACCESS.md`) | Qapsule K14 | Durum |
|---|---|---|---|---|
| 1m–1h | Saat/dakika başı (UTC+3 tam saat olduğu için UTC tabanı İstanbul'la örtüşür) | Bar başlangıcı | Bar başlangıcı | Uyumlu (zaman borsa değil varış zamanı) |
| Kapanış seansı (18:00–18:05) | 1h'de ayrı 18:00 barı | Ayrı 18:00 barı | Bir önceki bara birleştirilir | **Farklı** |
| 4h | 03:00, 07:00, 11:00, 15:00 sınırları → seans 07–11, 11–15, 15–19 | 240min türetilmiş bar hizası bilinmiyor | Seans takvimine göre | **Farklı / bilinmiyor** |
| 1d | 03:00 (UTC gece yarısı) | İstanbul gece yarısı (21:00 UTC) | 10:00 İstanbul (seans açılışı) | **Üçü de farklı.** Çıkarım: REST'ten gelen günün barı ile canlı akışın aynı günü farklı zamanlara düşer, aynı gün için iki bar oluşabilir |
| 1w | Perşembe 03:00 (epoch'un 7 günlük katı) | `weekly` türetilmiş; hizası bilinmiyor | Takvim haftası | **Farklı** |

Birim uyumsuzluğu: geçmiş barların hacmi TL tutar; canlı barların hacmi `incrementalQuantity` (Varsayım:
adet) ya da birikimli `volume` (gün içi toplam tutar). Aynı seride karışıyor. Ayrıca geçmiş düzeltilmiş,
canlı fiyat düzeltilmemiş; kurumsal işlem sonrası seri kırılır (Varsayım; düzeltme R3'te ölçüldü).

---

## 7. Lisans sınıfı

### 7.1 Veri türü → sınıf

Kaynakların çoğu Matriks belgesidir; kod lisansı denetlemiyor (7.2).

| Veri türü | Konu | Sınıf / gereken lisans | Gecikmeli seçenek | Kaynak |
|---|---|---|---|---|
| Fiyat ve özet (market) | `mx/symbol`, `mx/derivative` | Kullanıcı lisansına göre; anlık ve gecikmeli ayrı adresten | Var (`dl`, "genellikle 15 dakika") | `MD/MQTT/market.md:25,29`; `MD/topach.md:266` |
| Gecikmeli adet/hacim | `VQ` veri tipi | KD1 lisansı için | Yalnız gecikmeli | `MD/MQTT/market.md:168`; `MD/topach.md:46` |
| İşlem, düzey 0 | `mx/trade/<SYM>@lvl0` | KD1: yalnız sembol, fiyat, zaman | Yok (yalnız anlık) | `MD/MQTT/trade.md:4,30` |
| İşlem, düzey 1 | `mx/trade/<SYM>` | PD1P, PD2, PD2P: + işlem no, lot, yön | Yok | `MD/MQTT/trade.md:31` |
| İşlem, düzey 2 (kurumsal) | `mx/trade/<SYM>@lvl2` | AKDE: + alıcı/satıcı kurum | Yok | `MD/MQTT/trade.md:32` |
| Özel işlemler | `mx/tradeex` | Bilinmiyor | — | `MD/topach.md` (`tradeex`) |
| Derinlik ilk 10 kademe | `mx/depth/<SYM>@lvl2` (`depth`) | "Lisansa göre en fazla ilk 10"; lisans kodu bilinmiyor | Bilinmiyor | `MD/topach.md:58`; `MD/MQTT/derinlik.md:24` |
| Derinlik kalan 15 kademe | `@lvl3` (`depthExtended`) | "Lisansa göre son 15"; lisans kodu bilinmiyor | Bilinmiyor | `MD/topach.md:59`; `MD/MQTT/derinlik.md:25` |
| Derinlik özetleri, emir defteri | `mx/depthstats`, `mx/booklet` | Bilinmiyor | Bilinmiyor | — |
| Para giriş-çıkış (kurumsal akış) | `mx/pgc` | **AKDE zorunlu** | Yok | `MD/MQTT/para-giris-cikis.md:4,58-59`; kod yorumu `MUA/services/topicPlan.ts:138` |
| Prime sembol olayları | `mx/event/prime/symbol` | Prime + anlık (KD1, PD1P, PD2, PD2P); olay türü lisansa göre değişir | — | `MD/MQTT/prime.md:40` |
| Prime kurum olayları | `mx/event/prime/agent` | Prime + **AKDE** | — | `MD/MQTT/prime.md:41`; `MUA/services/topicPlan.ts:103-105` |
| Haber | `mx/news@<kaynak>` | Kaynak başına (Matriks, KAP, AA, yorum); lisans kodu bilinmiyor | — | `MD/topach.md` (`news`, `newsKap`, `newsComment`) |
| AI haber yorumu | `NewsMessage.aiAnalysis`, `mx/news@ai*` | Belge: AINEWS lisansı. Kod yorumu: "özel lisans yok" — **çelişki** | — | `MD/MQTT/haberler.md:51,100` ↔ `MUA/services/topicPlan.ts:111` |
| Fon oranı | `mx/fundratio` | Takas verisi gösterme yetkisi olan kurumlar | — | `MD/MQTT/fon-orani.md:4,55` |
| Greek değerleri | `mx/analytics/*` | Lisans kapsamında anlık ya da gecikmeli | Var | `MD/MQTT/opsiyon-varant-greeks.md:26-30` |
| Yabancı piyasa | `mx/iex/*`, `mx/nbbo/*` | Yabancı piyasa lisansı | NBBO gecikmeli | `MD/MQTT/yabanci-market.md:33,86` |
| Zaman damgası, açıklık | `mx/timestamp` | Bilinmiyor (Varsayım: herkese açık) | — | — |
| Meta | REST | Kullanıcı yalnız lisanslı borsaların sembollerini görür; borsa kimliği = lisans kimliği | — | `MD/Meta Veriler/symbols.md:47,149`; `MD/Meta Veriler/exchanges.md:73` |
| Takas/MKK verileri (REST) | REST | MKK, AKDE, KD1 vb. lisans | — | `services/quantex-api/src/services/matriks/SERVICES_CATALOG.md:325,389,631` |

Qapsule K33 sınıflarına eşleme: Matriks belgeleri **gösterimli / gösterimsiz** (display / non-display)
ayrımını hiç kullanmıyor; yalnız "veriyi alabilir mi" düzeyinde. AKDE kaynaklı veriler (işlem lvl2
alıcı/satıcı, PGC, Prime kurum olayları) K33'teki "kurumsal akış" sınıfına karşılık gelir; derinlik
için iki düzey (10 / 25 kademe) vardır; fiyat için anlık / gecikmeli ayrımı vardır. Hangisinin
yetkisiz kullanıcıya rakamsız yorum olarak gösterilebileceği (K33) **bilinmiyor**.

### 7.2 Kodda yetki denetimi nerede

- **Quantex kodunda Matriks veri sınıfı için denetim yok.** Aranan sözcükler (`entitle`, `licen`,
  `lisans`, `AKDE`, `KD1`, `MKK`, `permission`, `package`) yalnız yorumlarda, hata kodlarında ve
  aşağıdakilerde çıktı.
- Denetim **Matriks tarafında**: MQTT aracı lisanssız konuyu SUBACK 128 ile reddediyor; kod bunu
  kaydedip geçiyor (`MUA/services/userPool.ts:264-297,479-485`; `MA/services/mqtt-client.service.ts:248-255`).
  REST 403 → C++ `InsufficientLicense` (`PR/services/matriks_client.cpp:452`,
  `FL/services/matriks_client.cpp:139`, `DP/services/matriks_client.cpp:113`).
- Anomali motoru PGC verisi gelmezse PGC dedektörlerini kapatıyor ("AKDE gerekebilir" uyarısıyla) —
  örtük lisans algılama (`AN/main.cpp:2050-2055`). Kurum profilleri yalnız AKDE ile dolu
  (`AN/state/trade_flow_state.hpp:44,105-108`).
- Matriks belirteç yanıtında bir `licenses` listesi tanımlı ama kodda okunmuyor
  (`MA/services/matriks-auth.service.ts:27`).
- Anlık/gecikmeli seçimi `matriks-adapter`'da süreç başına tek ayar (`MA/config/env.ts:147-151`);
  konu kuralındaki sembol başına `rt`/`dl` yalnız günlüğe yazılıyor (`MA/services/mqtt-client.service.ts:478`).
  Varsayılan kurallarda hisse, endeks, varant, vadeli, opsiyon `dl`; döviz, kripto, emtia `rt`
  (`MA/services/topic-resolver.service.ts:116-143`).
- `jwt_validator` Quantex'in **kendi kullanıcısının** belirtecini doğruluyor (katman: free/standard/premium,
  izin listesi; `PR/security/jwt_validator.hpp:13-18`); anahtar yapılandırılmamışsa doğrulama atlanıp
  "premium" kabul ediliyor (`PR/security/jwt_validator.cpp:36-43`). Katman yalnız yüke ekleniyor
  (`PR/services/mqtt_manager.cpp:661`); veri sınıfına göre süzme bulunamadı.
- Sonuç: Quantex servis kimliğinin lisansıyla alınan veri iç NATS'ta lisans etiketi olmadan herkese
  akıyor. K33'ün "kısıtlama otoritede" kuralı için Qapsule bu sınıflandırmayı kendisi taşımalı.

### 7.3 Lisansı bilinmeyenler

`mx/timestamp`, `mx/depthstats`, `mx/booklet`, `mx/tradeex`, derinlik düzeylerinin lisans kodları,
haber kaynaklarının lisans kodları, `mx/stats/*`, `mx/ratingscore`, `ViopAgentList`; tüm sınıflar için
gösterim/gösterimsiz kullanım hakkı ve yeniden dağıtım (bir sistem kimliğiyle alınıp birden çok son
kullanıcıya gösterme). Sorulacak: Matriks lisans/ürün ekibi ve Arkin.

---

## 8. Arkin'in bilgileriyle karşılaştırma

| # | Bilgi | Sonuç | Kanıt |
|---|---|---|---|
| 3 | Mesajlar tipine göre ilk seferde en geniş haliyle, sonra güncelleme olarak gelir | **Kısmen doğrulandı.** Market için Matriks belgesi aynen söylüyor ve MA dinamik aboneliği buna dayandırıyor. Ama türe göre değişiyor: işlemde ilk durum yok; emir defterinde her güncelleme tam tablo; derinlikte ilk tablo + satır işlemi (kod çıkarımı); PGC'de ilk görüntü ayrı (ayna) konudan. Protoda ilk/güncelleme bayrağı yok (`publishReason` kullanılmıyor), sıra numarası yok. Quantex tüketicilerinin bir kısmı alan düzeyindeki anlamı koruyamıyor (§3.2) | `MD/MQTT/market.md:27-28,165`; `MA/services/nats-publisher.service.ts:323-325`; `MD/MQTT/trade.md:4`; `MD/MQTT/emir-defteri.md:4,97`; `AN/ingestion/depth_ingester.hpp:181-182`; `FL/services/mqtt_manager.cpp:520-523` |
| 4 | Borsanın açık olup olmadığı zaman damgasıyla gelir; bu bağlantı hem Türkiye saatini hem açıklık durumunu verir | **Kısmen doğrulandı.** Aynı mesajda zaman + `isBistPPOpen` + `isBistPPHoliday` var ve tüm C++ servisleri açılış/kapanışı buradan alıyor. Ancak: (a) zaman Unix epoch µs (UTC anı); Türkiye saati +3 ile türetiliyor, Quantex bu zamanı bar ya da tik zamanı olarak kullanmıyor; (b) açıklık yalnız **Pay Piyasası** için ve yalnız `MX` kaynağında anlamlı; VİOP için açıklık bayrağı yok; (c) seans evresi (emir toplama, sürekli işlem, kapanış seansı) bu mesajda yok, sembol başına `stockStatus` ile geliyor | `P/Timestamp.proto:7-12`; `MD/MQTT/zaman-damgasi.md:22-25`; `PR/services/market_clock.cpp:112-170`; `PR/main.cpp:368-372`; `P/Symbol.proto:81,107-163` |

---

## 9. Açık sorular

| # | Soru | Kime |
|---|---|---|
| S1 | `publishReason = REFRESH` ne zaman gönderilir; tam görüntü mü demektir? | Matriks |
| S2 | Bağlantı koptuktan sonra yeniden abonelikte initial yeniden gelir mi? Kayıp güncellemeyi anlamanın yolu var mı (sıra, sürüm)? | Matriks |
| S3 | `incrementalQuantity` tam olarak ne (son mesajdan beri adet mi)? `quantity`/`volume` gün içi birikimli mi? | Matriks |
| S4 | İşlem, derinlik, PGC, booklet `timestamp` birimleri ve dayanağı (gece yarısından ms mi µs mi, epoch mu); akış başına sabit mi? | Matriks |
| S5 | `TimeMessage.timestamp` µs mi ms mi (belge iki şey söylüyor); yayın sıklığı; `isBistPPOpen` hangi evrelerde true (açılış emir toplama dahil mi, kapanış seansı dahil mi)? | Matriks |
| S6 | Derinlikte ilk tablo nasıl işaretlenir (AN'nin sezgisi doğru mu); `@lvl2` son eki `mx/symbol` ve `mx/trade` için ne değiştirir (fiyat ve akış servisleri eksiz abone oluyor)? | Matriks |
| S7 | `Session.proto` sayı biçimi, `Tradedate`, `DepthRow/DepthV2`, `ViopAgentList`, `RankedSymbols`, `QStatus` hangi konu ya da istekte kullanılır? | Matriks |
| S8 | `derived-bar` 240min ve weekly barlarının hizası; kapanış seansı barının türetilmiş periyotlarda ele alınışı | Matriks (ya da R6'da REST ile küçük ölçüm) |
| S9 | Lisans: gösterim/gösterimsiz kullanım hakkı veri sınıfı başına; derinlik ve haber lisans kodları; `mx/timestamp` lisansı; bir sistem kimliğiyle alınan verinin birden çok kullanıcıya gösterilmesi | Matriks lisans ekibi, Arkin |
| S10 | BIST sembolleri için lot büyüklüğü ve para birimi nereden alınmalı? Fiyat adımı için `price-step-v2.json` mı canlı `priceStep` mi esas? | Matriks |
| S11 | MA'da fark hesabının 21 alanla sınırlı olması ve `sessionOpen`'ın farka girmemesi bilinçli mi? `defaults: true` ile eksik alanların 0'lanması bilinen bir durum mu? İşlem dönüşümünde `side`/`tradeId` alan adları neden protoyla uyuşmuyor? | Quantex ekibi |
| S12 | AN'nin `BISTPP` kaynağını yedek kabul etmesi ile fiyat servisinin reddetmesi arasındaki fark bilinçli mi? | Quantex ekibi |
| S13 | Fiyat servisinde günlük/haftalık bar hizasının (UTC tabanı) REST barlarıyla uyuşmaması ve canlı hacim biriminin (adet) geçmiş hacimle (TL) karışması bilinen bir durum mu? | Quantex ekibi |
| S14 | AI haber akışı (`mx/news@ai*`) AINEWS lisansı gerektiriyor mu (belge evet, kod yorumu hayır diyor)? | Matriks |
