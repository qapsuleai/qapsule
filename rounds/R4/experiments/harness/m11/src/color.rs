//! Colour science for M1 (SPEC "(a)"): sRGB decoding, colour-vision simulation, CIELAB
//! (D65) and ΔE76.
//!
//! Colour-vision deficiency: Machado, Oliveira & Fernandes (2009), "A Physiologically-based
//! Model for Simulation of Color Vision Deficiency", IEEE TVCG 15(6). The severity-1.0
//! matrices of the paper's Table (protanomaly, deuteranomaly, tritanomaly at 1.0 =
//! protanopia, deuteranopia, tritanopia) are applied to **linear** RGB, as in the paper,
//! then clamped to `[0, 1]`. Grayscale = the luminance `Y` of linear sRGB (a = b = 0).

/// A vision condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vision {
    /// Normal colour vision.
    Normal,
    /// Deuteranopia (Machado 2009, severity 1.0).
    Deutan,
    /// Protanopia (Machado 2009, severity 1.0).
    Protan,
    /// Tritanopia (Machado 2009, severity 1.0).
    Tritan,
    /// Luminance only.
    Gray,
}

impl Vision {
    /// All conditions, report order.
    pub const ALL: [Self; 5] = [
        Self::Normal,
        Self::Deutan,
        Self::Protan,
        Self::Tritan,
        Self::Gray,
    ];

    /// Report name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Deutan => "deutan",
            Self::Protan => "protan",
            Self::Tritan => "tritan",
            Self::Gray => "gray",
        }
    }

    /// The simulation matrix (linear RGB, row-major), if this is a CVD condition.
    #[must_use]
    pub fn matrix(self) -> Option<[[f64; 3]; 3]> {
        match self {
            Self::Protan => Some(PROTAN),
            Self::Deutan => Some(DEUTAN),
            Self::Tritan => Some(TRITAN),
            Self::Normal | Self::Gray => None,
        }
    }
}

/// Machado 2009, protanomaly severity 1.0.
pub const PROTAN: [[f64; 3]; 3] = [
    [0.152_286, 1.052_583, -0.204_868],
    [0.114_503, 0.786_281, 0.099_216],
    [-0.003_882, -0.048_116, 1.051_998],
];
/// Machado 2009, deuteranomaly severity 1.0.
pub const DEUTAN: [[f64; 3]; 3] = [
    [0.367_322, 0.860_646, -0.227_968],
    [0.280_085, 0.672_501, 0.047_413],
    [-0.011_820, 0.042_940, 0.968_881],
];
/// Machado 2009, tritanomaly severity 1.0.
pub const TRITAN: [[f64; 3]; 3] = [
    [1.255_528, -0.076_749, -0.178_779],
    [-0.078_411, 0.930_809, 0.147_602],
    [0.004_733, 0.691_367, 0.303_900],
];

/// sRGB 8-bit → linear (IEC 61966-2-1).
#[must_use]
pub fn srgb_to_linear(c: u8) -> f64 {
    let v = f64::from(c) / 255.0;
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Lookup table of [`srgb_to_linear`].
#[must_use]
pub fn linear_lut() -> [f64; 256] {
    let mut t = [0.0; 256];
    for (i, v) in t.iter_mut().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let b = i as u8;
        *v = srgb_to_linear(b);
    }
    t
}

