//! The design check's thresholds, in one place (`../refs/reports/VISUAL.md` §5 P1–P13 and
//! `docs/DESIGN.md` §1.6, R5's hard thresholds).
//!
//! **Initial; adjusted once after Arkin's interim decision** (RD2 `GOAL.md` §1), then
//! written into `docs/design/CONTRACT.md`.

/// P1: greatest OKLCH chroma of a chrome surface or text role.
pub const P1_CHROMA_MAX: f64 = 0.02;
/// P2: most distinct tones in the fixture chart scene, and the ΔE00 that separates two.
pub const P2_TONES_MAX: usize = 6;
pub const P2_TONE_DE: f64 = 5.0;
/// P2: below this CIELAB chroma a colour counts as the one neutral tone.
pub const P2_NEUTRAL_C: f64 = 6.0;
/// P3: dark up/down: greatest HSL saturation, lightness range (percent).
pub const P3_SAT_MAX: f64 = 65.0;
pub const P3_LIGHT: (f64, f64) = (40.0, 60.0);
/// P4: price-axis label spacing (CSS px) and least labels in a 300 px-high plot.
pub const P4_SPACING: (f64, f64) = (40.0, 60.0);
pub const P4_PLOT_H: u32 = 300;
pub const P4_MIN_LABELS: usize = 4;
/// P5: least time-axis labels at this width (two levels required).
pub const P5_WIDTH: u32 = 390;
pub const P5_MIN_LABELS: usize = 4;
/// (RD2/C2a) The pointer's bar in every measured scene (P6, P7: the crosshair and its
/// pills are drawn and checked for overlaps).
pub const CURSOR_BAR: u64 = 120;
/// P6: text-box overlaps of axis labels.
pub const P6_OVERLAPS_MAX: usize = 0;
/// P8: greatest height of an on-plot semantic mark (px).
pub const P8_MARK_MAX: f64 = 28.0;
/// P9: desktop layout: least regions, chart panes, numbers in the legend.
pub const P9_REGIONS: u64 = 4;
pub const P9_PANES: u64 = 2;
pub const P9_LEGEND_NUMBERS: u64 = 5;
/// P10 dense: body and chart figure size ranges (px); R5's smallest text.
pub const P10_DENSE_BODY: (f64, f64) = (13.0, 14.0);
pub const P10_DENSE_FIGURE: (f64, f64) = (11.0, 12.0);
pub const P10_DENSE_SMALLEST: f64 = 11.0;
/// P10 touch: least body, smallest text, control height (px).
pub const P10_TOUCH_BODY: f64 = 15.0;
pub const P10_TOUCH_SMALLEST: f64 = 13.0;
pub const P10_TOUCH_CONTROL: f64 = 44.0;
/// P12: greatest control and card radius (px).
pub const P12_CONTROL_RADIUS: f64 = 8.0;
pub const P12_CARD_RADIUS: f64 = 12.0;
/// P13: two border colours closer than this ΔE00 are one role; hairline contrast range;
/// least control-boundary contrast.
pub const P13_SAME_ROLE_DE: f64 = 2.0;
pub const P13_HAIRLINE: (f64, f64) = (1.2, 1.6);
pub const P13_CONTROL: f64 = 3.0;
/// R5 hard thresholds: text contrast (large text 3:1 is not used: every role is small),
/// non-text contrast, semantic ΔE00 in all five vision conditions; pairs closer than
/// `H3_SHARED` (named, deliberately shared roles, `docs/DESIGN.md` §1.6) are not measured;
/// every other pair is, however close (an accidental up = down fails).
pub const H1_TEXT: f64 = 4.5;
pub const H2_NON_TEXT: f64 = 3.0;
pub const H3_SEMANTIC_DE: f64 = 10.0;
pub const H3_SHARED: [(&str, &str); 4] = [
    ("valid", "text"),
    ("invalid", "closed"),
    ("invalid", "neutral"),
    ("closed", "neutral"),
];
/// Frame sizes: the desktop chart cell and the phone chart.
pub const DESK: (u32, u32) = (960, 600);
pub const PHONE_H: u32 = 300;
