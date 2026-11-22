//! The view's panels: canonical order, layer ids and the pixel rectangles.

use super::*;

/// The view's panels, canonical (`docs/GRAMMAR.md` §11 rule 1) even when the input still
/// uses the deprecated `y` list.
pub(super) fn canonical_panels(view: &View) -> Result<Vec<Panel>, GrammarError> {
    if !view.panels.is_empty() && !view.y.is_empty() {
        return Err(GrammarError::new(
            ErrorKind::Invalid,
            "/y",
            "both `panels` and the deprecated `y` are set",
        ));
    }
    let mut panels: Vec<Panel> = if !view.panels.is_empty() {
        view.panels.clone()
    } else if !view.y.is_empty() {
        (0u32..)
            .zip(&view.y)
            .map(|(i, y)| Panel {
                id: Some(i),
                name: None,
                weight: None,
                y: y.clone(),
            })
            .collect()
    } else {
        let ids: BTreeSet<u32> = view.layers.iter().map(|l| l.panel).collect();
        ids.into_iter()
            .map(|i| Panel {
                id: Some(i),
                name: None,
                weight: None,
                y: Scale::new(ScaleKind::Linear),
            })
            .collect()
    };
    let mut next = panels
        .iter()
        .filter_map(|p| p.id)
        .max()
        .map_or(0, |m| m + 1);
    for p in &mut panels {
        if p.id.is_none() {
            p.id = Some(next);
            next += 1;
        }
    }
    let mut seen = BTreeSet::new();
    for (i, p) in panels.iter().enumerate() {
        let id = p.id.unwrap_or_default();
        if !seen.insert(id) {
            return Err(GrammarError::new(
                ErrorKind::Invalid,
                format!("/panels/{i}/id"),
                format!("duplicate panel id {id}"),
            ));
        }
    }
    Ok(panels)
}

/// Layer ids, assigning the smallest unused `l<k>` to layers without one (§11 rule 3).
pub(super) fn layer_ids(layers: &[Layer]) -> Result<Vec<String>, GrammarError> {
    let mut used: BTreeSet<String> = BTreeSet::new();
    for (i, l) in layers.iter().enumerate() {
        if let Some(id) = &l.id {
            if !used.insert(id.clone()) {
                return Err(GrammarError::new(
                    ErrorKind::Invalid,
                    format!("/layers/{i}/id"),
                    format!("duplicate layer id `{id}`"),
                ));
            }
        }
    }
    let mut k = 0u32;
    Ok(layers
        .iter()
        .map(|l| {
            l.id.clone().unwrap_or_else(|| loop {
                let c = format!("l{k}");
                k += 1;
                if used.insert(c.clone()) {
                    break c;
                }
            })
        })
        .collect())
}

/// Panel rectangles: the plot split by weight with 8 px gaps (last panel takes the rest).
pub(super) fn panel_rects(
    panels: &[Panel],
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
    gap: i32,
) -> Result<Vec<PixelRect>, GrammarError> {
    let n = i32::try_from(panels.len()).unwrap_or(i32::MAX);
    let mut weights = Vec::with_capacity(panels.len());
    for (i, p) in panels.iter().enumerate() {
        let w = p.weight.unwrap_or(if i == 0 { 3.0 } else { 1.0 });
        if !(w.is_finite() && w > 0.0) {
            return Err(GrammarError::new(
                ErrorKind::Invalid,
                format!("/panels/{i}/weight"),
                "weight must be > 0",
            ));
        }
        weights.push(w);
    }
    let avail = bottom - top - gap * (n - 1);
    if avail < MIN_PANEL_H * n {
        return Err(GrammarError::new(
            ErrorKind::Invalid,
            "/panels",
            format!("{n} panels do not fit in a {}-pixel plot", bottom - top),
        ));
    }
    // Normalised by the largest weight so the sum cannot overflow (any finite weight > 0
    // is accepted, e.g. 1.7e308); every panel keeps at least MIN_PANEL_H pixels.
    let wmax = weights.iter().copied().fold(0.0_f64, f64::max);
    let weights: Vec<f64> = weights.iter().map(|w| w / wmax).collect();
    let total: f64 = weights.iter().sum();
    let mut rects = Vec::with_capacity(panels.len());
    let mut y = top;
    let mut used = 0;
    for (i, w) in weights.iter().enumerate() {
        let after = i32::try_from(panels.len() - i - 1).unwrap_or(0) * MIN_PANEL_H;
        #[allow(clippy::cast_possible_truncation)]
        let h = if i + 1 == panels.len() {
            avail - used
        } else {
            ((f64::from(avail) * w / total).floor() as i32)
                .max(MIN_PANEL_H)
                .min(avail - used - after)
        };
        rects.push(PixelRect {
            left,
            top: y,
            right,
            bottom: y + h,
        });
        used += h;
        y += h + gap;
    }
    Ok(rects)
}
