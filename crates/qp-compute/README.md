# qp-compute

**Hesap çekirdeği** (`Study` = "Hesap", `docs/CONCEPTS.md`). Her hesap girdi sütunlarından
adlı çıktı sütunlarına belirlenimci bir işlevdir: yalnız `f64`; saat, ağ, rastgelelik ya da
model yok; çıktı girdiyle aynı uzunlukta (K13); toplu (`run`), akış (`Study::push`) ve "son
barı düzelt" (`Study::update_last`) kipleri bit eşittir; aarch64, x86_64 ve wasm32 aynı
bitleri verir (`examples/digest.rs`).

**Katman:** çekirdek (`docs/ENGINEERING.md` §2).

**Bağımlılıklar:** `qp-schema`. Geliştirme: `proptest`, `serde_json`.

**Dışa açık yüzey (`lib.rs`):**

- `Study`, `run`, `Core`, `Streaming` — hesap sözleşmesi ve bekleyen bar modeli.
- `create`, `compute_bars`, `default_params`, `STUDY_NAMES`, `MAX_PERIOD` — kayıt defteri.
- Yerleşik hesaplar: `Sma`, `Ema`, `Rsi`, `Macd`, `Bollinger`, `Atr`, `Highest`, `Lowest`;
  RL1 hesap dili çekirdekleri `Stdev`, `PercentRank` (`add_study`'de yok, özeti
  `sweep::rl1_studies_digest`).
- `Address`, `Chain`, `Field`, `ComputeError`; modüller `graph`, `late`, `par`, `sweep`.

**Düzen (`src/`):**

- `studies/` — yerleşik hesaplar, her biri bir `Core`; dönem dönüşümü (`period.rs`).
- `sweep/` — taramada paylaşılan alt hesaplar (`share.rs`); sürücü `sweep.rs`'te.
- kökte: sözleşme (`study.rs`, `core.rs`), kayıt (`registry.rs`), adres, zincir, grafik,
  şeritler, paralel koşu ve profiller.

**Ne değildir:** veri kaynağı, çizim ya da otorite değildir; hesap sonucunu belgeye yazmaz
(onu `qp-session` yapar).

**Bilinen borç:** `tests/` klasöründe 16 dosya (klasör bütçesi 15; `dir.files`).
