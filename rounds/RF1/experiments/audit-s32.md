# RF1/S3.2 bağımsız denetim: `rf1/s32-read` @ 7871e187 (`main` bd0fdd76 üstünde)

**Hüküm: GEÇTİ (koşullu).** Davranış `main` ile aynı: charac 25/25, 11/11 ve ext 4/4, kitaplar
aynı, `compare.sh` 270/270. WASM izleyici yansısı (FOLLOWER) her adımda yerel yansıyla aynı.
**FOLLOWER güvenli:** ürün kodunda bir izleyici otoritenin sorgu ya da girdi işleyicisine ulaştığı
yol yok (§2). Okuma kodu `impl Rd` altında, gövdeleri aynı kalarak taşındı. Elle karşılaştırılan
yöntemlerde yalnız belgelenmiş mekanik değişiklikler var (§3).

Birleştirmeden önce kapanması gerekenler:
- **B1 (orta):** FOLLOWER mimari bir sapma. `PLAN.md` §3.3 satır 6'da yansı kendi otoritesini
  `qp-read` + `qp-watch` kaydıyla kurar. Bu sapma için karar satırı yok. `docs/ARCHITECTURE.md`
  "Yeni alt sistem nasıl eklenir" 6–7 ve `docs/CONCEPTS.md` "Bileşim" artık yanlış.
- **B4 (orta, lint):** `qp-read` crate düzeyinde yeni bir `#![allow(clippy::needless_borrow)]`
  taşıyor. Bu izin taşınan koddan gelmedi ve `rewritten.md`'de anılmıyor. Kapsadığı yer sayısı 47.

Denetçi kendi worktree'sinde çalıştı (`.claude/worktrees/audit-s32`, `--detach`). Derleme klasörü
`target/wt/audit-s32` idi (sürüm ölçümleri `…/rel`, `main` karşılaştırması `…/main`). Dal
değiştirilmedi, birleştirilmedi. B4 için izin satırı yalnız denetim worktree'sinde geçici olarak
silindi ve geri alındı; `git status` temiz.

## Sonuçlar

