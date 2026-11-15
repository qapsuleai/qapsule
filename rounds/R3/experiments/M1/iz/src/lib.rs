//! **İz** (trace) — R3/M1 candidate meaning-glyph alphabet.
//!
//! Every glyph is the *motion trace of a meaning*, never a picture of data: one stroke whose
//! topology says what kind of thing happened.
//!
//! | kind | trace topology |
//! |------|----------------|
//! | continuation | one straight stroke that keeps going |
//! | reversal | a stroke that comes down, turns round and goes back (U-turn) |
//! | breakout | a step that jumps across a threshold line and stays on the other side |
//! | squeeze | two strokes converging to one point |
//! | extreme | a stroke that accelerates past a limit line near the edge and spills over it |
//! | flow | a wavy side-current leaving a flat price path (rising, sinking; level when neutral) |
//!
//! Remaining channels, in the same spirit (the stroke *is* the meaning, so the channels
//! modify the stroke, its ground and its ending):
//!
//! - **direction**: the trace rises (up), falls (mirror image; down) or runs level
//!   (neutral); colour reinforces it (`up`, `down`, `s2` blue for neutral);
//! - **strength**: the weight of the stroke (and of its head);
//! - **confidence** (ledger): the trace's ground — an area bound to the stroke (its own
//!   per kind: under the slope, under the U, the wall beside the step, the wedge, the
//!   stretch above normal, the current's plume) tinted in proportion to the ledger; no
//!   ledger → no ground at all;
//! - **phase**: how the trace ends and whether it is closed — candidate: dashed stroke,
//!   open arrow head, only the frame's corners (provisional); final: solid stroke, arrow
//!   head, no frame (still moving); realized: closed frame, ends on a full disc (arrived);
//!   failed: dark closed frame, ends on a cross bar (blocked); retracted: everything grey,
//!   framed and crossed out;
//! - **freshness**: the trace fades as it ages (global opacity);
//! - **confirmations**: one dot per evidence family on the ground line under the trace;
//! - **server mark**: that ground line becomes an accent rule sealed by an accent diamond.
//!
//! Deterministic: IEEE basic operations and `sqrt` (correctly rounded by IEEE 754 on every
//! target) only; curves are polynomial Béziers, no trigonometry. Colours come from the
//! [`Theme`] only. Every primitive stays inside the box (a final clamp guarantees it).

use qp_grammar::glyph::{Alphabet as AlphabetTrait, GlyphBox};
use qp_grammar::theme::{with_opacity, Theme};
use qp_schema::capsule::{Direction, GlyphPhase, GlyphSpec, MeaningKind};
use qp_schema::scene::{Prim, Rgba};

/// The `iz` alphabet.
#[derive(Debug, Clone, Copy, Default)]
pub struct Alphabet;

/// Legend for the `legend` reading condition (≤ 120 words, Turkish).
pub const LEGEND: &str = "Her işaret bir anlamın hareket izidir; çizginin biçimi türü söyler. Düz çizgi: devam. U dönüşü yapan çizgi: dönüş. Gri eşiği basamakla aşıp öbür tarafta kalan çizgi: kırılım. Tek noktada birleşen iki çizgi: sıkışma. Kenardaki gri sınırı hızlanarak aşıp taşan eğri: aşırılık. Gri fiyat çizgisinden dikine ayrılan dalgalı akıntı: akış. Yön: iz yükseliyor ve yeşilse yukarı, alçalıyor ve kırmızıysa aşağı, mavi ya da yataysa belirsiz. Kalın çizgi ve büyük ok güçlü, ince çizgi zayıf. Aşama: kesik çizgi ve köşe işaretleri aday; dolu çizgi ve ok ucu kesinleşmiş; çerçeve içinde sonda yuvarlak gerçekleşti; çerçeve içinde sonda dik çubuk gerçekleşmedi; gri ve çarpıyla çizili geri çekildi. Gölge karne güveni, solukluk eskilik, alttaki noktalar teyit sayısı, turuncu taban çizgisi sunucu onayıdır.";

