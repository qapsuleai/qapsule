# RF1 PLAN v1 — Eleştiri 1 (yapılabilirlik ve davranış riski)

Bağımsız eleştirmen. Yalnız okuma ve `grep`; derleme yok. Kod `05430e06` + çalışma ağacı.
Her bulgu: plandaki iddia → bulunan (dosya:satır) → önem → öneri.

## A. Ölçümler ve kenarlar

### 1. "işlemler → otorite: 3 yöntem (`build`, `build_with`, `document`)" bir ölçüm yanılgısı
- **Bulunan:** `ops/*` içindeki `.build()` çağrıları `E::build` (`authority/error.rs:78`), hata
  kurucusu; `build_with` `BuildWith` trait'i (`ops/select.rs:361,365`); `document` yalnız
  `#[cfg(test)]` içinde (`ops/summary.rs:459`, `:479-480` `crate::Session::new(…).document()`).
  `ops/*`'tan `Session` yöntemine test dışı **gerçek çağrı yok**. Aynı ad çakışması
  (`build`) arayüz tablosunun neredeyse her satırında var (kapsül→otorite, veri→otorite,
  okuma→otorite, yansı→otorite, barındırma→otorite); yani sayılar biraz şişkin.
- **Önem:** düşük (sonuç plan için iyi haber, ama `Host` API'si "ölçümle" kurulacaksa ölçüm
  alıcıya bakmalı).
- **Öneri:** `graph.py` çağrıyı alıcı ifadesine göre süzsün (`E::new(…)…build()` zinciri,
  `Self::build`); arayüz tablosu yeniden üretilsin.

### 2. "`qp-ops`'un dışarıdan tek ihtiyacı 5 serbest fonksiyon" eksik
- **Bulunan:** planın `qp-ops`'a koyduğu `present` (`read/present.rs:8`) `DocProvider`'ı
  kullanır; `DocProvider` bütün `Session`'ı tutar (`data/provider.rs:30`
  `parts: Option<(&'a crate::Session, Language)>`, `:72`, `:248-250` `s.reader_parts(…)`) ve
  kapsül yaşamı fonksiyonlarını çağırır (`provider.rs:226,230` `crate::capsule::story_*`).
  `ops/expand/op.rs:716` `presenter_view`'u çağırdığı için `qp-ops` → `DocProvider` →
  `Session` kenarı doğar. Ayrıca `capsule/story.rs:58` `crate::replica::fnv1a64`,
  `ops/expand/undo.rs:123` `ENGINE_PRINCIPAL`. `ops/summary.rs:459-492` birim sınaması
  `Session` kurar: `qp-ops`'ta `src` içi test olarak kalamaz (geliştirme bağımlılığı döngüsü
  crate'in ikinci kopyasını derler, tipler uyuşmaz).
- **Önem:** orta.
- **Öneri:** `DocProvider`'ın `parts` alanı bir trait nesnesine (`&dyn HeadlineSource`:
  `headline_parts`, `lang`) çevrilsin; `DocProvider` (dışa açık tip; `qp_session::DocProvider`)
  `qp-ops`'a insin, yol değişikliği `paths.md`'ye. `fnv1a64` `qp-ops`'a. `summary.rs` testi
  `qp-session/tests/`'e taşınsın (S2a'nın listesine yazılsın).

