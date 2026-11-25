# RF1 — Yeniden yazılan öğeler

`PLAN.md` §0: taşınan öğe `moved.py` ile "değişmedi" çıkar; gövdesi ya da imzası değişen her
öğe burada sayılır.

## S2 — `qp-ops`, `qp-plane` (B7: sağlayıcının `&Session` bağı)

| Öğe | Ne değişti |
|---|---|
| `qp_ops::provider::DocProvider` (yapı) | `parts` alanı `Option<(&Session, Language)>` → `Option<(&dyn HeadlineSource, Language)>` (alan belgesi de) |
| `DocProvider::with_parts` | parametre `&Session` → `&dyn HeadlineSource` (gövde aynı; çağıranlar `with_parts(self, lang)` değişmedi) |
| `impl DataProvider for DocProvider` → `headline_parts` | `s.doc` / `s.proj` ile `reader_parts` çağrısı → `s.headline_parts(finding, lang)` |
| `impl DataProvider for DocProvider` → `voice` | `s.doc` / `s.proj` ile `reader_voice` çağrısı → `s.voice(finding, lang, v, self.clock)` |

Yeni öğeler (eski gövdeler buraya taşındı):

- `qp_ops::provider::HeadlineSource` (trait; iki yöntem).
- `impl qp_ops::HeadlineSource for Session` (`qp-session` `capsule/headsource.rs`): eski iki gövde
  aynen, `s` → `self`.

- `qp_plane::values::Computed::is_empty` (yeni, kullanılmıyor): `Computed` ve `len` crate
  sınırını geçince clippy `len_without_is_empty` istedi; modül/crate düzeyinde susturma yerine
  eklendi. `Computed` ve `impl`'i `values.rs`'ten `values/computed.rs`'e saf taşımayla gitti
  (`new` ve `main_ix` `pub(super)`), böylece `values.rs` taban çizgisi değerini aşmadı.

Toplam: 4 yeniden yazılan öğe, 3 yeni öğe. `moved.py`'nin bir CHANGED'ı daha var:
`qp-shellcore` `drive/viewer_less.rs` `fn deliver` — yalnız yol (`qp_session::headparts` →
`qp_plane::headparts`) ve rustfmt'in kısalan satırı yeniden sarması; gövde aynı.

## S4 — `expand_op` kol başına fonksiyon (`qp-ops` `expand/op.rs` + `expand/op/`)

`expand_op` bir dağıtıcı `match` olarak kaldı (`op.rs` 121 satır, fonksiyon 101 satır); 22 kol
kendi fonksiyonuna gitti. Her fonksiyonun gövdesi kolun gövdesidir: değerlendirme sırası, hata
iletileri ve erken dönüşler aynı; `return` kolda da `expand_op`'tan dönüyordu.

- **Kol fonksiyonları alanları parametre olarak alır.** Dağıtıcı alanları açar, örneğin
  `Op::PutSeries { series } => series::put_series(doc, series, p, ctx)`. Kalıpta
  `unreachable!` yoktur.
- **Yedi parametre sınırı:** alanı çok olan dört kol için `Arm { doc, p, ctx }` yapısı kullanılır
  (yeni öğe): `add_study`, `add_fact`, `add_finding`, `add_comment`.
- **`ptr_arg` izni:** alan bağları `&String` / `&Vec<_>` olduğundan, gövde değişmesin diye on
  kol fonksiyonuna öğe düzeyinde `#[allow(clippy::ptr_arg)]` kondu. Bu, `arch-check`'in saydığı
  beş lint'ten biri değil.
- **`ctx` ve `doc` almayanlar:** `ctx` kullanmayanlar onu almaz (`set_params`, `remove_layer`,
  `append_bars`, `update_last_bar`); `set_audience` `doc` almaz.
- **Satır içinde kalan kollar:** `add_watch`/`remove_watch`, `undo`, gramer işlemleri,
  `accept_proposal`.
- **Dağıtıcı 101 satır** (alanları açılmış 23 kol, `rustfmt` biçimi). Arkin'in kararıyla
  (`DECISION.md`, uygulama notları) `fn.lines` taban çizgisinde 101 satırla durur ve
  `too_many_lines` izni kalır (`lint.allow` `qp-ops` 21). `main`'e göre borç 903 → 101;
  `cognitive_complexity` izni kalktı.

| Yeni fonksiyon | Dosya |
|---|---|
| `put_series`, `put_series_source`, `add_study`, `set_params`, `append_bars`, `update_last_bar` | `op/series.rs` |
| `put_view`, `add_layer`, `remove_layer`, `set_view_range` | `op/view.rs` |
| `add_fact`, `add_finding`, `update_finding`, `decide_finding` (onay/ret), `add_narrative`, `show_finding` | `op/finding.rs` |
| `add_intent`, `set_intent_active`, `add_comment`, `remove`, `set_audience`, `set_sensitivity`, `reject_proposal` | `op/people.rs` |

