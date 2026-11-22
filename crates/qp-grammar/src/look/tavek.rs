//! The looks [`TAVEK`] (desktop) and [`TAVEK_TEL`] (phone): RD2 direction B.

use super::*;

const fn trole(px: f64, line: f64, font: &'static str) -> TypeRole {
    TypeRole { px, line, font }
}

/// (RD2/Y-b) **`tavek`** — "the chart carries the meaning" (`docs/design/mockups/tavek/
/// DIRECTION.md`). The shared alphabet unchanged (right price axis, two-level time, OHLC
/// legend, last value and dotted value line, crosshair pills, volume). Qapsule's own layer
/// sits on the plot in the chart's own grammar: every level ends in an axis tag; origin is
/// stroke plus colour (human solid ink, agent dashed violet, engine dotted graphite), so
/// only the agent adds a hue; a watch's state is a word on its axis tag; a capsule's
/// evidence window is a faint band (`evidence`, `muted`) with a bracket on the time axis.
/// Values chosen here (clean room, K83): cool neutral chrome, calm up/down on a lightness
/// ladder (dark: grey < down < up < ink; light reversed), IBM Plex Mono for figures.
pub const TAVEK: Look = Look {
    name: "tavek",
    note: "RD2 B: the chart carries the meaning; origin as stroke + axis tag, watch state at \
           the axis, evidence window as a band",
    scheme: "dark",
    types: TypeScale {
        headline: trole(16.0, 1.3, "atkinson-next-semibold"),
        title: trole(14.0, 1.3, "atkinson-next-semibold"),
        body: trole(13.0, 1.4, "atkinson-next"),
        label: trole(11.0, 1.3, DEFAULT_FONT),
        figure: trole(11.0, 1.3, DEFAULT_FONT),
        note: trole(11.5, 1.3, "atkinson-next"),
    },
    dark: Theme {
        name: "dark",
        bg: [19, 20, 23, 255],
        grid: [31, 33, 37, 255],
        axis: [108, 111, 118, 255],
        text: [230, 231, 234, 255],
        muted: [150, 153, 160, 255],
        up: [104, 196, 150, 255],
        down: [212, 92, 95, 255],
        accent: [104, 158, 232, 255],
        warn: [226, 178, 84, 255],
        // `s1` is the accent; the agent's violet is kept out of the series.
        series: [
            [104, 158, 232, 255],
            [222, 160, 72, 255],
            [78, 186, 200, 255],
            [214, 120, 170, 255],
            [180, 186, 104, 255],
            [196, 150, 120, 255],
            [140, 170, 140, 255],
            [180, 180, 184, 255],
        ],
        surface: [24, 25, 29, 255],
        line: [44, 46, 51, 255],
        neutral: [97, 99, 105, 255],
        valid: [230, 231, 234, 255],
        invalid: [97, 99, 105, 255],
        closed: [97, 99, 105, 255],
        closed_alpha: 255,
        fresh_alpha_min: 255,
        plate_alpha_min: 61,
        grey_plate_alpha_max: 40,
        focus: [104, 158, 232, 255],
        chrome: Chrome {
            bg: [16, 17, 20, 255],
            fg: [230, 231, 234, 255],
            muted: [150, 153, 160, 255],
            line: [44, 46, 51, 255],
            card: [24, 25, 29, 255],
            accent: [104, 158, 232, 255],
            on_accent: [16, 17, 20, 255],
            page: [10, 10, 12, 255],
            note_bg: [36, 38, 43, 255],
            note_fg: [230, 231, 234, 255],
            alert_bg: [62, 30, 34, 255],
            alert_fg: [250, 222, 224, 255],
            req: [226, 178, 84, 255],
            control: [104, 107, 114, 255],
        },
    },
    light: Theme {
        name: "light",
        bg: [255, 255, 255, 255],
        grid: [236, 237, 239, 255],
        axis: [128, 131, 138, 255],
        text: [24, 25, 28, 255],
        muted: [92, 95, 102, 255],
        up: [20, 128, 84, 255],
        down: [150, 26, 38, 255],
        accent: [40, 96, 200, 255],
        warn: [150, 92, 0, 255],
        series: [
            [40, 96, 200, 255],
            [184, 108, 0, 255],
            [0, 124, 138, 255],
            [176, 50, 120, 255],
            [110, 116, 20, 255],
            [130, 86, 60, 255],
            [70, 110, 70, 255],
            [100, 100, 104, 255],
        ],
        surface: [255, 255, 255, 255],
        line: [214, 215, 218, 255],
        neutral: [140, 142, 148, 255],
        valid: [24, 25, 28, 255],
        invalid: [140, 142, 148, 255],
        closed: [140, 142, 148, 255],
        closed_alpha: 255,
        fresh_alpha_min: 255,
        plate_alpha_min: 61,
        grey_plate_alpha_max: 29,
        focus: [40, 96, 200, 255],
        chrome: Chrome {
            bg: [246, 246, 247, 255],
            fg: [24, 25, 28, 255],
            muted: [92, 95, 102, 255],
            line: [214, 215, 218, 255],
            card: [255, 255, 255, 255],
            accent: [40, 96, 200, 255],
            on_accent: [255, 255, 255, 255],
            page: [232, 233, 235, 255],
            note_bg: [238, 239, 241, 255],
            note_fg: [24, 25, 28, 255],
            alert_bg: [250, 232, 233, 255],
            alert_fg: [110, 20, 30, 255],
            req: [150, 92, 0, 255],
            control: [128, 131, 138, 255],
        },
    },
    space: Space {
        scale: [4, 8, 12, 16, 24, 32],
        // Bottom: the time axis plus a row for a face's trust mark under it (RD2 polish).
        plot: [8, 64, 28, 40],
        panel_gap: 8,
        glyph_gap: 8.0,
    },
    surface: Surface {
        radius: 6.0,
        frame_radius: 28.0,
        pill_radius: 4.0,
        line: 1.0,
        shadow: true,
        shadow_css: "0 1px 2px rgba(0, 0, 0, 0.18), 0 4px 12px rgba(0, 0, 0, 0.12)",
        card_radius: 8.0,
        strip_alpha: 40,
        veil_alpha: 150,
    },
    glyph: GlyphLook {
        face_px: 24.0,
        card_px: 32.0,
        chart_px: 20.0,
        default_px: 20.0,
        place: GlyphPlace::Leading,
    },
    motion: Motion {
        duration_scale: 0.9,
        ease: Ease::Bezier([0.2, 0.0, 0.0, 1.0]),
        chrome_ms: 120.0,
        change_ms: 240.0,
    },
    chrome: ChromeLook {
        text_px: 13.0,
        line: 1.4,
        small_px: 12.0,
        table_px: 12.0,
        tiny_px: 11.0,
        trust_px: 11.0,
        head_weight: 600,
        head_tracking: 0.0,
        head_gap: 12.0,
        tab_h: 32.0,
        tab_pad: 8.0,
        tab_rule: 2.0,
        control_h: 28.0,
        control_pad: [4.0, 10.0],
        date_w: 112.0,
        bar_h: 40.0,
        cell_pad: [4.0, 8.0],
        badge_pad: [1.0, 6.0],
        face_pad: [12.0, 12.0],
        face_min_h: 56.0,
    },
    // The capsule's evidence window: graphite, quiet (the engine's own colour family).
    evidence: EvidenceLook {
        color: "muted",
        note_color: "warn",
        rule_w: 1.0,
        band_w: 1.0,
        band_alpha: 0.08,
        point_px: 6.0,
    },
    offsets: Offsets {
        title_baseline: 18,
        legend_gap: 12,
        legend_baseline: 13,
        ..R4.offsets
    },
    story: R4.story,
    open: R4.open,
    profile: Profile::Dense,
    // Human: ink, solid. Agent: the one origin hue, dashed. Engine: graphite, dotted
    // (RD2/C2b; 4.5:1 for a tag's text). Apart by lightness too, so grey and colour-blind
    // vision keep them.
    origin: Some(OriginLook {
        dark: OriginRoles {
            human: [230, 231, 234, 255],
            agent: [168, 150, 240, 255],
            engine: [124, 127, 134, 255],
        },
        light: OriginRoles {
            human: [24, 25, 28, 255],
            agent: [96, 64, 196, 255],
            engine: [115, 118, 125, 255],
        },
        agent_dash: Some([4.0, 3.0]),
    }),
    chart: ChartLook {
        price_label_px: 48.0,
        two_level_time: true,
        last_value_tag: true,
        value_line: ValueLine::Dotted,
        crosshair_pills: true,
        ohlc_legend: true,
        volume_share: 0.18,
        mark_max_px: 20.0,
        body_ratio: 0.72,
        axis_gutter_px: 64.0,
        origin_levels: true,
    },
};

