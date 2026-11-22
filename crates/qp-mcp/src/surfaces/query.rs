//! (M28 c) The `query` surface: one read tool (`ask`) speaking a small structured query
//! language, and one write tool (`act`). Decisions are not offered at all.
//!
//! # The language
//!
//! ```json
//! {"find": "capsules", "where": {"instrument": "THYAO", "open": true},
//!  "select": ["id", "stage"], "expand": ["evidence"], "limit": 5, "format": "card"}
//! ```
//!
//! - `find` names **what** (eleven nouns: `capsules`, `stories`, `facts`, `instrument`,
//!   `ledger`, `market`, `glossary`, `changes`, `comments`, `watches`, `proposals`), not
//!   **how**: the model never chooses among protocol queries (`capsules` vs `capsule` vs
//!   `list`), the surface does. Singular and common aliases are accepted (`capsule`,
//!   `story`, `log` …) and the answer says the canonical noun.
//! - `where` is one flat object whose keys are the protocol's selector vocabulary
//!   (`docs/PROTOCOL.md` §17: `instrument`, `meaning`, `direction`, `stage`, `open`,
//!   `latest` …) plus a few range keys (`since_bar`, `since_t`, `after_seq`, `within_ms`).
//!   Flat keys rather than a nested expression language: a small model fills a flat form
//!   reliably; there is no OR, no negation, no comparison operator (every key is an
//!   equality or a lower bound, AND-ed) — the protocol's own filters are of that shape, so
//!   nothing is lost. A key that does not apply to the noun is an error naming the keys
//!   that do (`expected`, `did_you_mean`), never silently ignored.
//! - Reference by meaning: `where.capsule` (and `act`'s `capsule`, `cites`, `proposal`)
//!   take an id **or** a protocol selector (`{"instrument":"THYAO","meaning":"breakout",
//!   "latest":true}`); the authority resolves it and the answer carries the ids.
//! - `select` keeps fields, `expand` attaches the related parts of a capsule or story
//!   (`story`, `evidence`, `ledger`, `comments`), `limit` bounds the items, `format`
//!   chooses the shape (M29): `card` (default; a short text card per item + ids), `json`
//!   (the protocol objects as the authority sent them), `layered` (a compact summary per
//!   item; the evidence layer only when expanded). The three formats carry the same
//!   numbers, copied from the authority's reply without rounding or arithmetic.
//!
//! The surface computes nothing: it translates a query into `qp/0` queries on its own
//! connection (so the authority's permission, entitlement and provenance rules hold for
//! every byte), joins them by id, filters by equality, selects fields and formats. The
//! market status, which has no protocol query, is read with the authority's own reader
//! projection (`Session::market_for`).

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::capsule::{Direction, MeaningKind, Stage};
use qp_schema::comment::ExternalQuote;
use qp_schema::protocol::{CapsuleRef, Language, ProposalRef, Range};
use qp_schema::WatchMatch;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::{input_schema, Annotations, Ctx, ToolDef, ToolSurface};
use crate::RpcError;

mod act;
mod args;
mod ask;
mod capsule;
mod check;
mod find;
mod find_records;
mod shape;

use act::*;
pub use args::{ActArgs, AskArgs, Do, Expand, Find, Format, OneOrMany, Where};
use ask::*;
use capsule::*;
use check::*;
use find::*;
use find_records::*;
use shape::*;

/// The query surface.
#[derive(Debug, Default)]
pub struct Query;

/// Registry constructor.
#[must_use]
pub fn new() -> Box<dyn ToolSurface> {
    Box::new(Query)
}

// ---------------------------------------------------------------------------------------
// Errors.

/// A tool-level failure (an `isError` result the model can correct from).
struct Fail(Value);

impl Fail {
    fn schema(path: &str, message: String, expected: &[&str], dym: Vec<String>) -> Self {
        let mut e = json!({"code": "schema", "path": path, "message": message});
        if !expected.is_empty() {
            e["expected"] = json!(expected);
        }
        if !dym.is_empty() {
            e["did_you_mean"] = json!(dym);
        }
        Fail(json!({ "error": e }))
    }