| # | Adım | Sonuç |
|---|---|---|
| 1 | `cargo fmt --all -- --check` | temiz |
| 1 | `cargo clippy --workspace --all-targets -- -D warnings` | temiz |
| 1 | `cargo test --workspace` | 209 ikili, 1242 geçti, 0 kaldı, 9 yok sayıldı |
| 1 | S1 charac (`qp-session --test charac`), G1 (`qp-designs --test charac`), ext (`qp-session --test extension`) | 25/25, 11/11, 4/4 |
| 1 | Kitaplar (`*/tests/charac/expected`) | `git diff bd0fdd76 HEAD` boş; koşulardan sonra `git status` temiz |
| 1 | Yeni sınamalar (`new-tests.txt` S3.2) | 2'si de koştu ve geçti |
| 1 | `scripts/refactor/compare.sh` | `270 items byte-identical to the baseline` |
| 2 | FOLLOWER güvenliği | sorgu ya da girdiye ulaşan izleyici yolu **yok**; aşağıda |
| 2 | Yansı şeridi (`qp replica-tape` lag 0/25, `--viewer` 0/25) + WASM düğüm tekrarı | 0 ihlal; wasm == yerel 186/186 ve 211/211; lag 0'da wasm == otorite 186/186 |
| 3 | Yeniden yazılan öğeler | değer aynı; aşağıda |
| 4 | `#![allow(clippy::needless_borrow)]` | `lint.inner` korunan lintlerden değil; yine de yeni ve atıfsız → B4 |
| 5 | `scripts/arch-check` | 0 hata; 2 adlı istisna; İ3 `qp-authority` **16.699 = tavan 16.699** |
| 5 | `scripts/arch-check --config-diff main` | 24 değişiklik, **0 atıfsız gevşeme**. İ3 20.129→16.699 (sıkı); `qp-read` `oturum` katmanına; `file.soft` 36→33; `fn.lines` 174→173 (taşınanlar adla eşleşiyor); `lint.allow` `qp-authority` 47→36, `qp-read` 11 (yeni crate'e yayılma, toplam büyümedi) |
| 5 | `cargo tree` | `qp-authority`'nin ağacında `qp-read` yok (normal, build, dev). `qp-read`'in ağacında `qp-authority` yok. `qp-read` → `qp-capsule, qp-compute, qp-designs, qp-glyph, qp-grammar, qp-look, qp-ops, qp-plane, qp-schema, qp-source, qp-subsystem, qp-voice` |
| 5 | Katman sırası | `qp-plane < qp-ops < qp-subsystem < qp-watch < qp-read < qp-authority < qp-session` |
| 6 | WASM | aşağıda; kabuk paketi **+%1,917**, sınıra 7.257 bayt kaldı |
| 7 | `ReadView` sapması | kabul edilebilir; "alt sistem otoriteyi tanımaz" ilkesi crate düzeyinde korunuyor (B6) |

## 2. FOLLOWER güvenliği

**Yapısal argüman.** `qp-authority` `qp-read`'e bağımlı değil (`cargo tree`: 0). Bu yüzden
otoritenin herhangi bir fonksiyonu okuma koduna ancak kayıt üstünden ulaşabilir. Kayda giden
çalışma zamanı kapıları üç tane:

| Kapı | Görünürlük | Kim çağırıyor |
|---|---|---|
| `dispatch_query` (`phases.rs:446`) | `pub(crate)` | `dispatch.rs:66` (`handle_raw` → `query` iletisi); `replica/viewer/updates.rs:227` `viewer_compute` (← `viewer_update`, yalnız `viewers`'ta kayıtlı bir bağlantı için; `viewers` yalnız `viewer` aboneliğiyle, yani `handle_raw` ile dolar) |
| `dispatch_input` (`phases.rs:456`) | `pub(super)` | `dispatch.rs:125` (`handle_raw` → `input` iletisi) |
| `with_subsystems` / `with_read_ctx` (`phases.rs:296/309`) | `pub` (gizli) / `pub(crate)` | yalnız `qp_session::Session::scene_for` (`compose.rs:151`; `STANDARD` otorite) |

Bu yüzden soru şuna indi: **izleyici bir otoritede `handle_raw` (ya da `handle`) çağrılıyor mu?**
İzleyici otoriteye dışarıdan erişimin tek yolu `Replica::session_mut()` (`replica.rs:359`;
`OpenSide::replica_mut()` üstünden de). Bütün çalışma alanında taradım:

| Yer | İzleyici otoritede ne çağrılıyor | Kayda gider mi |
|---|---|---|
| `qp-authority` `replica.rs`, `replica/follow.rs`, `judge.rs` | belge uygulama, `data_changed`, `scene`, `binding_state`, `candidates` | hayır (`handle_raw` yok) |
| `qp-authority` `replica/viewer/open.rs` (`face_scene`, `open_scene`, `open_scene_with`) | `s.provider()`, `s.row_stamp()`, `s.document()`, `item_headline_parts` | hayır (`provider` otoritede kaldı, `read.rs`) |
| `qp-authority` `replica/tape.rs:242/289/548` | `self.a.handle_raw` / `a.handle_raw` | `a` **başvuru otoritesi**, izleyici değil |
| `qp-shellcore` `shell/scene.rs:73/135/157`, `drive.rs:186/195`, `drive/viewer_less.rs:272` | `face_scene`, `open_scene_with`, `story_series`, `story_rows` | hayır |
| `qp-shellcore` `drive.rs:70/126/303/400`, `viewer_less.rs`, `gallery/build_impl.rs:20` | `host.handle_raw` | `host` bir `ReplayHost` (`STANDARD`); `drive` ve `gallery::build_impl` yalnız yerel (`cfg(not(wasm32))`) |
| `qp-shellcore` `viewer.rs:173/396` | `set_headline_calendar`, `replica()` okuma | hayır |
| `qp-wasm` `bind/replica.rs` | `options`, `set_options`, `watch`, `on_line`, `resync`, `take_outbox`, `display`, `display_digest`, `document_digest`, `seq` | hayır |
| `qp-wasm` `examples/startup.rs:101` | `host.handle_raw` | `ReplayHost` |
| `shells/web` (`main.js`, `tests/paint-paths.mjs`) | `screen`, `tap_at`/`hit` | sorgular sunucuya `POST` ile gider (`main.js:157`, `paint-paths.mjs:120`): yerel otorite (`qp demo`, `STANDARD`) |
| `verify.sh` adım 15 | `qp replica-tape` (yerel) + `crates/qp-wasm/tests/replica_tape.mjs` | şerit yansıya yalnız sunucu satırı ve veri olayı verir |
| `verify.sh` adım 18 | `qp demo` + kabuk | sorgular sunucuda |

Yansının giden kutusu (`take_outbox`) başvuru otoritesine gider; izleyiciye hiçbir yol sorgu ya
da girdi iletisi teslim etmiyor.

**Gözlem.** Adım 15'in yerel ve WASM kısmını kendim koştum. WASM paketi `qp_session::Replica`'yı,
yani FOLLOWER'ı kullanıyor. Yerel şerit ise STANDARD kullanıyor (bkz. B2). WASM tekrarı, lag 0
ve 25'te 186/186 ve 211/211 adımda yerel yansıyla doc, view ve display bakımından aynı.
`qp-shellcore` sürüş paritesi sınamaları (`cargo test`) da FOLLOWER yansıyla geçti. Adım 18
(tarayıcı) koşulmadı. Bu adımın sorguları sunucuya gidiyor, kodla kanıtlandı.

**Sonuç: FOLLOWER bugün hiçbir ürün yolunda davranışı değiştirmiyor.** Gizli kalan risk B3'te.

## 3. Yeniden yazılan öğeler

- **`bindings` (`qp-read/src/bindq.rs:74–87`).** `FeedStore::position` (`&mut`, önek belleği)
  yerine `feed()`, `bars().len()`, `bars().last().t`, `seq()` ve `data_digest(bars())`
  kullanılıyor. `position` `digest_u64()` = `prefix_digest_u64(len)` döndürüyordu. Önek belleği
  `vec![DATA_DIGEST_OFFSET]` ile başlıyor ve `data_digest_extend` ile uzuyor. Bar değişince
  belleğin geçersiz kılındığı yerler: `replace` sonrası `truncate(1)` (`feed.rs:276`), düzeltmede
  `truncate(index+1)` (`:389`) ve sona `push` (bellek tembel uzar). Bunlar dışında `bars`'ı
  değiştiren kod yok. Yani her durumda `prefix[len] == data_digest_u64(bars)` ve **değer aynı**.
  Fark yalnız maliyette: sorgu başına bütün barlar yeniden katlanıyor ve bellek ısınmıyor. Sorgu
  soğuk yolda, ihmal edilebilir.
- **`capsule_query_with` (`capsule/detail.rs`).** `main`'deki gövdeyle satır satır aynı. Tek
  fark, `self.conns[ci].look.{voice,clock}` yerine `(voice, clock)` parametresi. Okuma tarafı
  (`Rd::capsule_query`, `rd.rs:230`) `self.looks[ci]`'yi veriyor, bu da otoritenin `looks`'u
  (çağrı boyunca ödünç). Yorum doğrulayıcısı (`comments.rs:85` `seen`) `(None, false)` veriyor.
  Ses yalnız `item.voice`'u etkiliyor (`reader_voice`). `seen` ise yalnız `evidence`, `facts`,
  `sentence.text`, `ledger` ve `direction` okuyor. **Sonuç aynı.**
- **`ledger_now` / `ledger_rows_of` (`capsule/run.rs:512/567`).** `ledger_rows_of` gövdesi
  `main`'deki `ledger_rows` gövdesinin aynısı. Motor dalı `main`'de `capsules: Vec::new()` ile
  `Document` kuruyordu; şimdi `ledger_rows_of(&[], &engine.ledger())`. Boş liste aynı geri
  çekilmiş kümesini (boş) veriyor. **Aynı.**
- **`ReadView` uygulaması (`phases.rs:129–254`).** 27 yöntem; görevde 26 deniyor, `paths.md`
  27 listeliyor. Her biri otoritenin aynı adlı fonksiyonunu çağırıyor (`Session::X(self, …)`).
  Üç istisna var ve üçü de doğru:
  - `evidence_for` → `crate::claims::for_principal` (`main`'de okuma kodu doğrudan bunu çağırıyordu).
  - `accept_preview` → `accept_expand(pid, None, decider, None, seq+1)` ve `(doc, changes)`
    (`main` `preview.rs:63` ile aynı argümanlar).
  - `item_series` → `reader_item_series` → `item_series`.

  `capsule_item` → `capsule_item_with` (`main`'de okuma kodu bunu çağırıyordu).
- **`ANY` sırası (`qp-subsystem/src/registry.rs:37`).** Önce kayıtlı etiket (`kind_of` →
  `find`), sonra `ANY`. Tablo yalnız `ANY` tutuyorsa serileştirme yapılmadan doğrudan veriliyor.
  `main`: kayıtlı etiket, yoksa yerleşik yol. STANDARD'da (`watch` + `read`) yalnız `read`
  sorgu ve girdi kaydediyor. Kayıtlı türler önce, kalanlar okumaya gidiyor: **bugünkü
  cevapların aynısı.** Tek fark, kayıtlı (Tally gibi) bir işleyicinin artık `SessionCtx::read`
  ile çağrılması (zararsız). Kayıtsız otorite `unsupported` döner; sınaması var.
- **Elle karşılaştırma.** `main`'in `read/*.rs` ve `capsule/queries.rs` dosyalarını `qp-read`
  karşılıklarıyla `diff -w` ile karşılaştırdım. 10'dan fazla yöntem tamamen okundu: `screen_info`,
  `move_chart` (`input`), `interact`, `set_style`, `frame`, `reader_scene`, `capsule_scene`,
  `transition`, `preview`, `capsules_query`, `bindings`, `check_fact`, `query` (`Describe`,
  `Context`, `Evidence` kolları). Farkların hepsi `rewritten.md`'deki kalıplardan:
  - `impl Session` → `impl Rd<'_>`
  - `conns[ci].look` → `looks[ci]`
  - `input_counter` / `focus` → `st.…`
  - `with_parts(self, …)` → `with_parts(self.v.headline_source(), …)`; ikisi de aynı `&Session`
  - `principals.clone()` → `to_vec()`
  - `permissions(role)` → `ConnView::permissions` (aynı `permissions(role)`)
  - yol değişiklikleri: `crate::viewer::look_of` → `crate::look_of`, `FirstFacts` → `qp_ops::firsts`,
    `gated_view` / `without_layers` / `resolve_range` / `check_target_pub` → `qp_ops::bounds`
    (gövdeler aynı)

  Belgelenmemiş gövde değişikliği bulunmadı.
- **Durum ödünç verme.** `with_ctx_as` (`phases.rs:318`) `slots`, `outbox`, `cache` ve `looks`'u
  çağrı boyunca dışarı alıp sonra geri koyuyor. `ReadView` yöntemleri `&self`, bu yüzden iç içe
  `with_ctx` açamıyorlar. Bu yöntemlerin hiçbiri otoritenin `looks` ya da `cache` alanını okumuyor
  (tarandı: `looks` yalnız `project.rs:102` ve `updates.rs:188`'de, ikisi de okuma çağrısı
  dışında). `ReadState` yuvası `put` ile duruyor, dünya başına değil, yani iki dünya takasında
  takas edilmiyor. Bu, `main`'deki `Session`/`Conn` alanlarının davranışıyla aynı.
- **Başarım (ek).** `bench_input` (sürüm, 2×2000 örnek): `main` ile dal gürültü içinde.
  Sürükleme medyanı 20,6–21,2 µs, kıstırma 20,2–20,9 µs, aç/kapat 45,8–47,2 µs.

## 4. Lint: `#![allow(clippy::needless_borrow)]`

- `arch-check`'in `lint.inner` kuralı yalnız korunan lintlere bakıyor (`unwrap_used`,
  `expect_used`, `panic`, `too_many_lines`, `cognitive_complexity`, `restriction`, `nursery`;
  `rules.py:222`). Bu yüzden kural ihlali **yok**.
- Ancak izin yeni. `main`'in `qp-authority`'sinde yok; taşınan kod alan ödüncü olduğu için
  gerekmiyordu. `rewritten.md` S3.2'nin FLAG satırı yalnız `result_large_err`'i anıyor; bu izin
  hiçbir belgede anılmıyor.
- S2 emsali aynı lint için 6 yeri elle düzeltti (`rewritten.md:62`).
- Ölçüm: izin satırı silinince `cargo clippy -p qp-read -D warnings` **47 hata** veriyor:

  | Dosya | Hata |
  |---|---|
  | `interact/scenes.rs` | 14 |
  | `reading.rs` | 10 |
  | `query.rs` | 9 |
  | `bindq.rs` | 5 |
  | `interact.rs` | 4 |
  | `query/view.rs` | 3 |
  | `preview.rs` | 2 |

  Hepsi `&self.doc` / `&self.plane` → `self.doc` türünden mekanik değişiklik.
- **Öneri (B4):** izni kaldır, 47 yeri düzelt, `rewritten.md` S3.2'ye "mekanik `needless_borrow`
  düzeltmeleri (47; dosya listesi)" satırını ekle. Ya da gerçekten gerekiyorsa öğe düzeyinde izin
  ver. Crate düzeyindeki izin bundan sonra yazılacak her gereksiz ödüncü de susturur.

## 6. WASM

`perf.sh`'ın komutları; paket geçici klasöre kuruldu.

| Paket | Taban | Dal | Fark | % | Sınıra kalan (±%2) |
|---|---|---|---|---|---|
| varsayılan raw | 2.345.821 | 2.354.884 | +9.063 | **+%0,386** | 37.853 bayt |
| varsayılan gzip-9 | 769.836 | 773.647 | +3.811 | **+%0,495** | 11.585 bayt |
| kabuk (web, replica) raw | 8.789.414 | 8.957.945 | +168.531 | **+%1,917** | **7.257 bayt** |
| kabuk gzip-9 | 2.432.388 | 2.474.794 | +42.406 | **+%1,743** | 6.241 bayt |

Kapı içinde, ama kabuk paketi sınıra 0,08 puan uzakta (S3.1: +%1,90). FOLLOWER olmasaydı
yazarın ölçümüyle +1,38 MB (≈ +%17) olurdu. S3.3 (yansı ve görüntüleyici taşıması) için bu pay
neredeyse sıfır: B5.

## 7. Sözleşme sapması: `ReadView`

- Plan (§4.2, §9 Y9) `SessionView`'in yöntemlerle genişletilmesini söylüyordu. Yazar ayrı bir
  `ReadView: SessionView` ekledi ve bunu yalnız sorgu ve girdi çağrısında
  `SessionCtx::read: Option<&dyn ReadView>` olarak veriyor.
- Gerekçe ikna edici. `&dyn SessionView` tablosu her faz çağrısında kuruluyor; tablodaki her
  yöntemin arkasındaki kod (öneri açılımı, `stories`, kapsül ayrıntısı) kabuk paketine bağlanıyor.
  `READ` const jeneriği ise faz çağrısının okuma tablosunu hiç adlandırmamasını sağlıyor.
- **Kabul edilebilir.** Alt sistem yine yalnız `qp-subsystem` trait'lerini görüyor:
  `qp-read` → `qp-authority` kenarı yok (`cargo tree` 0, katman sırası `qp-read < qp-authority`).
  "Alt sistemler otoriteyi tanımaz" ilkesi crate düzeyinde korunuyor.
- Notlar (B6):
  - `ReadView` otorite biçimli bir arayüz. Bağlantı sırası indeksi (`usize`) taşıyor
    (`stories_query`, `capsule_query`). `item_memo()` iç `RefCell<ItemMemo>`'yu açıyor.
    `accept_preview` ve `headline_source` var. İlke biçimsel olarak korunuyor; anlamca okuma alt
    sistemi otoritenin kapsül yaşamına sıkı bağlı. Bu, planın "kapsül yaşamı çekirdekte kalır"
    kararının (B2) beklenen bedeli. `DEBT.md`'de anılması önerilir.
  - `SessionCtx`'e eklenen `cache` ve `looks` her faz çağrısında da veriliyor. Böylece
    `qp-watch` gibi bir alt sistem bütün bağlantıların görünüm durumunu değiştirebilir hale
    geliyor; sözleşme genişliyor. İkisini yalnız `READ` çağrısında vermek ya da bunu sözleşmede
    yazmak önerilir.

## Bulgular

| # | Ağırlık | Bulgu | Gerekli / önerilen |
|---|---|---|---|
| B1 | orta | **FOLLOWER belgelenmemiş bir mimari sapma.** `PLAN.md` §3.3 satır 6: yansı otoritesini "`qp-read` + `qp-watch` kaydıyla" kurar. Uygulamada `Replica` ve `OpenSide` `qp-read`'siz kuruluyor. `DECISIONS.md` ya da `rounds/RF1/DECISION.md`'de karar satırı yok (CLAUDE.md: mimari seçim → karar satırı). Şu iki yer artık yanlış: `docs/ARCHITECTURE.md` "Yeni alt sistem nasıl eklenir" 6 ("bileşimin kurduğu her otorite (… yansı, görüntüleyicinin açık tarafı) aynı listeyle kurulur") ve 7 ("`LIST`'ine bir satır"), `docs/CONCEPTS.md:290` "Bileşim" (aynı cümle). Yeni bir alt sistemin `FOLLOWER_LIST`'e girip girmeyeceği hiçbir yerde sorulmuyor. | Birleştirmeden önce: karar satırı (gerekçe: WASM ölçümü, izleyiciye sorgu yolu yok, §2 taraması); ARCHITECTURE 6–7 ve CONCEPTS "Bileşim" düzeltmesi; 7. adıma "izleyicide gerekiyorsa `FOLLOWER_LIST`'e de" maddesi. |
| B2 | düşük-orta | **Kapsam boşluğu.** `qp_session::tape::{run, run_with, run_viewer}` (`qp-session/src/tape.rs`) hem başvuru otoritesini hem yansıyı hem de `OpenSide`'ı `STANDARD` ile kuruyor. Yerel şerit ve charac yansı sınamaları ürünün FOLLOWER yansısını sınamıyor; FOLLOWER yalnız WASM tekrarında (adım 15, araç varsa) ve `qp-shellcore` sürüş paritesinde sınanıyor. İki listenin uyumunu (`FOLLOWER` = `STANDARD` − `read`) koruyan bir sınama da yok. | `qp_authority::tape` iki liste alsın (otorite `STANDARD`, izleyiciler `FOLLOWER`); ya da bir sınama `FOLLOWER` adlarının `STANDARD` eksi `read` olduğunu doğrulasın. |
| B3 | düşük | **Gizli risk.** `Replica::session_mut()` (`replica.rs:359`), `qp_session::Replica` `Deref`'i ve `OpenSide::replica_mut()` izleyici `qp_authority::Session`'ı dışa açıyor. Gelecekte biri bunun üstünde `handle_raw` ile sorgu ya da girdi gönderirse sessizce `unsupported` alır. Bugün çağıran yok (§2). | `session_mut` belgesine "izleyici: sorgu/girdi almaz (FOLLOWER)" notu; isteğe bağlı olarak `debug_assert`. |
| B4 | orta | `qp-read/src/lib.rs` crate düzeyinde `#![allow(clippy::needless_borrow)]`: yeni ve atıfsız, 47 yeri kapsıyor (§4). `lint.inner` kuralını çiğnemiyor ama S2 emsali ve "clippy uyarısız" kuralının ruhuna aykırı. | Birleştirmeden önce: kaldır ve mekanik olarak düzelt (ya da öğe düzeyinde izin ver); `rewritten.md`'ye yaz. |
| B5 | bilgi | WASM kabuk paketi +%1,917 (sınıra 7.257 bayt); gzip +%1,743 (6.241 bayt). | S3.3 öncesi pay planı: hangi kod pakete girecek, nasıl dışarıda tutulacak. |
| B6 | bilgi | `ReadView` 27 yöntem ve otorite biçimli (bağlantı indeksi, `RefCell` belleği). `SessionCtx::{cache, looks}` faz çağrılarında da ödünç veriliyor (§7). | `DEBT.md` notu; `cache`/`looks`'u yalnız `READ` çağrısında vermek. |
| B7 | bilgi | `rd::reading` (`rd.rs:96`) `ReadState` yuvası yoksa sessizce taze bir durumla koşuyor; durum kaybolur. Kayıtlı `READ` ile olmuyor. | `debug_assert!` ya da hata. |
| B8 | bilgi | İ3 tavanı 16.699, 13.000 sınırının üstünde. `DECISION.md` (S3.0 notu) sınırı S3.3 sonunda uyguluyor; bu dal için uygun. | S3.3 sonunda yeniden ölç. |

## Yeniden denetim: `rf1/s32-read` @ 93dd8bd7

**Hüküm: GEÇTİ.** B1–B4 kapandı. Davranış `main` ile aynı. Tape'teki liste değişikliği
davranışı korudu: dört şerit `compare.sh` içinde bayt eşit, son özetler ilk denetimdekiyle aynı.

Denetçi kendi worktree'sinde çalıştı (`.claude/worktrees/audit-s32b`, `--detach`). Derleme klasörü
`target/wt/audit-s32b` idi. Dal değiştirilmedi; `git status` temiz kaldı. İncelenen commit'ler:
690b1788 (B1–B3), ab9d578d (B4), 6f8224a3 (`paths.md`), 93dd8bd7 (`Lists`, İ3).

| Adım | Sonuç |
|---|---|
| `cargo fmt --all -- --check` | temiz |
| `cargo clippy --workspace --all-targets -- -D warnings` | temiz |
| `cargo test --workspace` | 209 ikili, 1243 geçti (+1: `the_follower_list_is_the_standard_list_without_reading`), 0 kaldı, 9 yok sayıldı |
| S1 / G1 / ext / `qp-session --test replica` | 25/25, 11/11, 4/4, 5/5 |
| Kitaplar | `git diff bd0fdd76 HEAD -- */tests/charac/expected` boş |
| `compare.sh` | `270 items byte-identical`; `replica/{tape0,tape25,viewer0,viewer25}.json` ve `drive/` kalemleri dahil |
| verify adım 15 (elle) | `qp replica-tape` lag 0/25 ve `--viewer` 0/25: 0 ihlal, nihai iddiaların hepsi nihai gösterildi, öbür enstrümana abonelik 0. WASM (node) tekrarı: wasm == yerel 186/186 ve 211/211; lag 0'da wasm == otorite 186/186. Son özetler (doc 832528870cf6c127, view 8a04abbca67e3bdd, display 0ce72165f9b4dbee) ilk denetimdekiyle aynı |
| `scripts/arch-check` | 0 hata; İ3 ölçülen **16.707 = tavan 16.707** |
| `--config-diff main` | 24 değişiklik, 0 atıfsız gevşeme. İ3 20.129 → 16.707 (sıkı); taban çizgisinde `interact.rs` 696→685, `depth_view` 106→105, `query` 372→365 (sıkılaştı) |
| WASM kabuk paketi | raw 8.957.945 (**+%1,917**, sınıra 7.257 bayt); gzip 2.474.782 (**+%1,743**, sınıra 6.253 bayt). ≤ +%2 |

**B1: kapandı.**
- `rounds/RF1/DECISION.md`'ye uygulama notu eklendi. Not FOLLOWER'ı tanımlıyor, ölçümü veriyor
  (+1,38 MB, +%17,6 → +%1,92), §3.3 satır 6'dan sapmayı adıyla anıyor, kanıt olarak bu
  denetimin adım 2'sini gösteriyor ve geri dönüş koşulunu yazıyor.
- `docs/ARCHITECTURE.md` 6 artık iki listeyi doğru anlatıyor. 7'ye `FOLLOWER_LIST` kuralı ve
  sınamaya atıf eklendi.
- `docs/CONCEPTS.md` "Bileşim" ve `crates/qp-session/README.md` kodla tutarlı. Kodla karşılaştırdım:
  `compose.rs` `LIST`/`FOLLOWER_LIST` ve `tape.rs`'in `(STANDARD, FOLLOWER)` çifti.

**B2: kapandı.**
- `qp_authority::tape::{run, run_with, run_viewer}` `Lists = (Subsystems, Subsystems)` alıyor.
  `subs.0` başvuru otoritesine gidiyor (`tape.rs:278`, `:530`). `subs.1` yansıya (`:279`) ve
  `viewer_deliver` üstünden `OpenSide::new`'e (`:479`, `:549`, `:565`) gidiyor. Başka kullanım yok.
- `qp_session::tape` `(STANDARD, FOLLOWER)` veriyor. Böylece yerel şerit artık ürünün izleyici
  listesini sınıyor.
- `the_follower_list_is_the_standard_list_without_reading` adları `Vec` eşitliğiyle
  karşılaştırıyor, yani **sırayı da sınıyor**. Ayrıca `read`'in `STANDARD`'da olduğunu doğruluyor.
- Sınır: sınama yalnız `read`'i dışarıda bırakan kuralı kodluyor. İzleyicide olmaması gereken
  yeni bir alt sistemde (yalnız sorgu/girdi) sınama elle güncellenmeli. ARCHITECTURE 7 bunu
  açıkça söylüyor; bu kabul edilebilir.

**B3: kapandı.** Notlar eklendi:
- `Replica::session_mut` belgesi (`replica.rs:358–361`): izleyici listesi, `handle_raw` ile
  sorgu ya da girdi `unsupported`.
- `OpenSide::replica_mut` belgesi (`open.rs:298–299`).
- `qp-session` README'de aynı cümle.

**B4: kapandı.**
- `crates/qp-read/src` içinde crate ya da modül düzeyinde izin olarak yalnız `result_large_err`
  kaldı (otoritenin aynısı, ilk denetimde bilgi).
- 7 dosyadaki farkın (`git diff -U0 7871e187 93dd8bd7 -- crates/qp-read/src`, +42/−65)
  tamamını okudum. Satırların hepsi `&self.doc` → `self.doc`, `&mut self.cache` → `self.cache`,
  `&self.cache` → `self.cache`, `&self.headlines` → `self.headlines` türünden ve `rustfmt` ile
  yeniden sarıldı. Elle bakılan 10 örnek:
  - `bindq.rs` `gate`, `need_series`, `find_series`, `values`
  - `query.rs` `present(…)`, `find_collection`, `collection_items`, `future_times`
  - `reading.rs` `view_bar_series_any` / `gate`
  - `preview.rs` `need_proposal`
- Başka hiçbir ifade değişmemiş.
- Değer aynı: `Rd`'nin alanları zaten başvuru. `&mut Cache` → `&Cache` beklenen yerde örtük
  dönüşüm var.
- `rewritten.md` S3.2'ye 7 dosyalık satır eklendi.

**Bilgi (yeni):**
- İ3 tavanı bu düzeltmelerle 16.699'dan **16.707'ye** çıktı (+8 satır: `Lists` tipi,
  `session_mut` ve `replica_mut` belge satırları). `main`'e göre hâlâ sıkılaşma (20.129 →
  16.707); `config-diff` 0 atıfsız gevşeme diyor. Dal birleşmeden ölçülen değere ayarlandı.
- `arch-config.json` İ3 `max_note`'unun S3.2 kısmı hâlâ "tavan ölçülen değere indi" diyor;
  +8'in nedenini (audit-s32 B2/B3) bir sözcükle anması önerilir.
- WASM payı değişmedi; S3.3 için uyarı (B5) geçerli.
