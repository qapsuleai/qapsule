//! **Control — not a candidate.** A deliberately trivial alphabet for testing the harness
//! itself: a filled circle in the theme's accent colour with the kind's first letter on
//! it. It encodes the kind and nothing else (direction, strength, phase, confidence,
//! freshness, confirmations and the server mark are ignored), so the harness must report
//! zero distance for every channel pair. Do not start a candidate from it.

use qp_grammar::glyph::{Alphabet, GlyphBox};
use qp_grammar::theme::Theme;
use qp_schema::capsule::{GlyphSpec, MeaningKind};
use qp_schema::scene::{Align, Prim};

/// The control alphabet.
#[derive(Debug, Clone, Copy, Default)]
pub struct Control;

/// The control's legend (used in the `legend` reading condition).
pub const LEGEND: &str = "Each sign is a circle with one capital letter: the first letter \
of the meaning kind (C continuation, R reversal, B breakout, S squeeze, E extreme, F flow). \
Nothing else is encoded.";

fn letter(k: MeaningKind) -> &'static str {
    match k {
        MeaningKind::Continuation => "C",
        MeaningKind::Reversal => "R",
        MeaningKind::Breakout => "B",
        MeaningKind::Squeeze => "S",
        MeaningKind::Extreme => "E",
        MeaningKind::Flow => "F",
    }
}

impl Alphabet for Control {
    fn name(&self) -> &'static str {
        "control"
    }

    fn draw(&self, spec: &GlyphSpec, bx: GlyphBox, theme: &Theme) -> Vec<Prim> {
        let c = bx.size / 2.0;
        let px = (bx.size * 0.6).round().max(1.0);
        #[allow(clippy::cast_possible_truncation)]
        let (x, baseline) = (
            (bx.x + c - px * 0.3).round() as i32,
            (bx.y + c + px * 0.35).round() as i32,
        );
        vec![
            Prim::Circle {
                cx: bx.x + c,
                cy: bx.y + c,
                r: c * 0.92,
                color: theme.accent,
            },
            Prim::Text {
                x,
                baseline,
                px,
                text: letter(spec.kind).to_owned(),
                color: theme.bg,
                align: Align::Left,
            },
        ]
    }
}
