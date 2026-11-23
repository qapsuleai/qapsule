# RF1/G3 bağımsız denetim — `rf1/g3-prepare` @ a23f466a

Denetçi: ayrı alt ajan (işi yapan ajan değil). Dal değiştirilmedi, birleştirilmedi.
Ortam: `git worktree add .claude/worktrees/audit-g3 rf1/g3-prepare --detach`,
`CARGO_TARGET_DIR=target/wt/audit-g3` (iş bitince ikisi de silindi). Dalın tabanı db6f992d;
`main` (6e50be09) yalnız `rounds/RF1/experiments/notes.md`'ye bir satır ekliyor, dal o
dosyaya dokunmuyor (çakışma yok).

## Hüküm: **GEÇTİ**

`prepare` adlandırılmış adımlara bölünmüş; deyimler, sıra, hata iletileri, erken dönüşler ve
palet sayacının sırası eskisiyle aynı. Davranış korunmuş (270/270 bayt eşit, kitaplar
değişmedi), yapı ölçütleri yeşil. Aşağıdaki bulgular birleştirmeyi engellemez.

## Sonuçlar

| Adım | Sonuç |
|---|---|
| 1 kapsam `git diff --name-only main...rf1/g3-prepare` | 11 dosya: `qp-grammar/src/compile/layer/` altında `prepare.rs`, `prepare/{source,series,rule,panel,point}.rs`, `README.md`, `prepare/README.md`; ayrıca `docs/DEBT.md`, `scripts/arch-config.json`, `scripts/arch-baseline.json`. Kapsam dışı dosya **0**; test ve `expected/` dosyası değişmedi |
| 2 `cargo fmt --all -- --check` | 0 |
| 2 `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| 2 `cargo test --workspace` | 0 — 1222 geçti, 0 kaldı, 9 yok sayıldı (192 ikili); `qp-designs/tests/charac` 11/11 geçti; koşudan sonra ağaç temiz (kitap yeniden yazılmadı) |
| 3 `scripts/refactor/compare.sh` | **270/270** bayt eşit (exit 0) |
| 6 `scripts/arch-check` | 0 hata; 3 adlı istisna (prepare.rs İ6 yok); taban içinde, `fn.lines` 176 |
| 6 `--config-diff main` | 4 değişiklik, **0 gerekçesiz gevşetme**: İ6 `file.hard prepare.rs` kaldırıldı, `fn.lines prepare` kalemi kaldırıldı, `fn.lines` 177→176, `lint.allow qp-grammar` 13→11 (hepsi sıkılaştırma) |
| 6 `--debt-md` | yeniden üretim daldaki `docs/DEBT.md` ile bayt bayt aynı |
| 6 `moved.py main HEAD` | 31 CHANGED, hepsi `prepare.rs` ve `prepare/*.rs` içinde (eski `fn prepare` − ; yeni 25 fn, 2 struct, 1 type); 5 FLAG (beş alt dosyada `use super::*;`; araç hepsini `source.rs` adıyla yazıyor, dosya başına bir tane); 1 path-only `use` (`prepare.rs`: `qp_look::theme::Theme`). Başka CHANGED yok |
| 6 fn ≤ 80 | arch-check yeni `fn.lines` bulgusu vermiyor; en uzunu `point_text` ≈75, `glyph` ≈76 satır |
| 6 dosya ≤ 600 | `prepare.rs` 229, `panel.rs` 207, `point.rs` 208, `series.rs` 192, `rule.rs` 185, `source.rs` 129 |
| 6 lint | `#[allow(clippy::cognitive_complexity)]` ve `#[allow(clippy::too_many_lines)]` kalktı (13→11); tek kalan `allow` `rule.rs::vertical` içindeki eski `cast_possible_truncation, cast_sign_loss` |

## Davranış eşdeğerliği (okuyarak, 4)

Eski `prepare` (`git show main:…/prepare.rs`, 891 satır) yeni adımlarla deyim deyim
karşılaştırıldı.

| Eski blok | Yeni yer | Not |
|---|---|---|
| `n_data` sayımı ve "at most one" hatası, `resolve_constants` | `prepare` | aynı sırada, kaynak çözümünden önce |
| `series`/`fact`/`finding`/yok seçimi, `all_rows` | `source::layer_source` | `finding_rows` (döngü, `confidence`/`claim`) sonra `capsule_texts` (voice → headline → ledger → detail → `field_text` → satır döngüsü → `open_text`); sıra ve iletiler aynı |
| `Ctx` kurulumu | `prepare` | aynı alanlar |
| opacity → width → `x` → `text_edge`/`x2` → `extent` | `check_encoding` | aynı sıra; `extent` yalnız iki denetimden sonra |
| `label`, boş `anchor/glyphs/seg_extent/skipped/keys/fields/traces`, `width = enc.width.unwrap_or(1.0)` | `start_state` (`PrepState`) | `check_encoding`'den sonra, işaret adımından önce — eskisiyle aynı konum; `fr.p.label` yan etkisiz |
| iç `fn main_or_err` | modül fn `main_or_err` | gövde aynı |
| `next_palette` kapanışı (`theme`, `palette` yakalar) | `next_palette(theme, palette, rule_given)` | aynı: önce `theme.palette(*palette)` sonra `+= 1`; yalnız renk verilmemişse |
| `match layer.mark` kolları | `mark_rows` | kol sırası aynı (`Text if is_panel…` koruması `Glyph`'ten ve `Point|Text`'ten önce) |
| Candle | `series::candle` | aynı; `keys = vec![…]` atama, `fields.clone_from` |
| Line | `series::line` | `width = 1.5` hatalardan önce (eskisi de); palet her kanal için `color_rule`'dan önce |
| Area/Band/Bar | `series::area_band_bar` | `y` → `y2` → band `y2` hatası → palet (yalnız area/band) → `color_rule` → `pair_rows(extent)`; aynı |
| Rule | `rule::rule` + `segment`/`horizontal`/`vertical` | `color_rule` ve `mk` önce; dallar aynı sırada; `konst` kapanışı aynı yakalamalarla `segment` içinde; `vertical`'ın hatası `keys.push`'tan önce (eskisi de) |
| panele göre Text | `panel::panel_text` (+`panel_text_size`) | anchor → kaynak denetimi → `text_field_of` → `y` → boyut → `color_rule`; aynı |
| Glyph | `panel::glyph` (+`glyph_channels`, `glyph_size`) | finding denetimi → kanal yasakları (aynı sıra) → `p.glyph` → boyut → anchor → `y`; aynı |
| Point/Text | `point::point_text` (+`point_text_field`, `point_size_chan`, `point_source_rows`, `scale_sizes`) | `main_or_err` → metin alanı (üç denetim aynı sırada) → `color_rule` → boyut kanalı → `const_size` → `push` kapanışı (aynı yakalamalar; `skipped` yine parametre) → kaynak satırları → ölçekleme |
| `Ok(Prepared {…})` | `finish` | alanlar aynı; `edge = enc.width.is_some()`, `has_data = n_data == 1` |

**Yeniden sıralama: yok.** Hesaplanan her değer eskisiyle aynı yerde ve aynı sırada
hesaplanıyor. Tek biçimsel fark: eski kodda `geom`'a her kolun başında atanıyordu, yenide kol
fonksiyonu `Geom` döndürüyor; hata yolunda `geom` hiç kullanılmadığından gözlenemez. Hata
yolunda `PrepState`'e yazılmış ara değerler (ör. `line`'ın `width`'i, `panel_text`'in
`anchor`'u) eskisinde de yerel değişkenlerdeydi ve `Err` ile atılıyor. `palette` hata yolunda
da eskisi gibi artmış kalır (çağıran tarafından görülebilen tek yan etki; sıra aynı).

## Duyarlılık (5)

Yeni adım fonksiyonlarına bilinçli değişiklik, `cargo test -p qp-designs --test charac`, geri al
(`git checkout`; ağaç temiz, HEAD a23f466a).

| # | Değişiklik | Sonuç |
|---|---|---|
| M1 | `check_encoding`: opacity ve width denetimlerinin yeri değişti | **yeşil kaldı** (beklenen; aşağıda B1) |
| M1b | `check_encoding`: `extent` hesabı `x2` işaret denetiminin önüne alındı | **kırmızı** — `charac_prepare_layer_matrix`; `err.x2_on_point` `/encoding/x2` yerine `/encoding/x: \`x2\` needs \`x\`` verdi |
| M2 | `next_palette`: önce `+= 1`, sonra `theme.palette` | **kırmızı** — 5 test (`charac_open_designs`, `charac_face_headlines`, `charac_depth_rules`, `charac_compile_options`, `charac_prepare_layer_matrix`) |
| M3 | `point_text_field`: "…of a fact or finding" → "…of a fact/finding" | **kırmızı** — `charac_prepare_layer_matrix`, `text.series` satırı |

## Bulgular

- **B1 (bilgi, engel değil).** İki bağımsız denetimin yer değiştirmesi ancak ikisini birden
  ihlal eden bir girdiyle görülür; kitapta opacity ve width'i birlikte bozan durum yok (M1
  yeşil kaldı). Bu bölmede sıra okuyarak doğrulandı, değişmedi. Kitaba "birden çok hata"
  durumları (ör. `opacity: 1.5, width: 0`; glyph'te `color` + kapsülsüz finding) eklemek
  ileride sıra değişikliklerini yakalar — öneri, bu round'un kapsamı dışında.
- **B2 (süreç notu).** `arch-config.json` `_exceptions_doc` "istisna eklemek ya da kaldırmak
  karar satırı ister" diyor; `DECISIONS.md`'de İ6 (ya da G2'de kapanan İ4) için kapanış satırı
  yok. `--config-diff` kaldırmayı sıkılaştırma sayıyor ve geçiyor; kapanışın round raporunda
  ya da bir karar satırında anılması ana akışın kararı.
- **B3 (bilgi).** `PLAN.md §3.1` adımları (`layer_source`, `capsule_texts`, `check_encoding`,
  `mark_rows`, `finish`, `PrepState`) adlarıyla var; ek olarak `start_state`, `Encoding` ve işaret
  başına alt adımlar doğdu. `series.rs`'te eski `let _ = base_default;` (ölü değişken) olduğu
  gibi taşındı — davranış kuralına uygun, temizliği ayrı iş.

## Temizlik

`.claude/worktrees/audit-g3` kaldırıldı (`git worktree remove`), `target/wt/audit-g3` silindi.
Süreç adla kapatılmadı; `.env.local` okunmadı.
