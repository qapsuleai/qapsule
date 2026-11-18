//! Themes (`docs/GRAMMAR.md` §9): the engine owns the colour values, views carry only the
//! theme name and colour token names.

use qp_schema::scene::Rgba;

use crate::error::{ErrorKind, GrammarError};

/// A resolved theme: the colour roles of one scheme of a look (`crate::look`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// Theme name.
    pub name: &'static str,
    /// Background (zemin).
    pub bg: Rgba,
    /// Grid lines.
    pub grid: Rgba,
    /// Axis lines.
    pub axis: Rgba,
    /// Title text (metin).
    pub text: Rgba,
    /// Axis labels, rules (ikincil metin).
    pub muted: Rgba,
    /// Rising candles, positive values (yukarı).
    pub up: Rgba,
    /// Falling candles, negative values (aşağı).
    pub down: Rgba,
    /// Accent (emphasis, the first series colour).
    pub accent: Rgba,
    /// Warning (uyarı).
    pub warn: Rgba,
    /// Series palette `s1`..`s8` (`s1` = `accent`).
    pub series: [Rgba; 8],
    /// (R5/T3) Surface of a card on the background (yüzey).
    pub surface: Rgba,
    /// (R5/T3) Hairlines and separators.
    pub line: Rgba,
    /// (R5/T3) A glyph without direction (nötr).
    pub neutral: Rgba,
    /// (R5/T3) A claim that still holds (geçerli).
    pub valid: Rgba,
    /// (R5/T3) A claim that no longer holds (geçersiz): its glyph and its sentence.
    pub invalid: Rgba,
    /// (R5/T3) A closed story's glyph (kapanmış işaret).
    pub closed: Rgba,
    /// (R5/T3b) The least alpha (0..=255) of a closed story's glyph: the alphabets fade a
    /// glyph with its freshness (A31: grey and transparent by freshness); a closed one
    /// never goes below this, so a look can keep it at a contrast it chose. `0`: the
    /// freshness alone (`r4`).
    pub closed_alpha: u8,
    /// (R5/T3c) The least opacity (0..=255) of an open story's glyph: the alphabets fade
    /// an open glyph with its freshness too; it never goes below this, so a look can keep
    /// an aging open glyph at a contrast it chose. `0`: the freshness alone (`r4`).
    pub fresh_alpha_min: u8,
    /// (R5/T3c) The least opacity (0..=255) of a glyph's confidence plate (the pale fill
    /// rising with the confidence): it fades with the glyph's freshness, never below this.
    /// `0`: the freshness alone (`r4`).
    pub plate_alpha_min: u8,
    /// (R5/T3) Focus and selection (odak).
    pub focus: Rgba,
    /// (R5/T3) The shell's own chrome (CSS custom properties, `look::css_vars`).
    pub chrome: Chrome,
}

/// (R5/T3) The colours of the shell's chrome (header, tabs, settings, controls) in one
/// scheme. The chrome is not a scene: these reach the page as CSS custom properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chrome {
    /// Phone background.
    pub bg: Rgba,
    /// Text.
    pub fg: Rgba,
    /// Secondary text.
    pub muted: Rgba,
    /// Borders.
    pub line: Rgba,
    /// Controls and cards.
    pub card: Rgba,
    /// Selection, the active tab.
    pub accent: Rgba,
    /// Text on `accent`.
    pub on_accent: Rgba,
    /// Page around the phone frame.
    pub page: Rgba,
    /// Notice banner (synthetic data) background and text.
    pub note_bg: Rgba,
    /// Notice banner text.
    pub note_fg: Rgba,
    /// Error toast background.
    pub alert_bg: Rgba,
    /// Error toast text.
    pub alert_fg: Rgba,
    /// "Requires an entitlement" text.
    pub req: Rgba,
}

/// Names of the theme colour tokens, for error hints.
pub const TOKENS: [&str; 24] = [
    "bg", "grid", "axis", "text", "muted", "up", "down", "accent", "warn", "s1", "s2", "s3", "s4",
    "s5", "s6", "s7", "s8", "surface", "line", "neutral", "valid", "invalid", "closed", "focus",
];

/// The theme names the engine knows.
pub const THEMES: [&str; 2] = ["dark", "light"];

const DARK_ACCENT: Rgba = [255, 183, 77, 230];
const LIGHT_ACCENT: Rgba = [0xef, 0x6c, 0x00, 230];

