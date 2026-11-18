//! (R5/T3, Ö44) Looks: the design records of the grammar (`docs/GRAMMAR.md` §9a).
//!
//! A look is one complete visual direction as **data**: the font assets and the type scale
//! (roles with size, line height and font), the colour roles of a dark and a light scheme
//! (a [`Theme`] each), the spacing scale, the surface (corner radius, line, shadow), the
//! glyph's place and sizes, the motion (duration scale and easing curve) and the default
//! scheme. The compiler, the face designs, the glyph alphabets, the transitions and the
//! shell's own chrome (as CSS custom properties, [`css_vars`]) read their values from the
//! look a connection chose (`style` gesture `look`, `ViewState.look`); nothing else holds
//! a design value. Pure data, deterministic.
//!
//! The reference **`r4`** carries the values the product had before looks existed: every
//! existing golden image and transcript is byte-identical under it (it is the default).
//!
//! **Adding a look (M16):** in this file only — write a `const` [`Look`] below the
//! `M16 looks` marker (start from [`R4`] with `..R4` and change what differs) and add one
//! line to [`LOOKS`]. A new font file: put the file and its licence next to the others
//! (`crates/qp-render/assets/fonts/`), add a [`FontAsset`] line to [`FONTS`] here and one
//! `include_bytes!` line to `qp_render::fonts::FILES`; a registry test checks both lists
//! agree. Nothing else changes; the settings page, `?look=` and `qp look` list the names
//! from here.

use qp_schema::scene::Rgba;
use serde_json::{json, Value};

use crate::error::{ErrorKind, GrammarError};
use crate::theme::{Chrome, Theme};

/// One embedded font file (a single family at a single weight: the reference renderer,
/// fontdue, draws a variable font at its default instance only).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontAsset {
    /// Asset id (`[a-z][a-z0-9-]*`): what a [`TypeRole`] and a scene's text name.
    pub id: &'static str,
    /// The family name as published.
    pub family: &'static str,
    /// Weight (CSS 100..=900) of this file.
    pub weight: u16,
    /// Italic.
    pub italic: bool,
    /// File name in `crates/qp-render/assets/fonts/`.
    pub file: &'static str,
    /// Licence file name in the same folder (e.g. `OFL.txt`).
    pub license: &'static str,
    /// Licence identifier (SPDX).
    pub spdx: &'static str,
    /// The CSS family the canvas paint path registers the file under (`FontFace`).
    pub css_family: &'static str,
    /// Mean advance per character, in em (a monospaced file: every glyph's advance, and
    /// the compiler measures `characters · size · advance`, R3's estimate).
    pub advance: f64,
    /// Ascent in em: where the first baseline of a wrapped text block sits below its top.
    pub ascent: f64,
    /// (R5/T3b) The file's glyph advance table (generated from the file with fontdue's
    /// metrics, [`crate::advances`]): the compiler measures text with it, so wrapping and
    /// fitting in a proportional family match what the renderers draw.
    pub advances: &'static Advances,
}

/// (R5/T3b) The horizontal advance of every character a font file maps, in font units,
/// as fontdue reads it (`hmtx`; no kerning: fontdue applies none and the canvas paint path
/// draws with `fontKerning = 'none'`). Generated, never written by hand: `crate::advances`
/// is written by `QP_BLESS=1 cargo test -p qp-render --test fonts`, and that test fails
/// when a file and its table disagree.
#[derive(Debug, PartialEq, Eq)]
pub struct Advances {
    /// Units per em.
    pub upem: u16,
    /// Advance of the missing glyph (a character the file does not map is drawn as it).
    pub notdef: u16,
    /// Monospaced: every spacing glyph has the missing glyph's advance (zero-width marks
    /// aside). The compiler then keeps R3's per-character estimate ([`FontAsset::width`]).
    pub mono: bool,
    /// `(first, last, advance)`: the code points `first..=last` all map to glyphs of that
    /// advance; sorted, disjoint.
    pub runs: &'static [(u32, u32, u16)],
}

