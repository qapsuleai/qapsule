//! SPEC §5(a): computed distinctness of realized ↔ failed (`m11 measure --style <name>`).
//!
//! The R3/M1 method (`rounds/R3/experiments/harness/src/m1/distinct.rs`, copied):
//! per condition (size 16/24/32 px × theme light/dark × vision normal/deutan/protan/tritan/
//! gray) every glyph is rasterized once through the style, then sRGB → linear RGB →
//! Gaussian blur σ = 0.5 px (radius 2, normalised, edges clamped) → vision simulation
//! (Machado 2009 severity 1.0, clamped; or luminance) → CIELAB (D65). Pair distance =
//! `min` over the nine shifts `(dx, dy) ∈ {-1, 0, 1}²` of the mean ΔE76 over all pixels
//! (pixels shifted in from outside take the background).
//!
//! Pairs: for each of the 6 kinds × 3 directions (18), the glyph closed as `realized`
//! against the same glyph closed as `failed` ([`crate::sheets::glyph`]: strength 0.6,
//! confidence 0.6, one confirmation, freshness 1, no server mark, validity `closed`).
//! Every alphabet ([`crate::alphabets::ALPHABETS`]) is measured. **Main metric: the
//! weakest pair in the worst condition**, per alphabet and over both. For information
//! only (not in the selection rule): each closed glyph against the same glyph still open
//! (`final`, validity `valid`).

use std::collections::BTreeMap;
use std::path::Path;

use qp_grammar::glyph::outcome::OutcomeStyle;
use qp_grammar::theme::Theme;
use qp_schema::capsule::{Direction, MeaningKind};
use serde::{Deserialize, Serialize};

use crate::alphabets::{self, ALPHABETS};
use crate::color::{linear_lut, linear_to_lab, simulate, Vision};
use crate::io::{round, write, write_json};
use crate::render::{glyph_prims, glyph_rgba, out_of_box};
use crate::sheets::{glyph, Outcome, DIRS};

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
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::needless_range_loop
)]
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
#[allow(clippy::cast_possible_wrap)]
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

/// One kind × direction and its distance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairD {
    /// `kind.direction`, e.g. `breakout.up`.
    pub glyph: String,
    /// Distance (mean ΔE76, rounded to 4 decimals).
    pub d: f64,
}

/// Weakest pair and mean of a pair set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetStat {
    /// Number of pairs.
    pub pairs: usize,
    /// Weakest pair (first in canonical order on ties).
    pub weakest: PairD,
    /// Mean distance.
    pub mean: f64,
}

/// One condition's result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionResult {
    /// `16`, `24` or `32`.
    pub size: u32,
    /// `light` / `dark`.
    pub theme: String,
    /// Vision condition.
    pub vision: String,
    /// realized ↔ failed (the measured set).
    pub realized_failed: SetStat,
    /// realized ↔ open (information only).
    pub realized_open: SetStat,
    /// failed ↔ open (information only).
    pub failed_open: SetStat,
}

/// A worst value over conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worst {
    /// Condition label `alphabet/size/theme/vision`.
    pub condition: String,
    /// The pair.
    pub pair: PairD,
}

/// One alphabet's result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlphabetReport {
    /// Main metric for this alphabet: weakest realized ↔ failed pair, worst condition.
    pub main: Worst,
    /// Lowest realized ↔ failed mean over conditions.
    pub mean_worst: f64,
    /// realized ↔ open, worst condition (information).
    pub realized_open_worst: Worst,
    /// failed ↔ open, worst condition (information).
    pub failed_open_worst: Worst,
    /// Primitives outside the glyph box (contract: 0), summed over all draws.
    pub out_of_box_prims: usize,
    /// Every condition (30).
    pub conditions: Vec<ConditionResult>,
}

/// Full report (`raw/<style>/distinct.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Harness version.
    pub harness: String,
    /// Style name.
    pub style: String,
    /// Method summary.
    pub method: String,
    /// Main metric over every alphabet: weakest realized ↔ failed pair in the worst
    /// condition.
    pub main: Worst,
    /// Per alphabet.
    pub alphabets: BTreeMap<String, AlphabetReport>,
}