/// (RD2/Y-b) **`tavek-tel`** — [`TAVEK`] on a phone: touch type and targets (44 px),
/// a narrower gutter, marks up to 24 px; colours, origin and chart grammar unchanged.
pub const TAVEK_TEL: Look = Look {
    name: "tavek-tel",
    note: "RD2 B on a phone: touch type and targets, same colours and chart grammar",
    types: TypeScale {
        headline: trole(19.0, 1.3, "atkinson-next-semibold"),
        title: trole(17.0, 1.3, "atkinson-next-semibold"),
        body: trole(16.0, 1.4, "atkinson-next"),
        label: trole(13.0, 1.3, DEFAULT_FONT),
        figure: trole(13.0, 1.3, DEFAULT_FONT),
        note: trole(13.5, 1.3, "atkinson-next"),
    },
    space: Space {
        plot: [8, 56, 28, 44],
        panel_gap: 12,
        glyph_gap: 10.0,
        ..TAVEK.space
    },
    glyph: GlyphLook {
        chart_px: 24.0,
        default_px: 24.0,
        ..TAVEK.glyph
    },
    chrome: ChromeLook {
        text_px: 16.0,
        small_px: 14.0,
        table_px: 14.0,
        tiny_px: 13.0,
        trust_px: 13.0,
        head_gap: 16.0,
        tab_h: 48.0,
        tab_pad: 10.0,
        control_h: 44.0,
        // Narrow sides: the replay bar fits 390 px with its step counter (RD2 polish).
        control_pad: [10.0, 8.0],
        bar_h: 56.0,
        cell_pad: [8.0, 12.0],
        badge_pad: [3.0, 8.0],
        face_pad: [14.0, 16.0],
        face_min_h: 72.0,
        ..TAVEK.chrome
    },
    offsets: Offsets {
        title_baseline: 20,
        legend_baseline: 15,
        ..TAVEK.offsets
    },
    profile: Profile::Touch,
    chart: ChartLook {
        mark_max_px: 24.0,
        axis_gutter_px: 56.0,
        ..TAVEK.chart
    },
    ..TAVEK
};
