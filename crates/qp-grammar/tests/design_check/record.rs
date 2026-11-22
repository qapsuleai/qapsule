//! Checks measured from the look record alone: P1, P3, P9–P13 and R5's H3 (H1, H2:
//! `grounds.rs`).

use qp_grammar::look::{Look, Profile, ROLES};
use qp_grammar::theme::Theme;
use qp_schema::scene::Rgba;

use crate::color::{contrast, de, hsl_sl, oklch_c, Vision};
use crate::thresholds as th;
use crate::Row;

/// P1: chrome surface and text roles are neutral.
pub fn p1(t: &Theme) -> Row {
    let c = t.chrome;
    let roles = [
        ("chrome.bg", c.bg),
        ("chrome.fg", c.fg),
        ("chrome.muted", c.muted),
        ("chrome.line", c.line),
        ("chrome.card", c.card),
        ("chrome.page", c.page),
        ("chrome.control", c.control),
        ("bg", t.bg),
        ("surface", t.surface),
        ("text", t.text),
        ("muted", t.muted),
        ("grid", t.grid),
        ("axis", t.axis),
        ("line", t.line),
    ];
    let (name, worst) = roles
        .iter()
        .map(|(n, c)| (*n, oklch_c(*c)))
        .fold(("", 0.0), |a, b| if b.1 > a.1 { b } else { a });
    Row::check(
        "P1",
        t.name,
        "chrome surface+text OKLCH C",
        format!("{worst:.3} ({name})"),
        format!("<= {}", th::P1_CHROMA_MAX),
        worst <= th::P1_CHROMA_MAX,
    )
}

/// P3: the dark scheme's up and down are calm (not neon).
pub fn p3(t: &Theme) -> Row {
    if t.name != "dark" {
        return Row::na("P3", t.name, "up/down HSL S, L (dark only)");
    }
    let (us, ul) = hsl_sl(t.up);
    let (ds, dl) = hsl_sl(t.down);
    let ok = |s: f64, l: f64| s <= th::P3_SAT_MAX && (th::P3_LIGHT.0..=th::P3_LIGHT.1).contains(&l);
    Row::check(
        "P3",
        t.name,
        "up/down HSL S, L",
        format!("up {us:.0}/{ul:.0}, down {ds:.0}/{dl:.0}"),
        format!(
            "S <= {}, L {}-{}",
            th::P3_SAT_MAX,
            th::P3_LIGHT.0,
            th::P3_LIGHT.1
        ),
        ok(us, ul) && ok(ds, dl),
    )
}

/// P9: the desktop layout dump (`--layout`), if one was given.
pub fn p9(layout: Option<&serde_json::Value>) -> Row {
    let rule = "layout regions / panes / legend numbers";
    let Some(l) = layout else {
        return Row::na("P9", "-", rule);
    };
    let n = |k: &str| l.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
    let (r, p, g) = (n("regions"), n("chart_panes"), n("legend_numbers"));
    Row::check(
        "P9",
        "-",
        rule,
        format!("{r} / {p} / {g}"),
        format!(
            ">= {} / {} / {}",
            th::P9_REGIONS,
            th::P9_PANES,
            th::P9_LEGEND_NUMBERS
        ),
        r >= th::P9_REGIONS && p >= th::P9_PANES && g >= th::P9_LEGEND_NUMBERS,
    )
}

/// The smallest text of a look: every type role and every chrome size.
fn smallest(l: &Look) -> f64 {
    let c = &l.chrome;
    ROLES
        .iter()
        .filter_map(|r| l.types.role(r).map(|r| r.px))
        .chain([c.text_px, c.small_px, c.table_px, c.tiny_px, c.trust_px])
        .fold(f64::INFINITY, f64::min)
}

