//! M1 (a): computed distinctness (`harness m1 measure --alt <name>`).
//!
//! Per condition (size 16/24/32 px × theme light/dark × vision normal/deutan/protan/tritan/
//! gray) every glyph is rasterized once, then:
//!
//! 1. sRGB → linear RGB;
//! 2. Gaussian blur, σ = 0.5 px (kernel radius 2, normalised, edges clamped), in linear RGB
//!    (eye/screen point spread);
//! 3. vision simulation in linear RGB (Machado 2009 severity 1.0, clamped; or luminance);
//! 4. linear RGB → CIELAB (D65).
//!
//! Pair distance = `min` over the nine integer shifts `(dx, dy) ∈ {-1, 0, 1}²` of the
//! **mean ΔE76 over all pixels** between glyph `a` and glyph `b` shifted (pixels shifted in
//! from outside take the background colour). Reported per condition: the weakest pair of
//! `K` and the mean over `K`'s pairs, and the same per channel set. Main metric: **the
//! weakest `K` pair in the worst condition**.

use std::collections::BTreeMap;
use std::path::Path;

use qp_grammar::glyph::GlyphBox;
use qp_grammar::theme::Theme;
use qp_schema::capsule::GlyphSpec;
use serde::Serialize;

use super::color::{linear_lut, linear_to_lab, simulate, Vision};
use super::glyphset::{channel_pairs, core_set, kd_id, spec_id, CHANNELS};
use super::registry::M1Entry;
use super::render::{glyph_rgba, out_of_box};
use crate::io::{round, write, write_json};

/// Glyph sizes in px.
pub const SIZES: [u32; 3] = [16, 24, 32];
/// Blur σ in px.
pub const SIGMA: f64 = 0.5;

/// A glyph in Lab after blur and vision simulation.
#[derive(Debug, Clone)]
pub struct LabImage {
    /// Side length.
    pub size: usize,
    /// Pixels, row-major.
    pub px: Vec<[f64; 3]>,
    /// Background colour in Lab (used for pixels shifted in from outside).
    pub bg: [f64; 3],
}

fn kernel() -> [f64; 5] {
    let mut k = [0.0; 5];
    for (i, v) in k.iter_mut().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let x = i as f64 - 2.0;
        *v = (-(x * x) / (2.0 * SIGMA * SIGMA)).exp();
    }
    let s: f64 = k.iter().sum();
    k.map(|v| v / s)
}

/// Converts straight RGBA8 (`size`×`size`, opaque) on background `bg` to a [`LabImage`].
#[must_use]
pub fn lab_image(rgba: &[u8], size: usize, bg: [u8; 4], vision: Vision) -> LabImage {
    let lut = linear_lut();
    let n = size * size;
    let lin: Vec<[f64; 3]> = (0..n)
        .map(|i| {
            [
                lut[rgba[i * 4] as usize],
                lut[rgba[i * 4 + 1] as usize],
                lut[rgba[i * 4 + 2] as usize],
            ]
        })
        .collect();
    let k = kernel();
    let clampi = |v: isize| -> usize { v.clamp(0, size as isize - 1).unsigned_abs() };
    let mut tmp = vec![[0.0; 3]; n];
    for y in 0..size {
        for x in 0..size {
            let mut acc = [0.0; 3];
            for (j, w) in k.iter().enumerate() {
                let xx = clampi(x as isize + j as isize - 2);
                let p = lin[y * size + xx];
                for c in 0..3 {
                    acc[c] += w * p[c];
                }
            }
            tmp[y * size + x] = acc;
        }
    }
    let mut px = vec![[0.0; 3]; n];
    for y in 0..size {
        for x in 0..size {
            let mut acc = [0.0; 3];
            for (j, w) in k.iter().enumerate() {
                let yy = clampi(y as isize + j as isize - 2);
                let p = tmp[yy * size + x];
                for c in 0..3 {
                    acc[c] += w * p[c];
                }
            }
            px[y * size + x] = linear_to_lab(simulate(vision, acc));
        }
    }
    let bgl = [
        lut[bg[0] as usize],
        lut[bg[1] as usize],
        lut[bg[2] as usize],
    ];
    LabImage {
        size,
        px,
        bg: linear_to_lab(simulate(vision, bgl)),
    }
}

