# RG1 — Bağımsız denetim

Denetçi: bağımsız denetçi alt ajanı (işi yapmadı), 2026-11-22. Denetlenen durum: `main` =
`07430545` (dal `rg1/audit`, kod değişikliği yok). Derleme klasörü özel:
`target/wt/audit` (denetim sonunda silindi). Karalama yalnız oturum klasöründe (`audit/`), sonra
silindi. `rounds/RG1/REPORT.md` denetim anında yoktu; depo durumu denetlendi, rapor değil.

## Hüküm: **Koşullu kabul**

Davranış değişmezi tuttu: tutanak karşılaştırmasını kendim, özel derleme klasörüyle yeniden
koştum — **225/225 kalem bayt eşit** (wasm32 ve x86_64 özetleri dahil); `fixtures/` altında
tutanaktan bu yana fark yok; `verify.sh` 19/19 PASS (benim koşum, özel derleme klasörü, 688 s; `verify: OK`). 20 rastgele taşımanın 15'i saf taşıma,
5'i yalnız görünürlük; `rg1-moved`'in tur boyunca bildirdiği 128 CHANGED kalemin hepsi izinli
türden (17 bağlanan sabitin ilklendiricisi, yeni `params` kodu ve 14 testi, §13.3 sarmalayıcı
kaldırma ve çağrı yerleri, bir `include_str!` yolu, `#[allow]`+yol birleşimi iki test). Gövde,
ad ya da sabit değeri değişikliği bulmadım; `NORTH_STAR.md` yalnız K84'lü ana akış commit'iyle,
`docs/ENGINEERING.md` hiç değişmedi. Kabulü koşullu yapan üç şey: (1) kesin sınırların bir kısmı
**adlı istisna isteklerine** dayanıyor (İ1 olmadan bir yukarı bağımlılık `qp-session → qp-source`
ve sekiz yan bağımlılık var; iki dosya 800'ü aşıyor; üç crate 12 bini aşıyor; kabuk WASM paketi
gzip +%2,54) — bunlar Arkin onaylamadan "karşılandı" sayılamaz; (2) `arch-check` bazı basit
hilelerle kandırılabiliyor (B1–B6; en ciddisi lint susturmanın sayılmayan biçimleri ve
`cfg(test)`'siz `tests.rs` muafiyeti); (3) kapanış işleri (REPORT, STATUS, ROADMAP, `target/`
76 GB → ≤ 6 GB, CI) denetim anında yapılmamıştı.

## 1. Kabul ölçütleri