impl Theme {
    /// The `r4` dark scheme: R0's colours.
    pub const DARK: Theme = Theme {
        name: "dark",
        bg: [16, 18, 24, 255],
        grid: [34, 38, 48, 255],
        axis: [84, 90, 104, 255],
        text: [200, 205, 215, 255],
        muted: [130, 136, 150, 255],
        up: [38, 166, 154, 255],
        down: [239, 83, 80, 255],
        accent: DARK_ACCENT,
        warn: [255, 202, 40, 255],
        series: [
            DARK_ACCENT,
            [0x42, 0xa5, 0xf5, 255],
            [0xab, 0x47, 0xbc, 255],
            [0x66, 0xbb, 0x6a, 255],
            [0xec, 0x40, 0x7a, 255],
            [0x26, 0xc6, 0xda, 255],
            [0x8d, 0x6e, 0x63, 255],
            [0xbd, 0xbd, 0xbd, 255],
        ],
        surface: [16, 18, 24, 255],
        line: [34, 38, 48, 255],
        neutral: DARK_ACCENT,
        valid: [200, 205, 215, 255],
        invalid: [130, 136, 150, 255],
        closed: [130, 136, 150, 255],
        closed_alpha: 0,
        fresh_alpha_min: 0,
        plate_alpha_min: 0,
        focus: DARK_ACCENT,
        chrome: Chrome {
            bg: [0x0f, 0x11, 0x15, 255],
            fg: [0xe6, 0xe8, 0xec, 255],
            muted: [0x8a, 0x90, 0x9c, 255],
            line: [0x26, 0x2a, 0x33, 255],
            card: [0x16, 0x19, 0x21, 255],
            accent: [0x5b, 0x9c, 0xff, 255],
            on_accent: [0xff, 0xff, 0xff, 255],
            page: [0x05, 0x06, 0x0a, 255],
            note_bg: [0x5a, 0x44, 0x10, 255],
            note_fg: [0xff, 0xe2, 0xa0, 255],
            alert_bg: [0x4a, 0x18, 0x20, 255],
            alert_fg: [0xff, 0xd6, 0xdc, 255],
            req: [0xff, 0xb4, 0x4c, 255],
        },
    };

    /// The `r4` light scheme, same token names.
    pub const LIGHT: Theme = Theme {
        name: "light",
        bg: [0xff, 0xff, 0xff, 255],
        grid: [0xe8, 0xea, 0xee, 255],
        axis: [0xa8, 0xad, 0xb8, 255],
        text: [0x1f, 0x23, 0x28, 255],
        muted: [0x5f, 0x66, 0x73, 255],
        up: [0x08, 0x99, 0x81, 255],
        down: [0xe5, 0x39, 0x35, 255],
        accent: LIGHT_ACCENT,
        warn: [0xf9, 0xa8, 0x25, 255],
        series: [
            LIGHT_ACCENT,
            [0x1e, 0x88, 0xe5, 255],
            [0x8e, 0x24, 0xaa, 255],
            [0x43, 0xa0, 0x47, 255],
            [0xd8, 0x1b, 0x60, 255],
            [0x00, 0xac, 0xc1, 255],
            [0x6d, 0x4c, 0x41, 255],
            [0x75, 0x75, 0x75, 255],
        ],
        surface: [0xff, 0xff, 0xff, 255],
        line: [0xe8, 0xea, 0xee, 255],
        neutral: LIGHT_ACCENT,
        valid: [0x1f, 0x23, 0x28, 255],
        invalid: [0x5f, 0x66, 0x73, 255],
        closed: [0x5f, 0x66, 0x73, 255],
        closed_alpha: 0,
        fresh_alpha_min: 0,
        plate_alpha_min: 0,
        focus: LIGHT_ACCENT,
        chrome: Chrome {
            bg: [0xf6, 0xf7, 0xf9, 255],
            fg: [0x15, 0x17, 0x1c, 255],
            muted: [0x5d, 0x64, 0x70, 255],
            line: [0xd9, 0xdc, 0xe2, 255],
            card: [0xff, 0xff, 0xff, 255],
            accent: [0x25, 0x63, 0xeb, 255],
            on_accent: [0xff, 0xff, 0xff, 255],
            page: [0xc9, 0xcc, 0xd3, 255],
            note_bg: [0xff, 0xf1, 0xc7, 255],
            note_fg: [0x6b, 0x4b, 0x00, 255],
            alert_bg: [0x4a, 0x18, 0x20, 255],
            alert_fg: [0xff, 0xd6, 0xdc, 255],
            req: [0xff, 0xb4, 0x4c, 255],
        },
    };

    /// The default theme: R0's colours (the `r4` look's dark scheme).
    #[must_use]
    pub fn dark() -> Self {
        Self::DARK
    }

    /// Light background, same token names (the `r4` look's light scheme).
    #[must_use]
    pub fn light() -> Self {
        Self::LIGHT
    }

