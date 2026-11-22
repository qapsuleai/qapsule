//! Colour math of the design check: WCAG contrast, CIELAB (D65) under five vision
//! conditions, CIEDE2000, OKLCH chroma and HSL. The vision matrices are Machado,
//! Oliveira & Fernandes (2009) at severity 1.0 on linear RGB, as the R3 measurement
//! harness used them (`rounds/R3/experiments/harness/src/m1/color.rs`); grey = the
//! luminance of linear sRGB.

use qp_schema::scene::Rgba;

/// A vision condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vision {
    Normal,
    Gray,
    Protan,
    Deutan,
    Tritan,
}

impl Vision {
    pub const ALL: [Self; 5] = [
        Self::Normal,
        Self::Gray,
        Self::Protan,
        Self::Deutan,
        Self::Tritan,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Gray => "grey",
            Self::Protan => "protan",
            Self::Deutan => "deutan",
            Self::Tritan => "tritan",
        }
    }

    fn matrix(self) -> Option<[[f64; 3]; 3]> {
        Some(match self {
            Self::Protan => [
                [0.152_286, 1.052_583, -0.204_868],
                [0.114_503, 0.786_281, 0.099_216],
                [-0.003_882, -0.048_116, 1.051_998],
            ],
            Self::Deutan => [
                [0.367_322, 0.860_646, -0.227_968],
                [0.280_085, 0.672_501, 0.047_413],
                [-0.011_820, 0.042_940, 0.968_881],
            ],
            Self::Tritan => [
                [1.255_528, -0.076_749, -0.178_779],
                [-0.078_411, 0.930_809, 0.147_602],
                [0.004_733, 0.691_367, 0.303_900],
            ],
            Self::Normal | Self::Gray => return None,
        })
    }
}