| # | Hüküm | Kanıt |
|---|---|---|
| 1 | ✅ | `git log -- rounds/RG1/experiments/baseline` → `6d03df21` (ilk), `26c513d7` (genişletme); ilk kod taşıması `bce705b6`'dan önce. Tutanakta 225 kalem, `perf.txt`, `lines.txt`, `verify.log`. |
| 2 | ✅ (CI ⚠️) | `scripts/rg1-compare.sh` (benim koşum): "225 items byte-identical to the baseline", çıkış 0; `tests.count` 1164, `tests.allow-missing` yok. `git diff --stat 6d03df21 HEAD -- fixtures/{golden,compute,protocol,views,schema,bars,oracle}` boş; `27656b1..HEAD -- fixtures` da boş. `verify.sh`: 19/19 PASS (benim koşum, özel derleme klasörü, 688 s; `verify: OK`). CI: denetçi push etmez; doğrulanamadı. |
| 3 | ⚠️ | K17: `perf-final.txt` A/B (aynı saat, `27656b1` vs HEAD) gürültü içinde eşit. WASM adım 8: ham +0,05 %, gzip +0,17 % ✅. **Kabuk WASM paketi: ham 8.923.686 → 9.101.130 (+1,99 %), gzip 2.506.216 → 2.569.974 (+2,54 %)** — ±%2'nin üstünde; İ7 isteği gerekli (A63 "+%2,36" diyor; kayıtlı dosyalardan hesap +%2,54 → B10). K52: tutanaktaki K52 bölümü kesik (yalnız JSON'un son 40 satırı), `frame_ms`/`open_ms` karşılaştırılamıyor; görünen aralıklar tek koşuda %8–28 yukarıda (B9). |
| 4 | ⚠️ | `scripts/arch-check`: 0 hata, `verify.sh` adım 19, `--self-test` PASS. İ1 ile yukarı 0, yan 5 (hepsi İ2 adıyla). **İ1 olmadan** (yapılandırmadan `layer_overrides` çıkarılarak, `rules.rule_layers`): `layer.up qp-session -> qp-source` 1; yan 8 (`qp-capsule→qp-facts`, `qp-host→{qp-live,qp-matriks,qp-mcp,qp-shellcore,qp-source}`, `qp-matriks→qp-source`, `qp-shellcore→qp-source`). "Yukarı 0" İ1 onayına bağlı. |
| 5 | ⚠️ | 800 üstü `src` dosyaları tam olarak: `qp-session/src/ops/expand/op.rs` 910, `qp-grammar/src/compile/layer/prepare.rs` 891 (ikisi İ6, tek fonksiyonlu) ve üretilmiş `qp-grammar/src/advances.rs` 878 (`generated_files`'ta adıyla + başlıkta GENERATED). İ6 onayına bağlı. |
| 6 | ✅ | `arch-check --table`: `qp-cli` 1.420, `qp-wasm` 1.324 test dışı satır. (Kabukta kalan mantık için B12.) |
| 7 | ⚠️ | 12 bin üstü: `qp-session` 31.999, `qp-grammar` 19.330, `qp-capsule` 13.230 (İ3–İ5, onay bekliyor; `qp-session` boyutu hiç küçülmedi, yalnız klasörlendi). 15 üstü klasör: yalnız `qp-session/tests` 39, `qp-wasm/tests` 20, `qp-compute/tests` 16 — M5 kesildi (kesme sırası 2), taban çizgisinde. `src` altında 15 üstü klasör yok. Yasak ad: `crates/` altında 0 (çalışma alanı dışındaki `experiments/t6*/src/common.rs` taranmıyor, kapsam dışı). |
| 8 | ✅ | `ls crates/*/README.md` = 16 = crate sayısı; 76 modül klasörü README'si. `root.logic` 0; ayrıca bütün `lib.rs`/`mod.rs`'te üst düzey makro çağrısı ve `include!` olmadığını ayrıca taradım (arch-check bunları görmez → B2). |
| 9 | ✅ | `docs/DEBT.md` `--debt-md` ile yeniden üretince bayt eşit; `arch-baseline.json` 238 kalem (fn.lines 177, file.soft 37, lint.allow 13, test.file 6, dir.files 3, doc.size 2). Mandal: öz-sınama + `ratchet.txt` + benim denemem (yeni 104 satırlık trait yöntemi → hata; taban çizgisindeki fonksiyonu yeniden adlandırma → hata; dosyayı taşıma → hata). Elle taban çizgisi düzenleme yakalanmıyor (B5). |
| 10 | ✅ (B1) | Karalama kopyasında `qp-calendar`'a `x.unwrap()` ekledim: `cargo clippy -p qp-calendar -- -D warnings` → `error: used unwrap() on an Option value`. Mevcut ihlaller öğe başına `#[allow]` (13 crate'te sayılı). Ama `#[allow(clippy::restriction)]`, `#[expect(clippy::unwrap_used)]`, `#[cfg_attr(all(), allow(…))]` derlemeyi de `arch-check`'i de geçiyor (B1). |
| 11 | ✅ | `qp_schema::params` (`ParamDef` 10 alan, `ParamScope`, saf `resolve`), `docs/PARAMETERS.generated.md` (16 anahtar; güncellik testi `params_generated_doc_is_current`), `docs/PARAMETERS.md` envanteri (65 tablo satırı). **17 sabit / 16 anahtar** bağlı; her biri için eski değişmez = `*_DEFAULT` değeri, tip aynı (tek tek karşılaştırdım, §3). Şema dışa aktarımı bayt eşit. Not: bağlama `resolve` yolundan değil adlı varsayılandan okur (B13). |
| 12 | ✅ | `scripts/rg1-decisions-check.py b341e2d 1957752`: "satır kümesi eşit: 352 satır, 12 dosya; HEAD: özgün satırların hepsi duruyor; başlık 12/12; yeni kırık 0, önceden de kırık 1 (A46)". |
| 13 | ⚠️ | `rounds/RG1/experiments/recovered.md`: 24 dalın her biri için karar satırı; `git branch --list 'recovered/*'` = 24 (silinmemiş). "Fark edilenler" ve "Kural önerileri" REPORT.md'de olacak; REPORT denetim anında yok → doğrulanamadı. |
| 14 | ✅ | Bu belge. |
| 15 | ❌ (henüz) | `git worktree list`: ana ağaç + bu denetim worktree'si. `du -sh target` = **76 GB** (bütçe 6 GB). REPORT/STATUS/ROADMAP yok. Orkestratörün kapanışı bekleniyor. |

## 2. VERIFY adımları

