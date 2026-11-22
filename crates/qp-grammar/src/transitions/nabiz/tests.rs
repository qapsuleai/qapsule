use super::*;
use crate::look::{ORNEK, R4};
use qp_schema::scene::XMap;

fn r(l: i32, t: i32, rr: i32, b: i32) -> PixelRect {
    PixelRect {
        left: l,
        top: t,
        right: rr,
        bottom: b,
    }
}

fn sh() -> Vec<SharedElement> {
    vec![
        SharedElement {
            id: "glyph".into(),
            before: Some(r(8, 34, 40, 66)),
            after: Some(r(20, 40, 60, 80)),
        },
        SharedElement {
            id: WHOLE_IN.into(),
            before: Some(r(0, 0, 390, 188)),
            after: None,
        },
        SharedElement {
            id: WHOLE_AFTER.into(),
            before: None,
            after: Some(r(0, 0, 390, 600)),
        },
    ]
}

#[test]
fn frame_is_a_deepening_cross_fade() {
    let s = sh();
    let f0 = frame(&s, (0.0, 100.0), (0.0, 50.0), 0.0);
    assert_eq!(f0.elements.len(), 2);
    // Same list at every p.
    for p in [0.25, 0.5, 0.75, 1.0] {
        let f = frame(&s, (0.0, 100.0), (0.0, 50.0), p);
        assert_eq!(f.elements.len(), 2);
        assert_eq!(f.elements[0].id, f0.elements[0].id);
    }
    // p = 0: old at its place, opaque; new invisible.
    assert_eq!(f0.elements[0].rect, [0.0, 100.0, 390.0, 188.0]);
    assert!((f0.elements[0].opacity - 1.0).abs() < 1e-12);
    assert!(f0.elements[1].opacity.abs() < 1e-12);
    assert!(f0.scene_opacity.abs() < 1e-12);
    // p = 1: new at its place, opaque; real scene shown; old gone.
    let f1 = frame(&s, (0.0, 100.0), (0.0, 50.0), 1.0);
    assert_eq!(f1.elements[1].rect, [0.0, 50.0, 390.0, 600.0]);
    assert!(f1.elements[0].opacity.abs() < 1e-12);
    assert!((f1.scene_opacity - 1.0).abs() < 1e-12);
    // In between: the old scene grew about its glyph (nothing flies: the glyph's
    // centre stays where it was).
    let fm = frame(&s, (0.0, 100.0), (0.0, 50.0), 0.5);
    let e = &fm.elements[0];
    assert!(e.rect[2] > 390.0);
    let gx = 24.0; // glyph centre x in the old scene
    let k = e.rect[2] / 390.0;
    assert!((e.rect[0] + gx * k - gx).abs() < 1e-9);
    // Determinism.
    assert_eq!(fm, frame(&s, (0.0, 100.0), (0.0, 50.0), 0.5));
}

#[test]
fn closing_shrinks() {
    let mut s = sh();
    s[1].id = WHOLE_OUT.into();
    let f = frame(&s, (0.0, 0.0), (0.0, 0.0), 0.5);
    assert!(f.elements[0].rect[2] < 390.0);
    assert!(f.elements[1].rect[2] > 390.0);
}

#[test]
fn changes() {
    use Stage::*;
    assert_eq!(
        Change::between(None, (Born, Validity::Valid)),
        Some(Change::Born)
    );
    assert_eq!(Change::between(None, (Strengthened, Validity::Valid)), None);
    assert_eq!(
        Change::between(Some((Born, Validity::Valid)), (Born, Validity::Valid)),
        None
    );
    assert_eq!(
        Change::between(Some((Born, Validity::Valid)), (Confirmed, Validity::Valid)),
        Some(Change::Strengthened)
    );
    assert_eq!(
        Change::between(Some((Born, Validity::Valid)), (Born, Validity::NoLonger)),
        Some(Change::Invalid)
    );
    assert_eq!(
        Change::between(Some((Born, Validity::Valid)), (Realized, Validity::Closed)),
        Some(Change::Closed)
    );
    assert!(is_live(Born, Validity::Valid));
    assert!(!is_live(Born, Validity::NoLonger));
    assert!(!is_live(Invalidated, Validity::Closed));
    assert_eq!(label(Born, Validity::Valid, Language::Tr), Some("doğdu"));
    assert_eq!(
        label(Weakened, Validity::NoLonger, Language::En),
        Some("no longer holds")
    );
}

