# RF1/S2+S4 — bağımsız denetim (`rf1/s2-ops` @ `aa4046a9`)

Denetçi: ayrı alt ajan; kendi worktree'si (`.claude/worktrees/audit-s2`, ayrık HEAD),
`CARGO_TARGET_DIR=target/wt/audit-s2`. Denetlenen dal değiştirilmedi, birleştirilmedi.
Karşılaştırma tabanı `main` = `2a303637` (dal bununla birleştirilmiş).

## Hüküm: **GEÇTİ** (koşullu: B1 birleştirmeden önce düzeltilmeli; yalnız belge)

Davranış, yapı ve boyut ölçütlerinin hepsi geçiyor. Tek eksik `rewritten.md`'nin S4 bölümünün
bcfbcbc2/aa4046a9 sonrasına güncellenmemiş olması (B1): `moved.py`'nin bir CHANGED'ı
(`struct Arm`) listede yok ve S4 metni artık var olmayan bir kalıbı anlatıyor.

## Sonuçlar

| # | Adım | Sonuç |
|---|------|-------|
| 1 | `cargo fmt --all -- --check` | 0 |
| 1 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| 1 | `cargo test --workspace` | 196 takım, 1222 geçti, 0 kaldı, 9 yok sayıldı |
| 1 | S1 charac (`qp-session --test charac`) | 25/25 |
| 1 | `git diff main...rf1/s2-ops -- '*/expected/*'` | boş |
| 1 | `git diff --stat 2a303637 rf1/s2-ops -- fixtures/{golden,compute,protocol,views,schema}` | boş |
| 2 | `scripts/refactor/compare.sh` | 270/270 bayt eşit |
| 3 | `moved.py main HEAD` | 1294 değişmedi, 96 yalnız yol, 43 CHANGED, 15 FLAG; CHANGED'ların biri (`struct Arm`) `rewritten.md`'de yok → B1 |
| 3 | Elle fark: `DocProvider` / `HeadlineSource` | uygun (aşağıda) |
| 3 | Elle fark: S4 kolları | 21 kolun tamamı (istenen 8 yerine) normalize metinle karşılaştırıldı; uygun |
| 3 | `put_series_source` | uygun |
| 3 | `Computed::is_empty`, `values/computed.rs` | uygun |
| 4 | Duyarlılık: 3 kasıtlı değişiklik | üçü de charac'ı kırmızı yaptı; geri alındı, 25/25 |
| 5 | `scripts/arch-check` | 0 hata; 2 adlı istisna (katman K85, İ3) |
| 5 | `--table` | `qp-session` 20437, `qp-ops` 9193 (pay 2807 ≥ 2500), `qp-plane` 2733 |
| 5 | `--config-diff main` | 33 değişiklik, 0 dayanaksız gevşetme |
| 5 | İ3 tavanı | 31999 → 20437 (= `qp-session` değeri) |
| 5 | İ6 | yapılandırmadan silinmiş (`removed exception İ6 … op.rs`) |
| 5 | `lint.allow` | `qp-session` 71 → 48, `qp-ops` 21, `qp-plane` 1; toplam 71 → 70 |
| 5 | `pub` genişletmeleri, 15 örnek | 15'i de başka crate'ten kullanılıyor |
| 5 | src içinde modül/crate düzeyi `#![allow]` | korunan lintlerde yok; `result_large_err` var → B2 |
| 5 | `git grep -c 'unreachable!' -- crates/qp-ops/src` | 4 (`summary/hints.rs` 3, `watch.rs` 1; ikisi de main'de vardı) |
| 5 | `arch-check --self-test`, `moved.py --self-test` | PASS, PASS |
| 6 | WASM `--profile wasm` | raw 2354876 (+%0,39), gzip -9 773645 (+%0,49); ±%2 içinde |
| 7 | Yeni crate'ler | README var (crate + 7 alt klasör), `[lints] workspace = true`, `lib.rs` yalnız bildirim + `pub use`, klasör başına en çok 12 dosya; katman sırası `qp-plane < qp-ops < qp-session` |

### 3 — Elle farklar

- **`DocProvider`** (`git show main:…/data/provider.rs` ↔ `qp-ops/src/provider.rs`): yol
  düzeltmeleri (`crate::plane` → `qp_plane::plane`, `crate::capsule::story_*` →
  `qp_plane::sentences::…`), `pub(crate)` → `pub` (`new`, `with_headlines`, `with_parts`,
  `with_voice`, `with_clock`, `warm`), `parts` alanı ve `with_parts` parametresi
  `&dyn HeadlineSource`. `headline_parts`/`voice` gövdeleri trait çağrısına indirgenmiş.
  `impl qp_ops::HeadlineSource for Session` (`capsule/headsource.rs`) eski iki gövdeyi
  aynen taşıyor: `find(|c| c.finding == finding)?`, `reader_parts`/`reader_voice` aynı
  argüman sırası, yeni `StudyMemo::default()`; tek fark `s` → `self` ve `self.clock` artık
  `clock` parametresiyle geliyor (çağıran `self.clock` veriyor). `Session`'da aynı adlı
  inherent yöntem yok; çağrı belirsizliği yok.
- **S4 kolları:** `main`'in `expand_op`'undaki 21 `=> { … }` kolu ile yeni fonksiyon
  gövdeleri boşluk, yorum ve sondaki virgüller atılarak karşılaştırıldı
  (betik denetçinin scratchpad'inde; sonuç burada). 18 kol birebir aynı. Farklar:
  `PutSeries` (beklenen: `put_series_source` çıkarımı); `SetViewRange` ve `AddFact` yalnız
  `crate::plane::` → `qp_plane::plane::` ve rustfmt'in tek ifadeli kapanış bloğunu açması
  (`ok_or_else(|| { invalid(…) })` → `ok_or_else(|| invalid(…))`), anlam aynı. Tek satırlık
  kollar (`AddWatch/RemoveWatch`, `Undo`, gramer, `AcceptProposal`) dağıtıcıda aynen.
  Dağıtıcı her kolu doğru fonksiyona ve alanlara gönderiyor (`ApproveFinding | RejectFinding`
  → `decide_finding(doc, op, …)`, `op` taşınıyor); `unreachable!` yok.
- **`put_series_source`:** eski `let src = ptr(&sp, "source")` + `match &series.source`
  bloğu aynen; `&sp` → `sp` 6 yerde (`sp: &str`), `s.source = …` `&mut Series` üstünden.
  `src`'nin hesaplandığı yer `let mut s = series.clone()`'dan sonra iken şimdi çağrının
  içinde; ikisinin de yan etkisi yok. Ardından `rebind_entitled`, `note_created` aynı sırada.
- **`Computed`:** yapı, `new`, `len`, `main_values` `authority/values.rs`'ten
  `qp-plane/src/values/computed.rs`'e aynen; görünürlük `pub(crate)` → `pub` (tip, `len`,
  `main_values`), `main_ix` ve `new` özel → `pub(super)`. `is_empty` yeni (`self.t.is_empty()`),
  kullanılmıyor, `rewritten.md`'de sayılmış.
- **`viewer_less.rs::deliver`:** yalnız `qp_session::headparts` → `qp_plane::headparts` ve
  rustfmt'in yeniden sarması; gövde aynı.

### 4 — Duyarlılık (S4 kol fonksiyonlarında, her biri tek başına, sonra geri alındı)

| Değişiklik | Kırmızı olan |
|---|---|
| `set_params`: birleşik `qp_compute::create` denetimi anahtar başına döngünün önüne (iki denetimin yeri değişti) | `ops_doc::charac_session_ops_series_and_views` (`set_params.err.unknown_param`, `.err.bad_value`: yol ve ileti değişti) |
| `add_comment`: hata metni `no series of instrument` → `no series for instrument` | `ops_flow::charac_session_ops_people` (`add_comment.err.instrument`) |
| `add_study`: yeni panelin ölçek kimliği `y{panel}` → `z{panel}` (üretilen değişiklik) | `ops_doc::charac_session_ops_series_and_views` (`add_study.ok.show_new_panel` özeti) |

Geri alındıktan sonra `git status` temiz, charac 25/25.

## Bulgular

- **B1 (belge; birleştirmeden önce).** `rounds/RF1/experiments/rewritten.md` S4 bölümü
  `43163340`'tan beri güncellenmemiş; bcfbcbc2 ve aa4046a9 sonrasındaki kodla çelişiyor:
  - "Her fonksiyon `op`'u `let Op::X { … } = op else { unreachable!(…) };` ile açar" — artık
    kol fonksiyonları alanları parametre olarak alıyor; dağıtıcı alanları açıyor.
  - "`expand_op` dağıtıcı `match`'i olarak kaldı (56 satırlık dosya)" — dosya 121 satır,
    dağıtıcı 101 satır ve taban çizgisinde (DECISION.md "Uygulama notları").
  - "`expand_op`'un `too_many_lines` … izinleri kalktı" ve "`expand_op` `fn.lines` kalemi
    kalktı" — `too_many_lines` izni geri geldi, `fn.lines` kalemi 101 olarak duruyor
    (`cognitive_complexity` izni gerçekten kalktı).
  - `moved.py`'nin CHANGED'ı `struct Arm` (`expand/op.rs`, yeni öğe) ve kol başına
    `#[allow(clippy::ptr_arg)]` listede yok.
  Gerekli: S4 bölümünü güncel kalıba göre yeniden yaz, `Arm`'ı yeni öğe olarak ekle.
- **B2 (bilgi).** `qp-ops/src/lib.rs:9` ve `qp-plane/src/lib.rs:8` crate düzeyinde
  `#![allow(clippy::result_large_err)]` taşıyor (gerekçe yorumu ile). Bu, `main`'de
  `qp-session/src/lib.rs:18`'de zaten olan iznin yeni crate'lere kopyası; arch-check'in
  `lint.inner` kuralının korunan lintleri (unwrap/expect/panic/too_many_lines/
  cognitive_complexity/restriction/nursery) arasında değil, `moved.py` FLAG olarak gösterdi.
  `rewritten.md` ya da `paths.md`'de anılmıyor; bir satırla anılması önerilir.
- **B3 (bilgi).** Kol fonksiyonlarının `#[allow(clippy::ptr_arg)]` izinleri (`&String`,
  `&Vec<_>` alan bağları) öğe düzeyinde; arch-check'in saydığı lintlerden değil.
- `paths.md` "## S2" tablosu ve `pub` listesi kodla tutarlı; örneklenen 15 genişletmenin
  (`slot_of`, `touched_now`, `begin_tick`, `eval_on`, `rd_same`, `ReaderView`, `prune`,
  `mentions_hidden`, `story_validity`, `finding_about`, `valid_id`, `undone_set`,
  `condition_entitled`, `same_but_anchor`, `conflicts`) hepsi başka crate'ten çağrılıyor.

## Temizlik

`.claude/worktrees/audit-s2` ve `target/wt/audit-s2` kaldırıldı; denetçi scratchpad'i
boşaltıldı. Hiçbir süreç adla kapatılmadı; `.env.local` okunmadı.
