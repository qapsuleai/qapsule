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

**Yeniden yazılan öğeler (moved.py `CHANGED -`, 64; sınama yardımcıları dahil):**

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
