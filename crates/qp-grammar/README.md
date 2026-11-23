# qp-grammar

**Görsel gramer derleyicisi** (R2; `docs/GRAMMAR.md`). `compile` bir görünümü
(`qp_schema::grammar::View`) ve verisini (`DataProvider`) çiziciden bağımsız sahne betimine
(`qp_schema::scene::SceneDesc`) çevirir; başvuru çizicisi bu sahneyi çizer, anlamsal okuma
(`screen`, `hit`) onu okur. Sunucunun derinlik kuralları ve jestler de buradadır. Belirlenimci:
yerleşim yalnız temel işlemler ve tamsayı; aynı görünüm ve veri her hedefte aynı sahneyi verir.

RF1'den beri adlı tasarımlar (yüz, açılmış kapsül, hikâye, akış, geçiş, hareket) `qp-designs`'ta,
görünüşler/temalar/hata tipi `qp-look`'ta, anlam glifleri `qp-glyph`'tedir. Derleyici tasarım
tarafını tanımaz: beş çağrı noktasında kendisine verilen **tasarım kaydından** (`Designs`,
`CompileOptions::designs`) çağırır.

**Katman:** alan (`docs/ENGINEERING.md` §2); katman içi sırada `qp-glyph`'ten sonra,
`qp-designs`'tan önce.

**Bağımlılıklar:** `qp-schema`, `qp-look`, `qp-glyph`, `qp-calendar` (zaman etiketlerinin
gösterim dilimi). Geliştirme: `qp-compute`, `serde_json`, `proptest`.

**Dışa açık yüzey (`lib.rs`):** `compile`, `CompileOptions` (`with_designs`,
`without_designs`; ürünün seçenekleri `qp_designs::options`), `Cursor`, `Designs`,
`DataProvider`, `MemProvider`, `HeadlineParts`, `SeriesKind`; modüller `designs` (kayıt ve
çağrı noktalarının tipleri: `HeadlineFn`, `DeviceCheck`, `headline_text`, `NONE`), `depth`,
`gesture`, `scale`, `terms`, `draw`, `compile`, `provider`.

**Düzen (`src/`):**

- `compile/` — derleyicinin içi: alfabe, paneller, seriler, eksen, vurgu, çerçeve, terimler;
  `layer/` katman hazırlama, `draw/` piksel işi.
- `present/` — derinlik kuralları (`depth/`) ve jestler.
- kökte: `designs.rs` (tasarım kaydı), `provider.rs`.

**Ne değildir:** hesap yapmaz (değerler sağlayıcıdan gelir, otorite `qp-compute`'u koşar);
piksel çizmez (`qp-render`); veri taşımaz, yalnız ona başvurur (K23); adlı tasarım taşımaz
(`qp-designs`).

**Bilinen borç:** tek işlevlik 800 üstü `src/compile/layer/prepare.rs` — İ6 (G3'te adımlara
bölünecek).
