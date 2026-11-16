//! M11 outcome style **`muhrak`** ("seal / broken seal"): the outcome is a transformation of
//! the whole glyph, not a corner badge.
//!
//! | phase | drawing |
//! |-------|---------|
//! | open (`Candidate`, `Final`) | the alphabet's own drawing, unchanged |
//! | realized | **sealed**: the whole box becomes one solid ink plate (siluet: rounded square; rozet: disc) and the open glyph, shrunk a little, is knocked out of it in the background colour (a stamped seal / medal) |
//! | failed | **broken**: the open glyph in grey inside an empty outline (siluet: thin rounded-square frame; rozet: its own ring), split top to bottom by a jagged crack in the background colour; the half right of the crack is faded (broken off) |
//! | retracted | the alphabet's own drawing |
//!
//! Three levels that read without colour: no frame (open) / empty, cracked frame (failed) /
//! solid plate (realized). Realized ↔ failed differ over most of the box (solid ink vs
//! background), so the pair survives blur, 16 px and every colour-vision condition.
//!
//! Every colour comes from the theme (`text` for the plate and frame, `muted` for the grey,
//! `bg` for knock-outs and the crack); only IEEE basic operations; every primitive inside
//! the box (the shrink keeps the alphabet's primitives inside, the crack's ends sit one
//! half-width from the edge).

use qp_schema::capsule::{GlyphPhase, GlyphSpec};
use qp_schema::scene::{Prim, Rgba};

use super::{open_look, OutcomeStyle};
use crate::glyph::{Alphabet, GlyphBox};
use crate::theme::{with_opacity, Theme};

/// The "seal / broken seal" outcome style.
#[derive(Debug, Clone, Copy, Default)]
pub struct Muhrak;

type P = [f32; 2];

/// How much the open glyph shrinks inside the plate or frame (around the box centre).
const SHRINK: f32 = 0.8;
/// rozet shrinks further when sealed so a solid rim stays outside its knocked-out ring.
const SHRINK_ROZET: f32 = 0.74;

/// cos/sin of 0°, 22.5°, 45°, 67.5°, 90° (a quarter circle).
const QUARTER: [P; 5] = [
    [1.0, 0.0],
    [0.923_879_5, 0.382_683_43],
    [0.707_106_77, 0.707_106_77],
    [0.382_683_43, 0.923_879_5],
    [0.0, 1.0],
];

/// The crack, top to bottom, in box units (`u` across, `v` down). The ends are clamped
/// to the half-width at draw time; the first and last legs are vertical so the cut through
/// a frame is clean.
const CRACK: [P; 8] = [
    [0.64, 0.0],
    [0.64, 0.12],
    [0.5, 0.33],
    [0.62, 0.48],
    [0.42, 0.68],
    [0.5, 0.79],
    [0.38, 0.9],
    [0.38, 1.0],
];

fn same_rgb(a: Rgba, b: Rgba) -> bool {
    a[0] == b[0] && a[1] == b[1] && a[2] == b[2]
}

/// `c` with its RGB replaced by `to`'s, keeping `c`'s alpha (scaled by `to`'s), unless `c`
/// is the background colour (halos and knock-outs stay background).
fn recolor(c: Rgba, to: Rgba, bg: Rgba) -> Rgba {
    if same_rgb(c, bg) {
        return c;
    }
    #[allow(clippy::cast_possible_truncation)]
    let a = ((u16::from(c[3]) * u16::from(to[3]) + 127) / 255) as u8;
    [to[0], to[1], to[2], a]
}

/// Scales every primitive around `(cx, cy)` by `k` (points, widths, radii) and recolours
/// it with `col`. Rects (none in the glyph alphabets) are kept, recoloured.
fn transform(prims: Vec<Prim>, cx: f32, cy: f32, k: f32, col: impl Fn(Rgba) -> Rgba) -> Vec<Prim> {
    let sp = |p: P| [cx + (p[0] - cx) * k, cy + (p[1] - cy) * k];
    prims
        .into_iter()
        .map(|p| match p {
            Prim::Polyline {
                points,
                width,
                color,
            } => Prim::Polyline {
                points: points.into_iter().map(sp).collect(),
                width: width * k,
                color: col(color),
            },
            Prim::Fill { points, color } => Prim::Fill {
                points: points.into_iter().map(sp).collect(),
                color: col(color),
            },
            Prim::Circle {
                cx: x,
                cy: y,
                r,
                color,
            } => {
                let [x, y] = sp([x, y]);
                Prim::Circle {
                    cx: x,
                    cy: y,
                    r: r * k,
                    color: col(color),
                }
            }
            Prim::Rect { x, y, w, h, color } => Prim::Rect {
                x,
                y,
                w,
                h,
                color: col(color),
            },
            t @ Prim::Text { .. } => t,
        })
        .collect()
}

