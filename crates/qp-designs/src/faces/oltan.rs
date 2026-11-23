//! R5/M16 face design **`oltan`**: the mark leads — a watch-face tile.
//!
//! The meaning glyph is the hero: the largest, highest-contrast element of the face, so a
//! glance (or a blurred, shrunk copy of the screen) lands on it first and reads direction,
//! kind and validity from its shape and colour (K7). The sentence is secondary but easy to
//! read: the headline in a column beside the glyph, the track record (karne) as a full
//! sentence under both. Nothing else: one sentence, one mark, one additional item (K43).
//!
//! Layout (390 × [`height`] CSS px; the compiler's plot margins apply; `oltan` look: plot
//! x 8..318, top 28, glyph 120 px, face 228 px):
//!
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │ ┌────────────┐ SYN20 son 20 günün en          │
//! │ │            │ yüksek fiyatını (227,08)       │  headline (look's `headline`
//! │ │   glyph    │ aştı · bugün, hâlâ geçerli     │  role), at most [`head_lines`]
//! │ │  (120 px)  │                                │  lines beside the glyph
//! │ └────────────┘                                │
//! │ Benzer 20 durumun 11'inde fiyat 10 işlem      │  karne (look's `body` role,
//! │ günü içinde yükseldi.                         │  muted), [`LEDGER_LINES`] lines
//! └──────────────────────────────────────────────┘
//! ```
//!
//! - `glyph`: the look's `glyph.face_px` at the panel's top left.
//! - `headline`: the finding's `headline` text field (the voice's headline, or under the
//!   `r4` voice `vadrel`'s template), beside the glyph; full text colour while the claim
//!   holds, `muted` once it no longer holds or has closed (the words say which; `muted`,
//!   not `invalid`: the look keeps `invalid` for the glyph, where it may sit under the
//!   4.5:1 text threshold).
//! - `ledger`: the finding's `ledger` text field (the voice's karne sentence; under `r4`
//!   the R3 ledger line) in a band of [`LEDGER_LINES`] lines at the panel's bottom, full
//!   width, `muted`. Glossary terms are marked here (never in the headline, K43).
//!
//! The title row stays empty: the instrument leads the headline. Without a capsule story
//! the face falls back to the reference card ([`depth::capsule_view_in`]).

use qp_schema::capsule::Validity;
use qp_schema::grammar::{
    ColorSpec, Layer, Mark, Panel, PanelPos, Position, Scale, ScaleKind, SizeSpec, View,
};

use qp_grammar::depth;
use qp_grammar::provider::DataProvider;
use qp_look::error::GrammarError;
use qp_look::look::Look;

/// The phone width every face is laid out at (CSS px; protocol `hello`).
const FACE_W: f64 = 390.0;
/// The headline limit both voices and `vadrel`'s template keep (characters).
const MAX_CHARS: f64 = 80.0;
/// Slack over the mean advance for word wrapping (a line breaks before a long word):
/// measured over the 1 062 marla headlines (`rounds/R5/experiments/M17/marla/TABLE.md`),
/// the worst case in the `oltan` look is 5 lines, what this gives.
const WRAP_SLACK: f64 = 1.15;
/// Lines of the ledger band: a 70-character karne sentence in the `oltan` look's `body`
/// role takes at most two lines at the panel's width (all 328 marla ledgers measured).
pub const LEDGER_LINES: f64 = 2.0;

fn line_px(px: f64, line: f64) -> f64 {
    (px * line).round()
}

/// The headline's column width in `look`: the panel's width less the glyph and its gap.
fn column(look: &Look) -> f64 {
    let [l, r, _, _] = look.space.plot;
    FACE_W - f64::from(l + r) - look.glyph.face_px - look.space.glyph_gap
}

/// Lines the headline may take beside the glyph in `look`: an 80-character headline at
/// the headline font's mean advance, with [`WRAP_SLACK`], in [`column`].
#[must_use]
pub fn head_lines(look: &Look) -> f64 {
    let h = look.types.headline;
    let adv = look.role_font("headline").advance;
    (MAX_CHARS * h.px * adv * WRAP_SLACK / column(look).max(1.0))
        .ceil()
        .max(2.0)
}

/// Height of the glyph-and-headline block in `look`.
fn top_block(look: &Look) -> f64 {
    let h = look.types.headline;
    (head_lines(look) * line_px(h.px, h.line)).max(look.glyph.face_px)
}

/// The ledger band's height in `look`.
fn ledger_band(look: &Look) -> f64 {
    let b = look.types.body;
    LEDGER_LINES * line_px(b.px, b.line)
}

