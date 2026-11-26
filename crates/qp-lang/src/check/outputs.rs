//! Outputs (PR §5, §6): names, kinds, node types, the base-clock rule, the signal's side,
//! outcome rule and source, the state's dimensions; `E_TRY_ONLY` for `add_program`.

use qp_schema::program::{ProgramOutputInfo, ProgramOutputKind, Side, SignalSource};
use serde_json::Value;

use super::args::{int_arg, num_value, resolve_ref, IntRole, NumRole, MAX_DIMS};
use super::fields::Fields;
use super::{Cx, Purpose};
use crate::checked::{CheckedOutput, Clock, OutputSpec, Rule, Slot, Ty};
use crate::diag::join_i;
use crate::limits::Limit;
use crate::message::{Msg, Space};
use crate::names::{is_lower_name, suggest, OUTPUT_KINDS};
use crate::reason::Reason;

/// What the outputs section gives.
#[derive(Default)]
pub(super) struct Outputs {
    /// More outputs than `program.max_outputs`: none was read.
    pub(super) over: bool,
    /// The outputs whose name and kind parsed.
    pub(super) infos: Vec<ProgramOutputInfo>,
    /// Resolved outputs (`None` for one with an error).
    pub(super) specs: Vec<Option<CheckedOutput>>,
}

/// Reads the `outputs` section.
pub(super) fn outputs(cx: &mut Cx<'_>, raw: Option<&Value>) -> Outputs {
    let mut out = Outputs::default();
    let items = match raw {
        None => {
            cx.d.at(Reason::NoOutput, "", &Msg::NoOutput)
                .expected(["outputs"]);
            return out;
        }
        Some(Value::Array(items)) if items.is_empty() => {
            cx.d.at(Reason::NoOutput, "/outputs", &Msg::NoOutput);
            return out;
        }
        Some(Value::Array(items)) => items,
        Some(_) => {
            cx.d.at(
                Reason::Type,
                "/outputs",
                &Msg::NotArray("outputs".to_owned()),
            );
            return out;
        }
    };
    let max = cx.limit(Limit::MaxOutputs);
    if items.len() as u64 > max {
        cx.d.at(
            Reason::TooLarge,
            "/outputs",
            &Msg::TooMany(Space::Output, items.len() as u64, max),
        )
        .expected([max]);
        out.over = true;
        return out;
    }
    let mut names: Vec<String> = Vec::new();
    for (i, v) in items.iter().enumerate() {
        out.specs.push(output(cx, v, i, &mut names, &mut out.infos));
    }
    out
}

fn output(
    cx: &mut Cx<'_>,
    v: &Value,
    i: usize,
    names: &mut Vec<String>,
    infos: &mut Vec<ProgramOutputInfo>,
) -> Option<CheckedOutput> {
    let at = join_i("/outputs", i);
    let mut f = Fields::new(v, &at, "outputs[]", &mut cx.d)?;
    let before = cx.d.count();
    let name = f.req_str("name", &mut cx.d).map(str::to_owned);
    if let Some(n) = &name {
        check_name(cx, n, &f.ptr("name"), Space::Output, names);
    }
    let kind = match f.req_str("kind", &mut cx.d) {
        Some(k) => kind_of(cx, k, &f.ptr("kind")),
        None => None,
    };
    if let (Some(n), Some(k)) = (&name, kind) {
        let documentable = matches!(k, ProgramOutputKind::Series | ProgramOutputKind::Event);
        infos.push(ProgramOutputInfo {
            name: n.clone(),
            kind: k,
            documentable,
        });
        if !documentable && cx.ctx.purpose == Purpose::Document {
            cx.d.at(Reason::TryOnly, &f.ptr("kind"), &Msg::TryOnly(n.clone()))
                .hint(&Msg::HintTryOnly);
        }
    }
    let label = name.clone().unwrap_or_default();
    let spec = match kind {
        Some(ProgramOutputKind::Series) => {
            out_node(cx, &mut f, "node", Ty::Num, &label).map(OutputSpec::Series)
        }
        Some(ProgramOutputKind::Event) => {
            out_node(cx, &mut f, "node", Ty::Bool, &label).map(OutputSpec::Event)
        }
        Some(ProgramOutputKind::Signal) => signal(cx, &mut f, &label),
        Some(ProgramOutputKind::State) => state(cx, &mut f, &label),
        None => {
            f.finish(&mut cx.d, &["node", "side", "rule", "source", "dims"]);
            return None;
        }
    };
    f.finish(&mut cx.d, &[]);
    let spec = spec?;
    (cx.d.count() == before).then_some(CheckedOutput { name: label, spec })
}

fn check_name(cx: &mut Cx<'_>, n: &str, at: &str, space: Space, seen: &mut Vec<String>) {
    if !is_lower_name(n) {
        cx.d.at(Reason::BadParam, at, &Msg::BadName(n.to_owned()))
            .expected(["^[a-z][a-z0-9_]{0,31}$"]);
    } else if seen.iter().any(|s| s == n) {
        cx.d.at(
            Reason::DuplicateName,
            at,
            &Msg::Duplicate(space, n.to_owned()),
        );
    } else {
        seen.push(n.to_owned());
    }
}

fn kind_of(cx: &mut Cx<'_>, k: &str, at: &str) -> Option<ProgramOutputKind> {
    Some(match k {
        "series" => ProgramOutputKind::Series,
        "event" => ProgramOutputKind::Event,
        "signal" => ProgramOutputKind::Signal,
        "state" => ProgramOutputKind::State,
        _ => {
            cx.d.at(Reason::UnknownOp, at, &Msg::UnknownOutputKind(k.to_owned()))
                .expected(OUTPUT_KINDS)
                .dym(suggest(k, &OUTPUT_KINDS));
            return None;
        }
    })
}

