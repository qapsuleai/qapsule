# R4/H-M11 — "gerçekleşti" ile "tutmadı"nın işarette ayrımı: ölçüm düzeneği

Bağlayıcı şartname: `rounds/R4/experiments/harness/SPEC.md` §0, §1, §5, §7. Bu klasör
bağımsız bir Rust crate'idir (`r4-m11-harness`; ana çalışma alanının üyesi değil,
`crates/*`'a yol bağımlılığı, kendi `Cargo.lock`'u) ve okuyucu ajanları başlatan
`run_readers.sh`. Hesap yöntemi ve dosya yardımcıları R3 düzeneğinden
(`rounds/R3/experiments/harness/`: `m1/color.rs`, `m1/distinct.rs`, `m1/render.rs`,
`io.rs`, `rng.rs`) kopyalandı; R3 düzeneği değişmedi.

## Bağımsızlık beyanı

Düzenek hiçbir alternatif görülmeden yazıldı. İçindeki tek gerçek stil başvurudur:
`r3` = R3'ün onay/çarpı çizimi (alfabenin kendi çizimi, aynen). Alternatif ajanlar
düzeneği değiştirmez; yalnız ürün kodunda bir stil ekler (aşağıda). Seçim kuralı
`src/score.rs`'te, sonuç görülmeden yazılıdır.

## Alternatif ekleme (arayüz)

Ürün kodunda **tek dosya**: `crates/qp-grammar/src/glyph/outcome.rs`.

1. Satır 100'deki `// --- M11 alternatives: …` işaretinin altına
   `qp_grammar::glyph::outcome::OutcomeStyle`'ı uygulayan bir birim tip yaz (ya da
   `glyph/outcome/<ad>.rs` modülü açıp burada `mod` ile bildir):

   ```rust
   pub trait OutcomeStyle: Sync {
       fn name(&self) -> &'static str;             // ayar adı: [a-z][a-z0-9_-]*, sıradan sözcük değil
       fn draw(&self, alphabet: &dyn Alphabet, spec: &GlyphSpec,
               bx: GlyphBox, theme: &Theme) -> Vec<Prim>;
       fn legend(&self, alphabet: &str) -> &'static str; // ≤ 30 sözcük, TR, adını içermez
   }
   ```

   - `draw` her aşamada çağrılır. Açık aşamalarda (`Candidate`, `Final`) **alfabenin kendi
     çizimini** döndür (`alphabet.draw(spec, bx, theme)`). Kapalı aşamalarda (`Realized`,
     `Failed`; isteğe bağlı `Retracted`) kendi sonuç işaretini çiz. `alphabet.name()`
     `"siluet"` ya da `"rozet"`tur; ikisi için de tanımla (SPEC §5).
   - Yardımcı: `open_look(alphabet, spec, bx, theme)` glifi açıkmış gibi (`Final`) çizer;
     üstüne kendi işaretlerini ekleyebilirsin. Ya da `alphabet.draw(spec, …)` çıktısını
     sonradan işle.
   - Kurallar (`glyph` işareti, `docs/GRAMMAR.md` §18): belirleyici, yalnız IEEE temel
     işlemleri (`sqrt` dahil; gerisi `crate::detmath`), renkler yalnız `Theme`'den, bütün
     ilkeller kutunun içinde (düzenek sayar: `out_of_box_prims` 0 olmalı).
   - **Süresi doldu** glife `Failed` olarak gelir (`GlyphPhase: From<Stage>`); ayrı işaret
     şema değişikliği ister (kapsam dışı).
   - `legend(alphabet)`: alternatifin en çok 30 sözcüklük tek cümlelik açıklaması
     (`LEGEND_MAX_WORDS`, satır 47); yalnız `legend` koşulunda okuyucuya gider. Stil adı,
     "varyant", "alternatif" gibi yasak sözcükler içerirse `sheets` hata verir.
2. Satır 103'teki `pub static STYLES` listesine, `// --- M11 alternatives: one line each`
   yorumunun altına tek satır: `&BenimStilim,`.

