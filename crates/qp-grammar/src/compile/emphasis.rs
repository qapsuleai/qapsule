//! Emphasis targets resolved against the view (`docs/GRAMMAR.md` §8).

use super::*;

/// Resolves emphasis targets (`docs/GRAMMAR.md` §8).
pub(super) fn resolve_emphasis(
    view: &View,
    list: &[(String, &Emphasis)],
    prepared: &[Prepared],
    panel_ids: &[u32],
    p: &dyn DataProvider,
) -> Result<Vec<Resolved>, GrammarError> {
    let mut out = Vec::new();
    for (path, e) in list {
        let t = &e.target;
        if t.view.as_deref().is_some_and(|v| v != view.id) {
            continue;
        }
        let nf = |what: &str, why: String| {
            GrammarError::new(ErrorKind::NotFound, format!("{path}/target/{what}"), why)
        };
        let mut layers: Vec<&Prepared> = Vec::new();
        if let Some(l) = &t.layer {
            layers.push(
                prepared
                    .iter()
                    .find(|x| &x.id == l)
                    .ok_or_else(|| nf("layer", format!("layer `{l}` does not exist")))?,
            );
        } else if let Some(s) = &t.series {
            layers.extend(prepared.iter().filter(|x| {
                matches!(&x.data, qp_schema::scene::DataSource::Series { series } if series == s)
            }));
            if layers.is_empty() {
                return Err(nf("series", format!("no layer draws series `{s}`")));
            }
        }
        let mut bars = match (t.from_bar, t.to_bar) {
            (Some(a), Some(b)) => Some((a.min(b), a.max(b))),
            (Some(a), None) | (None, Some(a)) => Some((a, a)),
            (None, None) => None,
        };
        if let Some(item) = &t.item {
            let fact_bars: Vec<u64> = if let Some(f) = p.fact(item) {
                vec![f.bar]
            } else if let Some(f) = p.finding(item) {
                f.facts
                    .iter()
                    .filter_map(|x| p.fact(x))
                    .map(|x| x.bar)
                    .collect()
            } else {
                return Err(nf(
                    "item",
                    format!("item `{item}` is not a fact or finding"),
                ));
            };
            if let (Some(a), Some(b)) = (fact_bars.iter().min(), fact_bars.iter().max()) {
                bars = Some((*a, *b));
            }
        }
        let panels = match t.panel {
            Some(pid) => {
                if !panel_ids.contains(&pid) {
                    return Err(nf("panel", format!("panel {pid} does not exist")));
                }
                vec![pid]
            }
            None => {
                let mut ps: Vec<u32> = layers.iter().map(|l| l.panel).collect();
                ps.dedup();
                ps
            }
        };
        if layers.is_empty() {
            out.push(Resolved {
                kind: e.kind,
                panels,
                layer: None,
                bars,
            });
        } else {
            for l in layers {
                out.push(Resolved {
                    kind: e.kind,
                    panels: if t.panel.is_some() {
                        panels.clone()
                    } else {
                        vec![l.panel]
                    },
                    layer: Some(l.id.clone()),
                    bars,
                });
            }
        }
    }
    Ok(out)
}
