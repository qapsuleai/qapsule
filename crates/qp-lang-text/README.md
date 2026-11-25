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
- `print(&Program)`, `print_doc(&ProgramDoc)`, `print_reject(&ProgramReject)` → `Result<String, PrintError>` — kanonik metin; `print_with(&Program, &TextLimits)`.
- `lower_with(src, lang, &TextLimits)`, `TextLimits`, `DEFAULT_MAX_*`.
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

**Sınırlar (`TextLimits`):** metin uzunluğu, sözcük uzunluğu ve ifade derinliği (parantez
dahil, ağaç yüksekliği; varsayılan 64 Ki karakter, 1024, 64). Bunlar dilin yapısal sınırlarıdır
(PR §8.4 dil sabiti; RL1 kayıt defteri anahtarları ara gösterimi sınırlar, metni değil); çağıran
daha sıkısını `lower_with` / `print_with` ile verir. Aşım `E_TOO_LARGE` (`limit`); özyineleme
yığını taşmaz. Yazıcı ara gösterimde tam bir kez kullanılan `_` zincirini en çok bu derinlikte
gömer, ötesine satır açar. `did_you_mean` 64 karakterden uzun adlarda öneri vermez ve en çok 512
aday puanlar. Üslü sayı (`1e5`) `E_PARSE` ("üslü yazım yok").

**Kodlama:** metin UTF-8 `&str`. Baştaki BOM (U+FEFF) tanınmayan karakterdir: 1:1'de `E_PARSE`.
`\r` boşluk sayılır: CRLF satır sonu çalışır, yalnız `\r` ile ayrılmış satırlar tek satır okunur
(ilk bozuk deyimde `E_PARSE`).

**Yazıcı reddi:** yazıcı ara gösterimdeki adları olduğu gibi yazmadan önce denetler (çıktı ve
boyut adı, parametre anahtarı ve başvuruları, `tf`, `.çıktı` seçici, tanımsız başvuru, ret
kodu). Metin dilbilgisine uymayan ad satır enjekte edebileceği için **reddedilir**
(`PrintError {at, value, expected}`), yeniden adlandırılmaz: çıktı ve parametre adları programın
anlamıdır. Düğüm kimlikleri anlam taşımaz; yazılamayanlar `v<k>` olur.

**Ne değildir:** denetçi değildir (tip, saat, bütçe `qp-lang`); çalıştırmaz; günlüğe metin
yazmaz (günlükte yalnız ara gösterim).
