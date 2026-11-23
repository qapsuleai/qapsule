# RF1/G2 bağımsız denetim — `rf1/g2-grammar` @ 4b841ffb

Denetçi: ayrı alt ajan (işi yapan ajan değil). Dal değiştirilmedi, birleştirilmedi.
Ortam: `git worktree add .claude/worktrees/audit-g2 rf1/g2-grammar --detach`,
`CARGO_TARGET_DIR=target/wt/audit-g2` (iş bitince ikisi de silindi). Dal `main` 88667fee'yi
içeriyor (`merge-base --is-ancestor` doğrulandı).

## Hüküm: **GEÇTİ**

Davranış korunmuş (270/270 bayt eşit, kitaplar yalnız yeniden adlandırma, bütün CHANGED öğeler
dört gruba giriyor, ürün kodunda `NONE` yok), yapı ölçütleri yeşil, WASM ±%2 içinde. Aşağıdaki
bulgular birleştirmeyi engellemez; B1 round kapanışından önce kapanmalı.

## Sonuçlar

| Adım | Sonuç |
|---|---|
| 1 `cargo fmt --all -- --check` | 0 |
| 1 `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| 1 `cargo test --workspace` | 0 — 1222 geçti, 0 kaldı, 9 yok sayıldı (191 ikili); `qp-designs/tests/charac` 11 test, `qp-session/tests/charac` 25 test geçti; koşudan sonra ağaç temiz (kitap yeniden yazılmadı) |
| 1 `git diff -M main...rf1/g2-grammar -- '*/expected/*'` | 12 dosya, hepsi `R100` (`qp-grammar/tests/charac/expected/*` → `qp-designs/tests/charac/expected/*`), içerik satırı farkı 0; S1 kitapları (`qp-session/tests/charac/expected`) hiç değişmedi |
| 2 `scripts/refactor/compare.sh` | **270/270** bayt eşit (exit 0) |
| 3 `scripts/refactor/moved.py main HEAD` | 1932 değişmemiş, 146 PATH, 0 ALLOW, **173 CHANGED** (aşağıda), 34 FLAG, 10 REEXPORT |
| 4 `cargo tree -p qp-grammar \| grep -i designs` | boş. Doğrudan bağımlılıklar: qp-calendar, qp-glyph, qp-look, qp-schema |
| 4 `scripts/arch-check` | 0 hata; 4 adlı istisna (İ4 yok), taban içinde |
| 4 `--table` | qp-grammar **7.449** (≤ 12.000; pay **4.551**), qp-designs 8.925, qp-look 2.813, qp-glyph 1.989; qp-session 31.999 → 31.997 (net satır eklenmedi) |
| 4 `--config-diff` | 28 değişiklik, **0 gerekçesiz gevşetme**; tek LOOSER `generated_files` girdisi, `DECISION.md`'yi anıyor |
| 4 katman sırası | `qp-source < qp-facts < qp-look < qp-glyph < qp-grammar < qp-designs < qp-capsule < qp-voice` — PLAN §3.1 ile aynı |
| 5 WASM varsayılan (`--profile wasm`) | raw 2.353.971 (+8.150, **+%0,35**), gzip -9 773.380 (+3.544, **+%0,46**) — taban 2.345.821 / 769.836 |
| 5 WASM kabuk paketi (`wasm-pack … --features replica`, geçici klasöre) | raw 8.813.479 (+24.065, **+%0,27**), gzip -9 2.437.562 (+5.174, **+%0,21**) — taban 8.789.414 / 2.432.388 |
| ek `scripts/design-contract --check` | fark 0 |
| ek `advances.rs` | `main:crates/qp-grammar/src/advances.rs` ile `crates/qp-look/src/advances.rs` `cmp` ile özdeş |

## 3. CHANGED öğelerin sınıflandırması

`moved.py`'nin 173 CHANGED satırı (eşleşen +/− çiftleri ve yeni öğeler) okunup her biri şu
gruplardan birine yerleştirildi. Ek olarak bütün `.rs` farkı, `use` satırları ve
crate/modül önekleri normalleştirilerek dosya dosya artık farka indirildi
(`resid.py`, 37 dosya); her artık satır aşağıdaki gruplardan birine düşüyor.

1. **G2b yeniden yazımları:** `compile/scene.rs::compile`, `layer/prepare.rs::prepare`,
   `terms.rs::mark` + yeni `underline_layer`, `frame.rs::Frame`, `compile.rs::CompileOptions`
   (+`with_designs`, `without_designs`; `new` kalktı), `designs.rs` (yeni: `Designs`, tip
   takma adları, `NONE` ve üç boş fonksiyon, `Debug`/`PartialEq`), `qp-designs/src/registry.rs`
   (`DESIGNS`, `options`, `headline_template` uyarlayıcısı).
2. **`CompileOptions::new(` → `qp_designs::options(` / `crate::options(` /
   `CompileOptions::without_designs(`:** bütün test, örnek ve ürün çağrı yerleri (qp-cli,
   qp-mcp, qp-wasm, qp-session `replica`, `viewer`, `reading`, `preview`, `interact`,
   `scenes`; qp-render testleri ve örneği; taşınan tasarım testleri). Bunların bir kısmı
   yalnız rustfmt yeniden sarması (ör. `tests/errors.rs`, `lod.rs`, `r0_equivalence.rs`:
   daha uzun ad yüzünden çok satıra açıldı; `qp-cli/src/main.rs`: tek satıra kapandı).
3. **qp-render `tests/fonts.rs`:** `advances.rs` yolu ve hata iletisi `qp-look`'u gösteriyor.
4. **İki satır bütçesi düzenlemesi:** `qp-session/src/read/interact.rs` `type StoryCompiled`
   tek satıra (yol kısaldığı için; qp-session'a net satır eklenmesin diye) ve aynı dosyada
   `depth::story_design` → içe aktarılmış `story_design`. Biçim değişikliği; davranış yok.

Gruplara dahil, taşımanın doğal parçası olan diğer artıklar (hepsi okundu, davranışsız):
`pub(crate)` → `pub` genişletmeleri (§6), `mod`/`pub use` bildirimlerinin yeni `lib.rs`'lere
geçmesi, `glyph.rs` → `qp-glyph/src/contract.rs` (alt modül bildirimleri `lib.rs`'e),
`depth`'in beş ileten girişinin (`is_face`, `face_view`, `face_view_in`, `story_design`)
`qp-designs/src/depth.rs`'e taşınması (gövdeler aynı), `HeadlineFn`/`headline_text`/
`DeviceCheck`'in `faces`/`open`'dan `qp_grammar::designs`'a sözcüğü sözcüğüne taşınması,
`NO_LONGER_HEADLINE`'ın `tesir`'den `qp-look/src/look.rs`'e taşınması (değer `"muted"`
aynı; look testi artık tasarıma bakmıyor), `transitions.rs` alan tipinin ve
`shellcore/look.rs`, `qp-wasm/tests/shell_s1.rs` satırlarının rustfmt yeniden sarması.

34 FLAG satırının hepsi crate/klasör yeniden adlandırması (`qp_grammar::glyph::X` →
`qp_glyph::X`, `qp_grammar::open` → `qp_designs::open`, `crate::error::GrammarError` →
`qp_look::GrammarError` …); aynı adlı başka bir öğeye yönlendirme yok. 10 REEXPORT: yeni
crate'lerin `pub use`'ları, `qp_grammar::Designs`, `qp_grammar::motion`'ın kalkması.

### G2b elle karşılaştırma (davranış korunuyor mu)

- **`compile` başlık şablonu.** Eski: `face` `Some(n)` → `faces::headline_template(Some(n))?`,
  `None` → `None`. Yeni: `(opts.designs.headline_template)(opts.face.as_deref())?`;
  `DESIGNS.headline_template` `Some(n)` → `faces::headline_template(Some(n))`, `None` →
  `Ok(None)`. Aynı sırada, aynı hata yolu. **Eşdeğer.** (Uyarlayıcı doğru: `faces::headline_template(None)`
  varsayılan yüzün şablonunu döndürürdü; uyarlayıcı onu çağırmıyor.)
- **`prepare` iki çağrı noktası.** `crate::open::field_text` → `(fr.designs.field_text)`,
  `crate::open::text` → `(fr.designs.open_text)`; `DESIGNS` bunları `open::field_text` ve
  `open::text`'e bağlıyor; argümanlar ve sıra aynı. **Eşdeğer.**
- **`terms::mark` + `underline_layer`.** `open::layer_term`/`voice_terms_in` → tablo alanları
  (`DESIGNS`: `open::layer_term`, `open::voice_terms_in`). Altı çizme gövdesi ayrı fonksiyona
  sözcüğü sözcüğüne çıkarıldı; `layer.id.clone()` ↔ `layer.to_owned()` (`&str`), `panel.rect`
  ↔ `hit` (Copy), `layer.prims` ↔ `prims`; ilk `Prim::Text` arama ve `push` sırası aynı.
  **Eşdeğer.**
- **`Frame`.** Yalnız `designs: &'static Designs` alanı; `headline` tipi yol değişikliği.
- **`CompileOptions`.** Tek yeni alan `designs`; `Debug`/`PartialEq` türetmeleri korunuyor,
  `Designs` eşitliği ve `Debug`'ı ada göre (PLAN §3.1). `Default` yok; yapı değişmezi ile
  kurulan yer yok (aranıp bulunmadı), yani her kurulum `with_designs` üstünden.
- **`designs.rs`.** Fonksiyon göstergesi tablosu, `dyn` yok; tam 5 çağrı noktasının alanı
  (`voice_terms_in` ile 5 işlev + `headline_template`; plan "5 nokta"yı scene 1, prepare 2,
  terms 2 sayıyor — alan sayısı 5, uyumlu).

### Varsayılan tablo herkese ulaşıyor mu

`without_designs`/`designs::NONE`/`with_designs` kullanımları, `qp-grammar/tests/` dışında:
yalnız `compile.rs` (tanım), `qp-grammar/examples/bench_compile.rs` (derleyicinin kendi
benchi; yüz/açık tasarım kullanmıyor) ve `qp-designs/src/registry.rs` (`DESIGNS` ile). **Ürün
kodunda `NONE` yok.** RF1 öncesi `CompileOptions::new` çağıran her ürün yeri (qp-cli `scene`,
`r0_scene`; qp-mcp `render`; qp-wasm `r0_scene`; qp-session `view_digest`, viewer `options`,
`hit`, `compiled_panels`, `preview`, `reader_scene`, `tap_at`) artık `qp_designs::options`
çağırıyor.

### Gramerin kendi sınamaları `NONE` ile

`qp-grammar/tests/` (depth, errors, hostile, lod, r0_equivalence, roundtrip, views,
design_check, glyph_alphabet) içinde `face` seçeneği, `ask/…` ya da `open.*` alanı,
`open-trust` katmanı (tesir'in terim sahibi katmanı) **yok** (aranıp doğrulandı). Bu yüzden
`NONE` ile `DESIGNS` bu sınamalarda aynı sonucu verir: başlık şablonu her iki tabloda `None`,
alan metinleri `None`, `voice_terms_in` her katmanda `true`. Yüz/açık tasarım kapsamı
kaybolmadı: o sınamalar (`capsule_depth`, `open_tesir`, `stories_korda`, `story_pavka*`,
`charac`, `faces.rs`/`open.rs` içi testler) `qp-designs`'a taşındı ve `qp_designs::options` /
`crate::options` ile derliyor.

### 15 rastgele taşınmış öğe (tohum 20261123)

`open.rs::OpenDesign`, `shell/look.rs::take_css`, `font_faces`,
`qp-wasm/tests/stories_korda.rs::zoomed_out_stories_cluster_and_a_cluster_zooms_in`,
`fonts.rs::font_registry_matches_the_looks`, `siluet.rs::confidence_plate_keeps_the_look_floor`,
`draw.rs::est_width`, `qp-designs/tests/stories_korda.rs::closed_marks_and_lane_text_keep_their_contrast`,
`scenes.rs::capsule_scene`, `input.rs::transition_frame`, `frame_at`, `open.rs::tap`,
`zemre.rs::heights_fit_the_phone`, `compile.rs::fit_text`, `cmd/scene.rs::look_cmd`.
Farkları elle okundu: yalnız yol (`qp_grammar::look` → `qp_look::look`, `qp_grammar::open` →
`qp_designs::open`, `depth::face_view_in` → `qp_designs::depth::face_view_in` — aynı ileten
gövde) ve belge yorumu yolları. Davranış farkı yok.

## 4. `DECISION.md` "Uygulama notları"

Not dürüst ve öğeyi adıyla anıyor: `crates/qp-grammar/src/advances.rs` →
`crates/qp-look/src/advances.rs`, üretici aynı (`QP_BLESS=1 cargo test -p qp-render --test
fonts`, yalnız yazdığı yol değişti — `fonts.rs` farkı bunu doğruluyor), dosya bayt özdeş,
`arch-config.json` girdisi yeni yolu gösteriyor ve gerekçesi notu anıyor. PLAN §9 B15–B19
gerçekten "`advances` taşıması `rule_changes` gerekçesiyle `DECISION.md`'yi anar" diyor.
Bu bir gevşetme değil, aynı istisnanın yer değiştirmesi; `config-diff` bunu LOOSER + removed
olarak gösteriyor ve gerekçeli sayıyor. İ4 (`qp-grammar` crate.lines, tavan 21.000) kaldırıldı;
qp-grammar 7.449 ile genel 12.000 kuralının altında.

## 6. Yeni crate'ler

- README: üçünde de var (katman, bağımlılıklar, dışa açık yüzey, düzen, "ne değildir").
- `[lints] workspace = true`: üçünde de var.
- `lib.rs` yalnız bildirim: qp-look (`mod` + `pub use`), qp-glyph (`mod contract` +
  `pub use`), qp-designs (`mod`/`pub mod` + `pub use registry::{options, DESIGNS}`) — kod yok.
- Klasör başına dosya: üç crate'te en kalabalık klasör ≤ 12 (sınır 15).
- Bağımlılık yönü: qp-look → qp-schema; qp-glyph → qp-schema, qp-look; qp-designs → qp-schema,
  qp-look, qp-glyph, qp-grammar. qp-designs'ın `qp-capsule` geliştirme bağımlılığı (yukarı
  katman, yalnız test SHA-256'sı) RF1 öncesi qp-grammar'ın aynı geliştirme bağımlılığının
  taşınması.

**`pub(crate)` → `pub` genişletmeleri** (hepsi crate sınırı doğduğu için; her biri yeni
üst crate'te kullanılıyor):

| Öğe | Kullanan |
|---|---|
| `qp_grammar::compile::draw` modülü (`pub mod draw`), `draw::rect`, `draw::wrap` | qp-designs `stories/korda.rs` (`rect`), `open/nobra.rs` (`wrap`) |
| `depth::{finding_and_fact, bar_series_bars, main_field, layer, time_scale}` | qp-designs `open/tesir/view.rs` (ve `layer` yüzlerde) |
| `depth/card::{has_story, panel_at, named_panel, uniq}` | qp-designs `open/tesir*.rs`, `faces/tavrik.rs` |
| `qp_look::error::GrammarError::{new, in_layer, with_layer}` | `new`: qp-glyph, qp-designs; `in_layer`/`with_layer`: qp-grammar `compile` |

Gerekçeli. Ama bkz. B1: listelendikleri yer yok.

## Bulgular

- **B1 (orta, kapanıştan önce): `experiments/paths.md` yok.** PLAN §0 ("Dışa açık Rust yolu
  değişiklikleri `experiments/paths.md`'ye yazılır") ve §9 B17 ("`pub` yapılanlar
  `paths.md`'ye") bunu istiyor; dalda ne `paths.md` ne başka bir liste var. Bu round'un en
  büyük dışa açık yol değişikliği bu dal: `qp_grammar::{look, theme, error, detmath, advances,
  glyph}` → `qp_look`/`qp_glyph`; `qp_grammar::{faces, open, stories, feeds, transitions,
  motion}` → `qp_designs`; `qp_grammar::depth::{is_face, face_view, face_view_in,
  story_design}` → `qp_designs::depth`; `faces::HeadlineFn`/`headline_text`,
  `open::DeviceCheck` → `qp_grammar::designs`; `CompileOptions::new` kalktı →
  `qp_designs::options` / `CompileOptions::without_designs`; yukarıdaki genişletme tablosu.
  Davranışı etkilemez; belgeleme borcu.
- **B2 (düşük): kırık/eskimiş belge bağlantıları.** `cargo doc` yeni uyarılar veriyor:
  `qp-designs/src/depth.rs:22` [`capsule_view`], `qp-designs/src/faces.rs:20`
  [`HeadlineParts`], `qp-grammar/src/compile.rs:87` [`crate::faces::FaceDesign::headline`],
  `qp-grammar/src/provider.rs:29` [`crate::faces::HeadlineFn`]; yorumlarda eski yollar
  (`compile.rs:85` `crate::faces`, `prepare.rs:94,115` `crate::open`). (`qp-look`
  `look/fonts.rs:8` [`TypeRole`] RF1 öncesinden.) `verify.sh` `cargo doc` koşmuyor; Q
  ("eskimiş yorumlar") kapsamına uygun.
- **B3 (düşük): `draw::wrap` artık `pub`, belgesi `pub(crate)` `est_width`'e bağlanıyor**
  (`rustdoc::private_intra_doc_links`; aynısı `terms` modül belgesinde). Ya `est_width`
  da `pub` (ya da `#[doc(hidden)]`), ya bağlantı düz metin.
- **N1 (bilgi):** `NONE.headline_template` bilinmeyen yüz için kendi iletisini veriyor
  ("no designs registered"); yalnız gramerin kendi sınamalarında görülebilir, ürün yolunda
  değil.
- **N2 (bilgi):** WASM artışı küçük ama sıfır değil (+%0,2…0,5): fonksiyon göstergesi
  tablosu ölü kod elemesini `open`/`faces` çağrı noktalarında tutuyor; ±%2 içinde.

## Temizlik

`.claude/worktrees/audit-g2` kaldırıldı (`git worktree remove`), `target/wt/audit-g2`
silindi, geçici paket klasörü ve günlükler (oturum scratchpad'i) silindi. Süreç kapatma
gerekmedi; `pkill`/`killall` kullanılmadı; `.env.local` okunmadı.