/// Pair distance: min over ±1 px shifts of the mean ΔE76.
#[must_use]
pub fn distance(a: &LabImage, b: &LabImage) -> f64 {
    let s = a.size;
    let mut best = f64::INFINITY;
    for dy in -1isize..=1 {
        for dx in -1isize..=1 {
            let mut sum = 0.0;
            for y in 0..s {
                for x in 0..s {
                    let (bx, by) = (x as isize - dx, y as isize - dy);
                    let pb = if bx < 0 || by < 0 || bx >= s as isize || by >= s as isize {
                        b.bg
                    } else {
                        b.px[by.unsigned_abs() * s + bx.unsigned_abs()]
                    };
                    let pa = a.px[y * s + x];
                    let d = [pa[0] - pb[0], pa[1] - pb[1], pa[2] - pb[2]];
                    sum += (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                }
            }
            #[allow(clippy::cast_precision_loss)]
            let mean = sum / (s * s) as f64;
            best = best.min(mean);
        }
    }
    best
}

/// One pair and its distance.
#[derive(Debug, Clone, Serialize)]
pub struct PairD {
    /// First glyph id.
    pub a: String,
    /// Second glyph id.
    pub b: String,
    /// Distance (mean ΔE76, rounded to 4 decimals).
    pub d: f64,
}

/// Weakest pair and mean of a pair set.
#[derive(Debug, Clone, Serialize)]
pub struct SetStat {
    /// Number of pairs.
    pub pairs: usize,
    /// Weakest pair (first in canonical order on ties).
    pub weakest: PairD,
    /// Mean distance.
    pub mean: f64,
}

/// One condition's result.
#[derive(Debug, Clone, Serialize)]
pub struct ConditionResult {
    /// `16`, `24` or `32`.
    pub size: u32,
    /// `light` / `dark`.
    pub theme: String,
    /// Vision condition.
    pub vision: String,
    /// The core set `K`.
    pub k: SetStat,
    /// Per channel.
    pub channels: BTreeMap<String, SetStat>,
}

/// A worst value over conditions.
#[derive(Debug, Clone, Serialize)]
pub struct Worst {
    /// Condition label `size/theme/vision`.
    pub condition: String,
    /// The pair.
    pub pair: PairD,
}

/// Full report (`distinct.json`).
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// Harness version.
    pub harness: String,
    /// Alternative name.
    pub alt: String,
    /// Method summary.
    pub method: String,
    /// Main metric: weakest `K` pair in the worst condition.
    pub main: Worst,
    /// Lowest `K` mean over conditions.
    pub k_mean_worst: f64,
    /// Per channel: weakest pair in the worst condition.
    pub channel_worst: BTreeMap<String, Worst>,
    /// Primitives outside the glyph box (contract: 0), summed over all draws.
    pub out_of_box_prims: usize,
    /// Every condition.
    pub conditions: Vec<ConditionResult>,
}

fn stat(pairs: Vec<PairD>) -> SetStat {
    let n = pairs.len();
    #[allow(clippy::cast_precision_loss)]
    let mean = pairs.iter().map(|p| p.d).sum::<f64>() / n.max(1) as f64;
    let weakest = pairs
        .iter()
        .fold(None::<&PairD>, |acc, p| match acc {
            Some(a) if a.d <= p.d => Some(a),
            _ => Some(p),
        })
        .cloned()
        .unwrap_or(PairD {
            a: String::new(),
            b: String::new(),
            d: 0.0,
        });
    SetStat {
        pairs: n,
        weakest: PairD {
            d: round(weakest.d, 4),
            ..weakest
        },
        mean: round(mean, 4),
    }
}

