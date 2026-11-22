//! (RD2 grey fix) R5's text and non-text thresholds (H1, H2) against **every ground a role
//! is drawn on**, not only the page: the look's chart surfaces and its translucent strips
//! and bands composited over the background (`rounds/RD2/experiments/E1/RESULTS.md` 1b:
//! the grey role failed against the emphasis strip, the evidence band and its own plate,
//! and `down` against its own volume bar on the strip, while H2 measured bg and surface).

use qp_grammar::compile::ALPHABET_FIXED;
use qp_grammar::look::Look;
use qp_grammar::theme::Theme;
use qp_schema::scene::Rgba;

use crate::color::{contrast, over};
use crate::thresholds as th;
use crate::Row;

fn alpha(c: Rgba, a: u8) -> Rgba {
    [c[0], c[1], c[2], a]
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn opacity(o: f64) -> u8 {
    (255.0 * o.clamp(0.0, 1.0)).round() as u8
}

/// The volume bars' opacity (a fixed alphabet measure).
fn volume_alpha() -> u8 {
    let v = ALPHABET_FIXED
        .iter()
        .find(|(n, _)| *n == "VOLUME_ALPHA")
        .map_or(1.0, |(_, v)| v[0]);
    opacity(v)
}

/// The grounds every chart, face and story element can stand on, opaque: the background,
/// the surface and card, the emphasis strip (accent at `strip_alpha`), the selected story's
/// row band (focus at `focus_row_alpha`), the evidence band (its role at `band_alpha`) on
/// the background and on the strip.
pub fn grounds(l: &Look, t: &Theme) -> Vec<(&'static str, Rgba)> {
    let bg = t.bg;
    let strip = over(alpha(t.accent, l.surface.strip_alpha), bg);
    let ev = t.token(l.evidence.color).unwrap_or(t.muted);
    let ev = alpha(ev, opacity(l.evidence.band_alpha));
    vec![
        ("bg", bg),
        ("surface", t.surface),
        ("card", over(t.chrome.card, bg)),
        ("strip", strip),
        (
            "row band",
            over(alpha(t.focus, l.story.focus_row_alpha), bg),
        ),
        ("evidence band", over(ev, bg)),
        ("evidence band on strip", over(ev, strip)),
    ]
}

/// Grounds that belong to one role's own marks: a grey mark's plate (at most
/// `grey_plate_alpha_max`) and an up/down colour's volume bar, under it.
fn own(l: &Look, t: &Theme, role: &str, c: Rgba) -> Vec<(&'static str, Rgba)> {
    let strip = over(alpha(t.accent, l.surface.strip_alpha), t.bg);
    match role {
        "grey" | "neutral" | "invalid" => vec![
            ("own plate", over(alpha(c, t.grey_plate_alpha_max), t.bg)),
            (
                "own plate on surface",
                over(alpha(c, t.grey_plate_alpha_max), t.surface),
            ),
        ],
        "up" | "down" => vec![
            ("own volume", over(alpha(c, volume_alpha()), t.bg)),
            ("own volume on strip", over(alpha(c, volume_alpha()), strip)),
        ],
        _ => Vec::new(),
    }
}

/// The lowest contrast of `roles` over `grounds` (each role also over its own grounds).
fn worst(
    l: &Look,
    t: &Theme,
    roles: &[(&'static str, Rgba)],
    grounds: &[(&'static str, Rgba)],
) -> (String, f64) {
    let mut low = (String::from("-"), f64::INFINITY);
    for (r, c) in roles {
        for (g, b) in grounds.iter().chain(own(l, t, r, *c).iter()) {
            let k = contrast(*c, *b);
            if k < low.1 {
                low = (format!("{r}/{g}"), k);
            }
        }
    }
    low
}

/// H1: every text role 4.5:1 on its ground: chart text roles (text, muted, a closed
/// story's label) on every chart ground; the shell's pairs on their own grounds.
pub fn h1(l: &Look, t: &Theme) -> Row {
    let c = t.chrome;
    let label = t.token(l.story.closed_label).unwrap_or(t.closed);
    let chart = [
        ("text", t.text),
        ("muted", t.muted),
        ("closed label", label),
    ];
    let mut low = worst(l, t, &chart, &grounds(l, t));
    for (n, f, b) in [
        ("chrome fg/bg", c.fg, c.bg),
        ("chrome muted/bg", c.muted, c.bg),
        ("chrome fg/card", c.fg, c.card),
        ("chrome muted/card", c.muted, c.card),
        ("on_accent/accent", c.on_accent, c.accent),
        ("note", c.note_fg, c.note_bg),
        ("alert", c.alert_fg, c.alert_bg),
    ] {
        let k = contrast(f, b);
        if k < low.1 {
            low = (n.to_owned(), k);
        }
    }
    let (name, k) = low;
    Row::check(
        "H1",
        t.name,
        "text contrast, every ground (worst)",
        format!("{k:.2} ({name})"),
        format!(">= {}", th::H1_TEXT),
        k >= th::H1_TEXT,
    )
}

/// H2: every non-text meaning and origin role 3:1 on every ground it is drawn on: the
/// grey role (`neutral` = `invalid` = `closed`), up and down, axis, focus and the origins.
pub fn h2(l: &Look, t: &Theme) -> Row {
    let o = l.origin_roles(t);
    let roles = [
        ("grey", t.closed),
        ("neutral", t.neutral),
        ("invalid", t.invalid),
        ("up", t.up),
        ("down", t.down),
        ("axis", t.axis),
        ("focus", t.focus),
        ("origin.human", o.human),
        ("origin.agent", o.agent),
        ("origin.engine", o.engine),
    ];
    let (name, k) = worst(l, t, &roles, &grounds(l, t));
    Row::check(
        "H2",
        t.name,
        "non-text contrast, every ground (worst)",
        format!("{k:.2} ({name})"),
        format!(">= {}", th::H2_NON_TEXT),
        k >= th::H2_NON_TEXT,
    )
}