/// A point in unit trace coordinates: `u` right, `v` up, both in `[0, 1]`.
type P = (f64, f64);

/// Geometry of one kind × direction, in unit coordinates.
struct Shape {
    /// Main trace strokes (drawn in the direction colour, dashed when candidate).
    strokes: Vec<Vec<P>>,
    /// Auxiliary reference lines (threshold, limit, price path): thin, grey, solid.
    aux: Vec<Vec<P>>,
    /// The area the trace sweeps (tinted by confidence).
    ground: Vec<P>,
    /// Indices of the strokes whose last point is the trace's end (terminal).
    ends: Vec<usize>,
}

fn lerp(a: P, b: P, t: f64) -> P {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Cubic Bézier sampled at `n + 1` points (polynomial, deterministic).
fn cubic(p0: P, p1: P, p2: P, p3: P, n: u32) -> Vec<P> {
    (0..=n)
        .map(|i| {
            let t = f64::from(i) / f64::from(n);
            let a = lerp(p0, p1, t);
            let b = lerp(p1, p2, t);
            let c = lerp(p2, p3, t);
            lerp(lerp(a, b, t), lerp(b, c, t), t)
        })
        .collect()
}

/// A wave along the segment `a → b`: `halves` half-periods of amplitude `amp`,
/// perpendicular to the segment, built from cubic arcs (no trigonometry).
fn wave(a: P, b: P, amp: f64, halves: u32) -> Vec<P> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l = (dx * dx + dy * dy).sqrt().max(1e-9);
    let n = (-dy / l, dx / l);
    let mut out: Vec<P> = Vec::new();
    for h in 0..halves {
        let t0 = f64::from(h) / f64::from(halves);
        let t1 = f64::from(h + 1) / f64::from(halves);
        let sg = if h % 2 == 0 { 1.0 } else { -1.0 };
        let p0 = lerp(a, b, t0);
        let p3 = lerp(a, b, t1);
        // Control points at 1/3 and 2/3 lifted by 4/3·amp: the half-wave peaks at `amp`.
        let c1 = lerp(a, b, t0 + (t1 - t0) / 3.0);
        let c2 = lerp(a, b, t0 + 2.0 * (t1 - t0) / 3.0);
        let k = sg * amp * 4.0 / 3.0;
        let seg = cubic(
            p0,
            (c1.0 + n.0 * k, c1.1 + n.1 * k),
            (c2.0 + n.0 * k, c2.1 + n.1 * k),
            p3,
            8,
        );
        let skip = usize::from(!out.is_empty());
        out.extend(seg.into_iter().skip(skip));
    }
    out
}

fn closed_under(trace: &[P], floor: f64) -> Vec<P> {
    let mut g = trace.to_vec();
    if let (Some(last), Some(first)) = (trace.last(), trace.first()) {
        g.push((last.0, floor));
        g.push((first.0, floor));
    }
    g
}