- **`put_series` ikiye bölündü:** 80 satırı aştığı için kaynağa göre denetim (`match &series.source`,
  `let src` ile) `put_series_source(doc, series, &mut s, &sp, ctx)` adımına gitti. Orada `sp`
  `&str` olduğundan 6 yerde `&sp` → `sp` (clippy `needless_borrow`); `s.source = …` ataması
  `&mut` üstünden.
- **Sayım:** yeniden yazılan öğe `expand_op` (1); yeni öğeler 23 kol fonksiyonu ve `struct Arm`.
  `expand_op` dışında yeni fonksiyonların hiçbiri 80 satırı aşmıyor; dosyalar ≤ 337 satır.
- **İ6:** `op.rs` kalemi yapılandırmadan silindi.

## Not: crate düzeyi izin

`qp-ops` ve `qp-plane`'in `lib.rs`'inde `#![allow(clippy::result_large_err)]` var. Bu,
`qp-session`'da zaten bulunan iznin kopyası: protokol hatası soğuk yolda kutusuz dönüyor,
gerekçe `qp-session` `lib.rs`'teki yorum. `arch-check`'in `lint.inner` kuralı bu lint'e bakmıyor.
Taşınan kodla birlikte geldi; yeni bir susturma değil.

## S3.0 — alt sistem sözleşmesi ve otoritede çağrı noktaları

Boş kayıtla davranış aynı (S1 karakterizasyonu, `compare.sh`). Bütün çağrı noktaları "önce kayıt":
kayıtta karşılığı olan tür (işlem, sorgu, girdi, konu, koşul, iddia) onun işleyicisine gider; öbürü
bugünkü yoldan, bugünkü hatayla. Kapalı şema enum'larında `_` kolu yazılamaz (her kol dolu:
`unreachable_patterns`, `-D warnings`), bu yüzden "varsayılan kol" dağıtıcının girişinde kayda
bakıştır; `allowed`/`proposable`/`agent_op`/`ops_to_pub` gibi zaten `_` kolu olanlarda da aynı biçim
kullanıldı (tek anlam).

**Yeniden yazılan öğeler (moved.py `CHANGED -`, 37):**

| Öğe | Dosya | Ne değişti |
|---|---|---|
| `struct Session` | `authority/session.rs` | 4 alan: `registry`, `slots`, `outbox`, `wake` |
| `allowed`, `proposable` | `authority/session.rs` | `reg: &Registry` parametresi; kayıtlı işlemde işleyicinin cevabı |
| `op_name` ×2 | `authority/session.rs`, `authority/agent.rs` | silindi; tek kopya `qp_subsystem::op_name` (gövde aynı, belge yorumu yeni) |
| `agent_check` | `authority/agent.rs` | ajan sınıfı: kayıtlı işlemde `OpHandler::agent`, öbürü `agent_op` |
| `command` | `authority/session/command.rs` | `allowed`/`proposable` kayıtla; `expand_ops(&self.registry, …)` ×3; `check_comments` → `check_command` (yorum denetimi + `Check` fazı) |
| `pub_expand`, `direct_two_worlds`, `propose_two_worlds` | `authority/session/worlds.rs` | `expand_ops(&…registry, …)` |
| `accept_expand` | `authority/session/accept.rs` | `expand_ops(&self.registry, …)` |
| `committed` | `authority/session/accept.rs` | kitle döngüsü `note_audiences`'e ayrıldı (yeni öğe, gövde aynen); sonuna `AfterCommit` |
| `build_with_keys` | `authority/session/create.rs` | 4 alan kurulumu; `audience_seen` kurulumu `initial_audiences`'e ayrıldı (yeni öğe, ifade aynen) |
| `handle` | `authority/session/dispatch.rs` | `handle_resolved` → `handle_msg` (ileti sonu yazma adımı) |
| `handle_resolved` | `authority/session/dispatch.rs` | `query`/`subscribe`/`input`/`dispute`/`hello` → `dispatch_*`; `viewer_unsubscribed` → `unsubscribed` |
| `handle_raw`, `resolve_selectors` | `authority/session/intake.rs` | `handle_msg`; `registry.expand_op` |
| `subscribe` | `authority/session/presence.rs` | `holds` → `registry.holds` |
| `advance_clock` | `authority/session/clock.rs` | `Proposals` fazı (toplama) ve yazma adımı |
| `data_changed` | `authority/session/settle.rs` | `AfterData` fazı (toplama), `pending_views`'den önce |
| `settle`, `settle_data` | `authority/session/settle.rs` | sonda yazma adımı (`emit`) |
| `eval_watchers`, `eval_intents`, `invalidate_these` | `authority/session/events.rs` | `eval_condition`/`holds` → `registry.…` |
| `settle_live` | `data/live/eval.rs` | sonda yazma adımı |
| `bar_command` | `data/live/eval.rs` | `registry.expand_op` |
| `eval_conditions_live`, `eval_intents_live`, `invalidate_live` | `data/live/eval.rs` | `eval_memo(…, &self.registry)` |
| `eval_memo` | `data/live.rs` | `reg` parametresi; kimlikle değerlendirme `reg.eval_condition` |
| `swap_world` | `capsule/public.rs` | `self.slots.swap_world();` (tek satır) |
| `ops_to_pub` | `capsule/public.rs` | `other` kolu: kayıtlı işlemde `OpHandler::to_public` |
| `invalidate_in_public`, `eval_intents_public` | `capsule/public/commit.rs` | `registry.holds` / `registry.eval_condition` |
| `project_entry` | `capsule/project.rs` | okuyucu gizliliği: kayıtlı işlemde `OpHandler::reader_private` |
| `settled_bars` | `capsule/dispute.rs` | `capsule_settled` → `settled_phase` (+ `Settled` fazı) |