Başka hiçbir şey değişmez. Seçim gramerden gelir: `qp_grammar::CompileOptions.outcome`
(`Option<String>`; tema geçersiz kılması gibi bir bakış ayarı, görünüme yazılmaz; yoksa
`"r3"`; bilinmeyen ad → `invalid`, `(options)/outcome`). Gösterim (T5) bu alanı bakış
ayarından doldurur; bugünkü varsayılan ve bütün altınlar bayt eşittir (`r3` alfabenin
çizimini aynen döndürür; test `crates/qp-grammar/tests/capsule_depth.rs`
`outcome_style_is_a_compile_option`, `glyph/outcome.rs` `r3_is_the_alphabet`).

`rozet` bugün ürün crate'inde değildir; düzenek onu R3 deney crate'inden
(`rounds/R3/experiments/M1/rozet/`, değişmeden) çizer ve stilin `draw`'ına
`alphabet.name() == "rozet"` olarak verir. Ürün `rozet`'i taşıdığında (T5) yalnız
`src/alphabets.rs` değişir.

## Komutlar

Hepsi bu klasörden. Çıktılar `rounds/R4/experiments/M11/` altına (`--root <dizin>` ile
başka köke; `run_readers.sh` için `M11_ROOT`).

```sh
cargo build --release
B=./target/release/m11

$B list                                   # kayıtlı stiller (STYLES)
$B measure --style <ad>                   # raw/<ad>/distinct.json + distinct.md  (SPEC §5(a))
$B sheets --style <ad> --seed 1 [--alphabet siluet|rozet] [--persona F|H] [--cond nolegend|legend]
                                          # raw/screens/<kod>.png, raw/keys/<kod>.json, raw/key.json,
                                          # raw/prompts/<persona>_<koşul>_<kod>.txt; kodu basar
./run_readers.sh <ad> F nolegend 3        # 3 taze okuyucu (tohum 1, 2, 3)
./run_readers.sh <ad> H nolegend 3
./run_readers.sh <ad> F legend 3
./run_readers.sh <ad> H legend 3
$B score                                  # raw/scores.json + scores.md, seçim kuralı
$B check-prompt <dosya>                   # yasak sözcük denetimi (run_readers her istemde çağırır)

cargo test                                # düzeneğin kendi testleri
```

`run_readers.sh` ortamı: `SEED_BASE` (ilk tohum, varsayılan 1; okuyucu k tohum
`SEED_BASE + k − 1`), `ALPHABET` (`siluet` varsayılan — ürün alfabesi; `rozet`), `MODEL`
(varsayılan `sonnet`), `M11_ROOT`, `DRY_RUN=1` (sayfa, istem ve meta yazılır; `claude`
çağrılmaz). Cevabı olan okuyucu dizini atlanır. Okuyucu: `claude -p --model sonnet --tools
Read --allowedTools Read --restricted --strict-mcp-config --permission-mode dontAsk
--no-session-persistence --output-format json`, çalışma dizini yalnız `<kod>.png`'yi
içeren yeni geçici dizin (`$TMPDIR/rdr.XXXXXX`).

SPEC planı (persona × alternatif × koşul × 3 okuyucu): stil başına 2 × 2 × 3 = 12 okuyucu;
başvuru `r3` dahil. R3 M1 okuma maliyeti (~0,018 $/okuyucu) ile stil başına ≈ 0,2 $.

## Yöntem

### (a) Hesap — `measure` (SPEC §5(a))

- Çiftler: 6 tür × 3 yön = 18; her biri `realized` ↔ `failed` (aynı glif: güç 0,6, güven
  0,6, teyit 1, tazelik 1, sunucu işareti yok, geçerlilik `closed`). Bilgi için (seçim
  kuralına girmez): kapalı ↔ açık (`final`, geçerlilik `valid`) çiftleri.
- Çizim: `OutcomeStyle::draw(alfabe, …)` → `qp_render::render_scene_rgba` (altınlarla aynı
  CPU yolu). Koşullar: 16/24/32 px × açık/koyu × normal/döteranopi/protanopi/tritanopi/gri,
  iki alfabe (`siluet`, `rozet`) → stil başına 60 koşul.
