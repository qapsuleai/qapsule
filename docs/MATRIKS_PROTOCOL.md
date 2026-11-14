# Matriks Veri Protokolü (eski prototipten çıkarılan bilgi)

Durum: R0 / T4. Kaynak: eski Q-Charts prototipi. Canlı sistemde yeniden doğrulanmadı.

> **Gizli bilgi yok.** Bu belge hiçbir token, kullanıcı adı, parola veya anahtar içermez.
> Kimlik bilgisi yalnızca ortam değişkeninden gelir (eski kodda `MATRIKS_TOKEN`; yeni adı
> R6'da belirlenir) ve hiçbir zaman commit edilmez. Eski depodaki `ts/.env` dosyası
> okunmadı ve okunmamalıdır.

## Kaynak kısaltmaları ve güven etiketi

| Kısaltma | Yol |
|----------|-----|
| `SVC` | `../Services/quantex-charts-core` |
| `DOC` | `../Docs/q-charts` |

Her bilgi şu etiketlerden birini taşır:

- **[doğrulanmış]** Çalışan kodda var ve eski `CHANGELOG.md` gerçek Matriks altyapısına karşı
  entegrasyon testlerinin geçtiğini kaydediyor (`SVC/CHANGELOG.md`: "All 20 integration
  tests (12 REST + 8 MQTT) pass").
- **[kod]** Kodda var, ama gerçek sunucuya karşı ayrıca doğrulandığına dair kayıt yok.
- **[belge]** Yalnızca tasarım belgesinde geçiyor; kodla desteklenmiyor.
- **[çıkarım]** Bizim yorumumuz.

## 1. Genel resim

```
1. GET  disco-v2.json                 (kimlik doğrulamasız)  → REST ve MQTT broker URL'leri
2. GET  /dumrul/v2/topach             (Authorization: jwt …) → lisansa göre erişilebilir konular
3. REST /dumrul/v1/tick/bar.gz        (Authorization: jwt …) → temel periyot barları (JSON)
   REST /dumrul/v1/tick/derived-bar   (Authorization: jwt …) → türetilmiş periyot barları (JSON)
4. MQTT 3.1 over WebSocket  /trade    (username=JWT, password=<token>) → TradeMessage (protobuf)
   MQTT 3.1 over WebSocket  /market   (aynı)                           → SymbolMessage, TimeMessage
```

Kaynak: `SVC/ts/src/providers/matriks/{auth,discovery,history,realtime}.ts` **[doğrulanmış]**.

## 2. Ortamlar ve host desenleri

| Ortam | REST taban | Discovery | MQTT (WSS) |
|-------|-----------|-----------|------------|
| prod | `https://api.matriksdata.com` | `https://api.matriksdata.com/disco-v2.json` | `wss://rt.radix.matriksdata.com:443/{trade,market}` (Discovery yoksa yedek) |
| test | kodda boş dize (`""`), `baseUrl` ile verilmesi gerekiyor | boş dize | boş dize |

- prod değerleri: `SVC/ts/src/providers/matriks/auth.ts` (`BASE_URLS`, `DISCOVERY_URLS`),
  `discovery.ts` (`DEFAULT_*_WSS_PROD`) **[doğrulanmış]**.
- Test ortamı için belgelerde geçen host desenleri: REST `apitest.matriksdata.com`
  (`auth.ts` yorum örneği), MQTT gerçek zamanlı `rttest.radix.matriksdata.com`, gecikmeli
  `dltest.radix.matriksdata.com` (`DOC/analysis/matriks-mqtt-integration.md`) **[belge]**.
  Prod için gecikmeli (`dl`) karşılığı Discovery yanıtından gelir; kodda sabit yok.
- Varsayılan ortam kodda `"test"` (`auth.ts`). Test URL'leri boş olduğundan pratikte
  `environment: "prod"` veya açık `baseUrl` gerekir **[çıkarım]**.

## 3. Kimlik doğrulama

- Biçim: **JWT**. Kimlik bilgisi tek bir token; kullanıcı adı/parola akışı kodda yok.
- Token'ın nereden alındığı kodda yok: "Currently supports static JWT tokens. Future versions
  will add CRM-based dynamic token generation and automatic refresh."
  (`SVC/ts/src/providers/matriks/auth.ts`) **[kod]**. Token alma uç noktası **bilinmiyor**;
  R6'da Matriks'ten öğrenilmeli.
- REST başlığı: `Authorization: jwt <token>` (küçük harf `jwt`, `Bearer` değil)
  (`auth.ts` `getHeaders`) **[doğrulanmış]**.
- MQTT: kullanıcı adı sabit dize `JWT`, parola token'ın kendisi. "Token-as-username değil"
  (`realtime.ts` `buildMqttOptions`; `SVC/CHANGELOG.md` "MQTT Protocol Details (Resolved)")
  **[doğrulanmış]**.
- Süre kontrolü: istemci JWT gövdesini imza doğrulamadan base64url çözüp `exp` (Unix
  saniye) alanına bakar; süresi geçmişse hata `QC-3010` (`auth.ts`) **[kod]**.
- JWT gövdesinde `licenses` (ör. `PD2`, `AKDE`) ve `symbols` alanları olduğu belgede
  yazıyor (`DOC/analysis/matriks-mqtt-integration.md` "JWT Token Structure") **[belge]**.
- Discovery çağrısı kimlik doğrulamasız yapılıyor (yalnızca `Accept: application/json`)
  (`discovery.ts` `fetchDiscovery`) **[doğrulanmış]**.

## 4. Discovery (`disco-v2.json`)

Kaynak: `SVC/ts/src/providers/matriks/discovery.ts`, `types.ts` (`MatriksDiscoveryResponse`).

Yanıt şekli (yalnızca kullanılan kısım) **[doğrulanmış]**:

```jsonc
{
  "rest": {
    "historicTick": { "bar": { "url": "…" }, "derivedBar": { "url": "…" } },
    "metaData":     { "symbols": { "url": "…" } }
  },
  "mqtt": {
    "mx/trade":  { "qos": { "rt": "…", "dl": "…", "wss": { "rt": "…", "dl": "…" }, "tcp": { "rt": "…", "dl": "…" } } },
    "mx/symbol": { "qos": { … } },
    "mx/event/prime/symbol": { … }
    // anahtarlar konu önekleridir
  }
}
```

- `mqtt["mx/trade"].qos.wss.rt` → trade broker URL'si; `mqtt["mx/symbol"].qos.wss.rt` →
  market broker URL'si. `tcp.rt` de okunur ama kullanılmaz.
- Burada `qos` **MQTT QoS değildir**; `rt` (gerçek zamanlı) / `dl` (gecikmeli) hizmet
  kalitesi anlamındadır **[kod]**.
- Anahtar eşleşmesi **tam** yapılmalı: önek eşleşmesi `mx/event/prime/symbol` gibi
  anahtarları yanlışlıkla `/news` broker'ına bağlıyordu (kod yorumu) **[kod]**.
- Discovery başarısızsa sabit prod URL'lerine düşülür **[kod]**.
- `rest.*.url` alanları okunmuyor; REST yolları kodda sabit (§6) **[kod]**.
  `metaData.symbols` sembol listesi uç noktasıdır; içeriği kodda kullanılmadı **[çıkarım]**.

## 5. Topach (`/dumrul/v2/topach`)

Kaynak: `discovery.ts` `fetchTopach`, `types.ts` (`MatriksTopachResponse`) **[doğrulanmış]**.

- `GET {baseUrl}/dumrul/v2/topach`, başlık `Authorization: jwt <token>`.
- Yanıt lisansa bağlıdır: yalnızca kullanıcının erişebildiği konular döner.
- Şekil:

```jsonc
{
  "mqtt": {
    "trade":     { "topic": "mx/trade/%s@lvl2", "qos": "rt", "prefix": "mx/trade" },
    "timestamp": { "topic": "…", "qos": "rt", "prefix": "…" },
    "symbol": {               // iç içe: alt tipler (S, V, F, …)
      "S": { "topic": "mx/symbol/%s", "qos": "rt", "prefix": "mx/symbol" }
    }
  }
}
```

- `topic` içinde sembol yer tutucusu `%s`; `buildTopic(pattern, symbol)` ilk `%s`'i
  değiştirir. Örnek: `mx/trade/%s` + `SYMBOL` → `mx/trade/SYMBOL`.
- Bilinen anahtarlar: `trade*`, `tradeex`, `timestamp`, `depth` (düz); `symbol`/`market`,
  `derivative` (iç içe). Alt tip harflerinin (`S`, `V`, `F`) anlamı belgelenmemiş **[kod]**.
- Hata kodları: `QC-3060` (HTTP hata durumu), `QC-3061` (ağ hatası).
- Topach sonucu bağlantıda alınıp saklanıyor ama abonelikte **kullanılmıyor** (§8) **[kod]**.

## 6. REST: tarihsel barlar

Kaynak: `SVC/ts/src/providers/matriks/history.ts`, `timeframe-map.ts`, `types.ts`.

### Uç noktalar **[doğrulanmış]**

| Ad | Yol | Periyotlar |
|----|-----|-----------|
| temel bar (`temel-bar`) | `GET {baseUrl}/dumrul/v1/tick/bar.gz` | `1min`, `5min`, `1hour`, `1day` |
| türetilmiş bar (`turetilmis-bar`) | `GET {baseUrl}/dumrul/v1/tick/derived-bar` | `15min`, `30min`, `240min`, `weekly`, `monthly` (kodda eşlenenler) |

`timeframe-map.ts` başlık yorumu türetilmiş uçta ayrıca `10min`, `120min`, `yearly`
olduğunu söyler; bunlar eşlenmemiş ve test edilmemiş **[kod yorumu]**.
`history.ts` yorumunda `4hour` geçer ama gönderilen değer `240min`'dir.

### Periyot eşlemesi

| Eski `Timeframe` | `period` | Uç | İstek başına en çok bar |
|------------------|----------|----|-------------------------|
| M1 | `1min` | temel | 1440 |
| M5 | `5min` | temel | 1000 |
| M15 | `15min` | türetilmiş | 1000 |
| M30 | `30min` | türetilmiş | 1000 |
| H1 | `1hour` | temel | 1000 |
| H4 | `240min` | türetilmiş | 1000 |
| D1 | `1day` | temel | 1000 |
| W1 | `weekly` | türetilmiş | 1000 |
| MN1 | `monthly` | türetilmiş | 1000 |

Dokuz periyodun hepsi entegrasyon testinde hata vermeden çağrıldı **[doğrulanmış]**.
Üst sınırlar (`maxBars`) kodda sabit; sunucunun gerçekten uyguladığı sınır belgelenmemiş **[kod]**.

### Sorgu parametreleri **[doğrulanmış]**

| Parametre | Biçim | Not |
|-----------|-------|-----|
| `symbol` | `SYMBOL` | BIST kodu, ek yok (ör. `GARAN`, `THYAO`) |
| `period` | yukarıdaki tablo | |
| `start` | `YYYY-MM-DD` | tarih aralığı kipinde |
| `end` | `YYYY-MM-DD` | her iki kipte; verilmezse bugün |
| `count` | tamsayı | sayı kipinde; `end` ile birlikte "`end`'den geriye N bar" |

- Başlıklar: `Authorization: jwt <token>`, `Accept: application/json`,
  `Accept-Encoding: gzip`. Adı `.gz` olsa da yanıt JSON olarak çözülüyor (gzip taşıma
  sıkıştırması) **[doğrulanmış]**.
- Sayfalama: "daha eski veri" için `count` + `end = before` tarihi kullanılır. `end` gün
  çözünürlüğünde olduğundan gün içi periyotlarda aynı günün barları tekrar gelebilir;
  tekilleştirme istemciye kalır **[çıkarım]**.
- Hafta sonu aralığı boş dizi döner (`integration.test.ts` "handles empty result") **[doğrulanmış]**.
- Hata kodları: `QC-3020` ağ, `QC-3021` HTTP durum, `QC-3022` JSON çözme, `QC-3023` dizi değil.

### Yanıt biçimi **[doğrulanmış]**

JSON dizi; her öğe (`MatriksBarResponse`, `types.ts`):

| Alan | Tip | Anlam |
|------|-----|-------|
| `date` | string `YYYY-MM-DD` | bar tarihi |
| `time` | number | **Unix epoch milisaniye** |
| `open`, `high`, `low`, `close` | number | fiyat |
| `volume` | number | işlem hacmi, **TL** |
| `totalQuantity` | number | işlem adedi, **lot** |
| `weightedAverage` | number | ağırlıklı ortalama fiyat (VWAP) |

- Eski kod `time`'ı saniyeye çevirir (`floor(time/1000)`) ve diziyi artan zamana göre
  yeniden sıralar; sunucu sıralamasına güvenmez **[kod]**.
- Hacim olarak varsayılan `totalQuantity` (lot) seçilmiş; `volume` (TL) seçilebilir **[kod]**.
- `time`'ın barın başlangıcını mı bitişini mi gösterdiği belgelenmemiş **[bilinmiyor]**.
- `date`'in hangi saat dilimine göre olduğu belgelenmemiş; BIST için İstanbul saati
  olması muhtemel **[çıkarım]**.

## 7. MQTT bağlantı kuralları

Kaynak: `SVC/ts/src/providers/matriks/realtime.ts` (`buildMqttOptions`), `SVC/CHANGELOG.md`.
Eski kod bu ayarları Matriks'in çalışan bir referans istemcisinden (`saska-client.js`,
depoda yok) aldığını yazıyor.

| Kural | Değer | Etiket |
|-------|-------|--------|
| Protokol | **MQTT 3.1** (`protocolVersion: 3`, `protocolId: "MQIsdp"`); 3.1.1/5 değil | [doğrulanmış] |
| Taşıma | WebSocket, TLS, port 443; WS alt protokolü `mqttv3.1` | [doğrulanmış] |
| Uç noktalar | `/trade` ve `/market` ayrı bağlantılar | [doğrulanmış] |
| Kullanıcı adı / parola | `JWT` / `<token>` | [doğrulanmış] |
| **Tek bağlantı kuralı** | Aynı token ile aynı uç noktaya ikinci bağlantı ilkini düşürür. Farklı uç noktalar (`/trade` + `/market`) aynı anda açık olabilir. Çoklu sembol tek bağlantıda çoklu abonelikle yapılır. | [doğrulanmış] |
| Client ID | Kodda verilmiyor; mqtt.js rastgele `mqttjs_xxxxxxxx` üretir. Broker'ın client ID kuralı belgelenmemiş. | [kod] |
| Clean session | Kodda verilmiyor; mqtt.js varsayılanı `true`. | [kod] |
| Keepalive | Kodda `10000`. mqtt.js'te birim saniye (≈ 2,8 saat); büyük olasılıkla yanlış birim. Sunucunun beklediği değer bilinmiyor. | [kod], [çıkarım] |
| Bağlantı zaman aşımı | `connectTimeout: 3000` ms | [kod] |
| Abonelik QoS | Tüm aboneliklerde **0** | [kod] |
| TLS doğrulama | `rejectUnauthorized: false` — **taşınmamalı**; yeni bağlayıcı sertifikayı doğrulamalı | [kod] |
| Kütüphanenin kendi yeniden bağlanması | kapalı (`reconnectPeriod: 0`) | [kod] |

TCP portları yalnızca belgede geçer (`DOC/analysis/matriks-mqtt-integration.md`) **[belge]**:

| Hizmet | WS yolu | TCP portu |
|--------|---------|-----------|
| Market | `/market` | 3441 (rt) / 3451 (dl) |
| Trade | `/trade` | 3444 |
| Derinlik | `/depth` | 3445 |
| Haber | `/news` | 3442 |
| Analitik | `/analytics` | 3446 |
| İstatistik | `/stats` | 3443 |

## 8. Konu (topic) yapısı

### Kodda kullanılan ve doğrulanan konular

| Konu | Bağlantı | Mesaj | Etiket |
|------|----------|-------|--------|
| `mx/trade/SYMBOL` | `/trade` | `TradeMessage` | [doğrulanmış] |
| `mx/symbol/SYMBOL` | `/market` | `SymbolMessage` | [doğrulanmış] |
| `mx/timestamp` | `/market` | `TimeMessage` | [doğrulanmış] |

- Eski kod `@lvlN` eki **olmadan** abone olur: "Matriks brokers expect simple topic names:
  mx/trade/GARAN, not mx/trade/GARAN@lvl2" (`realtime.ts` yorumu) **[doğrulanmış]**.
- Belgeler ise seviyeli konu yazar (`mx/trade/SYMBOL@lvl2`) ve Topach örneği de
  `mx/trade/%s@lvl2` döner. **Çelişki:** seviye ekinin lisansa göre zorunlu olup olmadığı
  açık soru. R6'da Topach'ın döndürdüğü kalıp aynen kullanılarak denenmeli **[çıkarım]**.
- Gelen mesaj konuya göre yönlendirilir: `mx/trade` içeren → trade çözücü; `mx/symbol` veya
  `mx/market` içeren → symbol çözücü; `mx/timestamp` → time çözücü (`handleMessage`) **[kod]**.
- Seans dışı saatlerde abonelik `SUBACK` hatası dönebilir; bu beklenen davranıştır
  (`SVC/CHANGELOG.md` Notes) **[doğrulanmış]**.

### Belgede geçen tam konu ağacı **[belge]**

Kaynak: `DOC/reference/matriks-api/mqtt-topics.md`, `DOC/analysis/matriks-mqtt-integration.md`.

```
mx/symbol/{SYMBOL}                 Symbol.proto      hisse/endeks piyasa verisi
mx/derivative/{SYMBOL}             Derivative.proto  vadeli, opsiyon, varant (ör. F önekli kodlar)
mx/trade/{SYMBOL}@lvl{0|1|2}       Trade.proto       gerçekleşen işlemler
mx/tradeex/{SYMBOL}                                  özel işlemler
mx/depth/{SYMBOL}@lvl{2|3}         DepthTable.proto  emir defteri (lvl2 ilk 10, lvl3 kalan 15 kademe)
mx/depthstats/{SYMBOL}                               derinlik istatistikleri
mx/booklet/{SYMBOL}                                  ayrıntılı emir defteri
mx/news@{matriks|kap|aa|comment|int|aimatriks|aikap|aiaa|aicomment|aiint}   News.proto
mx/ratingscore/{SYMBOL}            RatingScore.proto
mx/pgc/{SYMBOL}                    Pgc.proto         para giriş-çıkış (pgc5/pgc8/pgc10)
mx/fundratio/{SYMBOL}
mx/analytics/{warrant|option}/{SYMBOL}               Greeks
mx/stats/{low|high|volume}[/weekly|/monthly|/yearly] RankedSymbols.proto
mx/stats/{option|warrant|future}/{type}
mx/event/prime/{symbol|agent}
mx/user/{sub}/messenger, mx/messenger
mx/timestamp                       Timestamp.proto
```

Joker: `mx/symbol/+` (tüm semboller) belgede geçer; denenmedi **[belge]**.

### Lisans seviyeleri **[belge]**

| Lisans | Kapsam |
|--------|--------|
| KD1 | gecikmeli temel; `mx/symbol/*` gecikmeli, `mx/trade/*@lvl0` |
| PD1P | gerçek zamanlı temel; `mx/symbol/*`, `@lvl1` işlemler |
| PD2 | + derinlik (`mx/depth/*@lvl2`, `@lvl3`), `@lvl1` işlemler |
| AKDE | + `@lvl2` işlemler (alıcı/satıcı kurum), `mx/pgc/*` |
| AINEWS | `mx/news@ai*` |

## 9. Mesaj biçimleri (Protobuf, ikili)

Tüm MQTT yükleri **ikili Protobuf**'tur (JSON değil). Eski kod `.proto` dosyası yüklemez;
şemayı `protobufjs/light` ile satır içi tanımlar. Kodda atıf yapılan `/proto/Symbol.proto`,
`/proto/Trade.proto`, `/proto/Timestamp.proto` dosyaları iki klasörde de **yok**. Belgelerdeki
JSON örnekleri çözülmüş mesajın gösterimidir.

### `TradeMessage` (proto2) — `SVC/ts/src/providers/matriks/proto/trade.ts` **[doğrulanmış]**

| No | Alan | Tip | Seviye | Not |
|----|------|-----|--------|-----|
| 1 | `symbol` | string (required) | lvl0 | |
| 2 | `orderNo` | string | lvl1 | |
| 3 | `price` | **float** (32 bit) | lvl0 | `f64`'e çevrilirken yuvarlama hatası taşır **[çıkarım]** |
| 4 | `quantity` | int64 | lvl1 | lot |
| 5 | `activeBidOrAsk` | string | lvl1 | `"b"` alış, `"a"` satış |
| 6 | `timestamp` | int64 | lvl0 | Unix epoch **mikrosaniye** (bkz. §10 çelişki) |
| 7 | `buyer` | string | lvl2 | alıcı kurum kodu |
| 8 | `seller` | string | lvl2 | satıcı kurum kodu |
| 9 | `isTradeEx` | bool | | özel/blok işlem; belgelerdeki şemada yok, kodda var |

Trade konusunda ilk anlık görüntü yoktur; yalnızca yeni işlemler gelir **[belge]**.

### `SymbolMessage` (proto2) — `proto/symbol.ts` **[doğrulanmış]**

85'ten fazla alanı var; kodda yalnızca grafik için gerekenler çözülür:

| No | Alan | Tip | | No | Alan | Tip |
|----|------|-----|-|----|------|-----|
| 1 | `symbolId` | sint32 | | 17 | `differencePercent` | double |
| 2 | `symbolCode` | string | | 25 | `priceMean` | double |
| 3 | `symbolDesc` | string | | 26 | `limitUp` | double |
| 4 | `updateDate` | string | | 27 | `limitDown` | double |
| 5 | `bid` | double | | 39 | `sessionIsOpen` | bool |
| 6 | `ask` | double | | 47 | `open` | double |
| 7 | `low` | double | | 48 | `dailyQuantity` | double |
| 8 | `high` | double | | 52 | `lastQuantity` | double |
| 9 | `last` | double | | 67 | `direction` | int32 |
| 10 | `dayClose` | double (önceki kapanış) | | 74 | `stockStatus` | int32 |
| 11 | `fractionCount` | int32 | | 75 | `incrementalQuantity` | double |
| 12 | `dailyLow` | double | | 76 | `publishReason` | enum `UPDATE=0`, `REFRESH=1` |
| 13 | `dailyHigh` | double | | | | |
| 14 | `quantity` | double (lot) | | | | |
| 15 | `volume` | double (TL) | | | | |
| 16 | `difference` | double | | | | |

- `REFRESH` = abonelik sonrası tam anlık görüntü; `UPDATE` = yalnızca değişen alanlar
  (delta). İstemci alan bazında birleştirmelidir **[belge]**.
- UPDATE mesajında `symbolCode` olmayabilir (belge örneğinde yalnızca `symbolId` var); eski
  kod `symbolCode` yoksa mesajı **atar** (`handleSymbolMessage`). `symbolId → symbolCode`
  eşlemesi REFRESH'ten tutulmalı **[çıkarım]**.
- `updateDate` biçimi, `direction`, `stockStatus` değerlerinin anlamı belgelenmemiş.

### `TimeMessage` (proto3) — `proto/timestamp.ts` **[doğrulanmış]**

| No | Alan | Tip | Not |
|----|------|-----|-----|
| 1 | `timestamp` | int64 | Unix epoch **mikrosaniye** |
| 2 | `source` | string | `MX`, `BISTPP`, `BISTEX`, `BISTVIOP` |
| 3 | `isBistPPHoliday` | bool | yalnızca `source = MX` iken anlamlı |
| 4 | `isBistPPOpen` | bool | yalnızca `source = MX` iken anlamlı |

Saat eşitleme ve seans açık/kapalı tespiti için kullanılır. Seans dışında da gelebilir
(`realtime.integration.test.ts` başlık yorumu) **[doğrulanmış]**.

### `DepthTableMessage` **[belge]**

`symbol`(1), `timestamp`(3, int64), `asks`(4, repeated `DepthCell`), `bids`(5),
artımlı güncelleme: `action`(6, int32: Insert/Update/Delete), `bidAsk`(7, `"A"`/`"B"`),
`row`(8), `actionPrice`(9, double), `actionQuantity`(10, int64).
`DepthCell`: `price`(1, double), `quantity`(2, int64), `timestamp`(3, int64), `orderCount`(4, int32).
İlk mesaj tam defter (25 kademe × 2 taraf), sonrası artımlı.

### `NewsMessage` **[belge]**

`id`(1), `title`(2), `content`(3), `source`(4), `publishDate`(5, int64), `symbol`(6, repeated),
`aiAnalysis`(7: `summary`, `sentiment`, `importance` 1–10, `impact`). Alan numaraları
belgede var ama kaynak `.proto` olmadan güvenilmemeli.

## 10. Zaman, saat dilimi ve semboller

| Konu | Bilgi | Etiket |
|------|-------|--------|
| REST `time` | epoch **milisaniye** | [doğrulanmış] |
| MQTT `TradeMessage.timestamp`, `TimeMessage.timestamp` | epoch **mikrosaniye** (`/1_000_000` → saniye) | [doğrulanmış] (`CHANGELOG.md`) |
| Eski iç birim | Unix **saniye** (`OHLCV.timestamp`, `Tick.timestamp`) | [kod] |
| Seans | BIST hafta içi yaklaşık 10:00–18:00 İstanbul saati | [doğrulanmış] (test yorumu) |
| REST `start`/`end` | `YYYY-MM-DD`; eski kod **istemcinin yerel saat dilimiyle** biçimliyor | [kod] — hata kaynağı |
| Sembol | BIST kodu, büyük harf, ek yok: `GARAN`, `THYAO`, `ISCTR`, `AKBNK`. Türevlerde önek (belge örneği `F` önekli kod) | [doğrulanmış] / [belge] |
| `symbolId` | sayısal kimlik (sint32), sembol kodundan ayrı | [belge] |

**Çelişki — trade zaman damgası:** belge örneğindeki `"timestamp": "36729322000020"`
mikrosaniye epoch olarak yorumlanırsa 1971 yılına düşer. Bu değer "gece yarısından beri
36.729.322 ms (≈ 10:12:09.322) + 6 haneli sıra numarası" gibi görünüyor **[çıkarım]**.
Kod ise değeri epoch mikrosaniye kabul ediyor ve CHANGELOG bunu doğruluyor. Belge örneği eski
bir biçim olabilir; R6'da gerçek mesajla kontrol edilmeli.

## 11. Yeniden bağlanma

Kaynak: `realtime.ts` (`scheduleReconnect`, `handleDisconnect`) **[kod]**.

- İki bağlantıdan biri `close` veya `offline` olunca durum `reconnecting` olur.
- Üstel geri çekilme: `delay = min(1000 · 2^attempt, 30000)` ms; en çok 10 deneme, sonra
  `error` durumu. Başarıda sayaç sıfırlanır.
- Her denemede Discovery yeniden çağrılır, **iki** bağlantı birlikte yeniden kurulur, tüm
  aktif konulara yeniden abone olunur (QoS 0, clean session olduğundan abonelikler sunucuda
  kalmaz **[çıkarım]**).
- Topach yeniden bağlanmada tekrar çağrılmıyor; token yenileme yok (statik token süresi
  dolunca yeniden bağlanma sürekli başarısız olur) **[çıkarım]**.
- Eski istemciler kapatılmadan yenisi açılıyor; tek bağlantı kuralıyla birleşince
  bağlantıların birbirini düşürme riski var **[çıkarım]**. Yeni bağlayıcıda önce eskisi
  kapatılmalı.

## 12. Bilinen tuhaflıklar

1. Aynı token + aynı uç nokta = tek bağlantı. Birden çok sekme/süreç aynı token'la
   bağlanırsa birbirini düşürür. Sunucu tarafı çoğullayıcı (NATS vb.) gerekçesi budur.
2. Seans dışında abonelik `SUBACK` hatası dönebilir; hata sayılmamalı.
3. `bar.gz` adı yanıltıcı: yanıt gzip taşıma sıkıştırmalı JSON.
4. Türetilmiş/temel uç ayrımı periyoda göre; yanlış uca istek atılırsa davranış belgelenmemiş.
5. Trade `price` 32 bit `float`; Symbol fiyatları `double`. Aynı fiyat iki kaynakta küçük
   farkla görünebilir.
6. Hacim iki anlamda: `volume` = TL, `quantity`/`totalQuantity` = lot.
7. Discovery anahtarları önek gibi görünür ama tam eşleşmeyle okunmalı.
8. Tasarım belgelerindeki örnek istemci kodu (JSON `{"type":"AUTH"}`, `{"type":"SUBSCRIBE"}`,
   `{"type":"PING"}` mesajları, ham `tokio_tungstenite` örneği) **yanlıştır**; gerçek protokol
   MQTT 3.1'dir. Bu örneklere dayanılmamalı (`DOC/analysis/matriks-mqtt-integration.md`,
   `DOC/reference/matriks-api/mqtt-topics.md`).

## 13. Açık sorular (R6 için)

- Token nasıl alınır ve yenilenir (CRM uç noktası, süre)? Sunucu tarafı hizmet hesabı var mı?
- `@lvlN` eki gerekli mi; Topach kalıbı mı, sade konu mu kullanılmalı?
- Asıl `.proto` dosyaları nerede? (Rust tarafında `prost` ile üretim için şart.)
- Broker'ın client ID, keepalive ve clean session beklentisi nedir?
- REST `time` bar başlangıcı mı? `date` hangi saat diliminde?
- `maxBars` sınırları sunucuda gerçekten uygulanıyor mu; aşılırsa ne olur?
- Trade zaman damgası biçimi (§10 çelişkisi).
- `SymbolMessage` 85+ alanın tam listesi ve `stockStatus` / `direction` değerleri.