impl Advances {
    /// The advance of `c` in font units.
    #[must_use]
    pub fn units(&self, c: char) -> u16 {
        let c = u32::from(c);
        let at = self.runs.binary_search_by(|&(a, b, _)| {
            if b < c {
                std::cmp::Ordering::Less
            } else if a > c {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        at.map_or(self.notdef, |i| self.runs[i].2)
    }
}

impl FontAsset {
    /// The advance width of `text` at `size` px, rounded up, at least 1: the compiler's
    /// text measure (line wrapping, fitting, legends, hit boxes). A monospaced file:
    /// `ceil(characters · size · advance)`, R3's estimate, unchanged; any other: the sum
    /// of the characters' advances from [`Advances`].
    #[must_use]
    pub fn width(&self, text: &str, size: f64) -> i32 {
        let w = if self.advances.mono {
            #[allow(clippy::cast_precision_loss)]
            let n = text.chars().count() as f64;
            n * size * self.advance
        } else {
            let u: u64 = text
                .chars()
                .map(|c| u64::from(self.advances.units(c)))
                .sum();
            #[allow(clippy::cast_precision_loss)]
            let u = u as f64;
            u * size / f64::from(self.advances.upem)
        };
        #[allow(clippy::cast_possible_truncation)]
        let w = w.ceil() as i32;
        w.max(1)
    }
}

/// The default font asset: a text prim without `font` is drawn with it.
pub const DEFAULT_FONT: &str = "plex-mono";

/// Embedded font assets (the renderers embed the same files, `qp_render::fonts`).
pub static FONTS: &[FontAsset] = &[FontAsset {
    id: "plex-mono",
    family: "IBM Plex Mono",
    weight: 400,
    italic: false,
    file: "IBMPlexMono-Regular.ttf",
    license: "OFL.txt",
    spdx: "OFL-1.1",
    css_family: "QpMono",
    advance: 0.6,
    ascent: 0.8,
    advances: &crate::advances::PLEX_MONO,
}];

/// The font asset `id`, if embedded.
#[must_use]
pub fn font(id: &str) -> Option<&'static FontAsset> {
    FONTS.iter().find(|f| f.id == id)
}

/// One role of the type scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeRole {
    /// Size in CSS px.
    pub px: f64,
    /// Line height as a multiple of the size (wrapped lines: `round(px · line)`).
    pub line: f64,
    /// Font asset id ([`FONTS`]).
    pub font: &'static str,
}

/// Type role names, in [`TypeScale`] order.
pub const ROLES: [&str; 6] = ["headline", "title", "body", "label", "figure", "note"];

/// The type scale: the roles every text of a scene is drawn in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeScale {
    /// The face's headline sentence (başlık cümlesi).
    pub headline: TypeRole,
    /// The top row of a scene: a chart's title and close, a face's one additional item
    /// (ek bilgi).
    pub title: TypeRole,
    /// A sentence in a card or a chart note.
    pub body: TypeRole,
    /// Labels: axis dates, legends, a card's ledger line (etiket).
    pub label: TypeRole,
    /// Figures: axis values, last-value tags (rakam).
    pub figure: TypeRole,
    /// The smallest text (evidence labels).
    pub note: TypeRole,
}

impl TypeScale {
    /// The role named `name`.
    #[must_use]
    pub fn role(&self, name: &str) -> Option<&TypeRole> {
        Some(match name {
            "headline" => &self.headline,
            "title" => &self.title,
            "body" => &self.body,
            "label" => &self.label,
            "figure" => &self.figure,
            "note" => &self.note,
            _ => return None,
        })
    }

    /// The role a view's text mark of size `px` is drawn in: the first of `headline`,
    /// `body`, `label`, `note` with that size (a view names sizes, not roles; the face
    /// designs take the sizes from these roles, and the registry test checks the four
    /// sizes differ in every look). `None`: a size no role has.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn mark_role(&self, px: f64) -> Option<&'static str> {
        [
            ("headline", &self.headline),
            ("body", &self.body),
            ("label", &self.label),
            ("note", &self.note),
        ]
        .into_iter()
        // Scenes carry sizes as f32: compared at that precision.
        .find(|(_, r)| (r.px as f32).to_bits() == (px as f32).to_bits())
        .map(|(n, _)| n)
    }
}

/// Spacing (CSS px).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Space {
    /// The spacing scale, small to large (the shell's chrome: `--qp-space-<i>`).
    pub scale: [i32; 6],
    /// Plot margins of a scene: left, right (from the width), top, bottom (from the
    /// height). The top margin holds the title row.
    pub plot: [i32; 4],
    /// Gap between stacked panels.
    pub panel_gap: i32,
    /// Gap between a glyph and the text beside it (face, card).
    pub glyph_gap: f64,
}