- Piksel yolu ve çift uzaklığı R3 M1 ile aynı: sRGB → doğrusal → Gauss σ = 0,5 px (yarıçap
  2, kenar kıskaçlı) → Machado 2009 şiddet 1,0 ya da parlaklık → CIELAB D65; ±1 px dokuz
  kaydırmanın en küçüğü, her biri bütün piksellerde ortalama ΔE76.
- **Ana ölçü:** en kötü koşulda en zayıf çift; alfabe başına ve iki alfabe üzerinden.
  4 ondalık.

### (b) Persona sayfaları — `sheets`, `run_readers.sh` (SPEC §5(b))

- 4×4 ızgara, 16 işaret, A1…D4 (satır harfi, sütun rakamı; etiket üstte); 32 px çizilip
  4× en yakın komşu büyütme (128 px hücre), açık tema.
- Seçim yalnız tohuma bağlı (her stil aynı tohumda aynı 16 glifi gösterir): 18 tür × yön
  birleşiminden 16'sı; sonuçlar 6 gerçekleşti, 6 tutmadı, 4 açık (tohumlu sıra); güç
  {0,3; 0,6; 0,9}, güven {yok; 0,3; 0,7}, teyit 1–3.
- İstem (TR): SPEC §1 persona tarifi birebir, "Bu bir piyasa bilgi uygulamasının ekran
  görüntüsü." ("Uygulama tavsiye vermez" verilmez), görev, `legend` koşulunda stilin
  açıklaması, dört seçenek (`gerçekleşti` / `tutmadı` / `açık` / `bilmiyorum`), JSON cevap.
- **Körleme:** görüntü `raw/screens/<kod>.png`, kod (stil, alfabe, tohum, tuz)'un tohumlu
  karması (8 onaltılık hane; harfleri a–f, yasak sözcük oluşturamaz). Eşleme
  `raw/key.json` ve gizli anahtar `raw/keys/<kod>.json` okuyucu dizinine kopyalanmaz;
  okuyucu kaydı (`meta.json`) stil adı taşımaz. Her istem yasak sözcük listesine karşı
  denetlenir (`src/blind.rs`: kayıtlı bütün stil adları; `variant`, `varyant`,
  `alternatif`, `alternative(s)`, `harness`, `düzenek`, `m11`, `m11a`, `m11b`, `claude`,
  `r3`, `r4`, `style`, `stil`; alt dizeler `r4/`, `r3/`, `m11`, `alt-`, `alt_`, `/alt`,
  `outcome`, `variant`) — `sheets` yazarken ve `run_readers.sh` göndermeden önce.

### (c) Puanlama ve seçim — `score`

- Madde doğru: cevap = gerçek (`bilmiyorum`, eksik, tanınmayan → yanlış). TR harfleri ve
  büyük/küçük harf katlanır; İngilizce `realized/failed/open/unknown` da kabul. Şans 1/3,
  çoğunluk tabanı 6/16 = 0,375. Karışıklık matrisi (gerçek × cevap) kaydedilir.
- Stil × persona × koşul doğruluğu; ağırlıklı = (2·F + 1·H) / 3.
- **Seçim kuralı (SPEC §5):** (1) en kötü koşul ΔE'si başvuru `r3`'ün altında kalan elenir
  — **alfabe başına** (`siluet` ve `rozet` ayrı ayrı); (2) birincil: ağırlıklı
  açıklamasız doğruluk; (3) ilk iki arasındaki fark < 0,05 ise iki alfabe üzerinden en
  kötü koşul ΔE'si yüksek olan. Başvuru karşılaştırma için sıralanır, seçilmez.
- Yeniden puanlama ham dosyalardan bayt eşit (test).

## Sapmalar ve yorumlar (SPEC'e göre)

- **Yasak sözcük `alt`:** Türkçede "alt" sıradan sözcüktür ("sağ alt köşe"); tam sözcük
  olarak yasaklanırsa her Türkçe açıklama düşer. Düzenek `alt` kimlik biçimlerini
  (`alt-`, `alt_`, `/alt`) ve `alternatif`/`variant` sözcüklerini yasaklar, çıplak "alt"ı
  değil.
