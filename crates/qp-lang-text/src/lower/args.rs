//! Function calls and number fields. A number field takes a literal or a parameter name
//! (`{"param": k}`); anything else cannot be written in the IR (`E_TYPE`). Whether the
//! literal or parameter fits the field (integer, range, kind) is the checker's call.

use qp_schema::protocol::Span;
use serde_json::{json, Map, Value};

use super::state::{Def, Lower};
use crate::ast::{Expr, ExprKind};
use crate::diag::{Diag, Reason};
use crate::msg::Msg;
use crate::names::{did_you_mean, func, turkish, Arg, Func, FUNCTIONS};

/// A float as JSON (`null` if not finite, which the checker rejects).
pub(super) fn num_value(f: f64) -> Value {
    serde_json::Number::from_f64(f).map_or(Value::Null, Value::Number)
}

/// A literal integer field: JSON integer when written without a fraction and in range,
/// else a float.
fn int_lit(text: &str, neg: bool) -> Value {
    if !text.contains('.') {
        if let Ok(i) = text.parse::<i64>() {
            return Value::from(if neg { -i } else { i });
        }
    }
    let f = text.parse::<f64>().unwrap_or(f64::NAN);
    num_value(if neg { -f } else { f })
}

/// An integer field (`ctx` names it in the message).
pub(super) fn int_arg(lw: &mut Lower, e: &Expr, ctx: &str) -> Option<Value> {
    number_arg(lw, e, ctx, true)
}

/// A float field.
pub(super) fn float_arg(lw: &mut Lower, e: &Expr, ctx: &str) -> Option<Value> {
    number_arg(lw, e, ctx, false)
}

fn number_arg(lw: &mut Lower, e: &Expr, ctx: &str, int: bool) -> Option<Value> {
    let wrong = || {
        let msg = if int {
            Msg::IntArg(ctx.to_owned())
        } else {
            Msg::FloatArg(ctx.to_owned())
        };
        Diag::new(Reason::Type, msg, e.span)
    };
    match &e.kind {
        ExprKind::Num { text, neg } if int => Some(int_lit(text, *neg)),
        ExprKind::Num { text, neg } => {
            let f = text.parse::<f64>().unwrap_or(f64::NAN);
            Some(num_value(if *neg { -f } else { f }))
        }
        ExprKind::Name(n) => match lw.names.get(n) {
            Some(Def::Param | Def::Poisoned) => Some(json!({"param": n})),
            Some(Def::Node(_)) => {
                lw.diags.push(wrong().hint(Msg::HintNumberOrParam));
                None
            }
            None => {
                let d = lw.unknown(
                    n,
                    e.span,
                    Reason::UnknownParam,
                    Msg::UnknownParam(n.clone()),
                );
                lw.diags.push(d);
                None
            }
        },
        _ => {
            lw.diags.push(wrong());
            None
        }
    }
}

fn signature(f: &Func) -> String {
    let args: Vec<String> = f
        .args
        .iter()
        .enumerate()
        .map(|(i, (n, _))| {
            if i < f.required {
                (*n).to_owned()
            } else {
                format!("[{n}]")
            }
        })
        .collect();
    format!("{}({})", f.name, args.join(", "))
}

impl Lower {
    /// A function call.
    pub(super) fn call(
        &mut self,
        name: &str,
        name_span: Span,
        args: &[Expr],
        root: Option<&str>,
        s: Span,
    ) -> Option<String> {
        let Some(f) = func(name) else {
            self.unknown_call(name, name_span);
            for a in args {
                if !matches!(a.kind, ExprKind::List(_) | ExprKind::Interval(_)) {
                    self.expr(a, None);
                }
            }
            return None;
        };
        if args.len() < f.required || args.len() > f.args.len() {
            let msg = match f.args.get(args.len()) {
                Some((arg, _)) if args.len() < f.required => Msg::MissingArg { func: f.name, arg },
                _ => Msg::TooMany {
                    func: f.name,
                    max: f.args.len(),
                },
            };
            let reason = if args.len() < f.required {
                Reason::MissingArg
            } else {
                Reason::Parse
            };
            let d = Diag::new(reason, msg, s).hint(Msg::HintSignature(signature(f)));
            self.diags.push(d);
            return None;
        }
        let mut fields = Vec::new();
        let mut ok = true;
        for (a, (field, kind)) in args.iter().zip(f.args) {
            match self.arg(f, a, field, *kind) {
                Some(v) => fields.push((*field, v, Some(a.span))),
                None => ok = false,
            }
        }
        if !ok {
            return None;
        }
        if f.name == "line" {
            fields = line_fields(fields);
        }
        Some(self.node(root, f.name, fields, s))
    }

    fn arg(&mut self, f: &Func, a: &Expr, field: &str, kind: Arg) -> Option<Value> {
        let ctx = format!("`{}`: `{field}`", f.name);
        match kind {
            Arg::Expr => self.expr(a, None).map(Value::String),
            Arg::Int => int_arg(self, a, &ctx),
            Arg::Float => float_arg(self, a, &ctx),
            Arg::Edges => match &a.kind {
                ExprKind::List(items) => {
                    let vals: Vec<Option<Value>> =
                        items.iter().map(|e| float_arg(self, e, &ctx)).collect();
                    vals.into_iter()
                        .collect::<Option<Vec<_>>>()
                        .map(Value::Array)
                }
                _ => {
                    self.diags
                        .push(Diag::new(Reason::Type, Msg::FloatArg(ctx), a.span));
                    None
                }
            },
            Arg::Tf => match &a.kind {
                ExprKind::Interval(tf) => Some(Value::String(tf.clone())),
                _ => {
                    let d = Diag::new(Reason::Type, Msg::IntervalOnly, a.span);
                    self.diags.push(d);
                    None
                }
            },
        }
    }

    fn unknown_call(&mut self, name: &str, span: Span) {
        let mut d = Diag::new(Reason::UnknownOp, Msg::UnknownFunc(name.to_owned()), span)
            .expected(FUNCTIONS)
            .suggest(did_you_mean(name, FUNCTIONS.iter().copied()));
        if turkish(name).is_some() {
            d = d.hint(Msg::HintTurkish);
        }
        self.diags.push(d);
    }
}

/// `line(t1, p1, t2, p2)` → `{"a": {"t", "price"}, "b": {"t", "price"}}`.
fn line_fields(
    fields: Vec<(&'static str, Value, Option<Span>)>,
) -> Vec<(&'static str, Value, Option<Span>)> {
    let mut v = fields.into_iter().map(|(_, v, _)| v);
    let mut point = || {
        let mut m = Map::new();
        m.insert("t".into(), v.next().unwrap_or(Value::Null));
        m.insert("price".into(), v.next().unwrap_or(Value::Null));
        Value::Object(m)
    };
    let a = point();
    let b = point();
    vec![("a", a, None), ("b", b, None)]
}
