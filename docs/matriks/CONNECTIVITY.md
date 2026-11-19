# Matriks Bağlantısı ve Kimlik (RI1 / T1)

Kaynak: `quantex` deposu, HEAD `f3d95aea`, yalnız okuma. Bütün kanıtlar `services/...:satır`
biçimindedir. Uç nokta adresleri, portlar ve gizli değerler yazılmamıştır; yerlerine
yapılandırma değişkeninin adı verilmiştir. Bu belge GOAL.md §A'yı cevaplar; mesaj alanları
`MESSAGES.md` (T2), iç NATS yayını `NATS.md` (T3) konusudur.

## Kısa özet

1. **Belirteç tek yöntemle alınıyor:** Matriks belirteç servisine JSON gövdeli `POST`,
   HTTP Basic kimliği (servis hesabı) ile. Gövdede en az `typeCode` ve `hardwareId` var.
   Kimlik **servis hesabı + `hardwareId`** ikilisine bağlı; `hardwareId` varsayılan olarak
   pod adından (`HOSTNAME`) geliyor, yani **her pod ayrı bir Matriks kimliği** oluyor.
   Dönen belirteç bir JWT. Bütün sunucu servisleri aynı yöntemi kullanıyor, ama gövde
   alanları ve ortam değişkeni adları servisten servise biraz farklı.
2. **MQTT'de kullanıcı adı alanı sabit `JWT`, parola alanı belirtecin kendisi.** MQTT 3.1 /
   3.1.1, WebSocket üzerinden TLS, QoS 0, temiz oturum (clean session). Oturum sürekliliği
   yok; her yeniden bağlanmada abonelikler yeniden kuruluyor.
3. **Bağlantılar uç nokta yoluna (`/market`, `/trade`, `/depth`, `/news`, `/stats`, `/arf` …)
   göre ayrılıyor.** Her servis her uç nokta için tek bağlantı açıyor; o bağlantı üzerinden
   sembol başına konu aboneliği yapıyor.
4. **"İkinci bağlantı birincisini düşürür" kuralı kodda açıkça yazılı ve sunucu kuralı
   olarak anlaşılıyor:** `matriks-user-adapter` bunu "endpoint başına multi-connect koruması"
   diye anlatıyor; anomali motoru Matriks'in `DISCONNECT` / `MULTICONNECT` bildirimini
   tanıyor. İstemci kimliği (client id) çakışması **değil**: bütün servisler her bağlantıda
   benzersiz istemci kimliği üretiyor. Kural kimlik (belirteç / `sub`) + uç nokta düzeyinde.
5. **Borsa açıklığı `mx/timestamp` konusundan geliyor:** `TimeMessage` iletisi zaman damgası
   (mikrosaniye), kaynak, `isBistPPOpen`, `isBistPPHoliday` taşıyor. Açık/kapalı kararı yalnız
   `MX` kaynaklı iletilerden veriliyor. Seans saatleri ayrıca Matriks veri REST'inden çekiliyor.
6. **Qapsule ayrı bağlanacaksa** ayrı bir `hardwareId` (ve tercihen ayrı servis hesabı) ile
   kendi belirtecini almalı; aynı `hardwareId` ile bağlanmak çalışan Quantex bağlantısını
   düşürür. Kullanıcı oturumu ile sunucu servisinin aynı anda bağlanabildiği **kod
   yapısından çıkarılıyor**, ama açıkça denenmiş/yazılmış değil.
7. **Riskler:** paylaşılan ortam dosyasında `MATRIKS_SERVICE_HARDWARE_ID` sabitlenirse anomali
   işçileri (3 kopya) aynı kimliği paylaşır; `matriks-adapter` her yeniden bağlanma olayında
   yeni belirteç istiyor; C++ servislerinde yenilenen belirteç açık bağlantıya hiç
   yansıtılmıyor.

---

## 1. Belirteç (token) alma

### 1.1 Yöntem

| Özellik | Kodda görünen | Kanıt |
|---|---|---|
| Uç nokta | "Matriks belirteç servisi", adres `MATRIKS_TOKEN_SERVICE_URL` ortam değişkeninden | `services/quantex-stream/matriks-adapter/src/config/env.ts:40`, `services/quantex_indicators_price_cpp/src/config/config.hpp:156` |
| Yöntem | HTTP `POST`, `Content-Type: application/json` | `services/quantex-stream/matriks-adapter/src/services/matriks-auth.service.ts:83-92`, `services/quantex_indicators_price_cpp/src/services/token_service.cpp:194-201` |
| Taşıyıcı kimlik | HTTP Basic: `MATRIKS_API_USERNAME` : `MATRIKS_API_PASSWORD` (servis hesabı) | `services/quantex-stream/matriks-adapter/src/services/matriks-auth.service.ts:78-81`, `services/quantex_indicators_price_cpp/src/services/token_service.cpp:195,201` |
| Gövde alanları (TS bağlayıcı, anomali) | `typeCode`, `hardwareId` | `services/quantex-stream/matriks-adapter/src/services/matriks-auth.service.ts:12-15,73-76`, `services/quantex-anomaly-engine/src/services/token_service.hpp:166-167` |
| Gövde alanları (üç C++ gösterge) | `typeCode`, `deviceSourceId`, `hardwareId`; doluysa ayrıca `userName`, `password` (ikinci bir hesap: `MATRIKS_SERVICE_USERNAME/PASSWORD`) | `services/quantex_indicators_price_cpp/src/services/token_service.cpp:180-190` |
| `typeCode` varsayılanı | `H` | `services/quantex-stream/matriks-adapter/src/config/env.ts:44`, `services/quantex_indicators_price_cpp/src/config/config.hpp:32` |
| Yanıt | `{ success, code, message?, result: { token, userName, password, typeCode, pin?, licenses? } }` | `services/quantex-stream/matriks-adapter/src/services/matriks-auth.service.ts:17-29` |
| Yanıtta ömür | C++: `result.expiresIn` (saniye), yoksa 3600 | `services/quantex_indicators_price_cpp/src/services/token_service.cpp:279-286` |

Not: yanıt şeması `licenses` alanı da içeriyor (`matriks-auth.service.ts:27`); kod bu alanı
okumuyor. Lisans sınıfının belirtece gömülü olabileceğine işaret eder; lisans konusu T2'de.

### 1.2 Kimlik neye bağlı

