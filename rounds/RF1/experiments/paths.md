# RF1 — Dışa açık Rust yolu değişiklikleri

Protokol, şema, komut satırı ve MCP yüzeyi değişmedi. Bu dosya yalnız Rust crate yollarını
listeler; her birleştirmede güncellenir. Kaynaklar: ajan raporları ve denetim raporları
(`audit-v.md`, `audit-g2.md`).

## V — `qp-voice` (`6447b5c0`)

| Eski | Yeni |
|---|---|
| `qp_capsule::voice` ve bütün alt yolları (`speak`, `Input`, `Surfaces`, `Gloss`, `text_face`, `unnumber`, `VOICES`, `DEFAULT_VOICE`, `PRESENTED_VOICE`, `is_voice`, `names`, `marla`, `selvi`, `marla_glossary`, `selvi_glossary`) | `qp_voice::…` |
| `qp_session::voices` | adı aynı; artık `pub use qp_voice as voices` |

## G2 — gramerin bölünmesi

| Eski | Yeni |
|---|---|
| `qp_grammar::{look, theme, error, detmath, advances}` | `qp_look::…` |
| `qp_grammar::{GrammarError, ErrorKind, Theme}` | `qp_look::…` |
| `qp_grammar::glyph::X` (`outcome`, `rozet`, `siluet` alt modül) | `qp_glyph::X` |
| `qp_grammar::open::tesir::NO_LONGER_HEADLINE` | `qp_look::look::NO_LONGER_HEADLINE` |
| `qp_grammar::faces::{HeadlineFn, headline_text}`, `qp_grammar::open::DeviceCheck` | `qp_grammar::designs::…` |
| `qp_grammar::{faces, open, stories, feeds, transitions, motion}` | `qp_designs::…` |
| `qp_grammar::depth::{is_face, face_view, face_view_in, story_design}` | `qp_designs::depth::…` |
| `qp_grammar::CompileOptions::new(w, h)` | `qp_designs::options(w, h)` (varsayılan tasarım kaydıyla); tasarımsız: `CompileOptions::without_designs(w, h)` |

**Yeni yollar:**

- `qp_designs::DESIGNS`
- `qp_grammar::Designs`
- `qp_grammar::designs::NONE`
- `CompileOptions::with_designs`
- `CompileOptions::without_designs`

