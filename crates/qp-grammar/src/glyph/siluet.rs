//! The product meaning-glyph alphabet (R3/T7): **"siluet"**, chosen in R3/M1
//! (`rounds/R3/experiments/M1/RESULTS.md`). Moved here unchanged from the experiment crate
//! `rounds/R3/experiments/M1/siluet/` (only the type is renamed `Alphabet` → [`Siluet`]);
//! `tests/glyph_alphabet.rs` pins a digest of every primitive over the full M1 glyph set,
//! equal to the experiment crate's (`rounds/R3/experiments/S/S-B.md`).
//!
//! Every meaning kind is its own familiar silhouette, and
//! direction is the silhouette's orientation. The other channels are kept plain and
//! conventional so that the silhouette carries the kind alone.
//!
//! | channel | encoding |
//! |---------|----------|
//! | kind | silhouette: continuation = straight diagonal arrow; reversal = U-turn arrow; breakout = arrow crossing a bar; squeeze = two converging lines (triangle); extreme = spike with a dot on its tip; flow = two wavy lines under one wide arrowhead |
//! | direction | orientation (up / down mirror, neutral = sideways or symmetric); colour `up` / `down` / `accent` repeats it |
//! | strength | silhouette size and stroke weight |
//! | confidence | a pale plate filled from the bottom up to the confidence level; none = no plate |
//! | phase | candidate = dashed strokes, hollow arrowhead; final = solid; closed = grey silhouette with a corner badge: realized ✓, failed ✗; retracted = grey and struck through |
//! | validity (R4, K42/A28) | only `no_longer` (open story, state condition broke): grey silhouette and a hollow ring with a bar in the corner badge; absent / `valid` / `closed` draw as above |
//! | freshness | opacity |
//! | confirmations | 1–5 short ticks along the bottom edge |
//! | server | a solid folded corner (dog-ear) at the top right |
//!
//! Determinism: IEEE basic operations and `sqrt` (correctly rounded by IEEE 754 on every
//! target) only; the circle and sine samples are precomputed constants.

use super::{Alphabet as AlphabetTrait, GlyphBox};
use crate::theme::{at_least, opacity_at_least, with_opacity, Theme};
use qp_schema::capsule::{Direction, GlyphPhase, GlyphSpec, MeaningKind, Validity};
use qp_schema::scene::{Prim, Rgba};

mod shapes;

use shapes::{dashes, shape};

/// The "siluet" alphabet.
#[derive(Debug, Clone, Copy, Default)]
pub struct Siluet;

/// Legend for the `legend` reading condition (Turkish, ≤ 120 words).
pub const LEGEND: &str = "Şekil türü söyler. Devam: düz çapraz ok. Dönüş: U biçiminde \
geri dönen ok. Kırılım: yatay çizgiyi delip geçen ok. Sıkışma: birbirine yaklaşan iki çizgi. \
Aşırılık: ucunda nokta olan sivri tepe. Akış: geniş ok başı altında iki dalgalı çizgi. Yön: \
şeklin baktığı yer; yukarı yeşil, aşağı kırmızı, yönsüz turuncu (yan yatık ya da simetrik). \
Güç: büyük ve kalın güçlü, küçük ve ince zayıf. Aşama: kesik çizgi ve boş ok başı aday; dolu \
renkli çizgi kesinleşmiş; gri şekil kapanmış: sağ altta ✓ gerçekleşti, ✗ gerçekleşmedi, \
üstü çizili geri çekildi. Zemindeki soluk dolgunun yüksekliği güven. Solukluk eskilik. \
Alttaki kısa çizgiler teyit eden kanıt ailesi sayısı. Sağ üstte koyu kıvrık köşe: sunucu \
doğruladı.";

type P = [f32; 2];

/// cos/sin of 180°, 165°, …, 0° (13 samples of a half circle).
const HALF_CIRCLE: [P; 13] = [
    [-1.0, 0.0],
    [-0.965_925_8, 0.258_819_04],
    [-0.866_025_4, 0.5],
    [-0.707_106_77, 0.707_106_77],
    [-0.5, 0.866_025_4],
    [-0.258_819_04, 0.965_925_8],
    [0.0, 1.0],
    [0.258_819_04, 0.965_925_8],
    [0.5, 0.866_025_4],
    [0.707_106_77, 0.707_106_77],
    [0.866_025_4, 0.5],
    [0.965_925_8, 0.258_819_04],
    [1.0, 0.0],
];