Görünürlük yalnız (öğe aynı): `hello` (`pub(super)`), `verdict_line` (`pub(crate)`),
`Computed::new` (`pub`).

**Yeni öğeler:** `qp-subsystem` (bütün crate; ≈ 0,8 bin satır), `authority/session/phases.rs`
(`impl SessionView for Session`, `expand_ops`, `register`, `next_wake`, `with_ctx`, `phase`, `emit`,
`handle_msg`, `check_command`, `dispatch_hello`, `dispatch_query`, `dispatch_input`,
`dispatch_subscribe`, `unsubscribed`, `dispatch_dispute`, `settled_phase`, `host_adjust`),
`note_audiences`, `initial_audiences`, `crates/qp-session/src/lib.rs` (yüz).

**Saf taşıma:** `qp-session/src` → `qp-authority/src`; `host/conform.rs` → `qp-session/src/conform.rs`
(yalnız `use` yolları); `confine`, `read_json` → `authority/session/files.rs`; `AgentOp` →
`qp-subsystem`.

## S3.1 — `qp-watch` (ilk alt sistem), bileşimin kayıt yayılımı, audit-s30 düzeltmeleri

Davranış aynı (S1 karakterizasyonu 25/25, kitaplar aynı; `compare.sh`). İzleme çekirdekten
`qp-watch` alt sistemine çıktı: işlemler kayıt üstünden (`OpHandler`), eşleme `AfterData` fazında
(`Moment::Look` → aynı motor girdisine değişim; `Moment::Written` → `intent_matched` o satırda).
Bugünkü sıra korunur: bakışın değişimleri kapsül değişimlerinin hemen ardından (önce otoritenin
dünyası, sonra kamu dünyası), olaylar `capsule_topic_lines`'tan hemen sonra.

**Kayıt yayılımı (tasarım):** otorite alt sistem tanımaz; kurucuları alt sistem listesini
(`qp_authority::Subsystems`, `&'static [&'static dyn Subsystem]`) parametre alır ve kurulumda,
yükleme koşmadan önce kaydeder (`create.rs` `load`). Doğurduğu her otorite listeyi ondan alır:
`ReplayHost::rebuild` (`self.session.subsystems()`), `Replica` (alan `subs` → izleyici),
`OpenSide` (yansısına), `tape` (iki otoriteye ve açık tarafa). Ebeveynsiz giriş noktaları
(`Session`, `ReplayHost`, `Replica`, `OpenSide`, `tape::run*`, `agent::agent_op`) bileşimde aynı
adlı sarıcılardır ve `STANDARD`'ı verir. Yansı/barındırıcıyı yukarı taşımak (S3.3'ün işi) seçilmedi:
iç alanlara yazdıkları için yeniden yazma ister; liste parametresi otoriteyi alt sistemsiz bırakıp
aynı sonucu verir.

**Yeniden yazılan öğeler (moved.py `main HEAD`: `CHANGED -` 71, taşınan 565; sınama yardımcıları dahil; tek FLAG `tests/subsystems.rs` iç `#![allow]`ı — dosyanın eski satırı, `use` değişti):**