/// Shape for the "up" (or neutral) member; "down" is the mirror image (`v → 1 − v`).
#[allow(clippy::too_many_lines)]
fn shape(kind: MeaningKind, dir: Direction) -> Shape {
    let neutral = dir == Direction::Neutral;
    match kind {
        MeaningKind::Continuation => {
            // One straight stroke that keeps going.
            let t = if neutral {
                vec![(0.0, 0.5), (0.97, 0.5)]
            } else {
                vec![(0.0, 0.08), (0.97, 0.92)]
            };
            let ground = if neutral {
                vec![(0.0, 0.5), (0.97, 0.5), (0.97, 0.15), (0.0, 0.15)]
            } else {
                closed_under(&t, 0.0)
            };
            Shape {
                strokes: vec![t],
                aux: vec![],
                ground,
                ends: vec![0],
            }
        }
        MeaningKind::Reversal => {
            // Comes down, turns round, goes back up (a U-turn).
            let t = if neutral {
                // Lying U-turn: goes right, turns round, comes back left.
                let mut a = vec![(0.05, 0.82), (0.55, 0.82)];
                a.extend(
                    cubic((0.55, 0.82), (1.05, 0.82), (1.05, 0.18), (0.55, 0.18), 16)
                        .into_iter()
                        .skip(1),
                );
                a.push((0.2, 0.18));
                a
            } else {
                cubic((0.03, 1.0), (0.12, -0.25), (0.62, -0.25), (0.9, 0.62), 24)
            };
            let ground = if neutral {
                t.clone()
            } else {
                closed_under(&t, 0.0)
            };
            Shape {
                strokes: vec![t],
                aux: vec![],
                ground,
                ends: vec![0],
            }
        }
        MeaningKind::Breakout => {
            if neutral {
                // Level trace crossing a vertical boundary.
                let t = vec![(0.0, 0.5), (0.97, 0.5)];
                Shape {
                    ground: vec![(0.5, 0.5), (0.97, 0.5), (0.97, 0.2), (0.5, 0.2)],
                    strokes: vec![t],
                    aux: vec![vec![(0.5, 0.05), (0.5, 0.95)]],
                    ends: vec![0],
                }
            } else {
                // A step that jumps across the threshold and stays on the other side.
                let t = vec![(0.0, 0.18), (0.42, 0.18), (0.56, 0.86), (0.97, 0.86)];
                Shape {
                    // The jump: the gap between the trace and the threshold, below it
                    // before the step and above it after.
                    ground: vec![
                        (0.0, 0.18),
                        (0.42, 0.18),
                        (0.56, 0.86),
                        (0.97, 0.86),
                        (0.97, 1.0),
                        (0.0, 1.0),
                    ],
                    strokes: vec![t],
                    aux: vec![vec![(0.0, 0.52), (1.0, 0.52)]],
                    ends: vec![0],
                }
            }
        }
        MeaningKind::Squeeze => {
            // Two strokes converging to one point; the apex height is the direction.
            let apex = match dir {
                Direction::Up => (0.95, 0.85),
                Direction::Down => (0.95, 0.15),
                Direction::Neutral => (0.95, 0.5),
            };
            // Mirroring for "down" happens later, so build the up geometry for both.
            let apex = if dir == Direction::Down {
                (apex.0, 1.0 - apex.1)
            } else {
                apex
            };
            let hi = vec![(0.0, 1.0), apex];
            let lo = vec![(0.0, 0.0), apex];
            Shape {
                ground: vec![(0.0, 1.0), apex, (0.0, 0.0)],
                strokes: vec![hi, lo],
                aux: vec![],
                ends: vec![0, 1],
            }
        }
        MeaningKind::Extreme => {
            if neutral {
                let t = vec![
                    (0.0, 0.5),
                    (0.45, 0.5),
                    (0.6, 1.0),
                    (0.78, 0.0),
                    (0.95, 0.5),
                ];
                Shape {
                    ground: vec![(0.45, 0.5), (0.6, 1.0), (0.78, 0.0), (0.95, 0.5)],
                    strokes: vec![t],
                    aux: vec![
                        vec![(0.0, 0.84), (1.0, 0.84)],
                        vec![(0.0, 0.16), (1.0, 0.16)],
                    ],
                    ends: vec![0],
                }
            } else {
                // Accelerates past the limit near the edge and spills over it.
                let mut t = cubic((0.0, 0.05), (0.45, 0.05), (0.55, 0.3), (0.62, 1.0), 20);
                t.extend(
                    cubic((0.62, 1.0), (0.66, 1.08), (0.82, 1.04), (0.96, 0.84), 12)
                        .into_iter()
                        .skip(1),
                );
                // The stretch: what the trace swept above its normal range.
                let high: Vec<P> = t.iter().copied().filter(|p| p.1 >= 0.4).collect();
                Shape {
                    ground: closed_under(&high, 0.4),
                    strokes: vec![t],
                    aux: vec![vec![(0.0, 0.76), (1.0, 0.76)]],
                    ends: vec![0],
                }
            }
        }
        MeaningKind::Flow => {
            // A wavy side-current branching off a flat price path: it rises (money in),
            // sinks (money out, mirrored) or runs alongside (neutral).
            if neutral {
                let cur = wave((0.0, 0.62), (0.97, 0.62), 0.16, 3);
                Shape {
                    ground: vec![(0.0, 0.84), (0.97, 0.84), (0.97, 0.4), (0.0, 0.4)],
                    strokes: vec![cur],
                    aux: vec![vec![(0.0, 0.1), (1.0, 0.1)]],
                    ends: vec![0],
                }
            } else {
                let cur = wave((0.5, 0.3), (0.5, 1.0), 0.15, 3);
                Shape {
                    // The current's plume, widening away from the price path.
                    ground: vec![(0.42, 0.3), (0.58, 0.3), (0.85, 1.0), (0.15, 1.0)],
                    strokes: vec![cur],
                    aux: vec![vec![(0.0, 0.3), (1.0, 0.3)]],
                    ends: vec![0],
                }
            }
        }
    }
}

