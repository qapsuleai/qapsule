//! Golden glyph alphabet sheets (R3 criterion 7): the product alphabet
//! (`qp_grammar::glyph::alphabet`, `siluet`) for the six meaning kinds × three directions
//! (rows) × five phases (columns: candidate, final, realized, failed, retracted) at 32 px,
//! in the light and the dark theme: `fixtures/golden/views/g1-glyphs-light.png`,
//! `g1-glyphs-dark.png`. Other channels fixed: strength 0.6, confidence 0.6 (fill),
//! freshness 1, two confirmations, no server mark.

#[path = "support/golden_check.rs"]
mod golden_check;

use qp_grammar::glyph::{alphabet, GlyphBox};
use qp_grammar::Theme;
use qp_render::render_scene_png;
use qp_render::scene::scene_of;
use qp_schema::capsule::{Direction, GlyphPhase, GlyphSpec, MeaningKind};
use qp_schema::scene::{Align, Prim};

const PHASES: [(GlyphPhase, &str); 5] = [
    (GlyphPhase::Candidate, "candidate"),
    (GlyphPhase::Final, "final"),
    (GlyphPhase::Realized, "realized"),
    (GlyphPhase::Failed, "failed"),
    (GlyphPhase::Retracted, "retracted"),
];
const DIRS: [(Direction, &str); 3] = [
    (Direction::Up, "up"),
    (Direction::Down, "down"),
    (Direction::Neutral, "neutral"),
];
const GLYPH: f32 = 32.0;
const CELL: i32 = 76;
const ROW: i32 = 44;
const LABEL_W: i32 = 176;
const HEAD: i32 = 30;

fn sheet(theme: &Theme) -> Vec<u8> {
    let mut prims = Vec::new();
    for (j, (_, name)) in PHASES.iter().enumerate() {
        prims.push(Prim::Text {
            x: LABEL_W + CELL * i32::try_from(j).unwrap() + 34
                - i32::try_from(name.len() * 36 / 10).unwrap(),
            baseline: 20,
            px: 12.0,
            text: (*name).to_owned(),
            color: theme.muted,
            align: Align::Left,
        });
    }
    let mut row = 0;
    for kind in MeaningKind::ALL {
        for (dir, dname) in DIRS {
            let y = HEAD + ROW * row;
            if row % 3 == 0 && row > 0 {
                prims.push(Prim::Rect {
                    x: 8,
                    y: y - 3,
                    w: u32::try_from(LABEL_W + CELL * 5 - 16).unwrap(),
                    h: 1,
                    color: theme.grid,
                });
            }
            prims.push(Prim::Text {
                x: 10,
                baseline: y + 25,
                px: 13.0,
                text: format!("{} {dname}", kind.name()),
                color: theme.text,
                align: Align::Left,
            });
            for (j, (phase, _)) in PHASES.iter().enumerate() {
                let spec = GlyphSpec {
                    kind,
                    direction: dir,
                    strength: 0.6,
                    confidence: Some(0.6),
                    phase: *phase,
                    freshness: 1.0,
                    confirmations: 2,
                    server: false,
                };
                #[allow(clippy::cast_precision_loss)]
                let bx = GlyphBox {
                    x: (LABEL_W + CELL * i32::try_from(j).unwrap() + 18) as f32,
                    y: (y + 4) as f32,
                    size: GLYPH,
                };
                prims.extend(alphabet().draw(&spec, bx, theme));
            }
            row += 1;
        }
    }
    let w = u32::try_from(LABEL_W + CELL * 5).unwrap();
    let h = u32::try_from(HEAD + ROW * row + 6).unwrap();
    render_scene_png(&scene_of(w, h, theme.bg, prims)).unwrap()
}

#[test]
fn glyph_sheet_light() {
    let a = sheet(&Theme::light());
    assert_eq!(a, sheet(&Theme::light()), "two renders differ");
    golden_check::check_file("fixtures/golden/views/g1-glyphs-light.png", &a);
}

#[test]
fn glyph_sheet_dark() {
    let a = sheet(&Theme::dark());
    assert_eq!(a, sheet(&Theme::dark()), "two renders differ");
    golden_check::check_file("fixtures/golden/views/g1-glyphs-dark.png", &a);
}
