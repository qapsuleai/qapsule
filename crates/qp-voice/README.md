# qp-voice

**Kapsül sesleri** (R5/M17, `docs/LANGUAGE.md`): bir kapsülün dört yüzeyini (başlık, ayrıntı,
karne, sözlük) motor verisinden yazan diller. Belirlenimci: aynı girdi hep aynı metni verir.
RF1'de `qp-capsule`'ın `voice/` modülünden saf taşımayla ayrıldı (`rounds/RF1/PLAN.md` §3.2).

**Katman:** alan (`docs/ENGINEERING.md` §2); katman içi sırada `qp-capsule`'dan sonra.

**Bağımlılıklar:** `qp-schema`, `qp-calendar`, `qp-facts`, `qp-capsule` (R3 cümlesinin
yapı taşları `realize::{elapsed, morph, num, parts, stamp}`, `SentenceInput`, `Lang`).
Geliştirme: `qp-source` (sentetik piyasa).

**Dışa açık yüzey (`lib.rs`):**

- `speak`, `Input`, `Surfaces`, `Gloss`, `text_face`, `unnumber` — tek giriş noktası ve
  lisans koruması (`surfaces.rs`).
- `VOICES`, `DEFAULT_VOICE`, `PRESENTED_VOICE`, `is_voice`, `names` — ses kaydı
  (`registry.rs`).
- Modüller: `marla`, `marla_glossary`, `selvi`, `selvi_glossary`.

Oturum bu crate'i `qp_session::voices` adıyla yeniden dışa açar.

**Düzen (`src/`):** `registry.rs` ses adları ve varsayılanlar; `surfaces.rs` `speak` ve
`unnumber`; `marla/`, `selvi/` sesler ve `*_glossary.rs` sözlükleri.

**Ne değildir:** R3 cümlesi ve R4 yüz parçaları burada değil, `qp-capsule`'ın `realize/`
modülündedir; kapsül motoru değildir; hangi sesin seçildiğine oturum karar verir.