/// P10: type sizes and targets for the look's profile (a legacy look as dense).
pub fn p10(l: &Look) -> Row {
    let body = l.chrome.text_px;
    let figure = l.types.figure.px;
    let small = smallest(l);
    let control = l.chrome.control_h.min(l.chrome.tab_h);
    let within = |x: f64, r: (f64, f64)| (r.0..=r.1).contains(&x);
    if l.profile == Profile::Touch {
        return Row::check(
            "P10",
            "-",
            "touch: body / smallest / control px",
            format!("{body} / {small} / {control}"),
            format!(
                ">= {} / {} / {}",
                th::P10_TOUCH_BODY,
                th::P10_TOUCH_SMALLEST,
                th::P10_TOUCH_CONTROL
            ),
            body >= th::P10_TOUCH_BODY
                && small >= th::P10_TOUCH_SMALLEST
                && control >= th::P10_TOUCH_CONTROL,
        );
    }
    Row::check(
        "P10",
        "-",
        "dense: body / figure / smallest px",
        format!("{body} / {figure} / {small}"),
        format!(
            "{}-{} / {}-{} / >= {}",
            th::P10_DENSE_BODY.0,
            th::P10_DENSE_BODY.1,
            th::P10_DENSE_FIGURE.0,
            th::P10_DENSE_FIGURE.1,
            th::P10_DENSE_SMALLEST
        ),
        within(body, th::P10_DENSE_BODY)
            && within(figure, th::P10_DENSE_FIGURE)
            && small >= th::P10_DENSE_SMALLEST,
    )
}

/// P11: the figure font's digits are tabular (equal advances).
pub fn p11(l: &Look) -> Row {
    let f = l.role_font("figure");
    let w: Vec<i32> = (0..10).map(|d| f.width(&d.to_string(), 1000.0)).collect();
    let (lo, hi) = (
        w.iter().copied().min().unwrap_or(0),
        w.iter().copied().max().unwrap_or(0),
    );
    Row::check(
        "P11",
        "-",
        "figure font 0-9 advance spread (1000 px)",
        format!("{} ({}: {lo}-{hi})", hi - lo, f.id),
        "0".to_owned(),
        hi == lo,
    )
}

/// P12: control and card radius.
pub fn p12(l: &Look) -> Row {
    let (r, c) = (l.surface.radius, l.surface.card_radius);
    Row::check(
        "P12",
        "-",
        "control / card radius px",
        format!("{r} / {c}"),
        format!("<= {} / {}", th::P12_CONTROL_RADIUS, th::P12_CARD_RADIUS),
        r <= th::P12_CONTROL_RADIUS && c <= th::P12_CARD_RADIUS,
    )
}

/// P13: one border role, a hairline against the background, controls bounded at 3:1.
pub fn p13(t: &Theme) -> Row {
    let c = t.chrome;
    let one = de(c.line, t.line, Vision::Normal) < th::P13_SAME_ROLE_DE;
    let hair = contrast(c.line, c.bg);
    let ctl = contrast(c.control, c.bg).min(contrast(c.control, c.card));
    Row::check(
        "P13",
        t.name,
        "border roles / line:bg / control:bg,card",
        format!("{} / {hair:.2} / {ctl:.2}", if one { 1 } else { 2 }),
        format!(
            "1 / {}-{} / >= {}",
            th::P13_HAIRLINE.0,
            th::P13_HAIRLINE.1,
            th::P13_CONTROL
        ),
        one && (th::P13_HAIRLINE.0..=th::P13_HAIRLINE.1).contains(&hair) && ctl >= th::P13_CONTROL,
    )
}

/// H3: meaning roles (up, down, valid, invalid) and origin roles (human, agent, engine)
/// ΔE00 ≥ 10 apart in normal, grey, protan, deutan and tritan vision.
pub fn h3(l: &Look, t: &Theme) -> Row {
    let o = l.origin_roles(t);
    let op = |c: Rgba| crate::color::over(c, t.bg);
    let sets = [
        vec![
            ("up", op(t.up)),
            ("down", op(t.down)),
            ("valid", op(t.valid)),
            ("invalid", op(t.invalid)),
        ],
        vec![
            ("human", op(o.human)),
            ("agent", op(o.agent)),
            ("engine", op(o.engine)),
        ],
    ];
    let mut low = (String::from("-"), f64::INFINITY);
    for set in &sets {
        for (i, (a, ca)) in set.iter().enumerate() {
            for (b, cb) in &set[i + 1..] {
                let shared = |x: &str, y: &str| (*a, *b) == (x, y) || (*a, *b) == (y, x);
                if th::H3_SHARED.iter().any(|(x, y)| shared(x, y)) {
                    continue;
                }
                for v in Vision::ALL {
                    let d = de(*ca, *cb, v);
                    if d < low.1 {
                        low = (format!("{a}/{b} {}", v.name()), d);
                    }
                }
            }
        }
    }
    Row::check(
        "H3",
        t.name,
        "semantic + origin ΔE00, 5 conditions",
        format!("{:.1} ({})", low.1, low.0),
        format!(">= {}", th::H3_SEMANTIC_DE),
        low.1 >= th::H3_SEMANTIC_DE,
    )
}
