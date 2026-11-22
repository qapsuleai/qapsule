//! (RD2/C2a) The chart alphabet measured from the scene on a test-only look (dense profile,
//! chart conventions on, plausible values; not registered: the direction agents register
//! product looks): P4–P8 pass, axis-label overlaps are 0, the texts speak Turkish, the
//! volume histogram sits under the candles.
//!
//! `QP_DESIGN_SCENE_OUT=<file>` also writes the desk scene (960 × 600, dark, cursor) as
//! JSON for a look at it.

use qp_grammar::look::{ChartLook, Look, Profile, TypeRole, TypeScale, ValueLine, R4};
use qp_grammar::{compile, CompileOptions, Cursor, MemProvider};
use qp_schema::scene::{Prim, SceneDesc};
use qp_schema::Bar;

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

/// (C2a-fix) Each pane with a value scale keeps at least one axis label (a short volume or
/// RSI pane included); P6 above counts any overlap with the last-value tag.
#[test]
fn every_scaled_pane_keeps_a_label() {
    let fx = scene::fixture();
    let s = fx.scene(&TRIAL, "dark", 960, 600);
    for p in &s.panels {
        let n = s
            .under
            .iter()
            .filter(|t| {
                matches!(t, Prim::Text { x, baseline, role: Some(r), .. }
                if r == "figure" && *x > p.rect.right
                    && (p.rect.top..=p.rect.bottom).contains(baseline))
            })
            .count();
        assert!(n >= 1, "pane {:?} has no axis label", p.rect);
    }
}

/// (C2a-fix) Clock labels and the date pill read the view's display zone (K14): BIST bars
/// from 06:30 UTC are labelled from 09:30 Istanbul time, never 06:xx–08:xx.
#[test]
fn intraday_labels_read_the_display_zone() {
    // 2025-07-03 06:30 UTC, 80 five-minute bars.
    let bars: Vec<Bar> = (0..80)
        .map(|i| Bar {
            t: 1_751_524_200_000 + i * 300_000,
            o: 10.0,
            h: 11.0,
            l: 9.0,
            c: 10.5,
            v: 1e3,
        })
        .collect();
    let p = MemProvider::new().with_bars("px", bars, Some("THYAO"), Some("5m"));
    let compiled = |tz: Option<&str>| {
        let view = serde_json::json!({"id": "v", "display_tz": tz, "x": {"kind": "time"},
            "layers": [{"id": "c", "mark": "candle", "panel": 0, "series": "px"}],
            "panels": [{"id": 0, "y": {"kind": "linear"}}]});
        let mut o = CompileOptions::new(960, 400);
        o.look_record = Some(&TRIAL);
        o.cursor = Some(Cursor {
            bar: 0,
            price: None,
        });
        let s = compile(&serde_json::from_value(view).unwrap(), &p, &o).unwrap();
        texts(&s).into_iter().map(str::to_owned).collect::<Vec<_>>()
    };
    let early = |t: &[String]| {
        t.iter()
            .any(|x| x.len() == 5 && x[..2] < *"09" && x.contains(':'))
    };
    let utc = compiled(None);
    assert!(early(&utc), "the UTC reading has 06–08 h labels: {utc:?}");
    let ist = compiled(Some("Europe/Istanbul"));
    assert!(!early(&ist) && ist.iter().any(|x| x == "12:00"), "{ist:?}");
    assert!(ist.iter().any(|x| x == "3 Tem 09:30"), "date pill: {ist:?}");
}

/// (C2a-fix) The fixed alphabet measures listed in `docs/design/CONTRACT.md` ("Sabit
/// alfabe ölçüleri") equal the code's constants, one row per constant.
#[test]
fn contract_lists_the_fixed_measures() {
    let p = format!(
        "{}/../../docs/design/CONTRACT.md",
        env!("CARGO_MANIFEST_DIR")
    );
    let doc = std::fs::read_to_string(p).unwrap();
    let sec = doc
        .split("### Sabit alfabe ölçüleri")
        .nth(1)
        .expect("section");
    let sec = sec.split("\n#").next().unwrap();
    let rows: Vec<Vec<&str>> = sec
        .lines()
        .filter(|l| l.starts_with("| `"))
        .map(|l| l.split('|').map(|c| c.trim().trim_matches('`')).collect())
        .collect();
    assert_eq!(
        rows.len(),
        qp_grammar::compile::ALPHABET_FIXED.len(),
        "{rows:?}"
    );
    for (name, vals) in qp_grammar::compile::ALPHABET_FIXED {
        let row = rows.iter().find(|r| r[1] == name).expect(name);
        let doc: Vec<f64> = row[2]
            .split(',')
            .map(|v| v.trim().parse().unwrap())
            .collect();
        assert_eq!(doc, vals, "{name}");
    }
}