/// sin(2πk/16), k = 0..16.
const SINE16: [f32; 16] = [
    0.0,
    0.382_683_43,
    0.707_106_77,
    0.923_879_5,
    1.0,
    0.923_879_5,
    0.707_106_77,
    0.382_683_43,
    0.0,
    -0.382_683_43,
    -0.707_106_77,
    -0.923_879_5,
    -1.0,
    -0.923_879_5,
    -0.707_106_77,
    -0.382_683_43,
];

const AMP: f32 = 0.13;
const AMP2: f32 = 0.13;
const DIAG: f32 = 0.707_106_77;

/// Which colour a stroke takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ink {
    /// The silhouette colour (direction colour, grey when closed).
    Main,
    /// A reference line (the breakout bar): text colour, grey when closed.
    Edge,
}

/// A filled triangular arrowhead: `tip`, unit direction `dir` pointing at the tip,
/// length `len` and half base width `half` (normalized units).
#[derive(Debug, Clone, Copy)]
struct Head {
    tip: P,
    dir: P,
    len: f32,
    half: f32,
}

/// A silhouette in normalized coordinates (`[0, 1]²`, y down), already oriented.
#[derive(Debug, Default)]
struct Shape {
    strokes: Vec<(Vec<P>, Ink)>,
    heads: Vec<Head>,
    dots: Vec<(P, f32)>,
}

/// Orientation transform of a point: up = as drawn, down = mirrored top/bottom,
/// neutral = turned a quarter clockwise (what pointed up points right).
fn orient(d: Direction, p: P) -> P {
    match d {
        Direction::Up => p,
        Direction::Down => [p[0], 1.0 - p[1]],
        Direction::Neutral => [1.0 - p[1], p[0]],
    }
}

/// Orientation transform of a direction vector.
fn orient_v(d: Direction, v: P) -> P {
    match d {
        Direction::Up => v,
        Direction::Down => [v[0], -v[1]],
        Direction::Neutral => [-v[1], v[0]],
    }
}

impl Shape {
    /// Orients every element of an "up" silhouette.
    fn oriented(mut self, d: Direction) -> Self {
        for (pts, _) in &mut self.strokes {
            for p in pts.iter_mut() {
                *p = orient(d, *p);
            }
        }
        for h in &mut self.heads {
            h.tip = orient(d, h.tip);
            h.dir = orient_v(d, h.dir);
        }
        for (c, _) in &mut self.dots {
            *c = orient(d, *c);
        }
        self
    }
}

fn is_closed(p: GlyphPhase) -> bool {
    matches!(
        p,
        GlyphPhase::Realized | GlyphPhase::Failed | GlyphPhase::Retracted
    )
}