| Öğe | Dosya | Ne değişti |
|---|---|---|
| `match_watches`, `watch_lines` | `qp-authority` `capsule/watch.rs` → `qp-watch` `matching.rs` | `&self` yerine `(doc, clock, events)` / `&dyn SessionView`; `expect("on")` → `let … else continue`; `self.conns` → `view.conn_ids()`/`subs()`/`conn()`; `self.other.is_some() && public_view` → `view.public_view` (aynı anlam: `public_view` ikinci dünya yoksa `false`); `out(…)` → `OutboundLine {…}` |
| `intent_for_reader` | `capsule/watch.rs` → `capsule/project.rs` | yer; belge yorumu (gövde aynı) |
| `expand_watch` | `qp-ops` `watch.rs` → `qp-watch` `ops.rs` | `expand_watch` (dağıtıcı) + `add_watch` + `remove_watch`; `person_of_speaker` → `person_of`; `unreachable!` → `invalid` hatası; denetim sırası aynı |
| `run_capsules` | `capsule/run.rs` | `match_watches` ×2 → `look_phase(&events, false/true)`; `watch_lines` → `written_phase(seq)` |
| `allowed`, `proposable` | `authority/session.rs` | `AddWatch`/`RemoveWatch` kolları silindi (kayıttaki işleyici cevaplar) |
| `agent_op` | `authority/agent.rs` | `AddWatch`/`RemoveWatch` kolu silindi (`qp_session::agent::agent_op` bileşimde) |
| `project_entry` | `capsule/project.rs` | `matches!(AddWatch \| RemoveWatch)` silindi; yalnız `OpHandler::reader_private` |
| `expand_op` | `qp-ops` `expand/op.rs` | izleme kolu → `unserved(op, p)` (`unsupported` hatası; kayıtsızken) |
| `expand_ops` | `qp-ops` `expand.rs` | gövde `expand_ops_with`'e (B6); `expand_ops` = `expand_ops_with(…, expand_op)` |
| `Registry::expand_ops` | `qp-subsystem` `registry.rs` | kopya gövde silindi → `expand_ops_with` (B6) |
| `Registry::op` | `qp-subsystem` `registry.rs` | `op_name` serileştirmesi yerine `OpHandler::owns` |
| `struct Registry`, `Registry::{add, query, input, topic, claim, eval_condition}`, `claim` | `qp-subsystem` `registry.rs` | tablolar `BTreeMap` yerine kayıt sıralı vektör (`Table<V>`, `find`): kabuk paketinde tablo başına `BTreeMap` örneklemesi ≈ 15 KB tutuyordu (WASM ±%2 kapısı); arama sonucu aynı (her etiket bir kez) |
| `OpHandler`, `SessionView`, `Phase`, `Moment`, `Outbox` (+`is_empty`, `drain`) | `qp-subsystem` | `owns`; `conn_ids`, `subs`, `public_view`, `reader_proj`; belge (`AfterData`, `Close`, `Command`); `Look`, `Written`; `changes`/`lines` |
| `emit` | `authority/session/phases.rs` | derinlik sınırı `MAX_EMIT_DEPTH` (B3) |
| `dispatch_dispute` | `authority/session/phases.rs` | kayıtlı iddia türünde yerleşik korumalar: `dispute_guard` (B5) |
| `dispatch_subscribe` | `authority/session/phases.rs` | ret sonrası `viewer_unsubscribed` (B5) |
| `advance_clock` | `authority/session/clock.rs` | ulaşılan uyandırma silinir (B2) |
| `eval_memo` | `data/live.rs` | kayıtlı koşul/kaynak varken bellek atlanır (B1) |
| `struct Session`, `build_with_keys` | `authority/session.rs`, `create.rs` | alanlar `subs`, `emit_depth`; yükleme kuyruğu `load`'a (kayıt önce) |
| `new`, `new_with_trust_keys`, `new_inner`, `with_provider`, `with_provider_and_trust_keys`, `build_keys`, `build_with` (`build` silindi) | `authority/session.rs` | `subs` parametresi |
| `ReplayHost::{new, with_local_dir, with_window, live}`, `rebuild` | `host/replayhost/setup.rs`, `control.rs` | `subs`; yeniden kurulum listeyi eski oturumdan alır |
| `struct Replica`, `Replica::{new, with_memory, with}`, `follower`, `load` | `replica.rs`, `replica/follow.rs` | `subs` alanı ve parametresi |
| `OpenSide::new` | `replica/viewer/open.rs` | `subs` |
| `tape::{run, run_with, run_viewer}`, `viewer_deliver` | `replica/tape.rs` | `subs`; `OpenSide` `use` ile |
| `Ctx::{prov, new_id}` | `qp-ops` `expand.rs` | yalnız görünürlük ve belge |
| `Hub::{session, session_mut}` | `qp-mcp` `hub.rs` | dönüş tipi `&qp_session::Authority` |
| `trust` | `qp-shellcore` `viewer.rs` | `.map(OpenSide::trust)` → `.map(\|o\| o.trust())` |
| sınama yardımcıları: `host` ×2, `fast_hints_equal_the_resolvers`, `session`, `the_settled_phase_follows_a_settled_answer`, `Rig::{session, session_mut}`, `open_scene_in` | `qp-authority` `capsule/tests`, `read/tests.rs`, `tests/subsystems.rs`; `qp-session` `tests/charac/rig.rs`, `tests/open_nobra.rs` | `Subsystems::NONE` / `&Authority` (kitaplar aynı) |

**Yeni öğeler:** `qp-watch` (crate: `Watch`, `WATCH`, `Pending`, `WatchOp`, `ADD_WATCH`,
`REMOVE_WATCH`, `OPS`; `WatchOp::to_public` yalnız izleme kollarını alan alan kopyalar: varsayılan `op.clone()` bütün `Op` klonunu kabuk paketine ≈ 18 KB katıyordu); `qp-session` `compose.rs` (`STANDARD`, `LIST`, `wraps!`, `Session`,
`ReplayHost`, `Replica`, `OpenSide`), `agent.rs` (`agent_op`), `tape.rs` (`run`, `run_with`,
`run_viewer`), `replayhost.rs`, `replica.rs`, `viewer.rs` (yeniden dışa aktarım);
`qp_authority::Subsystems`, `Session::{subsystems, slots, slots_mut, look_phase, written_phase,
dispute_guard, load}`, `impl SessionView` yeni yöntemleri; `qp_ops::expand::expand_ops_with`;
`Registry::evaluates_conditions`, `MAX_EMIT_DEPTH`, `Outbox::{amend, say, take_changes,
take_lines}`. Sınamalar: `qp-authority/tests/contract_fixes.rs` (B2, B3, B5),
`data/live.rs` `memo_registry` (B1).