/// Runs the full measurement for one alphabet.
///
/// # Errors
/// Rasterizer failure.
pub fn measure(entry: &M1Entry) -> Result<Report, String> {
    let alpha = entry.alphabet.as_ref();
    let k = core_set();
    let cp = channel_pairs();
    // every distinct spec, canonical order
    let mut specs: BTreeMap<String, GlyphSpec> = BTreeMap::new();
    for s in &k {
        specs.insert(spec_id(s), s.clone());
    }
    for p in &cp {
        specs.insert(spec_id(&p.a), p.a.clone());
        specs.insert(spec_id(&p.b), p.b.clone());
    }
    let mut conditions = Vec::new();
    let mut oob = 0usize;
    for theme in [Theme::light(), Theme::dark()] {
        for size in SIZES {
            let mut rgba: BTreeMap<&str, Vec<u8>> = BTreeMap::new();
            for (id, s) in &specs {
                #[allow(clippy::cast_precision_loss)]
                let bx = GlyphBox {
                    x: 0.0,
                    y: 0.0,
                    size: size as f32,
                };
                oob += out_of_box(&alpha.draw(s, bx, &theme), bx);
                rgba.insert(id.as_str(), glyph_rgba(alpha, s, size, &theme)?);
            }
            for vision in Vision::ALL {
                let lab: BTreeMap<&str, LabImage> = rgba
                    .iter()
                    .map(|(id, px)| (*id, lab_image(px, size as usize, theme.bg, vision)))
                    .collect();
                let d = |a: &GlyphSpec, b: &GlyphSpec| {
                    distance(&lab[spec_id(a).as_str()], &lab[spec_id(b).as_str()])
                };
                let mut kp = Vec::new();
                for i in 0..k.len() {
                    for j in i + 1..k.len() {
                        kp.push(PairD {
                            a: kd_id(&k[i]),
                            b: kd_id(&k[j]),
                            d: d(&k[i], &k[j]),
                        });
                    }
                }
                let mut channels = BTreeMap::new();
                for ch in CHANNELS {
                    let v: Vec<PairD> = cp
                        .iter()
                        .filter(|p| p.channel == ch)
                        .map(|p| PairD {
                            a: spec_id(&p.a),
                            b: spec_id(&p.b),
                            d: d(&p.a, &p.b),
                        })
                        .collect();
                    channels.insert(ch.to_owned(), stat(v));
                }
                conditions.push(ConditionResult {
                    size,
                    theme: theme.name.to_owned(),
                    vision: vision.name().to_owned(),
                    k: stat(kp),
                    channels,
                });
            }
        }
    }
    let label = |c: &ConditionResult| format!("{}/{}/{}", c.size, c.theme, c.vision);
    let worst_of = |f: &dyn Fn(&ConditionResult) -> &SetStat| -> Worst {
        let c = conditions
            .iter()
            .fold(None::<&ConditionResult>, |acc, c| match acc {
                Some(a) if f(a).weakest.d <= f(c).weakest.d => Some(a),
                _ => Some(c),
            })
            .expect("conditions are not empty");
        Worst {
            condition: label(c),
            pair: f(c).weakest.clone(),
        }
    };
    let main = worst_of(&|c: &ConditionResult| &c.k);
    let mut channel_worst = BTreeMap::new();
    for ch in CHANNELS {
        channel_worst.insert(
            ch.to_owned(),
            worst_of(&|c: &ConditionResult| &c.channels[ch]),
        );
    }
    let k_mean_worst = conditions
        .iter()
        .map(|c| c.k.mean)
        .fold(f64::INFINITY, f64::min);
    Ok(Report {
        harness: crate::HARNESS_VERSION.to_owned(),
        alt: alpha.name().to_owned(),
        method: "qp_render CPU raster; sRGB->linear; Gaussian blur sigma 0.5 px (radius 2, \
                 clamped edges) in linear RGB; Machado 2009 severity 1.0 in linear RGB (clamped) \
                 or luminance; CIELAB D65; pair distance = min over +-1 px shifts of mean \
                 DeltaE76 over all pixels (background shifted in); values rounded to 4 decimals"
            .to_owned(),
        main,
        k_mean_worst,
        channel_worst,
        out_of_box_prims: oob,
        conditions,
    })
}

