//! M11 outcome style **`tg5`** ("negative tile / faded trace"): the verdict is carried by
//! the whole glyph's ink balance, not by a small corner mark.
//!
//! - **Realized:** the glyph is *stamped*: a solid ink plate (siluet: rounded square
//!   filling the box; rozet: the full coin) with the open-look glyph knocked out of it in
//!   the background colour (a negative print), and a light corner coin carrying a dark ✓.
//! - **Failed** (and expired): the glyph is *faded*: every stroke turns pale grey, nothing
//!   is filled, and a heavy dark ✗ (with a background halo) sits in the same corner.
//! - **Open** (candidate, final) and **retracted**: the alphabet's own drawing.
//!
//! So the two verdicts are opposite in every channel that survives without colour: solid
//! vs. empty plate (most of the box area flips between ink and background), dark vs. pale
//! glyph, ✓ vs. ✗ in a corner disc vs. a bare cross. Open glyphs keep their direction
//! colour and have no corner mark.
//!
//! Rules (`glyph`, `docs/GRAMMAR.md` §18): IEEE basic operations only (corner arcs are
//! precomputed constants), theme colours only, every primitive inside the box.

use qp_schema::capsule::{GlyphPhase, GlyphSpec};
use qp_schema::scene::{Prim, Rgba};

use super::{open_look, OutcomeStyle};
use crate::glyph::{Alphabet, GlyphBox};
use crate::theme::{with_opacity, Theme};

/// The `tg5` outcome style.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tg5;

/// cos/sin of 0°, 15°, …, 90° (a quarter circle).
const QUARTER: [[f32; 2]; 7] = [
    [1.0, 0.0],
    [0.965_925_8, 0.258_819_04],
    [0.866_025_4, 0.5],
    [0.707_106_77, 0.707_106_77],
    [0.5, 0.866_025_4],
    [0.258_819_04, 0.965_925_8],
    [0.0, 1.0],
];

/// Opacity of the faded (failed) glyph.
const FADE: f64 = 0.55;

/// `c`'s colour with `alpha`'s alpha channel.
fn rgb_of(c: Rgba, alpha: Rgba) -> Rgba {
    [c[0], c[1], c[2], alpha[3]]
}

/// Every primitive of `prims` recoloured by `f`.
fn recolor(mut prims: Vec<Prim>, f: impl Fn(Rgba) -> Rgba) -> Vec<Prim> {
    for p in &mut prims {
        let c = match p {
            Prim::Rect { color, .. }
            | Prim::Polyline { color, .. }
            | Prim::Fill { color, .. }
            | Prim::Circle { color, .. }
            | Prim::Text { color, .. } => color,
        };
        *c = f(*c);
    }
    prims
}

/// A rounded square covering `bx` inset by `inset`, corner radius `rad` (pixels).
fn rounded_square(bx: GlyphBox, inset: f32, rad: f32) -> Vec<[f32; 2]> {
    let (x0, y0) = (bx.x + inset, bx.y + inset);
    let (x1, y1) = (bx.x + bx.size - inset, bx.y + bx.size - inset);
    let mut pts = Vec::with_capacity(28);
    // Corners clockwise from bottom-right (y down): centre, start angle quadrant.
    let corners: [([f32; 2], [f32; 2]); 4] = [
        ([x1 - rad, y1 - rad], [1.0, 1.0]),
        ([x0 + rad, y1 - rad], [-1.0, 1.0]),
        ([x0 + rad, y0 + rad], [-1.0, -1.0]),
        ([x1 - rad, y0 + rad], [1.0, -1.0]),
    ];
    for (i, (c, sg)) in corners.iter().enumerate() {
        for k in 0..QUARTER.len() {
            // Alternate the sweep direction so the outline runs around once.
            let [u, v] = if i % 2 == 0 {
                QUARTER[k]
            } else {
                QUARTER[QUARTER.len() - 1 - k]
            };
            pts.push([c[0] + sg[0] * u * rad, c[1] + sg[1] * v * rad]);
        }
    }
    pts
}

/// The corner disc the verdict sits in: centre and radius (pixels).
fn corner(bx: GlyphBox) -> (f32, f32, f32) {
    let s = bx.size;
    let r = 0.215 * s;
    let halo = (0.06 * s).max(1.0);
    // Keep the haloed disc inside the box.
    let c = s - r - halo;
    (bx.x + c, bx.y + c, r)
}

/// ✓ in `ink` on a `paper` disc, ringed by `rim` (the plate colour).
fn check_coin(out: &mut Vec<Prim>, bx: GlyphBox, paper: Rgba, ink: Rgba, rim: Rgba) {
    let s = bx.size;
    let (cx, cy, r) = corner(bx);
    let halo = (0.06 * s).max(1.0);
    out.push(Prim::Circle {
        cx,
        cy,
        r: r + halo,
        color: rim,
    });
    out.push(Prim::Circle {
        cx,
        cy,
        r,
        color: paper,
    });
    let q = |u: f32, v: f32| [cx + u * r, cy + v * r];
    out.push(Prim::Polyline {
        points: vec![q(-0.56, 0.02), q(-0.16, 0.42), q(0.56, -0.4)],
        width: (0.3 * r).max(1.2),
        color: ink,
    });
}

