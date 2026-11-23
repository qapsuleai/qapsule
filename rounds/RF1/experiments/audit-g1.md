# RF1/G1 bağımsız denetim: `rf1/g1-charac` @ f506835f

**Hüküm: GEÇTİ** (iki küçük bulguyla; ikisi de birleştirmeyi engellemez, G2 öncesi dikkate alınmalı).

Denetçi kendi worktree'sinde (`.claude/worktrees/audit-g1`, ayrık HEAD) ve kendi derleme
klasöründe (`target/wt/audit-g1`) çalıştı. Denetlenen dala yazılmadı, birleştirme yapılmadı.

## 1. Fark kapsamı

`git diff main...rf1/g1-charac --stat`: 19 dosya, +3900 / −0, tek commit (f506835f).

- `crates/qp-grammar/tests/charac/{main,snap,fixture,prepare,designs,present}.rs`
- `crates/qp-grammar/tests/charac/expected/*.txt` (12 kitap)
- `rounds/RF1/experiments/new-tests.txt` (+11 ad)

`crates/*/src` altında değişiklik yok; `Cargo.toml` değişmedi (gerekli `qp-capsule`
geliştirme bağımlılığı zaten vardı). **Geçti.**

## 2. fmt, clippy, test

| Adım | Sonuç |
|---|---|
| `cargo fmt --all -- --check` | temiz |
| `cargo clippy -p qp-grammar --all-targets -- -D warnings` | uyarısız |
| `cargo test -p qp-grammar` (1. koşu) | tümü yeşil; `charac` 11/11 |
| `cargo test -p qp-grammar` (2. koşu) | tümü yeşil, aynı sayılar (deterministik) |

Koşulardan sonra worktree'de izlenen dosyada değişiklik yok (beklenen kitaplar yeniden yazılmadı).

## 3. Duyarlılık (kasıtlı davranış değişikliği → kırmızı mı?)

Her değişiklik tek tek uygulandı, `cargo test -p qp-grammar --test charac` koşuldu,
`git checkout` ile geri alındı ve `src` temiz doğrulandı.

| # | Değişiklik | Kırmızı olan sınamalar |
|---|---|---|
| a | `faces/zemre.rs` başlık şablonu `"{}: {} · {}, {}"` → `" - "` | face_headlines, depth_rules |
| a2 | `faces/tavrik.rs` başlık birleştirici `" · "` → `" ~ "` | face_headlines, depth_rules |
| b | `open/nobra/answers.rs` `"{w} bağımsız kanıt ailesi"` → `…ailesi!"` | **hiçbiri (yeşil kaldı)** — bkz. Bulgu 1 |
| b2 | `open/tesir.rs` `"… herkese açık, cihaz …"` → `"…; cihaz …"` | open_fields_and_terms, open_designs |
| b3 | `open/nobra/answers.rs` `"Hâlâ geçerli; ikinci bir kanıt ailesi…"` → `":"` | open_fields_and_terms, open_designs, prepare_layer_matrix |
| b4 | `open/nobra/answers.rs` `"tek bir kanıt ailesi"` → `"tek kanıt ailesi"` | open_designs |
| c | `compile/terms.rs` terim altı çizgisi `size * 0.18` → `0.25` | open_fields_and_terms, open_designs |
| d | `transitions/nabiz.rs` öğe saydamlığı `smooth(p, 0.0, 0.7)` → `0.8` | transition_frames |
| e | `compile/layer/prepare.rs` uzam dalı `a.max(b)` → `a.max(b) + 1` | prepare_layer_matrix, face_headlines, open_designs, depth_rules |
| e2 | `compile/layer.rs` `default_opacity` `Area => 0.25` → `0.3` | prepare_layer_matrix |

İstenen beş sınıfın (a–e) hepsi en az bir kasıtlı değişiklikte kırmızı oldu. Açık kapsül alan
metni sınıfında ilk denenen dal yakalanmadı; aynı sınıftan üç başka metin yakalandı.

## 4. Yol bağımsızlığı

Sınamalar yalnız dışa açık API'yi kullanıyor (`pub(crate)` ya da iç modül yok). Kullanılan yollar:

- **Gramerde kalacaklar** (planlanan bölünmeden etkilenmez): `qp_grammar::{compile, CompileOptions,
  Cursor, MemProvider, DataProvider, HeadlineParts}`.
- **`qp-look`'a inecekler:** `qp_grammar::look::{or_default, names, Look, R4}`,
  `qp_grammar::GrammarError` (`error`). Kök yeniden dışa aktarımı korunursa `GrammarError` kırılmaz.
- **`qp-glyph`'e inecekler:** `qp_grammar::glyph::{ALPHABETS, outcome::names}`.
- **`qp-designs`'a çıkacaklar:** `qp_grammar::faces` (`faces::names`, yüz çağrıları),
  `qp_grammar::open::{self, Check, DeviceCheck, OpenCtx, OpenTap, TextInput, capsule_view,
  nobra::ASKS}`, `qp_grammar::stories::{self, StoryInput}`, `qp_grammar::feeds::{self, FeedCtx,
  LocalStamp}`, `qp_grammar::transitions::{self, nabiz, Side}`, `qp_grammar::motion::{self,
  ChangeKind}`, `qp_grammar::depth` (tasarım sarmalayıcıları: `depth::chart_view_in` vb.).
  `DeviceCheck` planda gramerde kalan tip olarak geçiyor; o yol değişmeyebilir.

