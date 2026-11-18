//! R3/M1 candidate alphabet **"rozet"** (badge), the human test's other finalist (R4/T5:
//! moved here unchanged from `rounds/R3/experiments/M1/rozet/src/lib.rs` so that the
//! demo can switch alphabets; only the type is renamed `Alphabet` → [`Rozet`]). It has no
//! `validity` channel (R4/T3 added that to `siluet` only).
//!
//! Every glyph is the same circular badge: one fixed outer contour at every size and in
//! every state, so a row of glyphs reads as a row of equal coins and only the inside
//! changes. Channels (`docs/CAPSULE.md` §4, `GlyphSpec`):
//!
//! | channel | encoding |
//! |---|---|
//! | kind | the inner pattern: double chevron, opposed arrows, arrow through a broken line, two facing triangles, dot beyond a line, drop |
//! | direction | rotation of the motif: nose up = up, down = down, sideways (right) = neutral; colour (`up`/`down`/`text`) repeats it |
//! | strength | size and weight of the motif inside the fixed badge |
//! | confidence | a grey fill level rising from the bottom of the badge; none = empty badge |
//! | phase | candidate: hollow motif + thin grey ring; final: solid; realized: heavy bezel; failed: one slash; retracted: everything grey + a cross |
//! | freshness | opacity of the whole glyph |
//! | confirmations | the ring is cut into 1–5 equal segments |
//! | server | a small diamond notch at the top-right corner |
//!
//! Determinism: only IEEE basic operations (`+ - * /`, `sqrt`, `floor`, `round`);
//! sine/cosine come from a fixed polynomial ([`sincos_deg`]), never from `std`.
//! All colours come from the [`Theme`].

use super::{Alphabet as AlphabetTrait, GlyphBox};
use crate::theme::{opacity_at_least, Theme};
use qp_schema::capsule::{Direction, GlyphPhase, GlyphSpec, MeaningKind};
use qp_schema::scene::{Prim, Rgba};

/// The legend shown to readers in the `legend` condition (Turkish, ≤ 120 words).
pub const LEGEND: &str = "Her işaret aynı yuvarlak rozettir. İç desen türü söyler: iki ok başı = devam; küçük aşağı ve büyük yukarı iki ok = dönüş; açık çizgiyi delen ok = kırılım; birbirine dönük iki üçgen = sıkışma; çizginin ötesindeki nokta = aşırılık; damla = akış. Desenin ucu yönü gösterir: yukarı (yeşil), aşağı (kırmızı), yana = yönsüz (koyu). Büyük desen güçlü, küçük desen zayıf. Aşama: içi boş desen ve ince gri halka = aday; dolu desen = kesin; kalın çerçeve = gerçekleşti; eğik çizgiyle üstü çizili = gerçekleşmedi; tümü gri ve çarpılı = geri çekildi. Halkanın parça sayısı = bağımsız teyit sayısı. Alttan yükselen gri dolgu = güven. Soluk = eski. Sağ üst köşede elmas = sunucu doğruladı.";

/// The "rozet" alphabet.
#[derive(Debug, Clone, Copy, Default)]
pub struct Rozet;

type Pt = [f64; 2];

// ---------------------------------------------------------------------------------------
// Deterministic trigonometry.

/// `(sin, cos)` of an angle in degrees, from IEEE basic operations only: reduction to
/// `[-45°, 45°]` by quadrant, then Taylor polynomials to degree 15/16 (error < 1e-13).
#[must_use]
pub fn sincos_deg(deg: f64) -> (f64, f64) {
    let q = (deg / 90.0).round();
    let r = deg - 90.0 * q;
    let x = r * (core::f64::consts::PI / 180.0);
    let x2 = x * x;
    // sin x = x - x^3/3! + ... ; cos x = 1 - x^2/2! + ...
    let mut s = 0.0;
    let mut c = 0.0;
    let mut ts = x; // x^(2k+1)/(2k+1)!
    let mut tc = 1.0; // x^(2k)/(2k)!
    for k in 0..8 {
        let kf = f64::from(k);
        if k % 2 == 0 {
            s += ts;
            c += tc;
        } else {
            s -= ts;
            c -= tc;
        }
        ts = ts * x2 / ((2.0 * kf + 2.0) * (2.0 * kf + 3.0));
        tc = tc * x2 / ((2.0 * kf + 1.0) * (2.0 * kf + 2.0));
    }
    // rotate by q quadrants: angle = r + 90 q
    let qm = q - 4.0 * (q / 4.0).floor();
    #[allow(clippy::float_cmp)]
    if qm == 0.0 {
        (s, c)
    } else if qm == 1.0 {
        (c, -s)
    } else if qm == 2.0 {
        (-s, -c)
    } else {
        (-c, s)
    }
}

