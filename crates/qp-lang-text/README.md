# qp-lang-text

**Hesap dilinin metin yüzeyi** (`docs/PROGRAMS.md` §12): küçük, satır tabanlı yazım dili →
`qp-ir/0` ara gösterimi, ve ara gösterim → **kanonik metin**. Elle yazılmış sözcük çözücü ve
ayrıştırıcı; saat, rastgelelik, G/Ç yok; aynı girdi aynı çıktıyı verir.

**Katman:** alan (`docs/ENGINEERING.md` §2; `scripts/arch-config.json` alan sırasında en altta).

**Bağımlılıklar:** `qp-schema`, `serde_json`. Geliştirme bağımlılığı yok (özellik testinin
üreteci testin içinde, sabit tohumlu).

**Dışa açık yüzey (`lib.rs`):**

- `lower(src, lang) -> Result<Lowered, Vec<ProgramError>>` — metin → ham JSON ara gösterim
  (`Lowered::ir`, denetçinin tükettiği biçim; program ya da `{"reject": …}`) + `SourceMap`.
  Metin hataları hepsi birden, deyim başına bir, konum sırasıyla; `span`, kapalı kod, alt kod
  (`E_PARSE`, `E_UNKNOWN_OP`, `E_UNKNOWN_REF`, `E_UNKNOWN_PARAM`, `E_TYPE`, `E_MISSING_ARG`,
  `E_DUPLICATE_NAME`, `E_UNSUPPORTED`), `did_you_mean` (Türkçe ad tablosu dahil), iletiler
  `tr`/`en`.
- `Lowered::doc()` — tipli `ProgramDoc`; `Lowered::is_reject()`.
- `SourceMap::span_at(at)`, `SourceMap::attach(&mut errors)` — denetçinin `at` işaretçili
  hatalarını (tip, saat, maliyet) kaynak satırına eşler (§12.4).
- `print(&Program)`, `print_doc(&ProgramDoc)`, `print_reject(&ProgramReject)` — kanonik metin.
- `FUNCTIONS`, `REJECT_CODES` — işlev adları ve retin taşıyabileceği alt kodlar.

**Düzen (`src/`):** `lex.rs` (sözcükler), `parse/` (deyimler, ifadeler; satır başından yeniden
eşleme), `ast.rs`, `lower/` (adlar, düğümler, sayı alanları, çıktılar), `print/` (kanonik
yazıcı, öncelik, sayı yazımı), `names.rs` (sabit sözlük), `diag.rs` + `msg.rs` (hata ve iki
dilli iletiler), `map.rs` (kaynak haritası), `lowered.rs` (giriş noktası).

**İş bölümü:** burada yalnız ara gösterimde yazılamayan şey hatadır (sözdizimi, bilinmeyen ad
ya da işlev, sayı alanında ifade, metindeki ad çakışması, yanlış yerde `reject`). Ara
gösterimin taşıyabildiği her şey (tip, saat, aralık, kesirli dönem, eksik `side`/`max_bars`,
`tut`) olduğu gibi yazılır; kodlu cevabı denetçi verir.

**Sınanan özellikler:** §12.5 ve §14 örnekleri beklenen ara gösterime iner ve kendilerine
yazdırılır; gidiş-dönüş (a) `lower(print(lower(t))) == lower(t)`, (b) `_` kimliksiz ara
gösterimde `lower(print(p)) == p`, `_` kimlikliyse yeniden adlandırmaya kadar eşit, (c)
`print` eşgüçlü — her biri 300 rastgele programla.

**Ne değildir:** denetçi değildir (tip, saat, bütçe `qp-lang`); çalıştırmaz; günlüğe metin
yazmaz (günlükte yalnız ara gösterim).
