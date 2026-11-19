# Quantex iç NATS yayını (RI1 / T3)

Kaynak: `quantex` deposu (salt okunur inceleme; HEAD `f3d95aea`, bkz. `rounds/RI1/experiments/MAP.md`).
Bu belgede `qs/` kısaltması `services/quantex-stream/` demektir. Kanıtlar `dosya:satır` biçimindedir.
Canlı bağlantı kurulmadı; her şey koddan ve dağıtım dosyalarından çıkarıldı. İç adresler
genelleştirildi (ör. "NATS adresi, yapılandırmadan: `NATS_URL`").

## 0. Kısa hüküm

- NATS **Core** kullanılıyor; yayıncıların hepsi düz `nc.publish` çağırıyor. Depoda `jetstream()`,
  akış (stream), tüketici, KV ya da Object Store tanımı **yok**. Sunucu `-js` bayrağıyla açılıyor
  ama hiçbir kod JetStream'e yazmıyor; dağıtım yorumlarındaki "NATS JetStream'e yayınlar" ifadesi
  kodla çelişiyor (§3).
- Yük **protobuf** (`quantex.streamx.Envelope`), iç şemaya **çevrilmiş**; Matriks protosu aynen
  geçirilmiyor. Tek istisna kullanıcı bazlı `matriks.u.<user_id>.*` konuları: orada Matriks'in
  baytları **aynen** taşınıyor, biçim NATS başlığında (§2).
- Fiyat ve miktar alanları **ölçekli tam sayı** (fiyat ×10000, miktar ×100, yüzde ×100).
- Sıra numarası (`seq`) ve dönem (`snapshot_seq`) yalnız ham piyasa konularında var; gösterge ve
  anomali zarflarında yok. Ham piyasada `seq` boşlukları **kayıp anlamına gelmiyor** (değişmeyen
  mesaj atlanırken de artıyor) (§2.3).
- Geç katılan tüketici ilk durumu NATS'tan değil, bir **istek/yanıt** konusundan
  (`streamx.symbol.refresh`, bağlayıcının bellekteki durumundan) ya da göstergelerde motorun
  abonelik yanıtındaki geçmişten alıyor (§3.3).
- C++ motorları sonuçlarını **iç Mosquitto** aracısına yazıyor; dört bağlayıcı oradan NATS'a
  taşıyor; NATS'tan gelen abonelik komutlarını da Mosquitto'ya geri iletiyor. Bu aracı Matriks'in
  MQTT'si değil (§6).
- NATS'ta hesap/izin tanımı depoda yok; geçit NATS'a kimliksiz bağlanıyor. İstemci yetkisi geçitte
  JWT + HMAC el sıkışması + kapsam (scope) bit kümesiyle denetleniyor (§7).

## 1. Konu adları ve hiyerarşisi

### 1.1 Ham piyasa verisi (matriks-adapter)

Kalıp `market.{exchange}.{marketType}.b{NN}.{SYMBOL}.{dataType}`; küresel olanlar
`market.{exchange}.global.{dataType}` (`qs/matriks-adapter/src/services/nats-publisher.service.ts:376-386`).
`b{NN}` sembol kodunun FNV-1a özetinin 32'ye kalanı, iki haneli (`:842-850`, `BUCKET_COUNT=32` `:17`;
ortak tanım `qs/shared/src/constants.ts:95-107`). `exchange` her zaman `BIST` (`:427`).

| Kalıp | Yayıncı | Yük (Envelope oneof) | Anlam |
|---|---|---|---|
| `market.BIST.STOCK.bNN.SYM.quote` | matriks-adapter | `stock_snapshot` (ilk) / `stock_delta` (sonra) | Matriks `mx/symbol/SYM@lvl2` ve `mx/derivative/...` fiyat kartı (`:871-873`, `:455-480`) |
| `market.BIST.VIOP.bNN.SYM.quote` | matriks-adapter | aynı | `mx/derivative/*` ya da `F_`/`O_` önekli sembol (`:854-866`) |
| `market.BIST.STOCK.bNN.SYM.trade` | matriks-adapter | `trade` | `mx/trade/SYM` (yalnız `DATA_TYPES` ortam değişkeninde açıksa; varsayılan yalnız `symbol`: `qs/matriks-adapter/src/config/env.ts:73`) |
| `market.BIST.STOCKDEPTH.bNN.SYM.depth` | matriks-adapter | `depth_snapshot` (her mesaj tam tablo) | `mx/depth/SYM` (`:485-487`, `:777-810`) |
| `market.BIST.global.timestamp` | matriks-adapter | yalnız zarf, tipli yük **yok** | `mx/timestamp` (`:886-888`, `:488-491`) |
| `market.BIST.global.news` / `.stats` | matriks-adapter | yalnız zarf, tipli yük yok | haber / sıralama (`:870-888`) |
| `streamx.symbol.refresh` (istek/yanıt) | matriks-adapter dinler, geçit sorar | istek JSON `{symbol}`; yanıt ikili Envelope ya da JSON hata | geç katılana ilk durum (§3.3; `:211-252`) |

### 1.2 Fiyat göstergesi, sinyal, formasyon (price-indicator-adapter)

Eşleme `qs/price-indicator-adapter/src/nats/nats.publisher.ts:77-137`; MQTT kaynakları `qs/price-indicator-adapter/src/mqtt/mqtt.subscriber.ts:9-21`.

