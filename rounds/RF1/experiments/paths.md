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