/// A bold ✗ in `ink` with a `paper` halo, in the verdict corner.
fn cross(out: &mut Vec<Prim>, bx: GlyphBox, paper: Rgba, ink: Rgba) {
    let s = bx.size;
    let (cx, cy, r) = corner(bx);
    let halo = (0.06 * s).max(1.0);
    out.push(Prim::Circle {
        cx,
        cy,
        r: r + halo,
        color: paper,
    });
    let w = (0.36 * r).max(1.4);
    let k = r - 0.5 * w;
    let k = k * 0.86;
    for (a, b) in [
        ([cx - k, cy - k], [cx + k, cy + k]),
        ([cx + k, cy - k], [cx - k, cy + k]),
    ] {
        out.push(Prim::Polyline {
            points: vec![a, b],
            width: w,
            color: ink,
        });
    }
}

impl OutcomeStyle for Tg5 {
    fn name(&self) -> &'static str {
        "tg5"
    }

    fn draw(
        &self,
        alphabet: &dyn Alphabet,
        spec: &GlyphSpec,
        bx: GlyphBox,
        theme: &Theme,
    ) -> Vec<Prim> {
        let s = bx.size;
        let fresh = 0.3 + 0.7 * spec.freshness.clamp(0.0, 1.0);
        let ink = with_opacity(theme.text, fresh);
        match spec.phase {
            GlyphPhase::Realized => {
                let mut out = Vec::new();
                if alphabet.name() == "rozet" {
                    out.push(Prim::Circle {
                        cx: bx.x + 0.5 * s,
                        cy: bx.y + 0.5 * s,
                        r: 0.5 * s - 0.35,
                        color: ink,
                    });
                } else {
                    out.push(Prim::Fill {
                        points: rounded_square(bx, 0.0, 0.2 * s),
                        color: ink,
                    });
                }
                // The open glyph, knocked out of the plate (negative print).
                out.extend(recolor(open_look(alphabet, spec, bx, theme), |c| {
                    rgb_of(theme.bg, c)
                }));
                check_coin(&mut out, bx, theme.bg, ink, ink);
                out
            }
            GlyphPhase::Failed => {
                let mut out = recolor(open_look(alphabet, spec, bx, theme), |c| {
                    with_opacity(rgb_of(theme.muted, c), FADE)
                });
                cross(&mut out, bx, theme.bg, ink);
                out
            }
            GlyphPhase::Candidate | GlyphPhase::Final | GlyphPhase::Retracted => {
                alphabet.draw(spec, bx, theme)
            }
        }
    }

    fn legend(&self, alphabet: &str) -> &'static str {
        match alphabet {
            "rozet" => {
                "Koyu dolu para içinde açık renkli desen ve köşede ✓ gerçekleşti; soluk gri \
                 desen ve köşede kalın ✗ tutmadı; renkli ve köşe işaretsiz rozet hâlâ açık."
            }
            _ => {
                "Koyu dolu kare içinde açık renkli şekil ve köşede ✓ gerçekleşti; soluk gri \
                 şekil ve köşede kalın ✗ tutmadı; renkli ve köşe işaretsiz şekil hâlâ açık."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph::siluet::Siluet;
    use qp_schema::capsule::{Direction, MeaningKind};

    fn spec(phase: GlyphPhase) -> GlyphSpec {
        GlyphSpec {
            kind: MeaningKind::Flow,
            direction: Direction::Down,
            strength: 1.0,
            confidence: Some(0.7),
            phase,
            freshness: 1.0,
            confirmations: 5,
            server: true,
            validity: None,
        }
    }

    #[test]
    fn open_phases_are_the_alphabet_and_marks_stay_in_the_box() {
        for size in [16.0, 24.0, 32.0, 48.0] {
            let bx = GlyphBox {
                x: 2.5,
                y: 1.0,
                size,
            };
            let t = Theme::dark();
            for p in [
                GlyphPhase::Candidate,
                GlyphPhase::Final,
                GlyphPhase::Retracted,
            ] {
                assert_eq!(
                    Tg5.draw(&Siluet, &spec(p), bx, &t),
                    Siluet.draw(&spec(p), bx, &t)
                );
            }
            for p in [GlyphPhase::Realized, GlyphPhase::Failed] {
                let prims = Tg5.draw(&Siluet, &spec(p), bx, &t);
                for q in &prims {
                    let ok = match q {
                        Prim::Fill { points, .. } => points.iter().all(|[x, y]| {
                            *x >= bx.x - 0.01
                                && *y >= bx.y - 0.01
                                && *x <= bx.x + size + 0.01
                                && *y <= bx.y + size + 0.01
                        }),
                        Prim::Circle { cx, cy, r, .. } => {
                            cx - r >= bx.x - 0.01
                                && cy - r >= bx.y - 0.01
                                && cx + r <= bx.x + size + 0.01
                                && cy + r <= bx.y + size + 0.01
                        }
                        _ => true,
                    };
                    assert!(ok, "{q:?} at {size}");
                }
            }
        }
    }

    #[test]
    fn rounded_square_is_a_closed_loop_of_28_points() {
        let bx = GlyphBox {
            x: 0.0,
            y: 0.0,
            size: 10.0,
        };
        let p = rounded_square(bx, 0.0, 2.0);
        assert_eq!(p.len(), 28);
        assert_eq!(p[0], [10.0, 8.0]);
        assert_eq!(p[6], [8.0, 10.0]);
        assert_eq!(p[27], [10.0, 2.0]);
    }
}
