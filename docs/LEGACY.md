# Eski Prototip Envanteri

Durum: R0 / T4. Kod taşınmadı; bu belge yalnızca envanterdir.

Eski prototipin adı **Q-Charts**'tır. İki yerde yaşar ve ikisi de salt okunurdur:

| Kök | İçerik |
|-----|--------|
| `../Services/quantex-charts-core` | Rust çekirdek (`core/`), WASM bağlaması (`wasm/`), TS kütüphanesi (`ts/`), demo (`demo/`) |
| `../Docs/q-charts` | Tasarım belgeleri, ADR'ler, MVP planları, Matriks API referansı |

Bu belgedeki yollar, aksi yazılmadıkça `../Services/quantex-charts-core` köküne görelidir.
`target/`, `node_modules/`, `dist/`, `pkg/` sayılmadı.

## 1. Genel yapı

Kod okunarak çıkarılan akış:

```
Matriks REST/MQTT ──► TS provider (ts/src/providers/matriks)
                         │  ticks, bars
                         ▼
                   TS DataManager ──► WASM ChartEngine (wasm/) ──► qcharts-core (core/)
                                            │ buildScene(viewport)        │ TimeSeries, BarAggregator,
                                            ▼                             │ indicators, Viewport/LOD,
                   TS pipeline + WebGL + Canvas2D overlay                 │ VertexBuilder, HitTester
                   TS interaction (pan/zoom/crosshair)
```