**`pub(crate)` → `pub` (başka crate'ten kullanıldıkları için):**

- `qp_grammar::draw` modülü ve içinden `rect`, `wrap`
- `qp_grammar::depth::{finding_and_fact, bar_series_bars, main_field, layer, time_scale, has_story, named_panel, panel_at, uniq}`
- `qp_grammar::GrammarError::{new, in_layer, with_layer}` (şimdi `qp_look::GrammarError`)

## S2 — `qp-ops`, `qp-plane` (oturumun saf katmanları aşağı)

Çapraz crate yeniden dışa aktarma yok; çağıranlar yol düzeltmesiyle yeni crate'i kullanır.

| Eski | Yeni |
|---|---|
| `qp_session::{replay, apply_all, apply_change, find_collection, keys_overlap, touched_ids, touched_key, touched_keys}` | `qp_ops::…` (modül: `qp_ops::apply`) |
| `qp_session::ENGINE_PRINCIPAL` (`session` modülünde) | `qp_ops::ENGINE_PRINCIPAL` (`qp_ops::apply`) |
| `qp_session::{canon_document, thresholds}` | `qp_ops::…` (modül: `qp_ops::canon`) |
| `qp_session::profile_for` | `qp_ops::profile_for` (modül: `qp_ops::present`) |
| `qp_session::DocProvider` | `qp_ops::DocProvider` (modül: `qp_ops::provider`) |
| `qp_session::{DataError, DataOutcome, HostNotice}` | `qp_plane::…` (modül: `qp_plane::plane`) |
| `qp_session::{did_you_mean, edit_distance}` | `qp_plane::…` (modül: `qp_plane::suggest`) |
| `qp_session::headparts`, `qp_session::capsule::headparts` (`replay_calendar`, …) | `qp_plane::headparts` |
| (crate içi) `crate::{apply, canon, expand, gops, locate, select, summary, present, provider}` | `qp_ops::…` |
| (crate içi) `crate::{error, suggest, values, plane, capidx}`, `crate::capsule::Proj`, `crate::capsule::{story_fact_ids, sentence_pair_memo, story_validity, with_sentence_input}` | `qp_plane::{error, suggest, values, plane, capidx}`, `qp_plane::proj::Proj`, `qp_plane::sentences::…` |
| (crate içi) `crate::watch::{expand_watch, watch_visible, intent_visible, visible_ids, WATCH_PREFIX, person_of, MAX_WATCHES}`, `on` | `qp_ops::watch::…` (`Matched`, `negative`, `holds`, `matches` ve `impl Session` `qp-session`'da kaldı) |
| (crate içi) `crate::live::closure_of`, `cond_ids` | `qp_ops::live::…` |

**Yeni yollar:**

- `qp_ops::HeadlineSource` (`qp_ops::provider`): `DocProvider::with_parts` artık `&dyn HeadlineSource` alır;
  `impl HeadlineSource for Session` `qp-session`'da (`capsule/headsource.rs`). Çağıranlar
  (`with_parts(self, lang)`) değişmedi.
- `qp_ops::{apply, canon, expand, gops, live, locate, present, provider, select, summary, watch}` modülleri
- `qp_plane::{capidx, error, headparts, plane, proj, sentences, suggest, values}` modülleri

**`pub(crate)` → `pub` (derleyiciyle ölçüldü):** taşınan iki crate'te öğeler önce `pub(crate)`'e
geri alındı; yalnız başka crate'ten (çoğu `qp-session`, birkaçı `qp-ops` → `qp-plane`) kullanıldığı
için derleyicinin hata verdiği yerler açıldı, ayrıca `pub` yöntemlerin/alanların tipleri
(`private_bounds`/`private_interfaces`: `Indexed`, `Computed`, `Expanded`, `BoundSeries`, …).
Satır sayımıyla ≈ 160 bildirim (modül başına; `x::*` glob yeniden dışa aktarmalardır, `(özel idi)`
önceden özeldi). Ters yönde üç alan daraldı (main'de `pub`, artık `pub(crate)`; tipleri
`pub(crate)`): `canon::Site::reads`, `locate::Violation::kind`, `select::Site::kind`.

- `qp-ops::canon` (2): `origin::*`, `Site`
- `qp-ops::canon::origin` (1): `origin_text`
- `qp-ops::expand` (15): `bind::*`, `describe::summarize_ops`, `op::expand_op`, `Ctx`, `valid_id`, `need_series`, `need_view`, `need_finding`, `check_condition`, `view_bar_series_any`, `view_bar_series`, `undone_set`, `need_proposal`, `expand_ops`, `Expanded`
- `qp-ops::expand::bind` (1): `condition_entitled`
- `qp-ops::expand::describe` (1): `summarize_ops`
- `qp-ops::live` (2): `closure_of`, `cond_ids (özel idi)`
- `qp-ops::locate` (1): `locate`
- `qp-ops::present` (1): `present`
- `qp-ops::provider` (6): `new`, `with_headlines`, `with_parts`, `with_voice`, `with_clock`, `warm`
- `qp-ops::select` (7): `sites::*`, `Site`, `violation_error`, `finding_bar`, `finding_about`, `Resolver`, `resolve_group`
- `qp-ops::select::resolve` (4): `panel_hint`, `layer_hint`, `fact_hint`, `finding_hint`
- `qp-ops::select::sites` (2): `sites`, `is_selector`
- `qp-ops::summary` (2): `FastHints`, `summarize_doc`
- `qp-ops::summary::hints` (3): `new`, `fact_hint`, `finding_hint`
- `qp-ops::watch` (1): `on (özel idi)`
- `qp-plane::capidx` (24): `items::*`, `IdIndex`, `Indexed`, `refresh`, `get`, `by_finding`, `SentenceMemo`, `BarsSnap`, `snap`, `HeadState`, `unregister`, `Contrib`, `ReaderProj`, `new`, `contains`, `capsules_of_fact`, `remove`, `add`, `result`, `proj_breaks`, `rd_snap`, `rd_same`, `ReaderView`, `VIEWS: usize`
- `qp-plane::capidx::items` (14): `ItemKeyRef`, `capsule`, `facts`, `series`, `voice`, `clock`, `cal`, `tz`, `ItemMemo`, `hits`, `misses`, `put`, `clear`, `prune`
- `qp-plane::error` (14): `PResult`, `E`, `new`, `at`, `expected`, `dym`, `suggest`, `hint`, `reason`, `conflicts`, `build`, `invalid`, `ptr`, `num`
- `qp-plane::headparts` (3): `story_parts`, `story_voice`, `event_stamp`
- `qp-plane::plane` (27): `Provider`, `BoundSeries`, `Blocked`, `state_str`, `unentitled`, `error`, `EventOutcome`, `DataPlane`, `binding_of`, `is_bar_series`, `bar_root`, `same_but_anchor`, `new`, `sync`, `refresh`, `bars`, `host_state`, `entitled`, `state_for`, `gate_layer`, `apply_event`, `layer_reads`, `inject`, `reading_doc`, `Reader`, `bars_for`, `evidence_ref`
- `qp-plane::proj` (2): `Proj`, `mentions_hidden`
- `qp-plane::sentences` (3): `sentence_pair_memo`, `story_validity`, `story_fact_ids`
- `qp-plane::values` (23): `Delta`, `first_bar`, `Cache`, `clear`, `get`, `slot_of`, `val`, `touched_now`, `gen`, `forget`, `begin_tick`, `apply_delta`, `find_series`, `bar_series`, `bars_of`, `main_output`, `study_outputs`, `values`, `condition_series`, `cmp`, `eval_condition`, `eval_on`, `holds`
- `qp-plane::values::computed` (3): `Computed`, `len`, `main_values`

## S3.0 — `qp-authority`, `qp-subsystem`, `qp-session` bileşimi

Çağıranların yolu değişmedi: `qp-session` artık bileşimdir ve `qp-authority`'nin kök yüzeyini aynı
adlarla bilerek yeniden dışa aktarır (`crates/qp-session/src/lib.rs`). `qp-host`, `qp-mcp`,
`qp-shellcore`, `qp-cli`, `qp-wasm`, `qp-render` dokunulmadı.

| Eski | Yeni |
|---|---|
| `qp_session::…` (crate `crates/qp-session`, otorite) | `qp_authority::…` (crate `crates/qp-authority`); `qp_session::…` aynı adla yeniden dışa aktarım |
| `qp_session::{Session, permissions, confine, default_limits, read_json, Replica, ReplicaOptions, document_digest, view_digest, ReplayHost, demo_init, hit_scene, screen_frame, explain_scene, HIT_WIDTH, HIT_HEIGHT, CoinFlipRule, CommentAudit, CompileOptions, SettleRequest, MAX_DISPUTE_DIGESTS, MAX_OPEN_DISPUTES, MAX_DISPUTES_PER_CONN}` ve modüller `capsule`, `replica`, `viewer`, `replayhost`, `agent`, `voices`, `tape` (gizli) | aynı yol; tanım `qp_authority::…`'da |
| `qp_session::conform` (otorite crate'inde `host/conform.rs`) | aynı yol; tanım bileşimde (`crates/qp-session/src/conform.rs`, saf taşıma). `qp_authority::conform` yok |
| `qp_session::agent::AgentOp` | aynı yol; tanım `qp_subsystem::AgentOp` (`agent.rs`'te `pub use`) |
| (crate içi) `session::{confine, read_json}` (`authority/session.rs`) | `authority/session/files.rs`; `session.rs`'te `pub use files::{confine, read_json}` (dış yol aynı) |
| (crate içi) iki `op_name` kopyası (`authority/session.rs`, `authority/agent.rs`) | `qp_subsystem::op_name` (tek kopya) |
| `qp_plane::values::Computed::new` (`pub(super)`) | `pub` (değer kaynağı `ValueSource` kurabilsin) |

**Yeni yollar:**

- `qp_subsystem::{Subsystem, SessionView, ConnView, RunKind, SessionCtx, Slots, PerWorld, Outbox, EngineEntry, Phase, Moment, OpHandler, AgentOp, QueryHandler, InputHandler, TopicHandler, ConditionKind, ValueSource, ClaimKind, Registry, op_name}`
- `qp_authority::Session::{register (gizli), next_wake, host_adjust (gizli)}`; `impl qp_subsystem::SessionView for Session`
- `qp_session::conform` modülü bileşimin kendi modülü (`pub mod conform`)

## S3.1 — `qp-watch`, bileşimin kurucuları

Çağıranların yolu korunur: `qp_session::{Session, ReplayHost, Replica}`, `qp_session::viewer::OpenSide`
ve `qp_session::tape::{run, run_with, run_viewer}` aynı ad ve imzayla bileşimin sarıcılarıdır
(`crates/qp-session/src/compose.rs`; `Deref`/`DerefMut` otorite tipine). Değişen yerler, alt sistem
listesini taşıyan otorite imzaları ve sarıcının içindeki oturumun tipi:

| Eski | Yeni |
|---|---|
| `qp_session::Session` = `qp_authority::Session` (aynı tip) | `qp_session::Session` bileşimin yeni tipi (`Session(qp_authority::Session)`, `Deref`); otoritenin kendisi `qp_session::Authority` |
| `qp_session::{ReplayHost, Replica}`, `qp_session::viewer::OpenSide` = otorite tipleri | bileşimin sarıcıları (aynı kurucular, `STANDARD` ile); `ReplayHost::session()` / `session_mut()` `&Authority` döner (otorite tipi) |
| `qp_session::agent::agent_op` = `qp_authority::agent::agent_op` | bileşimin `agent_op`'u: standart alt sistemin işlemi kendi işleyicisiyle (`add_watch`, `remove_watch` → `Direct`), öbürü otoritenin listesi; `qp_session::agent::*` geri kalanı aynı |
| `qp_session::tape::{run, run_with, run_viewer}` = otoritenin | bileşimin sarıcıları (aynı imza) |
| `qp_authority::Session::{new, new_with_trust_keys, with_provider, with_provider_and_trust_keys}(…)` | son parametre `subs: qp_authority::Subsystems` |
| `qp_authority::ReplayHost::{new, with_local_dir, with_window, live}(…)` | son parametre `subs: Subsystems` |
| `qp_authority::Replica::{new, with_memory}(…)`, `qp_authority::viewer::OpenSide::new(…)` | son parametre `subs: Subsystems` |
| `qp_authority::tape::{run, run_with, run_viewer}(…)` | son parametre `subs: Subsystems` |
| `qp_ops::watch::expand_watch` (`pub(crate)`) | taşındı: `qp-watch` `ops.rs` (`expand_watch` → `add_watch`, `remove_watch`; gizli) |
| `qp_ops::watch::watch_visible` (`pub(crate)`) | `pub` (`qp-watch` kullanır) |
| `qp_ops::expand::Ctx::{prov, new_id}` (`pub(crate)`) | `pub` (`qp-watch`'ın açılımı) |
| (crate içi) `qp_authority` `capsule::watch` (`Matched`, `match_watches`, `watch_lines`, `intent_for_reader`) | `Matched`, `match_watches`, `watch_lines` → `qp-watch` `matching.rs` (gizli); `intent_for_reader` → `capsule/project.rs` (dokümanın izdüşümü) |
| `qp_mcp::Hub::{session, session_mut}` → `&qp_session::Session` | `&qp_session::Authority` (düz oturum `Deref` ile, barındırıcınınki doğrudan) |
| (sınama yardımcıları) `tests/charac/rig.rs` `Rig::{session, session_mut}`, `tests/open_nobra.rs` `open_scene_in` | `&qp_session::Authority` (kitaplar aynı) |
| `qp-shellcore` `viewer.rs` `.map(OpenSide::trust)` | `.map(|o| o.trust())` (yöntem `Deref` üstünden) |

**Yeni yollar:**

- `qp_watch::{Watch, WATCH}`
- `qp_authority::Subsystems` (`NONE`, `new`, `list`); `qp_authority::Session::{subsystems, slots (gizli), slots_mut (gizli)}`
- `qp_session::{STANDARD, Authority, Subsystems, compose}`; `qp_session::Session::{sub, sub_mut, into_inner}`
- `qp_subsystem::{MAX_EMIT_DEPTH}`; `OpHandler::owns`; `Outbox::{amend, say, take_changes, take_lines}`;
  `Moment::{Look, Written}`; `SessionView::{conn_ids, subs, public_view, reader_proj}`;
  `Registry::evaluates_conditions`
- `qp_ops::expand::expand_ops_with` (tek açılım gövdesi; `expand_ops` ve `Registry::expand_ops` onu kullanır)

## S3.2 — `qp-read` (okuma ve sunum)

Çağıranların yolu korunur: `qp_session::{explain_scene, hit_scene, screen_frame, HIT_WIDTH,
HIT_HEIGHT}` aynı adlarla (artık `qp_read`'den); `qp_session::Session::{scene_for, explain}` aynı
imzayla bileşimin yöntemleri; `provider`, `scene`, `scene_as_reader`, `surface_kind` otoritede
(`Deref` üstünden aynı çağrı). Sorgu ve girdi tel biçimi aynı.

| Eski | Yeni |
|---|---|
| `qp_authority::{explain_scene, hit_scene, screen_frame, HIT_WIDTH, HIT_HEIGHT}` | `qp_read::…` (`qp_session` aynı adlarla yeniden dışa aktarır) |
| `qp_authority::Session::scene_for` | `qp_session::Session::scene_for` (bileşim: `qp_read::scene_for_range` + otoritenin `scene`) |
| `qp_authority::Session::explain` | `qp_session::Session::explain` (bileşim: otoritenin `scene` + `qp_read::explain_scene`) |
| `qp_authority::viewer::{first_facts, FirstFacts}` | `qp_ops::firsts::{first_facts, FirstFacts}` (eski yolda `pub use` ile duruyor) |
| (crate içi) `qp_authority` `read::query::{check_target_pub, resolve_range}`, `read::reading::without_layers`, `Session::gated_view` | `qp_ops::bounds::{check_target_pub, resolve_range, without_layers, gated_view}` (`pub`; otorite ve `qp-read` ortak) |
| (crate içi) `read/{query, query/view, interact, interact/{scenes,style}, preview, explain, reading}.rs`, `data/bindq.rs`, `capsule/queries.rs` | `qp-read` `src/{query, query/view, interact, interact/{scenes,style}, preview, explain, reading, bindq, capsules}.rs` (`impl Session` → `impl Rd<'_>`) |
| (crate içi) `capsule/queries.rs` `capsule_query`, `capsule_query_with`, `evidence_access` | otoritede kaldı: `capsule/detail.rs` (yorum doğrulayıcısı aynı ayrıntıyı okur); `voice`/`clock` parametre |
| (crate içi) `capsule/queries.rs` `ledger_now`, `withheld_for` | otoritede kaldı: `capsule/run.rs` |
| (crate içi) `Session::query`, `Session::input` (yerleşik yol) | `qp-read` (`READ`, `ANY` kolu); kayıtsız otorite `unsupported` döner |
| (crate içi) `Conn::{viewports, current, last_input, last_at}`, `Session::{focus, input_counter}` | `qp_read::ReadState` (yuva) |
| (crate içi) `Conn::look` | `Session::looks` (bağlantı sırasıyla vektör; `SessionCtx::looks` ile ödünç) |
| (sınama) `qp-authority` `capsule/tests/item_memo.rs` | `qp-session` `tests/capsule_perf_equality/item_memo.rs` (bileşimle) |

**Yeni yollar:**

- `qp_read::{Read, READ, ReadState, scene_for_range, explain_scene, hit_scene, screen_frame, HIT_WIDTH, HIT_HEIGHT}`
- `qp_subsystem::ANY`; `SessionCtx::{cache, looks}`; `ConnView::{hello, profile}`;
  `SessionView::{principals, headlines, headline_source, proj, directives, project_entry,
  evidence_for, viewport_events, accept_preview, stories_query, story_rows, row_stamp,
  capsule_item, reader_voice, market_item, capsule_passes, coin_flip_hidden_by, relevant,
  coin_flip_of, is_current, item_series, fact_by_id, item_memo, headline_cal, comments_for,
  capsule_query, ledger_now}`
- `qp_authority::Session::with_subsystems` (gizli; bileşimin `scene_for`'u)
- `qp_ops::{bounds, firsts}`
