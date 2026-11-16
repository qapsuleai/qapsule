//! WCAG 2.x relative luminance and contrast ratio (SC 1.4.3, 1.4.11), sRGB 8-bit.

use qp_schema::scene::Rgba;

fn channel(c: u8) -> f64 {
    let s = f64::from(c) / 255.0;
    if s <= 0.040_45 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// Relative luminance of an opaque colour.
#[must_use]
pub fn luminance(c: Rgba) -> f64 {
    0.2126 * channel(c[0]) + 0.7152 * channel(c[1]) + 0.0722 * channel(c[2])
}

/// `fg` composited over the opaque `bg` (straight alpha, 8-bit rounding).
#[must_use]
pub fn over(fg: Rgba, bg: Rgba) -> Rgba {
    let a = f64::from(fg[3]) / 255.0;
    let mix = |f: u8, b: u8| -> u8 {
        let v = f64::from(f) * a + f64::from(b) * (1.0 - a);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let r = v.round().clamp(0.0, 255.0) as u8;
        r
    };
    [mix(fg[0], bg[0]), mix(fg[1], bg[1]), mix(fg[2], bg[2]), 255]
}

/// WCAG contrast ratio of `fg` (composited over `bg`) against `bg`, 1..=21.
#[must_use]
pub fn contrast(fg: Rgba, bg: Rgba) -> f64 {
    let f = luminance(over(fg, bg));
    let b = luminance(bg);
    let (hi, lo) = if f >= b { (f, b) } else { (b, f) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_ratios() {
        assert!((contrast([0, 0, 0, 255], [255, 255, 255, 255]) - 21.0).abs() < 1e-9);
        assert!((contrast([255, 255, 255, 255], [255, 255, 255, 255]) - 1.0).abs() < 1e-9);
        // #767676 on white is the classic 4.54:1.
        let r = contrast([0x76, 0x76, 0x76, 255], [255, 255, 255, 255]);
        assert!((r - 4.54).abs() < 0.01, "{r}");
    }
}