/// Markdown table of a report.
#[must_use]
pub fn markdown(r: &Report) -> String {
    let mut s = String::new();
    s.push_str(&format!("# M1 distinctness — `{}`\n\n", r.alt));
    s.push_str(&format!(
        "Main metric (weakest `K` pair in the worst condition): **{:.2}** — `{}` vs `{}` at `{}`.\n\n",
        r.main.pair.d, r.main.pair.a, r.main.pair.b, r.main.condition
    ));
    s.push_str(&format!(
        "Lowest `K` mean over conditions: {:.2}. Primitives outside the box: {}.\n\n",
        r.k_mean_worst, r.out_of_box_prims
    ));
    s.push_str("## Weakest pair per channel (worst condition)\n\n| channel | ΔE | condition | a | b |\n|---|---:|---|---|---|\n");
    for (ch, w) in &r.channel_worst {
        s.push_str(&format!(
            "| {ch} | {:.2} | {} | `{}` | `{}` |\n",
            w.pair.d, w.condition, w.pair.a, w.pair.b
        ));
    }
    s.push_str("\n## Per condition (weakest pair / mean, mean ΔE76)\n\n| size | theme | vision | K min | K mean | K weakest pair |");
    for ch in CHANNELS {
        s.push_str(&format!(" {ch} min |"));
    }
    s.push_str("\n|---:|---|---|---:|---:|---|");
    for _ in CHANNELS {
        s.push_str("---:|");
    }
    s.push('\n');
    for c in &r.conditions {
        s.push_str(&format!(
            "| {} | {} | {} | {:.2} | {:.2} | {} / {} |",
            c.size, c.theme, c.vision, c.k.weakest.d, c.k.mean, c.k.weakest.a, c.k.weakest.b
        ));
        for ch in CHANNELS {
            s.push_str(&format!(" {:.2} |", c.channels[ch].weakest.d));
        }
        s.push('\n');
    }
    s
}

/// Measures and writes `<root>/M1/raw/<alt>/distinct.json` and `distinct.md`.
///
/// # Errors
/// Rasterizer or I/O failure.
pub fn run(entry: &M1Entry, root: &Path) -> Result<Report, String> {
    let r = measure(entry)?;
    let dir = super::raw_dir(root, &r.alt);
    write_json(&dir.join("distinct.json"), &r)?;
    write(&dir.join("distinct.md"), markdown(&r).as_bytes())?;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(size: usize, c: [u8; 4]) -> Vec<u8> {
        (0..size * size).flat_map(|_| c).collect()
    }

    #[test]
    fn identical_is_zero_and_black_white_is_100() {
        let bg = [255, 255, 255, 255];
        let w = lab_image(&flat(8, bg), 8, bg, Vision::Normal);
        let black = [0, 0, 0, 255];
        let k = lab_image(&flat(8, black), 8, black, Vision::Normal);
        assert!(distance(&w, &w).abs() < 1e-9);
        let d = distance(&w, &k);
        assert!((d - 100.0).abs() < 0.01, "{d}");
        // with a white background shifted in, a diagonal shift turns one row and one
        // column (15 pixels) of the black image white: min over shifts = 100 * (1 - 15/64)
        let k2 = lab_image(&flat(8, black), 8, bg, Vision::Normal);
        assert!((distance(&w, &k2) - 76.5625).abs() < 0.01);
    }

    #[test]
    fn shift_tolerance() {
        let bg = [255, 255, 255, 255];
        let mut a = flat(8, bg);
        let mut b = flat(8, bg);
        let dot = |v: &mut Vec<u8>, x: usize, y: usize| {
            v[(y * 8 + x) * 4..(y * 8 + x) * 4 + 3].copy_from_slice(&[0, 0, 0]);
        };
        dot(&mut a, 3, 3);
        dot(&mut b, 4, 3);
        let la = lab_image(&a, 8, bg, Vision::Normal);
        let lb = lab_image(&b, 8, bg, Vision::Normal);
        assert!(distance(&la, &lb) < 1e-9);
    }
}
