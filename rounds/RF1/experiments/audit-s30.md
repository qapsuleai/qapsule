# RF1/S3.0 bağımsız denetim: `rf1/s30-registry` @ 85e5b07d

**Hüküm: GEÇTİ.** Boş kayıtla davranış bugünküyle aynı (charac, `compare.sh` 270/270, elle okuma).
Sözleşme bir alt sistemin `qp-authority`'ye dokunmadan yazılabileceği biçimde. Aşağıdaki bulguların
hiçbiri boş kayıtla bir yolu değiştirmiyor; B1 ve B2 taslak kancalarda, ilk gerçek kullanıcıdan
(S3.1 / RL1) önce kapatılmalı.

Denetçi kendi worktree'sinde çalıştı (`.claude/worktrees/audit-s30`, `--detach`), ortak olmayan
derleme klasörüyle (`target/wt/audit-s30`). Dal değiştirilmedi, birleştirilmedi. Geçici deneme
dosyaları (sınama, araç kopyası) yalnız denetim worktree'sinde ve oturum scratchpad'inde durdu,
silindi.

## Sonuçlar

| # | Adım | Sonuç |
|---|---|---|
| 1 | `cargo fmt --all --check` | temiz |
| 1 | `cargo clippy --workspace --all-targets -- -D warnings` | temiz |
| 1 | `cargo test --workspace` | 203 sınama ikilisi, 1230 geçti, 0 kaldı, 9 yok sayıldı |
| 1 | S1 charac (`qp-session --test charac`) / G1 charac (`qp-designs --test charac`) | 25/25 / 11/11 |
| 1 | Kitaplar (`crates/qp-session/tests/charac/expected`, `crates/qp-designs/tests/charac/expected`) | `git diff main HEAD` boş; koşulardan sonra `git status` temiz (yeniden yazılmadı) |
| 2 | `scripts/refactor/compare.sh` | `270 items byte-identical to the baseline` |
| 3 | `moved.py main HEAD` | `CHANGED -` 37 öğe = `rewritten.md` "## S3.0" tablosundaki 37 öğe, eksiksiz ve fazlasız. `CHANGED +` taraf: aynı 37'nin yeni hali, `phases.rs` (yeni), `note_audiences`, `initial_audiences`, `qp-subsystem` ve sınamalar. FLAG'ler: `qp-session/src/lib.rs` yönlendirmeleri (`crate::session::X` → `qp_authority::X`, yüzün kendisi), sınama dosyalarındaki `#![allow]`, `viewer/updates.rs` `use super::*` (taşınan dosya). Hepsi okundu, beklenen. |
| 3 | 10 öğe elle | `confine`, `read_json` (→ `files.rs`, bayt eşit), `AgentOp` (→ `qp-subsystem`, türetmeler ve belge aynı), iki `op_name` (`Value::as_str` ile `\|o\| o.as_str()` eşdeğer; tek kopya), `conform.rs` (yalnız 2 `use` satırı), `hello` ve `verdict_line` (yalnız görünürlük), `Computed::new` (yalnız `pub`), `note_audiences` (gövde aynen), `initial_audiences` (aynı ifade). Ek olarak 12 rastgele dosyanın 8'i `main:qp-session/src/X` ile `HEAD:qp-authority/src/X` blob'u aynı; kalan 4'ü yeniden yazılan öğelerin dosyası. |
| 4 | Sözleşme | aşağıda; yeniden giriş deneyle sınandı |
| 5 | `cargo tree -p qp-authority` | `qp-subsystem` (sözleşme) var, alt sistem crate'i yok, `qp-session` yok |
| 5 | `scripts/arch-check` | 0 hata; 2 adlı istisna; İ3 `qp-authority` 20.154 / tavan 20.437 |
| 5 | `scripts/arch-check --config-diff main` | 63 değişiklik, 0 atıfsız gevşeme; `moved exception İ3 [crate.lines] qp-session -> qp-authority (new crate; max 20437)`; `lint.allow` yalnız yeni crate'e yayıldı (toplam büyümedi); `committed` kalemi silindi, `fn.lines` sayısı 176 → 175 |
| 5 | Katman sırası | `qp-plane < qp-ops < qp-subsystem < qp-authority < qp-session` (arch-check çıktısı) |
| 5 | İ3 tavanı | yükselmedi (20.437 = `main`) |
| 6 | `scripts/arch-check --self-test` | PASS (3 yeni vaka dahil) |
| 6 | `_moved_exception` kaçış denemeleri | aşağıda (B7) |
| 7 | Performans | iddialar tutuyor; aşağıda |
| 8 | Yeni crate'ler | README'ler var; `[lints] workspace = true` üçünde de; `lib.rs` yalnız bildirim ve yeniden dışa aktarım; klasör başına dosya ≤ 15 (yeni klasörler: `qp-subsystem/src` 8, `qp-authority/src` 8, `capsule` 14) |

