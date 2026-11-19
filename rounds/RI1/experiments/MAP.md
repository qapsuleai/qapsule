# RI1/T0 — `quantex` deposunun haritası

Kaynak: `/Users/arkns/LocalGit/quantex` (dal `main`, inceleme anındaki HEAD `f3d95aea`).
Yalnız dosya listeleme ve okuma ile çıkarıldı; o depoda derleme ya da betik çalıştırılmadı.
Round başındaki `git status --porcelain` çıktısı: `quantex_status_before.txt` (3 satır;
iki `.env` dosyası değişmiş görünüyor, bir klasör izlenmiyor — hepsi round öncesinden;
`.env` içerikleri açılmadı).

## Diller ve derleme

| Parça | Dil | Derleme | Not |
|---|---|---|---|
| `services/quantex-stream/*` (bağlayıcılar, ağ geçidi, sembol servisi) | TypeScript (Node.js 20) | npm, her servis kendi `package.json`'ı; `shared/` ortak kitaplık | Her birinde `Dockerfile`; `k8s/` altında dağıtım tanımları |
| `services/quantex_indicators_price_cpp` | C++ | CMake + Conan; Paho MQTT C++, nlohmann_json, redis++, CURL, spdlog, prometheus-cpp, jwt-cpp (`CMakeLists.txt`) | `README.md`, `CHANGELOG.md`, `DEVAM_DOKUMANI.md`; `test-ui/` (Next.js) |
| `services/quantex_indicators_flow_cpp` | C++ | CMake | Belge yok |
| `services/quantex_indicators_depth_cpp` | C++ | CMake | Belge yok |
| `services/quantex-anomaly-engine` | C++ (motor) + TypeScript (`api/`, `lesson-learner/`) + SQL (`migrations/`) | CMake + Conan | `config/indicator_config.json`, `dashboard/`, `bench/` |

## Servisler ve ilk bakışta ilişkiler (alt görevlerde doğrulanacak)

- `quantex-stream/matriks-adapter`: Matriks belirteci (`services/matriks-auth.service.ts`),
  MQTT istemcisi (`mqtt-client.service.ts`), konu çözümü (`topic-resolver.service.ts`),
  protobuf çözümü (`protobuf.service.ts`, 22 Matriks protosu `src/proto/`), NATS'a yayın
  (`nats-publisher.service.ts`), sembol servisi istemcisi.
- `quantex-stream/matriks-user-adapter`: kullanıcı başına havuz (`userPool.ts`),
  konu planı, canlı fiyat denetimi, Redis kaydı, zarf (`envelope.ts`). Kullanıcı oturumu ile
  servis bağlantısının ilişkisi sorusu için önemli (A).
- Dört gösterge/anomali bağlayıcısı (`price-`, `flow-`, `depth-indicator-adapter`,
  `anomaly-adapter`): her birinde `mqtt/mqtt.subscriber.ts` + `nats/nats.publisher.ts` +
  `transform/*.transform.ts`; gösterge bağlayıcılarında ayrıca `nats.cmd-listener.ts`,
  `nats.signal-cmd-listener.ts`, `engine-registry.ts` (NATS'tan gelen abonelik komutlarını
  motorlara iletme olasılığı).
- Üç C++ gösterge servisi her biri kendi `services/token_service`, `matriks_client`,
  `mqtt_manager`, `market_clock`, `redis_service` dosyalarını taşıyor: **motorlar Matriks'e
  kendileri bağlanıyor olabilir** (bağlantı sayısı sorusu için kritik). Sonuçlarını bir MQTT
  aracısı (`mosquitto/` yapılandırması var) üzerinden yayınlayıp bağlayıcıların NATS'a
  geçirmesi olası.
- `quantex-anomaly-engine`: `ingestion/` (trade, depth, pgc, news, booklet, indicator),
  `coordinator/` (bölümleme, meta), `state/`, `dispatch/` (sinyal ve rejim yayını),
  `migrations/` (anomali tabloları: geçmiş kaydı olasılığı), `lesson-learner/`.
- `quantex-stream/shared/proto/`: iç yük protoları (`envelope`, `indicator`, `anomaly`,
  `flow`, `depth`, `market`…). `quantex-stream/protos/market.proto`.
- `quantex-stream/gateway`, `symbol-service`, `sync-service`: istemciye WebSocket, sembol
  kaydı, eşitleme.
- Dağıtım: `quantex-stream/k8s/*.yml` (güncel) ve `k8s/_legacy/`; `docker/` (yerel yığın:
  NATS, Redis, PostgreSQL). README "NATS Core + JetStream" diyor (doğrulanacak).

## İnceleme dışı bırakılanlar ve tehlikeli içerik

Okunmayacak / alıntılanmayacak: her klasördeki `.env`, `.env.local`, `.env.example`
(örnek de olsa değer taşıyabilir), `logs/` (gerçek veri ve belirteç içerebilir), `build/`,
`dist/`, `node_modules/`, `.next/`, `*.sst` / `*.meta` (gömülü veritabanı dosyaları; gerçek
veri), `*.tar.gz` arşivler. `quantex_indicators_crypto_cpp` ve öteki servisler kapsam dışı.

## Alt görev dağılımı

| Görev | Alt ajan | Belge |
|---|---|---|
| T1 Bağlantı | ayrı | `docs/matriks/CONNECTIVITY.md` |
| T2 Mesajlar | ayrı | `docs/matriks/MESSAGES.md` |
| T3 NATS | ayrı | `docs/matriks/NATS.md` |
| T4a/b/c Göstergeler | üç ayrı | `rounds/RI1/experiments/INDICATORS_{price,flow,depth}.md` → ana akış `docs/matriks/INDICATORS.md` içinde birleştirdi (parça dosyaları birleşimden sonra silindi) |
| T5 Anomali | ayrı | `docs/matriks/ANOMALY.md` |
| T6 Bütünleştirme | ana akış (T1–T5 sonrası) | `docs/matriks/INTEGRATION.md` |
| A Denetim | ayrı, bağımsız | `rounds/RI1/AUDIT.md` |
