# qp-grammar

**Görsel gramer derleyicisi** (R2; `docs/GRAMMAR.md`). `compile` bir görünümü
(`qp_schema::grammar::View`) ve verisini (`DataProvider`) çiziciden bağımsız sahne betimine
(`qp_schema::scene::SceneDesc`) çevirir; başvuru çizicisi bu sahneyi çizer, anlamsal okuma
(`screen`, `hit`) onu okur. Sunucunun derinlik kuralları, kapsül yüzü, açık kapsül, hikâye,
akış ve geçiş tasarımları, görünüşler (look) ve temalar da buradadır. Belirlenimci: yerleşim
yalnız temel işlemler ve tamsayı; aynı görünüm ve veri her hedefte aynı sahneyi verir.

**Katman:** alan (`docs/ENGINEERING.md` §2).

**Bağımlılıklar:** `qp-schema`, `serde_json`. Geliştirme: `qp-compute`, `qp-capsule`,
`proptest`.

**Dışa açık yüzey (`lib.rs`):** `compile`, `CompileOptions`, `DataProvider`, `MemProvider`,
`HeadlineParts`, `SeriesKind`, `Theme`, `GrammarError`, `ErrorKind`; modüller `depth`,
`motion`, `gesture`, `scale`, `terms`, `look`, `faces`, `open`, `stories`, `feeds`,
`transitions`, `glyph`, `detmath`, `advances`.

**Düzen (`src/`):**

- `compile/` — derleyicinin içi: paneller, seriler, eksen, vurgu, çerçeve; `layer/` katman
  hazırlama, `draw/` piksel işi.
- `present/` — derinlik kuralları (`depth/`), hareket (`motion/`), jestler.
- `look/` — görünüş kayıtları, yazı tipleri, kabuk CSS değişkenleri.
- `faces/`, `open/`, `stories/`, `feeds/`, `transitions/` — ada göre seçilen tasarımlar.
- `glyph/` — anlam glifi alfabeleri ve sonuç işareti biçemleri.
- kökte: `advances.rs` (üretilmiştir, yazı tipi ilerlemeleri; bütçe dışı), `theme.rs`,
  `detmath.rs`, `provider.rs`.

**Ne değildir:** hesap yapmaz (değerler sağlayıcıdan gelir, otorite `qp-compute`'u koşar);
piksel çizmez (`qp-render`); veri taşımaz, yalnız ona başvurur (K23).

**Bilinen borç:** crate ≈ 19,3 bin satır (`crate.lines`, bütçe 12 bin) — istisna isteği İ4
(`rounds/RG1/PLAN.md` §7); tek işlevlik 800 üstü `src/compile/layer/prepare.rs` — İ6.
