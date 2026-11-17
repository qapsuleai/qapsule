//! RP3/M12a: the retained painter (`qp_render::retained`) gives exactly the reference
//! renderer's pixels, frame after frame, whatever changes between frames.

use qp_render::cpu::rasterize;
use qp_render::retained::{Retained, RetainedOpts, Scratch};
use qp_render::scene::{scene_of, stress_scene, Prim, Rgba};

/// Deterministic test sequence (fixed seed; not compute code).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }
    fn below(&mut self, n: usize) -> usize {
        self.next() as usize % n.max(1)
    }
}

fn shift(p: &Prim, dx: f32, dy: f32) -> Prim {
    let mv = |v: &[[f32; 2]]| v.iter().map(|[x, y]| [x + dx, y + dy]).collect();
    match p.clone() {
        Prim::Rect { x, y, w, h, color } => Prim::Rect {
            x: x + dx as i32,
            y: y + dy as i32,
            w,
            h,
            color,
        },
        Prim::Polyline {
            points,
            width,
            color,
        } => Prim::Polyline {
            points: mv(&points),
            width,
            color,
        },
        Prim::Fill { points, color } => Prim::Fill {
            points: mv(&points),
            color,
        },
        Prim::Circle { cx, cy, r, color } => Prim::Circle {
            cx: cx + dx,
            cy: cy + dy,
            r,
            color,
        },
        Prim::Text {
            x,
            baseline,
            px,
            text,
            color,
            align,
        } => Prim::Text {
            x: x + dx as i32,
            baseline: baseline + dy as i32,
            px,
            text: format!("{text}{}", (dx as i32).rem_euclid(10)),
            color,
            align,
        },
    }
}

fn reference(w: u32, h: u32, bg: Rgba, prims: &[Prim]) -> Vec<u8> {
    rasterize(&scene_of(w, h, bg, prims.to_vec()))
        .unwrap()
        .data()
        .to_vec()
}

fn run(opts: RetainedOpts, bg: Rgba, frames: usize, seed: u64) -> (usize, usize) {
    let (w, h) = (300, 200);
    let mut prims: Vec<Prim> = stress_scene(w, h).prims().cloned().collect();
    let mut rng = Lcg(seed);
    let mut r = Retained::default();
    let mut scratch = Scratch::default();
    let (mut partial, mut layered) = (0, 0);
    for f in 0..frames {
        if f > 0 {
            // 1–4 edits: move, recolour, remove, insert, swap, or nothing at all.
            for _ in 0..=rng.below(4) {
                let n = prims.len();
                match rng.below(7) {
                    0 | 1 => {
                        let i = n - 1 - rng.below(n.min(12));
                        let d = (rng.below(21) as f32 - 10.0) * 0.75;
                        prims[i] = shift(&prims[i], d, -d * 0.5);
                    }
                    2 => {
                        let i = rng.below(n);
                        prims[i] = shift(&prims[i], 3.0, 0.0);
                    }
                    3 if n > 4 => {
                        prims.remove(n - 1 - rng.below(n.min(8)));
                    }
                    4 => {
                        let i = rng.below(n);
                        let p = shift(&prims[i], 7.25, 4.5);
                        prims.insert(n - rng.below(n.min(8)), p);
                    }
                    5 if n > 2 => {
                        let i = n - 2 - rng.below((n - 1).min(6));
                        prims.swap(i, i + 1);
                    }
                    _ => {}
                }
            }
        }
        let changed = r
            .paint(w, h, bg, prims.clone(), &mut scratch, opts)
            .unwrap();
        let want = reference(w, h, bg, &prims);
        assert_eq!(
            r.pixmap().unwrap().data(),
            want.as_slice(),
            "frame {f} differs ({:?}, {changed:?})",
            r.stats()
        );
        if !r.stats().full && changed.is_some() {
            partial += 1;
        }
        if r.stats().layer_used {
            layered += 1;
        }
    }
    (partial, layered)
}