/// Applies the vision condition to one linear RGB triple (clamped to `[0, 1]`). For
/// [`Vision::Gray`] all three channels become the luminance `Y`.
#[must_use]
pub fn simulate(v: Vision, rgb: [f64; 3]) -> [f64; 3] {
    let out = match v {
        Vision::Normal => rgb,
        Vision::Gray => {
            let y = 0.212_672_9 * rgb[0] + 0.715_152_2 * rgb[1] + 0.072_175_0 * rgb[2];
            [y, y, y]
        }
        _ => {
            let m = v
                .matrix()
                .unwrap_or([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
            [
                m[0][0] * rgb[0] + m[0][1] * rgb[1] + m[0][2] * rgb[2],
                m[1][0] * rgb[0] + m[1][1] * rgb[1] + m[1][2] * rgb[2],
                m[2][0] * rgb[0] + m[2][1] * rgb[1] + m[2][2] * rgb[2],
            ]
        }
    };
    [
        out[0].clamp(0.0, 1.0),
        out[1].clamp(0.0, 1.0),
        out[2].clamp(0.0, 1.0),
    ]
}

/// D65 reference white (2°), `Y = 1`.
pub const WHITE_D65: [f64; 3] = [0.950_47, 1.0, 1.088_83];

fn lab_f(t: f64) -> f64 {
    const D: f64 = 6.0 / 29.0;
    if t > D * D * D {
        t.cbrt()
    } else {
        t / (3.0 * D * D) + 4.0 / 29.0
    }
}

/// Linear sRGB → CIELAB (D65).
#[must_use]
pub fn linear_to_lab(rgb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rgb;
    let x = 0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b;
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175_0 * b;
    let z = 0.019_333_9 * r + 0.119_192_0 * g + 0.950_304_1 * b;
    let (fx, fy, fz) = (
        lab_f(x / WHITE_D65[0]),
        lab_f(y / WHITE_D65[1]),
        lab_f(z / WHITE_D65[2]),
    );
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// CIE76 colour difference.
#[must_use]
pub fn delta_e76(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// sRGB 8-bit triple → Lab under a vision condition (no blur).
#[must_use]
pub fn srgb8_to_lab(v: Vision, c: [u8; 3]) -> [f64; 3] {
    linear_to_lab(simulate(
        v,
        [
            srgb_to_linear(c[0]),
            srgb_to_linear(c[1]),
            srgb_to_linear(c[2]),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_white_is_100() {
        let w = srgb8_to_lab(Vision::Normal, [255, 255, 255]);
        let k = srgb8_to_lab(Vision::Normal, [0, 0, 0]);
        assert!((w[0] - 100.0).abs() < 0.01, "{w:?}");
        assert!(w[1].abs() < 0.01 && w[2].abs() < 0.01, "{w:?}");
        assert!(k[0].abs() < 1e-9);
        assert!((delta_e76(w, k) - 100.0).abs() < 0.01);
    }

    #[test]
    fn matrices_keep_neutrals() {
        for v in Vision::ALL {
            if let Some(m) = v.matrix() {
                for row in m {
                    assert!((row.iter().sum::<f64>() - 1.0).abs() < 1e-5);
                }
            }
            // grey stays grey in every condition
            let g = simulate(v, [0.3, 0.3, 0.3]);
            for c in g {
                assert!((c - 0.3).abs() < 1e-5, "{v:?} {g:?}");
            }
        }
    }

    #[test]
    fn red_green_collapse_for_red_green_deficiency() {
        let red = [213, 94, 0];
        let green = [0, 158, 115];
        let d = |v| delta_e76(srgb8_to_lab(v, red), srgb8_to_lab(v, green));
        let normal = d(Vision::Normal);
        assert!(normal > 50.0, "{normal}");
        // pure red vs pure green: protan/deutan shrink the difference a lot
        let r = [255, 0, 0];
        let g = [0, 255, 0];
        let dd = |v| delta_e76(srgb8_to_lab(v, r), srgb8_to_lab(v, g));
        assert!(dd(Vision::Deutan) < 0.5 * dd(Vision::Normal));
        assert!(dd(Vision::Protan) < 0.5 * dd(Vision::Normal));
        // tritan keeps red/green apart but merges blue/green-ish pairs more than normal
        assert!(dd(Vision::Tritan) > 0.5 * dd(Vision::Normal));
        let b = [0, 0, 255];
        let k = [0, 0, 0];
        let bk = |v| delta_e76(srgb8_to_lab(v, b), srgb8_to_lab(v, k));
        assert!(bk(Vision::Gray) < bk(Vision::Normal));
    }
}
