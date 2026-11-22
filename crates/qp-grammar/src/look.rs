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

mod css;
mod fonts;
mod oltan;
mod ornek;
mod r4;
mod registry;
mod zemre;

pub use css::css_vars;
pub use fonts::{font, Advances, FontAsset, DEFAULT_FONT, FONTS};
pub use oltan::OLTAN;
pub use ornek::ORNEK;
pub use r4::R4;
pub use registry::{
    by_name, companion_face, is_look, names, or_default, COMPANION_FACES, DEFAULT_LOOK, LOOKS,
    PRESENTED_LOOK,
};
pub use zemre::ZEMRE;

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
    /// `title` (R5/T3c), `body`, `label`, `note` with that size (a view names sizes, not
    /// roles; the face designs take the sizes from these roles, and the registry test
    /// checks the five sizes differ in every look). `None`: a size no role has.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn mark_role(&self, px: f64) -> Option<&'static str> {
        [
            ("headline", &self.headline),
            // (R5/T3c) A view's text at the title size (`vadrel`'s ledger band) is drawn
            // in the title's font (`r4`: Plex Mono, as before).
            ("title", &self.title),
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
    /// (R5/M21) Duration of a face's own change in the feed (a live value or a state
    /// change: `crate::motion`), ms, before [`Self::duration_scale`]. Read only by a motion
    /// design that marks changes (`motion::MOTIONS`); `r4`'s own motion marks none, so
    /// this value changes nothing under it.
    pub change_ms: f64,
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

/// (R5/S) The story chart `pavka`'s sizes and alphas (`crate::stories::pavka`; M18's
/// "values to record", `rounds/R5/experiments/M18/pavka/DESIGN.md`). `r4`: the design's
/// own values, so every scene stays byte-identical.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StoryLook {
    /// Share of the frame height the price context strip takes.
    pub price_share: f64,
    /// Least height of the price strip (px).
    pub price_min_h: u32,
    /// Least height left for the stories (px).
    pub story_min_h: u32,
    /// Lane height when every story fits (px).
    pub lane_h: i32,
    /// Thinnest lane before clustering (px).
    pub lane_min: i32,
    /// Thickest lane (px).
    pub lane_max: i32,
    /// Width of a cluster bucket (px).
    pub cluster_w: i32,
    /// Height of the cluster row (px).
    pub cluster_h: i32,
    /// Least hit box width of a lane (px).
    pub hit_min: i32,
    /// Alpha of the selected story's row band.
    pub focus_row_alpha: u8,
    /// Alpha of the selected story's guide lines.
    pub guide_alpha: u8,
    /// Lane bar thickness: selected, live, closed (px).
    pub bar_focus: i32,
    /// Lane bar thickness of a live story (px).
    pub bar_live: i32,
    /// Lane bar thickness of a closed story (px).
    pub bar_closed: i32,
    /// Half size of a lane's end mark (px).
    pub mark: f32,
    /// A coin-flip lane's dash and gap (px).
    pub dash: (i32, i32),
    /// Most dashes in one coin-flip lane.
    pub dash_max: i32,
}

/// (R5/S) The open capsule design `tesir`'s line and bar counts (`crate::open::tesir`;
/// M20's "values to record", `rounds/R5/experiments/M20/tesir/DESIGN.md`). Sizes,
/// colours and spacing already come from the look.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpenLook {
    /// Lines of the claim (the headline role).
    pub claim_lines: f64,
    /// Lines of the "why" detail (body role).
    pub why_lines: f64,
    /// Lines of the track record (body role).
    pub ledger_lines: f64,
    /// Height of the evidence thumbnail, in body lines.
    pub thumb_lines: f64,
    /// Lines of a block's note (label role).
    pub note_lines: f64,
    /// Bars the thumbnail shows on each side of the evidence's own window.
    pub thumb_lead: u64,
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
    /// (R5/S) The story chart's sizes (`pavka`).
    pub story: StoryLook,
    /// (R5/S) The open capsule design's counts (`tesir`).
    pub open: OpenLook,
}

const fn role(px: f64) -> TypeRole {
    TypeRole {
        px,
        line: 1.3,
        font: DEFAULT_FONT,
    }
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
                "ease": self.motion.ease.css(), "chrome_ms": self.motion.chrome_ms,
                "change_ms": self.motion.change_ms},
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

#[cfg(test)]
mod tests;