#[test]
fn every_frame_equals_the_reference() {
    for (seed, bg) in [
        (1, [15, 17, 21, 255]),
        (2, [250, 250, 245, 255]),
        (3, [20, 30, 40, 128]), // not opaque: premultiplied pixels must still match
    ] {
        let (partial, layered) = run(RetainedOpts::default(), bg, 120, seed);
        assert!(partial > 30, "seed {seed}: only {partial} partial paints");
        assert!(
            layered > 10,
            "seed {seed}: prefix image used {layered} times"
        );
        let (p, l) = run(
            RetainedOpts {
                dirty: true,
                layer: false,
            },
            bg,
            60,
            seed,
        );
        assert!(p > 10 && l == 0);
        let (p, l) = run(
            RetainedOpts {
                dirty: false,
                layer: true,
            },
            bg,
            60,
            seed,
        );
        assert!(p == 0 && l > 5);
    }
}

#[test]
fn unchanged_frames_report_nothing_and_size_changes_repaint() {
    let sc = stress_scene(120, 80);
    let prims: Vec<Prim> = sc.prims().cloned().collect();
    let mut r = Retained::default();
    let mut s = Scratch::default();
    let o = RetainedOpts::default();
    assert_eq!(
        r.paint(120, 80, sc.background, prims.clone(), &mut s, o)
            .unwrap(),
        Some([0, 0, 120, 80])
    );
    assert_eq!(
        r.paint(120, 80, sc.background, prims.clone(), &mut s, o)
            .unwrap(),
        None
    );
    let big = stress_scene(160, 90);
    let p2: Vec<Prim> = big.prims().cloned().collect();
    assert_eq!(
        r.paint(160, 90, big.background, p2.clone(), &mut s, o)
            .unwrap(),
        Some([0, 0, 160, 90])
    );
    assert_eq!(
        r.pixmap().unwrap().data(),
        reference(160, 90, big.background, &p2).as_slice()
    );
    assert!(r.paint(0, 90, big.background, p2, &mut s, o).is_err());
}