- **Eleme alfabe başına:** SPEC "en kötü koşul ΔE'si başvurunun altında kalan elenir" der;
  iki alfabe şartı nedeniyle iki alfabe için ayrı karşılaştırılır (daha katı yorum).
- **Okuyucu sayfası `siluet`** (ürün alfabesi) ile; `rozet` sayfası `ALPHABET=rozet` ile
  alınabilir, puanlama alfabeleri havuzlar (okuyucu kaydında alfabe yazılı).
- **Süresi doldu** ayrı ölçülmez: glif düzeyinde `failed`'dır.
- İstem okuyucunun geçici dizin yolunu içerir (R3 ile aynı); yasak sözcük denetimi yol
  yerine konmadan önceki şablona uygulanır (yol `$TMPDIR` içinde sistem adları
  taşıyabilir; dizin adı `rdr.XXXXXX`).
- Kayıt düzeni SPEC §7'ye uyar; okuyucu dizini `readers/<persona>_<koşul>_<kod>_r<k>/`
  (SPEC'teki `<sayfa>` = koşul + kod).

## Bilinen sınırlar

- Persona ajanı insan değildir; 4× büyütme pikselleri keskinleştirir, 32 px'te ekranda
  görülenden okunaklıdır. 16 px okunurluğu (a) ölçer. Bu bir ön elemedir (SPEC §0.5).
- ΔE76 algısal olarak düzgün değildir; ortalama ΔE küçük bir köşe işaretini seyreltir —
  sonuç işareti tam da böyle küçük bir öğedir, bu yüzden değerler küçük çıkar.
- `rozet` R3 deney crate'inden çizilir; ürün sürümü farklılaşırsa ölçü tekrar alınmalı.

## Başvuru `r3` sonuçları (öz-sınama, `M11/raw/r3/distinct.{json,md}`)

| alfabe | en kötü koşul ΔE | çift | koşul | en düşük ortalama | gerçekleşti↔açık en kötü | tutmadı↔açık en kötü |
|---|---:|---|---|---:|---:|---:|
| siluet | **2,98** | `flow.up` | 16 / koyu / döteranopi | 3,04 | 4,62 | 2,23 |
| rozet | **11,33** | `squeeze.neutral` | 16 / açık / protanopi | 11,86 | 9,56 | 2,02 |

İki alfabe üzerinden ana ölçü **2,98** (siluet). Seçim kuralının eşiği alfabe başına
siluet 2,98, rozet 11,33. Kutu dışı ilkel: 0. 32 px açık normalde siluet en zayıf çift 4,13
(ortalama 4,15). Not: R3 siluet onay/çarpısı 16 px'te ~3 ΔE ile ayrılır; tutmadı ↔ açık
(2,23 / 2,02) daha da zayıftır (bilgi; kurala girmez).

Uçtan uca sınama (gerçek ajan yok): `measure` + `sheets` + `DRY_RUN=1 run_readers.sh r3
{F,H} {nolegend,legend} 2` + sahte cevaplarla `score` geçici bir kökte koşuldu; istemler
temiz, yeniden puanlama bayt eşit. Aynı akış `tests/harness.rs`'te test olarak durur (sayfa
bayt eşitliği, yasak sözcük reddi, "ayırmayan" bir test stilinin ΔE = 0 olması ve kural
1'le elenmesi, kural 2 ve kural 3).

## Önerilen karar

- Sonuç işareti adla seçilen bir `OutcomeStyle` olarak alfabeyi sarar (`qp_grammar::glyph::
  outcome`); seçim `CompileOptions.outcome` bakış ayarıdır; varsayılan `r3`. M11 seçimi
  bütünleştirmede (S) varsayılanı değiştirir ve yeni altın ekler; `g1-glyphs` altınları
  değişmez.

## Öneriler (kapsam dışı)

- Gösterimin (T5) bakış ayarı yoluna `outcome` adı eklenmeli (`alphabet`/`face` yanında),
  M11 finalisti ayardan değiştirilebilsin.
- `rozet` ürüne taşınınca `src/alphabets.rs` ürün tipine çevrilmeli.
- "Tutmadı ↔ açık" ayrımı da sonuç işaretinin işi; seçim kuralına girmese de RESULTS'ta
  yazılmalı.