fn stat(pairs: &[PairD]) -> SetStat {
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
            glyph: String::new(),
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

fn worst_of(
    alphabet: &str,
    conditions: &[ConditionResult],
    f: &dyn Fn(&ConditionResult) -> &SetStat,
) -> Worst {
    let c = conditions
        .iter()
        .fold(None::<&ConditionResult>, |acc, c| match acc {
            Some(a) if f(a).weakest.d <= f(c).weakest.d => Some(a),
            _ => Some(c),
        })
        .expect("conditions are not empty");
    Worst {
        condition: format!("{alphabet}/{}/{}/{}", c.size, c.theme, c.vision),
        pair: f(c).weakest.clone(),
    }
}

/// `kind.direction` ids of the 18 pairs, canonical order.
#[must_use]
pub fn pair_ids() -> Vec<(MeaningKind, Direction, String)> {
    let mut v = Vec::new();
    for k in MeaningKind::ALL {
        for d in DIRS {
            v.push((k, d, format!("{}.{}", k.name(), crate::sheets::dir_name(d))));
        }
    }
    v
}

/// Measures one alphabet.
///
/// # Errors
/// Unknown alphabet or rasterizer failure.
pub fn measure_alphabet(
    style: &dyn OutcomeStyle,
    alphabet: &str,
) -> Result<AlphabetReport, String> {
    let alpha = alphabets::by_name(alphabet)?;
    let ids = pair_ids();
    let mut conditions = Vec::new();
    let mut oob = 0usize;
    for theme in [Theme::light(), Theme::dark()] {
        for size in SIZES {
            // (realized, failed, open) RGBA per pair
            let mut rgba: Vec<[Vec<u8>; 3]> = Vec::new();
            for (k, d, _) in &ids {
                let mut three: Vec<Vec<u8>> = Vec::new();
                for o in [Outcome::Realized, Outcome::Failed, Outcome::Open] {
                    let s = glyph(*k, *d, o, 0.6, Some(0.6), 1);
                    let (p, bx) = glyph_prims(style, alpha, &s, size, &theme);
                    oob += out_of_box(&p, bx);
                    three.push(glyph_rgba(style, alpha, &s, size, &theme)?);
                }
                let [a, b, c]: [Vec<u8>; 3] = three.try_into().expect("three images");
                rgba.push([a, b, c]);
            }
            for vision in Vision::ALL {
                let mut rf = Vec::new();
                let mut ro = Vec::new();
                let mut fo = Vec::new();
                for ((_, _, id), imgs) in ids.iter().zip(&rgba) {
                    let lab: Vec<LabImage> = imgs
                        .iter()
                        .map(|px| lab_image(px, size as usize, theme.bg, vision))
                        .collect();
                    let p = |i: usize, j: usize| PairD {
                        glyph: id.clone(),
                        d: distance(&lab[i], &lab[j]),
                    };
                    rf.push(p(0, 1));
                    ro.push(p(0, 2));
                    fo.push(p(1, 2));
                }
                conditions.push(ConditionResult {
                    size,
                    theme: theme.name.to_owned(),
                    vision: vision.name().to_owned(),
                    realized_failed: stat(&rf),
                    realized_open: stat(&ro),
                    failed_open: stat(&fo),
                });
            }
        }
    }
    let mean_worst = conditions
        .iter()
        .map(|c| c.realized_failed.mean)
        .fold(f64::INFINITY, f64::min);
    Ok(AlphabetReport {
        main: worst_of(alphabet, &conditions, &|c| &c.realized_failed),
        mean_worst,
        realized_open_worst: worst_of(alphabet, &conditions, &|c| &c.realized_open),
        failed_open_worst: worst_of(alphabet, &conditions, &|c| &c.failed_open),
        out_of_box_prims: oob,
        conditions,
    })
}

/// Runs the full measurement for one style over every alphabet.
///
/// # Errors
/// Rasterizer failure.
pub fn measure(style: &dyn OutcomeStyle) -> Result<Report, String> {
    let mut per = BTreeMap::new();
    for a in ALPHABETS {
        per.insert(a.to_owned(), measure_alphabet(style, a)?);
    }
    // Worst over alphabets in report order (first on ties).
    let main = ALPHABETS
        .iter()
        .map(|a| per[*a].main.clone())
        .fold(None::<Worst>, |acc, w| match acc {
            Some(a) if a.pair.d <= w.pair.d => Some(a),
            _ => Some(w),
        })
        .expect("alphabets are not empty");
    Ok(Report {
        harness: crate::HARNESS_VERSION.to_owned(),
        style: style.name().to_owned(),
        method: "qp_render CPU raster of OutcomeStyle::draw(alphabet); sRGB->linear; Gaussian \
                 blur sigma 0.5 px (radius 2, clamped edges) in linear RGB; Machado 2009 \
                 severity 1.0 in linear RGB (clamped) or luminance; CIELAB D65; pair distance = \
                 min over +-1 px shifts of mean DeltaE76 over all pixels (background shifted \
                 in); pairs = realized vs failed for 6 kinds x 3 directions; values rounded to 4 \
                 decimals"
            .to_owned(),
        main,
        alphabets: per,
    })
}

/// Markdown summary of a report.
#[must_use]
pub fn markdown(r: &Report) -> String {
    let mut s = format!("# M11 distinctness — `{}`\n\n", r.style);
    s.push_str(&format!(
        "Main metric (weakest realized ↔ failed pair, worst condition, both alphabets): \
         **{:.2}** — `{}` at `{}`.\n\n",
        r.main.pair.d, r.main.pair.glyph, r.main.condition
    ));
    s.push_str("| alphabet | main ΔE | pair | condition | lowest mean | realized↔open worst | failed↔open worst | out of box |\n|---|---:|---|---|---:|---:|---:|---:|\n");
    for (a, ar) in &r.alphabets {
        s.push_str(&format!(
            "| {a} | {:.2} | `{}` | `{}` | {:.2} | {:.2} | {:.2} | {} |\n",
            ar.main.pair.d,
            ar.main.pair.glyph,
            ar.main.condition,
            ar.mean_worst,
            ar.realized_open_worst.pair.d,
            ar.failed_open_worst.pair.d,
            ar.out_of_box_prims
        ));
    }
    for (a, ar) in &r.alphabets {
        s.push_str(&format!(
            "\n## `{a}` per condition (realized ↔ failed: weakest / mean; information: realized ↔ open, failed ↔ open weakest)\n\n\
             | size | theme | vision | min | mean | weakest pair | R↔open min | F↔open min |\n|---:|---|---|---:|---:|---|---:|---:|\n"
        ));
        for c in &ar.conditions {
            s.push_str(&format!(
                "| {} | {} | {} | {:.2} | {:.2} | {} | {:.2} | {:.2} |\n",
                c.size,
                c.theme,
                c.vision,
                c.realized_failed.weakest.d,
                c.realized_failed.mean,
                c.realized_failed.weakest.glyph,
                c.realized_open.weakest.d,
                c.failed_open.weakest.d
            ));
        }
    }
    s
}

/// Measures and writes `<root>/raw/<style>/distinct.json` and `distinct.md`.
///
/// # Errors
/// Rasterizer or I/O failure.
pub fn run(style: &dyn OutcomeStyle, root: &Path) -> Result<Report, String> {
    crate::io::check_name(style.name())?;
    let r = measure(style)?;
    let dir = root.join("raw").join(&r.style);
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
        assert!((distance(&w, &k) - 100.0).abs() < 0.01);
        let k2 = lab_image(&flat(8, black), 8, bg, Vision::Normal);
        assert!((distance(&w, &k2) - 76.5625).abs() < 0.01);
    }

    #[test]
    fn eighteen_pairs() {
        assert_eq!(pair_ids().len(), 18);
    }
}
