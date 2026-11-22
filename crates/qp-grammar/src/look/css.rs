//! The shell chrome's CSS custom properties of a look ([`css_vars`]).

use super::*;

fn hex(c: Rgba) -> String {
    if c[3] == 255 {
        format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3])
    }
}

/// The shell chrome's CSS custom properties for `look` in scheme `scheme` (`None`: the
/// look's own): colours, fonts, sizes, radius, motion. The page sets them on the root
/// element verbatim; its stylesheet only refers to them (`shells/web/style.css`), so the
/// chrome holds no design value of its own.
#[must_use]
pub fn css_vars(look: &Look, scheme: Option<&str>) -> Vec<(String, String)> {
    let t = look.theme(scheme).unwrap_or(look.dark);
    let c = t.chrome;
    let body = look.role_font("body");
    let mut v: Vec<(String, String)> = vec![
        ("--qp-scheme".into(), t.name.to_owned()),
        ("--bg".into(), hex(c.bg)),
        ("--fg".into(), hex(c.fg)),
        ("--muted".into(), hex(c.muted)),
        ("--line".into(), hex(c.line)),
        ("--card".into(), hex(c.card)),
        ("--accent".into(), hex(c.accent)),
        ("--on-accent".into(), hex(c.on_accent)),
        ("--page".into(), hex(c.page)),
        ("--note-bg".into(), hex(c.note_bg)),
        ("--note-fg".into(), hex(c.note_fg)),
        ("--alert-bg".into(), hex(c.alert_bg)),
        ("--alert-fg".into(), hex(c.alert_fg)),
        ("--req".into(), hex(c.req)),
        ("--control".into(), hex(c.control)),
        (
            "--qp-font".into(),
            if body.id == DEFAULT_FONT && look.name == DEFAULT_LOOK {
                // The r4 chrome keeps the system font it always had.
                "system-ui, -apple-system, \"Segoe UI\", sans-serif".to_owned()
            } else {
                format!("\"{}\", system-ui, sans-serif", body.css_family)
            },
        ),
        ("--radius".into(), format!("{}px", look.surface.radius)),
        (
            "--frame-radius".into(),
            format!("{}px", look.surface.frame_radius),
        ),
        (
            "--pill-radius".into(),
            format!("{}px", look.surface.pill_radius),
        ),
        ("--motion".into(), format!("{}ms", look.motion.chrome_ms)),
        ("--ease".into(), look.motion.ease.css()),
    ];
    for (i, s) in look.space.scale.iter().enumerate() {
        v.push((format!("--qp-space-{i}"), format!("{s}px")));
    }
    // (R5/T3b) The chrome's sizes: the stylesheet holds no length of its own.
    let ch = &look.chrome;
    let px = |x: f64| format!("{x}px");
    // A least height of 0 is the browser's own `auto` (the computed style stays as before).
    let least = |x: f64| if x > 0.0 { px(x) } else { "auto".to_owned() };
    let pad = |p: [f64; 2]| format!("{}px {}px", p[0], p[1]);
    v.extend([
        ("--qp-rule".into(), px(look.surface.line)),
        ("--qp-fs".into(), px(ch.text_px)),
        ("--qp-lh".into(), format!("{}", ch.line)),
        ("--qp-fs-small".into(), px(ch.small_px)),
        ("--qp-fs-table".into(), px(ch.table_px)),
        ("--qp-fs-tiny".into(), px(ch.tiny_px)),
        ("--qp-fs-trust".into(), px(ch.trust_px)),
        ("--qp-head-weight".into(), format!("{}", ch.head_weight)),
        (
            "--qp-head-tracking".into(),
            format!("{}em", ch.head_tracking),
        ),
        ("--qp-head-gap".into(), px(ch.head_gap)),
        ("--qp-tab-h".into(), least(ch.tab_h)),
        ("--qp-tab-pad".into(), px(ch.tab_pad)),
        ("--qp-tab-rule".into(), px(ch.tab_rule)),
        ("--qp-control-h".into(), least(ch.control_h)),
        ("--qp-control-pad".into(), pad(ch.control_pad)),
        ("--qp-date-w".into(), px(ch.date_w)),
        ("--qp-bar-h".into(), px(ch.bar_h)),
        ("--qp-cell-pad".into(), pad(ch.cell_pad)),
        ("--qp-badge-pad".into(), pad(ch.badge_pad)),
        ("--qp-face-pad".into(), pad(ch.face_pad)),
        ("--qp-face-min-h".into(), px(ch.face_min_h)),
    ]);
    v
}