fn lin(c: u8) -> f64 {
    let v = f64::from(c) / 255.0;
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn linear(c: Rgba) -> [f64; 3] {
    [lin(c[0]), lin(c[1]), lin(c[2])]
}

fn luminance(l: [f64; 3]) -> f64 {
    0.212_672_9 * l[0] + 0.715_152_2 * l[1] + 0.072_175_0 * l[2]
}

/// `fg` (with its alpha) composited over the opaque `bg`, opaque.
pub fn over(fg: Rgba, bg: Rgba) -> Rgba {
    let a = f64::from(fg[3]) / 255.0;
    let m = |i: usize| (f64::from(bg[i]) + a * (f64::from(fg[i]) - f64::from(bg[i]))).round();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = [m(0) as u8, m(1) as u8, m(2) as u8, 255];
    px
}

/// WCAG 2 contrast ratio of `fg` (alpha composited) over the opaque `bg`.
pub fn contrast(fg: Rgba, bg: Rgba) -> f64 {
    let (a, b) = (luminance(linear(over(fg, bg))), luminance(linear(bg)));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// CIELAB (D65) of an opaque colour under a vision condition.
pub fn lab(c: Rgba, v: Vision) -> [f64; 3] {
    let mut l = linear(c);
    if v == Vision::Gray {
        let y = luminance(l);
        l = [y, y, y];
    } else if let Some(m) = v.matrix() {
        l = [0, 1, 2].map(|i| (m[i][0] * l[0] + m[i][1] * l[1] + m[i][2] * l[2]).clamp(0.0, 1.0));
    }
    let x = 0.412_456_4 * l[0] + 0.357_576_1 * l[1] + 0.180_437_5 * l[2];
    let y = luminance(l);
    let z = 0.019_333_9 * l[0] + 0.119_192_0 * l[1] + 0.950_304_1 * l[2];
    let f = |t: f64| {
        const D: f64 = 6.0 / 29.0;
        if t > D * D * D {
            t.cbrt()
        } else {
            t / (3.0 * D * D) + 4.0 / 29.0
        }
    };
    let (fx, fy, fz) = (f(x / 0.950_47), f(y), f(z / 1.088_83));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// CIELAB chroma C*.
pub fn chroma_ab(c: Rgba) -> f64 {
    let [_, a, b] = lab(c, Vision::Normal);
    a.hypot(b)
}

/// CIEDE2000 (Sharma, Wu & Dalal 2005), kL = kC = kH = 1.
#[allow(clippy::many_single_char_names, clippy::similar_names)]
pub fn de00(p: [f64; 3], q: [f64; 3]) -> f64 {
    let (l1, a1, b1) = (p[0], p[1], p[2]);
    let (l2, a2, b2) = (q[0], q[1], q[2]);
    let cm = (a1.hypot(b1) + a2.hypot(b2)) / 2.0;
    let g = 0.5 * (1.0 - (cm.powi(7) / (cm.powi(7) + 25f64.powi(7))).sqrt());
    let (a1p, a2p) = ((1.0 + g) * a1, (1.0 + g) * a2);
    let (c1p, c2p) = (a1p.hypot(b1), a2p.hypot(b2));
    let hue = |b: f64, a: f64| {
        if a == 0.0 && b == 0.0 {
            0.0
        } else {
            b.atan2(a).to_degrees().rem_euclid(360.0)
        }
    };
    let (h1p, h2p) = (hue(b1, a1p), hue(b2, a2p));
    let dl = l2 - l1;
    let dc = c2p - c1p;
    let mut dh = h2p - h1p;
    if c1p * c2p == 0.0 {
        dh = 0.0;
    } else if dh > 180.0 {
        dh -= 360.0;
    } else if dh < -180.0 {
        dh += 360.0;
    }
    let dhh = 2.0 * (c1p * c2p).sqrt() * (dh / 2.0).to_radians().sin();
    let lm = (l1 + l2) / 2.0;
    let cmp = (c1p + c2p) / 2.0;
    let hm = if c1p * c2p == 0.0 {
        h1p + h2p
    } else if (h1p - h2p).abs() <= 180.0 {
        (h1p + h2p) / 2.0
    } else if h1p + h2p < 360.0 {
        (h1p + h2p + 360.0) / 2.0
    } else {
        (h1p + h2p - 360.0) / 2.0
    };
    let t = 1.0 - 0.17 * (hm - 30.0).to_radians().cos()
        + 0.24 * (2.0 * hm).to_radians().cos()
        + 0.32 * (3.0 * hm + 6.0).to_radians().cos()
        - 0.20 * (4.0 * hm - 63.0).to_radians().cos();
    let dtheta = 30.0 * (-((hm - 275.0) / 25.0).powi(2)).exp();
    let rc = 2.0 * (cmp.powi(7) / (cmp.powi(7) + 25f64.powi(7))).sqrt();
    let sl = 1.0 + 0.015 * (lm - 50.0).powi(2) / (20.0 + (lm - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * cmp;
    let sh = 1.0 + 0.015 * cmp * t;
    let rt = -(2.0 * dtheta).to_radians().sin() * rc;
    ((dl / sl).powi(2) + (dc / sc).powi(2) + (dhh / sh).powi(2) + rt * (dc / sc) * (dhh / sh))
        .sqrt()
}

/// ΔE00 of two opaque colours under a vision condition.
pub fn de(a: Rgba, b: Rgba, v: Vision) -> f64 {
    de00(lab(a, v), lab(b, v))
}

/// OKLCH chroma (Björn Ottosson's OKLab, from linear sRGB).
pub fn oklch_c(c: Rgba) -> f64 {
    let [r, g, b] = linear(c);
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    let a = 1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s;
    let bb = 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s;
    a.hypot(bb)
}

/// HSL saturation and lightness, percent.
pub fn hsl_sl(c: Rgba) -> (f64, f64) {
    let v = [0, 1, 2].map(|i| f64::from(c[i]) / 255.0);
    let (mx, mn) = (v[0].max(v[1]).max(v[2]), v[0].min(v[1]).min(v[2]));
    let l = (mx + mn) / 2.0;
    let s = if mx == mn {
        0.0
    } else {
        (mx - mn) / (1.0 - (2.0 * l - 1.0).abs())
    };
    (s * 100.0, l * 100.0)
}

#[test]
fn colour_math_matches_known_values() {
    // Sharma et al. (2005) test pair 1: ΔE00 = 2.0425.
    let d = de00([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485]);
    assert!((d - 2.0425).abs() < 1e-4, "{d}");
    assert!((contrast([0, 0, 0, 255], [255, 255, 255, 255]) - 21.0).abs() < 1e-4);
    assert!(oklch_c([128, 128, 128, 255]) < 1e-4);
    assert!(oklch_c([255, 0, 0, 255]) > 0.2);
    let (s, l) = hsl_sl([255, 0, 0, 255]);
    assert!((s - 100.0).abs() < 1e-9 && (l - 50.0).abs() < 1e-9);
    // Grey stays grey under every condition.
    for v in Vision::ALL {
        let g = lab([90, 90, 90, 255], v);
        assert!(g[1].abs() < 0.01 && g[2].abs() < 0.01, "{v:?}");
    }
}