/// Corners and surfaces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Surface {
    /// Corner radius of the shell's controls (CSS px). Scenes are flat rectangles.
    pub radius: f64,
    /// Corner radius of the phone frame (CSS px).
    pub frame_radius: f64,
    /// Corner radius of a small badge (CSS px).
    pub pill_radius: f64,
    /// Width of rules, grid and axis lines (px).
    pub line: f64,
    /// Elevation: `false` = flat (no shadow anywhere).
    pub shadow: bool,
    /// Alpha of an emphasis strip (accent).
    pub strip_alpha: u8,
    /// Alpha of a focus/dim veil (background).
    pub veil_alpha: u8,
}

/// Where the glyph sits next to a face's sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphPlace {
    /// Before the sentence (left), top-aligned.
    Leading,
    /// Above the sentence; the sentence takes the full width.
    Above,
}

/// The glyph's sizes and place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphLook {
    /// On a face (the presented face, `vadrel`).
    pub face_px: f64,
    /// In the capsule card (the protocol's default face and the open capsule).
    pub card_px: f64,
    /// Over the chart at `chart` depth.
    pub chart_px: f64,
    /// A glyph layer without a size.
    pub default_px: f64,
    /// Next to the face's sentence.
    pub place: GlyphPlace,
}

/// An easing curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ease {
    /// `p` unchanged.
    Linear,
    /// CSS `cubic-bezier(x1, y1, x2, y2)`, solved with a fixed number of bisection steps
    /// (deterministic).
    Bezier([f64; 4]),
}

impl Ease {
    /// The eased progress of `p` (clamped to 0..=1).
    #[must_use]
    pub fn apply(self, p: f64) -> f64 {
        let p = p.clamp(0.0, 1.0);
        match self {
            Ease::Linear => p,
            Ease::Bezier([x1, y1, x2, y2]) => {
                let b = |a: f64, c: f64, t: f64| {
                    let u = 1.0 - t;
                    3.0 * u * u * t * a + 3.0 * u * t * t * c + t * t * t
                };
                let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
                for _ in 0..40 {
                    let mid = (lo + hi) / 2.0;
                    if b(x1, x2, mid) < p {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                b(y1, y2, (lo + hi) / 2.0)
            }
        }
    }

    /// The CSS form (`linear`, `cubic-bezier(...)`).
    #[must_use]
    pub fn css(self) -> String {
        match self {
            Ease::Linear => "linear".to_owned(),
            Ease::Bezier([a, b, c, d]) => format!("cubic-bezier({a}, {b}, {c}, {d})"),
        }
    }
}

/// Motion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    /// Multiplies every transition design's duration (`transitions::TRANSITIONS`).
    pub duration_scale: f64,
    /// The curve a transition's progress follows.
    pub ease: Ease,
    /// Duration of the shell chrome's own changes (tabs, banners), ms; `0`: none.
    pub chrome_ms: f64,
}

/// (R5/T3b) The shell chrome's sizes (CSS px unless noted): type sizes, paddings, control
/// heights. The page reads them as CSS custom properties ([`css_vars`]); its stylesheet
/// holds no length of its own (`shells/web/arch-check.mjs`). Colours: [`Theme::chrome`];
/// corner radii: [`Surface`]; gaps and margins: [`Space::scale`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChromeLook {
    /// Body text of the chrome (`--qp-fs`).
    pub text_px: f64,
    /// Line height of the chrome's text, a multiple of the size (`--qp-lh`).
    pub line: f64,
    /// Small text: the notice banner, hints, the toast, code, section heads (`--qp-fs-small`).
    pub small_px: f64,
    /// Table cells (`--qp-fs-table`).
    pub table_px: f64,
    /// The smallest chrome text: a face's badge, the open layer's trust mark, the replay
    /// step counter, the benchmark table (`--qp-fs-tiny`).
    pub tiny_px: f64,
    /// A face's trust mark ("Sunucu doğruladı" / "Checked here", `--qp-fs-trust`).
    pub trust_px: f64,
    /// Weight of a section head (CSS 100..=900, `--qp-head-weight`).
    pub head_weight: u16,
    /// Letter spacing of a section head, em (`--qp-head-tracking`).
    pub head_tracking: f64,
    /// Space above a section head (`--qp-head-gap`).
    pub head_gap: f64,
    /// Least height of a tab, border box (`--qp-tab-h`; `0`: `auto`, its text and padding).
    pub tab_h: f64,
    /// Padding of a tab (`--qp-tab-pad`).
    pub tab_pad: f64,
    /// Thickness of the active tab's underline (`--qp-tab-rule`).
    pub tab_rule: f64,
    /// Least height of a button, select or field, border box (`--qp-control-h`; `0`:
    /// `auto`, its text and padding).
    pub control_h: f64,
    /// Padding of a button, select or field: vertical, horizontal (`--qp-control-pad`).
    pub control_pad: [f64; 2],
    /// Width of a date field (`--qp-date-w`).
    pub date_w: f64,
    /// Height of the replay control bar at the bottom (`--qp-bar-h`).
    pub bar_h: f64,
    /// Padding of a table cell: vertical, horizontal (`--qp-cell-pad`).
    pub cell_pad: [f64; 2],
    /// Padding of a face's badge: vertical, horizontal (`--qp-badge-pad`).
    pub badge_pad: [f64; 2],
    /// Padding of a face's text block: vertical, horizontal (`--qp-face-pad`).
    pub face_pad: [f64; 2],
    /// Least height of a face's text block (`--qp-face-min-h`).
    pub face_min_h: f64,
}