    fn invalid(path: &str, message: String, hint: &str) -> Self {
        Fail(json!({"error": {"code": "invalid", "path": path, "message": message, "hint": hint}}))
    }

    /// A protocol error reply, with the protocol query or op it came from.
    fn protocol(via: &str, reply: &Value) -> Self {
        let e = reply.get("error").cloned().unwrap_or_else(|| reply.clone());
        Fail(json!({"error": e, "via": via}))
    }

    fn result(self) -> Value {
        json!({
            "resultType": "complete",
            "content": [{"type": "text", "text": self.0.to_string()}],
            "structuredContent": self.0,
            "isError": true,
        })
    }
}

// ---------------------------------------------------------------------------------------
// Protocol I/O.

struct Io<'c, 'a> {
    ctx: &'c mut Ctx<'a>,
    seq: Option<u64>,
}

impl Io<'_, '_> {
    fn note_seq(&mut self, r: &Value) {
        if let Some(s) = r.get("seq").and_then(Value::as_u64) {
            self.seq = Some(s);
        }
    }

    /// One `query`; its `data` or the protocol error.
    fn query(&mut self, q: Value) -> Result<Value, Fail> {
        let kind = q
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let r = self.ctx.query(q);
        self.note_seq(&r);
        if r.get("type").and_then(Value::as_str) == Some("result") {
            Ok(r.get("data").cloned().unwrap_or(Value::Null))
        } else {
            Err(Fail::protocol(&format!("query {kind}"), &r))
        }
    }

    /// One command (`mode` optional); the ack or the protocol error.
    fn command(
        &mut self,
        ops: Vec<Value>,
        mode: Option<&str>,
        reason: Option<&str>,
    ) -> Result<Value, Fail> {
        let via = ops
            .first()
            .and_then(|o| o.get("op"))
            .and_then(Value::as_str)
            .unwrap_or("command")
            .to_owned();
        let id = self.ctx.request_id();
        let mut msg = json!({"type": "command", "id": id, "ops": ops});
        if let Some(m) = mode {
            msg["mode"] = json!(m);
        }
        if let Some(r) = reason {
            msg["reason"] = json!(r);
        }
        let r = self.ctx.send(&msg);
        self.note_seq(&r);
        if r.get("type").and_then(Value::as_str) == Some("ack") {
            Ok(r)
        } else {
            Err(Fail::protocol(&format!("op {via}"), &r))
        }
    }

    /// The instrument symbols of the document (series order, unique).
    fn symbols(&mut self) -> Result<Vec<String>, Fail> {
        let d = self.query(json!({"kind": "list", "collection": "series"}))?;
        let mut out: Vec<String> = Vec::new();
        for s in arr(&d, "items") {
            if let Some(sym) = s.pointer("/instrument/symbol").and_then(Value::as_str) {
                if !out.iter().any(|x| x == sym) {
                    out.push(sym.to_owned());
                }
            }
        }
        Ok(out)
    }

    /// `name` as the document spells the symbol (case-insensitive), or a correction.
    fn instrument(&mut self, name: &str, path: &str) -> Result<String, Fail> {
        let syms = self.symbols()?;
        if let Some(s) = syms.iter().find(|s| s.eq_ignore_ascii_case(name)) {
            return Ok(s.clone());
        }
        Err(Fail(json!({"error": {"code": "not_found", "path": path,
            "message": format!("no instrument `{name}` in the document"),
            "expected": syms.clone(),
            "did_you_mean": did_you_mean(name, syms.iter().map(String::as_str)),
            "hint": "Use one of the expected symbols (ask {\"find\":\"instrument\"} lists them)."}})))
    }

    /// A capsule reference resolved to the capsule's detail (`capsule` query).
    fn capsule_detail(&mut self, r: &CapsuleRef) -> Result<Value, Fail> {
        let d = self.query(json!({"kind": "capsule", "capsule": r}))?;
        Ok(d.get("detail").cloned().unwrap_or(Value::Null))
    }