| # | Hüküm | Ne koştum / gördüm |
|---|---|---|
| 1 | ✅ | `git log --oneline -- rounds/RG1/experiments/baseline \| tail -1` → `6d03df21`; ilk taşıma `bce705b6` sonra. |
| 2 | ✅ | `CARGO_TARGET_DIR=target/wt/audit scripts/rg1-compare.sh` → 225 kalem bayt eşit, çıkış 0. |
| 3 | ✅ | `CARGO_TARGET_DIR=target/wt/audit scripts/verify.sh` → 19/19 PASS (benim koşum, özel derleme klasörü, 688 s; `verify: OK`). (Orkestratörün `verify-final.log`'u: 19/19 PASS.) |
| 4 | ✅ | `git diff --stat 6d03df21 HEAD -- fixtures/golden fixtures/compute fixtures/protocol fixtures/views` boş. |
| 5 | ✅ | `scripts/arch-check` → "0 error(s); 11 exception(s) awaiting approval; within the baseline: dir.files 3, doc.size 2, file.soft 37, fn.lines 177, lint.allow 13, test.file 6". |
| 6 | ✅ | `scripts/arch-check --self-test` → 20 örnek "caught", mandal ve istisna denetimleri ok, PASS. |
| 7 | ⚠️ | `find … \| sort -rn \| sed -n 2,4p` → 910 `op.rs`, 891 `prepare.rs`, 878 `advances.rs`: ikisi İ6, biri üretilmiş. |
| 8 | ⚠️ | Kabuklar < 4 bin ✅; 12 bin üstü üç crate istisna isteğiyle (İ3–İ5). |
| 9 | ✅ | Kendi denemem (karalama kopyası): yeni uzun fonksiyon, yeniden adlandırma, dosya taşıma → hata; `ratchet.txt` aynı sonucu kaydetmiş. |
| 10 | ✅ | 16 = 16. |
| 11 | ✅ | Ölçüt 11. |
| 12 | ✅ | Ölçüt 12. |
| 13 | ⚠️ | Ölçüt 3 (kabuk paketi gzip +%2,54; K52 tutanağı kesik). |
| 14 | ❌ (henüz) | `target/` 76 GB; denetim worktree'si orkestratör kaldıracak. |
| 15 | ⚠️ | AUDIT.md bu; STATUS/ROADMAP kapanışta. |

## 3. Rastgele 20 taşıma

Yöntem: `27656b1..HEAD` arasında değişen bütün `.rs` dosyalarından `rg1-moved`'in öğe kesicisiyle
öğeler çıkarıldı; adı tek olan ve dosyası değişen 2.426 öğe (16 crate) aday; **tohum
`20261122`**, crate'ler arasında karışık sırayla dönüşümlü seçim (`random.Random(20261122)`).
Her öğenin eski (`git show 27656b1:<eski yol>`) ve yeni metni satır satır (baştaki boşluk
kırpılarak) karşılaştırıldı.

| # | Öğe | Eski → yeni | Hüküm |
|---|---|---|---|
| 1 | `fn render_view` | `qp-cli/src/main.rs:909` → `qp-cli/src/cmd/scene.rs:186` | yalnız görünürlük (`pub(crate)`) |
| 2 | `fn trade_connected` | `qp-matriks/src/feed.rs:668` → `feed/trades.rs:144` | saf taşıma |
| 3 | `fn render_rgba` | `qp-wasm/src/lib.rs:150` → `bind/render.rs:74` | saf taşıma |
| 4 | `fn fact_id` | `qp-facts/src/lib.rs:324` → `fact.rs:52` | saf taşıma |
| 5 | `fn sample_series` | `qp-grammar/tests/common/mod.rs:12` → `tests/support/mod.rs:12` | saf taşıma |
| 6 | `fn pf` | `qp-compute/src/studies/mod.rs:45` → `studies/period.rs:5` | saf taşıma |
| 7 | `fn d1_breakout` | `qp-capsule/src/voice/selvi.rs:1264` → `voice/selvi/detail.rs:278` | saf taşıma |
| 8 | `struct Permissions` | `qp-schema/src/protocol.rs:668` → `protocol/envelope.rs:412` | saf taşıma |
| 9 | `struct Samples` | `qp-wasm/src/shell.rs:244` → `qp-shellcore/src/shell/metrics.rs:8` | yalnız görünürlük (`pub(super)` tip ve alan) |
| 10 | `fn all_parts` | `qp-session/src/headparts.rs:305` → `capsule/headparts.rs:305` | saf taşıma |
| 11 | `fn at_local` | `qp-calendar/src/lib.rs:270` → `exchange.rs:105` | saf taşıma |
| 12 | `fn act_keys` | `qp-mcp/src/surfaces/query.rs:394` → `query/check.rs:92` | yalnız görünürlük (`pub(super)`) |
| 13 | `fn compare_bars_with` | `qp-source/src/livebar.rs:917` → `livebar/compare.rs:106` | saf taşıma |
| 14 | `struct RepairStat` | `qp-cli/src/livehost.rs:675` → `qp-host/src/livehost.rs:214` | saf taşıma |
| 15 | `struct RenderError` | `qp-render/src/lib.rs:35` → `error.rs:7` | saf taşıma |
| 16 | `struct TokenShared` | `qp-live/src/mock.rs:102` → `mock/issuer.rs:5` | saf taşıma |
| 17 | `fn schema_export` | `qp-cli/src/main.rs:1072` → `cmd/schema.rs:5` | yalnız görünürlük (`pub(crate)`) |
| 18 | `fn parse_update_date` | `qp-matriks/src/feed.rs:456` → `feed/date.rs:6` | saf taşıma (68 satır) |
| 19 | `struct Shell` | `qp-wasm/src/lib.rs:543` → `bind/shell.rs:23` | saf taşıma |
| 20 | `fn sig` | `qp-facts/tests/common/mod.rs:74` → `tests/support/mod.rs:74` | saf taşıma |

Sonuç: 15 saf taşıma, 5 yalnız görünürlük, 0 yol, **0 mantık değişikliği**.

### `rg1-moved 27656b1 HEAD` CHANGED kalemleri (128) — tek tek

`rg1-moved: 6701 items unchanged; 94 path-only; 235 with an added #[allow]; 128 CHANGED`. Hepsi izinli türden:

| Grup | Kalemler | Hüküm |
|---|---|---|
| Bağlanan sabitler (−17/+17) | `outcome.rs` `ATR_FAST` 14, `ATR_SLOW` 100, `C_DIR` 0.3, `C_NEUTRAL` 0.65, `INTRADAY_MAX_STEP_MS` 43_200_000; `theme.rs` `GAP_BARS` 10, `CONFIRM_BARS` 1; `watch.rs` `MAX_WATCHES` 32; `dispute.rs` `MAX_DIGESTS` 4, `MAX_PER_CONN` 256, `MAX_OPEN` 16; `qp-grammar` `FRESHNESS_HALF_BARS` 5.0, `CHART_LOOKBACK` 30, `GLYPH_MIN_LEDGER_N` 20; `qp-capsule` `FRESHNESS_HALF_BARS` 5.0; `LEDGER_MIN_N` 20; `STORY_ZOOM_MIN` 20 | Yalnız ilklendirici `qp_schema::params::system::*_DEFAULT` oldu; 17'sinin de `*_DEFAULT` değeri ve tipi eski değişmezle aynı (karşılaştırdım). İzinli (PLAN §13.7). |
| Yeni `params` kodu | `params.rs` (`ParamDef`, `ParamKind`, `ParamScope`, `ParamValue`, `ChangedBy`, `Resolved`, `RESOLUTION_ORDER`, `resolve`, `allows`), `params/system.rs` (16 anahtar + `_DEFAULT`, `REGISTRY`, `SYSTEM`, `SYSTEM_PLAN`, `SINCE`, `by_key`), `params/doc.rs`, `examples/params_doc.rs` | Yeni kod (K1). |
| Yeni testler (14) | `new-tests.txt`'teki 14 ad (+ yardımcı `is_key`) | Hepsi `27656b1`'de yok (`git grep` 0), HEAD'de var; izin listesi dürüst. |
| §13.3 | `bars.rs::synthetic_bars` (−), `synthetic_is_valid_and_stable` `qp-render` → `qp-source` (yalnız çağrı `synthetic_bars`→`synthetic_series`), beş örnek `main`, `golden.rs::sample_fixture_is_well_formed`, `qp-wasm` üç bağlama (`render_synthetic_rgba`, `…_vello_cpu`, `vello_gpu_…`) | Her biri tek satır: `qp_render::synthetic_bars(` → `qp_source::synthetic_series(`; sarmalayıcı aynen iletiyordu. İzinli. |
| `include_str!` yolu | `replica/viewer.rs::first_facts_lookup_equals_the_eager_map` | Yalnız `../../../fixtures/…` → `../../../../fixtures/…`. İzinli. |
| `#[allow]` + yol birlikte | `qp-host/src/livehost/tests/replica.rs` iki test | `#[allow(clippy::too_many_lines)]` eklendi ve `qp_wasm::shell::ShellCore` → `qp_shellcore::shell::ShellCore`; girinti dışında başka fark yok (`unpath` sonrası eşit). İzinli; araç ikisini birden tanımadığı için CHANGED görünüyor. |

Ek olarak `rg1-moved`'in görmediği yerlere baktım (kendi betiğimle, değişen bütün dosyalarda
önce/sonra çoklu küme): `impl` başlıkları **hiç kaybolmadı** (423 → 472; yalnız bölünmeden
gelen 49 ek `impl X` başlığı); öğe düzeyi makro çağrıları 11 → 11 (dört `proptest!` bloğunda
yalnız `common::` → `support::`); `mod` bildirimlerindeki `cfg` öznitelikleri: tek kayıp
`#[cfg(all(feature = "replica", not(target_arch = "wasm32")))] pub mod drive;` aynı öznitelikle
`pub use qp_shellcore::drive;` oldu. Eklenen `#[allow(clippy::…)]` satırlarında beş hedef lint
dışındakiler (`cast_*`, `too_many_arguments` …) aynı sayıda silinip taşınmış.