/// (R5/T3b) The evidence geometry over a chart (`depth::evidence_layers`) and the note
/// that says what the evidence needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvidenceLook {
    /// Colour token of the geometry (a level, a band, swings and their points).
    pub color: &'static str,
    /// Colour token of the note "requires …" (`r4`: `warn`).
    pub note_color: &'static str,
    /// Width of a level rule and a swing segment (px).
    pub rule_w: f64,
    /// Width of a band's edge (px).
    pub band_w: f64,
    /// Opacity of a band's fill.
    pub band_alpha: f64,
    /// Diameter of a swing point (px).
    pub point_px: f64,
}

/// (R5/T3b) Small layout offsets of scenes (CSS px): the title row, legends, a card's
/// ledger line, the evidence note, the glyph strip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Offsets {
    /// The title's left inset from the plot's left edge.
    pub title_x: i32,
    /// Baseline of the title row (title, close, the first panel's legends).
    pub title_baseline: i32,
    /// Where the close ("C …") starts, from the plot's left edge.
    pub close_x: i32,
    /// Margin at the right edge of the title row and between the title and the close.
    pub row_edge: i32,
    /// Gap between legends (and between the close and the first legend).
    pub legend_gap: i32,
    /// Inset of a lower panel's legends from its left edge.
    pub legend_x: i32,
    /// Baseline of a lower panel's legends below its top.
    pub legend_baseline: i32,
    /// A card's ledger line: gap between its bottom and the panel's bottom.
    pub ledger_gap: f64,
    /// The evidence note's left inset in the panel.
    pub note_x: f64,
    /// The evidence note's gap below the chart's glyph strip.
    pub note_gap: f64,
    /// Top of a glyph layer without `y` (the glyph strip over a chart).
    pub glyph_top: f64,
}

/// One look.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// Setting name (`[a-z][a-z0-9_-]*`, not an ordinary word).
    pub name: &'static str,
    /// One line on what this direction is.
    pub note: &'static str,
    /// The scheme used when the reader chose none (`dark` or `light`).
    pub scheme: &'static str,
    /// The type scale.
    pub types: TypeScale,
    /// The dark scheme's colour roles.
    pub dark: Theme,
    /// The light scheme's colour roles.
    pub light: Theme,
    /// Spacing.
    pub space: Space,
    /// Surfaces.
    pub surface: Surface,
    /// The glyph.
    pub glyph: GlyphLook,
    /// Motion.
    pub motion: Motion,
    /// (R5/T3b) The shell chrome's sizes.
    pub chrome: ChromeLook,
    /// (R5/T3b) The evidence geometry.
    pub evidence: EvidenceLook,
    /// (R5/T3b) Small scene offsets.
    pub offsets: Offsets,
}

const fn role(px: f64) -> TypeRole {
    TypeRole {
        px,
        line: 1.3,
        font: DEFAULT_FONT,
    }
}

