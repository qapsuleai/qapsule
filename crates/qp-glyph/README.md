# qp-glyph

**Anlam glifleri** (R3; `docs/GRAMMAR.md` §18): bir alfabenin uyguladığı sözleşme
(`Alphabet`, `GlyphBox`), alfabeler (`siluet`, `rozet`) ve sonuç işareti biçemleri
(`outcome`: `r3`, `tg5`, `muhrak`). Belirlenimci; bütün renkler temadan. RF1'de
`qp-grammar`'ın `glyph/` modülünden saf taşımayla ayrıldı (`rounds/RF1/PLAN.md` §3.1).

**Katman:** alan (`docs/ENGINEERING.md` §2); katman içi sırada `qp-look`'tan sonra,
`qp-grammar`'dan önce.

**Bağımlılıklar:** `qp-schema`, `qp-look` (tema, görünüş, hata tipi).

**Dışa açık yüzey (`lib.rs`):** `Alphabet`, `GlyphBox`, `alphabet`, `alphabet_named`,
`ALPHABETS`; modüller `outcome`, `siluet`, `rozet`.

**Düzen (`src/`):** `contract.rs` sözleşme ve alfabe seçimi; `siluet.rs` + `siluet/`,
`rozet.rs` alfabeler; `outcome.rs` + `outcome/` sonuç işareti biçemleri.

**Ne değildir:** glif parametrelerinin hesabı değildir (derinlik kuralları,
`qp_grammar::depth`); glif işaretini sahneye yerleştirmez (`qp-grammar` derleyicisi).