    /// A proposal reference resolved to its id. A selector is resolved here by the §17.3
    /// rule over the `list proposals` answer (document order; `first`/`latest` pick,
    /// several without them is `ambiguous`), since `preview` takes an id.
    fn proposal_id(&mut self, r: &ProposalRef) -> Result<String, Fail> {
        let sel = match r {
            ProposalRef::Id(id) => return Ok(id.clone()),
            ProposalRef::Select(sel) => ser(sel),
        };
        let mut q = json!({"kind": "list", "collection": "proposals"});
        if let Some(st) = sel.get("state") {
            q["state"] = st.clone();
        }
        let d = self.query(q)?;
        let ids: Vec<String> = arr(&d, "items")
            .iter()
            .map(|p| s(p, "id").to_owned())
            .collect();
        let first = sel.get("first") == Some(&Value::Bool(true));
        let latest = sel.get("latest") == Some(&Value::Bool(true));
        match (ids.len(), first, latest) {
            (0, _, _) => Err(Fail(
                json!({"error": {"code": "not_found", "path": "/proposal",
                "message": format!("no proposal matches {sel}"),
                "hint": "ask {\"find\":\"proposals\"} lists them."}}),
            )),
            (_, true, false) => Ok(ids[0].clone()),
            (n, false, true) => Ok(ids[n - 1].clone()),
            (1, false, false) => Ok(ids[0].clone()),
            _ => Err(Fail(
                json!({"error": {"code": "ambiguous", "path": "/proposal",
                "message": format!("{} proposals match {sel}", ids.len()), "candidates": ids,
                "hint": "Add \"first\":true or \"latest\":true, or use a candidate id."}}),
            )),
        }
    }
}

const ASK_DESCRIPTION: &str = "Read the chart document as your person may see it (engine stories, facts, \
track record, changes, comments, watches, proposals). One small query: {\"find\": noun, \"where\": {filters, AND}, \
\"select\": [fields], \"expand\": [parts], \"limit\": n, \"format\": \"card\"|\"json\"|\"layered\"}.\n\
find → where keys:\n\
- capsules (engine stories, most important first): id | capsule (id or selector), instrument, meaning, direction, stage, open, holdings, latest, audience, since_bar, since_t\n\
- stories (one instrument's story list): instrument (required), meaning, direction, stage, open, range, after\n\
- facts: id, capsule, instrument, meaning, direction, kind (exact, e.g. breakout.range.up), since_bar, since_t\n\
- instrument (series, data state, last bar, capsule counts): instrument\n\
- ledger (closed/realized per key): key, meaning, detector, direction, by_kind\n\
- market: instrument · glossary (terms in capsule texts): instrument, capsule\n\
- changes (log): after_seq (an answer's seq), since_t, within_ms, author, since_bar (capsules that moved)\n\
- comments: id, capsule, instrument · watches: id, state, instrument · proposals: id, state\n\
expand (capsules, stories): story, evidence, ledger, comments.\n\
Examples: {\"find\":\"capsules\",\"where\":{\"instrument\":\"THYAO\",\"open\":true}} · \
{\"find\":\"capsules\",\"where\":{\"capsule\":{\"instrument\":\"THYAO\",\"meaning\":\"breakout\",\"latest\":true}},\"expand\":[\"evidence\"]} · \
{\"find\":\"changes\",\"where\":{\"after_seq\":12}}.\n\
Answers carry the ids to cite (capsule, finding, fact); numbers are the engine's, unchanged in every format. \
Errors name the expected keys and did_you_mean.";

const ACT_DESCRIPTION: &str = "Write what an agent may write. Deciding proposals or findings is a person's \
decision and is not offered. {\"do\": ...}:\n\
- comment: capsule (id or selector) or instrument, text, cites [ids], lang (tr default), external [{source, quote, values}]. \
The engine validates it: cited ids must exist, every number must be in the cited evidence or a named external quote, \
no advice. A rejection lists reasons with hints; fix and resend.\n\
- watch: text, match {instrument, holdings, meaning [..], direction, stages [..], negative}, expires_t, max_matches. \
Matches arrive as notifications.\n\
- unwatch: watch (id)\n\
- propose: ops [view/drawing operations, e.g. {\"op\":\"set_view_range\",\"view\":\"main\",\"from_bar\":100,\"to_bar\":150}], reason. \
Becomes a pending proposal a person accepts or rejects.\n\
- preview: proposal (id or selector): what it would change; nothing is written.";