/// The reference look: the product's values before looks existed (R0 colours, R3 card,
/// R4 face `vadrel`). Every golden is drawn with it.
pub const R4: Look = Look {
    name: "r4",
    note: "R4 reference: IBM Plex Mono throughout, R0 colours, flat surfaces",
    scheme: "dark",
    types: TypeScale {
        headline: role(20.0),
        title: role(16.0),
        body: role(12.0),
        label: role(11.0),
        figure: role(11.0),
        note: role(9.0),
    },
    dark: Theme::DARK,
    light: Theme::LIGHT,
    space: Space {
        scale: [4, 6, 8, 12, 16, 24],
        plot: [8, 72, 34, 24],
        panel_gap: 8,
        glyph_gap: 10.0,
    },
    surface: Surface {
        radius: 6.0,
        frame_radius: 18.0,
        pill_radius: 8.0,
        line: 1.0,
        shadow: false,
        strip_alpha: 56,
        veil_alpha: 160,
    },
    glyph: GlyphLook {
        face_px: 32.0,
        card_px: 40.0,
        chart_px: 32.0,
        default_px: 24.0,
        place: GlyphPlace::Leading,
    },
    motion: Motion {
        duration_scale: 1.0,
        ease: Ease::Linear,
        chrome_ms: 0.0,
    },
    chrome: ChromeLook {
        text_px: 14.0,
        line: 1.35,
        small_px: 12.0,
        table_px: 13.0,
        tiny_px: 11.0,
        trust_px: 10.0,
        head_weight: 600,
        head_tracking: 0.04,
        head_gap: 10.0,
        tab_h: 0.0,
        tab_pad: 6.0,
        tab_rule: 2.0,
        control_h: 0.0,
        control_pad: [4.0, 6.0],
        date_w: 120.0,
        bar_h: 48.0,
        cell_pad: [3.0, 4.0],
        badge_pad: [1.0, 6.0],
        face_pad: [14.0, 12.0],
        face_min_h: 60.0,
    },
    evidence: EvidenceLook {
        color: "accent",
        note_color: "warn",
        rule_w: 2.0,
        band_w: 1.0,
        band_alpha: 0.18,
        point_px: 9.0,
    },
    offsets: Offsets {
        title_x: 4,
        title_baseline: 23,
        close_x: 140,
        row_edge: 8,
        legend_gap: 12,
        legend_x: 4,
        legend_baseline: 12,
        ledger_gap: 2.0,
        note_x: 4.0,
        note_gap: 8.0,
        glyph_top: 2.0,
    },
};

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
        invalid: [176, 160, 148, 255],
        focus: [120, 180, 255, 255],
        chrome: Chrome {
            bg: [24, 20, 18, 255],
            fg: [240, 230, 220, 255],
            muted: [176, 160, 148, 255],
            line: [60, 52, 46, 255],
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
        neutral: [0xb4, 0x53, 0x09, 230],
        invalid: [100, 88, 80, 255],
        chrome: Chrome {
            bg: [252, 248, 242, 255],
            fg: [36, 28, 24, 255],
            muted: [100, 88, 80, 255],
            line: [222, 212, 202, 255],
            req: [138, 74, 0, 255],
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
        ..R4.surface
    },
    motion: Motion {
        duration_scale: 1.25,
        ease: Ease::Bezier([0.2, 0.0, 0.0, 1.0]),
        chrome_ms: 160.0,
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

/// Looks a connection may choose; the first is the default.
pub static LOOKS: &[Look] = &[R4, ORNEK];

/// The default look's name.
pub const DEFAULT_LOOK: &str = "r4";

/// Registered names, in registry order.
#[must_use]
pub fn names() -> Vec<&'static str> {
    LOOKS.iter().map(|l| l.name).collect()
}

/// Whether `name` is a registered look.
#[must_use]
pub fn is_look(name: &str) -> bool {
    LOOKS.iter().any(|l| l.name == name)
}

/// The look named `name` (`None`: the default).
///
/// # Errors
/// An unknown name (`invalid`, `/look`).
pub fn by_name(name: Option<&str>) -> Result<&'static Look, GrammarError> {
    let n = name.unwrap_or(DEFAULT_LOOK);
    LOOKS.iter().find(|l| l.name == n).ok_or_else(|| {
        GrammarError::new(
            ErrorKind::Invalid,
            "/look",
            format!("unknown look `{n}` (known: {})", names().join(", ")),
        )
    })
}

/// The look named `name`, the default for `None` or an unknown name.
#[must_use]
pub fn or_default(name: Option<&str>) -> &'static Look {
    by_name(name).unwrap_or(&LOOKS[0])
}

