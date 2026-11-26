//! (RL1, PS §3.3, K86/6) The thin surface's program tools: `program_check`, `program_try`,
//! `program_explain` take a program as text or IR and forward it as the `check_program`,
//! `try_program`, `explain_program` query; `program_add` lowers a text to IR
//! (`qp-lang-text`, the registry's `program.text.*` limits) and sends one `add_program`
//! command. An MCP connection is agent-originated, so the authority turns the command
//! into a proposal; a person decides it (no decision tool here, §25.2). Nothing else is
//! added: the generated `query_*_program` and `op_add_program` tools stay as they are.

use std::collections::BTreeMap;

use qp_lang_text::{lower_with, TextLimits};
use qp_schema::program::{ExplainAt, ProgramParamValue};
use qp_schema::protocol::{Language, Range, Show, WriteMode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::surfaces::{input_schema, Annotations, Ctx, ToolDef};
use crate::{tool_result, RpcError};

/// The program tools, in list order.
pub const PROGRAM_TOOLS: [&str; 4] = [
    "program_check",
    "program_try",
    "program_explain",
    "program_add",
];

/// A program: text (a string) or IR (a JSON object, the `Program` schema).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ProgramInput {
    /// Program text (docs/PROGRAMS.md §12).
    Text(String),
    /// Program IR (`qp-ir/0`).
    Ir(Map<String, Value>),
}

/// `program_check` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckArgs {
    /// The program: text (string) or IR (object).
    pub program: ProgramInput,
    /// The bar series to bind (one); without it clock and budget are checked unbound.
    #[serde(default)]
    pub inputs: Vec<String>,
    /// Parameter values to bind.
    #[serde(default)]
    pub params: BTreeMap<String, ProgramParamValue>,
    /// Message language (`en` default, `tr`).
    #[serde(default)]
    pub lang: Option<Language>,
}

/// `program_try` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TryArgs {
    /// The program: text (string) or IR (object). Or give `instance`.
    #[serde(default)]
    pub program: Option<ProgramInput>,
    /// A program instance of the document, instead of `program`.
    #[serde(default)]
    pub instance: Option<String>,
    /// The bar series to run on (one).
    #[serde(default)]
    pub inputs: Vec<String>,
    /// Parameter values to bind.
    #[serde(default)]
    pub params: BTreeMap<String, ProgramParamValue>,
    /// Bars to read (default: the latest `program.try.max_bars`).
    #[serde(default)]
    pub range: Option<Range>,
    /// Message language (`en` default, `tr`).
    #[serde(default)]
    pub lang: Option<Language>,
}

/// `program_explain` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExplainArgs {
    /// The program: text (string) or IR (object). Or give `instance`.
    #[serde(default)]
    pub program: Option<ProgramInput>,
    /// A program instance of the document, instead of `program`.
    #[serde(default)]
    pub instance: Option<String>,
    /// The bar series (needed with `at`).
    #[serde(default)]
    pub inputs: Vec<String>,
    /// Parameter values to bind.
    #[serde(default)]
    pub params: BTreeMap<String, ProgramParamValue>,
    /// Trace one value: output name and bar time.
    #[serde(default)]
    pub at: Option<ExplainAt>,
    /// Message language (`en` default, `tr`).
    #[serde(default)]
    pub lang: Option<Language>,
}

/// `program_add` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddArgs {
    /// Instance name (new); assigned if absent.
    #[serde(default)]
    pub id: Option<String>,
    /// The program: text (string, lowered to IR here) or IR (object).
    pub program: ProgramInput,
    /// Exactly one bar series.
    pub inputs: Vec<String>,
    /// Parameter values to bind; missing ones take their defaults.
    #[serde(default)]
    pub params: BTreeMap<String, ProgramParamValue>,
    /// Draw the outputs in a view.
    #[serde(default)]
    pub show: Option<Show>,
    /// Command mode (`auto` default; an agent's add is a proposal, `direct` is refused).
    #[serde(default)]
    pub mode: Option<WriteMode>,
    /// Why (shown with the proposal).
    #[serde(default)]
    pub reason: Option<String>,
    /// Language of text errors (`en` default, `tr`).
    #[serde(default)]
    pub lang: Option<Language>,
}

