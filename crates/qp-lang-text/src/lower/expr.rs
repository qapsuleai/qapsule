//! Expressions to nodes. Children are lowered before their parent, so unnamed nodes are
//! numbered in post-order (PR §12.3). A bare name or `name.output` makes no node: it is
//! a reference (a binding to it is an alias).

use qp_schema::protocol::Span;
use serde_json::{json, Value};

use super::args::num_value;
use super::state::{Def, Lower};
use crate::ast::{Expr, ExprKind};
use crate::diag::{Diag, Reason};
use crate::msg::Msg;
use crate::names::{did_you_mean, turkish, FIELDS};

impl Lower {
    /// Lowers `e`; the node it creates at its top (if any) is named `root`. Returns the
    /// reference to its value, or `None` after recording an error.
    pub(super) fn expr(&mut self, e: &Expr, root: Option<&str>) -> Option<String> {
        let s = e.span;
        match &e.kind {
            ExprKind::Num { text, neg } => {
                let f = text.parse::<f64>().unwrap_or(f64::NAN);
                let v = num_value(if *neg { -f } else { f });
                Some(self.node(root, "const", vec![("value", v, None)], s))
            }
            ExprKind::Bool(b) => {
                Some(self.node(root, "const", vec![("value", Value::Bool(*b), None)], s))
            }
            ExprKind::Name(n) => self.name(n, root, s),
            ExprKind::Field { field, tf } => {
                let mut f = vec![("field", Value::String(field.clone()), None)];
                if let Some(tf) = tf {
                    f.push(("tf", Value::String(tf.clone()), None));
                }
                Some(self.node(root, "input", f, s))
            }
            ExprKind::Builtin(op) => Some(self.node(root, op, vec![], s)),
            ExprKind::Call {
                name,
                name_span,
                args,
            } => self.call(name, *name_span, args, root, s),
            ExprKind::Neg(x) => self.unary("neg", "x", x, root, s),
            ExprKind::Not(x) => self.unary("not", "x", x, root, s),
            ExprKind::Bin { op, a, b } => {
                let ra = self.expr(a, None);
                let rb = self.expr(b, None);
                let f = vec![
                    ("a", Value::String(ra?), Some(a.span)),
                    ("b", Value::String(rb?), Some(b.span)),
                ];
                Some(self.node(root, op.op(), f, s))
            }
            ExprKind::Lag { x, n } => {
                let rx = self.expr(x, None);
                let n_v = super::args::int_arg(self, n, "`x[n]`");
                let f = vec![
                    ("x", Value::String(rx?), Some(x.span)),
                    ("n", n_v?, Some(n.span)),
                ];
                Some(self.node(root, "lag", f, s))
            }
            ExprKind::Select { x, out } => self.expr(x, root).map(|r| format!("{r}.{out}")),
            ExprKind::List(_) => {
                self.diags.push(Diag::new(Reason::Type, Msg::ListOnly, s));
                None
            }
            ExprKind::Interval(_) => {
                self.diags
                    .push(Diag::new(Reason::Type, Msg::IntervalOnly, s));
                None
            }
        }
    }

    fn unary(
        &mut self,
        op: &str,
        field: &'static str,
        x: &Expr,
        root: Option<&str>,
        s: Span,
    ) -> Option<String> {
        let r = self.expr(x, None)?;
        Some(self.node(root, op, vec![(field, Value::String(r), Some(x.span))], s))
    }

    /// A name in an expression: a binding's reference, or a `const` reading a parameter.
    fn name(&mut self, n: &str, root: Option<&str>, s: Span) -> Option<String> {
        match self.names.get(n).cloned() {
            Some(Def::Node(r)) => Some(r),
            Some(Def::Param) => {
                let v = json!({"param": n});
                Some(self.node(root, "const", vec![("value", v, None)], s))
            }
            Some(Def::Poisoned) => None,
            None => {
                let d = self.unknown(n, s, Reason::UnknownRef, Msg::UnknownName(n.to_owned()));
                self.diags.push(d);
                None
            }
        }
    }

    /// The error of an unknown name, with suggestions from the known names, bar fields
    /// and builtins, and a hint when the name is defined later or is Turkish.
    pub(super) fn unknown(&self, n: &str, s: Span, reason: Reason, msg: Msg) -> Diag {
        let params_only = reason == Reason::UnknownParam;
        let mut cands: Vec<&str> = self
            .names
            .iter()
            .filter(|(_, d)| !params_only || **d == Def::Param)
            .map(|(k, _)| k.as_str())
            .collect();
        if !params_only {
            cands.extend(FIELDS.iter().copied());
            cands.extend(["bar_index", "time", "true", "false"]);
        }
        let mut d = Diag::new(reason, msg, s).suggest(did_you_mean(n, cands));
        if let Some(line) = self.lines.get(n).filter(|l| **l >= s.line) {
            d = d.hint(Msg::HintLater(*line));
        } else if turkish(n).is_some() {
            d = d.hint(Msg::HintTurkish);
        }
        d
    }
}