## Boş kayıt: her yol bugünkü mü? (adım 3)

Her yeniden yazılan otorite öğesi okundu. Boş kayıtla her kontrol hemen ıskalıyor; yan etki sırası
ve hatalar aynı:

- **Kayıt aramaları.** `Registry::{op, query, input, topic, claim}` tablo boşsa serileştirmeden
  `None` döner. `allowed`, `proposable`, `agent_check` (`map_or_else` → `agent_op`), `ops_to_pub`
  (`other` kolu → `Some(other.clone())`), `project_entry` (`|| false`) bugünkü sonucu verir.
- **Açılım.** `expand_ops` boşken doğrudan `qp_ops::expand::expand_ops`'u çağırır. `expand_op`
  (`resolve_selectors`, `bar_command`) boşken doğrudan `qp_ops::expand::expand_op`'u çağırır.
- **Koşullar.** `Registry::eval_condition` koşul ve kaynak tabloları boşken doğrudan
  `qp_plane::values::eval_condition`'dır. `Registry::holds` ile `qp_plane::values::holds` aynı
  ifade (`values.rs:601`).
- **Fazlar.** `phase` `wants` yanlışsa `with_ctx`'e girmez. `Moment` argümanları (`self.seq()`,
  `&changed`) saf. `check_command` önce `check_comments`, sonra `wants(Check)` yanlış → `Ok`.
  `dispatch_hello`: önce `hello`, sonra `conn_index` (saf arama). `settled_phase`: önce
  `capsule_settled`, sonra faz. `unsubscribed`: önce `viewer_unsubscribed`.
- **Yazma adımı.** `emit` `outbox.is_empty()` ise boş vektör döner. `handle_msg`, `settle`,
  `settle_data`, `settle_live` ve `advance_clock` sonlarına eklenen `extend(emit())` boşken satır
  eklemez. `handle_raw`'da `emit` `viewer_flush`'tan önce koşar, boşken fark yok.
- **Takas.** `swap_world` → `slots.swap_world()` boş `worlds` üstünde döngü yok.
- **Diğer.** `dispatch_subscribe`'da fazladan bir `conns[ci].id.clone()` (gözlenemez).
  `host_adjust` boşken satırsız ve girdisiz (sınamada da kilitli).

**Boş kayıtla sırayı ya da çıktıyı değiştirebilecek bir yer bulunmadı.** Tek gözlenebilir fark yeni
dışa açık yüzey: `register` ve `host_adjust` (gizli), `next_wake`, `impl SessionView for Session`.
Yerleşik yöntemler aynı adlı trait yöntemlerinden önce gelir.

## Sözleşme incelemesi (adım 4)

- **`dyn` uyumu.** `Subsystem`, `SessionView`, `OpHandler`, `QueryHandler`, `InputHandler`,
  `TopicHandler`, `ConditionKind`, `ValueSource`, `ClaimKind` jenerik yöntem taşımıyor. Jenerikler
  yalnız somut `Slots` üstünde (`put<T>`, `get<T>` …).
- **Bölünmüş ödünç.** `with_ctx` yuvaları ve giden kutusunu `mem::take` ile çıkarıp geri koyuyor.
  `view: &Session` değiştirilemez olduğu için kapanış içinden faz çağrılamaz. İç içe `with_ctx`
  olamaz, yani yuvalar sessizce boş görünmez.
- **Yeniden giriş, deneyle.** Geçici sınamada iki alt sistem vardı: W, `AfterCommit`'te iki motor
  girdisi istiyor, w1'in commit'inde bir üçüncüsünü; R yalnız not alıyor. Panik yok, `RefCell`
  yok. W'nin isteği commit'lenince R'nin `AfterCommit`'i de çağrılıyor. Günlük sırası
  `add_narrative`, `w1`, `w1-child`, `w2` (bkz. B3).