/// The program tools' definitions.
#[must_use]
pub fn tools() -> Vec<ToolDef> {
    let def = |name: &str, description: &str, input_schema, annotations| ToolDef {
        name: name.to_owned(),
        description: description.to_owned(),
        input_schema,
        annotations,
        output_schema: Some(crate::tools::output_schema()),
    };
    vec![
        def(
            "program_check",
            "Checks a program (text or qp-ir/0 IR): every error at once with `at`/`span`, \
             `did_you_mean`, `hint`; warnings, cost, outputs; the IR of a text. Program \
             errors are in the answer (`ok: false`), not a protocol error. Same as \
             query_check_program.",
            input_schema::<CheckArgs>(),
            Annotations::READ,
        ),
        def(
            "program_try",
            "Tries a program (text, IR or `instance`) over a bar series' closed bars, replay \
             mode: per output counts and items, simulated signal records, work used; stops \
             coded on the work budget. Writes nothing. Same as query_try_program.",
            input_schema::<TryArgs>(),
            Annotations::READ,
        ),
        def(
            "program_explain",
            "Explains a program (text, IR or `instance`): canonical text, program_id, per \
             node cost; with `at` a value's provenance. Same as query_explain_program.",
            input_schema::<ExplainArgs>(),
            Annotations::READ,
        ),
        def(
            "program_add",
            "Adds a program instance (series and event outputs) on one bar series: text is \
             lowered to IR, then one `add_program` command. From an agent it becomes a \
             proposal a person decides; check and try it first.",
            input_schema::<AddArgs>(),
            Annotations::WRITE,
        ),
    ]
}

fn parse<T: DeserializeOwned>(name: &str, args: Value) -> Result<T, RpcError> {
    serde_json::from_value(args).map_err(|e| RpcError::new(-32602, format!("tool `{name}`: {e}")))
}

/// Puts `program` as `source` (text) or `program` (IR) into a query or op object.
fn put_program(o: &mut Map<String, Value>, program: Option<ProgramInput>, text_key: &str) {
    match program {
        Some(ProgramInput::Text(t)) => {
            o.insert(text_key.to_owned(), json!(t));
        }
        Some(ProgramInput::Ir(m)) => {
            o.insert("program".to_owned(), Value::Object(m));
        }
        None => {}
    }
}

/// Inserts `v` under `k` unless it serializes to nothing (null, empty list or map).
fn put(o: &mut Map<String, Value>, k: &str, v: impl Serialize) {
    let v = serde_json::to_value(v).unwrap_or(Value::Null);
    let empty = match &v {
        Value::Null => true,
        Value::Array(a) => a.is_empty(),
        Value::Object(m) => m.is_empty(),
        _ => false,
    };
    if !empty {
        o.insert(k.to_owned(), v);
    }
}

