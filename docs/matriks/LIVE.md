# Canlı Matriks bağlantısı (R6)

Qapsule kendi kimliğiyle doğrudan Matriks MQTT'ye bağlanır (K60). Bu belge bağlayıcının
nasıl çalıştığını, nasıl başlatıldığını ve sorun gidermeyi anlatır. Değerler, adresler ve
belirteçler burada yazılmaz; yalnız değişken adları (K61). Ölçümler ve kanıt:
`rounds/R6/REPORT.md`, `rounds/R6/experiments/LIVE-LOG.md`.

## 1. Parçalar

| Parça | Crate | Ağ | Ne yapar |
|---|---|---|---|
| Matriks çözücü | `qp-matriks` | yok | 22 Matriks protosunun bayt eşit kopyası (`proto/`, köken `proto/PROVENANCE.md`), derlemede üretilen prost tipleri (`protox` + `prost-build`, protoc gerekmez); fiyat kartı kitabı (`SymbolBook`, protobuf birleştirme anlamı: **gelmeyen alan değişmedi**, 0 yazılmaz); borsa saati (`MarketClock`, yalnız `MX` kaynağı); bildirim sınıflayıcı (`MULTICONNECT`); seans evresi tablosu (`stockStatus`); ham ileti kaydı (`rawlog`); ortak besleme hattı (`feed::LiveFeed`). |
| Canlı bar üretici | `qp-source::livebar` | yok | Fiyat kartından XIST takvimine hizalı bar (K14): açılış emir toplama günün ilk barına, kapanış seansı son barına; hacim günlük birikimli tutarın farkı; yöntemler `Ticks`, `CardDaily`, `Mixed`; ek yeri (`resume`); `compare_bars`. |
| Canlı bağlayıcı | `qp-live` | **var** | Belirteç (bellekte, istek sıklığı sınırlı), MQTT 3.1 (`MQIsdp`, WebSocket alt protokolü `mqttv3.1`) WebSocket + TLS (rustls + ring, sertifika doğrulaması açık) üstünde, kendi küçük MQTT kodlayıcısı; uç nokta başına tek bağlantı (süreç içi kayıt + ana ağacın `fixtures/local/live-<uç>.lock` işletim sistemi kilidi; ayrıca belirteç isteyen her süreç için makine çapında `live-identity.lock`); çoklu bağlantı bildiriminde dur ve yeniden deneme yok (`.halted` işaret dosyası); üstel geri çekilme (2→60 sn, 6 ardışık hatada dur); günlük maskeleme (`Redactor`); REST istemcisi (`rest::RestClient`: kesin barlar ve sembol listesi, aynı servis belirteciyle); sahte sunucu (`mock` özelliği). |
| CLI ve gösterim | `qp-cli` | var | `qp live record|inspect|symbols|bars|compare`, `qp replay --raw`, `qp demo --live`. |

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

## 6. Sorun giderme

| Belirti | Neden / ne yapılır |
|---|---|
| `… another qp process holds the Matriks identity` | Belirteç isteyen başka bir `qp` süreci (record, symbols, bars, demo --live) çalışıyor; kimlik kilidi `fixtures/local/live-identity.lock` (ana ağaç). Aynı kimlikle eşzamanlı belirteç istenmez. |
| `lock: endpoint … is locked by another process (pid N)` | Aynı makinede başka bir Qapsule süreci bağlı. O süreci kapatın; ikinci bağlantı açmayın. |
| `HALTED (multi-connect)` | Aynı kimlikle başka bir oturum aynı uç noktaya bağlandı. Yeniden denemeyin; Arkin'e bildirin. `fixtures/local/live-<uç>.halted` dosyası notla sıfırlanana kadar bağlantı açılmaz. |
| `subscription refused mx/timestamp (code 0x80)` | Gecikmeli uçta borsa saati konusu yok (ölçüldü). Saat `updateDate`'ten ve takvimden kurulur. |
| `subscription refused mx/symbol/<SYM>@lvl2` | Sembol işlem görmüyor ya da yetki yok. |
| `retrying in …` | Kopma; ölçülü geri çekilme. 6 ardışık hatada durur (`TooManyFailures`). |
| `token request failed: HTTP 401` | Servis hesabı değişkenleri yanlış. |
| `missing: MATRIKS_…` | `.env.local`'a o değişkeni girin. |