/// The generated schema made compact for the model, without changing what it accepts:
/// a `oneOf` of documented string constants becomes one `enum`, `Option<T>`'s
/// "T or null" becomes T (absent is the same as null for every argument here),
/// `default: null` and integer `format` annotations are dropped.
fn compact(v: Value) -> Value {
    match v {
        Value::Array(a) => Value::Array(a.into_iter().map(compact).collect()),
        Value::Object(mut o) => {
            for key in ["oneOf", "anyOf"] {
                let Some(Value::Array(alts)) = o.get(key).cloned() else {
                    continue;
                };
                // Documented string constants → one enum.
                let mut consts = Vec::new();
                let all_const = alts.iter().all(|a| {
                    let Some(m) = a.as_object() else { return false };
                    if !m
                        .keys()
                        .all(|k| matches!(k.as_str(), "const" | "enum" | "type" | "description"))
                    {
                        return false;
                    }
                    match (m.get("const"), m.get("enum")) {
                        (Some(c @ Value::String(_)), None) => {
                            consts.push(c.clone());
                            true
                        }
                        (None, Some(Value::Array(e))) if e.iter().all(Value::is_string) => {
                            consts.extend(e.iter().cloned());
                            true
                        }
                        _ => false,
                    }
                });
                if all_const && !consts.is_empty() {
                    o.remove(key);
                    o.insert("type".into(), json!("string"));
                    o.insert("enum".into(), Value::Array(consts));
                    continue;
                }
                // T or null → T.
                let non_null: Vec<&Value> = alts
                    .iter()
                    .filter(|a| a.get("type") != Some(&json!("null")))
                    .collect();
                if non_null.len() == 1 && alts.len() == 2 {
                    let inner = non_null[0].clone();
                    o.remove(key);
                    if let Value::Object(im) = inner {
                        for (k, x) in im {
                            o.entry(k).or_insert(x);
                        }
                    }
                }
            }
            if let Some(Value::Array(ts)) = o.get("type").cloned() {
                let non_null: Vec<Value> = ts.into_iter().filter(|t| t != "null").collect();
                if non_null.len() == 1 {
                    o.insert("type".into(), non_null[0].clone());
                }
            }
            if o.get("default") == Some(&Value::Null) {
                o.remove("default");
            }
            if o.get("type") == Some(&json!("integer")) {
                o.remove("format");
            }
            Value::Object(o.into_iter().map(|(k, x)| (k, compact(x))).collect())
        }
        other => other,
    }
}

impl ToolSurface for Query {
    fn name(&self) -> &'static str {
        "query"
    }

    fn tools(&self) -> Vec<ToolDef> {
        vec![
            ToolDef {
                name: "ask".to_owned(),
                description: ASK_DESCRIPTION.to_owned(),
                input_schema: compact(input_schema::<AskArgs>()),
                annotations: Annotations::READ,
                output_schema: None,
            },
            ToolDef {
                name: "act".to_owned(),
                description: ACT_DESCRIPTION.to_owned(),
                input_schema: compact(input_schema::<ActArgs>()),
                annotations: Annotations::WRITE,
                output_schema: None,
            },
        ]
    }

    fn call(&mut self, ctx: &mut Ctx<'_>, name: &str, args: Value) -> Result<Value, RpcError> {
        let r = match name {
            "ask" => ask(ctx, args),
            "act" => act(ctx, &args),
            _ => return Err(RpcError::new(-32602, format!("Unknown tool: {name}"))),
        };
        Ok(r.unwrap_or_else(Fail::result))
    }

    fn instructions(&self, principal: &str) -> String {
        format!(
            "Qapsule chart document (protocol qp/0); you speak as `{principal}` and see what your person may see. \
             The deterministic engine finds facts, capsules (stories) and the track record; you read, explain, \
             watch and propose, you never compute or decide. Read with `ask` (start with \
             {{\"find\":\"capsules\"}} or {{\"find\":\"changes\"}}) and write with `act`. \
             Cite the ids the answers carry and copy numbers exactly; a comment whose numbers are not in the \
             cited evidence is rejected with reasons. Proposals wait for a person's decision. \
             Watch matches arrive as notifications (the `notifications` tool when your client cannot listen)."
        )
    }
}