/// Point on a circle; `deg` measured clockwise on screen from the +x axis.
fn on_circle(c: Pt, r: f64, deg: f64) -> Pt {
    let (s, co) = sincos_deg(deg);
    [c[0] + r * co, c[1] + r * s]
}

/// Points of an arc from `a0` to `a1` degrees (inclusive), at most `step` degrees apart.
fn arc(c: Pt, r: f64, a0: f64, a1: f64, step: f64) -> Vec<Pt> {
    let span = a1 - a0;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = ((span.abs() / step).ceil() as usize).max(1);
    #[allow(clippy::cast_precision_loss)]
    (0..=n)
        .map(|i| on_circle(c, r, a0 + span * (i as f64) / (n as f64)))
        .collect()
}

// ---------------------------------------------------------------------------------------
// Motifs, in unit coordinates: "up" orientation, y grows downwards, all inside the unit
// circle (the motif radius `R` scales them).

fn chevron(a: f64, w: f64, h: f64, t: f64) -> Vec<Pt> {
    vec![
        [0.0, a],
        [w, a + h],
        [w, a + h + t],
        [0.0, a + t],
        [-w, a + h + t],
        [-w, a + h],
    ]
}

/// Up-pointing arrow: tip at `(xc, tip)`, head half-width `hw`, head height `hh`, stem
/// half-width `sw`, tail at `tail`.
fn arrow_up(xc: f64, tip: f64, tail: f64, hw: f64, hh: f64, sw: f64) -> Vec<Pt> {
    let b = tip + hh;
    vec![
        [xc, tip],
        [xc + hw, b],
        [xc + sw, b],
        [xc + sw, tail],
        [xc - sw, tail],
        [xc - sw, b],
        [xc - hw, b],
    ]
}