Kırılacak yerlerin hepsi mekanik yol düzeltmesidir, ama **yalnız `use` satırları değil**:
gövde içinde tam nitelikli `qp_grammar::…` yolu kullanan 22 satır var; bölünmede kırılacak 13'ü:
`look` → `designs.rs:18-19`, `present.rs:41-42`, `fixture.rs:22`, `prepare.rs:259`; `open` →
`designs.rs:217`, `prepare.rs:251,257`; `depth` → `designs.rs:349`; `glyph` → `prepare.rs:362-363`;
`faces` → `present.rs:147`. Kalanlar (`GrammarError`, `MemProvider`, `DataProvider`) kökten gelir.
Bunlar da yalnız yol önekidir; davranış ya da beklenti değişikliği gerektirmez.

Yol dışında G2'de dokunulması muhtemel iki nokta (Bulgu 2):

- `CompileOptions::new(…)` 8 yerde (`fixture.rs:373`, `designs.rs:352`, `prepare.rs:337,365,382,412,423,426,475`).
  §3.1'e göre tasarım tablosu dışarıdan gelir ve çağıranın yolu `qp_designs::options(…)` olur.
  Yüz (`o.face`) kullanan `fixture::compile_in` ve `bad.face` vakası, tablo verilmezse farklı çıktı
  verir. Bu yüzden bu 8 kurucu çağrısı da (en azından yüz/tasarım kullananlar) yol düzeltmesi
  sayılmalı; beklenti kitapları **yeniden yazılmamalı**.
- `snap.rs` SHA-256 için `qp_capsule::trust::seal::sha256` kullanıyor (tek yerde, yorumla
  işaretli). Sınamalar `qp-designs`'a taşınırsa `qp-designs → (dev) qp-capsule → qp-designs`
  geliştirme döngüsü oluşur; bütünleşik sınamalar için Cargo bunu kabul eder, ama sağlayıcıyı
  değiştirmek tek satırlık iştir.

Sınamaların G2'de nerede duracağı (gramerde kalıp `qp-designs`'ı geliştirme bağımlılığı olarak
almak ya da `designs.rs`/`present.rs` kitaplarıyla birlikte `qp-designs/tests`'e taşınmak) planda
açık değil; taşıma yapılırsa kitaplar bayt eşit taşınmalı.

## 5. Kutsama anahtarı, kitaplar, boyut, adlar

- `QP_CHARAC_BLESS`: yalnız tam `"1"` değeriyle tetiklenir (`snap.rs:99`). Depoda (betikler,
  `.cargo/config.toml`, CI) bu değişkeni koyan yer yok; ortamda tanımlı değil. Kitap dosyası
  eksikse boş kabul edilip **kırmızı** olur (sessiz geçiş yok). **Geçti.**
- Kitaplar commit edilmiş ve önemsiz değil: 12 dosya, toplam 1751 satır (compile-options 32,
  depth 416, faces 255, feeds 24, glyph-styles 111, motion 317, open-fields 171, open 76,
  prepare-errors 36, prepare-matrix 66, stories 59, transitions 188). Satırlar özet + SHA-256
  önekidir (sahne JSON'u ya da `Debug` dökümü); prepare matrisi 9 işaret (candle, bar, line, area,
  band, rule, point, text, glyph) × kaynak (seri, olgu, bulgu, sabit, yok) × 6 görünüş + açık tema.
- Dosya boyları ≤ 800: en büyüğü `fixture.rs` 551 satır. **Geçti.**
- Adlar: 11 `charac_*` sınaması, her biri iş alanında tek (`fn <ad>(` bir kez) ve hepsi
  `new-tests.txt`'de. Kitap içi vaka adlarının tekliği `Book::put` içinde iddia ile korunuyor.
  **Geçti.**

## Bulgular

1. **(Düşük) Kapsama boşluğu:** `open/nobra/answers.rs::families_of` içinde "aile adı yok ve
   `confirmations > 1`" dalı (`"{w} bağımsız kanıt ailesi"`, ve büyük olasılıkla İngilizce eşi)
   hiçbir vakada üretilmiyor; değişikliği yeşil kaldı. Fikstüre aile listesi boş, iki ve üzeri
   teyitli bir bulgu eklenmesi önerilir (bu dal G2'de yukarı taşınacak koddadır).
2. **(Bilgi) G2 için yol düzeltmesi `use` satırlarından geniş:** 13 gövde içi tam nitelikli yol
   ve 8 `CompileOptions::new` çağrısı (tasarım tablosu kurucusuna geçmeli). Hepsi mekanik; kitaplar
   değişmemeli. Düzeltme kolaylığı için yolların `use` satırlarına toplanması önerilir.

## Temizlik

Worktree `.claude/worktrees/audit-g1` ve `target/wt/audit-g1` (≈1,2 GB) denetim sonunda silindi.
