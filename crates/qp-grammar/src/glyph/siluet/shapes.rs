//! `siluet`'s shapes: one per meaning kind, oriented by direction, and dashed strokes.

use super::*;

fn continuation(d: Direction) -> Shape {
    if d == Direction::Neutral {
        let h = Head {
            tip: [0.95, 0.5],
            dir: [1.0, 0.0],
            len: 0.32,
            half: 0.24,
        };
        return Shape {
            strokes: vec![(vec![[0.05, 0.5], back(&h, 0.5)], Ink::Main)],
            heads: vec![h],
            dots: vec![],
        };
    }
    let h = Head {
        tip: [0.9, 0.1],
        dir: [DIAG, -DIAG],
        len: 0.34,
        half: 0.25,
    };
    Shape {
        strokes: vec![(vec![[0.1, 0.9], back(&h, 0.5)], Ink::Main)],
        heads: vec![h],
        dots: vec![],
    }
    .oriented(d)
}

fn reversal(d: Direction) -> Shape {
    let h = Head {
        tip: [0.24, 0.04],
        dir: [0.0, -1.0],
        len: 0.32,
        half: 0.23,
    };
    // Down the right leg, round the bottom, back up the left leg (head top-left, so it
    // never sits where the continuation arrow's head does).
    let (cx, cy, r) = (0.5_f32, 0.56_f32, 0.26_f32);
    let mut pts = vec![[0.76, 0.1]];
    for [c, s] in HALF_CIRCLE.iter().rev() {
        pts.push([cx + r * c, cy + r * s]);
    }
    pts.push(back(&h, 0.5));
    Shape {
        strokes: vec![(pts, Ink::Main)],
        heads: vec![h],
        dots: vec![],
    }
    .oriented(d)
}

fn breakout(d: Direction) -> Shape {
    let h = Head {
        tip: [0.5, 0.04],
        dir: [0.0, -1.0],
        len: 0.32,
        half: 0.25,
    };
    Shape {
        strokes: vec![
            (vec![[0.04, 0.64], [0.96, 0.64]], Ink::Edge),
            (vec![[0.5, 0.96], back(&h, 0.5)], Ink::Main),
        ],
        heads: vec![h],
        dots: vec![],
    }
    .oriented(d)
}

fn squeeze(d: Direction) -> Shape {
    let strokes = match d {
        Direction::Neutral => vec![
            (vec![[0.05, 0.08], [0.9, 0.42]], Ink::Main),
            (vec![[0.05, 0.92], [0.9, 0.58]], Ink::Main),
        ],
        // Ascending triangle: flat top, rising floor; descending is its mirror.
        Direction::Up | Direction::Down => {
            let s = Shape {
                strokes: vec![
                    (vec![[0.05, 0.12], [0.92, 0.12]], Ink::Main),
                    (vec![[0.05, 0.92], [0.92, 0.3]], Ink::Main),
                ],
                ..Shape::default()
            };
            return s.oriented(d);
        }
    };
    Shape {
        strokes,
        ..Shape::default()
    }
}

fn extreme(d: Direction) -> Shape {
    if d == Direction::Neutral {
        return Shape {
            strokes: vec![(
                vec![
                    [0.04, 0.5],
                    [0.28, 0.5],
                    [0.38, 0.2],
                    [0.62, 0.8],
                    [0.72, 0.5],
                    [0.96, 0.5],
                ],
                Ink::Main,
            )],
            heads: vec![],
            dots: vec![([0.38, 0.12], 0.1), ([0.62, 0.88], 0.1)],
        };
    }
    Shape {
        strokes: vec![(
            vec![
                [0.04, 0.92],
                [0.32, 0.92],
                [0.5, 0.2],
                [0.68, 0.92],
                [0.96, 0.92],
            ],
            Ink::Main,
        )],
        heads: vec![],
        dots: vec![([0.5, 0.15], 0.13)],
    }
    .oriented(d)
}

/// One sine wave sample line: `steps` samples starting at phase 0.
fn wave(from: P, to: P, amp: f32, steps: usize, vertical: bool) -> Vec<P> {
    #[allow(clippy::cast_precision_loss)]
    let n = steps as f32;
    (0..=steps)
        .map(|k| {
            #[allow(clippy::cast_precision_loss)]
            let t = k as f32 / n;
            let off = amp * SINE16[k % 16];
            let x = from[0] + (to[0] - from[0]) * t;
            let y = from[1] + (to[1] - from[1]) * t;
            if vertical {
                [x + off, y]
            } else {
                [x, y + off]
            }
        })
        .collect()
}

fn flow(d: Direction) -> Shape {
    if d == Direction::Neutral {
        return Shape {
            strokes: vec![
                (wave([0.04, 0.34], [0.96, 0.34], 0.1, 24, false), Ink::Main),
                (wave([0.04, 0.66], [0.96, 0.66], 0.1, 24, false), Ink::Main),
            ],
            ..Shape::default()
        };
    }
    // Two wavy streams under one wide arrowhead.
    let h = Head {
        tip: [0.5, 0.04],
        dir: [0.0, -1.0],
        len: 0.3,
        half: 0.4,
    };
    let top = back(&h, 0.8)[1];
    Shape {
        strokes: vec![
            (wave([0.3, 0.96], [0.3, top], AMP, 16, true), Ink::Main),
            (wave([0.7, 0.96], [0.7, top], AMP2, 16, true), Ink::Main),
        ],
        heads: vec![h],
        dots: vec![],
    }
    .oriented(d)
}

pub(super) fn shape(kind: MeaningKind, d: Direction) -> Shape {
    match kind {
        MeaningKind::Continuation => continuation(d),
        MeaningKind::Reversal => reversal(d),
        MeaningKind::Breakout => breakout(d),
        MeaningKind::Squeeze => squeeze(d),
        MeaningKind::Extreme => extreme(d),
        MeaningKind::Flow => flow(d),
    }
}

/// Splits a polyline into dashes (`on` drawn, `off` skipped, in pixels).
pub(super) fn dashes(pts: &[P], on: f32, off: f32) -> Vec<Vec<P>> {
    let mut out: Vec<Vec<P>> = Vec::new();
    let mut cur: Vec<P> = Vec::new();
    let mut drawing = true;
    let mut left = on;
    if let Some(&p0) = pts.first() {
        cur.push(p0);
    }
    for w in pts.windows(2) {
        let (mut a, b) = (w[0], w[1]);
        let mut seg = ((b[0] - a[0]) * (b[0] - a[0]) + (b[1] - a[1]) * (b[1] - a[1])).sqrt();
        while seg > 0.0 {
            if seg <= left {
                left -= seg;
                if drawing {
                    cur.push(b);
                }
                seg = 0.0;
            } else {
                let t = left / seg;
                let m = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                if drawing {
                    cur.push(m);
                    out.push(std::mem::take(&mut cur));
                } else {
                    cur = vec![m];
                }
                drawing = !drawing;
                seg -= left;
                left = if drawing { on } else { off };
                a = m;
            }
        }
    }
    if drawing && cur.len() > 1 {
        out.push(cur);
    }
    out
}
