//! The looks' font assets ([`FontAsset`]), their advance tables ([`Advances`]) and the
//! font registry ([`FONTS`]).

/// One embedded font file (a single family at a single weight: the reference renderer,
/// fontdue, draws a variable font at its default instance only).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontAsset {
    /// Asset id (`[a-z][a-z0-9-]*`): what a [`TypeRole`](super::TypeRole) and a scene's text name.
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
pub static FONTS: &[FontAsset] = &[
    FontAsset {
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
    },
    // (R5/M16 `oltan`) Atkinson Hyperlegible Next 2.001 (Braille Institute; OFL-1.1, no
    // Reserved Font Name): static instances of the variable file at wght 400 and 600, Latin
    // subset, the figures 0–9 remapped in the cmap to the tabular glyphs (632 units each).
    // `advance`: the mean advance over a marla headline; `ascent`: the accented capitals'
    // top (İ, Ü: 861 units), so a wrapped block's first line stays inside its box.
    // (R5/M16 `zemre`) its 700 instance, same build
    // (`rounds/R5/experiments/M16/zemre/build_font.py`); (R5/S) the 400 file is shared.
    FontAsset {
        id: "atkinson-next",
        family: "Atkinson Hyperlegible Next",
        weight: 400,
        italic: false,
        file: "AtkinsonHyperlegibleNext-Regular.ttf",
        license: "OFL-AtkinsonHyperlegibleNext.txt",
        spdx: "OFL-1.1",
        css_family: "QpAtkinson",
        advance: 0.46,
        ascent: 0.86,
        advances: &crate::advances::ATKINSON_NEXT,
    },
    FontAsset {
        id: "atkinson-next-semibold",
        family: "Atkinson Hyperlegible Next",
        weight: 600,
        italic: false,
        file: "AtkinsonHyperlegibleNext-SemiBold.ttf",
        license: "OFL-AtkinsonHyperlegibleNext.txt",
        spdx: "OFL-1.1",
        css_family: "QpAtkinsonSemi",
        advance: 0.47,
        ascent: 0.86,
        advances: &crate::advances::ATKINSON_NEXT_SEMIBOLD,
    },
    FontAsset {
        id: "atkinson-next-bold",
        family: "Atkinson Hyperlegible Next",
        weight: 700,
        italic: false,
        file: "AtkinsonHyperlegibleNext-Bold.ttf",
        license: "OFL-AtkinsonHyperlegibleNext.txt",
        spdx: "OFL-1.1",
        css_family: "QpAtkinsonNextBold",
        advance: 0.526,
        ascent: 0.8,
        advances: &crate::advances::ATKINSON_NEXT_BOLD,
    },
];

/// The font asset `id`, if embedded.
#[must_use]
pub fn font(id: &str) -> Option<&'static FontAsset> {
    FONTS.iter().find(|f| f.id == id)
}