- **Servis hesabı** (HTTP Basic) belirteç servisine erişimi açıyor.
- **`hardwareId`** cihazı/örneği tanımlıyor. Çözüm sırası her serviste aynı: açık ortam
  değişkeni → `HOSTNAME` (k8s'te pod adı) → servis adı + rastgele onaltılık ek.
  - TS: `MATRIKS_HARDWARE_ID` (`services/quantex-stream/matriks-adapter/src/config/env.ts:120-132`,
    yorum: "unique Matriks token per pod/container").
  - C++: `MATRIKS_SERVICE_HARDWARE_ID` (`services/quantex_indicators_price_cpp/src/config/config.hpp:161-173`;
    anomali: `services/quantex-anomaly-engine/src/config/config.hpp:225-236`).
- C++ göstergelerde ayrıca **`deviceSourceId`** gönderiliyor; boşsa
  `quantex-indicators-cpp-<rastgele 8 hane>` üretiliyor (`services/quantex_indicators_price_cpp/src/config/config.hpp:177-185`).
- **Belirtecin `sub` değeri `hardwareId`'den türüyor gibi:** akış servisindeki yorum,
  kullanıcıya özel yansı konusunu `mx/user/QTX-<hwid>/mx/pgc/<SYM>` diye veriyor
  (`services/quantex_indicators_flow_cpp/src/services/mqtt_manager.cpp:520-523`).
  Varsayım: Matriks servis belirtecinin `sub` alanı `QTX-` + `hardwareId` biçiminde; yani
  Matriks açısından "kimlik" = `hardwareId`. Doğrulama: Matriks veri ekibi.
- IP'ye bağlılık kodda görünmüyor. Bilinmiyor — kime sorulur: Matriks veri ekibi.

### 1.3 Ömür ve yenileme

| Servis | Ömrü nereden alıyor | Ne zaman yeniliyor | Kanıt |
|---|---|---|---|
| `matriks-adapter` (TS) | JWT `exp` alanı; `exp` yoksa 5 saat, çözülemezse 1 saat | Önbellekteki belirteç bitişe `TOKEN_REFRESH_BUFFER_SECONDS` (varsayılan 300 s) kalmışsa siliniyor, bir sonraki `getToken` yenisini alıyor. Ayrıca **her MQTT `reconnect` olayında** önbellek silinip yeni belirteç isteniyor | `services/quantex-stream/matriks-adapter/src/services/matriks-auth.service.ts:150-170,211-233`, `services/quantex-stream/matriks-adapter/src/config/env.ts:77`, `services/quantex-stream/matriks-adapter/src/services/mqtt-client.service.ts:168-175,266-277` |
| Üç C++ gösterge | `expiresIn` (yoksa 3600 s) | Döngü 5 dakikada bir uyanıyor; bitişe ≤5 dk kaldıysa yeniliyor; hata varsa 1 dakikada bir deniyor | `services/quantex_indicators_price_cpp/src/services/token_service.cpp:103-149` |
| Anomali motoru | `expiresIn` (yoksa 3600 s) | Bitişten 5 dk önce uyanıp yeniliyor; belirteç geçersizse 30 s'de bir | `services/quantex-anomaly-engine/src/services/token_service.hpp:100-125,243-261` |
| `sync-service` (yalnız REST) | JWT `exp` | Bitişe 5 dk kalmışsa yeniden alıyor | `services/quantex-stream/sync-service/src/clients/matriks-meta.client.ts:203-250` |

TS bağlayıcıda belirteç Redis'te `matriks:adapter:token` anahtarıyla önbelleğe alınıyor
(`services/quantex-stream/matriks-adapter/src/services/matriks-auth.service.ts:36,175-194`);
birden çok kopya çalışsa aynı belirteci paylaşırlardı.

### 1.4 Yenilenen belirteç açık bağlantıya nasıl geçiyor

- **C++ göstergeler ve anomali:** üç gösterge servisinde belirteç servisi yeni belirteci
  yalnız bellekte tutuyor; `set_on_token_update` orada çağrılmıyor. Anomali motoru onu
  çağırıyor (`services/quantex-anomaly-engine/src/main.cpp:589-591`), ama geri çağrı yalnız
  kopmuş akışları yeniden bağlanmaya işaretliyor
  (`services/quantex-anomaly-engine/src/services/mqtt_manager.hpp:359-365`; denetimle
  düzeltildi, `rounds/RI1/AUDIT.md`). Sonuç her ikisinde aynı: açık MQTT bağlantısı eski belirteçle sürüyor; yeni belirteç yalnız **yeniden bağlanırken**
  kullanılıyor (`services/quantex_indicators_price_cpp/src/services/mqtt_manager.cpp:797-835`,
  `services/quantex-anomaly-engine/src/services/mqtt_manager.hpp:634-650`).
  Bundan çıkan dolaylı bilgi: Matriks açık bağlantıyı belirteç süresi dolunca hemen
  düşürmüyor olabilir. Anomali motoru yine de bağlantı kapanma nedenleri arasında "token
  expired" sayıyor (`services/quantex-anomaly-engine/src/services/mqtt_manager.hpp:774-777`).
  Bilinmiyor — kime sorulur: Matriks veri ekibi (süresi dolan belirteçle açık bağlantı
  kesiliyor mu?).
- **`matriks-adapter`:** `reconnect` olayında yeni belirteç alıp `client.options.password`
  alanına yazıyor (`services/quantex-stream/matriks-adapter/src/services/mqtt-client.service.ts:266-277,385-392`).
  **Çelişki:** `matriks-user-adapter` aynı kitaplık için "mqtt.js yenilenen parolayı uçuşta
  almaz" diyor ve bu yüzden bağlantıyı kapatıp yenisini açıyor
  (`services/quantex-stream/matriks-user-adapter/src/services/userPool.ts:361-367`). Ayrıca
  `reconnect` olayı ile belirteç isteği eşzamansız; ilk deneme büyük olasılıkla eski
  parolayla gidiyor. Hangisinin doğru olduğu koddan kesinleşmiyor.
- **`matriks-user-adapter`:** yeni zarf (envelope) geldiğinde her uç nokta için **önce yeni
  bağlantıyı açıp abone oluyor, sonra eskiyi kapatıyor** ("hot-swap")
  (`services/quantex-stream/matriks-user-adapter/src/services/userPool.ts:10-12,109-119`).

### 1.5 Süresi dolunca / 401'de davranış

- **Belirteç servisi HTTP hatası:** TS bağlayıcı hata fırlatıyor, özel 401 işlemi yok
  (`services/quantex-stream/matriks-adapter/src/services/matriks-auth.service.ts:118-135`).
  C++: ağ hatalarında ve 5xx'te artan beklemeyle yeniden deniyor; 200 dışı (401 dahil)
  yanıtta o turu bırakıyor (`services/quantex_indicators_price_cpp/src/services/token_service.cpp:213-245`).
  Açılışta belirteç alınamazsa C++ servisleri yine de ayağa kalkıyor
  (`token_service.cpp:54-57`); TS bağlayıcı açılışta düşüyor (`services/quantex-stream/matriks-adapter/src/index.ts:348-351`).
- **MQTT CONNACK yetki reddi:** yalnız `matriks-user-adapter` ayrı işliyor: MQTT 3.1 dönüş
  kodu 4 (hatalı kullanıcı adı/parola) ve 5 (yetkisiz) için bağlantıyı kapatıp mqlaw'dan
  yeni belirteç istiyor (`services/quantex-stream/matriks-user-adapter/src/services/userPool.ts:529-545,371-420`).
  Öteki servisler bunu genel bağlantı hatası gibi görüp yeniden bağlanma döngüsüne giriyor.

### 1.6 Her servis aynı yöntemi mi kullanıyor?

Yöntem aynı (POST + Basic + `typeCode`/`hardwareId`, MQTT'de `JWT`/belirteç). Farklar:

| Fark | Ayrıntı | Kanıt |
|---|---|---|
| Basic kimliğinin değişken adı | `matriks-adapter` ve C++: `MATRIKS_API_USERNAME/PASSWORD`; `sync-service`: `MATRIKS_SERVICE_USERNAME/PASSWORD` | `services/quantex-stream/matriks-adapter/src/config/env.ts:41-42`, `services/quantex-stream/sync-service/src/config/env.ts:47-48`, `services/quantex-stream/sync-service/src/clients/matriks-meta.client.ts:212` |
| `hardwareId` değişken adı | TS: `MATRIKS_HARDWARE_ID`; C++: `MATRIKS_SERVICE_HARDWARE_ID` | yukarıda |
| Ek gövde alanları | Yalnız üç C++ gösterge `deviceSourceId` (+ isteğe bağlı `userName`/`password`) gönderiyor; anomali ve TS göndermiyor | `services/quantex_indicators_price_cpp/src/services/token_service.cpp:180-190`, `services/quantex-anomaly-engine/src/services/token_service.hpp:165-167` |
| Ömür kaynağı | TS: JWT `exp`; C++: yanıttaki `expiresIn` | 1.3 |
| Kullanıcı kimliği | `matriks-user-adapter` belirteç **almıyor**; mqlaw'ın Redis'e yazdığı, HMAC imzalı kullanıcı JWT'sini kullanıyor | `services/quantex-stream/matriks-user-adapter/src/config/env.ts:4-6`, `services/quantex-stream/matriks-user-adapter/src/services/envelope.ts:19-48` |

Ek çelişki: yerel yığın tanımı `matriks-adapter` için `MATRIKS_SERVICE_USERNAME/PASSWORD`
veriyor (`services/quantex-stream/docker/docker-compose.fullstack.yml` matriks-adapter bölümü,
satır 95-97; `services/quantex-stream/docker/docker-compose.services.yml:80-81`), oysa
bağlayıcının şeması `MATRIKS_API_USERNAME/PASSWORD` istiyor (`env.ts:41-42`). Bu yığın
dosyaları güncel değil ya da bağlayıcı bu dosyalarla açılmıyor.

---

## 2. MQTT bağlantıları

### 2.1 Ortak bağlantı ayarları

| Ayar | TS (`mqtt` paketi v5, `matriks-adapter`, `matriks-user-adapter`) | C++ göstergeler (Paho C++) | Anomali (async_mqtt) |
|---|---|---|---|
| Protokol sürümü | `protocolVersion: 3`, `protocolId: 'MQIsdp'` (MQTT 3.1), WebSocket alt protokolü `mqttv3.1` | Sürüm ayarlanmıyor (kitaplık varsayılanı) | MQTT 3.1.1 (`v3_1_1`) |
| Taşıma | WebSocket + TLS; sertifika denetimi kapalı (`rejectUnauthorized: false`) | `wss` (host + port + yol); sunucu sertifika denetimi kapalı ("test env için") | WSS; üretimde doğrulama açık, değilse kapalı |
| Kullanıcı adı / parola | `JWT` / belirteç | `JWT` / belirteç | `JWT` / belirteç |
| Temiz oturum | Belirtilmiyor (kitaplık varsayılanı: açık) | `set_clean_session(true)` | CONNECT paketinde `true` |
| Keepalive | `10000` (mqtt.js'te birim saniye → ~2,8 saat; muhtemelen ms sanılarak yazılmış) | 10 s | 100 s |
| İstemci kimliği | **Verilmiyor** → kitaplık rastgele üretir | `quantex-{ind,flow,depth}-mx-<engine_instance_id>-<unix saniye>` | `<engine_id>_<akış adı>_<steady_clock ns>` |
| QoS | 0 | 0 | 0 (paket ayrıntısı T2) |
| Otomatik yeniden bağlanma | `reconnectPeriod: 1000` ms, sabit, sınırsız | Kapalı; kendi işçisi | Kapalı; kendi izleyicisi |

Kanıt: `services/quantex-stream/matriks-adapter/src/services/mqtt-client.service.ts:101-116,306-316`,
`services/quantex-stream/matriks-user-adapter/src/services/userPool.ts:430-441`,
`services/quantex_indicators_price_cpp/src/services/mqtt_manager.cpp:84-103`,
`services/quantex_indicators_flow_cpp/src/services/mqtt_manager.cpp:85-97`,
`services/quantex_indicators_depth_cpp/src/services/mqtt_manager.cpp:85-97`,
`services/quantex-anomaly-engine/src/services/mqtt_manager.hpp:57-58,466-499,627-650`.
TS kodu bu ayarları "saska-client.js ile birebir" diye açıklıyor
(`mqtt-client.service.ts:101`, `userPool.ts:430`): Matriks'in kendi istemcisinden alınmış.

mqtt.js'in istemci kimliği vermeyince rastgele ürettiği ve keepalive birimi saniye olduğu
kitaplık belgesine dayanıyor; bu round'da denenmedi (quantex'te `node_modules` okunmadı).

### 2.2 Uç noktalar (yayın tipi = bağlantı yolu)

`matriks-user-adapter`'in konu planı, uç nokta adının Matriks aracısının yol adıyla birebir
eşleştiğini ve hangi konunun hangi uç noktada olduğunu yazıyor
(`services/quantex-stream/matriks-user-adapter/src/services/topicPlan.ts:4-28`; kaynak olarak
mqlaw deposundaki `MatriksServices/MQTT/*.md` belgelerini anıyor):

| Uç nokta (yol) | Konular (kalıp) |
|---|---|
| `market` | `mx/timestamp`, `mx/symbol/<SYM>`, `mx/derivative/<SYM>`, `mx/fundratio/<SYM>`, `mx/ratingscore/<SYM>`, `mx/stats/{low,high,volume}[/weekly\|monthly\|yearly]`, `mx/stats/future/{low,high,volume}` |
| `arf` | `mx/user/<sub>/messenger`, `mx/messenger` (alarm/ileti) |
| `news` | `mx/event/prime/{symbol,agent}`, `mx/news@<kaynak>` (matriks, kap, aa, comment, int ve `ai*` karşılıkları) |
| `depth` | `mx/depth`, `mx/depth/<SYM>@lvl2`, `…@lvl3`, `mx/depthstats`, `mx/booklet` |
| `trade` | `mx/trade`, `mx/tradeex`, `mx/pgc` |
| `analytics` | `mx/analytics/warrant`, `mx/analytics/option` |
| `stats` | `mx/stats/option/…`, `mx/stats/warrant/…` (hisse ve vadeli "mover" konuları `market`'te) |
| `foreignmarket` | `mx/iex/symbol`, `mx/nbbo/symbol` |
| `foreigntrade` | `mx/iex/trade` |

Ayrıca gerçek zamanlı / gecikmeli ayrımı **ayrı sunucu** ile yapılıyor:
`MATRIKS_MQTT_RT_HOST` / `MATRIKS_MQTT_DL_HOST`, seçim `MATRIKS_MQTT_MODE`
(varsayılan `delayed`) (`services/quantex-stream/matriks-adapter/src/config/env.ts:47-49,147-151`).
Konu kuralları da sembol tipine göre `qos: dl|rt` etiketi taşıyor (burada `qos` MQTT QoS'u
değil, gecikmeli/gerçek zamanlı anlamında) (`services/quantex-stream/matriks-adapter/src/services/topic-resolver.service.ts:56-60,116-143`).

### 2.3 Servis × bağlantı sayımı

| Servis (kopya sayısı) | Matriks bağlantısı | Uç nokta | İstemci kimliği kuralı | Konu kalıpları | Belirteç kimliği |
|---|---|---|---|---|---|
| `matriks-adapter` (k8s 1; yığın 1, "DO NOT CHANGE") | 1 ana + en çok 2 ikincil = **≤3** | ana: `MATRIKS_MQTT_RT_HOST` veya `_DL_HOST` (yorumda `/market`); ikincil: `MATRIKS_MQTT_STATS_HOST`, `MATRIKS_MQTT_NEWS_HOST` (yalnız `DATA_TYPES` içeriyorsa) | yok → kitaplık rastgele | ana: `mx/symbol/<SYM>@lvl2`, `mx/derivative/<SYM>@lvl2`, `mx/depth/<SYM>`, `mx/trade/<SYM>`, `mx/timestamp` (`DATA_TYPES`'a göre; varsayılan yalnız `symbol`); stats: `mx/stats/{high,low,volume}`; news: `mx/news@<10 kaynak>` | 1 (`hardwareId` = pod adı); üç bağlantı **aynı belirteci** kullanıyor |
| `matriks-user-adapter` (k8s 1, ölçekleme kapalı) | kullanıcı × zarftaki uç nokta (en çok 9) | `MATRIKS_MQTT_{MARKET,ARF,NEWS,DEPTH,TRADE,ANALYTICS,STATS,FOREIGN_MARKET,FOREIGN_TRADE}_HOST` | yok → kitaplık rastgele | market: `mx/timestamp` + dinamik `mx/symbol/<SYM>@lvl2`; arf: `mx/user/<sub>/messenger` (aracı kendisi abone ediyor); news: `mx/event/prime/{symbol,agent}`, `mx/news@ai{matriks,kap,aa,comment}`; trade: `mx/pgc/+`; ötekiler boş | kullanıcının kendi JWT'si |
| `quantex_indicators_price_cpp` (deploy formunda kopya yok, ölçekleme kapalı) | **1** | `MATRIKS_MQTT_HOST` + `MATRIKS_MQTT_PORT` + `MATRIKS_MQTT_PATH` (varsayılan `/market`) | `quantex-ind-mx-<engine_instance_id>-<unix s>` | `mx/timestamp`, `mx/symbol/<SYM>` (`@lvl2` eki **yok**) | 1 (`hardwareId` = pod adı) |
| `quantex_indicators_flow_cpp` (aynı) | **1** | aynı değişkenler, varsayılan `/trade` | `quantex-flow-mx-…` | `mx/timestamp`, `mx/trade/<SYM>`, `mx/pgc/<SYM>` | 1 |
| `quantex_indicators_depth_cpp` (aynı) | **1** | aynı değişkenler, varsayılan `/depth` | `quantex-depth-mx-…` | `mx/timestamp`, `mx/depth/<SYM>` | 1 |
| `quantex-anomaly-engine` işçi (k8s ölçekleme açık, "önerilen 3"; yığında `replicas: 3`) | **4** (news yalnız host verilmişse) | `MATRIKS_MQTT_{DEPTH,TRADE,MARKET,NEWS}_{HOST,PORT,PATH}` | `<engine_id>_<akış>_<ns>`; `engine_id` işçide rastgele `eng_<6 hane>` | depth: `mx/depth/<SYM>@lvl2`, `mx/depthstats/<SYM>`, `mx/booklet/<SYM>`; trade: `mx/trade/<SYM>@lvl2`, `mx/tradeex/<SYM>`, `mx/pgc/<SYM>`; market: `mx/symbol/<SYM>@lvl2`, `mx/timestamp`; news: `mx/news@{matriks,kap,comment,aimatriks,aikap,aicomment}` | işçi başına 1 (`hardwareId` = pod adı, açıkça verilmemişse) |
| anomali koordinatörü (1) | 0 (yalnız REST için belirteç) | — | — | — | 1 |
| `sync-service` | 0 (yalnız REST) | — | — | — | 1 |

Kanıt: `services/quantex-stream/matriks-adapter/src/services/mqtt-client.service.ts:18-45,193-228,283-339`;
`services/quantex-stream/k8s/matriks-adapter-deploy.yml:12`;
`services/quantex-stream/docker/docker-compose.fullstack.yml:109`;
`services/quantex-stream/matriks-user-adapter/src/config/env.ts:43-51,133-145`,
`services/quantex-stream/matriks-user-adapter/src/services/topicPlan.ts:59-160,187-197`,
`services/quantex-stream/k8s/matriks-user-adapter-deploy.yml:12,23-27`;
`services/quantex_indicators_price_cpp/src/config/config.hpp:41-44,189-191,332-335`,
`services/quantex_indicators_price_cpp/src/services/mqtt_manager.cpp:85,125,392`,
`services/quantex_indicators_price_cpp/cpp-indicator-deploy.yml:20-24`;
`services/quantex_indicators_flow_cpp/src/config/config.hpp:44`, `services/quantex_indicators_flow_cpp/src/services/mqtt_manager.cpp:85,397`;
`services/quantex_indicators_depth_cpp/src/config/config.hpp:44`, `services/quantex_indicators_depth_cpp/src/services/mqtt_manager.cpp:85,392`;
`services/quantex-anomaly-engine/src/services/mqtt_manager.hpp:43-53,426-453`,
`services/quantex-anomaly-engine/src/main.cpp:232-238,436-438,616-640`,
`services/quantex-anomaly-engine/cpp-anomaly-engine-deploy.yml:133-138,162-163`,
`services/quantex-anomaly-engine/docker-compose.yml:104,128`;
`services/quantex-anomaly-engine/src/main.cpp:300-311` (koordinatör);
`services/quantex-stream/sync-service/src/clients/matriks-meta.client.ts:201-250`.

Her C++ servis ayrıca yerel (iç) bir MQTT aracısına ikinci bir bağlantı açıyor
(`CLIENT_MQTT_*`; komut ve sonuç yayını). Bu Matriks bağlantısı değil, sayıma girmedi
(`services/quantex_indicators_price_cpp/src/services/mqtt_manager.cpp:151-238`).

**Gözlemlenen tutarsızlıklar** (aynı veri için farklı konu adları):
- `mx/symbol/<SYM>` (fiyat göstergesi) ile `mx/symbol/<SYM>@lvl2` (bağlayıcı, anomali,
  kullanıcı bağlayıcısı); `mx/trade/<SYM>` (akış) ile `mx/trade/<SYM>@lvl2` (anomali);
  `mx/depth/<SYM>` (derinlik göstergesi, bağlayıcı) ile `mx/depth/<SYM>@lvl2` (anomali).
  Eksiz kalıbın ne döndürdüğü (varsayılan seviye mi, ret mi) bilinmiyor — kime sorulur:
  Matriks veri ekibi.
- `matriks-adapter` `depth` ve `trade` konularını **ana (market) bağlantısına** abone ediyor
  (`mqtt-client.service.ts:18-27,200-217`); konu planına göre bunlar ayrı `depth` / `trade`
  uç noktalarında. Varsayılan `DATA_TYPES=symbol` olduğundan bu yol üretimde kullanılmıyor
  olabilir; üretim değeri ortam dosyasında, okunmadı.
- `matriks-adapter` `mx/stats/{high,low,volume}` konularını `stats` uç noktasına bağlıyor
  (`mqtt-client.service.ts:43-44,288-293`); konu planı hisse "mover" konularının `market`
  uç noktasına düştüğünü, `stats` sunucusuna değil, açıkça yazıyor (`topicPlan.ts:23-26`).

### 2.4 "İkinci bağlantı birincisini düşürür" kuralı kodda

0. **Matriks belgesi:** quantex deposundaki Matriks servis belgesi kopyası kuralı açıkça
   yazıyor: aynı kullanıcıya ait belirteçle aynı adrese ikinci bağlantı kurulunca ilk
   bağlantı sonlandırılır (`.worktrees/mcp-feat-anve-regenerate-dual-scope/docs/working-docs/MatriksServices/MQTT/market.md:26,166`;
   belge, kod değil; denetimde bulundu). Kuralın anahtarı belgeye göre **belirteç /
   kullanıcı**; `hardwareId`'nin bu kimliği nasıl belirlediği yine Matriks'e sorulur (§7 soru 1).
1. **Açık yorum:** `matriks-user-adapter` her kullanıcı için her uç noktaya ayrı bağlantı
   açtığını ve bunun nedenini yazıyor: Matriks'in çoklu bağlantı koruması uç nokta
   başına; aynı uç noktaya ikinci bağlantı eskisini düşürüyor
   (`services/quantex-stream/matriks-user-adapter/src/services/userPool.ts:5-7`). Giriş
   dosyası, mqlaw'ın Matriks'e doğrudan bağlanmayı bırakıp bu bağlayıcıyı kullanmasının
   amacını "multi-connect sorununu kökten çözmek" diye veriyor
   (`services/quantex-stream/matriks-user-adapter/src/index.ts:4-7`).
2. **Sunucu bildirimi:** anomali motoru `mx/messenger` iletisinde `code == "DISCONNECT"` ve
   `subject == "MULTICONNECT"` görünce "aynı belirteci kullanan başka oturum" uyarısı
   yazıyor (`services/quantex-anomaly-engine/src/main.cpp:1786-1797,1822-1833`). Yani
   Matriks düşürmeyi bir **uygulama düzeyi ileti** ile bildiriyor; bu, kuralın istemci
   kimliği çakışması (MQTT protokol kuralı) değil, **sunucu politikası** olduğunu gösteriyor.
   Dikkat: motor `mx/messenger`'a Matriks bağlantısından değil **yerel** MQTT bağlantısından
   abone oluyor (`main.cpp:1811,1844,1943`; `subscribe_client`). Matriks iletisinin yerel
   aracıya nasıl geldiği kodda görünmüyor; bu uyarı pratikte hiç tetiklenmiyor olabilir.
3. **Kopma nedeni yorumu:** anomali motoru aracının bağlantıyı kapatmasını (EOF) olası
   nedenleriyle yazıyor: "aynı belirteçle ikinci bağlantı", "belirteç süresi doldu",
   "keepalive zaman aşımı" (`services/quantex-anomaly-engine/src/services/mqtt_manager.hpp:774-777`).
4. **İstemci kimliği çakışması değil:** bütün servisler her bağlantı denemesinde zaman
   damgalı ya da rastgele benzersiz istemci kimliği kuruyor (2.1). Bu yüzden düşürme,
   MQTT'nin "aynı client id ikinciyi atar" kuralından değil, kimlik (belirteç / `sub`) +
   uç nokta eşleşmesinden geliyor. Sunucunun anahtarı tam olarak ne (belirteç dizgisi mi,
   `sub` mu, `hardwareId` mi) — bilinmiyor; kime sorulur: Matriks veri ekibi.
5. **Kodun kurala uyduğu yerler:** fiyat göstergesi yeniden bağlanmadan önce eski
   istemciyi kapatıyor (`services/quantex_indicators_price_cpp/src/services/mqtt_manager.cpp:811-820`);
   anomali motoru da öyle (`services/quantex-anomaly-engine/src/services/mqtt_manager.hpp:610-619`);
   anomali motoru dört akışı TLS el sıkışma çökmesine karşı sırayla açıyor (`mqtt_manager.hpp:424-453`).
6. **Kuralla gerilimde olan yer:** `matriks-user-adapter` belirteç yenilemede yeni
   bağlantıyı eskisi açıkken açıyor (`userPool.ts:109-119`) ve yorumda "multi-connect ihlali
   yok" diyor (`userPool.ts:10-12`). Kural doğruysa yeni bağlantı açıldığı anda sunucu
   eskisini zaten düşürür; bu zararsız (eski kapanacaktı), ama yorumun dayanağı belirsiz.

### 2.5 Kopya sayıları ve aynı kimlikle çoklu bağlantı riski

- `matriks-adapter`: k8s formu "leader election destekli, replika artırılabilir"
  diyor (`services/quantex-stream/k8s/matriks-adapter-deploy.yml:6-7,25-28`), yığın dosyası da
  "Leader election" notu düşüyor (`services/quantex-stream/docker/docker-compose.fullstack.yml:109`).
  **Kodda lider seçimi yok:** `src/` içinde yalnız sabit 1 yazılan bir ölçü var ve yorumu
  "Single instance mode" (`services/quantex-stream/matriks-adapter/src/utils/metrics.ts:33-35,251`).
  Kod esastır: kopya sayısı 2 olursa iki pod ayrı `hardwareId` ile bağlanır (çakışma olmaz
  ama aynı veri iki kez NATS'a yayınlanır); ancak belirteç Redis'te ortak önbellekte
  tutulduğu için iki pod **aynı belirteci** kullanabilir (`matriks-auth.service.ts:36,49-62`)
  → aynı uç noktaya aynı belirteçle iki bağlantı → karşılıklı düşürme riski.
- C++ göstergeler: deploy formlarında kopya alanı yok, ölçekleme kapalı; çoklu örnek için
  `ENGINE_INSTANCE_ID`'nin her kopyada farklı olması isteniyor
  (`services/quantex_indicators_price_cpp/cpp-indicator-deploy.yml:20-24`); boşsa Redis
  sırasından otomatik atanıyor (`services/quantex_indicators_price_cpp/src/main.cpp:90-109`).
  Bu yalnız istemci kimliğini etkiliyor; Matriks kimliği `hardwareId`'den geliyor.
- Anomali işçileri: 3 kopya. `MATRIKS_SERVICE_HARDWARE_ID` k8s formunda "Ortak env (tüm C++
  bileşenler paylaşır)" listesinde (`services/quantex-anomaly-engine/cpp-anomaly-engine-deploy.yml:54-60`).
  **Bu değişken ortak ortam dosyasında doluysa bütün işçiler (ve koordinatör) aynı kimlikle
  belirteç alır; her işçi aynı dört uç noktaya bağlanacağı için birbirini düşürür.** Boşsa
  her pod kendi adını kullanır ve sorun olmaz. Ortam dosyası okunmadı (kural gereği).
  Bilinmiyor — kime sorulur: Quantex ekibi (üretimde bu değişken dolu mu?).
- Aynı ortak liste C++ gösterge formlarında da var (`cpp-indicator-deploy.yml:43`); üç
  gösterge servisi aynı değeri paylaşsa bile **farklı uç noktalara** (`/market`, `/trade`,
  `/depth`) bağlandıkları için uç nokta başına kural gereği çakışmazlar. Ama anomali
  işçileri de aynı değeri alırsa onlarla çakışırlar.

### 2.6 Yeniden bağlanma ve geri çekilme

| Servis | Davranış | Kanıt |
|---|---|---|
| `matriks-adapter` | mqtt.js kendi döngüsü, 1 s sabit aralık, sınırsız; her `reconnect` olayında yeni belirteç isteği (geri çekilme yok → aracı uzun süre kapalıysa belirteç servisine saniyede bir istek) | `mqtt-client.service.ts:106,168-175`; ikincil bağlantılar için de aynı (`:372-375`) |
| `matriks-user-adapter` | mqtt.js 1 s döngüsü; CONNACK 4/5'te döngüyü durdurup yenileme istiyor; 30 s'de bir zarfları yeniden doğruluyor, bitişe 60 s kala yenileme istiyor (30 s soğuma) | `userPool.ts:43-46,203-255,529-545` |
| C++ göstergeler | Açılışta `MQTT_MATRIKS_MAX_RETRIES` (3) deneme, doğrusal bekleme; koptuktan sonra üstel geri çekilme (başlangıç `MQTT_RECONNECT_INITIAL_DELAY_MS`=1 s, tavan `MQTT_RECONNECT_MAX_DELAY_MS`=30 s, en çok `MQTT_RECONNECT_MAX_RETRIES`=5); her denemeden önce belirteç yenileme; sonra bütün semboller ve `mx/timestamp` yeniden abone; 15 s'lik bekçi, etkin abonelik varsa döngüyü yeniden kuruyor | `services/quantex_indicators_price_cpp/src/config/config.hpp:61-65`, `services/quantex_indicators_price_cpp/src/services/mqtt_manager.cpp:78-149,735-925` |
| Anomali | 5 s'lik izleyici; düşen akış için `do_full_reconnect_wss`, üstel bekleme; `trade` akışı 3 ardışık hatadan sonra 60 s'de bir deneniyor ("seans dışında normal") | `services/quantex-anomaly-engine/src/services/mqtt_manager.hpp:1196-1240,1259-1300`, `services/quantex-anomaly-engine/src/main.cpp:603-605` |

Oturum sürekliliği yok: her yerde temiz oturum; abonelikler istemci tarafında tutulup
yeniden bağlanınca yeniden gönderiliyor (C++: `resubscribe_all_symbols`, anomali:
`resubscribe_stream`). Kaçırılan iletileri sunucudan geri alma yolu kodda görünmüyor.

---

## 3. Qapsule ayrı sistem olarak bağlanırsa

Koddan çıkanlar:

- **Kendi belirteci gerekir.** Belirteç almak için bir servis hesabı (Basic kimliği) ve bir
  `hardwareId` gerekiyor (1.1–1.2). Quantex'in hesabını kullanmak mümkün görünüyor (C++ ve
  TS aynı `MATRIKS_API_*` hesabını farklı `hardwareId`'lerle kullanıyor), ama bu Quantex'in
  kimliğinin altında çalışmak olur.
- **`hardwareId` asla Quantex pod'larıyla aynı olmamalı.** Aynı `hardwareId` → (varsayım:
  aynı `sub`) → aynı uç noktada ikinci bağlantı → Quantex'in bağlantısı düşer (2.4).
  Ayrıca belirteç önbelleği (Redis `matriks:adapter:token`) paylaşılmamalı.
- **Uç nokta başına tek bağlantı** açmalı; sembolleri o bağlantıda konu konu abone etmeli.
  Qapsule'ün ihtiyacı en az `market` (fiyat + `mx/timestamp`), çoğunlukla `trade` ve
  `depth` olur (2.2).
- **Ayrı servis hesabı mı gerekir?** Arkin'in 1. bilgisi "belirteç sisteme özel" diyor;
  kodda `typeCode` (`H`) ve `deviceSourceId` (`quantex-indicators-cpp…`) sisteme özgü
  değerler gibi duruyor. Hesabın kendisinin sisteme özel olup olmadığı, Qapsule için yeni
  bir hesap / `typeCode` gerekip gerekmediği koddan anlaşılmıyor — kime sorulur: Matriks
  veri ekibi + Arkin.
- **Lisans:** yanıttaki `licenses` listesi belirtecin hangi veriye eriştiğini belirliyor
  olabilir; aracı lisanssız aboneliği `granted=128` ile reddediyor
  (`services/quantex-stream/matriks-user-adapter/src/services/topicPlan.ts:103-105`).
  Qapsule hesabının lisans kapsamı ayrıca konuşulmalı.

### 3.1 `matriks-user-adapter` kullanıcının oturumuyla ne yapıyor

- mqlaw (ayrı depo) kullanıcı adına Matriks JWT'sini alıyor ve Redis'e HMAC imzalı bir
  zarf olarak yazıyor: `user_id`, `matriks_sub`, `matriks_jwt`, `endpoints[]`, `issued_at`,
  `expires_at`, `hmac_signature` (`services/quantex-stream/matriks-user-adapter/src/services/envelope.ts:19-27`).
  Bağlayıcının kendi servis kimliği yok (`services/quantex-stream/matriks-user-adapter/src/config/env.ts:4-6`).
- Bağlayıcı açılışta Redis'i tarıyor, sonra güncelleme kanalını dinliyor; her kullanıcı
  için zarftaki her uç noktaya **kullanıcının JWT'siyle** ayrı MQTT bağlantısı açıyor ve
  gelen iletileri olduğu gibi NATS'ta `matriks.u.<user_id>.*` konularına aktarıyor
  (`services/quantex-stream/matriks-user-adapter/src/index.ts:149-169`,
  `services/quantex-stream/matriks-user-adapter/src/services/userPool.ts:61-147,510-527`).
- Kullanıcı çevrimdışı olsa da bağlantı tutuluyor (alarm izleme için)
  (`services/quantex-stream/matriks-user-adapter/src/index.ts:149-152`).
- Sembol aboneliği iç NATS denetim konusu `matriks.control.u.<user_id>.symbol.{subscribe,unsubscribe}`
  ile ekleniyor/kaldırılıyor (`services/quantex-stream/matriks-user-adapter/src/services/liveQuoteControl.ts:6-7,21`).
- Aracı bazı kullanıcıya özel akışları `mx/user/<sub>/mx/<konu>` sarmalıyla teslim ediyor;
  bağlayıcı sarmalı soyuyor (`userPool.ts:503-509,647-659`).

### 3.2 Kullanıcı oturumu ile sunucu servisi aynı anda bağlanabilir mi

- **Koddan anlaşılan:** kural "aynı kimlik + aynı uç nokta" düzeyinde. Kullanıcı belirtecinin
  `sub`'ı kullanıcıya (`matriks_sub`), servis belirtecininki `hardwareId`'ye bağlı. Farklı
  kimlikler olduğu için kullanıcı ve servis aynı uç noktaya aynı anda bağlanabilir olmalı.
  `matriks-user-adapter` ve `matriks-adapter` aynı kümede birlikte konuşlanıyor ve ikisi de
  `market` uç noktasına bağlanıyor (deploy formları); kodda bunun çakıştığına dair bir not yok.
- **Anlaşılmayan:** aynı kullanıcının mqlaw üzerinden bu bağlayıcıyla ve aynı anda kendi
  masaüstü/mobil Matriks uygulamasıyla aynı uç noktaya bağlanması durumunda hangisinin
  düşeceği. index.ts'teki "mqlaw doğrudan bağlanmaz, multi-connect sorunu kökten çözülür"
  notu, daha önce bir kullanıcı adına iki bağlantı açılıp birbirini düşürdüğünü
  düşündürüyor (varsayım). Kime sorulur: Matriks veri ekibi (kullanıcı oturumu ile istemci
  uygulaması aynı uç noktada çakışır mı?) ve Quantex ekibi (mqlaw geçmişi).

---

## 4. Abonelik modeli

| Konu | Koddan | Kanıt |
|---|---|---|
| Biçim | `mx/<tip>/<SYM>[@lvl2\|@lvl3]`; yayın konuları eksiz: `mx/timestamp`, `mx/news@<kaynak>`, `mx/stats/<ölçü>` | 2.2, 2.3 |
| Sembol başına mı joker mi | Çoğunlukla **sembol başına**. Joker yalnız kullanıcı bağlayıcısında `mx/pgc/+`; aracı `mx/ratingscore/+` jokerini reddediyor (`granted=128`) | `topicPlan.ts:71-78,143-150` |
| Sembol tipine göre konu | Hisse, endeks, döviz, YF, hak, kripto, emtia, tahvil, libor → `mx/symbol/…@lvl2`; varant, vadeli, opsiyon, sertifika → `mx/derivative/…@lvl2`. Kurallar `symbol-service`'in `/v1/topic-rules` yanıtından (Matriks "topach" API'sinden türetilmiş), yoksa koddaki varsayılanlar; borsa/alt pazar/sembol kodu istisnaları destekleniyor | `services/quantex-stream/matriks-adapter/src/services/topic-resolver.service.ts:1-12,79-143,177-237` |
| Sembol listesi | `matriks-adapter`: `symbol-service` `/v1/symbols/bist-stocks` (BIST hisseleri) ya da `SYMBOLS_TO_SUBSCRIBE`; sonradan ağ geçidinin NATS `streamx.symbol.refresh` isteğiyle tek sembol ekleniyor (tip her zaman `S` varsayılıyor) | `services/quantex-stream/matriks-adapter/src/index.ts:171-218`, `services/quantex-stream/matriks-adapter/src/clients/symbol-service.client.ts:38-39`, `services/quantex-stream/matriks-adapter/src/services/nats-publisher.service.ts:211-267` |
| | C++ göstergeler: istek geldikçe; sembol başına referans sayımı, ilk hesap gelince abone, son hesap gidince abonelikten çık | `services/quantex_indicators_price_cpp/src/services/mqtt_manager.cpp:380-436` |
| | Anomali işçisi: koordinatörün Redis üzerinden verdiği bölüm (partition); bağımsız modda `WATCH_SYMBOLS` | `services/quantex-anomaly-engine/src/main.cpp:450,491-538,616-640` |
| Parti / hız | `matriks-adapter`: tek tek, sırayla ("Matriks toplu aboneliği düzgün desteklemeyebilir"); anomali: 20'lik partiler, partiler arası 1000 ms (`MQTT_SUBSCRIBE_BATCH_SIZE` 1–200, `MQTT_SUBSCRIBE_BATCH_DELAY_MS`); bağlantı koparsa parti 3 s bekleyip sürüyor | `mqtt-client.service.ts:224-261`, `services/quantex-anomaly-engine/src/config/config.hpp:96-97,304-305,447-448`, `services/quantex-anomaly-engine/src/services/mqtt_manager.hpp:879-967` |
| Ret | SUBACK'te `granted=128` → ret (lisans ya da joker) | `userPool.ts:295-297,481-485`, `mqtt-client.service.ts:248-255` |
| Sınır | Bağlantı başına en çok abonelik sayısı, saniyelik abonelik hızı **kodda yok**. Bilinmiyor — kime sorulur: Matriks veri ekibi | — |
| `BUCKET_START/END` | `matriks-adapter` ortamında var ama MQTT tarafında kullanılmıyor; yayın tarafında 32 kova (NATS) | `env.ts:74-75`, `nats-publisher.service.ts:17,848` |

İlk durum ve güncelleme ayrımına dair bağlantı tarafından görülen: akış servisinin yorumuna
göre bir aboneliğin **ilk görüntüsü** kullanıcıya özel yansı konusundan
(`mx/user/<sub>/mx/pgc/<SYM>`), sonraki **canlı değişiklikler** düz konudan geliyor
(`services/quantex_indicators_flow_cpp/src/services/mqtt_manager.cpp:520-529`). Kullanıcı
bağlayıcısı da aynı sarmalı görüyor (`userPool.ts:503-509`). Ayrıntı T2'de.

---

## 5. Borsa saati ve açıklık (`market_clock`)

Üç C++ göstergede `market_clock` dosyaları birebir aynı (karşılaştırıldı); anomali motorunda
başlık-yalnız ayrı bir sürüm var.

- **Kaynak 1 — MQTT `mx/timestamp`** (`market` uç noktası). Her C++ servis bağlanır
  bağlanmaz bu konuya abone oluyor, yeniden bağlanınca da yeniliyor
  (`services/quantex_indicators_price_cpp/src/services/mqtt_manager.cpp:123-129,475-481`).
  İleti protobuf `TimeMessage`: `timestamp` (int64, yorumda mikrosaniye), `source`
  (`"MX"`, `"BISTPP"` …), `isBistPPHoliday`, `isBistPPOpen`
  (`services/quantex-stream/matriks-adapter/src/proto/Timestamp.proto:7-12`,
  `services/quantex_indicators_price_cpp/src/services/market_clock.hpp:81-92`).
  - Zaman damgası her kaynaktan alınıp saklanıyor (`market_clock.cpp:121-124`).
  - **Açık/kapalı yalnız `MX` kaynağından** belirleniyor; gerekçe yorumda: `BISTPP` iletileri
    eski/çelişkili `isBistPPOpen` taşıyıp açık/kapalı arasında hızlı gidip gelmeye yol açıyor
    (`services/quantex_indicators_price_cpp/src/services/market_clock.cpp:126-133`).
    Anomali motoru farkı: `MX` gelmeden önce `BISTPP`'yi ilk kaynak olarak kabul ediyor,
    öteki kaynakları (`BISTEX`, `BISTVIOP`) yok sayıyor
    (`services/quantex-anomaly-engine/src/services/market_clock.hpp:137-144`).
  - Açılış/kapanış geçişinde geri çağrılar tetikleniyor; tatil bayrağı saklanıyor
    (`market_clock.cpp:139-170`).
- **Kaynak 2 — seans saatleri REST'i:** Matriks veri API'sinin seans saatleri yolu
  (`MATRIKS_DATA_API_URL` + session-hours), başlık `Authorization: jwt <belirteç>` (aynı
  servis belirteci). Yanıt `"BISTPP": [açılış1, kapanış1, açılış2, kapanış2]` biçiminde
  `"HH:MM"` dizgileri (`services/quantex_indicators_price_cpp/src/services/market_clock.cpp:267-272`,
  `services/quantex_indicators_price_cpp/src/services/market_clock.hpp:96-103`). Açılışta
  bir kez çekiliyor (`services/quantex_indicators_price_cpp/src/main.cpp:136-145`);
  `SESSION_HOURS_REFRESH_INTERVAL_MINUTES` ayarı tanımlı ama kaynakta kullanıldığı yer yok
  (`config.hpp:105,242`; başka kullanım bulunamadı).
  Yorumdaki yol ile koddaki yol arasında önek farkı var (başlık yorumu ek bir önek içeriyor);
  önekin `MATRIKS_DATA_API_URL` içinde olduğu varsayılıyor.
- **Öncelik:** en az bir `MX` iletisi geldiyse karar ondan; gelmediyse seans saatleri; o da
  yoksa "kapalı" (`market_clock.cpp:25-63`).
- **Saat dilimi:** seans saatleri `localtime_r` ile kapsayıcının yerel saatine göre
  yorumlanıyor (`market_clock.cpp:39-42,317`). C++ Dockerfile'larında `TZ` ayarı bulunamadı.
  Varsayım: kapsayıcılar UTC'de çalışıyorsa yedek yol (seans saatleri) 3 saat kaymalı karar
  verir. Birincil yol (`mx/timestamp`) bundan etkilenmiyor. Türkiye saatinin `timestamp`
  alanında nasıl ifade edildiği (UTC epoch mu, yerel mi) T2'nin konusu.
- `matriks-adapter` `mx/timestamp`'ı yalnız `DATA_TYPES` içinde `timestamp` varsa abone
  ediyor (`mqtt-client.service.ts:23`); kullanıcı bağlayıcısı her kullanıcının `market`
  bağlantısında abone ediyor (`topicPlan.ts:66-70`).

---

## 6. Arkin'in bilgileriyle karşılaştırma

| # | Bilgi | Hüküm | Kaynak |
|---|---|---|---|
| 1 | Belirteç alma yöntemi sisteme özel; Quantex belirteci yalnız o sistemin kimliğini kurar | **Kısmen doğrulandı.** Belirteç bir servis hesabı + `typeCode` + `hardwareId` (+ C++'ta `deviceSourceId`) ile alınıyor; kimlik `hardwareId`'ye (pod) bağlı, yani "sisteme" değil **örneğe** özel. Belirtecin başka bir sistemde kullanılamayacağı (hesap ya da `typeCode` sisteme kayıtlı mı) koddan anlaşılmıyor. | `services/quantex-stream/matriks-adapter/src/services/matriks-auth.service.ts:73-81`, `services/quantex-stream/matriks-adapter/src/config/env.ts:120-132`, `services/quantex_indicators_price_cpp/src/services/token_service.cpp:180-190`, `services/quantex_indicators_flow_cpp/src/services/mqtt_manager.cpp:520-521` |
| 2 | Yayın tipi başına bir bağlantı; abonelikler onun üzerinden; aynı uç noktaya ikinci bağlantı birincisini düşürür | **Doğrulandı** (kodda açık yorum ve sunucu bildirimi). Bağlantılar uç nokta yoluna göre ayrılıyor; her servis her uç noktaya tek bağlantı açıp sembolleri onun üzerinden abone ediyor. Kural **uç nokta + kimlik** düzeyinde; istemci kimliği çakışması değil. Matriks bunu `mx/messenger` üzerinden `DISCONNECT`/`MULTICONNECT` iletisiyle bildiriyor. "Yanlış bir şey yapıyoruz" uyarısına uyan yerler: `matriks-adapter`'in kopyalar arasında Redis'te belirteç paylaşması ve anomali işçilerinin ortak `MATRIKS_SERVICE_HARDWARE_ID` alma olasılığı (2.5). | `services/quantex-stream/matriks-user-adapter/src/services/userPool.ts:5-7`, `services/quantex-stream/matriks-user-adapter/src/index.ts:4-7`, `services/quantex-anomaly-engine/src/main.cpp:1792-1793`, `services/quantex-anomaly-engine/src/services/mqtt_manager.hpp:774-777` |
| 4 | Borsa açıklığı zaman damgasıyla gelir; bu bağlantı Türkiye saatini ve açıklık durumunu verir | **Bağlantı tarafı doğrulandı.** `market` uç noktasındaki `mx/timestamp` konusu zaman damgası + kaynak + `isBistPPOpen` + `isBistPPHoliday` taşıyor; açıklık yalnız `MX` kaynağından alınıyor. "Türkiye saati" ifadesinin alanın içeriği mi (yerel saat) yoksa yalnız borsa saati anlamında mı olduğu T2'de. Ek olarak seans saatleri REST'ten geliyor. | `services/quantex-stream/matriks-adapter/src/proto/Timestamp.proto:7-12`, `services/quantex_indicators_price_cpp/src/services/market_clock.cpp:107-170` |
| 5 | Gösterge servislerine abonelikle dinlenir; anomali motoru anomali yakalar; hepsi NATS üzerinden yayınlar | **Bağlantı tarafından not:** C++ servisler sonuçlarını ve komutlarını Matriks'e değil **yerel bir MQTT aracısına** (`CLIENT_MQTT_*`) bağlanarak alıp yayınlıyor; komut konuları `quantex/<tip>/{indicator,signal,pattern}/cmd` ve `quantex/engine/<id>/…`; anomali verisi `quantex/anomaly/data/#`. NATS'a geçiş ayrı bağlayıcılarda (`*-indicator-adapter`, `anomaly-adapter`); bu T3'ün konusu. | `services/quantex_indicators_price_cpp/src/services/mqtt_manager.cpp:151-238,363-372`, `services/quantex-anomaly-engine/src/services/mqtt_manager.hpp:43-53` |

---

## 7. Açık sorular

| # | Soru | Kime |
|---|---|---|
| 1 | Çoklu bağlantı kuralının anahtarı tam olarak ne: belirteç dizgisi mi, JWT `sub` mi, `hardwareId` mi, servis hesabı mı? Aynı `hardwareId` ile yeni alınmış ikinci belirteç de eskisini düşürür mü? | Matriks veri ekibi |
| 2 | Servis belirtecinin `sub`'ı `QTX-<hardwareId>` mi? `QTX` öneki Quantex'e mi özgü (hesaba mı bağlı)? | Matriks veri ekibi |
| 3 | Qapsule için yeni servis hesabı / `typeCode` / `deviceSourceId` gerekir mi, yoksa Quantex hesabıyla farklı `hardwareId` kullanmak kabul edilir mi? Lisans (`licenses`) kapsamı ne olur? | Matriks veri ekibi + Arkin |
| 4 | Süresi dolan belirteçle açık MQTT bağlantısı kesiliyor mu, yoksa yalnız yeni bağlantıda mı denetleniyor? | Matriks veri ekibi |
| 5 | Bağlantı başına abonelik sayısı ya da abonelik hızı sınırı var mı? Toplu (çok konulu) SUBSCRIBE destekleniyor mu (anomali 20'lik parti kullanıyor, bağlayıcı tek tek)? | Matriks veri ekibi |
| 6 | `mx/symbol/<SYM>` ile `mx/symbol/<SYM>@lvl2` (ve `trade`, `depth` için aynı fark) aynı akışı mı veriyor? | Matriks veri ekibi |
| 7 | Bir kullanıcının kendi Matriks uygulaması ile `matriks-user-adapter` bağlantısı aynı uç noktada çakışır mı? | Matriks veri ekibi; geçmiş için Quantex ekibi (mqlaw) |
| 8 | Üretimde `MATRIKS_SERVICE_HARDWARE_ID` / `MATRIKS_HARDWARE_ID` ortam dosyalarında dolu mu? Doluysa anomali işçileri aynı değeri mi alıyor? Kaç anomali işçisi çalışıyor? | Quantex ekibi |
| 9 | `matriks-adapter` üretimde hangi `DATA_TYPES` ile ve hangi kipte (`realtime`/`delayed`) çalışıyor? Kopya sayısı gerçekten 1 mi (form "leader election" diyor, kod tek örnek)? | Quantex ekibi |
| 10 | Anomali motorunun `mx/messenger`'ı yerel aracıdan dinlemesi bilinçli mi (bir köprü mü var), yoksa MULTICONNECT uyarısı hiç görülmüyor mu? | Quantex ekibi |
| 11 | `matriks-adapter`'in her `reconnect` olayında yeni belirteç istemesi (1 s aralık, geri çekilme yok) belirteç servisinde sorun çıkarıyor mu? mqtt.js `options.password` güncellemesi gerçekten etkili mi? | Quantex ekibi |
| 12 | C++ kapsayıcıları hangi saat diliminde çalışıyor (seans saatleri yedek yolu için)? | Quantex ekibi |
| 13 | Qapsule'ün Matriks'e geliştirme ortamından bağlanması için (çalışan sistemi etkilemeden) ayrı test kimliği / test sunucusu (RT/DL ayrımı) verilebilir mi? | Arkin + Matriks veri ekibi |

---

## Ek: incelemede görülen gömülü değerler (yalnız yer)

Bunlar Matriks bağlantısıyla ilgili değil; kural gereği yalnız yerleri yazıldı, değerleri
okunup aktarılmadı:

- `services/quantex-stream/docker/docker-compose.scale-test.yml:78,81,112,113` — `JWT_SECRET` ve
  `HMAC_SECRET` için gömülü değer (yük testi yığını).
- `services/quantex-stream/docker/docker-compose.scale-test.yml:159` — Grafana yönetici parolası
  gömülü.
- `services/quantex-stream/matriks-user-adapter/src/services/userPool.ts:652-654` — yorumdaki örnek
  konularda sayısal bir Matriks kullanıcı kimliği (`sub`) var; gerçek bir kullanıcıya ait
  olabilir.
- Kullanıcı adı (parolasız) gömülü: `services/quantex-anomaly-engine/docker-compose.yml:20`,
  `services/quantex-stream/docker/docker-compose.fullstack.yml:34,64`,
  `services/quantex-stream/docker/docker-compose.services.yml:24,52`.

Matriks servis hesabı, belirteç ve MQTT için kodda ya da dağıtım tanımlarında gömülü gizli
değer görülmedi; hepsi ortam değişkeni / k8s Secret üzerinden geliyor
(`services/quantex-stream/k8s/matriks-adapter-deploy.yml:72-76`,
`services/quantex_indicators_price_cpp/cpp-indicator-deploy.yml:56-68`).