/// Pixel mapping of the trace field inside the box.
struct Field {
    x0: f64,
    y0: f64,
    w: f64,
    h: f64,
    mirror: bool,
}

impl Field {
    fn px(&self, p: P) -> (f64, f64) {
        let v = if self.mirror { 1.0 - p.1 } else { p.1 };
        (self.x0 + p.0 * self.w, self.y0 + (1.0 - v) * self.h)
    }
    fn path(&self, ps: &[P]) -> Vec<(f64, f64)> {
        ps.iter().map(|p| self.px(*p)).collect()
    }
}

#[allow(clippy::cast_possible_truncation)]
fn pts(ps: &[(f64, f64)]) -> Vec<[f32; 2]> {
    ps.iter().map(|(x, y)| [*x as f32, *y as f32]).collect()
}

fn len(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    (dx * dx + dy * dy).sqrt()
}

/// Splits a pixel polyline into dashes (`on`, `off` in pixels, starting with a dash and
/// always ending with one, so the trace's end stays drawn).
fn dashes(ps: &[(f64, f64)], on: f64, off: f64) -> Vec<Vec<(f64, f64)>> {
    let total: f64 = ps.windows(2).map(|w| len(w[0], w[1])).sum();
    if total <= 0.0 {
        return vec![ps.to_vec()];
    }
    // Stretch the pattern so that a whole number of periods fits and the last dash ends
    // exactly at the end.
    let period = on + off;
    let k = ((total + off) / period).round().max(1.0);
    let scale = (total + off) / (k * period);
    let (on, off) = (on * scale, off * scale);
    let point_at = |d: f64| -> (f64, f64) {
        let mut acc = 0.0;
        for w in ps.windows(2) {
            let l = len(w[0], w[1]);
            if acc + l >= d && l > 0.0 {
                let t = (d - acc) / l;
                return (
                    w[0].0 + (w[1].0 - w[0].0) * t,
                    w[0].1 + (w[1].1 - w[0].1) * t,
                );
            }
            acc += l;
        }
        *ps.last().unwrap_or(&(0.0, 0.0))
    };
    let mut out = Vec::new();
    let mut d = 0.0;
    while d < total - 1e-9 {
        let e = (d + on).min(total);
        let mut dash = vec![point_at(d)];
        let mut acc = 0.0;
        for w in ps.windows(2) {
            let l = len(w[0], w[1]);
            acc += l;
            if acc > d && acc < e {
                dash.push(w[1]);
            }
        }
        dash.push(point_at(e));
        out.push(dash);
        d = e + off;
    }
    out
}

