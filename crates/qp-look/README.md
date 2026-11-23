# qp-look

**Görünüşler** (R5/T3; `docs/GRAMMAR.md` §9a): tasarım kayıtları (`look`: tür rolleri, yazı
tipleri, ölçüler, renk rolleri, grafik gelenekleri, kabuk CSS değişkenleri), temalar
(`theme`), gramer hatası (`error`) ve belirlenimci matematik (`detmath`). Derleyicinin,
anlam gliflerinin ve adlı tasarımların ortak tabanıdır. RF1'de `qp-grammar`'dan saf taşımayla
ayrıldı (`rounds/RF1/PLAN.md` §3.1).

**Katman:** alan (`docs/ENGINEERING.md` §2); katman içi sırada `qp-facts`'ten sonra,
`qp-glyph`'ten önce.

**Bağımlılıklar:** `qp-schema`, `serde_json` (`qp look show` dökümü).

**Dışa açık yüzey (`lib.rs`):** modüller `look`, `theme`, `error`, `detmath`, `advances`;
kökte `GrammarError`, `ErrorKind`, `Theme`.

**Düzen (`src/`):** `look.rs` kayıt tipleri ve `look/` görünüşler (`r4`, `ornek`, `oltan`,
`zemre`, `tavek`), yazı tipleri, CSS; `advances.rs` üretilmiştir (yazı tipi ilerlemeleri,
`QP_BLESS=1 cargo test -p qp-render --test fonts`; bütçe dışı); `theme.rs`, `error.rs`,
`detmath.rs`.

**Ne değildir:** sahne derlemez (`qp-grammar`), glif çizmez (`qp-glyph`), adlı tasarım
(yüz, açılmış kapsül, hikâye, akış, geçiş) taşımaz (`qp-designs`); piksel çizmez
(`qp-render`).