fn scene() -> SceneDesc {
    SceneDesc {
        view: "v".into(),
        width: 390,
        height: 188,
        background: [0, 0, 0, 255],
        x: XMap {
            from_bar: 0,
            to_bar: 0,
            left: 8.0,
            slot: 1.0,
        },
        bars: 1,
        lod: None,
        panels: vec![],
        emphasis: vec![],
        under: vec![],
        over: vec![],
        terms: vec![],
    }
}

fn places_r4() -> Places {
    Places {
        glyph: r(8, 34, 40, 66),
        dot: (11.0, 17.0, 3.0),
        label_at: (20, 21),
        row: (10, 24),
    }
}

#[test]
fn overlay_pulse_beat_trail() {
    let th = R4.dark;
    let mut fm = FaceMotion {
        places: places_r4(),
        live: true,
        direction: Direction::Up,
        beat: None,
        label_w: 40,
        t: 0.0,
    };
    // Pulse: invisible at phase 0, strongest at half the period.
    let a = overlay(&fm, &R4, &th);
    assert_eq!(a.len(), 1);
    assert!(a[0].opacity.abs() < 1e-12);
    fm.t = PULSE_MS / 2.0;
    let b = overlay(&fm, &R4, &th);
    assert!((b[0].opacity - PULSE_ALPHA).abs() < 1e-12);
    // Not live: still.
    fm.live = false;
    assert!(overlay(&fm, &R4, &th).is_empty());
    // Beats have distinct forms.
    let at = |c: Change, age: f64| {
        let mut f = fm;
        f.beat = Some(Beat { change: c, age });
        overlay(&f, &R4, &th)
    };
    let born = at(Change::Born, 450.0);
    let strong = at(Change::Strengthened, 450.0);
    let inv = at(Change::Invalid, 450.0);
    assert!(born[0].scale > 1.0 && born.iter().filter(|b| b.key.starts_with("beat")).count() == 1);
    assert_eq!(
        strong.iter().filter(|b| b.key.starts_with("beat")).count(),
        2
    );
    assert!(inv[0].scale > 1.0 && at(Change::Invalid, 800.0)[0].scale < inv[0].scale);
    assert!(born[0].scale < at(Change::Born, 800.0)[0].scale);
    // Trail: present after the beat, gone after the trail.
    let tr = at(Change::Born, BEAT_MS + 100.0);
    assert!(tr.iter().any(|b| b.key == "trail" && b.opacity > 0.0));
    assert!(beat_running(BEAT_MS + 100.0, &R4));
    assert!(!beat_running(BEAT_MS + TRAIL_MS, &R4));
    // Determinism.
    assert_eq!(at(Change::Weakened, 300.0), at(Change::Weakened, 300.0));
}

fn lum(c: [f64; 3]) -> f64 {
    let ch = |v: f64| {
        let v = v / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])
}

fn contrast(fg: Rgba, bg: Rgba) -> f64 {
    let a = f64::from(fg[3]) / 255.0;
    let mix = |i: usize| f64::from(bg[i]) + a * (f64::from(fg[i]) - f64::from(bg[i]));
    let (l1, l2) = (
        lum([mix(0), mix(1), mix(2)]),
        lum([0, 1, 2].map(|i| f64::from(bg[i]))),
    );
    (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05)
}

/// The still marks keep the thresholds in the reference and the trial look: the change
/// label (small text) 4.5:1, the live dot (a non-text mark) 3:1, on the scene's
/// background, in both schemes.
#[test]
fn still_marks_keep_contrast() {
    let mut bad = Vec::new();
    for look in [&R4, &ORNEK] {
        for t in [look.dark, look.light] {
            for c in [
                None,
                Some(Change::Born),
                Some(Change::Strengthened),
                Some(Change::Weakened),
                Some(Change::Invalid),
                Some(Change::Closed),
            ] {
                let k = contrast(change_color(c, &t), t.bg);
                if k < 4.5 {
                    bad.push(format!("{} {} label {c:?} {k:.2}", look.name, t.name));
                }
            }
            for d in [Direction::Up, Direction::Down, Direction::Neutral] {
                let k = contrast(pulse_color(d, &t), t.bg);
                if k < 3.0 {
                    bad.push(format!("{} {} dot {d:?} {k:.2}", look.name, t.name));
                }
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn still_marks_from_the_look() {
    let mut s = scene();
    let m = FaceMarks {
        live: true,
        label: Some("doğdu"),
        change: Some(Change::Born),
        direction: Direction::Up,
        live_word: None,
    };
    // No glyph: nothing drawn.
    draw_face(&mut s, &m, &R4, &R4.dark);
    assert!(s.over.is_empty());
    let _ = ORNEK;
}