| Parça | Yol | Satır (yaklaşık) | Not |
|-------|-----|------------------|-----|
| Rust çekirdek | `core/src` | 6.760 | `data/` 2.476, `indicator/` 1.643, `rendering/` 1.311, `engine/` 1.212 |
| Çekirdek testleri | `core/tests` | 1.107 | `proptest_data.rs`, `proptest_indicators.rs`, `integration_engine.rs` (8 test) |
| Çekirdek birim testleri | `core/src/**` | — | 178 `#[test]` |
| Benchmark | `core/benches` | 434 | Criterion: `indicators.rs`, `timeseries.rs` |
| WASM bağlaması | `wasm/src` | 1.039 (+241 test) | `WasmChartEngine`, `buildScene`, `hit_test`, `feed_tick` |
| TS kütüphanesi | `ts/src` | 10.943 (2.687'si test) | `providers/` 3.647, `rendering/` 3.872, motor/veri yöneticileri |
| Demo | `demo/src` | 1.061 | Vite + WebGL, token'ı arayüzden alır |
| Belgeler | `../Docs/q-charts` | ~10.300 | 45 Markdown dosyası |
| Yapılandırma | kök | — | `Cargo.toml` (workspace: `core`, `wasm`), `rustfmt.toml`, `clippy.toml`, `deny.toml`, `.github/workflows/ci.yml` |

Önemli yapısal gözlemler:

- `core/src/data/ohlcv.rs`: `OHLCV` fiyatları **`f32`**, hacmi `f64`. Kuralımız fiyat ve
  hesap değerleri için `f64`'tür; tip olduğu gibi taşınamaz.
- Göstergeler (`core/src/indicator/*`) **toplu** hesaplar: `calculate(&[f64]) -> Vec<f64>`.
  Akışkan (her yeni bar O(1)) güncelleme yoktur. Çıktı dizisi kırpılmıştır (ısınma dönemi
  için `NaN` doldurma yok), hizalama çağırana kalır.
- Çizim çekirdekte yalnızca **hesap** olarak vardır (viewport, LOD, vertex tamponu); asıl
  çizim, etkileşim, eksen etiketleri, paneller, açıklamalar ve veri bağlantısı TS'tedir.
- Doküman modeli, komut yolu, olgu/bulgu kavramı yoktur (bkz. `DECISIONS.md` K8).
- Eski depo bu dizinde git deposu değildir (`.git` yok); geçmiş incelenemedi.

## 2. Taşınacaklar

"Taşınmak" kodu kopyalamak değil, yeni crate'te yeniden yazarken kaynak/referans olarak
kullanmak demektir. Her taşıma `f64`, determinizm ve "kabuklar incedir" kurallarına göre
yeniden biçimlenir.

| Eski kaynak | Hedef | Round | Neden / nasıl |
|-------------|-------|-------|---------------|
| `core/src/indicator/sma.rs` | `qp-compute` | R1 | Kayan pencere toplamı doğru ve hızlı. Akışkan `Study` olarak yeniden yazılır. |
| `core/src/indicator/ema.rs` | `qp-compute` | R1 | SMA ile tohumlanan EMA (`2/(n+1)`). Tohum kuralı aynen korunmalı; akışkan hale getirilir. |
| `core/src/indicator/rsi.rs` | `qp-compute` | R1 | Wilder yumuşatması; `avg_loss == 0` kenar durumları (50/100) tanımlı. StockCharts referans testi var. |
| `core/src/indicator/macd.rs` | `qp-compute` | R1 | Çok çıktılı (`macd_line`, `signal_line`, `histogram`); çok çıktılı `Study` için iyi örnek. Hizalama ofset mantığı korunmalı. |
| `core/src/indicator/bollinger.rs` | `qp-compute` | R1 | Popülasyon std sapması, `bandwidth` çıktısı. O(n·period); akışkan sürümde kayan varyansa çevrilmeli. |
| `core/src/indicator/mod.rs` (`Kind`, `create`, parametre çıkarma) | `qp-compute` + `qp-schema` | R1 | Parametre sözlüğü fikri parametre taraması için kullanılabilir; `Kind` enum'u şemaya `Study` tanımı olarak taşınır. |
| `core/tests/proptest_indicators.rs` | `qp-compute` testleri | R1 | Değişmezler: çıktı uzunluğu, sabit girdi kimliği, RSI ∈ [0,100], MACD histogram = macd − signal, Bollinger sıralaması. Akışkan = toplu eşitliği eklenmeli. |
| `core/src/data/aggregator.rs` | `qp-compute` | R1 | Tick → bar toplayıcı; deterministik, tick başına ayırma yok. `f64`'e çevrilir. |
| `core/src/data/resample.rs` | `qp-compute` | R1 | M1 → üst periyot yeniden örnekleme kuralları (O/H/L/C/V). Saat dilimi sorunu çözülerek (bkz. Riskler). |
| `core/src/data/mod.rs` (`Timeframe`, hizalama, gün↔tarih çevirisi) | `qp-schema` (tip) + `qp-compute` (hizalama) | R1 | Periyot tipi şemaya; `W1`/`MN1` takvim hizalaması hesaba. Belge yorumundaki ofset çelişkisi düzeltilmeli. |
| `core/src/data/timeseries.rs` | `qp-compute` | R1 | Sıralı bar deposu, `range_query`, `merge`. Fikir taşınır; `BTreeMap` dizini yeniden değerlendirilir. |
| `core/tests/proptest_data.rs` | `qp-compute` testleri | R1 | OHLCV, toplayıcı (hacim toplamı, tick sayısı), yeniden örnekleme (hacim korunumu, hizalama) değişmezleri. |
| `core/benches/indicators.rs`, `core/benches/timeseries.rs` | `qp-compute/benches` | R1 | Criterion düzeneği (1K/10K/100K) hazır kalıp. R1'in "1.000 parametre varyasyonu" ölçümüne genişletilir. |
| `BENCHMARK_BASELINE.md` | `rounds/R1` referansı | R1 | Sayılar aşağıda (bölüm 4). Yeni çekirdek bunlardan yavaş olmamalı. |
| `core/src/rendering/viewport.rs` | `qp-render` (veya `qp-grammar` ölçek) | R2 | İndeks tabanlı X ekseni (kapalı seans boşluğu yok), fiyat↔piksel, ters dönüşümler. Ölçek kavramının ilk hali. |
| `core/src/rendering/lod.rs` | `qp-render` | R2, R4 | Görünen bar sayısına göre ayrıntı seviyesi (200/1.000/5.000/10.000 eşikleri). R4 anlam zoom'unun ham ayağı. |
| `core/src/rendering/vertex.rs`, `primitives.rs`, `scene.rs` | `qp-render` | R2 | Mum/hacim/çizgi için vertex üretimi; R0 T3'teki çizim yığını kararına göre uyarlanır. |
| `core/src/rendering/hit_test.rs` | `qp-render` | R7 | O(1) indeks tabanlı bara yakalama, `snap_to_bar`. Etkileşim çekirdekte yaşayacağı için Rust'ta kalır. |
| `core/src/engine/mod.rs`, `engine/multi_tf.rs` | `qp-compute` (fikir) | R1, R6 | Geçmiş yükleme + canlı tick + otomatik yeniden hesap akışı; tek M1 akışından çoklu periyot. Akışkan çekirdeğin ve R6 canlı verinin tasarım girdisi. |
| `ts/src/providers/matriks/*` | Matriks bağlayıcısı (crate adı R6'da kararlaşır) | R6 | Protokol bilgisi `docs/MATRIKS_PROTOCOL.md`'ye çıkarıldı. Kod TS'te; veri mantığı çekirdekte olacağı için Rust'ta yeniden yazılır. |
| `ts/src/providers/matriks/proto/*.ts` | aynı bağlayıcı | R6 | Protobuf alan numaraları ve tipleri. Rust'ta `prost` ile `.proto` dosyalarından üretilmeli (eski ADR-005 de `prost` seçmişti). |
| `ts/src/providers/types.ts`, `registry.ts` (`HistoryProvider` / `RealtimeProvider` / `AuthProvider`) | `SeriesSource` arayüzü | R1 (arayüz), R6 (gerçekleme) | Geçmiş + canlı + kimlik ayrımı `SeriesSource` tasarımına girdi. |
| `ts/src/rendering/interaction.ts` (`applyZoomDelta`, `clampViewport`, `applyPanPixels`) | `qp-render` | R7 | İmleç çapalı zoom, min/max bar aralığı, kenar dolgusu formülleri. TS'ten Rust'a matematik olarak taşınır, olay işleme kabukta kalır. |
| `ts/src/rendering/chart/panels.ts` (`computePanelLayout`) | `qp-grammar` | R2 | Alt panel yerleşimi; gramerdeki "katman/panel" kavramına girdi. |
| `../Docs/q-charts/mvp/mvp1-core-foundation/indicators-spec.md` | `qp-compute` test vektörleri | R1 | Formüller ve TA-Lib / StockCharts referans değerleri. |
| `../Docs/q-charts/03-PERFORMANCE-REQUIREMENTS.md` | `rounds/R1`, `rounds/R2` hedefleri | R1, R2 | Hedef tabloları (ör. 10K bar SMA < 5 ms, kare < 16 ms). |
| `deny.toml`, `clippy.toml`, `rustfmt.toml` | kök yapılandırma | R0/R1 | T1 kendi ayarlarını kurdu; `cargo-deny` lisans/advisory listesi ileride eklenebilir. |

### Eski 5 indikatör

ROADMAP R1'deki "eski 5 indikatör" şunlardır (`core/src/indicator/mod.rs`, `Kind` enum'u):

| # | İndikatör | Dosya | Varsayılan | Çıktı |
|---|-----------|-------|------------|-------|
| 1 | SMA | `core/src/indicator/sma.rs` | period=20 | tek seri, uzunluk `n - period + 1` |
| 2 | EMA | `core/src/indicator/ema.rs` | period=20 | tek seri, SMA tohumlu, uzunluk `n - period + 1` |
| 3 | RSI | `core/src/indicator/rsi.rs` | period=14 | tek seri, Wilder, uzunluk `n - period` |
| 4 | MACD | `core/src/indicator/macd.rs` | 12, 26, 9 | `macd_line`, `signal_line`, `histogram` |
| 5 | Bollinger Bands | `core/src/indicator/bollinger.rs` | 20, 2σ | `upper`, `middle`, `lower`, `bandwidth` |

Varsayılanlar TS tarafında `ts/src/indicator-manager.ts` (`DEFAULT_PARAMS`, `PRESETS`) içinde de
tekrarlanır; yeni yapıda tek kaynak şema olmalı.

## 3. Taşınmayacaklar

| Eski kaynak | Neden |
|-------------|-------|
| `ts/src/rendering/backend/webgl/*`, `pipeline.ts`, `chart/index.ts` | Çizim TS/WebGL'de. Yeni kural: çizim çekirdekte (Rust), kabuk ince. Çizim yığını R0 T3'te ayrıca kararlaşıyor (A1). |
| `ts/src/rendering/annotations.ts` | Açıklama/işaret modeli ve çizimi TS'te, dokümana bağlı değil. Yerini `Glyph`, `View` ve doküman alır. Her işaret bir olguya bağlı olmalı; eski açıklamalar bağsız. |
| `ts/src/rendering/interaction.ts` (olay işleme kısmı) | Etkileşim mantığı TS'te ve doğrudan durumu değiştiriyor; komut yolu yok. Yalnızca matematik formülleri referans (bkz. Taşınacaklar). |
| `ts/src/engine.ts`, `data-manager.ts`, `indicator-manager.ts`, `events.ts` | Veri ve durum yönetimi TS'te; doküman modeli ve komut yolu yok (K8). Yeni yapıda bunlar `qp-doc` komutları ve çekirdek olaylarıdır. |
| `wasm/src/*` | API emir kipli (`add_indicator`, `feed_tick`, `build_scene`) ve şemasız. `qp-wasm` şemadan üretilen tiplerle, doküman/komut üzerinden bağlanır. Yalnızca `Float64Array` ile düz dizi aktarımı fikri akılda tutulur. |
| `wasm/src/engine.rs` `build_scene` otomatik sığdırma kuralı | `start_idx < 0` "otomatik sığdır" anlamına geliyor; ama viewport sol dolguda meşru olarak negatif olabiliyor (bkz. Riskler). Taşınmaz. |
| `core/src/data/compression.rs` | Gorilla benzeri sıkıştırma; şimdilik ihtiyaç yok. Fiyatlar `f32` üzerinden "toleranslı" geri dönüyor (`prop_compression_prices_tolerance`), "kayıpsız" iddiasıyla çelişiyor. Gerekirse R6'da yeniden değerlendirilir. |
| `core/src/error.rs` ve `QC-xxxx` hata kodları | Eski hata sözlüğü; yeni şema kendi hata tiplerini tanımlar. |
| `core/src/rendering/viewport.rs` "Legacy compat" yöntemleri (`time_to_x`, `px_per_second`, `visible_bar_count(_tf)`) | Kullanılmayan parametreli, zamana dayalı eski uyumluluk katmanı. |
| `demo/` | Prototip demo; token'ı metin kutusundan alıp `sessionStorage`'a yazıyor. Yeni test sayfası R0 T3'te ayrı kuruluyor. |
| `ts/src/__mocks__/wasm.ts` ve TS birim testleri | Taşınmayan TS koduna ait. Matriks testlerindeki senaryolar (boş hafta sonu, 9 periyot) R6'da Rust testine fikir olarak alınır. |
| `CHANGELOG.md`, `CONTRIBUTING.md` | Eski süreç belgeleri. Matriks protokol notları `docs/MATRIKS_PROTOCOL.md`'ye çıkarıldı. |
| `../Docs/q-charts/decisions/ADR-002-svelte-web-framework.md`, `analysis/ui-layer-design.md`, `mvp/mvp2-web-platform/canvas-renderer.md`, `typescript-api.md` | UI/çizim mantığını web katmanına koyan tasarım; Kuzey Yıldızı ve "kabuklar incedir" ile çelişir. |
| `../Docs/q-charts/analysis/product-vision.md`, `00-OVERVIEW.md`, MVP 3–5 belgeleri | "20 indikatör", "TradingView seviyesi" gibi hedefler; Kuzey Yıldızı'ndaki "Ne değiliz" ile çelişir. |
| `../Docs/q-charts/analysis/matriks-mqtt-integration.md` içindeki örnek kodlar | JSON `AUTH`/`SUBSCRIBE`/`PING` mesajları kodla doğrulanmış MQTT 3.1 protokolüyle çelişiyor (bkz. `MATRIKS_PROTOCOL.md` §10). |

## 4. Benchmark referansı

Kaynak: `BENCHMARK_BASELINE.md` (2026-02-05, Apple Silicon, `bench` profili, Criterion).
Tüm göstergeler toplu hesap, girdi `&[f64]` kapanış dizisi.

| Ölçüm | 1K | 10K | 100K |
|-------|----|-----|------|
| SMA(20) | 563 ns | 9,6 µs | 113 µs |
| EMA(20) | 1,33 µs | 14,1 µs | 141 µs |
| RSI(14) | — | — | ~150 µs (tahmini) |
| MACD(12,26,9) | — | — | ~350 µs (tahmini) |
| Bollinger(20, 2σ) | 4,83 µs | 54,9 µs | 567 µs |
| `TimeSeries.push` (sıralı) | 17,8 µs | 331 µs | 3,95 ms |
| `range_query` (100K içinden 1K) | — | — | 1,66 µs |
| M1 → M5 yeniden örnekleme | 3,21 µs | 44,2 µs | 716 µs |
| M1 → D1 yeniden örnekleme | 679 ns | 6,62 µs | 67,5 µs |

Notlar:

- RSI ve MACD 100K değerleri ölçüm değil, orantıyla tahmin. R1'de gerçek ölçüm alınmalı.
- 100K bar SMA ≈ 1,13 ns/eleman. R1'in "1.000 parametre varyasyonu" hedefi için kaba üst
  sınır: 1.000 × 113 µs ≈ 113 ms (tek çekirdek, 100K bar, SIMD/paralellik olmadan).
- Ölçüm makinesi ve Rust sürümü kayıtlı değil; birebir karşılaştırma yerine "aynı
  mertebede veya daha hızlı" ölçütü kullanılmalı.

## 5. Viewport, LOD ve hit-test matematiği

- **Viewport** (`core/src/rendering/viewport.rs`): X ekseni zaman değil **bar indeksi**;
  hafta sonu/seans boşluğu doğal olarak yok. Dönüşümler:
  `x = margin_left + (idx - start_idx + 0.5) · bar_width`,
  `y = margin_top + (price_high - price) · chart_height / price_span`, tersleri
  `x_to_index`, `y_to_price`. `fit_bars` %5 dikey dolgu ve yarım bar yatay dolgu ekler.
  Kenar boşlukları sabit (sağ 70, alt 30, üst 10, sol 5 px) ve TS'te
  (`ts/src/rendering/constants.ts`) **tekrar** tanımlı; yeni yapıda tek kaynak olmalı.
  `dpr` alanı tutulur ama dönüşümlerde kullanılmaz (CSS pikseli).
- **LOD** (`core/src/rendering/lod.rs`): `Full ≤ 200 < Simplified ≤ 1000 < Aggregated ≤ 5000
  < Line ≤ 10000 < Heatmap`. `Heatmap` tanımlı ama vertex üreticisinde karşılığı yok.
- **Hit-test** (`core/src/rendering/hit_test.rs`): `x_to_index` sonucunu yuvarlayıp
  `[0, n-1]`'e kırpar; O(1). Y ekseninde gösterge/çizim isabeti yok, yalnızca bar.
- **Zoom/pan** (`ts/src/rendering/interaction.ts`): İmleç oranı `ratio` ile çapalı zoom
  (`start -= Δ·ratio`, `end += Δ·(1-ratio)`), adım `±%8·zoomSpeed`; görünür bar sayısı
  `[minBars, min(maxBars, total + max(20, %15·total))]` aralığına kırpılır; kenar dolgusu
  `min(%10·span, 20)` bar.
- **Test kapsamı:** viewport 4, LOD 4, hit-test 2 birim testi var; dönüşümlerin
  gidiş-dönüş (`x_to_index ∘ index_to_x = id`) özellik testi **yok**. R2'de eklenmeli.

## 6. Riskler ve bilinen sorunlar

| # | Bulgu | Yer | Etki |
|---|-------|-----|------|
| L1 | Gerçek kimlik bilgisi içerdiği varsayılan bir `.env` dosyası var (açılmadı, okunmadı). Taşınmamalı, kopyalanmamalı. | `ts/.env` | Sızıntı riski |
| L2 | `.gitignore` içinde `.env` ve `.env.local` satırları **yorum satırı**; `.env` dışlanmıyor. | `.gitignore` | `ts/.env` commit edilebilir durumda |
| L3 | `.env.example` var (açılmadı). Gerekli değişken adı koddan çıkarıldı: `MATRIKS_TOKEN`. | `ts/.env.example`, `ts/src/providers/matriks/*.test.ts` | — |
| L4 | Demo token'ı tarayıcı `sessionStorage`'ına düz metin yazıyor. | `demo/src/main.ts` | Token XSS ile okunabilir |
| L5 | MQTT bağlantısında `rejectUnauthorized: false` (TLS sertifika doğrulaması kapalı; Node ortamında etkili). | `ts/src/providers/matriks/realtime.ts` (`buildMqttOptions`) | Ortadaki adam saldırısı |
| L6 | `keepalive: 10000`; mqtt.js'te birim **saniye**dir (≈ 2,8 saat). Muhtemelen milisaniye sanılmış. | aynı yer | Ölü bağlantı geç fark edilir |
| L7 | Yeniden bağlanmada eski MQTT istemcileri kapatılmadan yenisi açılıyor; tek-bağlantı kuralı nedeniyle eski/yeni istemci birbirini düşürebilir. | `realtime.ts` (`scheduleReconnect`) | Bağlantı döngüsü riski (çıkarım) |
| L8 | Fiyatlar `f32`. | `core/src/data/ohlcv.rs` | `f64` kuralına aykırı; BIST fiyatlarında hassasiyet kaybı |
| L9 | Periyot hizalaması UTC'ye göre (`D1`, `H4`, `W1`, `MN1`). BIST İstanbul saatiyle (UTC+3) çalışır; `H4` kovaları yerel 03/07/11/15/19'da başlar, Matriks'in `240min` türetilmiş barlarıyla örtüşmeyebilir. `W1` yorumunda "3 gün" ve "4 gün" ofseti çelişiyor (kod 4 gün kullanıyor). | `core/src/data/mod.rs` | Yerel hesap ile sunucu barları farklı çıkabilir |
| L10 | `MN1.seconds()` 30 gün yaklaşıklığı. | `core/src/data/mod.rs` | Süreye dayalı hesaplarda hata |
| L11 | Göstergeler akışkan değil; her yeni barda tüm dizi yeniden hesaplanıyor. | `core/src/indicator/*`, `engine/mod.rs` | R1 hedefiyle ("her yeni bar O(1)") uyumsuz |
| L12 | `build_scene` `start_idx < 0` değerini "otomatik sığdır" işareti sayıyor; TS kırpması sola `-leftPad` kadar negatif başlangıca izin veriyor. Sol kenara kaydırınca görünüm sıfırlanabilir. | `wasm/src/engine.rs`, `ts/src/rendering/interaction.ts` | Çıkarım; doğrulanmadı |
| L13 | REST tarih parametresi istemcinin **yerel** saat dilimiyle biçimleniyor (`getFullYear/getMonth/getDate`). | `ts/src/providers/matriks/history.ts` (`formatDate`) | Saat dilimine göre bir gün kayma |
| L14 | Viewport/hit-test için özellik testi yok; TS rendering katmanının (3.872 satır) birim testi yok. | `core/src/rendering`, `ts/src/rendering` | Regresyon riski |
| L15 | Tasarım belgelerindeki Matriks örnek kodu (JSON AUTH/SUBSCRIBE, PING) gerçek protokolle çelişiyor. | `../Docs/q-charts/analysis/matriks-mqtt-integration.md`, `reference/matriks-api/mqtt-topics.md` | Yanlış bilgiyle yazılmış bağlayıcı |
| L16 | Lisans alanı `PROPRIETARY`, yazar "Matriks". Kod Matriks mülkü; taşımada lisans/sahiplik netleşmeli. | `Cargo.toml`, `ts/package.json` | Hukuki |

Kodda (`.env*` dışında) gerçek token, parola veya API anahtarı **bulunmadı**.
`ts/src/providers/matriks/auth.test.ts` içindeki JWT biçimli dizgiler sahte test
verisidir (`iss: "TEST"`, imza `test_signature`). `.env*` dosyaları kurala uygun olarak
açılmadı.

## 7. Öneriler (kapsam dışı, sonraki round'lar için)

- **R1:** Beş indikatörü akışkan `Study` olarak yaz; her biri için "akışkan sonuç = eski toplu
  sonuç" özellik testi ekle (eski kod referans oracle olarak, testte sabit vektörlerle).
  Isınma dönemi hizalama kuralını (kırpma mı `NaN` mı) şemada karara bağla.
- **R1:** Periyot hizalaması için saat dilimi parametresi (`Europe/Istanbul`) kavramını
  sözlüğe ekleme ihtiyacını değerlendir.
- **R2:** Viewport dönüşümleri için gidiş-dönüş özellik testleri; kenar boşlukları tek kaynakta.
- **R6:** Matriks bağlayıcısı için crate adı ve konumu (`ARCHITECTURE.md` düzeninde yok);
  `.proto` dosyalarının asıl kaynağı (eski kod `/proto/*.proto`'ya atıf yapıyor ama dosyalar
  iki klasörde de yok) temin edilmeli.
- **R6:** Token alma/yenileme akışı (eski kodda yalnızca statik token var) ve tek-bağlantı
  kuralına uygun yeniden bağlanma tasarımı.
- **Hijyen:** Eski depoda `.gitignore` düzeltilmeli ve `ts/.env` içindeki token iptal
  edilip yenilenmeli (eski depo salt okunur olduğu için bu iş Arkin'e/sahibine bırakılır).