### 3. Önerilen bölüm döngüsüz değil (katman içi sıra tutmaz)
- **Bulunan:**
  - `qp-watch` (3a) ↔ `qp-capsule-life` (3c): `capsule/watch.rs:333` `self.reader_proj(…)`,
    `:315` `public_view`; ters yönde `capsule/run.rs:121` `self.match_watches(&events)`
    (izleme değişimleri kapsül girdisiyle **aynı girdide**).
  - `qp-claims` (3b) ↔ `qp-capsule-life`: `dispute.rs` → `capsule_settled`, `capsule_dispute`
    (`settled.rs:252,366`), `settled_bars`; ters yönde `settled.rs` `self.settle` (itiraz
    durumu, 4 yer), `crate::dispute::SettleRequest`, `crate::claims::anchor_of`.
    `comments.rs:227,235,530` `as_reader`, `capsule_query_with` (`queries.rs:178`).
  - `qp-viewer` (3f, oturum katmanında, `qp-session`'ın altında) → `Replica`:
    `replica/viewer/open.rs:231,258` `OpenSide` bir `Replica` tutar; `viewer.rs:43`. Plan
    `qp-replica`'yı `qp-session`'ın **üstüne** koyuyor → yukarı kenar.
  - `qp-livepath` → kapsül: `data/live` → `capsule/run`, `capsule/claims`, `capsule/public`
    (`experiments/graph/session.txt`).
- **Önem:** yüksek (K-2 sıra kuralı ilk gün kırmızı; "alt sistemler katman içi sırayla tanır"
  varsayımı bu sınırlarla geçersiz).
- **Öneri:** Karar noktasından önce `graph.py`'ye "önerilen bölüm" eşlemesi verilip (dosya →
  hedef crate) crate düzeyinde SCC çıkarılsın; sıra listesi bundan türesin. Somut:
  `settled.rs`'in itiraz/uzlaşma kısmı `qp-claims`'e ya da `claims`+`dispute`+`settled`
  kapsül yaşamıyla aynı crate'e; `match_watches` saf fonksiyon (olaylar → değişimler) olarak
  `qp-watch`'a, görünürlük için projeksiyon parametre; `OpenSide` (cihaz tarafı) `qp-replica`'ya,
  `qp-viewer` yalnız otorite tarafı.

## B. `Subsystem` / `Host` / `HostMut` tasarımı (Rust ödünç alma)

### 4. `as_reader` bir "yerleşim" kancası değil; otoritenin dokümanını takas eder
- **Bulunan:** `capsule/project.rs:452-540`: okuyucu projeksiyonunda otoritenin `doc`, `rd`,
  `view_reads`, `live_index`, `headlines`, `real_facts` alanları projeksiyonla **yer
  değiştirir**, kapanış `f(&mut Self)` çalışır, sonra geri konur. Çağıranlar otoritenin
  kalbi: `dispatch.rs:66,87,125,145,282`, `intake.rs:148`, `command.rs`. `Host::document()
  -> &Document` ile yapılamaz; `HostMut` "yazma yalnız komut yolundan" dediği için de yapılamaz.
  Plandaki `project(&self, host, reader, out: &mut Outgoing)` bu düzeneği karşılamaz.
- **Önem:** yüksek.
- **Öneri:** takas otoritede kalsın (`Authority::as_reader(principal, lang, f)`); kapsül yaşamı
  yalnız saf bir "projeksiyon kurucu" versin (`fn reader_view(&self, view, principal, lang)
  -> ReaderView`). `project` kancası silinsin ya da "projeksiyon kurucu" diye yeniden tanımlansın.

### 5. "İki dünya otoritede kalır" ile alt sistem durumunun alt sistemde olması çelişiyor
- **Bulunan:** `public.rs:58-76` `World` 16 alanı takas eder (`swap_world`, `:172-193`): bunların
  7'si kapsül yaşamının önbellekleri (`fact_ix`, `cap_ix`, `sentences`, `projs`, `views`,
  `head_state`, `headlines`), 2'si canlı yolun (`live_index`, `view_reads`), 2'si izlemenin
  (`intent_state`, `intent_epoch`). `public_view` (`public.rs:204-215`) `runner`'ı (kapsül
  yaşamı) okur; `run.rs:128-133` otoritenin `pending_pub` alanını `engine_entry_with`'ten önce
  yazar (alanla kurulan gizli protokol). Ayrıca `HostMut::in_world<R>(…)` genel (generic)
  yöntem: `&mut dyn HostMut` üzerinde çağrılamaz (dyn uyumlu değil).
- **Önem:** yüksek.
- **Öneri:** dünyaya duyarlı durum otoritenin sahip olduğu tipli yuvalarda dursun
  (`WorldSlot`: kayıt anında "dünyaya duyarlı" işaretli; `swap_world` bunları tek yerde
  takas eder). İmza: `fn in_world(&mut self, public: bool, f: &mut dyn FnMut(&mut dyn HostMut))`
  (dönüş değeri kapanışın yakaladığı `Option` ile). `public_view` için kapsül yaşamı
  `start_public` anında akış listesini (`flow_feeds`) otoriteye bildirsin; `pending_pub`
  `engine_entry_with`'in bir argümanı olsun.