/// An output's node field: a reference of type `want` on the base clock.
fn out_node(
    cx: &mut Cx<'_>,
    f: &mut Fields<'_>,
    field: &'static str,
    want: Ty,
    out: &str,
) -> Option<Slot> {
    let at = f.ptr(field);
    let s = match f.req(field, &mut cx.d)? {
        Value::String(s) => s,
        _ => {
            cx.d.at(Reason::Type, &at, &Msg::NotString(field.to_owned()));
            return None;
        }
    };
    let here = cx.nodes.len();
    let slot = resolve_ref(cx, s, &at, here)?;
    let typed = cx.nodes[slot.node].typed.as_ref()?;
    let (ty, clock) = (typed.outputs[slot.out], typed.clock.clone());
    if ty != want {
        let m = Msg::TypeMismatch(field.to_owned(), want.name(), ty.name());
        cx.d.at(Reason::Type, &at, &m).expected([want.name()]);
        return None;
    }
    if let Clock::Tf(tf) = &clock {
        cx.d.at(
            Reason::OutputClock,
            &at,
            &Msg::OutputClock(out.to_owned(), tf.name.clone()),
        )
        .expected(["base"]);
        return None;
    }
    Some(slot)
}

fn signal(cx: &mut Cx<'_>, f: &mut Fields<'_>, out: &str) -> Option<OutputSpec> {
    let slot = out_node(cx, f, "node", Ty::Bool, out);
    let side = match f.req_str("side", &mut cx.d) {
        Some("al") => Some(Side::Al),
        Some("sat") => Some(Side::Sat),
        Some(other) => {
            cx.d.at(
                Reason::Unsupported,
                &f.ptr("side"),
                &Msg::Side(other.to_owned()),
            )
            .expected(["al", "sat"])
            .hint(&Msg::HintSide);
            None
        }
        None => None,
    };
    let rule = match f.opt("rule") {
        Some(v) => rule(cx, v, &f.ptr("rule")),
        None => {
            cx.d.at(Reason::NoOutcomeRule, &f.at, &Msg::NoRule)
                .expected(["rule"]);
            None
        }
    };
    let source = source(cx, f);
    Some(OutputSpec::Signal {
        slot: slot?,
        side: side?,
        rule: rule?,
        source: source?,
    })
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn rule(cx: &mut Cx<'_>, v: &Value, at: &str) -> Option<Rule> {
    let mut f = Fields::new(v, at, "rule", &mut cx.d)?;
    if f.has("kind") {
        let k = f
            .opt("kind")
            .map(|k| k.as_str().unwrap_or("?").to_owned())
            .unwrap_or_default();
        cx.d.at(Reason::Unsupported, &f.ptr("kind"), &Msg::RuleKind(k))
            .hint(&Msg::HintRuleKind);
    }
    let pct =
        |cx: &mut Cx<'_>, f: &mut Fields<'_>, name: &'static str| -> Result<Option<f64>, ()> {
            let at = f.ptr(name);
            match f.opt(name) {
                None => Ok(None),
                Some(v) => num_value(cx, v, &at, name, NumRole::Pct)
                    .map(Some)
                    .ok_or(()),
            }
        };
    let target_pct = pct(cx, &mut f, "target_pct");
    let stop_pct = pct(cx, &mut f, "stop_pct");
    let max_bars = int_arg(cx, &mut f, "max_bars", IntRole::MaxBars);
    f.finish(&mut cx.d, &["kind"]);
    Some(Rule {
        target_pct: target_pct.ok()?,
        stop_pct: stop_pct.ok()?,
        max_bars: max_bars? as usize,
    })
}

fn source(cx: &mut Cx<'_>, f: &mut Fields<'_>) -> Option<SignalSource> {
    let at = f.ptr("source");
    let v = f.req("source", &mut cx.d)?;
    let mut s = Fields::new(v, &at, "source", &mut cx.d)?;
    let strategy = s.req_str("strategy", &mut cx.d).map(str::to_owned);
    let version = s.req_str("version", &mut cx.d).map(str::to_owned);
    s.finish(&mut cx.d, &[]);
    Some(SignalSource {
        strategy: strategy?,
        version: version?,
    })
}

fn state(cx: &mut Cx<'_>, f: &mut Fields<'_>, out: &str) -> Option<OutputSpec> {
    let at = f.ptr("dims");
    let v = f.req("dims", &mut cx.d)?;
    let Value::Array(items) = v else {
        cx.d.at(Reason::Type, &at, &Msg::NotArray("dims".to_owned()));
        return None;
    };
    if items.is_empty() || items.len() > MAX_DIMS {
        cx.d.at(Reason::BadParam, &at, &Msg::DimsCount(items.len()))
            .expected([format!("1..={MAX_DIMS}")]);
        return None;
    }
    let mut seen = Vec::new();
    let mut dims = Vec::new();
    for (k, item) in items.iter().enumerate() {
        let dat = join_i(&at, k);
        let Some(mut d) = Fields::new(item, &dat, "dims[]", &mut cx.d) else {
            dims.push(None);
            continue;
        };
        let name = d.req_str("name", &mut cx.d).map(str::to_owned);
        if let Some(n) = &name {
            check_name(cx, n, &d.ptr("name"), Space::Dim, &mut seen);
        }
        let slot = out_node(cx, &mut d, "node", Ty::Cat, out);
        d.finish(&mut cx.d, &[]);
        dims.push(name.zip(slot));
    }
    Some(OutputSpec::State(dims.into_iter().collect::<Option<_>>()?))
}
