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