/// Unit tangent at the end of a pixel polyline (looking back at least `back` pixels).
fn end_tangent(ps: &[(f64, f64)], back: f64) -> (f64, f64) {
    let Some(&e) = ps.last() else {
        return (1.0, 0.0);
    };
    let mut q = e;
    for p in ps.iter().rev().skip(1) {
        q = *p;
        if len(*p, e) >= back {
            break;
        }
    }
    let l = len(q, e);
    if l <= 0.0 {
        (1.0, 0.0)
    } else {
        ((e.0 - q.0) / l, (e.1 - q.1) / l)
    }
}

fn dir_colour(theme: &Theme, d: Direction) -> Rgba {
    match d {
        Direction::Up => theme.up,
        Direction::Down => theme.down,
        Direction::Neutral => theme.series[1],
    }
}

fn clamp01(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else {
        v.clamp(0.0, 1.0)
    }
}

/// Keeps every primitive inside the box (stroke half-widths included).
#[allow(clippy::cast_possible_truncation)]
fn clamp_into(prims: &mut [Prim], bx: GlyphBox) {
    let (x0, y0, x1, y1) = (bx.x, bx.y, bx.x + bx.size, bx.y + bx.size);
    let c = |v: f32, lo: f32, hi: f32| {
        if lo > hi {
            (lo + hi) / 2.0
        } else {
            v.clamp(lo, hi)
        }
    };
    for p in prims.iter_mut() {
        match p {
            Prim::Polyline { points, width, .. } => {
                let hw = *width / 2.0 + 0.005;
                for q in points.iter_mut() {
                    q[0] = c(q[0], x0 + hw, x1 - hw);
                    q[1] = c(q[1], y0 + hw, y1 - hw);
                }
            }
            Prim::Fill { points, .. } => {
                for q in points.iter_mut() {
                    q[0] = c(q[0], x0, x1);
                    q[1] = c(q[1], y0, y1);
                }
            }
            Prim::Circle { cx, cy, r, .. } => {
                *r = r.min(bx.size / 2.0);
                *cx = c(*cx, x0 + *r, x1 - *r);
                *cy = c(*cy, y0 + *r, y1 - *r);
            }
            Prim::Rect { .. } | Prim::Text { .. } => {}
        }
    }
}

