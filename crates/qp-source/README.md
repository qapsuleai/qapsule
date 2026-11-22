# qp-source

**Seri kaynağı ve veri düzlemi** (`SeriesSource` = "Seri kaynağı", `docs/CONCEPTS.md`).
Bir bar serisini `BarEvent` akışı olarak verir (yeni bar ya da son barın düzeltmesi);
tüketici kaynağın türünü görmez. Veri düzlemi (RP2, K23; `docs/PROTOCOL.md` §18): alan
düzeyinde `DataEvent`'leri §18.4 kurallarıyla uygulayan `FeedStore`, pencere çözümü,
sağlayıcı (`SourceProvider`, `MemoryProvider`). Ayrıca sentetik piyasa ve akış (R3),
yeniden oynatma (`Replay`, R4/K44) ve fiyat kartından canlı bar kurma (`LiveBars`, R6).

**Katman:** `docs/ENGINEERING.md` §2'de uyarlayıcı; içeriği veri düzlemi; istisna isteği İ1
(`rounds/RG1/PLAN.md` §3.1: alan katmanına alınması önerisi). Dış dünyaya bakan kısım
yalnız dosya okuma (`file.rs`, `local.rs`: `fixtures/local/bars/`).

**Bağımlılıklar:** `qp-calendar`, `qp-schema`, `serde_json`. Geliştirme: `qp-compute`.
Bunu kullananlar: `qp-session`, `qp-matriks`, `qp-shellcore` ve kabuklar.

**Dışa açık yüzey (`lib.rs`):**

- `SeriesSource`, `BarEvent`, `MemorySource`, `FileSource`, `open_binding`, `collect_bars`.
- `FeedStore`, `Applied`, `EventError`, `resolve_window`, `SyntheticEvents`.
- `SourceProvider`, `MemoryProvider`, `bind`, `validate_binding`, `validate_entitlement`.
- `synthetic_series`, `synthetic_market`, `synthetic_flow`, `load_local_bars`.
- `Replay`, `ReplayFeed`, `ReplayCursor`, `local_replay_feeds`, `synthetic_replay_feeds`.
- `LiveBars`, `CardTick`, `TradeTick`, `compare_bars`, `BarDiff`.

**Düzen (`src/`):** `livebar/` — canlı bar kurucusunun parçaları (`push.rs` kart ve işlem
tıkları, `compare.rs` REST karşılaştırması); geri kalanı kökte konu başına bir dosya.

**Ne değildir:** ağa çıkmaz (canlı bağlantı `qp-live`, Matriks çevirisi `qp-matriks`'tedir);
otorite değildir; `qp_schema::SeriesSource` bir bağın *betimidir*, buradaki aynı adlı trait
veriyi üreten çalışma zamanı tarafıdır.

**Bilinen borç:** `qp-session → qp-source` bugün `layer.up`, `qp-matriks → qp-source` ve
`qp-shellcore → qp-source` `layer.lateral` olarak raporlanır; İ1 kabul edilirse üçü de
aşağı bağımlılık olur.
