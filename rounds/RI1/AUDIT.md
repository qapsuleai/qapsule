# RI1 — Bağımsız denetim

Denetçi: bağımsız alt ajan (belgeleri yazmadı). Tarih: 2026-11-19. Kaynak: `quantex` HEAD
`f3d95aea`, yalnız okuma (Read/grep/sed/git status). Canlı bağlantı yok; `quantex`'te hiçbir şey
çalıştırılmadı ya da yazılmadı; `.env*` ve gizli dosyalar açılmadı (gömülü değer yerleri yalnız
anahtar adı düzeyinde denetlendi, değer okunmadı).

## Özet hüküm

**Koşullu kabul.** Örneklenen iddiaların hiçbiri yanlış çıkmadı; altısı kısmen doğru (biri
satır kayması, ikisi sayım/kapsam inceliği, üçü INTEGRATION'da kaynak belgeden daha kesin dil).
Sınır taraması temiz. Koşullar: (1) aşağıdaki altı düzeltme uygulanır; (2) INTEGRATION (d2) için
"çalışan sistemi etkilemeden deneme yolu" yazılır ya da neden yazılamadığı + kime sorulacağı
eklenir (ölçüt 6); (3) `rounds/RI1/REPORT.md` henüz yok (ölçüt 9, ana akışın işi).

| Örneklenen | Doğru | Kısmen | Yanlış | Doğrulanamadı |
|---|---|---|---|---|
| 54 | 48 | 6 | 0 | 0 |

Belge başına: CONNECTIVITY 9 (1 kısmen), MESSAGES 9, NATS 9 (1 kısmen), INDICATORS 10,
ANOMALY 9 (1 kısmen), INTEGRATION 8 (3 kısmen; 5'i atıf denetimi).

## Seçim yöntemi

Her belge N satırsa k·N/8 (k = 1…8) satırlarına gidildi ve o satırda ya da hemen sonrasında
kaynak satırı veren ilk olgusal iddia alındı (eşit aralıklı tarama; kolay/zor ayrımı yapılmadı).
Buna, görev tanımındaki yüksek önemli iddialar eklendi: Arkin'in beş bilgisi hükümleri, bağlantı
sayısı/kimlik, ilk durum/güncelleme, zaman birimleri, JetStream yokluğu, RSI 100↔50, Bollinger
`bandwidth` ×100, MACD ısınması, UTC epoch bar hizası, anomali determinizmi ve eşlemesi.
INTEGRATION'da ayrıca atıfların hedef bölümü açılıp aynı şeyi söyleyip söylemediği sınandı.
Aynı olgunun iki belgedeki tekrarı (ör. fiyat adımı tablosu) bir kez sayıldı. Örneklem dışında
kalan, okuma sırasında doğrulanmış ek iddialar sayıma katılmadı (bkz. "Diğer gözlemler").

## İddia tablosu

| # | Belge:satır | İddia (kısa) | Kaynak | Hüküm | Not |
|---|---|---|---|---|---|
| 1 | CONNECTIVITY:12-13 | `hardwareId` çözüm sırası: ortam → `HOSTNAME` → servis adı + rastgele ek | `matriks-adapter/src/config/env.ts:118-132`; `quantex_indicators_price_cpp/src/config/config.hpp:161-173` | Doğru | |
| 2 | CONNECTIVITY:152-157 | TS: `protocolVersion 3`, `MQIsdp`, `JWT`/belirteç, keepalive 10000, `rejectUnauthorized false`, istemci kimliği verilmiyor | `matriks-adapter/src/services/mqtt-client.service.ts:102-114` | Doğru | |
| 3 | CONNECTIVITY:157 | C++ istemci kimliği `quantex-ind-mx-<id>-<unix s>`, temiz oturum, keepalive 10, otomatik yeniden bağlanma kapalı | `quantex_indicators_price_cpp/src/services/mqtt_manager.cpp:85-102` | Doğru | |
| 4 | CONNECTIVITY:91-96 | `set_on_token_update` hiçbir yerde çağrılmıyor (C++ göstergeler **ve anomali**) | `quantex-anomaly-engine/src/main.cpp:589-591`; `.../services/mqtt_manager.hpp:359-365` | **Kısmen** | Üç gösterge için doğru. Anomali motoru çağırıyor; geri çağrı yalnız düşmüş akışları yeniden bağlanmaya işaretliyor. Sonuç (açık bağlantı eski belirteçle sürer) yine geçerli |
| 5 | CONNECTIVITY:116-120 | C++ ağ hatası ve 5xx'te yeniden dener; TS açılışta belirteç alamazsa düşer | `token_service.cpp:213-245`; `matriks-adapter/src/index.ts:348-351` | Doğru | |
| 6 | CONNECTIVITY:256-263 | `DISCONNECT`/`MULTICONNECT` uyarısı; abonelik yerel aracıdan (`subscribe_client`) | `quantex-anomaly-engine/src/main.cpp:1786-1797,1811` | Doğru | |
| 7 | CONNECTIVITY:297-299 | `MATRIKS_SERVICE_HARDWARE_ID` "ortak env" listesinde | `quantex-anomaly-engine/cpp-anomaly-engine-deploy.yml:54-60` | Doğru | |
| 8 | CONNECTIVITY:314 | C++ yeniden bağlanma: 3 ilk deneme, 1 s → 30 s üstel, en çok 5 | `quantex_indicators_price_cpp/src/config/config.hpp:61-65` | Doğru | |
| 9 | CONNECTIVITY:410-423 | `TimeMessage` 4 alan; açıklık yalnız `MX`; anomali `MX` gelene dek `BISTPP` kabul eder | `matriks-adapter/src/proto/Timestamp.proto:7-12`; `price .../market_clock.cpp:121-133`; `quantex-anomaly-engine/src/services/market_clock.hpp:137-144` | Doğru | |
| 10 | MESSAGES:12, 94-96 | 22 proto; 4'ü proto3 (DepthStats, QStatus, Timestamp, ViopAgentList) | `matriks-adapter/src/proto/` | Doğru | |
| 11 | MESSAGES:100 | `SymbolMessage` 85 alan, 1–85; `publishReason` 84-88 | `Symbol.proto` (85 alan satırı; en büyük numara 85) | Doğru | |
| 12 | MESSAGES:196-209 | `TradeMessage` alanları; `price` 32 bit float | `Trade.proto:8-18` | Doğru | |
| 13 | MESSAGES:262-268 | market: ilk mesaj tam, sonra değişen; trade'de ilk durum yok; emir defteri güncellemesi tam tablo | `MD/MQTT/market.md:27-28,165`; `trade.md:4,92`; `emir-defteri.md:4,97` | Doğru | |
| 14 | MESSAGES:290 | `matriks-adapter` fark hesabı 21 alanla sınırlı | `nats-publisher.service.ts:733-755` | Doğru | 17 + 4 eşleşme alanı sayıldı |
| 15 | MESSAGES:316 | İşlem/derinlik/PGC zamanı AN'ye göre İstanbul gece yarısından bu yana; birim büyüklükten; epoch µs ise olduğu gibi | `quantex-anomaly-engine/src/utils/timestamp_utils.hpp:8-20,48-69` | Doğru | Qapsule `docs/MATRIKS_PROTOCOL.md` §10 ile çelişki anılmamış (gözlem 2) |
| 16 | MESSAGES:400 | Bar sınırı `floor(t/p)·p`, UTC epoch; periyotlar 1m…1w, bilinmeyen → 5m | `price .../engine/bar_aggregator.cpp:350-352`; `core/types.hpp:209-235` | Doğru | Günlük 03:00 / haftalık Perşembe çıkarımı aritmetikle tutarlı |
| 17 | MESSAGES:455 | AI haber lisansı: belge AINEWS, kod yorumu "özel lisans yok" | `MD/MQTT/haberler.md:51,100`; `matriks-user-adapter/src/services/topicPlan.ts:111` | Doğru | |
| 18 | MESSAGES:487-489 | JWT anahtarı yoksa doğrulama atlanıp `premium` | `price .../security/jwt_validator.cpp:36-43` | Doğru | |
| 19 | NATS:10-13 | JetStream kodda yok (`jetstream(` vb. yok) | depo geneli arama: kod dosyalarında yok; yalnız belge/yml/betik | Doğru | |
| 20 | NATS:36 | Kova = FNV-1a % 32 | `nats-publisher.service.ts:17,842-850` | Doğru | |
| 21 | NATS:17 | Fiyat ×10000, miktar ×100 | `quantex-stream/shared/src/proto/index.ts:591-614` | Doğru | |
| 22 | NATS:96-97 | Anomali bağlayıcısında komut dinleyicisi yok; AI biçimi atılıyor | `anomaly-adapter/src/` (NATS `subscribe` yok); `transform/anomaly.transform.ts:38-43` | Doğru | |
| 23 | NATS:245-249 | `refresh` 7 sn bekler; önbellekte yoksa dinamik abonelik + 5 sn; yanıt hep `STOCK` | `gateway/src/nats/nats.subscriber.ts:296`; `nats-publisher.service.ts:270,298` | Doğru | |
| 24 | NATS:252-253 | Geçit LRU 10.000 anahtar, 1 saat | `gateway/src/cache/snapshot.cache.ts:52-53` | Doğru | |
| 25 | NATS:283-284 | Geçit yalnız `STOCK` ve `STOCKDEPTH` konularına abone | `gateway/src/nats/nats.subscriber.ts:158-166` | Doğru | |
| 26 | NATS:389 | Geçit NATS'a kimlik geçirmiyor | `gateway/src/nats/nats.subscriber.ts:72-79` | Doğru | |
| 27 | NATS:437 | Geçit `regime`/`health`'e abone değil (`nats.analytics-subscriber.ts:48-50`) | `gateway/src/nats/nats.analytics-subscriber.ts:51-53` | **Kısmen** | İddia doğru, satır yanlış: 48-50 fiyat sinyal konuları; anomali listesi 51-53 |
| 28 | INDICATORS:17 | Fiyat servisi 30 sınıf, 36 ad | `indicator_factory.hpp` (36 tekil `register_indicator`); `indicators/*/` 30 dosya | Doğru | |
| 29 | INDICATORS:110-112 | Doğrulayıcı 11 periyot kabul eder; `parse` 8'ini tanır, gerisi 5m | `security/validation.cpp:112-116`; `core/types.hpp:222-235` | Doğru | |
| 30 | INDICATORS:115 | Bar halkası derleme zamanında 300 | `core/ring_buffer.hpp:19` | Doğru | |
| 31 | INDICATORS:119 | Canlı yayında `is_closed` hep false | `engine/calculation_registry.cpp:1649-1652`; `bar_aggregator.cpp:111-210` (her dal `UPDATED` ile biter) | Doğru | |
| 32 | INDICATORS:339 | RSI: `avg_loss == 0` → Quantex 100; Qapsule düz seride 50 | `indicators/momentum/rsi.hpp:60-61,85-86`; Qapsule `crates/qp-compute/src/studies/rsi.rs:43` | Doğru | |
| 33 | INDICATORS:340 | MACD ısınmada `signal=0`, `histogram=macd` kaydedilir; `fast<slow` denetimi yalnız Qapsule'de | `indicators/trend/macd.hpp:85-89`; Qapsule `studies/macd.rs:36-40` | Doğru | |
| 34 | INDICATORS:341 | BB σ /N; `bandwidth` Quantex ×100, Qapsule oran | `indicators/volatility/bollinger_bands.hpp:101,108`; Qapsule `studies/bollinger.rs:10` | Doğru | |
| 35 | INDICATORS:172-174 | `reset()` hiçbir göstergede ezilmemiş | `indicators/` araması: yalnız `base_indicator.hpp:69` (yorum) ve `:75` | Doğru | |
| 36 | INDICATORS:267 | 44 hazır sinyal: 5/14/18/5/2 | `engine/signal_registry.cpp:864-923`; `signals/definitions/*` sayımı | Doğru | Yol `P/src/engine/signal_registry.cpp` (belgede önek yok) |
| 37 | INDICATORS:604 | `BUY_SELL_EXTREME` iki koşulu VE ile bağlı → tetiklenmez | `quantex_indicators_flow_cpp/src/engine/signal_registry.cpp:1230-1239`; `signals/definitions/flow/buy_sell_extreme.hpp` | Doğru | |
| 38 | ANOMALY:12 | "25 dedektör kayıtlı (`anomaly_detector_factory.hpp:110-715`)" | aynı dosya: 26 `register_detector` (son: `REGIME`, :705) | **Kısmen** | 25 anomali dedektörü + REGIME bağlam dedektörü = 26 kayıt |
| 39 | ANOMALY:53 | Eşik 0,70 / 0,80 / 0,72 | `quantex-anomaly-engine/src/config/config.hpp:71-73` | Doğru | |
| 40 | ANOMALY:62-64 | Soğuma 10/30/60/120 sn, tırmanmada yarıya | `engine/cooldown_manager.hpp:11-16` | Doğru | |
| 41 | ANOMALY:79-83 | Gösterge kurallarında ad uyuşmazlığı; motor tam ad eşitliği arar | `engine/composite_engine.hpp:750`; `config/indicator_config.json:17,27,46` | Doğru | |
| 42 | ANOMALY:121 | TRADE_CLUSTER z eşiği fabrikada 4,5, dosyada 3,0 | `anomaly_detector_factory.hpp:329`; `trade_cluster_detector.hpp:20` | Doğru | |
| 43 | ANOMALY:231-233 | Proto zaman "ms" diyor, motor µs gönderiyor | `quantex-stream/shared/proto/anomaly.proto:18,39` | Doğru | |
| 44 | ANOMALY:238 | ClickHouse yazımı varsayılan kapalı | `config.hpp:114` | Doğru | |
| 45 | ANOMALY:305 | Olay kimliği = duvar saati ms + `random_device` tohumlu sayı | `dispatch/signal_dispatcher.hpp:147-160` | Doğru | |
| 46 | ANOMALY:351-353 | `Muhur` yalnız `net`'i okur, işaret + `floor(log2|x|)` | Qapsule `crates/qp-capsule/src/trust.rs:6-12` | Doğru | |
| 47 | INTEGRATION:20-21 | "`hardwareId` her Quantex pod'unda ayrıdır" | `CONNECTIVITY.md` §1.2 ve §2.5 | **Kısmen** | Varsayılan durumda doğru; §2.5 ortak ortam dosyası doluysa anomali işçilerinin aynı değeri alabileceğini ve bunun bilinmediğini söylüyor |
| 48 | INTEGRATION:42 | Arkin 1: "kimlik `hardwareId`'ye, yani örneğe bağlı" (kesin dil) | `CONNECTIVITY.md` §1.2 ("Varsayım: `sub` = `QTX-` + `hardwareId`") | **Kısmen** | Kaynak belge bunu varsayım olarak işaretliyor; INTEGRATION kesinleştirmiş |
| 49 | INTEGRATION:44 | Arkin 3 → `MESSAGES.md` §3.1, §8 | MESSAGES §3.1 tablo, §8 satır 3 | Doğru | Atıf hedefi aynı şeyi söylüyor |
| 50 | INTEGRATION:45 | Arkin 4 → MESSAGES §2.7, §4.2; CONNECTIVITY §5; NATS §2.5 | dört bölüm | Doğru | |
| 51 | INTEGRATION:55 | PROTOCOL §18.4: kaynak numara taşımıyorsa barındırıcı yerel numara üretir | Qapsule `docs/PROTOCOL.md:1472-1474` (§18.4) | Doğru | |
| 52 | INTEGRATION:63 | "Anomali motoru (25 dedektör …)" | #38 | **Kısmen** | #38 ile aynı |
| 53 | INTEGRATION:104 | Test kimliği/uç noktası → `CONNECTIVITY.md` soru 13 (ve soru 4, 5) | CONNECTIVITY §7 satır 4, 5, 13 | Doğru | |
| 54 | INTEGRATION:147 | µs/ms tutarsızlığı → `ANOMALY.md` soru 7 | ANOMALY §8 satır 7 | Doğru | |

## Yanlış / kısmen olanlar için düzeltme önerileri

- **CONNECTIVITY:91-96** → "Üç C++ gösterge servisinde `set_on_token_update` hiç çağrılmıyor.
  Anomali motoru çağırıyor (`services/quantex-anomaly-engine/src/main.cpp:589-591`), ama geri
  çağrı yalnız o an kopuk olan akışları yeniden bağlanmaya işaretliyor
  (`.../services/mqtt_manager.hpp:359-365`). Her iki durumda açık MQTT bağlantısı eski belirteçle
  sürüyor; yeni belirteç yalnız yeniden bağlanırken kullanılıyor."
- **NATS:437** → kanıtı `nats.analytics-subscriber.ts:51-53` yap (anomali listesinde yalnız
  `data` ve `outcome` var).
- **ANOMALY:12** → "26 dedektör kayıtlı: 25 anomali dedektörü + bağlam olarak REGIME
  (`anomaly_detector_factory.hpp:110-715`, REGIME `:705`; §2.6)."
- **INTEGRATION:63** → "Anomali motoru (25 anomali dedektörü + REGIME bağlamı, sembol başına
  birleşik olay)".
- **INTEGRATION:20-21** → "`hardwareId` varsayılan olarak her pod'da ayrıdır (pod adı); ortak
  ortam dosyasında sabitlenmişse anomali işçileri aynı değeri paylaşabilir — bilinmiyor
  (`CONNECTIVITY.md` §2.5, soru 8)."
- **INTEGRATION:42** → "…kimlik `hardwareId`'ye, yani örneğe (pod'a) bağlı görünüyor
  (Varsayım: belirtecin `sub`'ı `QTX-<hardwareId>`; `CONNECTIVITY.md` §1.2, soru 2)…"

## Sınır taraması (VERIFY 3–6)

Taranan: `docs/matriks/*.md` (6 dosya), `rounds/RI1/*.md`, `rounds/RI1/experiments/*`.

| Tarama | Kalıp | Bulgu |
|---|---|---|
| (a) Gizli değer | `eyJ…` JWT, `BEGIN … PRIVATE/CERT`, `AKIA…`, `sk-…`, `ghp_…`; `password/secret/token/api_key/parola` ardından `=`/`:` + değer | **0** |
| (b) Tam iç adres | IPv4; `scheme://host` (http, ws/wss, mqtt, nats, tcp, redis, postgres, clickhouse); `:port` ve bilinen kapılar; `.svc`, `cluster.local`, `localhost`, alan adı kalıpları | **0.** Tek eşleşmeler yanlış pozitif: `ANOMALY.md:109` (`:443-455` satır aralığı), `rounds/RI1/PROMPT.md:57` ve `experiments/MAP.md:50` (`.env.local` dosya adı). Adresler her yerde değişken adıyla genelleştirilmiş |
| (c) Kopyalanmış proto / uzun kod | `message`/`enum`/`syntax`/`optional|required|repeated` satırları, `= n;` alan tanımları; çitli bloklar | **0 proto.** Çitli bloklar: `INDICATORS.md:77-90` (12 satır) ve `NATS.md:348-375` (26 satır) — ikisi de ASCII akış şeması, kod değil. `PROMPT.md:6-10` (3 satır). Satır içi alıntılar alan/konu adı düzeyinde |
| (d) Gerçek piyasa verisi | 7+ haneli sayılar, yaygın BIST kodları yanında fiyat, sayısal kullanıcı kimliği | **0.** Fiyat adımı tablosu (`INDICATORS.md:1029-1036`) kural tablosudur, piyasa verisi değil |

Gömülü gizli değer bildirimi (yalnız yer): `CONNECTIVITY.md:484-490` yerleri doğru — denetçi
`docker-compose.scale-test.yml:78,81,112,113,159` satırlarında yalnız anahtar adlarını görerek
teyit etti (değer okunmadı). `ANOMALY.md:465-466` (takas servisinde belirteç önekinin günlüğe
yazılması) yalnız yer veriyor.

## Kapsam tablosu (VERIFY 8, 9)

| GOAL sorusu | Nerede | Durum |
|---|---|---|
| A1 Belirteç (uç nokta, istek, kimlik, ömür, yenileme, süre dolunca) | CONNECTIVITY §1.1–1.5 | Cevaplı; açık bağlantıda süre dolumu bilinmiyor + Matriks (soru 4) |
| A2 MQTT (sayı, yayın tipi, istemci kimliği, düşürme kuralı, yeniden bağlanma, oturum, QoS) | CONNECTIVITY §2.1–2.6 | Cevaplı; kuralın anahtarı bilinmiyor + Matriks (soru 1) |
| A3 Qapsule ayrı bağlanırsa; kullanıcı + servis aynı anda | CONNECTIVITY §3, §3.2 | Cevaplı + bilinmeyenler kime (soru 3, 7) |
| A4 Abonelik modeli, sınırlar | CONNECTIVITY §4 | Cevaplı; sınır bilinmiyor + Matriks (soru 5) |
| B1 Proto kataloğu | MESSAGES §1–2 | Cevaplı (mesaj → kullanım → ilk/güncelleme → alanlar) |
| B2 İlk durum/güncelleme, eksik alan, sıra, boşluk, yeniden eşitleme | MESSAGES §3 | Cevaplı + S1, S2 |
| B3 Zaman alanları, saat dilimi, açıklık durumları | MESSAGES §4 | Cevaplı + S4, S5 |
| B4 Sembol evreni, üst veri | MESSAGES §5 | Cevaplı; lot ve para birimi bilinmiyor + S10 |
| B5 Bar, REST ilişkisi | MESSAGES §6 | Cevaplı + S8 |
| B6 Lisans sınıfı; yetki denetiminin yeri | MESSAGES §7 | Cevaplı; bilinmeyenler §7.3 + kime |
| C1 Konular, yük, kalıcılık, geç katılan | NATS §1–3 | Cevaplı; üretim NATS'ı bilinmiyor + kime (§10) |
| C2 Geçit dönüşümleri | NATS §4 | Cevaplı |
| D1 Gösterge listesi, parametre, periyot, sembol, ısınma, yayın | INDICATORS T4a §1, T4b §1, T4c §1 | Cevaplı |
| D2 Abonelik, çapraz gösterge, isteğe bağlı parametre | INDICATORS T4a §2, T4b §3, T4c §3; NATS §5 | Cevaplı; "çapraz gösterge" anlamı Arkin'e soru |
| D3 Sayısal tanım farkları (en az 5) | INDICATORS Özet tablosu + T4a §3 (SMA, EMA, RSI, MACD, BB, ATR, Donchian, StdDev) | Cevaplı |
| D4 Performans ve ölçek | INDICATORS T4a §5, T4b §6, T4c §7 | Cevaplı (sınırlar yazılı değil diye işaretli) |
| E1 Türler, kural, eşik, pencere, yaşam döngüsü, güven, bastırma | ANOMALY §1–2 | Cevaplı |
| E2 Yük, konu, geçmiş kaydı | ANOMALY §3 | Cevaplı; dağıtım değerleri + kime (§8/1) |
| E3 Determinizm | ANOMALY §4.1 | Cevaplı |
| E4 Anlam türü, aile, dedektör örtüşmesi, kanıta açılabilirlik, mühür | ANOMALY §4.2, §5 | Cevaplı |
| F1 Kavram eşlemesi | INTEGRATION §3 | Cevaplı (karşılığı olmayanlar listeli) |
| F2 R6 seçenekleri (a)–(d) karşılaştırmalı | INTEGRATION §4 | Cevaplı; **(d2) deneme yolu "Bilinmiyor", kime sorulacağı yazılı değil** |
| F3 Tespit nerede | INTEGRATION §5 | Cevaplı |
| F4 R6 kapsam taslağı; Arkin'e / Matriks'e sorular | INTEGRATION §8; sorular belgelere dağılmış | Taslak var; birleşik soru listesi REPORT.md'ye bırakılmış, REPORT.md henüz yok |

| Arkin'in bilgisi | Hüküm | Kaynak var mı |
|---|---|---|
| 1 Belirteç sisteme özel | Kısmen (CONNECTIVITY §6, INTEGRATION §2) | Var (bkz. düzeltme #48) |
| 2 Yayın tipi başına bağlantı, ikinci bağlantı düşürür | Doğrulandı | Var |
| 3 İlk en geniş hal, sonra güncelleme | Kısmen | Var |
| 4 Açıklık + saat zaman damgasıyla | Kısmen | Var |
| 5 Gösterge/anomali, NATS | Kısmen | Var (NATS §9, INDICATORS üç bölüm, ANOMALY §6) |

## quantex durumu

`git status --porcelain` (denetim anında) ile `rounds/RI1/experiments/quantex_status_before.txt`
`diff` ile **aynı** (3 satır, hepsi round öncesinden). HEAD `f3d95aea`. Denetim sırasında
`quantex`'te yazma, derleme ya da betik yok.

## Karar verilmedi mi (ölçüt 6)

- INTEGRATION başlığı ve §4 "Öneri (karar değil)" diliyle yazılmış; kararın ana akışta
  verileceği açık. §5 ve §8 de "Öneri" etiketli. Karar dili görülmedi.
- Öneri gerekçeli: dört maddelik gerekçe (değişmezler, K44, çoklu bağlantı/lisans, anomali
  olgu değil) kaynak bölümlere bağlı; ön koşul ve geri çekilme yolu ((b) salt okuma) yazılı.
- Deneme yolu: (a), (b), (c) için ayrı madde; (d1) ve (d3) için yalnız tablo hücresi ("ayrı
  kimlik, kısa kayıt", "Quantex test ortamı"); **(d2) için "Bilinmiyor"** — eksik.

## Diğer gözlemler

1. **Arkin 2 için doğrudan belge kanıtı kullanılmamış:** `MD/MQTT/market.md:26` "aynı
   kullanıcıya ait token ile aynı adrese ikinci bağlantı ilkini sonlandırır" diyor (ayrıca
   `:166`). Bu, CONNECTIVITY soru 1'e (kuralın anahtarı) kısmi cevap: Matriks belgesine göre
   anahtar "token + adres". CONNECTIVITY §2.4 ve §6'ya eklenebilir.
2. **Zaman birimi çelişkisi Qapsule içinde açık kalıyor:** MESSAGES §4.1 işlem zamanını (AN'ye
   göre) gece yarısından bu yana ms/µs/ns sayıyor; Qapsule `docs/MATRIKS_PROTOCOL.md:375`
   aynı alanı "epoch µs [doğrulanmış]" diye yazıyor (`:381-386` çelişkiyi anıyor). MESSAGES bu
   eski belgeye atıf yapmıyor; R6 öncesi iki belge uzlaştırılmalı (INDICATORS:892 ise eski
   belgeye dayanıyor).
3. **Anomali motorunun gösterge girdisi büyük olasılıkla boş:** motor `quantex/indicator/data/#`
   dinliyor (ANOMALY:76, :209), fiyat motoru `quantex/price/indicator/update/<key>`'e yazıyor
   (INDICATORS:241-246). ANOMALY bu uyuşmazlığı anmıyor; §1'deki "yalnız `bb_depth_breakout`
   çalışabilir" yargısı da bu yüzden zayıflayabilir. Quantex ekibine sorulmalı.
4. Geçit kısma katmanlarında `ENTERPRISE` (0 ms) de var (`gateway/src/throttle/throttler.ts:44-49`);
   NATS:277 yalnız üç katmanı sayıyor. Yanlış değil, eksik.
5. `publishReason` başka iki serviste (`quantex-broadcast`, `q-demo`) gömülü proto metninde geçiyor;
   okuma yok, MESSAGES:134 hükmü geçerli.
6. INTEGRATION (b)'deki "`NATS.md` soru 2" atfı doğru satırı gösteriyor ama NATS §10 tablosu
   numarasız; numaralandırılması atfı kalıcı kılar.
7. VERIFY 10 (Qapsule `git diff --stat`) ve REPORT/STATUS/ROADMAP bu denetimin kapsamı dışında
   bırakıldı; `rounds/RI1/REPORT.md` denetim anında yok.

## Düzeltmeler


Ana akış, denetimden sonra (2026-11-19). Her düzeltme kaynağında yeniden okundu.

| # | Bulgu | Yapılan | Yer |
|---|---|---|---|
| 1 | `set_on_token_update` "hiçbir yerde çağrılmıyor" — anomali motoru çağırıyor | Metin üç gösterge servisi ile anomali motorunu ayırıyor; anomali geri çağrısının yalnız kopmuş akışları işaretlediği eklendi; sonuç (açık bağlantı eski belirteçle sürer) korunur | `docs/matriks/CONNECTIVITY.md` §1.4 |
| 2 | Kanıt satırı kayık | `nats.analytics-subscriber.ts:48-50` → `:51-53` (kaynakta `anomaly.BIST.*.data/outcome` satırları 51–52) | `docs/matriks/NATS.md` §9 |
| 3 | "25 dedektör kayıtlı" | "fabrikada 26 kayıt: 25 anomali dedektörü + `REGIME` (`:705`)" | `docs/matriks/ANOMALY.md` §0 |
| 4 | Aynı sayım | "25 anomali dedektörü + rejim" | `docs/matriks/INTEGRATION.md` §3 |
| 5 | "`hardwareId` her pod'da ayrıdır" | "varsayılan olarak pod adından, pod başına ayrı; ortak ortam dosyasında sabitlenmişse paylaşılabilir, bilinmiyor" | `docs/matriks/INTEGRATION.md` §1 |
| 6 | Bilgi 1 hükmünde kesin dil | "Varsayım: `sub` = `QTX-<hardwareId>` … tek bir kod yorumuna dayanıyor" eklendi | `docs/matriks/INTEGRATION.md` §2 |
| K2 | (d2) deneme yolu "Bilinmiyor" | Neden güvenli yol bilinmediği, kime sorulacağı ve olumlu cevapta dar deneme yolu yazıldı; (d1) ve (d3) için de metin içinde deneme yolu eklendi; tablo hücreleri güncellendi | `docs/matriks/INTEGRATION.md` §4 (d1–d3) |
| K3 | REPORT yok | `rounds/RI1/REPORT.md` yazıldı | — |
| G1 | Matriks belgesi bilgi 2'yi doğrudan söylüyor | Kural belgedeki ifadesiyle eklendi (anahtar: belirteç/kullanıcı); bilgi 2 hükmüne atıf | `CONNECTIVITY.md` §2.4 madde 0; `INTEGRATION.md` §2 |
| G2 | İşlem zaman damgası birimi Qapsule'ün eski belgesiyle çelişiyor | Çelişki notu eklendi; Matriks'e soru S4'te | `MESSAGES.md` Özet 6 |
| G3 | Anomali motorunun gösterge girdisi büyük olasılıkla boş (konu ağaçları eşleşmiyor) | Not ve kime sorulacağı eklendi | `ANOMALY.md` §1 "Gösterge girdisi" |

Koşulların üçü de karşılandı. Düzeltmelerden sonra sınır taraması (IP, URL, port, host adı,
gizli kalıp, proto sözdizimi, uzun kod bloğu, uzun sayısal kimlik) ana akışta yeniden koşuldu: 0 bulgu.