impl AlphabetTrait for Siluet {
    fn name(&self) -> &'static str {
        "siluet"
    }

    #[allow(clippy::too_many_lines)]
    fn draw(&self, spec: &GlyphSpec, bx: GlyphBox, theme: &Theme) -> Vec<Prim> {
        let s = bx.size;
        #[allow(clippy::cast_possible_truncation)]
        let st = spec.strength.clamp(0.0, 1.0) as f32;
        let closed = is_closed(spec.phase);
        // (R5/T3c) An open glyph fades with its freshness down to the look's floor
        // (`r4`: 0, freshness alone).
        let fresh = 0.3 + 0.7 * spec.freshness.clamp(0.0, 1.0);
        let fresh = if closed {
            fresh
        } else {
            opacity_at_least(fresh, theme.fresh_alpha_min)
        };
        let candidate = spec.phase == GlyphPhase::Candidate;
        // (R4, K42/A28) "No longer holds": only an open story whose state condition broke.
        // Absent / valid / closed validity draws exactly as in R3.
        let no_longer = !closed && spec.validity == Some(Validity::NoLonger);
        let grey = closed || no_longer;

        let dir_col = match spec.direction {
            Direction::Up => theme.up,
            Direction::Down => theme.down,
            Direction::Neutral => theme.neutral,
        };
        // (R5/T3) Grey: the look's closed colour for a closed story, its invalid colour for
        // one that no longer holds (`r4`: both `muted`).
        let grey_col = if closed { theme.closed } else { theme.invalid };
        let main: Rgba = with_opacity(if grey { grey_col } else { dir_col }, fresh);
        let edge: Rgba = with_opacity(if grey { grey_col } else { theme.text }, fresh);
        // (R5/T3b) A closed glyph's strokes keep the look's least alpha (`r4`: 0).
        let (main, edge) = if closed {
            (
                at_least(main, theme.closed_alpha),
                at_least(edge, theme.closed_alpha),
            )
        } else {
            (main, edge)
        };
        let ink = with_opacity(theme.text, fresh);
        // (R5/T3c) So do a closed glyph's outcome marks and confirmation blocks.
        let ink = if closed {
            at_least(ink, theme.closed_alpha)
        } else {
            ink
        };
        let mut out = Vec::new();

        // Confidence: pale plate filled from the bottom up to the confidence level.
        if let Some(c) = spec.confidence {
            #[allow(clippy::cast_possible_truncation)]
            let c = c.clamp(0.0, 1.0) as f32;
            if c > 0.0 {
                let top = bx.y + s * (1.0 - c);
                let tint = if grey { grey_col } else { dir_col };
                // (R5/T3c) The plate keeps the look's least opacity (`r4`: 0); (R5/fix47) a
                // grey plate (grey strokes on it) at most the look's grey ceiling, so its
                // strokes keep 3:1 against it.
                let mut o = opacity_at_least(0.24 * fresh, theme.plate_alpha_min);
                if theme.grey_plate_alpha_max > 0 && (grey || spec.direction == Direction::Neutral)
                {
                    o = o.min(f64::from(theme.grey_plate_alpha_max) / 255.0);
                }
                out.push(Prim::Fill {
                    points: vec![
                        [bx.x, top],
                        [bx.x + s, top],
                        [bx.x + s, bx.y + s],
                        [bx.x, bx.y + s],
                    ],
                    color: with_opacity(tint, o),
                });
            }
        }

        // Silhouette: size and weight follow strength.
        let r = s * 0.75 * (0.72 + 0.35 * st).min(1.0);
        let (cx, cy) = (bx.x + 0.5 * s, bx.y + 0.43 * s);
        let map = |p: P| [cx + (p[0] - 0.5) * r, cy + (p[1] - 0.5) * r];
        let w = (s * (0.05 + 0.1 * st)).max(1.1);
        let sh = shape(spec.kind, spec.direction);
        let (on, off) = ((0.16 * s).max(2.8), (0.12 * s).max(2.0));
        for (pts, which) in &sh.strokes {
            let color = if *which == Ink::Main { main } else { edge };
            let px: Vec<P> = pts.iter().map(|p| map(*p)).collect();
            if candidate {
                for dash in dashes(&px, on, off) {
                    out.push(Prim::Polyline {
                        points: dash,
                        width: w,
                        color,
                    });
                }
            } else {
                out.push(Prim::Polyline {
                    points: px,
                    width: w,
                    color,
                });
            }
        }
        for h in &sh.heads {
            let base = back(h, 1.0);
            let perp = [-h.dir[1], h.dir[0]];
            let tri = vec![
                map(h.tip),
                map([base[0] + perp[0] * h.half, base[1] + perp[1] * h.half]),
                map([base[0] - perp[0] * h.half, base[1] - perp[1] * h.half]),
            ];
            if candidate {
                // Hollow head: the outline on the solid head's edge, kept inside the box.
                let ow = (w * 0.8).max(1.0);
                let m = ow * 0.5;
                let mut ring: Vec<P> = tri
                    .iter()
                    .map(|v| {
                        [
                            v[0].clamp(bx.x + m, bx.x + s - m),
                            v[1].clamp(bx.y + m, bx.y + s - m),
                        ]
                    })
                    .collect();
                ring.push(ring[0]);
                out.push(Prim::Polyline {
                    points: ring,
                    width: ow,
                    color: main,
                });
            } else {
                out.push(Prim::Fill {
                    points: tri,
                    color: main,
                });
            }
        }
        for (c, rad) in &sh.dots {
            let [x, y] = map(*c);
            let rr = (rad * r).max(1.2);
            out.push(Prim::Circle {
                cx: x,
                cy: y,
                r: rr,
                color: main,
            });
            if candidate {
                out.push(Prim::Circle {
                    cx: x,
                    cy: y,
                    r: (rr - w * 0.8).max(0.4),
                    color: theme.bg,
                });
            }
        }

        // Confirmations: one small block per evidence family along the bottom edge.
        let n = spec.confirmations.min(5);
        let (bw_, bh) = (0.12 * s, 0.17 * s);
        for i in 0..n {
            let x = bx.x + s * (0.03 + 0.155 * f32::from(i));
            let y = bx.y + s * 0.81;
            out.push(Prim::Fill {
                points: vec![[x, y], [x + bw_, y], [x + bw_, y + bh], [x, y + bh]],
                color: ink,
            });
        }

        // Closed outcome: realized = solid disc with a check; failed = a bare cross.
        let bw = (0.08 * s).max(1.2);
        match spec.phase {
            GlyphPhase::Realized | GlyphPhase::Failed => {
                let (bcx, bcy, br) = (bx.x + 0.83 * s, bx.y + 0.83 * s, 0.165 * s);
                let q = |u: f32, v: f32| [bx.x + u * s, bx.y + v * s];
                out.push(Prim::Circle {
                    cx: bcx,
                    cy: bcy,
                    r: br,
                    color: if spec.phase == GlyphPhase::Realized {
                        ink
                    } else {
                        theme.bg
                    },
                });
                if spec.phase == GlyphPhase::Realized {
                    out.push(Prim::Polyline {
                        points: vec![q(0.745, 0.835), q(0.805, 0.895), q(0.915, 0.765)],
                        width: (0.06 * s).max(1.0),
                        color: theme.bg,
                    });
                } else {
                    let k = 0.1;
                    for (p0, p1) in [
                        (q(0.83 - k, 0.83 - k), q(0.83 + k, 0.83 + k)),
                        (q(0.83 + k, 0.83 - k), q(0.83 - k, 0.83 + k)),
                    ] {
                        out.push(Prim::Polyline {
                            points: vec![p0, p1],
                            width: bw,
                            color: ink,
                        });
                    }
                }
            }
            GlyphPhase::Retracted => {
                out.push(Prim::Polyline {
                    points: vec![
                        [bx.x + 0.04 * s, bx.y + 0.42 * s],
                        [bx.x + 0.96 * s, bx.y + 0.42 * s],
                    ],
                    width: bw,
                    color: ink,
                });
            }
            GlyphPhase::Candidate | GlyphPhase::Final if no_longer => {
                // No longer holds: grey silhouette and a hollow ring with a bar (a "no
                // longer" sign) in the outcome corner: neither the solid ✓ disc nor the
                // bare ✗, and readable without colour.
                let (bcx, bcy, br) = (bx.x + 0.83 * s, bx.y + 0.83 * s, 0.165 * s);
                let q = |u: f32, v: f32| [bx.x + u * s, bx.y + v * s];
                out.push(Prim::Circle {
                    cx: bcx,
                    cy: bcy,
                    r: br,
                    color: ink,
                });
                out.push(Prim::Circle {
                    cx: bcx,
                    cy: bcy,
                    r: (br - bw).max(0.4),
                    color: theme.bg,
                });
                out.push(Prim::Polyline {
                    points: vec![q(0.83 - 0.095, 0.83), q(0.83 + 0.095, 0.83)],
                    width: bw,
                    color: ink,
                });
            }
            GlyphPhase::Candidate | GlyphPhase::Final => {}
        }

        // Server-verified: a solid folded corner at the top right.
        if spec.server {
            out.push(Prim::Fill {
                points: vec![
                    [bx.x + 0.72 * s, bx.y],
                    [bx.x + s, bx.y],
                    [bx.x + s, bx.y + 0.28 * s],
                ],
                color: ink,
            });
        }
        out
    }
}

