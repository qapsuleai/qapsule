//! The look [`OLTAN`].

use super::*;

/// (R5/M16) **`oltan`** — "the mark leads / watch face" (`rounds/R5/experiments/M16/oltan/
/// DESIGN.md`). The meaning glyph is the face's hero: the largest and highest-contrast
/// thing on the screen (with the face design `oltan`, 120 px), its colour a lightness
/// ladder so direction and validity survive grey and the three colour-blind simulations
/// (ΔE00 ≥ 10 between every meaning pair, `oltan_meets_the_hard_thresholds`); the
/// sentence is secondary but easy to read: Atkinson Hyperlegible Next (a typeface made
/// for legibility at a glance, open shapes, distinct 0/O, 1/l/I), semibold headline, no
/// text under 13 px. Faces are tiles on a near-black ground (dark, the default) or on a
/// warm grey page (light), like complications on a watch face.
///
/// The ladder (CIE L*): dark — failed/no longer/no direction 47 < down 59 < up 80 < ink
/// 97; light — the same order reversed in salience: grey 57 > up 43 > down 28 > ink 6.
/// "No direction" (`neutral`) is the same grey as "no longer holds" (`invalid`): colour
/// means direction; grey means none (the badge and the words say which). `invalid` is
/// what `tg5`'s faded `muted` glyph composites to (`muted` × 0.55 over the ground), so a
/// failed and a lapsed story are one colour told apart by shape (✗ / ring with a bar).
pub const OLTAN: Look = Look {
    name: "oltan",
    note: "M16: the mark leads (watch face): 120 px glyph, lightness-ladder meaning colours, \
           Atkinson Hyperlegible Next",
    scheme: "dark",
    types: TypeScale {
        headline: TypeRole {
            px: 19.0,
            line: 1.27,
            font: "atkinson-next-semibold",
        },
        title: TypeRole {
            px: 15.0,
            line: 1.3,
            font: "atkinson-next",
        },
        body: TypeRole {
            px: 16.0,
            line: 1.375,
            font: "atkinson-next",
        },
        label: TypeRole {
            px: 14.0,
            line: 1.3,
            font: "atkinson-next",
        },
        figure: TypeRole {
            px: 13.0,
            line: 1.3,
            font: "atkinson-next",
        },
        note: TypeRole {
            px: 13.0,
            line: 1.3,
            font: "atkinson-next",
        },
    },
    dark: Theme {
        name: "dark",
        bg: [18, 19, 23, 255],
        grid: [36, 38, 44, 255],
        // (R5/fix47, A46) A chart's axis line is a non-text element: ≥ 3:1 (WCAG 1.4.11),
        // 3.23:1 here (was 2.95). The grid stays decorative (1.23:1).
        axis: [98, 102, 112, 255],
        text: [246, 246, 242, 255],
        muted: [185, 188, 196, 255],
        up: [78, 224, 148, 255],
        down: [246, 88, 74, 255],
        accent: [138, 170, 255, 255],
        warn: [255, 200, 80, 255],
        // `s1` is the accent (§9).
        series: [
            [138, 170, 255, 255],
            [255, 176, 64, 255],
            [196, 140, 255, 255],
            [92, 214, 230, 255],
            [255, 128, 196, 255],
            [214, 214, 120, 255],
            [176, 150, 130, 255],
            [200, 200, 200, 255],
        ],
        surface: [18, 19, 23, 255],
        line: [44, 46, 54, 255],
        neutral: [110, 112, 118, 255],
        valid: [246, 246, 242, 255],
        invalid: [110, 112, 118, 255],
        // (M16 rebase) `tg5` draws a closed glyph at this floor: the failed trace in
        // `closed` = `invalid` (3.75:1: "did not hold" and "no longer valid" stay one grey),
        // the realized plate in opaque ink. The ladder of DESIGN.md decision 2, at any
        // freshness.
        closed: [110, 112, 118, 255],
        closed_alpha: 255,
        // An open glyph keeps ≥ 3:1 at freshness 0 from 214 up (`neutral`); 255 keeps the
        // ladder itself: below it the faded `down` slides onto the grey (harness `compute`,
        // grey ΔE00 4.2 at 216; the presented capsules sit at freshness ≈ 0). Freshness is
        // read from the sentence ("bugün, hâlâ geçerli"), not from the glyph's opacity.
        fresh_alpha_min: 255,
        // The confidence plate never fades below a fresh glyph's (0.24).
        plate_alpha_min: 61,
        // (R5/fix47, A46) A grey glyph's plate (3.09:1 for its strokes; 61 gave 2.93).
        grey_plate_alpha_max: 49,
        focus: [138, 170, 255, 255],
        chrome: Chrome {
            bg: [6, 6, 8, 255],
            fg: [246, 246, 242, 255],
            muted: [170, 173, 180, 255],
            line: [40, 42, 48, 255],
            card: [18, 19, 23, 255],
            accent: [138, 170, 255, 255],
            on_accent: [6, 6, 8, 255],
            page: [0, 0, 0, 255],
            note_bg: [58, 46, 14, 255],
            note_fg: [255, 226, 160, 255],
            alert_bg: [74, 24, 32, 255],
            alert_fg: [255, 214, 220, 255],
            req: [255, 190, 90, 255],
            // (R5/fix47, A46) Control boundary (WCAG 1.4.11): 4.09:1 on the chrome, 3.75:1
            // on a card (`line` stays a quiet divider).
            control: [110, 112, 118, 255],
        },
    },
    light: Theme {
        name: "light",
        bg: [255, 255, 255, 255],
        grid: [232, 232, 228, 255],
        // (R5/fix47) 3.19:1 (was 2.88); the grid decorative (1.23:1).
        axis: [142, 144, 150, 255],
        text: [17, 18, 22, 255],
        muted: [39, 42, 50, 255],
        up: [16, 118, 66, 255],
        down: [132, 14, 22, 255],
        accent: [40, 80, 220, 255],
        warn: [150, 90, 0, 255],
        series: [
            [40, 80, 220, 255],
            [196, 104, 0, 255],
            [126, 52, 196, 255],
            [0, 128, 150, 255],
            [196, 40, 120, 255],
            [120, 120, 20, 255],
            [110, 82, 64, 255],
            [96, 96, 96, 255],
        ],
        surface: [255, 255, 255, 255],
        line: [222, 222, 216, 255],
        neutral: [136, 138, 142, 255],
        valid: [17, 18, 22, 255],
        invalid: [136, 138, 142, 255],
        // (M16 rebase) As in the dark scheme: `closed` = `invalid` (3.46:1), opaque.
        closed: [136, 138, 142, 255],
        closed_alpha: 255,
        // ≥ 3:1 at freshness 0 from 232 up (`neutral`); 255 as in the dark scheme.
        fresh_alpha_min: 255,
        plate_alpha_min: 61,
        // (R5/fix47) 3.08:1 for the grey strokes on their plate (61 gave 2.70).
        grey_plate_alpha_max: 29,
        focus: [40, 80, 220, 255],
        chrome: Chrome {
            bg: [238, 238, 234, 255],
            fg: [17, 18, 22, 255],
            muted: [82, 86, 94, 255],
            line: [214, 214, 208, 255],
            card: [255, 255, 255, 255],
            accent: [40, 80, 220, 255],
            on_accent: [255, 255, 255, 255],
            page: [200, 200, 194, 255],
            note_bg: [255, 241, 199, 255],
            note_fg: [107, 75, 0, 255],
            alert_bg: [74, 24, 32, 255],
            alert_fg: [255, 214, 220, 255],
            req: [150, 80, 0, 255],
            // (R5/fix47) Control boundary: 3.40:1 on the chrome, 3.96:1 on a card.
            control: [126, 128, 132, 255],
        },
    },
    space: Space {
        scale: [4, 8, 12, 16, 20, 28],
        plot: [8, 72, 28, 24],
        panel_gap: 10,
        glyph_gap: 14.0,
    },
    surface: Surface {
        radius: 14.0,
        frame_radius: 32.0,
        pill_radius: 10.0,
        line: 1.0,
        shadow: false,
        strip_alpha: 48,
        veil_alpha: 170,
    },
    glyph: GlyphLook {
        face_px: 120.0,
        card_px: 64.0,
        chart_px: 32.0,
        default_px: 28.0,
        place: GlyphPlace::Leading,
    },
    motion: Motion {
        duration_scale: 0.85,
        ease: Ease::Bezier([0.2, 0.0, 0.0, 1.0]),
        chrome_ms: 140.0,
        ..R4.motion
    },
    chrome: ChromeLook {
        text_px: 16.0,
        line: 1.4,
        small_px: 14.0,
        table_px: 14.0,
        tiny_px: 13.0,
        trust_px: 13.0,
        head_weight: 600,
        head_tracking: 0.06,
        head_gap: 14.0,
        tab_h: 48.0,
        tab_pad: 8.0,
        tab_rule: 3.0,
        control_h: 44.0,
        control_pad: [8.0, 8.0],
        date_w: 100.0,
        bar_h: 60.0,
        cell_pad: [6.0, 6.0],
        badge_pad: [3.0, 8.0],
        face_pad: [14.0, 14.0],
        face_min_h: 72.0,
    },
    evidence: EvidenceLook {
        color: "accent",
        note_color: "warn",
        rule_w: 2.5,
        band_w: 1.5,
        band_alpha: 0.16,
        point_px: 11.0,
    },
    offsets: Offsets {
        title_baseline: 20,
        legend_gap: 14,
        legend_baseline: 14,
        ledger_gap: 4.0,
        ..R4.offsets
    },
    // (R5/S) The story chart and open capsule counts as `r4`; (R5/fix47, A46) the selected
    // story's row band fainter (20, `r4` 40), so the grey lanes of other closed stories in
    // that row keep 3:1 against it (3.33 / 3.08; 40 gave 2.85 / 2.73).
    story: StoryLook {
        focus_row_alpha: 20,
        ..R4.story
    },
    open: R4.open,
};