impl AlphabetTrait for Alphabet {
    fn name(&self) -> &'static str {
        "iz"
    }

    #[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
    fn draw(&self, spec: &GlyphSpec, bx: GlyphBox, theme: &Theme) -> Vec<Prim> {
        let s = f64::from(bx.size.max(1.0));
        let (bx0, by0) = (f64::from(bx.x), f64::from(bx.y));
        let strength = clamp01(spec.strength);
        let fresh = clamp01(spec.freshness);
        let alpha = 0.3 + 0.7 * fresh;
        let retracted = spec.phase == GlyphPhase::Retracted;
        let fade = |c: Rgba| with_opacity(c, alpha);

        let field = Field {
            x0: bx0 + 0.08 * s,
            y0: by0 + 0.08 * s,
            w: 0.84 * s,
            h: 0.68 * s,
            mirror: spec.direction == Direction::Down,
        };
        let sh = shape(spec.kind, spec.direction);
        let ink = if retracted {
            theme.muted
        } else {
            dir_colour(theme, spec.direction)
        };
        let width = s * (0.03 + 0.13 * strength);
        let aux_w = (s * 0.04).max(0.9);
        let mut prims = Vec::new();

        // Ground: the area the trace sweeps, tinted by the ledger's confidence.
        if let (Some(conf), false) = (spec.confidence, retracted) {
            let a = 0.16 + 0.56 * clamp01(conf);
            prims.push(Prim::Fill {
                points: pts(&field.path(&sh.ground)),
                color: fade(with_opacity(ink, a)),
            });
        }
        // Reference lines (threshold, limit, price path).
        for a in &sh.aux {
            prims.push(Prim::Polyline {
                points: pts(&field.path(a)),
                width: aux_w as f32,
                color: fade(theme.axis),
            });
        }
        // The trace itself.
        let paths: Vec<Vec<(f64, f64)>> = sh.strokes.iter().map(|t| field.path(t)).collect();
        for p in &paths {
            if spec.phase == GlyphPhase::Candidate {
                for d in dashes(p, s * 0.15, s * 0.11) {
                    prims.push(Prim::Polyline {
                        points: pts(&d),
                        width: width as f32,
                        color: fade(ink),
                    });
                }
            } else {
                prims.push(Prim::Polyline {
                    points: pts(p),
                    width: width as f32,
                    color: fade(ink),
                });
            }
        }
        // Terminal: how the trace ends tells its phase.
        let head = s * (0.13 + 0.14 * strength);
        let mut done_end = false;
        for &i in &sh.ends {
            let Some(p) = paths.get(i) else { continue };
            let Some(&e) = p.last() else { continue };
            let t = if sh.ends.len() > 1 {
                (1.0, 0.0)
            } else {
                end_tangent(p, head * 0.8)
            };
            let n = (-t.1, t.0);
            if done_end {
                continue;
            }
            done_end = true;
            match spec.phase {
                GlyphPhase::Candidate | GlyphPhase::Final => {
                    let tip = (e.0 + t.0 * head * 0.45, e.1 + t.1 * head * 0.45);
                    let base = (e.0 - t.0 * head * 0.55, e.1 - t.1 * head * 0.55);
                    let hw = head * 0.5;
                    let tri = vec![
                        tip,
                        (base.0 + n.0 * hw, base.1 + n.1 * hw),
                        (base.0 - n.0 * hw, base.1 - n.1 * hw),
                    ];
                    if spec.phase == GlyphPhase::Candidate {
                        // Open head: outline only.
                        let mut o = tri.clone();
                        o.push(tri[0]);
                        o.push(tri[1]);
                        prims.push(Prim::Polyline {
                            points: pts(&o),
                            width: (width * 0.7).max(0.9) as f32,
                            color: fade(ink),
                        });
                    } else {
                        prims.push(Prim::Fill {
                            points: pts(&tri),
                            color: fade(ink),
                        });
                    }
                }
                GlyphPhase::Realized => {
                    prims.push(Prim::Circle {
                        cx: e.0 as f32,
                        cy: e.1 as f32,
                        r: (s * 0.13 + width * 0.35) as f32,
                        color: fade(ink),
                    });
                }
                GlyphPhase::Failed => {
                    let hw = s * 0.2;
                    prims.push(Prim::Polyline {
                        points: pts(&[
                            (e.0 + n.0 * hw, e.1 + n.1 * hw),
                            (e.0 - n.0 * hw, e.1 - n.1 * hw),
                        ]),
                        width: (width * 1.2).max(s * 0.1) as f32,
                        color: fade(theme.text),
                    });
                }
                GlyphPhase::Retracted => {}
            }
        }
        // A closed outcome (realized, failed, retracted) closes the trace in a frame.
        let frame = match spec.phase {
            GlyphPhase::Realized => Some(ink),
            GlyphPhase::Failed => Some(theme.text),
            GlyphPhase::Retracted => Some(theme.muted),
            GlyphPhase::Candidate | GlyphPhase::Final => None,
        };
        if spec.phase == GlyphPhase::Candidate {
            // Provisional: only the corners of the frame are drawn.
            let fw = (s * 0.045).max(0.9);
            let (l, t, r, b) = (
                bx0 + fw / 2.0,
                by0 + fw / 2.0,
                bx0 + s - fw / 2.0,
                by0 + 0.82 * s,
            );
            let c = 0.2 * s;
            for corner in [
                [(l, t + c), (l, t), (l + c, t)],
                [(r - c, t), (r, t), (r, t + c)],
                [(r, b - c), (r, b), (r - c, b)],
                [(l + c, b), (l, b), (l, b - c)],
            ] {
                prims.push(Prim::Polyline {
                    points: pts(&corner),
                    width: fw as f32,
                    color: fade(ink),
                });
            }
        }
        if let Some(fc) = frame {
            let fw = (s * 0.045).max(0.9);
            let (l, t, r, b) = (
                bx0 + fw / 2.0,
                by0 + fw / 2.0,
                bx0 + s - fw / 2.0,
                by0 + 0.82 * s,
            );
            prims.push(Prim::Polyline {
                points: pts(&[(l, b), (l, t), (r, t), (r, b), (l, b), (l, t)]),
                width: fw as f32,
                color: fade(fc),
            });
        }
        if retracted {
            // Crossed out.
            let m = 0.1 * s;
            let (l, r, t, b) = (bx0 + m, bx0 + s - m, by0 + m, by0 + 0.8 * s);
            for line in [[(l, t), (r, b)], [(l, b), (r, t)]] {
                prims.push(Prim::Polyline {
                    points: pts(&line),
                    width: (s * 0.06).max(1.0) as f32,
                    color: fade(theme.text),
                });
            }
        }
        // Ground line row: confirmation dots and the server diamond.
        let row_y = by0 + 0.91 * s;
        let dot_r = (s * 0.075).max(0.9);
        let gap = s * 0.15;
        for k in 0..spec.confirmations.clamp(1, 5) {
            prims.push(Prim::Circle {
                cx: (bx0 + 0.1 * s + f64::from(k) * gap) as f32,
                cy: row_y as f32,
                r: dot_r as f32,
                color: fade(theme.text),
            });
        }
        if spec.server {
            // The evidence row stands on a verified (accent) ground line, sealed by a diamond.
            let lw = (s * 0.045).max(0.8);
            let ly = by0 + s - lw / 2.0;
            prims.push(Prim::Polyline {
                points: pts(&[(bx0 + 0.04 * s, ly), (bx0 + 0.96 * s, ly)]),
                width: lw as f32,
                color: fade(theme.accent),
            });
            let (cx, dx, dy) = (bx0 + 0.85 * s, s * 0.14, s * 0.088);
            prims.push(Prim::Fill {
                points: pts(&[
                    (cx, row_y - dy),
                    (cx + dx, row_y),
                    (cx, row_y + dy),
                    (cx - dx, row_y),
                ]),
                color: fade(theme.accent),
            });
        }
        clamp_into(&mut prims, bx);
        prims
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_specs() -> Vec<GlyphSpec> {
        let mut v = Vec::new();
        for kind in MeaningKind::ALL {
            for direction in [Direction::Up, Direction::Down, Direction::Neutral] {
                for phase in [
                    GlyphPhase::Candidate,
                    GlyphPhase::Final,
                    GlyphPhase::Realized,
                    GlyphPhase::Failed,
                    GlyphPhase::Retracted,
                ] {
                    for (strength, confidence, freshness, confirmations, server) in [
                        (0.0, None, 1.0, 1, false),
                        (0.2, Some(0.3), 0.3, 2, true),
                        (0.6, Some(0.6), 1.0, 3, false),
                        (0.9, Some(0.8), 0.0, 4, true),
                        (1.0, Some(1.0), 0.5, 5, false),
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
                        });
                    }
                }
            }
        }
        v
    }

    fn boxes() -> Vec<GlyphBox> {
        [16.0, 20.0, 24.0, 32.0, 48.0, 64.0]
            .into_iter()
            .map(|size| GlyphBox {
                x: 3.0,
                y: 7.5,
                size,
            })
            .collect()
    }

    fn inside(p: &Prim, bx: GlyphBox) -> bool {
        let tol = 0.01;
        let ok = |x: f32, y: f32| {
            x >= bx.x - tol
                && y >= bx.y - tol
                && x <= bx.x + bx.size + tol
                && y <= bx.y + bx.size + tol
        };
        match p {
            Prim::Polyline { points, width, .. } => {
                let hw = width / 2.0;
                points
                    .iter()
                    .all(|[x, y]| ok(x - hw, y - hw) && ok(x + hw, y + hw))
            }
            Prim::Fill { points, .. } => points.iter().all(|[x, y]| ok(*x, *y)),
            Prim::Circle { cx, cy, r, .. } => ok(cx - r, cy - r) && ok(cx + r, cy + r),
            #[allow(clippy::cast_precision_loss)]
            Prim::Rect { x, y, w, h, .. } => {
                ok(*x as f32, *y as f32) && ok((*x + *w as i32) as f32, (*y + *h as i32) as f32)
            }
            Prim::Text { .. } => false,
        }
    }

    #[test]
    fn deterministic() {
        for theme in [Theme::light(), Theme::dark()] {
            for bx in boxes() {
                for s in all_specs() {
                    let a = Alphabet.draw(&s, bx, &theme);
                    let b = Alphabet.draw(&s, bx, &theme);
                    assert_eq!(a, b);
                }
            }
        }
    }

    #[test]
    fn inside_box_and_non_empty() {
        for theme in [Theme::light(), Theme::dark()] {
            for bx in boxes() {
                for s in all_specs() {
                    let prims = Alphabet.draw(&s, bx, &theme);
                    assert!(!prims.is_empty(), "{s:?}");
                    // At least one visible stroke.
                    assert!(prims.iter().any(|p| matches!(p, Prim::Polyline { points, width, .. } if points.len() >= 2 && *width > 0.0)));
                    for p in &prims {
                        assert!(inside(p, bx), "{s:?} @ {bx:?}: {p:?}");
                        let finite = match p {
                            Prim::Polyline { points, .. } | Prim::Fill { points, .. } => {
                                points.iter().all(|[x, y]| x.is_finite() && y.is_finite())
                            }
                            Prim::Circle { cx, cy, r, .. } => {
                                cx.is_finite() && cy.is_finite() && r.is_finite()
                            }
                            _ => true,
                        };
                        assert!(finite);
                    }
                }
            }
        }
    }

    #[test]
    fn colours_come_from_theme() {
        for theme in [Theme::light(), Theme::dark()] {
            let rgb: Vec<[u8; 3]> = [
                theme.up,
                theme.down,
                theme.series[1],
                theme.muted,
                theme.axis,
                theme.text,
                theme.accent,
            ]
            .iter()
            .map(|c| [c[0], c[1], c[2]])
            .collect();
            for s in all_specs() {
                for p in Alphabet.draw(&s, boxes()[0], &theme) {
                    let c = match p {
                        Prim::Polyline { color, .. }
                        | Prim::Fill { color, .. }
                        | Prim::Circle { color, .. } => color,
                        _ => continue,
                    };
                    assert!(rgb.contains(&[c[0], c[1], c[2]]));
                }
            }
        }
    }

    #[test]
    fn hostile_values_stay_finite() {
        let mut s = all_specs()[0].clone();
        s.strength = f64::NAN;
        s.freshness = f64::INFINITY;
        s.confidence = Some(-3.0);
        s.confirmations = 0;
        let prims = Alphabet.draw(&s, boxes()[0], &Theme::light());
        assert!(!prims.is_empty());
        s.confirmations = 200;
        assert_eq!(
            Alphabet
                .draw(&s, boxes()[0], &Theme::light())
                .iter()
                .filter(|p| matches!(p, Prim::Circle { .. }))
                .count(),
            5
        );
    }

    #[test]
    fn legend_is_short() {
        assert!(
            LEGEND.split_whitespace().count() <= 120,
            "{}",
            LEGEND.split_whitespace().count()
        );
    }
}