/// A rounded square inset by `m` with corner radius `rc` (closed outline, clockwise).
fn rounded_square(bx: GlyphBox, m: f32, rc: f32) -> Vec<P> {
    let (x0, y0) = (bx.x + m, bx.y + m);
    let (x1, y1) = (bx.x + bx.size - m, bx.y + bx.size - m);
    let mut v = Vec::with_capacity(20);
    // top-right, bottom-right, bottom-left, top-left corners
    for [c, s] in QUARTER.iter().rev() {
        v.push([x1 - rc + rc * c, y0 + rc - rc * s]);
    }
    for [c, s] in &QUARTER {
        v.push([x1 - rc + rc * c, y1 - rc + rc * s]);
    }
    for [c, s] in QUARTER.iter().rev() {
        v.push([x0 + rc - rc * c, y1 - rc + rc * s]);
    }
    for [c, s] in &QUARTER {
        v.push([x0 + rc - rc * c, y0 + rc - rc * s]);
    }
    v
}

/// Freshness opacity of the alphabet (each alphabet fades its whole drawing).
fn fresh_of(alphabet: &str, spec: &GlyphSpec) -> f64 {
    let f = if spec.freshness.is_finite() {
        spec.freshness.clamp(0.0, 1.0)
    } else {
        1.0
    };
    if alphabet == "rozet" {
        0.22 + 0.78 * f
    } else {
        0.3 + 0.7 * f
    }
}

impl Muhrak {
    fn realized(alphabet: &dyn Alphabet, spec: &GlyphSpec, bx: GlyphBox, t: &Theme) -> Vec<Prim> {
        let s = bx.size;
        let (cx, cy) = (bx.x + 0.5 * s, bx.y + 0.5 * s);
        let rozet = alphabet.name() == "rozet";
        let plate = with_opacity(t.text, fresh_of(alphabet.name(), spec));
        let mut out = Vec::new();
        if rozet {
            out.push(Prim::Circle {
                cx,
                cy,
                r: 0.5 * s - 0.2,
                color: plate,
            });
        } else {
            out.push(Prim::Fill {
                points: rounded_square(bx, 0.02 * s, 0.2 * s),
                color: plate,
            });
        }
        let bg = t.bg;
        out.extend(transform(
            open_look(alphabet, spec, bx, t),
            cx,
            cy,
            if rozet { SHRINK_ROZET } else { SHRINK },
            |c| recolor(c, bg, bg),
        ));
        out
    }

    fn failed(alphabet: &dyn Alphabet, spec: &GlyphSpec, bx: GlyphBox, t: &Theme) -> Vec<Prim> {
        let s = bx.size;
        let (cx, cy) = (bx.x + 0.5 * s, bx.y + 0.5 * s);
        let rozet = alphabet.name() == "rozet";
        let fresh = fresh_of(alphabet.name(), spec);
        let (muted, bg) = (t.muted, t.bg);
        // rozet keeps its own ring at full size (it is the frame); siluet shrinks into a
        // thin frame of its own.
        let k = if rozet { 1.0 } else { SHRINK };
        let mut out = transform(open_look(alphabet, spec, bx, t), cx, cy, k, |c| {
            recolor(c, muted, bg)
        });
        if !rozet {
            let fw = (0.06 * s).max(1.0);
            let mut ring = rounded_square(bx, 0.02 * s + 0.5 * fw, 0.2 * s);
            // start and end mid top edge (butt caps meet flush)
            let mid = [cx, ring[0][1]];
            ring.insert(0, mid);
            ring.push(mid);
            out.push(Prim::Polyline {
                points: ring,
                width: fw,
                color: with_opacity(t.text, fresh),
            });
        }
        // The crack: background stroke from top to bottom, ends kept inside the box.
        let hw = (0.065 * s).max(0.9).min(0.5 * s);
        let crack: Vec<P> = CRACK
            .iter()
            .map(|[u, v]| {
                [
                    bx.x + u * s,
                    (bx.y + v * s).max(bx.y + hw).min(bx.y + s - hw),
                ]
            })
            .collect();
        // Broken off: fade everything right of the crack.
        let mut ghost = crack.clone();
        ghost[0][1] = bx.y;
        let last = ghost.len() - 1;
        ghost[last][1] = bx.y + s;
        ghost.push([bx.x + s, bx.y + s]);
        ghost.push([bx.x + s, bx.y]);
        out.push(Prim::Fill {
            points: ghost,
            color: with_opacity(bg, 0.72),
        });
        out.push(Prim::Polyline {
            points: crack,
            width: 2.0 * hw,
            color: bg,
        });
        out
    }
}

