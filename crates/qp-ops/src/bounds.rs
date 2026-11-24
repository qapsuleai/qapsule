//! Bounds of a reading request (RF1 S3.2: moved from the authority's `read/query.rs`, used
//! by both the reading subsystem `qp-read` and the authority): a target's ids and bar
//! order, a range's bar indexes.

use qp_schema::protocol::{ErrorCode, Range, Target};
use qp_schema::Document;

use crate::apply::{all_ids, find_collection};
use crate::expand::{need_series, need_view};
use qp_plane::error::{invalid, ptr, PResult, E};

/// Checks target `t` against `doc`: its view, series and item exist; its bars are in order.
///
/// # Errors
/// An unknown id, or `from_bar` after `to_bar`.
pub fn check_target_pub(doc: &Document, t: &Target, path: &str) -> PResult<()> {
    if let Some(v) = &t.view {
        need_view(doc, v, &ptr(path, "view"))?;
    }
    if let Some(s) = &t.series {
        need_series(doc, s, &ptr(path, "series"))?;
    }
    if let Some(i) = &t.item {
        if find_collection(doc, i).is_none() {
            return Err(
                E::new(ErrorCode::NotFound, format!("item `{i}` does not exist"))
                    .at(ptr(path, "item"))
                    .dym(i, all_ids(doc))
                    .build(),
            );
        }
    }
    if let (Some(a), Some(b)) = (t.from_bar, t.to_bar) {
        if a > b {
            return Err(invalid(
                format!("from_bar {a} is after to_bar {b}"),
                ptr(path, "from_bar"),
            ));
        }
    }
    Ok(())
}

/// Resolves a range to `(from, to)` inclusive bar indexes over `t`.
///
/// # Errors
/// No bars, not exactly one range form, or an empty range.
pub fn resolve_range(r: Option<&Range>, t: &[i64], path: &str) -> PResult<(usize, usize)> {
    let n = t.len();
    if n == 0 {
        return Err(invalid("the series has no bars", path));
    }
    let last = n - 1;
    let Some(r) = r else { return Ok((0, last)) };
    let rp = ptr(path, "range");
    let forms = [
        r.last.is_some(),
        r.from_bar.is_some() || r.to_bar.is_some(),
        r.from_t.is_some() || r.to_t.is_some(),
    ];
    if forms.iter().filter(|x| **x).count() != 1 {
        return Err(E::new(ErrorCode::Invalid, "give exactly one range form")
            .at(rp)
            .expected([
                "{\"last\":n}",
                "{\"from_bar\":a,\"to_bar\":b}",
                "{\"from_t\":a,\"to_t\":b}",
            ])
            .build());
    }
    let empty = || {
        E::new(ErrorCode::Invalid, "the range is empty")
            .at(rp.clone())
            .expected([format!("bars 0..={last}")])
            .build()
    };
    if let Some(k) = r.last {
        let k = usize::try_from(k).unwrap_or(usize::MAX).min(n);
        if k == 0 {
            return Err(empty());
        }
        return Ok((n - k, last));
    }
    if forms[1] {
        let a = r
            .from_bar
            .map_or(0, |x| usize::try_from(x).unwrap_or(usize::MAX));
        let b = r
            .to_bar
            .map_or(last, |x| usize::try_from(x).unwrap_or(usize::MAX).min(last));
        if a > b {
            return Err(empty());
        }
        return Ok((a, b));
    }
    let lo = r.from_t.unwrap_or(i64::MIN);
    let hi = r.to_t.unwrap_or(i64::MAX);
    let a = t.iter().position(|x| *x >= lo);
    let b = t.iter().rposition(|x| *x <= hi);
    match (a, b) {
        (Some(a), Some(b)) if a <= b => Ok((a, b)),
        _ => Err(empty()),
    }
}