/// Calls a program tool; `None` when `name` is not one.
pub fn call(ctx: &mut Ctx<'_>, name: &str, args: Value) -> Option<Result<Value, RpcError>> {
    let r = match name {
        "program_check" => parse::<CheckArgs>(name, args).map(|a| {
            let mut q = Map::new();
            q.insert("kind".into(), json!("check_program"));
            put_program(&mut q, Some(a.program), "source");
            put(&mut q, "inputs", a.inputs);
            put(&mut q, "params", a.params);
            put(&mut q, "lang", a.lang);
            tool_result(ctx.query(Value::Object(q)))
        }),
        "program_try" => parse::<TryArgs>(name, args).map(|a| {
            let mut q = Map::new();
            q.insert("kind".into(), json!("try_program"));
            put_program(&mut q, a.program, "source");
            put(&mut q, "instance", a.instance);
            put(&mut q, "inputs", a.inputs);
            put(&mut q, "params", a.params);
            put(&mut q, "range", a.range);
            put(&mut q, "lang", a.lang);
            tool_result(ctx.query(Value::Object(q)))
        }),
        "program_explain" => parse::<ExplainArgs>(name, args).map(|a| {
            let mut q = Map::new();
            q.insert("kind".into(), json!("explain_program"));
            put_program(&mut q, a.program, "source");
            put(&mut q, "instance", a.instance);
            put(&mut q, "inputs", a.inputs);
            put(&mut q, "params", a.params);
            put(&mut q, "at", a.at);
            put(&mut q, "lang", a.lang);
            tool_result(ctx.query(Value::Object(q)))
        }),
        "program_add" => parse::<AddArgs>(name, args).map(|a| add(ctx, a)),
        _ => return None,
    };
    Some(r)
}

/// The registry's text limits (`program.text.*`; the host checks the IR with its own).
fn text_limits() -> TextLimits {
    let n = |key: &str, d: usize| {
        qp_schema::params::by_key(key)
            .and_then(|p| match p.default {
                qp_schema::params::ParamValue::Int(v) => usize::try_from(v).ok(),
                _ => None,
            })
            .unwrap_or(d)
    };
    TextLimits {
        max_source_chars: n(
            "program.text.max_source_chars",
            qp_lang_text::DEFAULT_MAX_SOURCE_CHARS,
        ),
        max_token_chars: n(
            "program.text.max_token_chars",
            qp_lang_text::DEFAULT_MAX_TOKEN_CHARS,
        ),
        max_depth: n("program.text.max_depth", qp_lang_text::DEFAULT_MAX_DEPTH),
    }
}

/// `program_add`: text → IR, then one `add_program` command (a proposal from an agent).
fn add(ctx: &mut Ctx<'_>, a: AddArgs) -> Value {
    let ir = match a.program {
        ProgramInput::Ir(m) => Value::Object(m),
        ProgramInput::Text(src) => {
            match lower_with(&src, a.lang.unwrap_or(Language::En), &text_limits()) {
                Ok(l) => l.ir,
                Err(errors) => return lowering_error(ctx, &errors),
            }
        }
    };
    let mut op = Map::new();
    op.insert("op".into(), json!("add_program"));
    put(&mut op, "id", a.id);
    op.insert("program".into(), ir);
    op.insert("inputs".into(), json!(a.inputs));
    put(&mut op, "params", a.params);
    put(&mut op, "show", a.show);
    let mut cmd = Map::new();
    cmd.insert("type".into(), json!("command"));
    cmd.insert("id".into(), json!(ctx.request_id()));
    put(&mut cmd, "mode", a.mode);
    put(&mut cmd, "reason", a.reason);
    cmd.insert("ops".into(), json!([op]));
    tool_result(ctx.send(&Value::Object(cmd)))
}

/// The text did not lower: nothing is sent; the reply is shaped as the protocol's
/// all-at-once program error (PS §3.1: the first error's fields, every error in `errors`).
fn lowering_error(ctx: &mut Ctx<'_>, errors: &[qp_schema::protocol::ProgramError]) -> Value {
    let first = errors.first();
    let mut e = json!({
        "code": first.and_then(|f| f.code).map_or_else(|| json!("invalid"), |c| json!(c)),
        "message": first.map_or("the program text does not lower", |f| f.message.as_str()),
        "path": "/program",
        "errors": errors,
    });
    if let Some(f) = first {
        e["reason"] = json!(f.reason);
        if !f.did_you_mean.is_empty() {
            e["did_you_mean"] = json!(f.did_you_mean);
        }
        if let Some(h) = &f.hint {
            e["hint"] = json!(h);
        }
    }
    tool_result(json!({"type": "error", "id": ctx.request_id(), "error": e}))
}
