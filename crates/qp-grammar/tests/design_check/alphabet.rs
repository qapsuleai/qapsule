//! (RD2/C2a) The chart alphabet measured from the scene on a test-only look (dense profile,
//! chart conventions on, plausible values; not registered: the direction agents register
//! product looks): P4–P8 pass, axis-label overlaps are 0, the texts speak Turkish, the
//! volume histogram sits under the candles.
//!
//! `QP_DESIGN_SCENE_OUT=<file>` also writes the desk scene (960 × 600, dark, cursor) as
//! JSON for a look at it.

use qp_grammar::look::{ChartLook, Look, Profile, TypeRole, TypeScale, ValueLine, R4};
use qp_schema::scene::{Prim, SceneDesc};

use crate::{scene, thresholds as th, Res};

/// A dense look with every chart convention on (R4's values otherwise).
pub const TRIAL: Look = Look {
    name: "c2a-trial",
    profile: Profile::Dense,
    types: TypeScale {
        note: TypeRole {
            px: 11.0,
            ..R4.types.note
        },
        ..R4.types
    },
    chart: ChartLook {
        price_label_px: 50.0,
        two_level_time: true,
        last_value_tag: true,
        value_line: ValueLine::Dotted,
        crosshair_pills: true,
        ohlc_legend: true,
        volume_share: 0.18,
        mark_max_px: 28.0,
        body_ratio: 0.8,
        axis_gutter_px: 64.0,
    },
    ..R4
};

fn texts(s: &SceneDesc) -> Vec<&str> {
    s.under
        .iter()
        .chain(&s.over)
        .filter_map(|p| match p {
            Prim::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn alphabet_passes_p4_to_p8() {
    let fx = scene::fixture();
    let (dw, dh) = th::DESK;
    let mut rows = Vec::new();
    for scheme in ["dark", "light"] {
        let desk = fx.scene(&TRIAL, scheme, dw, dh);
        let phone = fx.scene(&TRIAL, scheme, th::P5_WIDTH, th::PHONE_H);
        let plot = fx.price_only(&TRIAL, scheme, th::P5_WIDTH, th::P4_PLOT_H);
        let s: &'static str = if scheme == "dark" { "dark" } else { "light" };
        rows.extend([
            scene::p4(&desk, &plot, s),
            scene::p5(&phone, &TRIAL, s),
            scene::p6(&desk, &phone, s),
            scene::p7(&desk, &TRIAL, s),
            scene::p8(&desk, &TRIAL, s),
        ]);
    }
    crate::print_table(&TRIAL, &rows);
    let failed: Vec<_> = rows.iter().filter(|r| r.res != Res::Pass).collect();
    assert!(failed.is_empty(), "{failed:#?}");

    let desk = fx.scene(&TRIAL, "dark", dw, dh);
    if let Ok(p) = std::env::var("QP_DESIGN_SCENE_OUT") {
        std::fs::write(&p, serde_json::to_string(&desk).unwrap()).unwrap();
    }
    let t = texts(&desk);
    // Turkish: decimal comma, month abbreviations, no ISO month labels; OHLC legend.
    assert!(
        t.iter().any(|x| x.contains(',') && !x.contains('.')),
        "{t:?}"
    );
    assert!(
        t.iter().any(|x| ["Şub", "Mar", "Nis", "Haz"].contains(x)),
        "{t:?}"
    );
    assert!(!t.iter().any(|x| x.starts_with("2025-")), "{t:?}");
    for k in ["O", "H", "L", "C"] {
        assert!(t.contains(&k), "legend letter {k}: {t:?}");
    }
    // The view's volume pane: compact figures, soft bars.
    assert!(t.iter().any(|x| x.ends_with(" Mn")), "{t:?}");
    // Without a volume layer the histogram goes under the candles of the price panel.
    let plot = fx.price_only(&TRIAL, "dark", dw, 400);
    let r = plot.panels[0].rect;
    let soft = plot
        .under
        .iter()
        .filter(|p| matches!(p, Prim::Rect { color, y, h, .. } if color[3] < 255 && color[3] > 0 && *y + i32::try_from(*h).unwrap() == r.bottom))
        .count();
    assert!(soft > 100, "volume bars under the candles: {soft}");
    // P8: a mark asked to be 40 px is drawn at the cap.
    assert!(TRIAL.mark_px(40.0) <= th::P8_MARK_MAX);
}

#[test]
fn alphabet_is_deterministic_and_english_aware() {
    let fx = scene::fixture();
    let a = fx.scene(&TRIAL, "dark", 960, 600);
    let b = fx.scene(&TRIAL, "dark", 960, 600);
    assert_eq!(a, b);
    let en = fx.scene_en(&TRIAL, 960, 600);
    let t = texts(&en);
    assert!(
        t.iter().any(|x| ["Feb", "Apr", "Jun", "Aug"].contains(x)),
        "{t:?}"
    );
    assert!(t.iter().any(|x| x.ends_with('M')), "{t:?}");
}