### 6. Alt sistem kayıttayken `&mut` otoriteyle çağrılamaz; yeniden girme zinciri var
- **Bulunan:** `run.rs:139` `engine_entry_with` → `settle.rs:177-205` → `committed`
  (`accept.rs:151`) → `claims_after_bars` (`:197`, iddia), `head_state.dirty_all` (`:183`,
  kapsül yaşamı), `public_entry`/`commit_public` (`:229-231`, kapsül motorunun kamu dünyası).
  `dispute.rs:239-289` → `data_changed`/`settle`/`settle_data` → `run_capsules`. Yani
  çalışmakta olan alt sisteme, otorite üstünden geri girilir. "Çağrı süresince kayıttan çıkar"
  kalıbında `Registry::get::<T>()` etkin alt sistem için `None` döner; `RefCell` kalıbında
  `BorrowMutError` paniği. Bugün kod bunu `runner` için elle yapıyor ve kırılgan
  (`run.rs:150-156` "runner put back").
- **Önem:** yüksek.
- **Öneri:** davranış ve durum ayrılsın: alt sistem nesnesi **durumsuz** (`&'static dyn
  Subsystem`, sıfır boyutlu), durum otoritenin tip haritasında (`state::<T>()`,
  `state_mut::<T>()`; `World` takası da buradan). Böylece `sub.after_commit(&mut authority)`
  takma ad yaratmaz, yeniden girme güvenli olur. Durum ile otorite görünümü birlikte
  gerektiğinde: `fn split<T>(&mut self) -> (&AuthorityView, &mut T)`.

### 7. `rd` + `&mut cache` aynı anda: trait yöntemleriyle bölünmüş ödünç alınamaz
- **Bulunan:** `rd!` makrosu (`macros.rs`) bilerek alan düzeyinde ödünç alır;
  okuma tarafı `warm(rd!(self), &mut self.cache)` (`reading.rs:154,169,250,412`,
  `interact/scenes.rs:39,71,103,134`), `values(rd!(self), &mut self.cache, …)` (`query.rs:358`,
  `query/view.rs:25`). `rd!` 21 dosyada ≈ 99 kez. Plandaki `query(&self, host: &dyn Host, …)`
  önbelleği ısıtamaz.
- **Önem:** orta.
- **Öneri:** `HostMut::reading(&mut self) -> Reading<'_> { rd: &Document, cache: &mut Cache,
  headlines: &BTreeMap<…> }` demeti; `query` kancası `&mut dyn HostMut` alsın; `rd!`
  makrosu otorite crate'inde kalsın, alt sistemler demeti kullansın.

### 8. `Conn`/`Sub` alt sistem tiplerini taşıyor
- **Bulunan:** `authority/session.rs:83-96` `Sub.watch: Option<(u64, crate::live::CondInfo)>`,
  `Sub.stories: Option<crate::story::StoriesSent>`; `conns` 20 grupta okunuyor.
- **Önem:** orta (otorite → alt sistem tip kenarı).
- **Öneri:** abonelik başına alt sistem verisi alt sistemin durumunda yan tablo
  (`(conn, sub id)` anahtarlı); abonelik bitince otorite `unsubscribed` kancasını çağırır
  (bugün `viewer_unsubscribed`, `dispatch.rs:110`; aynı sırada).

### 9. `Deref` yeni tipi işler, ama yansı ve barındırıcı "dışa açık API" kullanmıyor
- **Bulunan:** `replica.rs:229-241` `Session::build_with` (crate içi), sonra doğrudan alan
  yazımı: `s.doc = …`, `s.log.clear()`, `s.plane = …`, `s.rd = …`, `s.cache.clear()`;
  `replica/follow.rs:130` `s.plane.provider` takası, `:184` `apply_in_place(&mut s.doc…)`,
  `:207` `s.committed(e)`, `:236,250` `s.data_changed(…)`; `judge.rs:83-175` `s.doc`,
  `s.plane`, `s.rd` okur. `host/*` `agent_origin` (crate içi), `crate::error::{E, ptr}`.
  `replica.rs:86` `CompileOptions::new` (tasarım enjeksiyonu buraya da dokunur).
- **Önem:** orta-yüksek (S2b "taşıma, düşük" olarak sınıflanmış; gerçekte yeniden yazma).
- **Öneri:** S2b, S3.0'dan **sonra** ve "yeniden yazma" olarak: otoriteye dar bir izleyici
  arayüzü (`follow_entry`, `follow_data`, `reset_from(doc, plane)`); `committed`'in alt
  sistem kancaları izleyicide de aynı sırayla çalışmalı. WASM yansısının şemasız kurulumu
  (`build_with(…, Null)`, yorum `session.rs:482-484`) korunmalı. `Deref` bunun dışında
  makul: `Session::new` (95 yer) yeni tipte yeniden tanımlanır, `&Session → &Authority`
  zorlaması çalışır; trait sınırlarında ve ilişkili fonksiyonlarda çalışmaz (bugün `Session`
  için trait uygulaması yok, risk düşük).

