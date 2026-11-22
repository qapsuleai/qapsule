# qp-capsule

**Kapsül motoru** (R3): olgular → bulgular → kapsül hikâyeleri → karne. Belirlenimci:
saat yok (zaman bardır), ağ, rastgelelik ve model yok; aynı barlar ve aynı dedektör çıktısı
hep aynı hikâyeleri, geçişleri, karneyi ve cümleleri verir. Yaşam döngüsü kuralları
`docs/CAPSULE.md` §9.3; ölçülmüş tasarım soruları trait'tir (`Realizer`, `Ranker`,
`ThemeRule`, `OutcomeRule`, `TrustPath`; sözleşme `contract.rs`).

**Katman:** alan (`docs/ENGINEERING.md` §2).

**Bağımlılıklar:** `qp-schema`, `qp-calendar`, `qp-compute`, `qp-facts`, `serde`, `regex`.
Geliştirme: `qp-source`, `serde_json`.

**Dışa açık yüzey (`lib.rs`):**

- `Engine`, `EngineConfig` (`r3`, `named`), `ENGINES`, `DEFAULT_ENGINE`, `EngineSnapshot`,
  `StoryEvent`, `LedgerRecord`, `glyph_spec` — motor, adlı kayıtlar, karne.
- `simulate`, `InstrumentInput`, `SimResult` — başsız benzetim (dedektör → olgu → motor).
- `admits`, `admits_at` — duyarlılık süzgeci (`audience`, §9.8).
- Modüller: `theme`, `outcome`, `rank`, `realize`, `trust`, `voice`, `sentence`, `comment`,
  `feedrule`, `sentinel`, `market`, `scaffold`.

**Düzen (`src/`):**

- `engine/` — motorun parçaları: bar adımı (`run.rs`), karne dizini, glif ölçüleri.
- `realize/` — R3 cümle üretici (TR + EN; şablon dilbilgisi), yüz parçaları, geçen süre.
- `voice/` — kapsül sesleri (`marla/`, `selvi/` ve sözlükleri; `speak`).
- `comment/` — ajan yorumu denetimi (sayı okuma, sözcük listeleri, kanıta karşı denetim).
- `trust/` — mühür makbuzu (SHA-256, HMAC; crate içinde yazıldı).

**Ne değildir:** olgu dedektörü değildir (`qp-facts`); oturum ya da otorite değildir (motoru
belgeye `qp-session` bağlar); çizim yapmaz.

**Bilinen borç:** crate ≈ 13,2 bin satır (`crate.lines`, bütçe 12 bin) — istisna isteği İ5
(`rounds/RG1/PLAN.md` §7); `qp-capsule → qp-facts` yan bağımlılık (`layer.lateral`, İ2, §3.3).