/// (RP3/S) The shell's change check: every field a pixel depends on flips the scene
/// fingerprint (size, background, each field of each primitive kind, order, count), an
/// identical scene keeps it, and fields that do not reach the pixels (view id, mapping)
/// do not matter.
#[test]
fn scene_fingerprint_flips_on_any_pixel_field() {
    use qp_render::retained::scene_fingerprint;
    use qp_render::scene::Align;
    let c = [10, 20, 30, 255];
    let base = vec![
        Prim::Rect {
            x: 1,
            y: 2,
            w: 3,
            h: 4,
            color: c,
        },
        Prim::Polyline {
            points: vec![[1.0, 2.0], [3.0, 4.0]],
            width: 1.5,
            color: c,
        },
        Prim::Fill {
            points: vec![[0.0, 0.0], [5.0, 0.0], [0.0, 5.0]],
            color: c,
        },
        Prim::Circle {
            cx: 7.0,
            cy: 8.0,
            r: 2.0,
            color: c,
        },
        Prim::Text {
            x: 5,
            baseline: 9,
            px: 11.0,
            text: "ab".to_owned(),
            color: c,
            align: Align::Left,
        },
    ];
    let sc = |prims: Vec<Prim>| scene_of(40, 30, [0, 0, 0, 255], prims);
    let f0 = scene_fingerprint(&sc(base.clone()));
    assert_eq!(
        f0,
        scene_fingerprint(&sc(base.clone())),
        "identical scene, same key"
    );
    let mut other = sc(base.clone());
    other.view = "elsewhere".to_owned();
    other.x.slot += 1.0;
    assert_eq!(
        f0,
        scene_fingerprint(&other),
        "non-pixel fields do not matter"
    );
    // Size and background.
    assert_ne!(
        f0,
        scene_fingerprint(&scene_of(41, 30, [0, 0, 0, 255], base.clone()))
    );
    assert_ne!(
        f0,
        scene_fingerprint(&scene_of(40, 31, [0, 0, 0, 255], base.clone()))
    );
    assert_ne!(
        f0,
        scene_fingerprint(&scene_of(40, 30, [0, 0, 1, 255], base.clone()))
    );
    // Order and count.
    let mut swapped = base.clone();
    swapped.swap(0, 1);
    assert_ne!(f0, scene_fingerprint(&sc(swapped)));
    assert_ne!(f0, scene_fingerprint(&sc(base[..4].to_vec())));
    let mut more = base.clone();
    more.push(base[0].clone());
    assert_ne!(f0, scene_fingerprint(&sc(more)));
    // Every field of every primitive kind.
    let c2 = [10, 20, 31, 255];
    let edits: Vec<(usize, Prim)> = vec![
        (
            0,
            Prim::Rect {
                x: 9,
                y: 2,
                w: 3,
                h: 4,
                color: c,
            },
        ),
        (
            0,
            Prim::Rect {
                x: 1,
                y: 9,
                w: 3,
                h: 4,
                color: c,
            },
        ),
        (
            0,
            Prim::Rect {
                x: 1,
                y: 2,
                w: 9,
                h: 4,
                color: c,
            },
        ),
        (
            0,
            Prim::Rect {
                x: 1,
                y: 2,
                w: 3,
                h: 9,
                color: c,
            },
        ),
        (
            0,
            Prim::Rect {
                x: 1,
                y: 2,
                w: 3,
                h: 4,
                color: c2,
            },
        ),
        (
            1,
            Prim::Polyline {
                points: vec![[1.0, 2.0], [3.0, 4.5]],
                width: 1.5,
                color: c,
            },
        ),
        (
            1,
            Prim::Polyline {
                points: vec![[1.0, 2.0], [3.0, 4.0], [3.0, 4.0]],
                width: 1.5,
                color: c,
            },
        ),
        (
            1,
            Prim::Polyline {
                points: vec![[1.0, 2.0], [3.0, 4.0]],
                width: 1.25,
                color: c,
            },
        ),
        (
            1,
            Prim::Polyline {
                points: vec![[1.0, 2.0], [3.0, 4.0]],
                width: 1.5,
                color: c2,
            },
        ),
        (
            2,
            Prim::Fill {
                points: vec![[0.0, 0.0], [5.0, 0.0], [0.0, 5.5]],
                color: c,
            },
        ),
        (
            2,
            Prim::Fill {
                points: vec![[0.0, 0.0], [5.0, 0.0], [0.0, 5.0]],
                color: c2,
            },
        ),
        (
            3,
            Prim::Circle {
                cx: 7.5,
                cy: 8.0,
                r: 2.0,
                color: c,
            },
        ),
        (
            3,
            Prim::Circle {
                cx: 7.0,
                cy: 8.5,
                r: 2.0,
                color: c,
            },
        ),
        (
            3,
            Prim::Circle {
                cx: 7.0,
                cy: 8.0,
                r: 2.5,
                color: c,
            },
        ),
        (
            3,
            Prim::Circle {
                cx: 7.0,
                cy: 8.0,
                r: 2.0,
                color: c2,
            },
        ),
        (
            4,
            Prim::Text {
                x: 6,
                baseline: 9,
                px: 11.0,
                text: "ab".to_owned(),
                color: c,
                align: Align::Left,
            },
        ),
        (
            4,
            Prim::Text {
                x: 5,
                baseline: 8,
                px: 11.0,
                text: "ab".to_owned(),
                color: c,
                align: Align::Left,
            },
        ),
        (
            4,
            Prim::Text {
                x: 5,
                baseline: 9,
                px: 12.0,
                text: "ab".to_owned(),
                color: c,
                align: Align::Left,
            },
        ),
        (
            4,
            Prim::Text {
                x: 5,
                baseline: 9,
                px: 11.0,
                text: "ac".to_owned(),
                color: c,
                align: Align::Left,
            },
        ),
        (
            4,
            Prim::Text {
                x: 5,
                baseline: 9,
                px: 11.0,
                text: "ab".to_owned(),
                color: c2,
                align: Align::Left,
            },
        ),
        (
            4,
            Prim::Text {
                x: 5,
                baseline: 9,
                px: 11.0,
                text: "ab".to_owned(),
                color: c,
                align: Align::Right,
            },
        ),
        // A primitive of another kind in the same place.
        (
            0,
            Prim::Circle {
                cx: 1.0,
                cy: 2.0,
                r: 3.0,
                color: c,
            },
        ),
    ];
    let mut seen = vec![f0];
    for (i, p) in edits {
        let mut v = base.clone();
        v[i] = p;
        let f = scene_fingerprint(&sc(v));
        assert!(
            !seen.contains(&f),
            "edit of primitive {i} kept or reused a key"
        );
        seen.push(f);
    }
    // The stress scene: one bit of one coordinate anywhere flips it.
    let st = stress_scene(120, 80);
    let fs = scene_fingerprint(&st);
    let mut prims: Vec<Prim> = st.prims().cloned().collect();
    let last = prims.len() - 1;
    prims[last] = shift(&prims[last], 1.0, 0.0);
    assert_ne!(
        fs,
        scene_fingerprint(&scene_of(120, 80, st.background, prims))
    );
}
