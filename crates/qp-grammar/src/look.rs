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
    /// Mean advance per character, in em: the compiler's width estimate (it has no font;
    /// exact for a monospaced file).
    pub advance: f64,
    /// Ascent in em: where the first baseline of a wrapped text block sits below its top.
    pub ascent: f64,
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
};

// --- M16 looks: `const` records go below this line ---------------------------------------

/// (R5/T3) A trial look proving the mechanism: other colours and larger type, same font.
/// Not a design proposal.
pub const ORNEK: Look = Look {
    name: "ornek",
    note: "T3 trial: warm colours, larger type (mechanism check, not a proposal)",
    scheme: "dark",
    types: TypeScale {
        headline: role(22.0),
        title: role(15.0),
        body: role(13.0),
        label: role(12.0),
        figure: role(12.0),
        note: role(10.0),
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
        closed: [110, 98, 90, 255],
        invalid: [100, 88, 80, 255],
        chrome: Chrome {
            bg: [252, 248, 242, 255],
            fg: [36, 28, 24, 255],
            muted: [100, 88, 80, 255],
            line: [222, 212, 202, 255],
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
        })
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
    }
}