/// A point `k` units back from the tip of `h` (where its shaft should end, inside the head).
fn back(h: &Head, k: f32) -> P {
    [
        h.tip[0] - h.dir[0] * h.len * k,
        h.tip[1] - h.dir[1] * h.len * k,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINDS: [MeaningKind; 6] = MeaningKind::ALL;
    const DIRS: [Direction; 3] = [Direction::Up, Direction::Down, Direction::Neutral];
    const PHASES: [GlyphPhase; 5] = [
        GlyphPhase::Candidate,
        GlyphPhase::Final,
        GlyphPhase::Realized,
        GlyphPhase::Failed,
        GlyphPhase::Retracted,
    ];

    fn all_specs() -> Vec<GlyphSpec> {
        let mut v = Vec::new();
        for kind in KINDS {
            for direction in DIRS {
                for phase in PHASES {
                    for strength in [0.0, 0.2, 0.6, 1.0] {
                        for confidence in [None, Some(0.0), Some(0.3), Some(1.0)] {
                            for confirmations in 1..=5 {
                                for (freshness, server) in [(1.0, false), (0.0, true)] {
                                    v.push(GlyphSpec {
                                        kind,
                                        direction,
                                        strength,
                                        confidence,
                                        phase,
                                        freshness,
                                        confirmations,
                                        server,
                                        validity: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        v
    }

    fn eps_inside(prims: &[Prim], bx: GlyphBox) -> bool {
        let (x0, y0, x1, y1) = (bx.x, bx.y, bx.x + bx.size, bx.y + bx.size);
        let tol = 0.01;
        let inside =
            |x: f32, y: f32| x >= x0 - tol && y >= y0 - tol && x <= x1 + tol && y <= y1 + tol;
        prims.iter().all(|p| match p {
            #[allow(clippy::cast_precision_loss)]
            Prim::Rect { x, y, w, h, .. } => {
                inside(*x as f32, *y as f32)
                    && inside((*x as f32) + *w as f32, (*y as f32) + *h as f32)
            }
            Prim::Polyline { points, width, .. } => {
                let hw = width / 2.0;
                points
                    .iter()
                    .all(|[x, y]| inside(x - hw, y - hw) && inside(x + hw, y + hw))
            }
            Prim::Fill { points, .. } => points.iter().all(|[x, y]| inside(*x, *y)),
            Prim::Circle { cx, cy, r, .. } => inside(cx - r, cy - r) && inside(cx + r, cy + r),
            Prim::Text { .. } => true,
        })
    }

    #[test]
    fn name_and_legend() {
        assert_eq!(Siluet.name(), "siluet");
        assert!(LEGEND.split_whitespace().count() <= 120);
    }

    #[test]
    fn deterministic() {
        for spec in all_specs().iter().step_by(7) {
            for size in [16.0, 24.0, 32.0] {
                let bx = GlyphBox {
                    x: 3.0,
                    y: 5.0,
                    size,
                };
                let a = Siluet.draw(spec, bx, &Theme::light());
                let b = Siluet.draw(spec, bx, &Theme::light());
                assert_eq!(a, b);
            }
        }
    }

    #[test]
    fn inside_box_and_non_empty() {
        for theme in [Theme::light(), Theme::dark()] {
            for spec in all_specs() {
                for size in [16.0, 20.0, 24.0, 32.0, 48.0, 64.0] {
                    for (x, y) in [(0.0, 0.0), (10.5, 7.25)] {
                        let bx = GlyphBox { x, y, size };
                        let p = Siluet.draw(&spec, bx, &theme);
                        assert!(!p.is_empty(), "{spec:?}");
                        assert!(eps_inside(&p, bx), "{spec:?} at {size}");
                        // The silhouette itself is never empty.
                        assert!(
                            p.iter().any(|q| matches!(q, Prim::Polyline { .. })),
                            "{spec:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn dashes_cover_part_of_line() {
        let d = dashes(&[[0.0, 0.0], [10.0, 0.0]], 3.0, 2.0);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0], vec![[0.0, 0.0], [3.0, 0.0]]);
        assert_eq!(d[1], vec![[5.0, 0.0], [8.0, 0.0]]);
    }

    /// (R5/T3c) The confidence plate's floor: `r4` fades it with freshness alone
    /// (0.24 · 0.3 at freshness 0), the trial look keeps a fresh glyph's plate (0.24) at
    /// every freshness; the rozet's level likewise never goes below the floor.
    #[test]
    fn confidence_plate_keeps_the_look_floor() {
        use crate::glyph::rozet::Rozet;
        use crate::look::{ORNEK, R4};
        let bx = GlyphBox {
            x: 0.0,
            y: 0.0,
            size: 32.0,
        };
        let spec = |freshness: f64| GlyphSpec {
            kind: MeaningKind::Flow,
            direction: Direction::Down,
            strength: 0.2,
            confidence: Some(0.6),
            phase: GlyphPhase::Final,
            freshness,
            confirmations: 1,
            server: false,
            validity: None,
        };
        let plate = |t: &Theme, f: f64| match &Siluet.draw(&spec(f), bx, t)[0] {
            Prim::Fill { color, .. } => *color,
            p => panic!("{p:?}"),
        };
        for t in [R4.dark, R4.light] {
            assert_eq!(t.plate_alpha_min, 0);
            assert_eq!(plate(&t, 0.0), with_opacity(t.down, 0.24 * 0.3));
        }
        for t in [ORNEK.dark, ORNEK.light] {
            assert!(t.plate_alpha_min > 0);
            assert_eq!(plate(&t, 0.0), plate(&t, 1.0), "{}", t.name);
            assert!(plate(&t, 0.0)[3] >= t.plate_alpha_min);
            let level = Rozet
                .draw(&spec(0.0), bx, &t)
                .iter()
                .find_map(|p| match p {
                    Prim::Fill { color, .. } if color[..3] == t.muted[..3] => Some(*color),
                    _ => None,
                })
                .unwrap();
            assert!(level[3] >= t.plate_alpha_min, "{}: {level:?}", t.name);
        }
    }
}