fn flip_y(p: Vec<Pt>) -> Vec<Pt> {
    // Mirror in y (keeps orientation consistent for filling: winding flips, non-zero ok).
    p.into_iter().map(|[x, y]| [x, -y]).collect()
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Pt> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

fn disc(cx: f64, cy: f64, r: f64) -> Vec<Pt> {
    let mut v = arc([cx, cy], r, 0.0, 360.0, 12.0);
    v.pop(); // last == first
    v
}

/// A drop (money as liquid): tip at the top, round belly below.
fn drop_shape() -> Vec<Pt> {
    // belly centre (0, 0.32), radius 0.58, tip (0, -0.92): the tangent points lie
    // 62.1° either side of the centre→tip direction (cos β = 0.58 / 1.24).
    let mut v = vec![[0.0, -0.92]];
    v.extend(arc([0.0, 0.32], 0.58, -27.9, 207.9, 12.0));
    v
}

/// The motif of `kind` in its "up" orientation (squeeze: in its neutral orientation when
/// `neutral_native` is true).
fn motif(kind: MeaningKind, neutral_native: bool) -> Vec<Vec<Pt>> {
    match kind {
        MeaningKind::Continuation => vec![
            chevron(-0.88, 0.70, 0.50, 0.36),
            chevron(-0.18, 0.70, 0.50, 0.36),
        ],
        MeaningKind::Reversal => vec![
            // big up arrow on the right, small down arrow on the left
            arrow_up(0.30, -0.90, 0.80, 0.50, 0.55, 0.17),
            flip_y(arrow_up(-0.44, -0.62, 0.40, 0.34, 0.40, 0.13)),
        ],
        MeaningKind::Breakout => vec![
            arrow_up(0.0, -0.92, 0.85, 0.56, 0.60, 0.17),
            rect(-0.86, 0.20, -0.36, 0.46),
            rect(0.36, 0.20, 0.86, 0.46),
        ],
        MeaningKind::Squeeze => {
            let left = vec![[-0.86, -0.56], [-0.10, 0.0], [-0.86, 0.56]];
            let right = vec![[0.86, -0.56], [0.10, 0.0], [0.86, 0.56]];
            if neutral_native {
                vec![left, right]
            } else {
                let sh = |p: Vec<Pt>| -> Vec<Pt> {
                    p.into_iter().map(|[x, y]| [x, y * 0.8 + 0.22]).collect()
                };
                vec![
                    sh(left),
                    sh(right),
                    vec![[0.0, -0.95], [0.30, -0.55], [-0.30, -0.55]],
                ]
            }
        }
        MeaningKind::Extreme => vec![disc(0.0, -0.58, 0.37), rect(-0.92, 0.0, 0.92, 0.22)],
        MeaningKind::Flow => vec![drop_shape()],
    }
}

/// Rotation for a direction: up = identity, down = 180°, neutral = 90° clockwise (the
/// nose points right).
fn orient(p: Pt, d: Direction, kind: MeaningKind) -> Pt {
    let [x, y] = p;
    match (d, kind) {
        (Direction::Up, _) | (Direction::Neutral, MeaningKind::Squeeze) => [x, y],
        (Direction::Down, _) => [-x, -y],
        (Direction::Neutral, _) => [-y, x],
    }
}

// ---------------------------------------------------------------------------------------
// Drawing helpers.

fn with_alpha(c: Rgba, a: f64) -> Rgba {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let al = (f64::from(c[3]) * a.clamp(0.0, 1.0)).round() as u8;
    [c[0], c[1], c[2], al]
}

fn f32p(p: &[Pt]) -> Vec<[f32; 2]> {
    #[allow(clippy::cast_possible_truncation)]
    p.iter().map(|[x, y]| [*x as f32, *y as f32]).collect()
}

fn fill(out: &mut Vec<Prim>, p: &[Pt], color: Rgba) {
    out.push(Prim::Fill {
        points: f32p(p),
        color,
    });
}

fn line(out: &mut Vec<Prim>, p: &[Pt], width: f64, color: Rgba) {
    #[allow(clippy::cast_possible_truncation)]
    out.push(Prim::Polyline {
        points: f32p(p),
        width: width as f32,
        color,
    });
}

fn lerp(a: Pt, b: Pt, t: f64) -> Pt {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// A closed outline as one open polyline that starts and ends in the middle of the first
/// edge (butt caps meet flush, no doubled segment).
fn closed_outline(p: &[Pt]) -> Vec<Pt> {
    let m = lerp(p[0], p[1], 0.5);
    let mut v = vec![m];
    v.extend_from_slice(&p[1..]);
    v.push(p[0]);
    v.push(m);
    v
}

/// Annulus sector between `r0` < `r1` from `a0` to `a1` degrees.
fn annulus_sector(c: Pt, r0: f64, r1: f64, a0: f64, a1: f64) -> Vec<Pt> {
    let mut v = arc(c, r1, a0, a1, 6.0);
    let mut inner = arc(c, r0, a0, a1, 6.0);
    inner.reverse();
    v.extend(inner);
    v
}

/// Clips a convex-or-not polygon to the half-plane `y >= level` (Sutherland–Hodgman).
fn clip_below(p: &[Pt], level: f64) -> Vec<Pt> {
    let mut out = Vec::new();
    for i in 0..p.len() {
        let a = p[i];
        let b = p[(i + 1) % p.len()];
        let ina = a[1] >= level;
        let inb = b[1] >= level;
        if ina {
            out.push(a);
        }
        if ina != inb {
            let t = (level - a[1]) / (b[1] - a[1]);
            out.push(lerp(a, b, t));
        }
    }
    out
}

// ---------------------------------------------------------------------------------------

impl AlphabetTrait for Rozet {
    fn name(&self) -> &'static str {
        "rozet"
    }

    #[allow(clippy::too_many_lines)]
    fn draw(&self, spec: &GlyphSpec, bx: GlyphBox, theme: &Theme) -> Vec<Prim> {
        let s = f64::from(bx.size).max(8.0);
        let c: Pt = [
            f64::from(bx.x) + f64::from(bx.size) * 0.5,
            f64::from(bx.y) + f64::from(bx.size) * 0.5,
        ];
        let fresh = if spec.freshness.is_finite() {
            spec.freshness.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let alpha = 0.22 + 0.78 * fresh;
        let strength = if spec.strength.is_finite() {
            spec.strength.clamp(0.0, 1.0)
        } else {
            0.5
        };
        let phase = spec.phase;
        let retracted = phase == GlyphPhase::Retracted;
        // (R5/T3b) A closed badge keeps the look's least alpha; (R5/T3c) an open one its
        // least freshness opacity (`r4`: both 0, freshness alone).
        let closed = matches!(
            phase,
            GlyphPhase::Realized | GlyphPhase::Failed | GlyphPhase::Retracted
        );
        let alpha = opacity_at_least(
            alpha,
            if closed {
                theme.closed_alpha
            } else {
                theme.fresh_alpha_min
            },
        );

        // Geometry (pixels).
        let r_out = s * 0.5 - 0.35;
        let ring_t = (s * 0.08).max(1.3);
        let bezel_t = if phase == GlyphPhase::Realized {
            (s * 0.16).max(2.6)
        } else if phase == GlyphPhase::Candidate {
            // a thin grey "pencil" ring: the badge is not inked yet
            (ring_t * 0.55).max(0.9)
        } else {
            ring_t
        };
        let r_in = r_out - bezel_t; // inner edge of the ring
        let r_disc = r_out - ring_t * 0.5; // the fill level stops under the ring
        let pad = (s * 0.045).max(0.7);
        let k_str = 0.56 + 0.44 * strength;
        let big_r = (r_out - ring_t - pad) * k_str;
        let motif_r = big_r.min(r_in - pad * 0.6);

        // Colours.
        let dir_col = if retracted {
            theme.closed
        } else {
            match spec.direction {
                Direction::Up => theme.up,
                Direction::Down => theme.down,
                Direction::Neutral => theme.text,
            }
        };
        let ring_col = if retracted {
            theme.closed
        } else if phase == GlyphPhase::Candidate {
            theme.muted
        } else {
            theme.text
        };
        let a = |col: Rgba| with_alpha(col, alpha);

        let mut out = Vec::new();

        // 1. Confidence: grey level rising from the bottom of the badge.
        if let Some(conf) = spec.confidence {
            let conf = if conf.is_finite() {
                conf.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let h = 2.0 * r_disc * conf.max(0.1);
            let level = c[1] + r_disc - h;
            let mut d = arc(c, r_disc, 0.0, 360.0, 6.0);
            d.pop();
            let poly = clip_below(&d, level);
            if poly.len() >= 3 {
                fill(&mut out, &poly, with_alpha(theme.muted, alpha * 0.42));
            }
            // the water line: a crisp edge at the level
            let dy = level - c[1];
            let half = (r_disc * r_disc - dy * dy).max(0.0).sqrt();
            if half > 0.5 {
                let lw = (s * 0.04).max(0.75);
                line(
                    &mut out,
                    &[
                        [c[0] - half, level + lw * 0.5],
                        [c[0] + half, level + lw * 0.5],
                    ],
                    lw,
                    with_alpha(theme.muted, alpha * 0.8),
                );
            }
        }

        // 2. Ring: 1–5 equal segments, gaps centred at the top and then evenly around.
        let n = spec.confirmations.clamp(1, 5);
        if n == 1 {
            fill(
                &mut out,
                &annulus_sector(c, r_in, r_out, -90.0, 270.0),
                a(ring_col),
            );
        } else {
            let r_mid = (r_in + r_out) * 0.5;
            let gap_px = (s * 0.19).max(2.8);
            let gap_deg = gap_px / r_mid * (180.0 / core::f64::consts::PI);
            let seg = 360.0 / f64::from(n);
            for i in 0..n {
                let a0 = -90.0 + seg * f64::from(i) + gap_deg * 0.5;
                let a1 = -90.0 + seg * f64::from(i + 1) - gap_deg * 0.5;
                fill(
                    &mut out,
                    &annulus_sector(c, r_in, r_out, a0, a1),
                    a(ring_col),
                );
            }
        }

        // 3. Retracted: a cross under the motif (the motif stays readable; everything is
        //    grey).
        let halo = (s * 0.04).max(0.8);
        let bar_w = (s * 0.08).max(1.35);
        let rr = r_in - 0.2;
        let slash = [on_circle(c, rr, 135.0), on_circle(c, rr, -45.0)];
        let back = [on_circle(c, rr, -135.0), on_circle(c, rr, 45.0)];
        if retracted {
            line(&mut out, &slash, bar_w, a(theme.text));
            line(&mut out, &back, bar_w, a(theme.text));
        }

        // 4. Motif: solid with a thin background halo (separates it from the fill level
        //    and the cross); candidate = outline only.
        let neutral_native = spec.direction == Direction::Neutral;
        let shapes: Vec<Vec<Pt>> = motif(spec.kind, neutral_native)
            .into_iter()
            .map(|poly| {
                poly.into_iter()
                    .map(|p| {
                        let [x, y] = orient(p, spec.direction, spec.kind);
                        [c[0] + x * motif_r, c[1] + y * motif_r]
                    })
                    .collect()
            })
            .collect();
        if phase == GlyphPhase::Candidate {
            let stroke = (s * 0.06).max(1.1);
            for poly in &shapes {
                line(&mut out, &closed_outline(poly), stroke, a(dir_col));
            }
        } else {
            for poly in &shapes {
                line(&mut out, &closed_outline(poly), 2.0 * halo, a(theme.bg));
            }
            for poly in &shapes {
                fill(&mut out, poly, a(dir_col));
            }
        }

        // 5. Failed: one slash over everything, with a background halo.
        if phase == GlyphPhase::Failed {
            let w = (s * 0.065).max(1.15);
            line(&mut out, &slash, w + 2.0 * halo * 0.8, a(theme.bg));
            line(&mut out, &slash, w, a(theme.text));
        }

        // 6. Server mark: a small diamond notch at the top-right corner.
        if spec.server {
            let d = (s * 0.19).max(3.0);
            let m = [
                f64::from(bx.x) + f64::from(bx.size) - d - 0.15,
                f64::from(bx.y) + d + 0.15,
            ];
            let dia = |r: f64| {
                vec![
                    [m[0], m[1] - r],
                    [m[0] + r, m[1]],
                    [m[0], m[1] + r],
                    [m[0] - r, m[1]],
                ]
            };
            fill(&mut out, &dia(d), a(theme.bg));
            fill(&mut out, &dia(d * 0.74), a(theme.text));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_specs() -> Vec<GlyphSpec> {
        let mut v = Vec::new();
        let phases = [
            GlyphPhase::Candidate,
            GlyphPhase::Final,
            GlyphPhase::Realized,
            GlyphPhase::Failed,
            GlyphPhase::Retracted,
        ];
        let dirs = [Direction::Up, Direction::Down, Direction::Neutral];
        for kind in MeaningKind::ALL {
            for direction in dirs {
                for phase in phases {
                    for (strength, confidence, freshness, confirmations, server) in [
                        (0.0, None, 1.0, 1u8, false),
                        (0.2, Some(0.0), 0.3, 2, true),
                        (0.6, Some(0.6), 1.0, 3, false),
                        (0.9, Some(1.0), 0.0, 4, true),
                        (1.0, Some(0.3), 0.7, 5, false),
                    ] {
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
        v
    }

    fn points(p: &Prim) -> Vec<[f32; 2]> {
        match p {
            Prim::Fill { points, .. } | Prim::Polyline { points, .. } => points.clone(),
            #[allow(clippy::cast_precision_loss)]
            Prim::Rect { x, y, w, h, .. } => vec![
                [*x as f32, *y as f32],
                [(*x + *w as i32) as f32, (*y + *h as i32) as f32],
            ],
            Prim::Circle { cx, cy, r, .. } => vec![[cx - r, cy - r], [cx + r, cy + r]],
            Prim::Text { .. } => vec![],
        }
    }

    #[test]
    fn trig_is_accurate() {
        for d in [
            -720.0, -90.0, 0.0, 30.0, 45.0, 90.0, 135.0, 180.0, 270.0, 359.0,
        ] {
            let (s, c) = sincos_deg(d);
            let r = f64::to_radians(d);
            assert!((s - r.sin()).abs() < 1e-12, "{d}");
            assert!((c - r.cos()).abs() < 1e-12, "{d}");
        }
    }

    #[test]
    fn deterministic() {
        let t = Theme::light();
        for spec in all_specs() {
            for size in [16.0, 24.0, 32.0] {
                let bx = GlyphBox {
                    x: 3.0,
                    y: 5.0,
                    size,
                };
                let a = format!("{:?}", Rozet.draw(&spec, bx, &t));
                let b = format!("{:?}", Rozet.draw(&spec, bx, &t));
                assert_eq!(a, b);
            }
        }
    }

    #[test]
    fn inside_box_and_non_empty() {
        for t in [Theme::light(), Theme::dark()] {
            for spec in all_specs() {
                for size in [16.0_f32, 20.0, 24.0, 32.0, 48.0, 64.0] {
                    let bx = GlyphBox {
                        x: 10.0,
                        y: 20.0,
                        size,
                    };
                    let prims = Rozet.draw(&spec, bx, &t);
                    assert!(prims.len() >= 2, "{spec:?}");
                    for p in &prims {
                        let hw = match p {
                            Prim::Polyline { width, .. } => width / 2.0,
                            _ => 0.0,
                        };
                        for [x, y] in points(p) {
                            assert!(x.is_finite() && y.is_finite());
                            assert!(
                                x - hw >= bx.x - 0.01
                                    && y - hw >= bx.y - 0.01
                                    && x + hw <= bx.x + size + 0.01
                                    && y + hw <= bx.y + size + 0.01,
                                "{spec:?} {size} {p:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn legend_is_short() {
        assert!(LEGEND.split_whitespace().count() <= 120);
    }
}