**Saf taşıma:** `Matched`, `negative`, `holds`, `matches` (`capsule/watch.rs` → `qp-watch`
`matching.rs`); `max_watches_binds_registry` sınaması (`qp-watch` `lib.rs`).

## S3.2 — `qp-read` (okuma ve sunum), otoritede okuma kancaları

Davranış aynı (S1 karakterizasyonu 25/25, kitaplar aynı; `compare.sh` 270/270). Sorgular ve girdi
hareketleri çekirdekten `qp-read` alt sistemine çıktı. Otorite onları bugünkü çağrı noktasında
(`dispatch_query` / `dispatch_input`, `as_reader` içinde) kayıt üstünden çağırır: `qp-read` sorgu ve
girdi tablolarına `ANY` etiketiyle kaydolur (başka alt sistemin kaydetmediği her tür; Tally gibi
bir türü kaydeden alt sistem önce gelir). Okuma kodu `impl Session` yerine `impl Rd<'_>` üstünde:
`Rd` bölünmüş ödünçtür (critique-1 B9) — doküman, okuma dokümanı (`rd!` → `self.rdoc`), günlük,
düzlem, başlıklar, kimlikler, saat, izdüşüm görünümden; değer önbelleği ve bağlantıların görünüm
durumu (`look`) çağrı bağlamından (`SessionCtx::{cache, looks}`; otorite çağrı boyunca dışarı
alır, sonra geri koyar: yuva ve giden kutusu gibi); görünüm aralıkları, son görünüm, son girdi,
odak ve girdi sayacı alt sistemin yuvasından (`ReadState`). Kapsül yaşamı (kapsül öğesi, cümle,
ses, piyasa notu, süzgeçler, `stories`, `capsule` ayrıntısı, sicil) otoritede kalır ve
`SessionView` üstünden sorulur.

**Yeniden yazılan öğeler (moved.py `main HEAD`: `CHANGED -` 82, `CHANGED +` 209, taşınan 435,
yalnız yol 2; FLAG 5: `qp-read` `reading.rs`'in `Conn`/`without_layers` içe aktarımları yeni
yerlerine, `qp-read` `lib.rs` iç `#![allow(clippy::result_large_err)]`ı — otoritenin aynısı):**

