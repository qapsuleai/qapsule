# qp-designs

**Adlı tasarımlar** (RF1; `rounds/RF1/PLAN.md` §3.1): kapsül yüzleri (`faces`), açılmış kapsül
tasarımları (`open`), hikâye grafikleri (`stories`), akış düzenleri (`feeds`), derinlik
geçişleri (`transitions`), hareket tasarımları (`motion`) ve bunlara ileten derinlik girişleri
(`depth`: `face_view`, `face_view_in`, `is_face`, `story_design`). Ayrıca derleyiciye verilen
**tasarım kaydının** varsayılanı (`DESIGNS`) ve onu taşıyan seçenekler (`options`): derleyici
tasarım tarafını tanımaz, beş çağrı noktasında (yüzün başlık şablonu, açılmış tasarımın iki
metin alanı, tasarım katmanının terimi, sesin terimlerinin arandığı katmanlar) bu tablodan
çağırır. RF1'de `qp-grammar`'dan saf taşımayla ayrıldı.

**Katman:** alan (`docs/ENGINEERING.md` §2); katman içi sırada `qp-grammar`'dan sonra,
`qp-capsule`'dan önce.

**Bağımlılıklar:** `qp-schema`, `qp-look`, `qp-glyph`, `qp-grammar`. Geliştirme: `qp-compute`
(sağlayıcıların çalışma değerleri), `qp-capsule` (karakterizasyon kitaplarının SHA-256'sı),
`serde_json`.

**Dışa açık yüzey (`lib.rs`):** `options(width, height)` (RF1 öncesi
`CompileOptions::new(width, height)`), `DESIGNS`; modüller `faces`, `open`, `stories`,
`feeds`, `transitions`, `motion`, `depth`.

**Düzen (`src/`):** her tasarım ailesinin kaydı kökteki dosyasında (`faces.rs`, `open.rs`,
`stories/mod.rs`, `feeds.rs`, `transitions.rs`, `motion.rs`), tasarımlar alt klasörlerde;
`registry.rs` varsayılan kayıt; `depth.rs` ileten derinlik girişleri. Sınamalar (`tests/`):
yüz, açılmış kapsül ve hikâye sınamaları ile gramer karakterizasyonu (`tests/charac`;
fikstürler `qp-grammar/tests/support`'tan, `tests/support/mod.rs` üzerinden).

**Yeni tasarım:** ailesinin kayıt dosyasına bir satır ve bir dosya (her ailenin modül belgesi);
derleyiciye (`qp-grammar`) dokunulmaz. Yeni bir derleyici → tasarım çağrı noktası gerekirse
`qp_grammar::designs::Designs`'a bir alan eklenir ve `registry.rs`'te doldurulur.

**Ne değildir:** derleyici değildir (`qp-grammar`), görünüş kaydı değildir (`qp-look`), glif
alfabesi değildir (`qp-glyph`); hesap yapmaz, piksel çizmez.