## C. Olay ve bildirim sırası

### 10. Kayıt döngüsü bugünkü iki farklı sırayı üretemez
- **Bulunan:**
  - `settle.rs:215-243`: izleyiciler girdiden **hemen sonra** hesaplanır (`eval_watchers`,
    `:216,221,228`) ama çıktıları **en sonda**; sıra: `entry_events`* → `invalidate_in_public`
    → `run_capsules` → `view_events` → koşullar → niyetler → `claim_events`.
  - `data/live/eval.rs:165-178` (`settle_live`): koşullar → `claim_events` **hesaplanır** →
    `intent_lines` → `eval_intents_public` → iddialar **yazılır**. İki yol farklı sıra.
  - `run.rs:157-162`: her motor girdisinde `entry_events` → kapsül konusu satırları →
    `watch_lines` (kapsül yaşamı ile izleme iç içe).
  - `dispatch.rs:49-55` `comment_events` yalnız `AddComment` varsa ve komuttan sonra;
    `:93-100` önce `ack`, sonra `viewer_start`, sonra `stories_start` (durum, satır yok);
    `intake.rs:111-113,167-169` her mesajdan sonra `viewer_flush`; `events.rs:303-318` kamu
    dünyası satırlarını birleştirip `parts[0]`'ı bağlantı sırasına göre sıralar.
  - Plan `on_clock`'a "kapsül koşusu, uzlaşma" diyor; `clock.rs:139-170` yalnız süresi dolan
    önerileri girer, sonra `settle`. Kapsül koşusu veri/girdi yolunda.
- **Önem:** yüksek (yanıt akışı baytı en kolay burada kayar).
- **Öneri:** genel "`for s in subs { s.after_commit() }`" yerine otoritede **adlı, sabit
  sıralı evreler**: `EntryEvents, PublicInvalidate, Capsules, Views, Conds, Claims, Intents,
  PublicIntents`; her evre "topla" ve "yaz" diye iki adım (hesap/yazım ayrımı korunur);
  `settle` ve `settle_live` evre listesini bugünkü sırayla açıkça yazar. Yeni alt sistem bir
  evreye sona eklenir (otorite değişmez). S1 karakterizasyonu artımlı yol açık/kapalı
  (`set_incremental`) iki kez ve kamu dünyası olan/olmayan iki fikstürle koşsun.

## D. Gramer enjeksiyonu

### 11. Çağrı yeri sayısı ve `CompileOptions` türetmeleri
- **Bulunan:** `CompileOptions` `#[derive(Debug, Clone, PartialEq)]` (`compile.rs:60`); `&dyn
  Designs` alanı `Debug`/`PartialEq` türetmesini bozar. `CompileOptions::new(w, h)`
  (`compile.rs:117`) ≈ 70 yerde; gramer `src` dışında 103 başvuru (57'si `qp-grammar/tests`).
  Plan ≈ 25 diyor ve çağıranlar listesinde `qp-wasm` varsayılan paket (`bind/render.rs:6`),
  `qp-mcp`, `qp-session` yansısı (`replica.rs:86`) yok.
- **Önem:** orta.
- **Öneri:** yalnız derleyicinin kendisinin çağırdıkları enjekte edilsin — 4 nokta:
  `scene.rs:46` (`headline_template`, hata sırası korunarak `outcome` ve `alphabet`
  denetiminden sonra), `prepare.rs:100,117` (`field_text`, `text`), `terms.rs:149,182`
  (`layer_term`, `voice_terms_in`). `headline_text` (`faces.rs:227-235`) saf; çekirdeğe iner.
  Taşıyıcı olarak `DataProvider` düşünülsün (her derlemede zaten `&dyn`; `device_check`
  orada, `provider.rs:150`) ya da `Option<&'static dyn CompileDesigns>` + elle `PartialEq`
  (`ptr::eq`) ve `Debug`. `depth.rs:370-414` sarmalayıcıları (`is_face`, `face_view*`,
  `story_design`) enjeksiyon değil: `qp-designs`'a taşınır, çağıranların yolu değişir.

### 12. `motion` çekirdekte kalmamalı; `MOTIONS` durağan tablosu tasarım tarafının
- **Bulunan:** `present/motion.rs:74-99` `static MOTIONS` `transitions::nabiz::*` fonksiyon
  işaretçileri ve `PRESENTED_TRANSITION` taşır; `motion/frame.rs:5` `surek::ramp`. Derleyici
  ve derinlik `motion`'ı hiç çağırmıyor (grep: `compile/`, `present/depth*`'te `motion::` yok).