| Öğe | Dosya | Ne değişti |
|---|---|---|
| okuma yöntemlerinin tümü (`query`, `need_principal`, `only_live_finding`, `at_bar`, `future_times`, `initial_range`, `last_slot`, `range_of`, `conn_viewport`, `screen_info`, `input`, `interact`, `resolve_target`, `find_capsule`, `depth_of`, `present_at`, `tap_at`, `story_tap`, `move_chart`, `depth_view`, `reader_scene`, `capsule_scene`, `story_extend`, `chart_range`, `side_scene`, `transition`, `set_style`, `set_frame`, `frame`, `view_of`, `hit_range`, `hit`, `screen_panels`, `blocked_layers`, `compiled_panels`, `warm_values`, `preview_doc`, `preview`, `gate_read`, `bindings`, `verify_evidence`, `check_fact`, `capsules_query`, `ledger_query`) | `qp-authority` `read/**`, `data/bindq.rs`, `capsule/queries.rs` → `qp-read` | `impl Session` → `impl Rd<'_>` (gövde aynı); `self.conns[ci].look` → `self.looks[ci]`; `self.input_counter`/`self.focus` → `self.st.…`; `.with_parts(self, …)` → `.with_parts(self.v.headline_source(), …)`; `crate::viewer::look_of` → `crate::look_of`; `crate::claims::for_principal(…)` → `self.v.evidence_for(…)`; `self.headline_cal`/`self.item_memo` → `self.v.…()`; `self.directives.get` → `self.v.directives`; `principals.clone()` → `to_vec()`; `permissions(role)` → bağlantının `permissions`'ı; `screen_info` `c.look` → `self.look_at(c)`; `frame` `c.look` → `l`; `preview_doc` `accept_expand` → `self.v.accept_preview`; `capsule_query` (dönüşte `transition`) → görünümün `capsule_query`'si (bağlantının sesi ve saati ile) |
| `bindings` | `qp-read` `bindq.rs` | veri konumu `FeedStore::position` (`&mut`, önek özeti belleği) yerine `feed()`, `bars()`, `seq()`, `data_digest(bars)` ile (aynı değer: `digest_u64` = bütün çubukların `data_digest_u64`'ü); düzlem görünümden salt okunur |
| `capsule_query`, `capsule_query_with` | `capsule/queries.rs` → `qp-authority` `capsule/detail.rs` | çekirdekte kaldı (yorum doğrulayıcısı aynı ayrıntıyı okur, okuma alt sistemi görünümle sorar); ses ve saat `(voice, clock)` parametresi (okuma çağrısı sırasında `looks` alt sistemde); doğrulayıcı `(None, false)` verir (yalnız öğenin `voice`'unu etkiler, doğrulayıcı okumaz) |
| `withheld_for`, `evidence_access` | `capsule/queries.rs` → `capsule/run.rs`, `capsule/detail.rs` | yer (gövde aynı) |
| `ledger_now`, `ledger_rows` (+ yeni `ledger_rows_of`) | `capsule/queries.rs` → `capsule/run.rs`; `capsule.rs` | otoritede kaldı (motoru okur); motor dalı `Document { … }` değişmezi kurmadan `ledger_rows_of(&[], &engine.ledger())` (boş kapsül listesi: aynı satırlar); `ledger_rows(doc)` = `ledger_rows_of(&doc.capsules, &doc.ledger.entries)` (D2: otoritede bir doküman değişmezi az) |
| `scene_for`, `explain` (bileşim), `capsule_query_with` | `qp-session` `compose.rs` | `#[allow(clippy::result_large_err)]` (otoritenin hata tipi) |
| needless_borrow düzeltmeleri, 7 dosya (`bindq.rs`, `interact.rs`, `interact/scenes.rs`, `preview.rs`, `query.rs`, `query/view.rs`, `reading.rs`) | `qp-read` | `&self.doc`, `&mut self.cache`, `&self.headlines` … → `self.doc`, `self.cache`, … (`Rd`'nin alanları zaten başvuru; aynı değer). Crate düzeyi `#![allow(clippy::needless_borrow)]` kaldırıldı (audit-s32 B4) |
| `tape::{run, run_with, run_viewer}` | `qp-authority` `replica/tape.rs`, `qp-session` `tape.rs` | liste parametresi `Lists` (`subs.0`, `subs.1`): otorite `STANDARD`, yansı ve açık taraf `FOLLOWER` (ürün yolu; audit-s32 B2); `Replica::session_mut`, `OpenSide::replica_mut` belgesi (B3) |
| `scene_as_reader`, `preview_doc` (çekirdek kopyası), `gated_view` | `read/preview.rs` → `qp-authority` `read.rs`; `gated_view` → `qp_ops::bounds` | barındırıcının derlemesi çekirdekte; `gated_view` serbest fonksiyon (`plane` parametresi) |
| `provider`, `scene`, `grammar_error`, `surface_kind` | `read/reading.rs`, `read/interact.rs` → `qp-authority` `read.rs` | yer (gövde aynı) |
| `scene_for`, `explain` | `qp-authority` → `qp-session` `compose.rs` | bileşimin yöntemleri: `scene_for` = `qp_read::scene_for_range` (görünüm, aralık) + otoritenin `scene`'i; `explain` = otoritenin `scene`'i + `explain_scene` |
| `check_target_pub`, `resolve_range`, `without_layers` | `read/query.rs`, `read/reading.rs` → `qp_ops::bounds` | yer ve görünürlük (`pub`) |
| `first_facts`, `FirstFacts`, `FnvHasher`, `Fnv` (+ sınaması) | `replica/viewer.rs` → `qp_ops::firsts` | yer; eski yolda `pub use` |
| `dispatch_query`, `dispatch_input` | `authority/session/phases.rs` | yerleşik yol yerine kayıt; kayıtsızsa `unsupported` (`unserved`) |
| `with_ctx` | `phases.rs` | önbellek ve `looks` da çağrı süresince dışarı alınıp geri konur |
| `impl SessionView for Session`, yeni `impl ReadView for Session` | `phases.rs` | `ConnView::{hello, profile}`; okuma eki 27 yöntem ayrı trait `ReadView`'da (`paths.md` S3.2) — her biri otoritenin aynı adlı fonksiyonunu çağırır. Ayrı trait, çünkü `SessionView` tablosu her faz çağrısında kurulur ve yöntemlerinin arkasındaki kodu (öneri açılımı `accept_expand` → `expand_op`, `stories`, kapsül ayrıntısı) kabuk paketine çeker (+≈ 250 KB); `ReadView` yalnız `dispatch_query`/`dispatch_input`/`with_subsystems`'te verilir |
| `with_ctx` → `with_ctx`, `with_read_ctx`, `with_ctx_as::<READ>` | `phases.rs` | `READ` sabit: faz çağrısı okuma tablosunu adlandırmaz |
| `Replica::{new, with_memory}`, `OpenSide::new` (bileşim) | `qp-session` `compose.rs` | `STANDARD` yerine `FOLLOWER` (`STANDARD` eksi okuma). İzleyici otoriteye bağlantı konuşmaz, sorgu ya da girdi almaz (`handle_raw` çağrısı yok: `replica.rs`, `replica/*`); okuma alt sistemi orada hiç çağrılmazdı. Kaydı, sorgu işleyicisinin tablosu üstünden bütün okuma kodunu (+ `schemars` şemaları) kabuk paketine çekiyordu (+1,38 MB) |
| `struct Conn`, `struct Session`, `hello`, `build_with_keys` | `authority/session.rs`, `dispatch.rs`, `create.rs` | `viewports`, `current`, `last_input`, `last_at`, `has_screen`, `focus`, `input_counter` → `qp-read` `ReadState`/`rd::Conn`; `look` → `Session::looks` |
| `capsule_topic_lines_in`, `viewer_update` | `capsule/project.rs`, `replica/viewer/updates.rs` | `c.look` → `self.looks[ci]` |
| `viewer_compute` | `replica/viewer/updates.rs` | `self.query` → `self.dispatch_query` (aynı `capsules` cevabı) |
| `seen` | `capsule/comments.rs` | `capsule_query_with(…, (None, false))` |
| `project_changes` | `capsule/project.rs` | yalnız belge yorumu (audit-s1 d2: savunma kolu) |
| `presence.rs` `attend` yolu, `story.rs` `stories_query` | `authority/session/presence.rs`, `capsule/story.rs` | `crate::query::…` → `qp_ops::bounds::…` (yol) |
| `incremental_items_equal_from_scratch` | `qp-authority` `capsule/tests/item_memo.rs` → `qp-session` `tests/capsule_perf_equality/item_memo.rs` | bileşimle kurulur (`Subsystems::NONE` yerine `STANDARD`); bellek sayısı `item_memo_counts()` |

**Yeni öğeler:** `qp-read` (crate: `Read`, `READ`, `Queries`, `Inputs`, `ReadState`, `ConnRead`,
`rd::Conn`, `Rd` ve görünüme giden ince sarmalayıcıları, `reading`, `scene_for_range`, `look_at`,
`keep`, `look_of`); `qp_subsystem::ANY`, `find_kind`; `SessionCtx::{cache, looks}`;
`ConnView::{hello, profile}`; `SessionView` okuma ve kapsül yaşamı yöntemleri;
`qp_authority::Session::{with_subsystems, reader_item_series}`, `looks`; `qp_ops::{bounds, firsts}`;
`qp_session::Session::{scene_for, explain}`. Sınamalar: `qp-subsystem` `tests/contract.rs`
`any_answers_every_kind_no_other_handler_took`; `qp-session` `compose.rs` `the_follower_list_is_the_standard_list_without_reading` (audit-s32 B2); `qp-authority` `tests/subsystems.rs`
`without_the_reading_subsystem_queries_and_inputs_are_unsupported`.

## S3.3 — `qp-replayhost`, `qp-viewer` (alt sistem), `qp-replica`; otoritede itilen konular ve izleyici adımları

Davranış aynı (S1 karakterizasyonu 25/25; `compare.sh`, doğrulama 15). Yeniden oynatma
barındırıcısı ve yansı otoritenin **istemcisi** oldu: otoriteyi kendilerine verilen listeyle kurar,
ona yalnız dışa açık ve dar gizli yöntemlerle ulaşır (`paths.md` S3.3). Görüntüleyici bir **alt
sistem** oldu: otorite onu itilen konu olarak (`TopicHandler`) her girdiden sonra, bugünkü çağrı
noktalarında (`handle_raw`, `host_data_event`, `set_market_status`, barındırıcının adımları) ve
okuyucunun dünyasında (`as_reader` + okuma görünümü) çağırır; görüntüleyici durumu bağlantı başına
yuvadadır. Toplu itme bayrağı (`viewer_batch`) otoritede kaldı: `stories` (§23, çekirdek) de ona
bağlı. `FOLLOWER` görüntüleyiciyi almaz (izleyiciye abonelik gelmez); WASM'ı ölçüldü, İ8.

**moved.py `main HEAD`:** 580 öğe değişmeden taşındı ya da yerinde; 20 yalnız yol; 154 `CHANGED`
(53 `-`, 101 `+`); 27 `FLAG` (içe aktarma); 37 yeniden dışa aktarım değişikliği.

| Öğe | Dosya | Ne değişti |
|---|---|---|
| `follower` | `qp-replica` `replica.rs` | alan yazımları (eski 229-241) → `Session::build_follower` + `follow_start` (gövde aynı, otoritede) |
| `load`, `mirror`, `data_event`, `resync` | `qp-replica` `replica/follow.rs` | `plane.provider` değişimi → `take_provider`; `apply_in_place`/`Restore` → `mirror_apply`; `committed` + `pending_views.clear()` + `claims.take()` → `mirror_commit`; `apply_event` + `data_changed` + temizlik → `follow_data`; yeniden çözüm → `follow_resync`; `s.doc` → `document()` |
| `judge`, `send_dispute`, `bars`, `candidates` | `qp-replica` `replica/judge.rs`, `replica.rs` | `rd!(s)`, `s.doc`, `s.plane`, `&mut s.cache` → `eval_parts()` / `SessionView::{reading, document}`; `claims::{anchor_of, closed, eval_at}` gizli kök yolundan |
| `new`, `with_memory`, `with` | `qp-replica` `replica.rs` | `crate::Subsystems` → `qp_authority::Subsystems` (yol) |
| `fnv1a64`, `document_digest` | `replica.rs` → `qp_ops::digest` | yer (çekirdeğin `story.rs`'i de kullanıyor) |
| `tape::run_viewer`, `ViewerOutcome`, `viewer_deliver` | `replica/tape.rs` → `qp-viewer` `tape.rs` | yer; `holds` `pub` |
| `ReplayHost::{control_msg, as_replay}` | `qp-replayhost` `replayhost/control.rs` | `session.conns` → `SessionView::conn`; `agent_origin(ci)` → `conn_agent_origin(conn)`; `seq_to` gizli `pub` |
| `reader_parts_by_language_and_audience` (+ `host`, `all_parts`, `example`) | `capsule/tests/headparts.rs` → `qp-replayhost` `tests/headparts.rs` | iç alanlar yerine `document()` ve gizli `reader_parts_of` (`as_reader` içinde başlık kaynağının `headline_parts`'ı = eski çağrı) |
| `viewer_check`, `viewer_start`, `viewer_unsubscribed` | `replica/viewer/updates.rs` → `qp-viewer` | `ViewerTopic::subscribe` (denetim + yuva; hatalar ve sırası aynı: yerleşik abonelikten sonra, ret aboneliği geri alır); ilk olay onaydan sonra `push(all)`; düşen abonelik `Unsubscribe` fazında |
| `viewer_flush`, `has_viewers`, `has_pushers`, `viewer_data`, `set_viewer_batch` | otorite `authority/session/push.rs` | genel: konu işleyicilerine `pushing`/`pushes`/`data` sorulur; bağlantı sırası aynı |
| `viewer_lines`, `viewer_update`, `viewer_compute`, `viewer_row` | `qp-viewer` `viewer/updates.rs` | `impl Session` → çağrı bağlamı; `dispatch_query` → `ctx.registry.query(&q)` + `answer` (aynı `capsules` cevabı); `self.doc`/`plane`/`looks`/`conns[ci]` → `ctx.view`/`ctx.looks`/`ConnView`; `market_for` → `ReadView`; satır ve `seq_to` otoritede (`push_lines`, ölçüm öncesi aynı dünyada); `Push` yapısı (parametre demeti) |
| `visible_msg` | `qp-viewer` `ViewerTopic::{visible, visible_result}` + otorite `dispatch_visible` | kayıt okuyucunun dünyası dışında, yanıt içinde (eskisi gibi). Görüntüleyicisiz otoritede `visible` artık `unsupported` (eskiden `not_found` "no `viewer` subscription"); bileşimde çıktı aynı (görüntüleyici kayıtlı; tutanak 270/270) — fark **yalnız standart dışı listede gözlenir** |
| `viewer_claim_sub` | `qp-viewer` `ViewerTopic::claim_sub`; otorite `push_claim_sub` | yuvadan |
| `face_scene` | `qp-viewer` `viewer/open.rs` | `face_with` (ortak gövde) + `wants_parts`; alt sistem `face_scene_in` (bağlamın okuma dokümanı, önbelleği ve okuyucunun başlıklarıyla `Session::provider`'ın kurduğu sağlayıcı; başlık parçaları `ReadView`'dan). Dışa açık imza aynı |
| `OpenSide::{new, update}` | `qp-viewer` | `start((*open.document).clone())` → `start_from(&open.document)` (WASM: tek klon örneği) |
| `dispatch_subscribe`, `unsubscribed` | `phases.rs` → `push.rs` | konu işleyicisini döndürür; `viewer_unsubscribed` çağrıları kalktı |
| `handle_resolved` (`Subscribe`, `Visible` kolları), `subscribe` (`Viewer` kolu), `claim_events`, `build_with_keys` (`viewers` alanı) | otorite | yukarıdaki genel düzenek |
| `with_ctx_as` | `phases.rs` | `SessionCtx::registry` |
| `impl ReadView for Session` | `phases.rs` | `item_headline_parts`, `market_for` (otoritenin aynı adlı fonksiyonları) |
| `the_follower_list_is_the_standard_list_without_reading` → `…_without_reading_and_viewers` | `qp-session` `compose.rs` | `FOLLOWER` = `STANDARD` eksi okuma ve görüntüleyici |
| `feeds_of`, `closed`, `anchor_of`, `eval_at` | `capsule/claims.rs` | `pub` (gizli kök yeniden dışa aktarımı) |

**Yeni öğeler:** `qp-replayhost`, `qp-viewer` (`Viewer`, `VIEWER`, `ViewerTopic`, `Push`,
`feed_items`, `check`, `face_scene_in`, `face_with`, `wants_parts`), `qp-replica` (crate);
otoritede `authority/session/push.rs`, `authority/session/follow.rs`, `reader_parts_of`,
`conn_agent_origin`; `qp_ops::digest`; `qp-subsystem` ekleri (`paths.md` S3.3).