## 4. `arch-check`'i kandırma denemeleri

Hepsi depo dışındaki karalama kopyasında (`fixtures/`, `.env*`, `rounds/` kopyalanmadan),
kopyanın kendi `scripts/arch-check`'iyle; her denemeden sonra geri alındı.

| # | Hile | Sonuç | Ağırlık |
|---|---|---|---|
| 1 | 900 satırlık dosya, başında "@generated", `generated_files`'ta yok | **yakalandı** (`file.hard`) | — |
| 2 | `lib.rs`'te mantık `include!("hidden_logic.rs")` ile | **geçti** | orta (B2) |
| 3 | `lib.rs`'te mantık kullanıcı makrosuyla (`mk!(f)` → `fn $n`) | **geçti** | orta (B2) |
| 4 | `lib.rs`'te `thread_local!` durum | **geçti** | düşük (B2) |
| 5 | `src/` dışındaki 950 satırlık dosya (150 satırlık fn) `include!("../outside/big.rs")` ile | **geçti** (boyut, fonksiyon ve crate bütçelerinden kaçar) | orta (B4) |
| 6 | Yukarı bağımlılık `[build-dependencies]` | **yakalandı** (`layer.up`) | — |
| 7 | Yukarı bağımlılık `[target.'cfg(unix)'.dependencies]` | **yakalandı** (`layer.up`) | — |
| 8 | Yukarı bağımlılık `[dev-dependencies]` | geçti — kural gereği serbest (K-önerisi); not | — |
| 9 | 100 satırlık closure (`pub const LONG: fn() -> u64 = \|\| {…}`) | **geçti** (clippy `too_many_lines` de görmez; derlendi) | düşük (B6) |
| 10 | 104 satırlık yöntem bir trait `impl`'inde | **yakalandı** (`fn.lines`) | — |
| 11 | 100 satırlık fonksiyon makro gövdesinde (`fn $n`) | **geçti** | düşük (B6) |
| 12 | `#[path = "innocent.rs"] mod util;` | **geçti** (yasak ad yalnız dosya adına bakar) | düşük (B6) |
| 13 | Satır içi `pub mod util { … }` (kök dışı dosyada) | **geçti** | düşük (B6) |
| 14 | `src/util.rs` + `src/util/` | **yakalandı** (`name.forbidden`) | — |
| 15 | `cfg(test)`'siz `pub mod tests;` + 150 satırlık mantık (crate'te başka bir `#[cfg(test)] mod tests` varken; klasör README'leri eklenince) | **geçti** — dosya test sayılır, `fn.lines`/`crate.lines`/`file.soft`'tan muaf | orta (B3) |
| 16 | Taban çizgisindeki uzun fonksiyonu yeniden adlandırmak | **yakalandı** (yeni borç) | — |
| 17 | Taban çizgisindeki `file.soft` dosyasını başka yola taşımak | **yakalandı** | — |
| 18 | Tek öğe izni yerine modül/crate düzeyinde `#![allow(clippy::unwrap_used,…)]` (sayı aynı) | **geçti** (sayı 1, kapsam bütün crate) | yüksek (B1) |
| 19 | `#[allow(clippy::restriction)]`, `#[expect(clippy::unwrap_used)]`, `#[cfg_attr(all(), allow(clippy::unwrap_used))]` | **geçti** — `lint.allow` saymaz; clippy de susar (gerçek derlemeyle doğruladım) | yüksek (B1) |
| 20 | Crate `Cargo.toml`'dan `[lints] workspace = true`'yu silmek | **geçti** (bütün lint'ler o crate'te kapanır) | yüksek (B1) |
| 21 | `[[bin]] path = "bin/sneaky.rs"` (`src/` dışı, 950 satır) | **geçti** (taranmaz) | orta (B4) |
| 22 | `arch-config.json`'a yeni `exceptions` kalemi | geçti (tasarım gereği; her koşuda "EXCEPTION" basılır ama çıkış 0) | orta (B5) |
| 23 | `generated_files`'a ekleyip başlığa "üretilmiştir" yazmak | **geçti** (sessiz: hiçbir şey basılmaz) | orta (B5) |
| 24 | `arch-baseline.json`'da değeri elle büyütmek, fonksiyonu büyütmek | **geçti** (`--write-baseline` korunuyor, elle düzenleme korunmuyor) | orta (B5) |
| 25 | Hiçbir yerden `mod` ile anılmayan 900 satırlık dosya | **yakalandı** (`file.hard`) | — |
| 26 | `foo.rs` (kök olmayan) 641 satır | **yakalandı** (`file.soft` yeni borç) | — |

