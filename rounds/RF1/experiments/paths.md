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

**`pub(crate)` → `pub`:** taşınan iki crate'te bütün `pub(crate)` öğeler `pub` oldu (252 yer;
crate sınırını geçtikleri için, tek tek ayıklanmadı). Ayrıca özel olan `qp_ops::watch::on` ve
`qp_ops::live::cond_ids` `pub` oldu (oturum kullanıyor).