impl Look {
    /// The scheme named `name` (`dark`, `light`; `None`: the look's own default).
    ///
    /// # Errors
    /// Unknown scheme name.
    pub fn theme(&self, name: Option<&str>) -> Result<Theme, GrammarError> {
        match name.unwrap_or(self.scheme) {
            "dark" => Ok(self.dark),
            "light" => Ok(self.light),
            n => Err(GrammarError::new(
                ErrorKind::Invalid,
                "/theme",
                format!(
                    "unknown theme `{n}` (themes: {})",
                    crate::theme::THEMES.join(", ")
                ),
            )),
        }
    }

    /// The font asset of type role `role` (the default asset for an unknown role).
    #[must_use]
    pub fn role_font(&self, role: &str) -> &'static FontAsset {
        self.types
            .role(role)
            .and_then(|r| font(r.font))
            .unwrap_or(&FONTS[0])
    }

    /// The scene `font` field of a text in `role`: `None` for the default asset.
    #[must_use]
    pub fn scene_font(&self, role: &str) -> Option<String> {
        let f = self.role_font(role);
        (f.id != DEFAULT_FONT).then(|| f.id.to_owned())
    }

    /// The font assets this look uses, each once, in role order.
    #[must_use]
    pub fn fonts(&self) -> Vec<&'static FontAsset> {
        let mut out: Vec<&'static FontAsset> = Vec::new();
        for r in ROLES {
            let f = self.role_font(r);
            if !out.iter().any(|o| o.id == f.id) {
                out.push(f);
            }
        }
        out
    }

    /// The whole record as JSON (`qp look show`; the measurement harness computes each
    /// element's colour, size and touch target from a scene and this).
    #[must_use]
    pub fn to_json(&self) -> Value {
        let role = |r: &TypeRole| {
            let f = font(r.font);
            json!({"px": r.px, "line": r.line, "line_px": (r.px * r.line).round(),
                "font": r.font, "family": f.map(|f| f.family), "weight": f.map(|f| f.weight)})
        };
        let roles: serde_json::Map<String, Value> = ROLES
            .iter()
            .filter_map(|n| Some(((*n).to_owned(), role(self.types.role(n)?))))
            .collect();
        let fonts: Vec<Value> = self
            .fonts()
            .iter()
            .map(|f| {
                json!({"id": f.id, "family": f.family, "weight": f.weight, "italic": f.italic,
                    "file": f.file, "license": f.license, "spdx": f.spdx,
                    "css_family": f.css_family, "advance_em": f.advance, "ascent_em": f.ascent})
            })
            .collect();
        json!({
            "name": self.name,
            "note": self.note,
            "scheme": self.scheme,
            "fonts": fonts,
            "type": roles,
            "colors": {"dark": self.dark.to_json(), "light": self.light.to_json()},
            "space": {"scale": self.space.scale, "plot": {"left": self.space.plot[0],
                "right": self.space.plot[1], "top": self.space.plot[2],
                "bottom": self.space.plot[3]}, "panel_gap": self.space.panel_gap,
                "glyph_gap": self.space.glyph_gap},
            "surface": {"radius": self.surface.radius, "frame_radius": self.surface.frame_radius,
                "pill_radius": self.surface.pill_radius,
                "line": self.surface.line, "shadow": self.surface.shadow,
                "strip_alpha": self.surface.strip_alpha, "veil_alpha": self.surface.veil_alpha},
            "glyph": {"face_px": self.glyph.face_px, "card_px": self.glyph.card_px,
                "chart_px": self.glyph.chart_px, "default_px": self.glyph.default_px,
                "place": match self.glyph.place { GlyphPlace::Leading => "leading",
                    GlyphPlace::Above => "above" }},
            "motion": {"duration_scale": self.motion.duration_scale,
                "ease": self.motion.ease.css(), "chrome_ms": self.motion.chrome_ms},
            "chrome": self.chrome_json(),
            "evidence": {"color": self.evidence.color, "note_color": self.evidence.note_color,
                "rule_w": self.evidence.rule_w, "band_w": self.evidence.band_w,
                "band_alpha": self.evidence.band_alpha, "point_px": self.evidence.point_px},
            "offsets": {"title_x": self.offsets.title_x,
                "title_baseline": self.offsets.title_baseline, "close_x": self.offsets.close_x,
                "row_edge": self.offsets.row_edge, "legend_gap": self.offsets.legend_gap,
                "legend_x": self.offsets.legend_x,
                "legend_baseline": self.offsets.legend_baseline,
                "ledger_gap": self.offsets.ledger_gap, "note_x": self.offsets.note_x,
                "note_gap": self.offsets.note_gap, "glyph_top": self.offsets.glyph_top},
        })
    }

    fn chrome_json(&self) -> Value {
        let c = &self.chrome;
        json!({"text_px": c.text_px, "line": c.line, "small_px": c.small_px,
            "table_px": c.table_px, "tiny_px": c.tiny_px, "trust_px": c.trust_px,
            "head_weight": c.head_weight, "head_tracking": c.head_tracking,
            "head_gap": c.head_gap, "tab_h": c.tab_h, "tab_pad": c.tab_pad,
            "tab_rule": c.tab_rule, "control_h": c.control_h, "control_pad": c.control_pad,
            "date_w": c.date_w, "bar_h": c.bar_h, "cell_pad": c.cell_pad,
            "badge_pad": c.badge_pad, "face_pad": c.face_pad, "face_min_h": c.face_min_h})
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_well_formed() {
        assert_eq!(LOOKS[0].name, DEFAULT_LOOK);
        assert_eq!(LOOKS[0], R4);
        let mut seen = Vec::new();
        for l in LOOKS {
            assert!(!seen.contains(&l.name), "duplicate look {}", l.name);
            seen.push(l.name);
            assert!(l.name.starts_with(|c: char| c.is_ascii_lowercase()));
            assert!(l
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'));
            assert!(["dark", "light"].contains(&l.scheme), "{}", l.name);
            assert_eq!(l.dark.name, "dark");
            assert_eq!(
                l.dark.series[0], l.dark.accent,
                "{}: s1 is the accent",
                l.name
            );
            assert_eq!(
                l.light.series[0], l.light.accent,
                "{}: s1 is the accent",
                l.name
            );
            assert_eq!(l.light.name, "light");
            // Every role's font is embedded; the mark roles have distinct sizes.
            let mut sizes = Vec::new();
            for r in ROLES {
                let role = l.types.role(r).unwrap();
                assert!(
                    font(role.font).is_some(),
                    "{} {r}: font {}",
                    l.name,
                    role.font
                );
                assert!(role.px > 0.0 && role.line >= 1.0, "{} {r}", l.name);
                if ["headline", "body", "label", "note"].contains(&r) {
                    assert!(!sizes.contains(&role.px.to_bits()), "{} {r}", l.name);
                    sizes.push(role.px.to_bits());
                    assert_eq!(l.types.mark_role(role.px), Some(r));
                }
            }
            assert!(l.motion.duration_scale > 0.0);
            assert!(l.to_json()["type"]["headline"]["px"].is_number());
        }
        for f in FONTS {
            assert!(f.advance > 0.0 && f.ascent > 0.0, "{}", f.id);
        }
        assert_eq!(FONTS[0].id, DEFAULT_FONT);
        assert!(by_name(Some("nope")).is_err());
        assert_eq!(by_name(None).unwrap().name, "r4");
    }

    #[test]
    fn ease_is_monotone_and_ends_fixed() {
        for e in [Ease::Linear, Ease::Bezier([0.2, 0.0, 0.0, 1.0])] {
            assert!(e.apply(0.0).abs() < 1e-9);
            assert!((e.apply(1.0) - 1.0).abs() < 1e-9);
            let mut prev = 0.0;
            for i in 0..=20 {
                let v = e.apply(f64::from(i) / 20.0);
                assert!(v + 1e-12 >= prev);
                prev = v;
            }
        }
        assert_eq!(Ease::Linear.apply(0.37).to_bits(), 0.37_f64.to_bits());
    }

    #[test]
    fn r4_css_matches_the_former_stylesheet() {
        let d: std::collections::BTreeMap<_, _> = css_vars(&R4, None).into_iter().collect();
        assert_eq!(d["--bg"], "#0f1115");
        assert_eq!(d["--card"], "#161921");
        assert_eq!(d["--accent"], "#5b9cff");
        let l: std::collections::BTreeMap<_, _> =
            css_vars(&R4, Some("light")).into_iter().collect();
        assert_eq!(l["--bg"], "#f6f7f9");
        assert_eq!(l["--page"], "#c9ccd3");
        // (R5/T3b) The sizes the stylesheet held before they moved into the record.
        for (k, v) in [
            ("--qp-fs", "14px"),
            ("--qp-lh", "1.35"),
            ("--qp-fs-small", "12px"),
            ("--qp-fs-table", "13px"),
            ("--qp-fs-tiny", "11px"),
            ("--qp-fs-trust", "10px"),
            ("--qp-head-weight", "600"),
            ("--qp-head-tracking", "0.04em"),
            ("--qp-head-gap", "10px"),
            ("--qp-tab-h", "auto"),
            ("--qp-tab-pad", "6px"),
            ("--qp-tab-rule", "2px"),
            ("--qp-control-h", "auto"),
            ("--qp-control-pad", "4px 6px"),
            ("--qp-date-w", "120px"),
            ("--qp-bar-h", "48px"),
            ("--qp-cell-pad", "3px 4px"),
            ("--qp-badge-pad", "1px 6px"),
            ("--qp-face-pad", "14px 12px"),
            ("--qp-face-min-h", "60px"),
            ("--qp-rule", "1px"),
            ("--qp-space-0", "4px"),
            ("--qp-space-1", "6px"),
            ("--qp-space-2", "8px"),
            ("--qp-space-3", "12px"),
            ("--qp-space-5", "24px"),
        ] {
            assert_eq!(d[k], v, "{k}");
        }
    }

    /// (R5/T3b) A proportional file is measured glyph by glyph; a monospaced one keeps
    /// R3's estimate bit for bit.
    #[test]
    fn text_width_uses_the_advance_table() {
        static PROP: Advances = Advances {
            upem: 1000,
            notdef: 500,
            mono: false,
            runs: &[(0x20, 0x20, 250), (0x69, 0x69, 230), (0x6d, 0x6d, 820)],
        };
        let prop = FontAsset {
            id: "prop",
            advances: &PROP,
            ..FONTS[0]
        };
        // "mi i": 820 + 230 + 250 + 230 = 1530 units; at 10 px: 15.3 → 16.
        assert_eq!(prop.width("mi i", 10.0), 16);
        // Unmapped: the missing glyph's advance.
        assert_eq!(prop.width("x", 10.0), 5);
        assert_eq!(prop.width("", 10.0), 1);
        let mono = &FONTS[0];
        assert!(mono.advances.mono);
        for (t, s) in [
            ("Benzerleri: 74'te 36 tuttu", 11.0),
            ("C 101.25", 16.0),
            ("ı", 9.0),
        ] {
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let r3 = ((t.chars().count() as f64 * s * 0.6).ceil() as i32).max(1);
            assert_eq!(mono.width(t, s), r3, "{t}");
        }
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

    /// WCAG contrast of `fg` (with its alpha) over the opaque `bg`.
    fn contrast(fg: Rgba, bg: Rgba) -> f64 {
        let a = f64::from(fg[3]) / 255.0;
        let mix = |i: usize| f64::from(bg[i]) + a * (f64::from(fg[i]) - f64::from(bg[i]));
        let (l1, l2) = (
            lum([mix(0), mix(1), mix(2)]),
            lum([0, 1, 2].map(|i| f64::from(bg[i]))),
        );
        (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05)
    }

    /// (R5/T3b) The trial look meets the hard thresholds from its record alone.
    #[test]
    fn ornek_meets_the_hard_thresholds() {
        let o = &ORNEK;
        for r in ROLES {
            assert!(o.types.role(r).unwrap().px >= 13.0, "{r}");
        }
        let c = &o.chrome;
        for px in [c.text_px, c.small_px, c.table_px, c.tiny_px, c.trust_px] {
            assert!(px >= 13.0);
        }
        assert!(c.tab_h >= 44.0 && c.control_h >= 44.0);
        assert!(
            c.bar_h >= c.control_h + 2.0 * 6.0,
            "the control bar holds a control"
        );
        for t in [o.dark, o.light] {
            let floor = [t.closed[0], t.closed[1], t.closed[2], t.closed_alpha];
            let k = contrast(floor, t.bg);
            assert!(k >= 3.0, "{}: closed glyph {k:.2}:1", t.name);
            let w = contrast(t.warn, t.bg);
            assert!(w >= 4.5, "{}: evidence note {w:.2}:1", t.name);
            let q = contrast(t.chrome.req, t.chrome.bg);
            assert!(q >= 4.5 || t.name == "dark", "{}: req {q:.2}:1", t.name);
        }
        // r4 keeps freshness alone.
        assert_eq!((R4.dark.closed_alpha, R4.light.closed_alpha), (0, 0));
    }
}
