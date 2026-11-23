//! Scale, panel, weight and color checks.

use super::*;

// ---------------------------------------------------------------------------
// Scales and panels
// ---------------------------------------------------------------------------

fn check_domain(d: [f64; 2], log: bool, strict: bool, path: &str) -> PResult<()> {
    let ok =
        d[0].is_finite() && d[1].is_finite() && if strict { d[0] < d[1] } else { d[0] <= d[1] };
    if !ok {
        return Err(E::new(
            ErrorCode::Invalid,
            format!(
                "domain [{}, {}] must be finite with min {} max",
                num(d[0]),
                num(d[1]),
                if strict { "<" } else { "<=" }
            ),
        )
        .at(path)
        .expected([if strict {
            "[min, max] with min < max"
        } else {
            "[min, max] with min <= max"
        }])
        .build());
    }
    if log && d[0] <= 0.0 {
        return Err(E::new(
            ErrorCode::Invalid,
            format!(
                "a log scale needs a positive domain (got min {})",
                num(d[0])
            ),
        )
        .at(path)
        .expected(["min > 0"])
        .build());
    }
    Ok(())
}

/// Checks a panel's value scale (kind, id, domain); fills its id.
pub fn check_value_scale(s: &mut Scale, panel: u32, path: &str) -> PResult<()> {
    let id = format!("y{panel}");
    if s.kind == ScaleKind::Time {
        return Err(E::new(
            ErrorCode::Invalid,
            "a panel's value scale is `linear` or `log`; `time` is only the x axis",
        )
        .at(ptr(path, "kind"))
        .expected(["linear", "log"])
        .build());
    }
    if let Some(given) = &s.id {
        if *given != id {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("the scale of panel {panel} has id `{id}`, not `{given}`"),
            )
            .at(ptr(path, "id"))
            .expected([id])
            .build());
        }
    }
    if let Some(d) = s.domain {
        check_domain(d, s.kind == ScaleKind::Log, true, &ptr(path, "domain"))?;
    }
    s.id = Some(id);
    Ok(())
}

pub fn check_weight(w: f64, path: &str) -> PResult<()> {
    if w.is_finite() && w > 0.0 {
        Ok(())
    } else {
        Err(E::new(
            ErrorCode::Invalid,
            format!("panel weight must be > 0 (got {})", num(w)),
        )
        .at(path)
        .expected(["> 0"])
        .build())
    }
}

/// Checks a panel whose id is assigned; fills its scale id.
pub fn check_panel(p: &mut Panel, path: &str) -> PResult<()> {
    let id = p.id.unwrap_or_default();
    if let Some(w) = p.weight {
        check_weight(w, &ptr(path, "weight"))?;
    }
    check_value_scale(&mut p.y, id, &ptr(path, "y"))
}

pub(super) fn check_x(x: &mut Scale, path: &str) -> PResult<()> {
    if x.kind != ScaleKind::Time {
        return Err(E::new(ErrorCode::Invalid, "the x scale is always `time`")
            .at(ptr(path, "kind"))
            .expected(["time"])
            .build());
    }
    if let Some(given) = &x.id {
        if given != "x" {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("the time axis has id `x`, not `{given}`"),
            )
            .at(ptr(path, "id"))
            .expected(["x"])
            .build());
        }
    }
    if let Some(d) = x.domain {
        check_domain(d, false, false, &ptr(path, "domain"))?;
    }
    x.id = Some("x".to_owned());
    Ok(())
}

// ---------------------------------------------------------------------------
// Colours and style channels
// ---------------------------------------------------------------------------

fn is_hex(c: &str) -> bool {
    let b = c.as_bytes();
    (b.len() == 7 || b.len() == 9) && b[0] == b'#' && b[1..].iter().all(u8::is_ascii_hexdigit)
}

pub fn check_color(c: &str, path: &str) -> PResult<()> {
    if is_hex(c) || THEME_TOKENS.contains(&c) {
        return Ok(());
    }
    Err(E::new(
        ErrorCode::Invalid,
        format!("`{c}` is not a colour: use #rrggbb, #rrggbbaa or a theme name"),
    )
    .at(path)
    .dym(c, THEME_TOKENS)
    .expected(
        ["#rrggbb", "#rrggbbaa"]
            .into_iter()
            .chain(THEME_TOKENS)
            .map(str::to_owned),
    )
    .build())
}

pub(super) fn positive(v: f64, what: &str, path: &str) -> PResult<()> {
    if v.is_finite() && v > 0.0 {
        Ok(())
    } else {
        Err(E::new(
            ErrorCode::Invalid,
            format!("{what} must be > 0 (got {})", num(v)),
        )
        .at(path)
        .expected(["> 0"])
        .build())
    }
}