    /// The colour roles as JSON (`#rrggbbaa` strings), for `qp look show`.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        let h = |c: Rgba| format!("#{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3]);
        let mut m = serde_json::Map::new();
        for t in TOKENS {
            if let Some(c) = self.token(t) {
                m.insert(t.to_owned(), serde_json::Value::String(h(c)));
            }
        }
        m.insert(
            "closed_alpha".to_owned(),
            serde_json::Value::from(self.closed_alpha),
        );
        m.insert(
            "fresh_alpha_min".to_owned(),
            serde_json::Value::from(self.fresh_alpha_min),
        );
        m.insert(
            "plate_alpha_min".to_owned(),
            serde_json::Value::from(self.plate_alpha_min),
        );
        let c = &self.chrome;
        m.insert(
            "chrome".to_owned(),
            serde_json::json!({"bg": h(c.bg), "fg": h(c.fg), "muted": h(c.muted),
                "line": h(c.line), "card": h(c.card), "accent": h(c.accent),
                "on_accent": h(c.on_accent), "page": h(c.page), "note_bg": h(c.note_bg),
                "note_fg": h(c.note_fg), "alert_bg": h(c.alert_bg), "alert_fg": h(c.alert_fg),
                "req": h(c.req)}),
        );
        serde_json::Value::Object(m)
    }

    /// The theme named `name`.
    ///
    /// # Errors
    /// Unknown theme name.
    pub fn by_name(name: &str) -> Result<Self, GrammarError> {
        match name {
            "dark" => Ok(Self::dark()),
            "light" => Ok(Self::light()),
            _ => Err(GrammarError::new(
                ErrorKind::Invalid,
                "/theme",
                format!("unknown theme `{name}` (themes: {})", THEMES.join(", ")),
            )),
        }
    }

    /// The value of a colour token, if `name` is one.
    #[must_use]
    pub fn token(&self, name: &str) -> Option<Rgba> {
        Some(match name {
            "bg" => self.bg,
            "grid" => self.grid,
            "axis" => self.axis,
            "text" => self.text,
            "muted" => self.muted,
            "up" => self.up,
            "down" => self.down,
            "accent" => self.accent,
            "warn" => self.warn,
            "surface" => self.surface,
            "line" => self.line,
            "neutral" => self.neutral,
            "valid" => self.valid,
            "invalid" => self.invalid,
            "closed" => self.closed,
            "focus" => self.focus,
            _ => {
                let i = name.strip_prefix('s')?.parse::<usize>().ok()?;
                *self.series.get(i.checked_sub(1)?)?
            }
        })
    }

    /// Series palette colour for the `i`-th (0-based) uncoloured line/area/band in a panel.
    #[must_use]
    pub fn palette(&self, i: usize) -> Rgba {
        self.series[i % self.series.len()]
    }

    /// Resolves a colour: `#rrggbb`, `#rrggbbaa` or a token.
    ///
    /// # Errors
    /// Anything else (the reason lists the tokens).
    pub fn color(&self, spec: &str) -> Result<Rgba, String> {
        if let Some(hex) = spec.strip_prefix('#') {
            let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
            if hex.is_ascii() && (hex.len() == 6 || hex.len() == 8) {
                if let (Some(r), Some(g), Some(b)) = (byte(0), byte(2), byte(4)) {
                    let a = if hex.len() == 8 { byte(6) } else { Some(255) };
                    if let Some(a) = a {
                        return Ok([r, g, b, a]);
                    }
                }
            }
            return Err(format!(
                "invalid colour `{spec}` (use #rrggbb or #rrggbbaa)"
            ));
        }
        self.token(spec).ok_or_else(|| {
            format!(
                "unknown colour `{spec}` (use #rrggbb, #rrggbbaa or one of: {})",
                TOKENS.join(", ")
            )
        })
    }
}

/// (R5/T3b) `c` with its alpha raised to at least `min` (a closed glyph's floor,
/// [`Theme::closed_alpha`]); unchanged for `min = 0`.
#[must_use]
pub fn at_least(c: Rgba, min: u8) -> Rgba {
    [c[0], c[1], c[2], c[3].max(min)]
}

/// (R5/T3c) The opacity `o` raised to at least `min / 255` (a glyph's floor,
/// [`Theme::closed_alpha`], [`Theme::fresh_alpha_min`], [`Theme::plate_alpha_min`]); exactly `o` for `min = 0`.
#[must_use]
pub fn opacity_at_least(o: f64, min: u8) -> f64 {
    if min == 0 {
        o
    } else {
        o.max(f64::from(min) / 255.0)
    }
}

/// Multiplies a colour's alpha by `opacity` (clamped to `0..=1`).
#[must_use]
pub fn with_opacity(c: Rgba, opacity: f64) -> Rgba {
    let o = opacity.clamp(0.0, 1.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let a = (f64::from(c[3]) * o).round() as u8;
    [c[0], c[1], c[2], a]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours() {
        let t = Theme::dark();
        assert_eq!(t.color("#ef5350"), Ok([239, 83, 80, 255]));
        assert_eq!(t.color("#ffb74de6"), Ok([255, 183, 77, 230]));
        assert_eq!(t.color("accent"), Ok(t.accent));
        assert_eq!(t.color("s1"), Ok(t.accent));
        assert_eq!(t.color("s8"), Ok([0xbd, 0xbd, 0xbd, 255]));
        assert!(t.color("s9").is_err());
        assert!(t.color("s0").is_err());
        assert!(t.color("#12345").is_err());
        assert!(t.color("#gg0000").is_err());
        assert!(t.color("red").unwrap_err().contains("accent"));
        assert_eq!(with_opacity([1, 2, 3, 200], 0.5), [1, 2, 3, 100]);
        assert!(Theme::by_name("neon").is_err());
        assert_eq!(Theme::by_name("light").unwrap().name, "light");
    }
}