/// The face height in `look` (CSS px at the phone width): the plot's margins, the glyph
/// and headline block, the panel gap and the ledger band. `oltan`: 28 + 120 + 10 + 44 + 24
/// = 226.
#[must_use]
pub fn height(look: &Look) -> u32 {
    let total = f64::from(look.space.plot[2] + look.space.plot[3])
        + top_block(look)
        + f64::from(look.space.panel_gap)
        + ledger_band(look);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = total.ceil().clamp(120.0, 844.0) as u32;
    px
}

fn layer(id: &str, mark: Mark, finding: &str) -> Layer {
    let mut l = Layer::new(mark, 0);
    l.id = Some(id.to_owned());
    l.finding = Some(finding.to_owned());
    l
}

fn at(panel: f64, px: f64) -> Position {
    Position::Panel(PanelPos { panel, px })
}

/// The face view of `finding` (see the module doc).
///
/// # Errors
/// Unknown finding/fact, a finding without facts, or a series without bars.
pub fn view(
    look: &Look,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    let story = p.capsule(finding).is_some_and(|c| c.story.is_some());
    if !story {
        return depth::capsule_view_in(look, finding, max_points, p, id);
    }
    // The reference card over one point: its x domain is the last bar only (no axis).
    let card = depth::capsule_card_view_in(look, finding, 1, p, id)?;
    let glyph_px = look.glyph.face_px;
    let holds = p
        .headline_parts(finding)
        .is_none_or(|h| h.validity == Validity::Valid);

    let mut glyph = layer("glyph", Mark::Glyph, finding);
    glyph.encoding.x = Some(at(0.0, 0.0));
    glyph.encoding.y = Some(at(0.0, 0.0));
    glyph.encoding.size = Some(SizeSpec::Constant(glyph_px));

    let mut head = layer("headline", Mark::Text, finding);
    head.encoding.text = Some("headline".to_owned());
    head.encoding.x = Some(at(0.0, glyph_px + look.space.glyph_gap));
    head.encoding.y = Some(at(0.0, 0.0));
    head.encoding.size = Some(SizeSpec::Constant(look.types.headline.px));
    if !holds {
        head.encoding.color = Some(ColorSpec::Constant("muted".to_owned()));
    }

    let mut ledger = layer("ledger", Mark::Text, finding);
    ledger.encoding.text = Some("ledger".to_owned());
    ledger.encoding.x = Some(at(0.0, 0.0));
    ledger.encoding.y = Some(at(1.0, -ledger_band(look)));
    ledger.encoding.size = Some(SizeSpec::Constant(look.types.body.px));
    ledger.encoding.color = Some(ColorSpec::Constant("muted".to_owned()));

    Ok(View {
        id: id.to_owned(),
        title: Some(String::new()),
        display_tz: None,
        theme: None,
        x: card.x,
        y: Vec::new(),
        panels: vec![Panel {
            id: Some(0),
            name: Some("face".to_owned()),
            weight: Some(1.0),
            y: Scale {
                kind: ScaleKind::Linear,
                domain: None,
                id: Some("y0".to_owned()),
            },
        }],
        layers: vec![glyph, head, ledger],
        emphasis: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qp_look::look::{OLTAN, R4};

    #[test]
    fn oltan_geometry() {
        // The column holds the measured worst case (5 lines) and the glyph leads the block.
        assert!((column(&OLTAN) - 176.0).abs() < 1e-9);
        assert!((head_lines(&OLTAN) - 5.0).abs() < 1e-9);
        assert!((top_block(&OLTAN) - 120.0).abs() < 1e-9);
        assert!((ledger_band(&OLTAN) - 44.0).abs() < 1e-9);
        assert_eq!(height(&OLTAN), 226);
        // The glyph is the largest element: taller than a headline line by far and as
        // tall as the whole headline block.
        const { assert!(OLTAN.glyph.face_px >= 6.0 * OLTAN.types.headline.px) };
        assert!(OLTAN.glyph.face_px >= head_lines(&OLTAN) * 24.0);
    }

    #[test]
    fn any_look_gets_a_face_that_fits() {
        // The reference look (monospaced 20 px headline, 32 px glyph): 22 characters a
        // line in a 268 px column, five lines for 80 characters; the face follows.
        assert!((column(&R4) - 268.0).abs() < 1e-9);
        assert!((head_lines(&R4) - 5.0).abs() < 1e-9);
        assert_eq!(height(&R4), 34 + 130 + 8 + 2 * 16 + 24);
    }
}