- **Önem:** düşük (iş azalır).
- **Öneri:** `present/motion` bütünüyle `qp-designs`'a; `ramp` `detmath`'e. `MotionDesign`,
  `Frame`, `LiveMarks`'ı çekirdek sözlüğe indirmeye gerek yok.

### 13. Varsayılan WASM boyutu: geniş `Designs` vtable'ı bugün erişilmeyen kodu bağlar
- **Bulunan:** bugün derleyici `FACES`'e (`faces.rs:95`, bütün yüz görünüm işaretçileri) ve
  açılmış kapsül tasarımlarına erişir, `stories`/`transitions`/`motion`/`feeds`'e erişmez.
  `qp-wasm` varsayılan pakette `qp-grammar`'a doğrudan bağımlı (`Cargo.toml:28`, isteğe bağlı
  değil). `story`/`transition`/`motion` yöntemleri olan tek bir `Designs` vtable'ı onları
  varsayılan WASM'a sokar (vtable bütün yöntemleri canlı tutar). Tersi de ölçüt dışı: varsayılan
  paket tasarımsız (`None`) kurulursa küçülür; GOAL "±%2" iki yönlü.
- **Önem:** orta.
- **Öneri:** dar `CompileDesigns` (yalnız madde 11'deki 4–5 yöntem); G2b sonunda varsayılan
  WASM ölçülsün; küçülmenin ölçüt sayılıp sayılmadığı karar noktasında sorulsun.

### 14. `qp-capsule`/`qp-render` taşınan parçaları çağırmıyor (doğrulandı); ama şema tuzağı var
- **Bulunan:** `qp-render` yalnız `qp-schema`'ya bağımlı (testler `CompileOptions`
  kullanır); `qp-capsule`'de `qp_grammar` yalnız yorumlarda (`voice/registry.rs:10`,
  `feedrule.rs:60`). Ancak `qp-schema/src/protocol/input.rs:128-173` belge yorumları
  `qp_grammar::faces::FACES`, `open::OPENS`, `transitions::TRANSITIONS`, `stories::STORIES`,
  `feeds::LAYOUTS` adlarını taşır; schemars bunları `description`'a koyar ve tutanak
  `qp schema export`'u alır (`scripts/refactor/snapshot.sh:85`).
- **Önem:** orta (Q "eskimiş yorumlar" adımı şemayı oynatır).
- **Öneri:** `qp-schema` belge yorumlarına dokunulmaz kuralı plana yazılsın; eskiyen yollar
  "Fark edilenler"e.

## E. WASM, testler, görünürlük, küçükler

### 15. `src` içi testler alt crate'e taşınamaz
- **Bulunan:** `present/motion.rs:318-328` (`transitions::is_transition`), `look/tests.rs:261`
  (`open::tesir`), `compile/terms.rs` ve `present/depth.rs` testleri tasarımlara bakıyor;
  `ops/summary.rs:459` `Session`. Alt crate'e inince bu testler üst crate'i kullanamaz.
- **Önem:** düşük-orta (test adları tutanakta süzülüyor: taşınan testler `new-tests.txt`'e de
  girmeli, yoksa tutanak "eksik test" görür).
- **Öneri:** G2a/S2a için "taşınacak testler" listesi; `qp-designs/tests/` ve
  `qp-session/tests/`'e.

### 16. `pub(crate)` → `pub` genişlemesi plan dışında
- **Bulunan:** tasarımlar `draw::rect`, `draw::wrap` kullanır (`draw.rs:22,193`,
  `pub(crate)`; `lib.rs` `pub(crate) use compile::{draw, …}`). Oturumda `E`, `ptr`,
  `PResult`, `Ctx`, `Cache`, `DataPlane`, `reading_doc`, `apply_in_place`, `LiveIndex`, `Proj`,
  `committed`, `data_changed`, `agent_origin` crate sınırını geçecek.