| NATS kalıbı | MQTT kaynağı (iç Mosquitto) | Yük | Anlam |
|---|---|---|---|
| `price.BIST.{SYM}.data.{key}` | `quantex/price/indicator/update/{key}` | `indicator_update` | canlı gösterge değeri; `key` ör. `SYM:periyot:GÖSTERGE:param` |
| `price.reply.{clientId}.{key}` | `quantex/price/indicator/reply/{clientId}/{key}` | `indicator_snapshot` | abonelik yanıtı: geçmiş (`history_json`) |
| `price.signal.BIST.{SYM}.{signalId}` | `quantex/price/signal/data/...`, `.../signal/instance/...` | `signal_event` | sinyal tetiklendi |
| `price.signal.status.{instanceKey}` | `quantex/price/signal/status/...` | `signal_status_change` | sinyal örneği durum geçişi |
| `price.signal.state.{instanceKey}` | `quantex/price/signal/state/...` | `signal_instance_detail` | koşullara uzaklık / ilerleme |
| `price.signal.reply.{clientId}.{key}` | `quantex/price/signal/reply/...` | `signal_command_reply` | sinyal komutu yanıtı |
| `price.pattern.BIST.{SYM}.{instanceKey}` | `quantex/price/pattern/data/...` | `pattern_event` | formasyon |
| `price.engine.status`, `price.engine.heartbeat` | `quantex/engine/status`, `quantex/engine/heartbeat` | **boş zarf** (motorun JSON'u atılıyor) | yalnız "bir şey geldi" işareti (`qs/price-indicator-adapter/src/index.ts:185-199`) |
| `price.cmd.{clientId}` (dinlenir) | → `quantex/price/indicator/cmd` | `IndicatorCommand` | gösterge aboneliği (§5) |
| `price.signal.cmd.{clientId}` (dinlenir) | → `quantex/price/signal/cmd` | `SignalCommand` | sinyal aboneliği (§5) |

### 1.3 Akış (flow) ve derinlik (depth) göstergeleri

Fiyatla aynı yapı, önek `flow.` ve `depth.`; `signal.state` ve `pattern` yok, periyot ve bar
zamanı yok (tik/defter bazlı: `qs/shared/proto/flow.proto:19-33`, `qs/shared/proto/depth-indicator.proto:22-38`).
Eşleme `qs/flow-indicator-adapter/src/nats/nats.publisher.ts:90-122`,
`qs/depth-indicator-adapter/src/nats/nats.publisher.ts:90-122`.

| Kalıp | Yük |
|---|---|
| `flow.BIST.{SYM}.data.{key}` / `depth.BIST.{SYM}.data.{key}` | `flow_update` / `depth_indicator_update` |
| `flow.reply.{clientId}.{key}` / `depth.reply...` | `flow_snapshot` / `depth_indicator_snapshot` (geçmiş yok, yalnız onay: `flow.proto:35-36`) |
| `flow.signal.BIST.{SYM}.{signalId}`, `.signal.status.{k}`, `.signal.reply.{c}.{k}` | sinyal olayı, durum, yanıt |
| `flow.engine.status|heartbeat`, `depth.engine.status|heartbeat` | boş zarf |
| `flow.cmd.*`, `flow.signal.cmd.*`, `depth.cmd.*`, `depth.signal.cmd.*` (dinlenir) | `FlowCommand`, `FlowSignalCommand`, `DepthCommand`, `DepthSignalCommand` |

Not: üç bağlayıcı da ortak `quantex/engine/status|heartbeat` MQTT konusunu dinleyip kendi
önekine yeniden yayınlıyor (`qs/flow-indicator-adapter/src/mqtt/mqtt.subscriber.ts:16-17`); yani
aynı motor mesajı üç ayrı NATS konusuna kopyalanıyor ve içeriği düşürülüyor.

### 1.4 Anomali (anomaly-adapter)

`qs/anomaly-adapter/src/nats/nats.publisher.ts:83-95`; kaynaklar `qs/anomaly-adapter/src/mqtt/mqtt.subscriber.ts:9-14`.

| Kalıp | MQTT kaynağı | Yük |
|---|---|---|
| `anomaly.BIST.{SYM}.data` | `quantex/anomaly/data/{SYM}` | `anomaly_event` |
| `anomaly.BIST.{SYM}.outcome` | `quantex/anomaly/outcome/{SYM}` | `anomaly_outcome` (sonradan değerlendirme) |
| `anomaly.BIST.{SYM}.regime` | `quantex/anomaly/regime/{SYM}` | `regime_event` (`{SYM}` `_SESSION` olabilir: `qs/shared/proto/anomaly.proto:46`) |
| `anomaly.health.{engineId}` | `quantex/anomaly/health/{engineId}` | boş zarf (`qs/anomaly-adapter/src/index.ts:160-170`) |

Anomali bağlayıcısında komut dinleyicisi **yok**: anomali aboneliksiz, sürekli yayın.
"AI dispatch" biçimindeki anomali mesajları NATS'a hiç geçmiyor (`qs/anomaly-adapter/src/transform/anomaly.transform.ts:38-43`).

### 1.5 Kullanıcı bazlı konular (matriks-user-adapter)

| Kalıp | Yön | Yük |
|---|---|---|
| `matriks.u.{user_id}.timestamp` | yayın | Matriks `TimeMessage` baytları aynen |
| `matriks.u.{user_id}.alarm` | yayın | Matriks `Messenger` (kullanıcının `mx/user/<sub>/messenger` konusu) |
| `matriks.u.{user_id}.prime.symbol` / `.prime.agent` | yayın | Matriks `Event` |
| `matriks.u.{user_id}.news.{aimatriks|aikap|aiaa|aicomment}` | yayın | Matriks `News` |
| `matriks.u.{user_id}.pgc.{SYM}` | yayın | Matriks `Pgc` (para giriş-çıkış) |
| `matriks.u.{user_id}.symbol.{SYM}` | yayın | Matriks `Symbol` (`mx/symbol/SYM@lvl2`) |
| `matriks.control.u.{user_id}.symbol.{subscribe|unsubscribe}` | dinlenir | JSON `{symbol}`; sembol `^[A-Z0-9_]{2,32}$` |

Kanıt: `qs/matriks-user-adapter/src/services/natsPublisher.ts:1-11, 39, 88-107`;
`qs/matriks-user-adapter/src/services/topicPlan.ts:65-145, 185-197`;
`qs/matriks-user-adapter/src/services/liveQuoteControl.ts:2-7, 21, 31-56`.
Tüketicisi bu depoda değil: yorumlar "mqlaw" adlı başka bir sistemi gösteriyor (`topicPlan.ts:95-96`,
`liveQuoteControl` başlığı). Geçit bu konulara abone değil.

### 1.6 Diğer

- `insight.score.changed.{exchange}.{segment}.{target}`, `insight.regime.shift...`, `insight.trend.reversal...`,
  `insight.divergence.detected.{exchange}.{target}` — insight-adapter, `insight_event`
  (`qs/insight-adapter/src/nats/nats.publisher.ts:87-94`).
- `news.visual.{nlp_completed|ready|failed}` — quantex-news-visual, **JSON**
  (`services/quantex-news-visual/src/modules/nats/nats-publisher.ts:11-15, 127`); geçit `news.visual.>` dinliyor.
- sync-service NATS kullanmıyor (kodda NATS yok; `qs/k8s/sync-service-deploy.yml:77` "NATS kullanmaz"
  diyor, aynı dosyanın `:5` satırı "NATS üzerinden ... yayınlar" diyor: belge kendi içinde çelişik, kod esas).

## 2. Yük biçimi

### 2.1 Zarf

Bütün iç yayınlar (kullanıcı bazlı ve news.visual hariç) `quantex.streamx.Envelope` protobuf
ikilisi: `qs/shared/proto/envelope.proto:24-80`; kodlama `qs/shared/src/proto/index.ts:101-111`.
Derlenen proto `envelope.proto` ve kardeşleri (`qs/shared/package.json:19`). `qs/shared/proto/market.proto`
içinde eski, eksik bir `Envelope` kopyası var (yalnız alan 20-31), derlemeye **girmiyor**: okurken yanılmamak gerek.

Zarf alanları:

| Alan | Anlam | Kim dolduruyor |
|---|---|---|
| `exchange` | `BIST` | hepsi |
| `market_type` | `STOCK`, `VIOP`, `STOCKDEPTH`, `INDICATOR`, `SIGNAL`, `PATTERN`, `ANOMALY` | hepsi |
| `symbol_id`, `symbol_code` | sembol kodu (ikisi aynı) | ham piyasa; göstergede yalnız `symbol_code` |
| `data_type` | `quote`, `trade`, `depth`, `timestamp`, `indicator_update`, `signal_event`, `regime_transition`… | hepsi |
| `snapshot_seq` | dönem kimliği: o anahtar için ilk mesajdaki `Date.now()` | yalnız matriks-adapter (`nats-publisher.service.ts:414-416`) |
| `seq` | anahtar başına artan sayaç | yalnız matriks-adapter (`:418`) |
| `ts_event` | kaynak zamanı (ms) | Matriks `updateDate` ya da `timestamp`, yoksa `Date.now()` (`:893-905`); göstergede motorun `timestamp`'i; rejimde µs→ms (`anomaly.transform.ts:128-131`) |
| `ts_ingest` | bağlayıcıya varış (ms) | hepsi |
| `flags` | `SNAPSHOT=1`, `DELTA=2`, `HEARTBEAT=4` (`qs/shared/src/proto/index.ts:91-96`) | hepsi; HEARTBEAT kodda hiç kullanılmıyor |
| `payload` (oneof) | 20-46 arası tipli yük | — |

Kaynak/sürüm alanı **yok**: zarf hangi bağlayıcıdan geldiğini ve şema sürümünü taşımıyor
(proto başlıklarında yalnız "Version: 1.0.0" yorumu var). Motor kimliği tipli yükün içinde (`engine_id`).

### 2.2 Ham piyasa: Matriks protosu çevriliyor

matriks-adapter Matriks protosunu `protobufjs` ile açıp (`qs/matriks-adapter/src/services/protobuf.service.ts:112-118`,
`defaults: true`) alanları iç `StockSnapshot`/`StockDelta`/`Trade`/`DepthSnapshot` tiplerine eşliyor
(`nats-publisher.service.ts:501-605, 763-810`). Alan adları değişiyor (ör. `dayClose`→`close`,
`difference`→`change`, `priceMean`→`vwap`, `dailyVolume`→`amount`). Ölçek: `scalePrice` ×10000,
`scaleQty` ×100, `scalePct` ×100, yuvarlayarak (`qs/shared/src/proto/index.ts:591-636`;
`qs/shared/src/constants.ts:110-114`). Qapsule'de `f64` istiyorsak bölmek gerekir.

Gösterge, sinyal ve anomali yükleri motorların **JSON**'undan proto'ya çevriliyor; ama değer
kısmı yine JSON bayt olarak taşınıyor: `value_json`, `history_json`, `trigger_json`,
`contributors_json`, `axes_json` vb. (`qs/shared/proto/indicator.proto:15-39`; `qs/price-indicator-adapter/src/transform/price.transform.ts:28-99`).
Sinyal güveni 0-100 gelirse 0-1'e bölünüyor (`price.transform.ts:100-107`).

### 2.3 Sıra numarası ve boşluk algılama — dikkat

- `seq` anahtarı `{symbolCode}:{topicType}` (`:411`), ama `streamx.symbol.refresh` ile yeniden
  yayında anahtar `{symbolCode}:quote` (`:287`). Canlı fiyat kartı `topicType=symbol` ile geldiği
  için yeniden yayın **ayrı bir sayaç** kullanıyor; aynı NATS konusunda iki bağımsız `seq` dizisi
  (aynı `snapshot_seq` olmadan) karışabiliyor.
- `seq` zarf kurulurken artıyor (`:418`), değişiklik yoksa mesaj atlanıyor (`:366-368`). Yani
  `seq` boşluğu kayıp demek değil. Qapsule'ün `BindingState::stale` kuralı ("sıra boşluğu")
  bu `seq`'e doğrudan bağlanırsa yanlış eskime üretir.
- `snapshot_seq` bağlayıcı yeniden başlayınca değişir (bellekteki harita sıfırlanır: `:20`). Tüketici
  "dönem değişti → durumu baştan iste" diye kullanabilir; ama dönem değişince ilk mesajın SNAPSHOT
  olacağı da garanti: `stateCache` boş olduğu için (`:461-465`).
- Gösterge/anomali zarflarında `seq`/`snapshot_seq` hiç yok: kayıp algılanamaz.

### 2.4 Seyrek güncelleme (sparse delta) — risk

Fiyat kartında ilk mesaj tam `stock_snapshot`, sonrakiler yalnız değişen alanlarla `stock_delta`
(`nats-publisher.service.ts:449-480`, `:720-760`); bu Qapsule'ün `DataEvent::patch` kavramına uyuyor.
Ancak karşılaştırma, Matriks mesajının **tamamı** üzerinden yapılıyor: `defaults: true` ve
`toNumber(undefined)=0` (`:830-838`) yüzünden Matriks bir mesajda bir alanı göndermezse o alan 0
olarak okunuyor, delta'ya 0 yazılıyor ve önbellekteki durum 0 ile eziliyor (`:472-474`).
Varsayım: Matriks `@lvl2` her mesajda bütün alanları dolu gönderiyorsa sorun yok; Arkin'in 3.
bilgisi ("ilk seferde en geniş, sonra güncelleme") alan düzeyinde seyrek güncelleme ise bu
eşleme yanlış sıfır üretir. T2 (`docs/matriks/MESSAGES.md`) ile birlikte doğrulanmalı.

Ayrıca delta yalnız 21 "canlı" alanı karşılaştırıyor (`:733-755`); `open`, `limitUp`, `basePrice`
gibi alanlar değişse de delta'ya girmez, yalnız bir sonraki tam snapshot'ta görünür.

### 2.5 Küresel `timestamp` mesajı borsa açıklığını taşımıyor

Matriks `TimeMessage` alanları `timestamp`, `source`, `isBistPPHoliday`, `isBistPPOpen`
(`qs/matriks-adapter/src/proto/Timestamp.proto:7-11`). matriks-adapter bu mesaj için tipli yük
kurmuyor (`nats-publisher.service.ts:488-491`), yalnız `ts_event`'e zamanı koyuyor. Yani
**borsa açık/tatil bilgisi `market.BIST.global.timestamp` üzerinden geçmiyor**. Aynı bilgi
kullanıcı bazlı `matriks.u.{user_id}.timestamp` konusunda ham baytlarla var.

### 2.6 Kullanıcı bazlı konular: aynen geçirme

matriks-user-adapter gövdeyi çözmüyor; NATS başlıklarına `mx-topic` (özgün MQTT konusu),
`mx-format` (`proto:Symbol`, `proto:Messenger`, `json`, `text`…), `mx-endpoint`, `mx-ts-recv`
koyuyor (`qs/matriks-user-adapter/src/services/natsPublisher.ts:1-11, 154-166`). Sıra numarası yok.
Not: bu servisteki `envelope.ts` bir mesaj zarfı değil; Redis'ten okunan HMAC imzalı Matriks JWT
kaydı (`qs/matriks-user-adapter/src/services/envelope.ts:1-9, 19-27`).

## 3. Kalıcılık ve yeniden oynatma

### 3.1 JetStream yok (kodda)

- Bütün yayıncılar `nc.publish(subject, bytes)` ile NATS Core'a yazıyor
  (`nats-publisher.service.ts:390`; `qs/price-indicator-adapter/src/nats/nats.publisher.ts:69`;
  `qs/anomaly-adapter/src/nats/nats.publisher.ts` aynı yapı; `natsPublisher.ts:107`).
- Bütün tüketiciler düz `nc.subscribe` (`qs/gateway/src/nats/nats.subscriber.ts:167, 186`;
  `qs/gateway/src/nats/nats.analytics-subscriber.ts:91`).
- Depo genelinde `jetstream(`, `jetstreamManager`, `streams.add`, `consumers.add`, `kv(`
  araması `node_modules` dışında boş döndü.
- Sunucu `-js --store_dir /data` ile başlatılıyor (`qs/k8s/_legacy/base/nats.yaml:34-45`;
  `docker/docker-compose.quantex.yml:587-589`), legacy k8s'te depo `emptyDir` (`nats.yaml` sonu):
  JetStream açık ama boş.
- Çelişki: `qs/README.md:11-13` "NATS Core + JetStream", `qs/k8s/matriks-adapter-deploy.yml:5`,
  `indicator-adapter-deploy.yml:5`, `anomaly-adapter-deploy.yml:5`, `matriks-user-adapter-deploy.yml:92`
  "JetStream'e yayınlar" diyor; runbook'lar (`qs/runbooks/nats-failure.md:58, 102`) akışlardan
  söz ediyor. Kod esas: JetStream kullanılmıyor.
- Üretimdeki NATS sunucusu bu deponun dışında, ayrı bir altyapı yığınında
  (`docker/docker-compose.quantex.yml:581-584` yorumu). Orada akış tanımı olup olmadığı **bilinmiyor**.

Sonuç: kaçırılan mesaj geri gelmez; yeniden oynatma, geçmiş, "son değer" konusu yok.

### 3.2 İç MQTT tarafı

Bağlayıcılar Mosquitto'ya `clean: true`, QoS 0 ile bağlanıyor
(`qs/price-indicator-adapter/src/mqtt/mqtt.subscriber.ts:69-74, 137`; anomali `:47, 113`).
Mosquitto'da kalıcılık açık (`services/quantex_indicators_price_cpp/mosquitto/config/mosquitto.conf:8-9`)
ama QoS 0 + temiz oturumla bağlayıcı kopukken gelenler kaybolur. Motor kayıt mesajları
(`quantex/registry/engine/{tür}/{id}`) MQTT "retained" (`qs/price-indicator-adapter/src/nats/engine-registry.ts:11-13`).

### 3.3 Geç katılan tüketici ilk durumu nereden alıyor

1. **Ham fiyat**: `streamx.symbol.refresh` istek/yanıt. Geçit JSON `{symbol}` gönderir, 7 sn
   bekler (`qs/gateway/src/nats/nats.subscriber.ts:284-324`). matriks-adapter bellekteki son durumu
   (`stateCache`) tam `stock_snapshot` olarak hem yanıtlar **hem de** herkese açık
   `market.BIST.STOCK.bNN.SYM.quote` konusuna yeniden yayınlar (`nats-publisher.service.ts:296-320`).
   Sembol önbellekte yoksa Matriks'e **dinamik abone olur** ve 5 sn ilk veriyi bekler (`:263-283`).
   Yani bu istek salt okuma değildir: çalışan sistemin Matriks aboneliğini değiştirebilir ve bütün
   tüketicilere ek mesaj üretir. Yanıt her zaman `market_type=STOCK` (`:298`), VİOP için yanlış.
2. **Geçidin kendi önbelleği**: geçit her gelen zarfı LRU önbelleğe (10.000 anahtar, 1 saat)
   **olduğu gibi** koyuyor, birleştirmiyor (`qs/gateway/src/cache/snapshot.cache.ts:52-55, 87-97`;
   `qs/gateway/src/fanout/fanout.engine.ts:113`). Sonra istemciye "snapshot" bayrağıyla gönderiyor
   (`qs/gateway/src/encoder/frame.encoder.ts:80-84`). Son mesaj bir `stock_delta` ise istemci
   snapshot diye bir delta alır. Risk; kodda bunu önleyen bir birleştirme görmedim.
3. **Göstergeler**: abonelik komutu motora gider, motor geçmişi `indicator_snapshot.history_json`
   olarak `price.reply.{clientId}.{key}` konusuna döner (`indicator.proto:29-39`). Akış/derinlik
   motoru geçmiş göndermiyor, yalnız onay (`flow.proto:34-36`, `depth-indicator.proto:40-41`).
4. **Anomali**: ilk durum yok; geçmiş yalnız motorun veritabanında (T5 `docs/matriks/ANOMALY.md`).
5. Redis: matriks-adapter Redis'i yalnız Matriks belirteci için kullanıyor
   (`qs/matriks-adapter/src/index.ts:226-232`); piyasa durumu Redis'te tutulmuyor. sync-service
   yalnız sembol üst verisini PostgreSQL'e eşitliyor (`qs/sync-service/src/scheduler/cron.scheduler.ts:41`).

## 4. Bağlayıcının geçit olarak yaptığı dönüşümler

| Dönüşüm | Var mı | Kanıt |
|---|---|---|
| Alan düzeyinde birleştirme | Var, bağlayıcıda: sembol başına son durum tutulur, değişen alanlar delta olarak yayınlanır | `nats-publisher.service.ts:128, 455-480` |
| Değişmeyeni süzme | Var: hiç alan değişmediyse mesaj yayınlanmaz | `:366-368, 478` |
| Fiyat ölçeği | ×10000 / ×100 / ×100 tam sayı | `qs/shared/src/proto/index.ts:591-636` |
| Sembol adı | Matriks konusundaki sembol büyük harfe çevrilir; `@lvl2` atılır | `protobuf.service.ts:167-172` |
| Pazar sınıflaması | `F_`/`O_` öneki ve `mx/derivative` → `VIOP` | `nats-publisher.service.ts:854-866` |
| Zaman | `updateDate` metni ms'ye; yoksa `timestamp`; yoksa bağlayıcı saati | `:893-905` |
| Derinlik | her mesajda tam tablo, seviye numarası dizideki sıradan | `:777-810` |
| Zenginleştirme | Yok (sembol servisi yalnız abone listesi için: `qs/matriks-adapter/src/index.ts:171-218`) | — |
| Örnekleme / kısma | Bağlayıcıda yok; **geçitte** istemci katmanına göre (FREE/STANDARD/PREMIUM, varsayılan 500/100/10 ms) toplama ile | `qs/gateway/src/throttle/throttler.ts:42-49`; `qs/gateway/src/config/env.ts:56-59` (dosya başı yorumu 1000/500/100 diyor: yorum eski) |
| Alan süzme (preset) | Geçitte: `LAST_ONLY`, `TOB`, `DEFAULT`, `FULL` | `frame.encoder.ts:116-140` |

Kayıp alanlar: `timestamp` mesajının açık/tatil bayrakları (§2.5); haber ve istatistik
mesajlarının içeriği (tipli yük kurulmuyor, `:488-491`); motor durum JSON'u (§1.2).

Geçitin abone olduğu ham piyasa konuları yalnız `STOCK` ve `STOCKDEPTH`
(`qs/gateway/src/nats/nats.subscriber.ts:158-166`): `VIOP` konuları NATS'ta var ama geçitten
istemciye gitmiyor.

## 5. Gösterge aboneliği: komut yolu

Arkin'in "gösterge ve çapraz gösterge aboneliğiyle dinleme" ifadesinin NATS tarafı:

1. İstemci geçide WebSocket üzerinden JSON `{symbol, period, indicator, params}` gönderir;
   geçit kapsamı denetler (`BIST:INDICATOR`) ve sınırı uygular
   (`qs/gateway/src/subscription/analytics-subscribe.manager.ts:38-47, 160-170`), sonra
   `price.cmd.{clientId}` konusuna protobuf `IndicatorCommand` yayınlar
   (`qs/gateway/src/server/gateway.server.ts:607-623`; `qs/gateway/src/nats/nats.cmd-publisher.ts:84-110`).
   Gövde alanları: `action` (`subscribe`/`unsubscribe`), `symbol`, `period`, `indicator`,
   `params_json` (JSON bayt, boş olabilir), `reply_to` (MQTT yanıt konusu
   `quantex/price/indicator/reply/{clientId}/{SYM}:{period}:{IND}[:param…]`), `client_id`
   (`qs/shared/proto/indicator.proto:52-60`).
2. "Çapraz/iç içe gösterge": `indicator` alanı bileşik tanım taşıyabilir, ör. `SMA[EMA[RSI:14]:20]:20`;
   o zaman `params_json` boş (`indicator.proto:43-50`; `qs/price-indicator-adapter/src/nats/nats.cmd-listener.ts:10-11`).
   Kodda "çapraz gösterge" adıyla ayrı bir konu ya da mesaj yok; bileşik tanım bu mekanizmayla
   ifade ediliyor. Varsayım: Arkin'in "çapraz gösterge"si bu iç içe tanım ya da sinyal
   tanımlarındaki kesişim koşullarıdır (`signal_type` örneği `macd_crossover`, `indicator.proto:70`).
   Kesin anlamı Arkin'e sorulmalı.
3. price-indicator-adapter `price.cmd.*` dinler, protobuf'u çözer, JSON'a çevirip iç Mosquitto'da
   **motor türüne genel** `quantex/price/indicator/cmd` konusuna QoS 0 ile yazar
   (`nats.cmd-listener.ts:30-36, 83-129`; `mqtt.subscriber.ts:149-160`).
4. Bütün fiyat motoru kopyaları bu konuyu dinler; Redis'te "talep" (claim) kazanan tek kopya işi
   üstlenir (`engine-registry.ts:11-19`; motor tarafı `services/quantex_indicators_price_cpp/src/engine/calculation_registry.cpp:114-133`, ayrıntı T4).
   `engine-registry` artık yönlendirme yapmıyor, yalnız gözlem için motor listesini tutuyor.
5. Motor ilk yanıtı (geçmiş) `reply_to` konusuna, canlı değerleri
   `quantex/price/indicator/update/{key}` konusuna yazar
   (`services/quantex_indicators_price_cpp/src/engine/calculation_registry.hpp:413`); bağlayıcı bunları
   `price.reply.{clientId}.{key}` ve `price.BIST.{SYM}.data.{key}` olarak NATS'a taşır.
6. Kopya ölürse diye geçit her 30 sn bütün etkin abonelikleri `__reassert__` istemci kimliğiyle
   yeniden gönderir; bağlayıcı bunu `reassert: true` işaretiyle, yanıt konusu olmadan iletir
   (`qs/gateway/src/subscription/price-reassert.manager.ts:38-45`; `nats.cmd-publisher.ts:48, 74-76`;
   `nats.cmd-listener.ts:105-118`). Yani abonelik **yumuşak durum**: yenilenmezse motor zaman aşımıyla
   bırakabilir (Varsayım; motor tarafı T4'te).

Sinyal aboneliği aynı yoldan: `price.signal.cmd.{clientId}` → `SignalCommand` (`action`:
`signal_subscribe|signal_unsubscribe|signal_pause|signal_resume`, `signal_id`, `instance_key`,
`params_json` içine `symbol`/`period` katılır, `user_id`, `delivery_json`, `reply_to`, `client_id`)
→ `quantex/price/signal/cmd` (`indicator.proto:94-103`; `nats.cmd-publisher.ts:144-175`;
`nats.signal-cmd-listener.ts:25-30, 78-133`). Akış ve derinlik için `flow.cmd.*` → `quantex/flow/indicator/cmd`,
`flow.signal.cmd.*` → `quantex/flow/signal/cmd`, `depth.cmd.*` → `quantex/depth/indicator/cmd`,
`depth.signal.cmd.*` → `quantex/depth/signal/cmd` (`qs/flow-indicator-adapter/src/nats/nats.cmd-listener.ts:30-34`;
`nats.signal-cmd-listener.ts:27-31`; depth karşılıkları `:30-35`, `:27-30`). Akış/derinlik
komutunda periyot yok (`flow.proto:50-57`).

Yanıt NATS istek/yanıtı değil: komut "ateşle ve unut" yayınlanıyor, sonuç ayrı bir konuya
(`*.reply.{clientId}.*`) asenkron geliyor. `clientId` konunun bir parçası; Qapsule kendi
`clientId`'siyle komut gönderirse yanıtını `price.reply.{clientId}.>` ile dinleyebilir.

Kod içi yorum çelişkisi: `nats.cmd-publisher.ts:7-10` `indicator.cmd.{clientId}` ve
`quantex/indicator/cmd` diyor; kod `price.cmd.{clientId}` ve `quantex/price/indicator/cmd` kullanıyor (`:105-108`; `nats.cmd-listener.ts:36`).

## 6. Ara MQTT aracısı

C++ motorlarının iki MQTT bağlantısı var: Matriks girişi (`MATRIKS_MQTT_HOST`) ve çıkış için
"istemci" aracısı (`CLIENT_MQTT_HOST`) (`services/quantex_indicators_price_cpp/src/config/config.hpp:189-196`).
Çıkış aracısı iç Mosquitto; bağlayıcıların `MQTT_BROKER_URL`'si de aynı iç aracıyı gösteriyor
(`docker/docker-compose.quantex.yml:771, 809, 847, 885, 924, 1012`; bağlayıcı günlükleri "Connected to Mosquitto"
diyor: `qs/price-indicator-adapter/src/mqtt/mqtt.subscriber.ts:85`). Konu ağacı `quantex/...`, Matriks'in
`mx/...` ağacı değil. Sonuç: **ara aracı Matriks'in MQTT'si değil, Quantex'in kendi Mosquitto'su.**

```
 Matriks MQTT (mx/...)                                 Matriks MQTT (kullanıcı JWT'siyle)
   │            │                                              │
   │            └──────────────┐                               ▼
   ▼                           ▼                     matriks-user-adapter
 matriks-adapter        C++ motorlar (price/flow/          │ aynen bayt + başlık
 (çöz, ölçekle,         depth, anomaly) ── kendi           ▼
  delta üret)           Matriks bağlantıları        NATS matriks.u.<id>.*  ──▶ mqlaw (depo dışı)
   │                           │  ▲                 NATS matriks.control.u.<id>.symbol.* ◀── mqlaw
   │                           │  │ JSON komut
   │                     yazar ▼  │ quantex/{tür}/{indicator|signal}/cmd
   │                  ┌─────────────────────┐
   │                  │ iç Mosquitto        │  quantex/{price|flow|depth}/..., quantex/anomaly/...,
   │                  │ (QoS 0)             │  quantex/engine/..., quantex/registry/engine/...
   │                  └─────────────────────┘
   │                     │ okur        ▲ yazar
   │                     ▼             │
   │        price/flow/depth-indicator-adapter, anomaly-adapter
   │        (JSON → Envelope protobuf)  ▲ NATS {price|flow|depth}[.signal].cmd.<clientId>
   ▼                     ▼              │
 ┌───────────────────────────────────────────────┐
 │ NATS Core  market.*  price.*  flow.*  depth.* │
 │            anomaly.*  insight.*  news.visual.*│
 └───────────────────────────────────────────────┘
        │ abone (kuyruk grupları)     ▲ komut yayını, streamx.symbol.refresh isteği
        ▼                             │
     gateway (WebSocket, JWT+HMAC, kapsam, katman kısması, preset) ──▶ istemciler
```

Kayda değer: Mosquitto ACL'si legacy dağıtımda (`qs/k8s/_legacy/base/mosquitto.yaml:58-114`) bağlayıcılara
yalnız motora özgü `quantex/engine/+/indicator/cmd` yazma izni veriyor; bugünkü kod genel
`quantex/{tür}/indicator/cmd` konusuna yazıyor. Legacy ACL ile bugünkü kod uyumsuz; güncel
Mosquitto yapılandırması bu depoda yok (yerel örnek `allow_anonymous true`:
`services/quantex_indicators_price_cpp/mosquitto/config/mosquitto.conf:1-6`).

## 7. Yetki

**NATS tarafı.** Depoda NATS sunucu yapılandırma dosyası, hesap, kullanıcı, `.creds` ya da
konu izni tanımı yok (dosya araması boş). Sunucu komut satırında kimlik seçeneği yok
(`nats.yaml:34-45`; `docker-compose.quantex.yml:589`). Bağlayıcılar `NATS_USER`/`NATS_PASSWORD`
geçiriyor (`nats-publisher.service.ts:153-160`; `qs/price-indicator-adapter/src/nats/nats.publisher.ts:24-32`;
`qs/matriks-user-adapter/src/services/natsPublisher.ts:47-54`), geçit hiç kimlik geçirmiyor
(`qs/gateway/src/nats/nats.subscriber.ts:72-79`). Geçit kimliksiz çalışabildiğine göre üretim
NATS'ı ya kimliksiz ya da geçit ayrı bir yolla bağlanıyor: **bilinmiyor**; dağıtım yorumu
"NATS auth gerekirse buraya eklenecek" diyor (`qs/k8s/indicator-adapter-deploy.yml:71`).
Sonuç: NATS'a erişebilen her süreç her konuyu okuyabilir ve `*.cmd.*` ile motorlara iş
yaptırabilir, `matriks.control.u.*` ile başka kullanıcının Matriks bağlantısına sembol ekletebilir.
Kullanıcı bazlı `matriks.u.<id>.*` konularında konu düzeyinde ayrım yok.

**Geçit (istemci) tarafı.** İstemci el sıkışmada `connection_key`, `timestamp`, HMAC-SHA256 imzası
ve JWT gönderiyor (`qs/shared/proto/market.proto:271-276`); geçit HMAC'i ve JWT'yi (`issuer`, `audience`)
doğruluyor, JWT'den `tier` ve `scopes` alıyor (`qs/gateway/src/auth/auth.service.ts:52-131`). Kapsamlar
bit kümesi: `BIST:STOCK`, `BIST:STOCKDEPTH`, `BIST:FUTURE`, `BIST:INDEX`, `BIST:INDICATOR`,
`BIST:ANOMALY`, `BIST:SIGNAL`, `BIST:FLOW`, `BIST:DEPTH`, `QUANTEX:NEWS_VISUAL`… (`auth.service.ts:19-34`);
abonelikte denetleniyor (`qs/gateway/src/subscription/subscribe.manager.ts:135-150`;
`analytics-subscribe.manager.ts:160-166`). Abonelik sayısı katmana göre sınırlı. JWT gizli anahtarı
ve HMAC anahtarı ortamdan (`qs/gateway/src/config/env.ts:33-39`), değer okunmadı.
Bu, Matriks lisans sınıfı (K33) denetimi değil; Quantex'in kendi ürün kapsamı.

## 8. Qapsule için çıkarımlar (R6 öncesi)

- NATS'tan tüketmek salt dinleyiciyse yan etkisiz: Core yayınında yeni abone başkasından mesaj
  çalmaz. Ancak geçidin kuyruk grubu adlarını (`gateway-bucket-*`, `gateway-global-*`, `gw-analytics-*`:
  `nats.subscriber.ts:143-145, 183`; `nats.analytics-subscriber.ts:86`) **kullanmamalı**, yoksa
  geçitin mesajlarını bölüşür.
- `streamx.symbol.refresh` ve `*.cmd.*` çağrıları salt okuma değil (Matriks aboneliği ekler,
  motora iş yükler, herkese ek mesaj üretir). Arkin'in onayı olmadan kullanılmamalı.
- Ham fiyat konusu bar değil, fiyat kartı (son fiyat, hacim, gün içi yüksek/düşük); Qapsule'ün
  `DataEvent::bar/patch`'ine bar üretimi Qapsule tarafında ya da gösterge motorundan gelir.
- `BindingState::stale` için `seq` boşluğuna güvenilemez (§2.3); `snapshot_seq` değişimi ve
  bağlantı kopması güvenilir eskime işaretleri. Kopukluktan sonra yeniden oynatma olmadığı için
  durum yeniden istenmeli (refresh ya da REST: `docs/DATA_ACCESS.md`).
- Fiyatlar ölçekli tam sayı; Qapsule `f64`'e bölerek almalı (fiyat /10000).
- Borsa açık/kapalı bilgisi iç ham yayında yok (§2.5).

## 9. Arkin'in 5. bilgisiyle karşılaştırma (NATS yanı)

> "Gösterge servislerine gösterge ve çapraz gösterge aboneliğiyle dinleme yapılabilir; anomali
> motoru piyasa anomalilerini yakalar; hepsi NATS üzerinden yayınlar."

- **Gösterge aboneliği NATS üzerinden: doğrulandı.** `price|flow|depth.cmd.{clientId}` ile abone
  olunuyor, sonuçlar `{tür}.BIST.{SYM}.data.{key}` ve `{tür}.reply.{clientId}.{key}` konularından
  geliyor (§5). Ama motorlar NATS'a **doğrudan** yayınlamıyor: araya iç Mosquitto ve bağlayıcı
  giriyor (§6). Komut da NATS'tan Mosquitto'ya köprüleniyor.
- **Çapraz gösterge: kısmen.** Kodda ayrı bir "çapraz gösterge" aboneliği yok; iç içe/bileşik
  gösterge tanımı (`SMA[EMA[RSI:14]:20]:20`) aynı komutla isteniyor. Sinyal aboneliği ayrı bir
  komut ailesi (`*.signal.cmd.*`). Arkin'in kastettiği hangisi, sorulmalı.
- **Anomali NATS'ta yayınlanıyor: doğrulandı**, ama abonelik **yok**: anomali bağlayıcısının
  komut dinleyicisi yok, bütün semboller için sürekli yayın (`anomaly.BIST.{SYM}.data|outcome|regime`).
  Geçit `regime` ve `health` konularına abone değil (`nats.analytics-subscriber.ts:51-53`).
- **"Hepsi NATS üzerinden": büyük ölçüde doğru**, ama kalıcılık yok (JetStream kullanılmıyor),
  gösterge/anomali zarflarında sıra numarası yok, motor durum içeriği NATS'a geçmiyor.

## 10. Açık sorular

| Soru | Kime |
|---|---|
| Üretim NATS'ında (depo dışı altyapı yığını) hesap/izin ve JetStream akışı tanımlı mı? Geçit nasıl kimliksiz bağlanıyor? | Quantex altyapı sahibi (Arkin üzerinden) |
| Qapsule'e ayrı bir NATS kullanıcısı ve yalnız okuma izni (`market.>`, `price.BIST.>`…) verilebilir mi? | Quantex altyapı sahibi |
| Matriks `@lvl2` mesajları her seferinde bütün alanları mı taşıyor, yoksa seyrek mi? (seyrekse §2.4 sıfır yazma hatası) | T2 bulgusu + Matriks belgeleri / Arkin |
| "Çapraz gösterge" neyi kastediyor: iç içe gösterge mi, kesişim sinyali mi, iki sembol arası mı? | Arkin |
| Geçidin önbellekten "snapshot" diye delta gönderebilmesi (§3.3/2) bilinen bir durum mu? | Quantex geçit geliştiricisi |
| `streamx.symbol.refresh` yanıtının ayrı `seq` sayacı ve sabit `STOCK` pazar tipi (§2.3, §3.3) bilinçli mi? | Quantex matriks-adapter geliştiricisi |
| `matriks.u.*` konularının tüketicisi "mqlaw" kim, Qapsule bu kullanıcı bazlı yolu kullanabilir mi (kullanıcının kendi Matriks yetkisi = K33 lisans uyumu)? | Arkin |
| Güncel Mosquitto ACL'si (legacy ACL bugünkü komut konularıyla uyumsuz) | Quantex altyapı sahibi |
