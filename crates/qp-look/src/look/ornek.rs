//! The trial look [`ORNEK`] (the M16 looks start here).

use super::*;

// --- M16 looks: `const` records go below this line ---------------------------------------

/// (R5/T3) A trial look proving the mechanism: other colours and larger type, same font.
/// Not a design proposal. (R5/T3b) It also proves the hard thresholds can be met from a
/// record alone: no text under 13 px (scenes and chrome), tabs and controls 44 px high,
/// a closed glyph at least 3:1 against the background in both schemes (`closed_alpha`),
/// (R5/T3c) an open glyph too at freshness 0 (`fresh_alpha_min`),
/// the evidence's "requires" text in the light scheme at least 4.5:1 (`warn`, `req`).
pub const ORNEK: Look = Look {
    name: "ornek",
    note: "T3 trial: warm colours, larger type (mechanism check, not a proposal)",
    scheme: "dark",
    types: TypeScale {
        headline: role(22.0),
        title: role(16.0),
        body: role(15.0),
        label: role(14.0),
        figure: role(13.0),
        note: role(13.0),
    },
    dark: Theme {
        name: "dark",
        bg: [24, 20, 18, 255],
        grid: [48, 40, 36, 255],
        axis: [104, 92, 84, 255],
        text: [240, 230, 220, 255],
        muted: [176, 160, 148, 255],
        up: [96, 196, 128, 255],
        down: [240, 112, 96, 255],
        accent: [120, 180, 255, 255],
        // `s1` is the accent (§9).
        series: [
            [120, 180, 255, 255],
            Theme::DARK.series[1],
            Theme::DARK.series[2],
            Theme::DARK.series[3],
            Theme::DARK.series[4],
            Theme::DARK.series[5],
            Theme::DARK.series[6],
            Theme::DARK.series[7],
        ],
        surface: [34, 29, 26, 255],
        line: [60, 52, 46, 255],
        neutral: [120, 180, 255, 255],
        closed: [150, 138, 128, 255],
        closed_alpha: 184,
        // (R5/T3c) An open glyph at freshness 0 keeps ≥ 3:1 (`down` needs ≥ 153).
        fresh_alpha_min: 168,
        // (R5/T3c) The confidence plate never fades below a fresh glyph's (0.24).
        plate_alpha_min: 61,
        grey_plate_alpha_max: 0,
        invalid: [176, 160, 148, 255],
        focus: [120, 180, 255, 255],
        chrome: Chrome {
            bg: [24, 20, 18, 255],
            fg: [240, 230, 220, 255],
            muted: [176, 160, 148, 255],
            line: [60, 52, 46, 255],
            control: [60, 52, 46, 255],
            card: [34, 29, 26, 255],
            accent: [120, 180, 255, 255],
            on_accent: [24, 20, 18, 255],
            page: [10, 8, 7, 255],
            ..Theme::DARK.chrome
        },
        ..Theme::DARK
    },
    light: Theme {
        name: "light",
        bg: [252, 248, 242, 255],
        text: [36, 28, 24, 255],
        muted: [100, 88, 80, 255],
        surface: [255, 255, 255, 255],
        line: [222, 212, 202, 255],
        warn: [138, 84, 0, 255],
        closed: [110, 98, 90, 255],
        closed_alpha: 204,
        // (R5/T3c) An open glyph at freshness 0 keeps ≥ 3:1 (`up` needs ≥ 231, `neutral`
        // ≥ 229); the r4 light accent (2.9:1 even opaque) is too light for a neutral glyph.
        fresh_alpha_min: 236,
        plate_alpha_min: 61,
        grey_plate_alpha_max: 0,
        neutral: [0xb4, 0x53, 0x09, 230],
        invalid: [100, 88, 80, 255],
        chrome: Chrome {
            bg: [252, 248, 242, 255],
            fg: [36, 28, 24, 255],
            muted: [100, 88, 80, 255],
            line: [222, 212, 202, 255],
            req: [138, 74, 0, 255],
            control: [222, 212, 202, 255],
            ..Theme::LIGHT.chrome
        },
        ..Theme::LIGHT
    },
    glyph: GlyphLook {
        face_px: 40.0,
        ..R4.glyph
    },
    surface: Surface {
        radius: 12.0,
        card_radius: 12.0,
        ..R4.surface
    },
    motion: Motion {
        duration_scale: 1.25,
        ease: Ease::Bezier([0.2, 0.0, 0.0, 1.0]),
        chrome_ms: 160.0,
        ..R4.motion
    },
    chrome: ChromeLook {
        text_px: 15.0,
        small_px: 13.0,
        table_px: 14.0,
        tiny_px: 13.0,
        trust_px: 13.0,
        tab_h: 44.0,
        control_h: 44.0,
        control_pad: [6.0, 10.0],
        date_w: 136.0,
        bar_h: 60.0,
        ..R4.chrome
    },
    evidence: EvidenceLook {
        point_px: 11.0,
        ..R4.evidence
    },
    ..R4
};