- **Yalnız bu crate'le yazılabilir mi?** Evet. Kayıt `Session::register(&'static dyn Subsystem)`
  ile yapılıyor. Okuma `SessionView`'dan, yazma `Outbox`'tan ya da açılımdan geçiyor. Kayıt
  kolları kapalı şema enum'larının tel etiketine bağlı. Bu yüzden bir alt sistem yalnız **var
  olan** bir işlem, sorgu, girdi, konu, koşul ya da iddia türünü üstlenebilir; yeni tür şema
  değişikliği ister. Plan da böyle diyor (§4.5, "şema değişikliği yok"), ama `ConditionKind`
  bugün yalnız `threshold`, `compare` ya da `text`'i ezebilir. RL1'in `qp-program`'ı şemaya bir
  koşul türü ekleyecek. Bunu RL1 planına not edin.
- **4 + 4 yeni sınama.** Hepsi kayıt yollarını gerçekten kullanıyor:
  - Otorite üstünden sınananlar: `Check` reddi, işlem açılımı (dolu `Registry::expand_ops`),
    `AfterCommit`, `Connect`, `Subscribe`, `Unsubscribe`, sorgu dağıtıcısı, iddia türü
    dağıtıcısı, `Settled`, `Adjust`.
  - Otorite üstünden **sınanmayanlar:**
    - `AfterData` fazı.
    - `Proposals` fazı: Probe adını veriyor ama sınamada saat ilerlemiyor; 8 kayıtlı not içinde yok.
    - `InputHandler`, `TopicHandler`.
    - `ConditionKind` ve `ValueSource` canlı yol üstünden (yalnız `Registry` doğrudan sınanıyor).
    - `OpHandler`'ın politika yöntemleri (`allowed`, `proposable`, `agent`, `to_public`,
      `reader_private`): hepsi varsayılanla çalışıyor, hiçbiri iddia edilmiyor.
    - `swap_world` ile yuva takası (yalnız `Slots` birim sınaması var).
    - `wake` ve `next_wake` otorite üstünden.
    - Yeniden giriş.

    PLAN §4.7 "uygulanan kancalar gerçek kodla sınanır" diyor. S3.0'da gerçek alt sistem yok;
    S3.1 (`qp-watch`: `OpHandler` politikası, `AfterCommit`/`AfterData`) ve S3.3 (konu,
    görüntüleyici) bunları kapatmalı.
  - `an_empty_registry_calls_nothing_and_a_probe_sees_every_phase_in_order` adı fazla vaat ediyor:
    boş kaydı yalnız `next_wake() == None` ile sınıyor. Asıl boş kayıt kanıtı charac ve
    `compare.sh`.
- **Ölü ya da sınanamaz kalemler** (B4).

## Bulgular

**B1 (orta; taslak kanca `ConditionKind`): canlı yolun memo hızlı yolu kaydı atlıyor.**
- **Yer:** `data/live.rs` `eval_memo`. Kullananlar: `eval_conditions_live`, `eval_intents_live`,
  `invalidate_live`.
- **Ne oluyor:** İlk değerlendirme `reg.eval_condition`'a gidiyor. Aynı önbellek kuşağında
  sonraki çağrılar `eval_slots` ile yerleşik `Threshold`/`Compare` değerlendirmesini döndürüyor;
  `Text` için `Some(None)` döndürüyor. Kayıtlı bir `text` ya da `threshold` türü (sınamadaki
  `Always` gibi) ikinci çağrıda devreden çıkıyor.
- **`ValueSource` etkilenmiyor:** kaynağın serisi önbellekte yuva almadığından memo düşüyor.
- **Etki:** boş kayıtla yok. Sınama (`live.rs`) yalnız `Registry::eval_condition`'ı doğrudan
  çağırıyor; bu yolu görmüyor.
- **Öneri:** koşul tablosu doluysa memo'yu atlayın, ya da `eval_slots`'tan önce türü kayda sorun.

**B2 (orta; taslak `wake`): uyandırma hiç temizlenmiyor.**
- **Ne oluyor:** `emit` ve `host_adjust` `wake`'i `min` ile birleştiriyor. Saat uyandırma anını
  geçince de değer kalıyor.
- **Deney:** `wake_at(100)` sonrası `host_clock(i64::MAX/2)`; `next_wake()` hâlâ `Some(100)`.
  Sonraki her istek bundan büyükse yok sayılır.
- **Etki:** bugün tüketicisi yok (hiçbir barındırıcı `next_wake` okumuyor).
- **Öneri:** `advance_clock`'ta `now >= wake` ise sıfırlayın.

**B3 (düşük): yazma adımının sırası derinlik öncelikli.**
- **Ne oluyor:** `emit` → `settle` → `emit` özyinelemesi, ilk girdinin doğurduğu girdiyi ikinci
  istenen girdiden önce yazıyor (`w1`, `w1-child`, `w2`).
- **Belge farkı:** belge "request order" diyor. Belirlenimci, ama belgede yazmıyor.
- **Risk:** kendi `AfterCommit`'ine her girdide tepki veren bir alt sistem sonsuz döngüye
  girebilir; bunun koruması yok.
- **Öneri:** sırayı belgeleyin (ya da kuyruğa alın) ve derinlik sınırı ekleyin.

**B4 (düşük): ölü ya da sınanamaz sözleşme kalemleri.**
- `Phase::Close`: çağrı noktası yok (belgede yazıyor).
- `Slots::drop_conn`: otoriteden hiç çağrılmıyor. Bağlantı başı durum bağlantı kapanınca sızar.
- `Moment::Command`: hiç kurulmuyor; `Check` fazı `Subsystem::check(conn, ops)` ile çağrılıyor.
- `RunKind`, `run_kind`, `replay_position`, `param`: hep `None` dönüyor.
- `next_wake`: tüketicisi yok.
- `Phase::Expand`: toplama fazı değil (yalnız ad).

Ya kullanım round'unda kurulmalı ya da kaldırılmalı.

**B5 (düşük; S3.3 için önemli): kayıtlı dağıtıcılar yerleşik korumaları atlıyor.**
- `dispatch_dispute` kayıtlı iddia türünde doğrudan hüküm satırı yazıyor. Yerleşik `dispute`'un
  sınırlarını (`MAX_OPEN`, `MAX_PER_CONN`, özet sayısı) ve denetimlerini atlıyor.
- `dispatch_subscribe` konu işleyicisi reddedince yalnız `subs.retain` yapıyor. Yerleşik
  `subscribe`'ın öbür yan etkilerini geri almıyor (ör. görüntüleyici kaydı; `viewer_unsubscribed`
  çağrılmıyor).

**B6 (düşük): kopya gövde.** `Registry::expand_ops`, `qp_ops::expand::expand_ops`'un ≈ 30 satırlık
birebir kopyası (hata kodu seçimi dahil). İki kopya zamanla ayrışabilir. Öneri: açılımcıyı kapanış
olarak alan tek gövde `qp-ops`'ta dursun.

**B7 (düşük; araç `_moved_exception`).** Denemeler araç kopyasında yapıldı (`Citer` boş, atıf yolu
kapalı):

| Deneme | Sonuç |
|---|---|
| yeni crate'e aynı id, kural, tavan | geçti (beklenen) |
| var olan crate'e (`qp-session`) | 1 hata ✓ |
| tavan yükseltme / tavanı kaldırma | 1 hata ✓ |
| kural değişikliği / id değişikliği / başka istisnanın id'si | 1 hata ✓ |
| eskisi kalırken yeni crate'e kopya | 1 hata ✓ |
| tavan düşürme | geçti (doğru) |
| yeni crate yalnız `layer_overrides`'ta | taşıma geçti, override atıfsız → 1 hata ✓ |
| **iki istisna tek (kural, öğe) anahtarına** (X `qp-ops`→`qp-n` 100, Y `qp-plane`→`qp-n` 200) | **0 hata** |
| `crate.lines` istisnası bir fn yoluna (`crates/qp-n/src/a.rs::f`) | 0 hata |
| `status`/`reason` metni değişti | 0 hata |

- **Çift anahtar.** Sözlük sonuncuyu tutuyor; X "removed" diye raporlanıyor ama yapılandırmada
  duruyor. `baseline.split_exceptions` aynı sözlük anlamını kullandığı için tutarlı ve tavan
  yükselmiyor. Zaaf önceden var (çift anahtarın genel sorunu), taşımaya özgü değil. Öneri: yeni
  yapılandırmada çift `(rule, item)` anahtarını FAIL sayın.
- **Fn yolu.** `crate.lines` kuralının fn yoluna taşınması anlamsız ama zararsız. Öneri: `rule`
  `crate.lines` ise öğe yalnız crate adı olsun.
- **Belge farkı.** DECISION.md "eski crate artık yok" diyor; kod bunu şart koşmuyor (S3.0'da
  `qp-session` yaşıyor). Kod yalnız "eski (kural, öğe) kalemi gitti" şartına bakıyor. Belgeyi ya
  da kodu hizalayın. Modül belgesindeki "Known limits" istisna taşımasını anmıyor.
- **Bu değişiklikte gerçekten gerekli miydi?** `rounds/RF1/DECISION.md` bu değişiklikte değişti
  ve `qp-authority`'yi anıyor. Araç olmasa da atıf yoluyla `LOOSER` geçerdi. Araç taşımayı
  "gevşeme" yerine "taşıma" diye adlandırıyor; doğru ama zorunlu değildi.

**B8 (düşük; mandal): İ3 tavanı indirilmedi.**
- **Durum:** ölçülen 20.154, tavan 20.437. Fark 283 satır.
- **Neden önemli:** `conform` (≈ 600 satır) çıktı, `phases.rs` ve kayıt satırları girdi.
  "Tavan mandallıdır" kuralına göre tavan 20.154'e inebilirdi. İnmezse sonraki dalgalarda
  çekirdeğe ≈ 280 net satır sessizce girebilir.
- **Öneri:** S3.1'den önce tavanı ölçülen değere indirin. Bu bir sıkılaşma, karar gerektirmez.

**B9 (belge).** `crates/qp-session/README.md`:
- "Bağımlılıklar: `qp-authority`" diyor. Normal bağımlılıklar arasında `conform` için `qp-ops`,
  `qp-plane`, `qp-schema`, `serde`, `serde_json`, `tempfile` de var.
- "Düzen: `src/lib.rs` (yalnız yeniden dışa aktarımlar)" diyor; `src/conform.rs` de var.

İstenen kural yazılı: "Yeni kod `Deref`'e yaslanmaz, `sub::<T>()` / `sub_mut::<T>()` kullanır".

## Performans (adım 7)

`perf-s30.txt` (commit 56d8463f) ile `baseline/perf.txt` (7fb01d11) karşılaştırması:

- **K17 `sweep_bench` medyan ms:**
  - sma 4,37 / 4,39
  - ema 4,05 / 4,05
  - rsi 10,13 / 9,59 (+%5,6)
  - macd 8,20 / 8,63 (−%5,0)
  - bollinger 8,95 / 9,71 (−%7,8)

  İki yönlü saçılma, gürültü. Özetler eşit. Sınanan kod (`qp-compute`) S3.0'da değişmedi. Not:
  VERIFY 11 "A/B aynı saatte" istiyor; dosyada tek koşu var, taban bir gün önceki.
- **K52 (CPU ×4), ortalama ms:**
  - `tick` 0,142 / 0,135 (p50 0,1 ve p95 0,5 aynı)
  - `data_pass` 3,544 / 3,763
  - `msg` 1,241 / 1,297
  - `paint_pass` 3,049 / 3,234

  Gürültü içinde; boş kaydın tik maliyeti görünmüyor. Not: S3.0 koşusunda `"bundle": null`
  (tabanda dolu).
- **WASM varsayılan:** 2.354.924 / 2.345.821 (+%0,39); gzip 773.690 / 769.836 (+%0,50). ±%2 içinde.
- **WASM kabuk paketi:** 8.913.208 / 8.789.414 (+%1,41); gzip 2.460.047 / 2.432.388 (+%1,14).
  ±%2 içinde, ama bu round boyunca birikmiş artış.
- **S3.0'ın kendi payı (denetçinin ölçümü):** `cargo build -p qp-wasm --target
  wasm32-unknown-unknown --profile wasm`, aynı derleme klasörü, `main` 04fff120 ile HEAD:
  - varsayılan: bayt bayt aynı (2.354.884 / 773.647)
  - `--features replica`: 9.737.705 → 9.762.676 (**+24.971, +%0,26**); gzip 2.486.906 →
    2.492.276 (+%0,22)

  Kabuk paketindeki %1,41'in çoğu önceki dalgalardan geliyor. Kalan pay ≈ %0,6; S3.1–S3.3 için
  izlenmeli.

## Öneriler (kapsam dışı; yapılmadı)

1. B1 ve B2'yi S3.1'den önce düzeltin; her birine otorite üstünden bir sınama ekleyin (canlı yolda
   kayıtlı koşul türü ve uyandırmanın saat sonrası temizlenmesi).
2. İ3 tavanını ölçülen 20.154'e indirin (B8).
3. `configdiff`'te çift `(rule, item)` istisnasını FAIL sayın; `crate.lines` taşımasını crate
   adlı öğeyle sınırlayın; belge/kod farkını kapatın (B7).
4. B4 kalemlerini kullanım round'una bağlayın ya da kaldırın. `Close` gelince `drop_conn`'u çağırın.
5. `Registry::expand_ops` ile `qp_ops::expand::expand_ops`'u tek gövdeye indirin (B6).
6. Kayıtlı `ClaimKind` ve `TopicHandler` dağıtıcılarında yerleşik korumaları ve geri almayı
   koruyun (B5; S3.3 görüntüleyici dalgası).
7. Bileşim README'sini düzeltin (B9).

## Disk

- Silinenler: `.claude/worktrees/audit-s30` (`git worktree remove`), `target/wt/audit-s30`
  (debug, release, wasm ve `compare.sh` ağacı dahil), scratchpad'deki araç kopyası, deneme sınaması
  ve dökümler.
- Bilerek bırakılan: yok.