`scripts/arch-check --self-test`: PASS (20 bilinen ihlal + mandal + istisna).

## 5. Yapı denetimi (davranışın ötesi)

- **Klasör bölmeleri (örnek 5):** `qp-session/src/{authority,ops,capsule,data,read,replica,host}`,
  `qp-matriks/src/feed/{ingest,trades,output,date}`, `qp-grammar/src/look/{fonts,r4,ornek,oltan,
  zemre,registry,css}`, `qp-capsule/src/voice/marla/{words,head,why,since,record}`,
  `qp-schema/src/protocol/` (14 mesaj ailesi). Konuya göre ve tutarlı; türe göre klasör yok.
- **README'ler (tohum 7 ile 6 örnek):** `qp-matriks/src/feed`, `qp-grammar/src/look`,
  `qp-session/src/authority`, `qp-capsule/src/voice/marla`, `qp-compute/src/studies`,
  `qp-session/src/replica/viewer`. Hepsinde andığı dosyalar var; yalnız `look/README` `tests.rs`'i,
  `authority/README` `agent`, `error`, `suggest`, `values`, `session/` dosya adlarını anmıyor
  (kavramlarla anlatıyor). Crate README'leri `qp-shellcore`, `qp-host`: bağımlılık listeleri
  `cargo metadata` kenarlarıyla uyumlu.
