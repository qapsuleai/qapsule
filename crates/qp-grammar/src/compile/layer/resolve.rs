//! A layer's named constants resolved against the frame ([`resolve_constants`]).

use super::*;

#[allow(clippy::too_many_lines)]
/// Resolves the layer's origins (`docs/GRAMMAR.md` §10).
pub(super) fn resolve_constants(
    fr: &Frame<'_>,
    layer: &Layer,
    id: &str,
) -> Result<BTreeMap<String, ResolvedConstant>, GrammarError> {
    let mut out = BTreeMap::new();
    let n_thresholds = match &layer.encoding.color {
        Some(ColorSpec::Steps(s)) => s.thresholds.len(),
        _ => 0,
    };
    for (ptr, origin) in &layer.origin {
        let valid = matches!(
            ptr.as_str(),
            "/encoding/x"
                | "/encoding/x2"
                | "/encoding/y"
                | "/encoding/y2"
                | "/encoding/color/when/than"
        ) || ptr
            .strip_prefix("/encoding/color/thresholds/")
            .and_then(|i| i.parse::<usize>().ok())
            .is_some_and(|i| i < n_thresholds);
        if !valid {
            return Err(GrammarError::in_layer(
                ErrorKind::Invalid,
                id,
                format!("/origin/{}", ptr.replace('~', "~0").replace('/', "~1")),
                format!("origin pointer `{ptr}` does not name a numeric constant of the layer"),
            ));
        }
        let nf = |what: String| GrammarError::in_layer(ErrorKind::NotFound, id, ptr.as_str(), what);
        let value = match origin {
            Origin::Fact { fact, field } => {
                let f =
                    fr.p.fact(fact)
                        .ok_or_else(|| nf(format!("origin fact `{fact}` does not exist")))?;
                #[allow(clippy::cast_precision_loss)]
                let v = match field.as_str() {
                    "bar" => Some(f.bar as f64),
                    "t" => Some(f.t as f64),
                    "value" => fr.fact_value(&f, id, ptr)?,
                    k => f
                        .evidence
                        .get(k)
                        .copied()
                        .or_else(|| crate::depth::geometry_field(&f, k)),
                };
                v.ok_or_else(|| {
                    nf(format!(
                        "origin fact `{fact}` has no numeric field `{field}` (fields: bar, t, value{}{})",
                        if f.evidence.is_empty() { "" } else { ", " },
                        f.evidence.keys().cloned().collect::<Vec<_>>().join(", ")
                    ))
                })?
            }
            Origin::Threshold { name } => {
                fr.p.threshold(name)
                    .ok_or_else(|| nf(format!("unknown named threshold `{name}`")))?
            }
            Origin::Series { series, field, .. } => {
                let field = match field {
                    Some(f) => f.clone(),
                    None => fr
                        .main_field(series)
                        .ok_or_else(|| nf(format!("origin series `{series}` does not exist")))?,
                };
                let col = fr.column(series, &field, id, ptr)?;
                let last = match fr.kind(series) {
                    Some(SeriesKind::Points) => fr.total_slots(),
                    _ => fr.n_bars(),
                };
                (0..last).rev().find_map(|s| col.get(s)).ok_or_else(|| {
                    nf(format!(
                        "origin series `{series}` has no value for `{field}`"
                    ))
                })?
            }
        };
        out.insert(
            ptr.clone(),
            ResolvedConstant {
                origin: origin.clone(),
                value,
            },
        );
    }
    Ok(out)
}
