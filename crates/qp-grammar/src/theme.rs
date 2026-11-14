//! Themes (`docs/GRAMMAR.md` §9): the engine owns the colour values, views carry only the
//! theme name and colour token names.

use qp_schema::scene::Rgba;

use crate::error::{ErrorKind, GrammarError};

/// A resolved theme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// Theme name.
    pub name: &'static str,
    /// Background.
    pub bg: Rgba,
    /// Grid lines.
    pub grid: Rgba,
    /// Axis lines.
    pub axis: Rgba,
    /// Title text.
    pub text: Rgba,
    /// Axis labels, rules.
    pub muted: Rgba,
    /// Rising candles, positive values.
    pub up: Rgba,
    /// Falling candles, negative values.
    pub down: Rgba,
    /// Accent (emphasis, the first series colour).
    pub accent: Rgba,
    /// Warning.
    pub warn: Rgba,
    /// Series palette `s1`..`s8` (`s1` = `accent`).
    pub series: [Rgba; 8],
}

/// Names of the theme colour tokens, for error hints.
pub const TOKENS: [&str; 17] = [
    "bg", "grid", "axis", "text", "muted", "up", "down", "accent", "warn", "s1", "s2", "s3", "s4",
    "s5", "s6", "s7", "s8",
];

/// The theme names the engine knows.
pub const THEMES: [&str; 2] = ["dark", "light"];

impl Theme {
    /// The default theme: R0's colours.
    #[must_use]
    pub fn dark() -> Self {
        let accent = [255, 183, 77, 230];
        Self {
            name: "dark",
            bg: [16, 18, 24, 255],
            grid: [34, 38, 48, 255],
            axis: [84, 90, 104, 255],
            text: [200, 205, 215, 255],
            muted: [130, 136, 150, 255],
            up: [38, 166, 154, 255],
            down: [239, 83, 80, 255],
            accent,
            warn: [255, 202, 40, 255],
            series: [
                accent,
                [0x42, 0xa5, 0xf5, 255],
                [0xab, 0x47, 0xbc, 255],
                [0x66, 0xbb, 0x6a, 255],
                [0xec, 0x40, 0x7a, 255],
                [0x26, 0xc6, 0xda, 255],
                [0x8d, 0x6e, 0x63, 255],
                [0xbd, 0xbd, 0xbd, 255],
            ],
        }
    }

    /// Light background, same token names.
    #[must_use]
    pub fn light() -> Self {
        let accent = [0xef, 0x6c, 0x00, 230];
        Self {
            name: "light",
            bg: [0xff, 0xff, 0xff, 255],
            grid: [0xe8, 0xea, 0xee, 255],
            axis: [0xa8, 0xad, 0xb8, 255],
            text: [0x1f, 0x23, 0x28, 255],
            muted: [0x5f, 0x66, 0x73, 255],
            up: [0x08, 0x99, 0x81, 255],
            down: [0xe5, 0x39, 0x35, 255],
            accent,
            warn: [0xf9, 0xa8, 0x25, 255],
            series: [
                accent,
                [0x1e, 0x88, 0xe5, 255],
                [0x8e, 0x24, 0xaa, 255],
                [0x43, 0xa0, 0x47, 255],
                [0xd8, 0x1b, 0x60, 255],
                [0x00, 0xac, 0xc1, 255],
                [0x6d, 0x4c, 0x41, 255],
                [0x75, 0x75, 0x75, 255],
            ],
        }
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