impl OutcomeStyle for Muhrak {
    fn name(&self) -> &'static str {
        "muhrak"
    }

    fn draw(
        &self,
        alphabet: &dyn Alphabet,
        spec: &GlyphSpec,
        bx: GlyphBox,
        theme: &Theme,
    ) -> Vec<Prim> {
        match spec.phase {
            GlyphPhase::Realized => Self::realized(alphabet, spec, bx, theme),
            GlyphPhase::Failed => Self::failed(alphabet, spec, bx, theme),
            GlyphPhase::Candidate | GlyphPhase::Final | GlyphPhase::Retracted => {
                alphabet.draw(spec, bx, theme)
            }
        }
    }

    fn legend(&self, alphabet: &str) -> &'static str {
        match alphabet {
            "rozet" => {
                "Tamamen koyu dolu, deseni içine oyulmuş rozet gerçekleşti demek; gri, \
                 çatlakla ikiye bölünmüş ve bir yarısı silik rozet tutmadı demek; renkli \
                 normal rozet hâlâ açık."
            }
            _ => {
                "Koyu dolu kareye oyulmuş işaret gerçekleşti demek; gri, boş çerçeveli, \
                 çatlakla ikiye bölünmüş ve bir yarısı silik işaret tutmadı demek; renkli, \
                 çerçevesiz işaret hâlâ açık."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph::siluet::Siluet;
    use qp_schema::capsule::{Direction, MeaningKind};

    fn inside(prims: &[Prim], bx: GlyphBox) -> bool {
        let (x0, y0, x1, y1) = (bx.x, bx.y, bx.x + bx.size, bx.y + bx.size);
        let tol = 0.01;
        let ins = |x: f32, y: f32| x >= x0 - tol && y >= y0 - tol && x <= x1 + tol && y <= y1 + tol;
        prims.iter().all(|p| match p {
            Prim::Polyline { points, width, .. } => {
                let h = width / 2.0;
                points
                    .iter()
                    .all(|[x, y]| ins(x - h, y - h) && ins(x + h, y + h))
            }
            Prim::Fill { points, .. } => points.iter().all(|[x, y]| ins(*x, *y)),
            Prim::Circle { cx, cy, r, .. } => ins(cx - r, cy - r) && ins(cx + r, cy + r),
            Prim::Rect { .. } | Prim::Text { .. } => true,
        })
    }

    #[test]
    fn open_is_the_alphabet_and_closed_is_inside() {
        for theme in [Theme::light(), Theme::dark()] {
            for kind in MeaningKind::ALL {
                for direction in [Direction::Up, Direction::Down, Direction::Neutral] {
                    for phase in [
                        GlyphPhase::Candidate,
                        GlyphPhase::Final,
                        GlyphPhase::Realized,
                        GlyphPhase::Failed,
                        GlyphPhase::Retracted,
                    ] {
                        for size in [16.0, 24.0, 32.0, 48.0] {
                            let bx = GlyphBox {
                                x: 3.5,
                                y: 1.0,
                                size,
                            };
                            let spec = GlyphSpec {
                                kind,
                                direction,
                                strength: 1.0,
                                confidence: Some(0.7),
                                phase,
                                freshness: 1.0,
                                confirmations: 5,
                                server: true,
                                validity: None,
                            };
                            let p = Muhrak.draw(&Siluet, &spec, bx, &theme);
                            assert!(inside(&p, bx), "{spec:?} {size}");
                            assert_eq!(p, Muhrak.draw(&Siluet, &spec, bx, &theme));
                            if !matches!(phase, GlyphPhase::Realized | GlyphPhase::Failed) {
                                assert_eq!(p, Siluet.draw(&spec, bx, &theme));
                            }
                        }
                    }
                }
            }
        }
    }
}