- **Önem:** düşük (ENGINEERING §4 "`pub` varsayılan değildir").
- **Öneri:** `paths.md`'de "genişleyen görünürlük" bölümü; iç kullanım için `#[doc(hidden)]`.

### 17. Kavram adları ve yapılandırma
- **Bulunan:** `Host` adı "barındırıcı" kavramıyla, `Role::Host`, `qp-host` ve `host/`
  modülüyle çakışır; "alt sistem", "tasarım kaydı" `docs/CONCEPTS.md`'de yok.
  `arch-config.json:101` üretilmiş `crates/qp-grammar/src/advances.rs` yolu taşınınca
  `config-diff` kırmızı yanar.
- **Önem:** düşük.
- **Öneri:** trait adı başka (`AuthorityView`/`AuthorityMut`); kavramlar önce sözlüğe;
  `config-diff` yalnız yol değişen üretilmiş dosyayı karar satırısız geçirsin ya da karar satırı.

### 18. Başarım
- **Bulunan:** canlı tik yolu (`settle_live`) her tikte kancalar ve `Registry::get::<T>()`
  aşağı dönüşümlerinden geçecek; satır içine alma kaybolur.
- **Önem:** düşük. **Öneri:** S3.0'dan sonra K17/K52 bir kez ölçülsün (yalnız WASM değil).

## F. Geri dönüş

### 19. Kapsül yaşamı istisnası iyi tanımlı değil
- **Bulunan:** tetik yok ("tutmazsa"). Madde 3'e göre kapsül yaşamı otoritede kalırsa
  `claims`/`dispute`/`settled` ve `watch` de kalmak zorunda; madde 4–5'e göre `as_reader` ve
  iki dünya zaten otoritede. Hesap: otorite 5,6 + kapsül yaşamı 5,0 + iddia 2,1 + izleme 0,5
  ≈ **13,2 bin** > planın "≈ 13 bin" tavanı.
- **Önem:** orta.
- **Öneri:** (a) geri dönüş crate'inin dosya listesi şimdi yazılsın ve `graph.py` ile tavanı
  ölçülsün; (b) tetik somut olsun (ör. S3.4'te `compare.sh` farkının nedeni iki denemede
  bulunamazsa ya da `Host` arayüzü N yöntemi aşarsa); (c) karar noktasında Arkin'e önceden
  sorulsun. Gerçekçi bir A planı: otorite + kapsül yaşamı + iddia/itiraz + izleme tek
  crate (adlı istisna, ölçülmüş tavan); `qp-ops`, `qp-viewer` (otorite tarafı), `qp-read`,
  `qp-livepath`, `qp-replica`, `qp-replayhost` dışarıda. R7/RL1/RS1/RA1 için kayıt düzeneği
  yine kurulur; sonraki bir RF kapsül yaşamını ayırır.

## Doğrulananlar (sorun yok)

- `qp-voice`: `voice/` çekirdeği çağırmıyor, çekirdek `voice`'u çağırmıyor (yalnız `lib.rs:60`
  modül bildirimi); kullandığı `realize::*`, `sentence::*`, `pipeline::simulate_engine`,
  `outcome::base_params` hepsi `pub`. Saf taşıma yapılabilir.
- `qp-look`/`qp-glyph`: test dışı yukarı kenar yok (`look` → `open` yalnız `look/tests.rs`).
- Alan dağılımı (`conns` 20, `runner` 8) ve SCC (25.895 satır) ölçümle tutarlı.

## Hüküm

Gramer ve ses tarafı yapılabilir; gramer enjeksiyonu plandan **daha dar** tutulmalı (4–5
derleyici noktası; sarmalayıcılar ve `motion` taşınır). Oturum tarafında §4'teki iskelet
yazıldığı haliyle çalışmaz: `as_reader`'ın doküman takası, iki dünyanın alt sistem durumunu
takası, yeniden girme zinciri ve iki ayrı olay sırası bu arayüzle ifade edilemez; önerilen
sınırlar da döngülü. Karar noktasından önce: (1) önerilen bölüm `graph.py` ile ölçülsün,
(2) arayüz "durumsuz davranış + otoritenin tipli durum haritası + sabit adlı evreler" diye
yeniden çizilsin, (3) geri dönüş A planı olarak tanımlanıp tavanı ölçülsün.
