# qp-facts

**Olgu dedektörleri** (R3; `Detector` = "Olgu dedektörü", `docs/CONCEPTS.md`). Bir bar
serisini (sunucu olgularında lisanslı akış beslemesini de) altı türlü anlam sözlüğünde
(`qp_schema::capsule::MeaningKind`) tipli olgulara çevirir. Kurallar `qp-compute` ile aynı:
belirlenimci `f64`; toplu = akış, bit eşit; bekleyen bardaki tespit **adaydır**, bar
kesinleşince olgu olur; dedektör düzey değil kenar (değişim) üretir.

**Katman:** alan (`docs/ENGINEERING.md` §2).

**Bağımlılıklar:** `qp-schema`, `qp-compute`, `qp-calendar`. Geliştirme: `qp-source`
(sentetik piyasa).

**Dışa açık yüzey (`lib.rs`):**

- `Detector`, `Engine`, `Staged`, `Detection`, `FlowPoint` — sözleşme ve bekleyen bar sarmalı.
- `run_batch`, `to_fact`, `fact_id` — olguya çevirme.
- Dedektörler: `RangeBreakout`, `SeasonalBreakout`, `RsiExtreme`, `RsiDivergence`,
  `MaStack`, `BandSqueeze`, `VolumeSurge`, `SeasonalVolumeSurge`, `InstitutionalFlow`.
- Takımlar: `default_detectors`, `rp4_detectors`, `detectors_named`, `DETECTOR_SETS`;
  mevsimsellik `season` (`Season`, `SEASON_*` sabitleri).

**Düzen:** `src/` düz; dedektör başına bir dosya, sözleşme `detector.rs`, ortak olgu
kurma `fact.rs`, takımlar `sets.rs`.

**Ne değildir:** kapsül motoru değildir (olgudan hikâye kurmak `qp-capsule`'dadır); veri
okumaz; hesapları `qp-compute`'tan alır, kendisi gösterge tanımlamaz.