- **Yeni crate belgesi:** `docs/ARCHITECTURE.md:134–135` (`qp-shellcore`, `qp-host`),
  `docs/CONCEPTS.md:191–192` ("Kabuk çekirdeği", "Barındırma"), `docs/decisions/rounds/RG1.md`
  Ö104, Ö105. ✅
- **Kabuklarda kalan mantık:** `qp-wasm` yalnız `bind/` bağlamaları. `qp-cli/src/cmd/replay.rs`
  `replay_cmd` (≈200 satır, `#[allow(too_many_lines)]`): denetim zamanlaması ve sürücü
  döngüsü; `cmd/serve.rs` stdio sunucu döngüleri; `cmd/scene.rs` `scene_session` (döküm
  uygulama). Alan kararı değil ama barındırma mantığı; `qp-host`'a gitmeliydi (B12).
- **Kural dosyaları:** `docs/ENGINEERING.md` round boyunca değişmedi; `NORTH_STAR.md` yalnız
  `b341e2d9` "ana akış: K84" (K84 satırı `docs/decisions/kararlar.md:92`'de) — izinli.
  `.env.local` hiçbir commit'te yok. Diff'te `refs/` ya da OpenChart başvurusu yok (kopya
  izine rastlamadım; tam kanıt değil).

## 6. Bulgular

| # | Ağırlık | Bulgu | Öneri |
|---|---|---|---|
| B1 | **yüksek** | Lint mandalı delik: `lint.allow` yalnız `#[allow(clippy::<beş lint>)]` / `#![allow(…)]` metnini crate başına sayar. (a) Bir öğe izni silinip modül/crate düzeyinde tek `#![allow(clippy::unwrap_used)]` konunca sayı aynı kalır, kapsam bütün crate olur; (b) `allow(clippy::restriction)` (grup), `#[expect(…)]`, `#[cfg_attr(…, allow(…))]` hiç sayılmaz ve clippy'yi gerçekten susturur (karalama derlemesiyle doğrulandı); (c) bir crate `[lints] workspace = true`'yu silerse beş lint o crate'te kapanır, hiçbir kural fark etmez. | `lint.allow`'u öğe başına (yol::öğe) anahtarlayın; `#![allow]` (iç), `expect`, `cfg_attr`, grup (`restriction`, `pedantic`, `nursery`, `all`) biçimlerini de sayın ya da kesin kural yapın; her crate'te `[lints] workspace = true`'yu `arch-check`'te kesin kural olarak denetleyin; öz-sınamaya her biri için örnek. |
| B2 | orta | `root.logic` yalnız `fn/impl/struct/enum/union/trait/static/const/macro_rules!` sözcüklerine ve satır içi `mod {}`'a bakar; `lib.rs`/`mod.rs`'te `include!("x.rs")`, kullanıcı makrosu çağrısı (`mk!(f)`), `thread_local!` geçer. Bugün gerçek kodda böyle bir kullanım yok (taradım). | `lib.rs`/`mod.rs`'te `mod`, `use`, `pub use`, öznitelik ve belge dışındaki her üst düzey öğeyi (makro çağrısı dahil) bulgu sayın. |
| B3 | orta | `tests.rs` / `tests/` muafiyeti crate düzeyinde ad eşleşmesiyle verilir: crate'in herhangi bir yerinde `#[cfg(test)] mod tests` varsa, başka bir yerde `cfg`'siz `pub mod tests;` ile bağlanan `tests.rs` de test sayılır ve `fn.lines`, `crate.lines`, `file.soft`'tan muaf olur (150 satırlık mantık geçti). Öz-sınamadaki örnek yalnız crate'te hiç `cfg(test)` yokken çalışıyor. | Bildirimi dosyanın gerçek ebeveyn modülünde arayın (`a/tests.rs` için `a.rs` ya da `a/mod.rs` içinde `#[cfg(test)] mod tests`); öz-sınamaya bu ikili durumu ekleyin. |
| B4 | orta | Tarama yalnız `crates/*/{src,tests,examples,benches}/**` ve kök `build.rs`'i görür. `include!("../outside/x.rs")` ile ya da `[[bin]]/[lib] path` ile `src/` dışına konan kod bütün bütçelerden kaçar (`path.outside` yalnız `#[path]`'e bakar). | `include!` hedeflerini `#[path]` gibi denetleyin (`src/` dışı kesin hata, `OUT_DIR` hariç); `cargo metadata` hedeflerinin (`targets[].src_path`) hepsini tarayın. |
| B5 | orta | `arch-config.json` (istisnalar, `generated_files`, `layer_overrides`, bütçeler) ve `arch-baseline.json` elle düzenlenince hiçbir şey hata vermez; `generated_files` eklemek sessizdir (her koşuda basılmaz). Mandal yalnız `--write-baseline` yolunda korunuyor. Ayrıca küçülen kalem "stale" diye yalnız yazılır, hata değildir — fonksiyon kısalıp sonra eski boyuna geri büyüyebilir. | `verify.sh`'a: yapılandırma ve taban çizgisinin `main`'e göre farkını basan ve her yeni istisna/üretilmiş dosya için `docs/decisions` içinde adlı bir satır (İn/K/Ö) arayan adım; `generated_files`'ı da her koşuda basın; "stale" kalemleri round kapanışında hata yapın (ya da `--write-baseline` her birleştirmede zorunlu). |
| B6 | düşük | Fonksiyon bütçesi `fn <ad>` metnine bağlı: `const`/`static` closure'lar ve makro gövdesindeki `fn $n` sayılmaz (clippy `too_many_lines` closure'ları da saymaz). Yasak ad yalnız dosya/klasör adına bakar: `#[path = "x.rs"] mod util;` ve satır içi `mod util {}` geçer. | Uzun closure ve makro gövdelerini satır sayısıyla ölçün; yasak adı `mod <ad>` bildirimlerinde de arayın. |
| B7 | orta | İstisna istekleri onaysız: ölçüt 4 (İ1, İ2), 5 (İ6), 7 (İ3–İ5) ve 3 (İ7) ancak Arkin onaylarsa karşılanmış sayılır. İ1 olmadan bir yukarı (`qp-session → qp-source`) ve sekiz yan bağımlılık var; `qp-session` 32 bin satırda kaldı. | Raporda bu ölçütler "istisnaya bağlı" diye yazılsın; A63 karara bağlanmadan RD2/RL1 yeni kodu bu crate'lere eklemesin (ENGINEERING §1.2 "önce yer aç"). |
| B8 | orta | `rg1-moved`'in kör noktaları: `use` satırları (bir `use a::f` → `use b::f` farklı bir `f`'yi bağlayabilir), `impl` başlıkları, öğe düzeyi makro çağrıları, `mod`/`impl` üstündeki `cfg` öznitelikleri öğe değildir; ayrıca `#[allow]` ekleme ile yol değişikliği aynı öğede olunca CHANGED görünür. Bu turda ayrı betikle baktım: `impl` başlığı kaybı 0, makro çağrıları eşit, `cfg` değişikliği yok — sorun bulunmadı; ama araç bunu garanti etmiyor. | `impl` başlıklarını ve öğe düzeyi makroları da parmak izine katın; `use` hedeflerini (son bölüm + tam yol) çoklu küme olarak karşılaştırın; ALLOW+PATH birleşimini tanıyın. |
| B9 | düşük | Tutanak K52 bölümü kesik (`baseline/perf.txt` 56 satır; JSON'un yalnız son ≈40 satırı): `frame_ms`, `open_ms`, `counts` tutanakta yok, K52 "sınırlar içinde" iddiası karşılaştırılamıyor. Görünen aralıklar tek koşuda yukarıda: `paint_pass` p95 3,8 → 4,4, p99 10,5 → 13,4; `pass_wait` p95 9,1 → 9,9; `scene_build` p99 6,2 → 7,9. | K17'deki gibi aynı saatte `27656b1` ile A/B K52 koşusu; tutanak dosyasına bütün JSON. |
| B10 | düşük | Kabuk WASM paketi gzip +%2,54 (2.506.216 → 2.569.974), ±%2 dışında; `docs/decisions/rounds/RG1.md` A63 "+%2,36" diyor. | İ7 isteğini kayıtlı ölçümle (+%2,54) yazın; büyümenin nedeni (crate sınırı, `qp-shellcore` ayrı derleme birimi) ölçülsün. |
| B11 | düşük | Tutanak kapsamı: MCP `tools/call` dört yüzeyde yalnız 12 çağrı (op_* yazma araçları yok); `qp sim`, `qp live`, `qp demo` HTTP sunucusu (adım 18 dışında) tutanakta yok, oysa en büyük taşıma (`qp-cli` → `qp-host`) tam bunlar. `tests.names` yalnız ad karşılaştırır (gövde ya da geçme değil; geçmeyi `verify.sh` sınar). | Sonraki düzenleme round'u için tutanağa `qp sim` belirlenimci çıktısı ve her yüzeyde birer yazma aracı çağrısı eklensin. |
| B12 | düşük | `qp-cli`'de barındırma mantığı kaldı: `cmd/replay.rs::replay_cmd` (≈200 satır, denetim zamanlaması), `cmd/serve.rs` stdio sunucu döngüleri, `cmd/scene.rs::scene_session`. Bütçe içinde (1.420) ama ENGINEERING §2 "yalnız bağlama". | `qp-host`'a taşınsın (sonraki düzenleme). |
| B13 | düşük | "Kayıt defterinden okunur" bağlamada `resolve` yolu kullanılmıyor: bağlanan sabit adlı `*_DEFAULT`'u okur; çalışma zamanında değer değiştirmenin yolu yok (round kapsamına uygun, ama "okuma yolu" iskelette yalnız tanımda). `resolve` `min`/`max` aralığını denetlemiyor. | Profil round'unda `resolve` aralık denetimi ve tipli hata; bağlanan sabitlerin çağrı yerine `resolve` ile geçişi. |
| B14 | düşük | ENGINEERING §5 `unimplemented!`'i de yasaklar ama `clippy::unimplemented` açık değil; `unreachable!` (kütüphane kodunda 15 yer, ör. `qp-compute/src/registry.rs:137`, `qp-session/src/capsule/watch.rs:183`) `clippy::panic` kapsamında değil, mandalda görünmüyor. | `clippy::unimplemented` ve `clippy::unreachable`'ı açın, mevcutları taban çizgisine yazın; ya da Kural önerisi olarak ENGINEERING'e. |
| B15 | düşük | Plan §2.6 "geçici yeniden dışa aktarmalar round sonunda listelenir (borç)": `docs/DEBT.md`'de yok (`qp-wasm` → `qp_shellcore::{shell,gallery,viewer,drive}`, `qp-session`'ın `pub(crate) use` takma yolları, `#[doc(hidden)] pub use`). | REPORT ya da DEBT'e liste. |
| B16 | düşük | Kapanış eksik (denetim anında): REPORT/STATUS/ROADMAP yok, `target/` 76 GB (bütçe 6 GB), CI koşusu görülmedi. | Orkestratör kapanışı (K68, `scripts/clean.sh --deep`, `preflight.sh` + `qgh`). |

## 7. Doğrulayamadıklarım

- CI (push/PR yapmadım; `preflight.sh`/`qgh` denetçinin işi değil).
- REPORT.md'deki "Fark edilenler", "Kural önerileri", istisna istekleri metni ve recovered karar
  listesinin rapordaki hali (dosya yok; yalnız `experiments/recovered.md`).
- Kapanış temizliğinin sonucu (ölçüt 15).
- K52 telefon bütçesinin tam karşılaştırması (tutanak kesik, B9); K17'yi yeniden ölçmedim
  (orkestratörün A/B'sine dayandım).
- `../refs` kodunun kopyalanmadığını yalnız metin aramasıyla denetledim; benzerlik taraması yok.
- `--features replica` dışındaki özellik kapılı testler `tests.names`'e girmiyor; o yollar yalnız
  `verify.sh` adım 8/15 ile sınanıyor.
