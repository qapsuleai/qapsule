//! Literals in canonical text: numbers (integer-valued without `.0`, others in the
//! shortest round-trip form; Rust's `Display` for `f64` never uses an exponent), strings.

use qp_schema::program::{IntArg, ProgramParamKind, ProgramParamValue};

/// A float in canonical form; the sign is written separately so that `-0.0` survives.
pub(super) fn fmt_num(f: f64) -> String {
    if f.is_sign_negative() {
        format!("-{}", -f)
    } else {
        format!("{f}")
    }
}

/// An integer field.
pub(super) fn int_arg(a: &IntArg) -> String {
    match a {
        IntArg::Lit(i) => i.to_string(),
        IntArg::Param(p) => p.param.clone(),
    }
}

/// A parameter value. A float value of a non-`float` parameter keeps its `.0`, so that
/// lowering reads it back as a float (and the checker can answer it).
pub(super) fn param_value(kind: ProgramParamKind, v: ProgramParamValue) -> String {
    match v {
        ProgramParamValue::Int(i) => i.to_string(),
        ProgramParamValue::Bool(b) => b.to_string(),
        ProgramParamValue::Float(f) => {
            let s = fmt_num(f);
            if kind != ProgramParamKind::Float && f.is_finite() && !s.contains('.') {
                format!("{s}.0")
            } else {
                s
            }
        }
    }
}

/// A string in double quotes with `\\`, `\"` and `\n` escaped.
pub(super) fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
