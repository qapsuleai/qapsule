//! The `rule` mark: a segment (`x`..`x2`), a horizontal rule (`y`), a vertical rule (`x`)
//! or one vertical rule per fact/finding row.

use super::*;

/// `rule`: its rows by the channels it is given.
pub(super) fn rule(
    cx: &Ctx<'_, '_>,
    st: &mut PrepState,
    opacity: f64,
    x2_given: bool,
) -> Result<Geom, GrammarError> {
    let (fr, enc, theme) = (cx.fr, &cx.layer.encoding, &cx.fr.theme);
    let geom;
    let rule = cx.color_rule(ColorRule::Const(theme.muted))?;
    let mut rows = Vec::new();
    let mk = |slot: u64, v: Option<f64>, row: Option<&Row>| {
        let (color, ckey) = rule.at(slot, row, None);
        PRow {
            slot,
            n: 1,
            t: fr.slot_time(slot),
            v: [v, None, None, None],
            color: with_opacity(color, opacity),
            ckey,
            size: 0.0,
            text: None,
        }
    };
    if x2_given {
        geom = Geom::Segment;
        segment(cx, st, &mk, &mut rows)?;
    } else if let Some((y, key)) = cx.position(enc.y.as_ref(), "/encoding/y", "y")? {
        geom = Geom::RuleH;
        horizontal(cx, st, &mk, &mut rows, &y);
        if !matches!(y, Chan::Const(_)) {
            st.fields.push(key.clone());
        }
        st.keys.push(key);
    } else if let Some((x, key)) = cx.position(enc.x.as_ref(), "/encoding/x", "x")? {
        geom = Geom::RuleV;
        vertical(cx, &mk, &mut rows, &x, &key)?;
        st.keys.push(key);
    } else if matches!(cx.src, Src::Rows) {
        geom = Geom::RuleV;
        for r in cx.visible_rows() {
            rows.push(mk(r.slot, None, Some(r)));
        }
        st.keys.push("x".into());
    } else {
        return Err(cx.err(
            ErrorKind::Invalid,
            "/encoding",
            "`rule` needs `y` (horizontal) or `x` (vertical), or fact/finding data",
        ));
    }
    let base = rule.at(fr.to, None, None).0;
    st.traces.push(Trace {
        rows,
        split: Split::None,
        base,
        lod_line: false,
    });
    Ok(geom)
}

/// A segment from bar `x` to bar `x2` at `y` (sloped to `y2`): evidence geometry (R3).
/// Every value is a constant with an origin.
fn segment(
    cx: &Ctx<'_, '_>,
    st: &mut PrepState,
    mk: &impl Fn(u64, Option<f64>, Option<&Row>) -> PRow,
    rows: &mut Vec<PRow>,
) -> Result<(), GrammarError> {
    let enc = &cx.layer.encoding;
    let a = cx
        .bar_const("/encoding/x", enc.x.as_ref())?
        .ok_or_else(|| {
            cx.err(
                ErrorKind::Invalid,
                "/encoding/x",
                "`x2` needs `x` (the first bar)",
            )
        })?;
    let b = cx.bar_const("/encoding/x2", enc.x2.as_ref())?.unwrap_or(a);
    let konst = |ptr: &str, pos: Option<&Position>, key: &str| match cx.position(pos, ptr, key)? {
        Some((Chan::Const(v), _)) => Ok(Some(v)),
        None => Ok(None),
        Some(_) => Err(cx.err(
            ErrorKind::Invalid,
            ptr,
            "a segment's `y`/`y2` are constants with an origin",
        )),
    };
    let y0 = konst("/encoding/y", enc.y.as_ref(), "y")?
        .ok_or_else(|| cx.err(ErrorKind::Invalid, "/encoding/y", "a segment needs `y`"))?;
    let y1 = konst("/encoding/y2", enc.y2.as_ref(), "y2")?;
    let mut row = mk(a, Some(y0), None);
    row.v[1] = y1;
    rows.push(row);
    st.seg_extent = Some((a, b));
    st.keys.push("y".into());
    if y1.is_some() {
        st.keys.push("y2".into());
    }
    Ok(())
}

/// A horizontal rule at `y`: a constant, the last visible series value or one per row.
fn horizontal(
    cx: &Ctx<'_, '_>,
    st: &mut PrepState,
    mk: &impl Fn(u64, Option<f64>, Option<&Row>) -> PRow,
    rows: &mut Vec<PRow>,
    y: &Chan<'_>,
) {
    let fr = cx.fr;
    match (y, &cx.src) {
        (Chan::Const(v), _) => rows.push(mk(fr.to, Some(*v), None)),
        (_, Src::Slots(_)) => {
            // A rule over series data: one rule at the last visible value.
            if let Some((s, v)) = (fr.from..=fr.to)
                .rev()
                .find_map(|s| y.at(s, None).map(|v| (s, v)))
            {
                rows.push(mk(s, Some(v), None));
            }
        }
        _ => {
            for r in cx.visible_rows() {
                match y.at(r.slot, Some(r)) {
                    Some(v) => rows.push(mk(r.slot, Some(v), Some(r))),
                    None => st.skipped += 1,
                }
            }
        }
    }
}

/// A vertical rule at `x`: a constant or a fact/finding field, by bar or by time.
fn vertical(
    cx: &Ctx<'_, '_>,
    mk: &impl Fn(u64, Option<f64>, Option<&Row>) -> PRow,
    rows: &mut Vec<PRow>,
    x: &Chan<'_>,
    key: &str,
) -> Result<(), GrammarError> {
    let fr = cx.fr;
    let by_bar = matches!(
        cx.constants.get("/encoding/x").map(|c| &c.origin),
        Some(Origin::Fact { field, .. }) if field == "bar"
    ) || key == "bar";
    let to_slot = |v: f64| -> Option<u64> {
        if by_bar {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            (v >= 0.0).then(|| v.round() as u64)
        } else {
            fr.time_slot(v)
        }
    };
    match (x, &cx.src) {
        (Chan::Const(v), _) => {
            if let Some(s) = to_slot(*v).filter(|s| *s >= fr.from && *s <= fr.to) {
                rows.push(mk(s, Some(*v), None));
            }
        }
        (_, Src::Rows) => {
            for r in cx.visible_rows() {
                if let Some(s) = x.at(r.slot, Some(r)).and_then(to_slot) {
                    if s >= fr.from && s <= fr.to {
                        rows.push(mk(s, x.at(r.slot, Some(r)), Some(r)));
                    }
                }
            }
        }
        _ => {
            return Err(cx.err(
                ErrorKind::Invalid,
                "/encoding/x",
                "a vertical rule's `x` is a constant or a fact/finding field",
            ))
        }
    }
    Ok(())
}
