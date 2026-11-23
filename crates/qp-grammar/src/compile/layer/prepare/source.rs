//! The layer's data source (`series`, `fact`, `finding` or none) and its rows; a finding's
//! rows carry the capsule texts.

use super::*;

/// The layer's source, its scene data source and the rows of a fact/finding layer.
pub(super) fn layer_source<'l>(
    fr: &Frame<'_>,
    layer: &'l Layer,
    id: &str,
) -> Result<(Src<'l>, DataSource, Vec<Row>), GrammarError> {
    let mut all_rows = Vec::new();
    let (src, data) = if let Some(s) = &layer.series {
        if fr.kind(s).is_none() {
            return Err(GrammarError::in_layer(
                ErrorKind::NotFound,
                id,
                "/series",
                format!("series `{s}` does not exist"),
            ));
        }
        (
            Src::Slots(s.as_str()),
            DataSource::Series { series: s.clone() },
        )
    } else if let Some(f) = &layer.fact {
        let fact = fr.p.fact(f).ok_or_else(|| {
            GrammarError::in_layer(
                ErrorKind::NotFound,
                id,
                "/fact",
                format!("fact `{f}` does not exist"),
            )
        })?;
        all_rows.push(fr.fact_row(&fact, id, "/fact")?);
        (Src::Rows, DataSource::Fact { fact: f.clone() })
    } else if let Some(f) = &layer.finding {
        finding_rows(fr, f, id, &mut all_rows)?;
        capsule_texts(fr, layer, f, &mut all_rows);
        (Src::Rows, DataSource::Finding { finding: f.clone() })
    } else {
        (Src::Nothing, DataSource::Constant {})
    };
    Ok((src, data, all_rows))
}

/// One row per fact of finding `f`, each with the finding's confidence and claim.
fn finding_rows(
    fr: &Frame<'_>,
    f: &str,
    id: &str,
    all_rows: &mut Vec<Row>,
) -> Result<(), GrammarError> {
    let finding = fr.p.finding(f).ok_or_else(|| {
        GrammarError::in_layer(
            ErrorKind::NotFound,
            id,
            "/finding",
            format!("finding `{f}` does not exist"),
        )
    })?;
    for fid in &finding.facts {
        let fact = fr.p.fact(fid).ok_or_else(|| {
            GrammarError::in_layer(
                ErrorKind::NotFound,
                id,
                "/finding",
                format!("fact `{fid}` of finding `{f}` does not exist"),
            )
        })?;
        let mut row = fr.fact_row(&fact, id, "/finding")?;
        row.nums.insert("confidence".to_owned(), finding.confidence);
        row.texts.insert("claim".to_owned(), finding.claim.clone());
        all_rows.push(row);
    }
    Ok(())
}

/// The capsule texts of finding `f` on every row: voice, headline, ledger, detail and the
/// open design fields the layer reads.
fn capsule_texts(fr: &Frame<'_>, layer: &Layer, f: &str, all_rows: &mut [Row]) {
    // R3 capsule texts (the capsule card): its sentence and its ledger line. (R4/H-FP)
    // In a face scene whose design has a headline template, the template's sentence.
    // (R5/m17-int) Under a voice every capsule scene speaks from it: its headline,
    // its ledger sentence and its detail (`docs/LANGUAGE.md`).
    let voice = fr.p.voice(f);
    let headline = match &voice {
        Some(v) => Some(v.headline.clone()),
        None => crate::designs::headline_text(fr.headline, f, fr.p),
    };
    let ledger = match &voice {
        Some(v) => Some(v.ledger.clone()),
        None => fr.p.capsule(f).and_then(|c| {
            c.story
                .as_ref()
                .map(|s| crate::depth::ledger_text(&s.ledger_at_birth))
        }),
    };
    let detail = voice.map(|v| v.detail);
    // (R5/M20) An open capsule design's text field (`ask/…`, `crate::open`): resolved
    // here from the provider, only for the field the layer reads.
    let asked = layer
        .encoding
        .text
        .as_deref()
        .and_then(|k| (fr.designs.field_text)(k, f, fr.p).map(|t| (k.to_owned(), t)));
    for row in all_rows.iter_mut() {
        if let Some((k, t)) = &asked {
            row.texts.insert(k.clone(), t.clone());
        }
        if let Some(h) = &headline {
            row.texts.insert("headline".to_owned(), h.clone());
        }
        if let Some(l) = &ledger {
            row.texts.insert("ledger".to_owned(), l.clone());
        }
        if let Some(d) = &detail {
            row.texts.insert("detail".to_owned(), d.clone());
        }
    }
    // (R5/M20) An open capsule design's own field (`open.*`, `crate::open::text`).
    if let Some(k) = layer.encoding.text.as_deref() {
        if let Some(t) = (fr.designs.open_text)(k, f, fr.p) {
            for row in all_rows.iter_mut() {
                row.texts.insert(k.to_owned(), t.clone());
            }
        }
    }
}
