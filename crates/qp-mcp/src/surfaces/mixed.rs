//! (M28 d) The mixed surface: a few task-level tools for what an agent does most —
//! `overview` (what changed, what matters to the person first), `find` (capsules by
//! meaning), `capsule` (one capsule explained: story, evidence, ledger, comments),
//! `watch` (watches and capsule follows), `write` (a comment or a view proposal) — and
//! one escape tool, `query`, that runs any `qp/0` query as is.
//!
//! Every tool becomes `qp/0` messages on the endpoint's own connection ([`Ctx`]); the
//! surface computes nothing: it picks fields of the authority's replies and lays them
//! out. Numbers are written exactly as the protocol gives them (the JSON number text), so
//! the three capsule formats (`card`, `json`, `layered`, M29) carry the same numbers.
//! No decision tool exists here: accepting or rejecting proposals and findings is a
//! person's (§25.2); `write` refuses a decision op before sending anything, and the
//! authority refuses it on any path anyway. Input schemas come from the `qp-schema`
//! types (`schemars`); the proposal ops are the protocol's `Op` schema narrowed to the
//! agent's proposal-only ops (§25.3), the escape tool's is the protocol's `Query`.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use qp_schema::capsule::{MeaningKind, Stage};
use qp_schema::comment::ExternalQuote;
use qp_schema::protocol::{CapsuleRef, CommentAbout, Language, Op, Query};
use qp_schema::WatchMatch;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::{input_schema, Annotations, Ctx, ToolDef, ToolSurface};
use crate::{tool_result, RpcError};

/// Ops an agent may send only as a proposal (`docs/PROTOCOL.md` §25.3); the `write`
/// tool's `ops` schema is the protocol `Op` schema narrowed to these.
pub const PROPOSAL_OPS: [&str; 16] = [
    "put_view",
    "insert_layer",
    "delete_layer",
    "move_layer",
    "split_layer",
    "set_encoding",
    "insert_panel",
    "delete_panel",
    "move_panel",
    "set_panel",
    "set_scale",
    "set_emphasis",
    "set_view_range",
    "add_narrative",
    "show_finding",
    "add_study",
];

/// The decision ops (§25.2): never sent from this surface.
pub const DECISION_OPS: [&str; 4] = [
    "accept_proposal",
    "reject_proposal",
    "approve_finding",
    "reject_finding",
];

/// The surface's own tools, in order.
pub const TOOLS: [&str; 6] = ["overview", "find", "capsule", "watch", "write", "query"];

/// Default number of capsules `overview` and `find` give.
pub const DEFAULT_LIMIT: u32 = 8;
/// Most capsules `overview` and `find` give.
pub const MAX_LIMIT: u32 = 50;
/// Log entries `overview` summarises.
pub const LOG_LIMIT: u32 = 8;

/// How a capsule is given (M29).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    /// Short text card: ids, sentence, counts, evidence values (default).
    #[default]
    Card,
    /// The full protocol object(s).
    Json,
    /// A JSON summary; with `detail: true` also the story, evidence and comments.
    Layered,
}

/// `overview` arguments.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OverviewArgs {
    /// Only what moved at or after this time (epoch ms; pass the `now` of your previous
    /// overview). Default: everything the person may see.
    #[serde(default)]
    pub since_t: Option<i64>,
    /// Capsules to give (default 8, at most 50).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Capsule format (default `card`).
    #[serde(default)]
    pub format: Option<Format>,
    /// `layered` only: the full capsule items instead of summaries.
    #[serde(default)]
    pub detail: Option<bool>,
}

/// `find` arguments (filters combine with AND).
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindArgs {
    /// Instrument symbol, e.g. `THYAO`.
    #[serde(default)]
    pub instrument: Option<String>,
    /// Meaning kind.
    #[serde(default)]
    pub meaning: Option<MeaningKind>,
    /// `true`: open stories only; `false`: closed only.
    #[serde(default)]
    pub open: Option<bool>,
    /// Only stories now at these stages.
    #[serde(default)]
    pub stages: Vec<Stage>,
    /// Only stories that moved at or after this time (epoch ms).
    #[serde(default)]
    pub since_t: Option<i64>,
    /// Only stories born at or after this time (epoch ms).
    #[serde(default)]
    pub born_since_t: Option<i64>,
    /// Only the most recently born match.
    #[serde(default)]
    pub latest: bool,
    /// Apply the person's audience (watch list, positions, sensitivity).
    #[serde(default)]
    pub audience: bool,
    /// At most this many (default 8, at most 50).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Capsule format (default `card`).
    #[serde(default)]
    pub format: Option<Format>,
    /// `layered` only: the full capsule items instead of summaries.
    #[serde(default)]
    pub detail: Option<bool>,
}

/// `capsule` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapsuleArgs {
    /// Capsule id, its finding id, or a selector, e.g.
    /// `{"instrument":"THYAO","meaning":"breakout","latest":true}`.
    #[schemars(with = "CapsuleRef")]
    pub capsule: Value,
    /// Format (default `card`).
    #[serde(default)]
    pub format: Option<Format>,
    /// `layered` only: add the story, the evidence values and the comments.
    #[serde(default)]
    pub detail: Option<bool>,
}

/// What `watch` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WatchAction {
    /// Leave a watch (`text`, `match`; optional `expires_t`, `max_matches`).
    Add,
    /// Remove a watch (`intent`).
    Remove,
    /// The person's watches.
    List,
    /// Be notified of capsule transitions (optional `instrument`).
    Follow,
    /// Stop a follow (`sub`).
    Unfollow,
}

/// `watch` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WatchArgs {
    /// What to do.
    pub action: WatchAction,
    /// `add`: the request in words.
    #[serde(default)]
    pub text: Option<String>,
    /// `add`: which story transitions match (every given field must hold).
    #[serde(default, rename = "match")]
    pub match_: Option<WatchMatch>,
    /// `add`: expiry time (epoch ms).
    #[serde(default)]
    pub expires_t: Option<i64>,
    /// `add`: the watch ends after this many matches.
    #[serde(default)]
    pub max_matches: Option<u32>,
    /// `remove`: the watch's intent id (`watch-…`).
    #[serde(default)]
    pub intent: Option<String>,
    /// `follow`: only this instrument.
    #[serde(default)]
    pub instrument: Option<String>,
    /// `unfollow`: the subscription id `follow` returned.
    #[serde(default)]
    pub sub: Option<String>,
}

/// What `write` writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteKind {
    /// A comment, validated by the engine before it is published.
    Comment,
    /// A proposal of view operations; a person accepts or rejects it.
    Proposal,
}

/// `write` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WriteArgs {
    /// `comment` or `proposal`.
    pub kind: WriteKind,
    /// comment: what it is about, `{"capsule": id}` or `{"instrument": symbol}`.
    #[serde(default)]
    pub about: Option<CommentAbout>,
    /// comment: language of `text`.
    #[serde(default)]
    pub lang: Option<Language>,
    /// comment: the text; `[n]` marks the n-th entry of `cites`.
    #[serde(default)]
    pub text: Option<String>,
    /// comment: ids of the capsules, findings or facts it rests on (at least one).
    #[serde(default)]
    pub cites: Vec<String>,
    /// comment: external sources quoted, with the numbers taken from them.
    #[serde(default)]
    pub external: Vec<ExternalQuote>,
    /// proposal: the operations (applied together if a person accepts).
    #[serde(default)]
    #[schemars(with = "Vec<Op>")]
    pub ops: Vec<Value>,
    /// proposal: why, in a sentence.
    #[serde(default)]
    pub reason: Option<String>,
}

/// `query` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryArgs {
    /// A qp/0 query with its `kind`, e.g. `{"kind":"stories","instrument":"THYAO"}`,
    /// `{"kind":"ledger","meaning":"breakout"}`, `{"kind":"describe"}`.
    #[schemars(with = "Query")]
    pub query: Value,
}

/// The mixed surface.
#[derive(Debug, Default)]
pub struct Mixed {
    /// Subscription id of the `intents` topic (taken at the first `watch add`).
    intents_sub: Option<String>,
}

/// Registry constructor.
#[must_use]
pub fn new() -> Box<dyn ToolSurface> {
    Box::new(Mixed::default())
}

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

fn collect_refs(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::Object(o) => {
            if let Some(name) = o
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|r| r.strip_prefix("#/$defs/"))
            {
                out.insert(name.to_owned());
            }
            o.values().for_each(|x| collect_refs(x, out));
        }
        Value::Array(a) => a.iter().for_each(|x| collect_refs(x, out)),
        _ => {}
    }
}

/// Drops the `$defs` that the schema (outside `$defs`) no longer reaches.
fn prune_defs(schema: &mut Value) {
    let Some(defs) = schema.get("$defs").and_then(Value::as_object).cloned() else {
        return;
    };
    let mut body = schema.clone();
    if let Some(o) = body.as_object_mut() {
        o.remove("$defs");
    }
    let mut seen = BTreeSet::new();
    let mut todo = BTreeSet::new();
    collect_refs(&body, &mut todo);
    while let Some(n) = todo.pop_first() {
        if seen.insert(n.clone()) {
            if let Some(d) = defs.get(&n) {
                let mut more = BTreeSet::new();
                collect_refs(d, &mut more);
                todo.extend(more.into_iter().filter(|m| !seen.contains(m)));
            }
        }
    }
    let kept: Map<String, Value> = defs.into_iter().filter(|(k, _)| seen.contains(k)).collect();
    if let Some(o) = schema.as_object_mut() {
        if kept.is_empty() {
            o.remove("$defs");
        } else {
            o.insert("$defs".into(), Value::Object(kept));
        }
    }
}

/// The `write` schema: `Op` narrowed to [`PROPOSAL_OPS`].
fn write_schema() -> Value {
    let mut s = input_schema::<WriteArgs>();
    if let Some(variants) = s
        .pointer_mut("/$defs/Op/oneOf")
        .and_then(Value::as_array_mut)
    {
        variants.retain(|v| {
            v.pointer("/properties/op/const")
                .and_then(Value::as_str)
                .is_some_and(|n| PROPOSAL_OPS.contains(&n))
        });
    }
    prune_defs(&mut s);
    s
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

fn text_result(text: String) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "isError": false,
    })
}

fn json_result(v: Value) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": v.to_string()}],
        "structuredContent": v,
        "isError": false,
    })
}

/// A text result with the protocol reply as structured content.
fn reply_result(text: String, reply: Value, is_error: bool) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "structuredContent": reply,
        "isError": is_error,
    })
}

/// A tool error before any message was sent (the agent's arguments do not fit).
fn usage_error(text: String) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "isError": true,
    })
}

fn is_error(reply: &Value) -> bool {
    reply.get("type").and_then(Value::as_str) == Some("error")
}

/// A protocol error as text an agent can act on: code, message, path, reason, hint,
/// suggestions, candidates and (comment refusals) every reason with its hint.
fn error_text(tool: &str, reply: &Value) -> String {
    let e = reply.get("error").unwrap_or(reply);
    let mut t = format!(
        "{tool}: {} — {}",
        s(e, "code"),
        e.get("message").and_then(Value::as_str).unwrap_or("")
    );
    for k in ["path", "reason", "hint"] {
        if let Some(v) = e.get(k).and_then(Value::as_str) {
            let _ = write!(t, "\n{k}: {v}");
        }
    }
    if let Some(v) = e.get("did_you_mean") {
        let _ = write!(t, "\ndid_you_mean: {v}");
    }
    if let Some(v) = e.get("expected") {
        let _ = write!(t, "\nexpected: {v}");
    }
    if let Some(c) = e.get("candidates").and_then(Value::as_array) {
        t.push_str("\ncandidates:");
        for x in c {
            let _ = write!(
                t,
                "\n- {}",
                x.get("id").map_or_else(|| x.to_string(), plain)
            );
            if let Some(a) = x.get("about").and_then(Value::as_str) {
                let _ = write!(t, ": {a}");
            }
        }
    }
    if let Some(rs) = e.get("reasons").and_then(Value::as_array) {
        t.push_str("\nreasons (fix each and send again):");
        for r in rs {
            let _ = write!(t, "\n- [{}] {}", s(r, "check"), s(r, "code"));
            if let Some(tok) = r.get("token").and_then(Value::as_str) {
                let _ = write!(t, " token `{tok}`");
            }
            if let Some(n) = r.get("sentence") {
                let _ = write!(t, " sentence {n}");
            }
            if let Some(h) = r.get("hint").and_then(Value::as_str) {
                let _ = write!(t, ": {h}");
            }
        }
    }
    t
}

fn protocol_error(tool: &str, reply: Value) -> Value {
    reply_result(error_text(tool, &reply), reply, true)
}

// ---------------------------------------------------------------------------
// Text helpers (numbers exactly as the protocol wrote them)
// ---------------------------------------------------------------------------

/// A JSON value as plain text: strings without quotes, numbers as their JSON text.
fn plain(v: &Value) -> String {
    match v {
        Value::String(x) => x.clone(),
        Value::Null => "-".to_owned(),
        other => other.to_string(),
    }
}

fn s(v: &Value, key: &str) -> String {
    v.get(key).map_or_else(|| "-".to_owned(), plain)
}

fn ledger_text(l: &Value) -> String {
    format!("{} of {} realized", s(l, "realized"), s(l, "n"))
}

/// One line for a capsule item.
pub(crate) fn item_line(c: &Value) -> String {
    let mut flags = String::new();
    if c.get("relevant").and_then(Value::as_bool) == Some(true) {
        flags.push_str(", RELEVANT to the person");
    }
    if c.get("redacted").and_then(Value::as_bool) == Some(true) {
        flags.push_str(", redacted (no licence for the numbers)");
    }
    format!(
        "- {} [{} {} {}] stage {}, {}, tier {}, importance {}{flags}\n  {}\n  finding {}; born bar {}, last bar {} (t {}); ledger at birth {}",
        s(c, "id"),
        s(c, "instrument"),
        s(c, "kind"),
        s(c, "direction"),
        s(c, "stage"),
        s(c, "validity"),
        s(c, "tier"),
        s(c, "importance"),
        c.pointer("/sentence/text").map_or_else(String::new, plain),
        s(c, "finding"),
        s(c, "born_bar"),
        s(c, "last_bar"),
        s(c, "t"),
        c.get("ledger_at_birth").map_or_else(String::new, ledger_text),
    )
}

/// The summary of a capsule item (`layered`).
pub(crate) fn item_summary(c: &Value) -> Value {
    let mut o = Map::new();
    for k in [
        "id",
        "finding",
        "instrument",
        "kind",
        "direction",
        "stage",
        "validity",
        "tier",
        "importance",
        "born_bar",
        "last_bar",
        "t",
        "ledger_at_birth",
        "relevant",
        "redacted",
    ] {
        if let Some(v) = c.get(k) {
            o.insert(k.into(), v.clone());
        }
    }
    if let Some(t) = c.pointer("/sentence/text") {
        o.insert("sentence".into(), t.clone());
    }
    Value::Object(o)
}

fn comment_line(c: &Value) -> String {
    let by = c
        .pointer("/provenance/author/name")
        .map_or_else(|| "-".to_owned(), plain);
    let mut t = format!("- {} by {by}", s(c, "id"));
    if let Some(p) = c.get("on_behalf").and_then(Value::as_str) {
        let _ = write!(t, " for {p}");
    }
    let _ = write!(
        t,
        " (cites {}): {}",
        c.get("cites").map_or_else(String::new, Value::to_string),
        s(c, "text")
    );
    t
}

fn evidence_line(e: &Value) -> String {
    let mut t = format!(
        "- {} ({}, {})",
        s(e, "fact"),
        s(e, "family"),
        s(e, "license")
    );
    if e.get("open").and_then(Value::as_bool) == Some(true) {
        let vals: Vec<String> = e
            .get("values")
            .and_then(Value::as_object)
            .map(|m| m.iter().map(|(k, v)| format!("{k}={}", plain(v))).collect())
            .unwrap_or_default();
        let _ = write!(t, ": {}", vals.join(", "));
    } else {
        let _ = write!(t, ": values need `{}`", s(e, "requires"));
    }
    let _ = write!(t, "; rests on {}", s(e, "rests_on"));
    t
}

pub(crate) fn capsule_card(d: &Value) -> String {
    let i = d.get("item").cloned().unwrap_or(Value::Null);
    let mut t = format!(
        "Capsule {} (finding {})\n{} {} {} · stage {} · state {} · validity {} · tier {} · importance {} · confirmations {} · trust {}",
        s(&i, "id"),
        s(&i, "finding"),
        s(&i, "instrument"),
        s(&i, "kind"),
        s(&i, "direction"),
        s(&i, "stage"),
        s(&i, "state"),
        s(&i, "validity"),
        s(&i, "tier"),
        s(&i, "importance"),
        s(&i, "confirmations"),
        s(&i, "trust"),
    );
    if i.get("redacted").and_then(Value::as_bool) == Some(true) {
        t.push_str(" · redacted (no licence for the numbers)");
    }
    let _ = write!(
        t,
        "\nSentence: {}",
        i.pointer("/sentence/text").map_or_else(String::new, plain)
    );
    let hist = d
        .pointer("/story/history")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let _ = write!(t, "\nStory ({} transitions):", hist.len());
    for h in &hist {
        let _ = write!(
            t,
            "\n- bar {} (t {}) {}: {} [facts {}]",
            s(h, "bar"),
            s(h, "t"),
            s(h, "stage"),
            s(h, "reason"),
            h.get("facts").map_or_else(String::new, Value::to_string)
        );
    }
    let _ = write!(
        t,
        "\nLedger now: {}; at birth: {}",
        d.get("ledger").map_or_else(String::new, ledger_text),
        i.get("ledger_at_birth")
            .map_or_else(String::new, ledger_text)
    );
    t.push_str("\nEvidence:");
    for e in d
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let _ = write!(t, "\n{}", evidence_line(e));
    }
    let comments = d
        .get("comments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let _ = write!(t, "\nComments ({}):", comments.len());
    for c in &comments {
        let _ = write!(t, "\n{}", comment_line(c));
    }
    if let Some(w) = d.get("withheld") {
        let _ = write!(
            t,
            "\nWithheld: newer licensed activity needs {}",
            w.get("requires").map_or_else(String::new, Value::to_string)
        );
    }
    let facts: Vec<String> = d
        .get("facts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|f| s(f, "id"))
        .collect();
    let _ = write!(
        t,
        "\nIds: capsule {}, finding {}, facts {}",
        s(&i, "id"),
        s(&i, "finding"),
        facts.join(", ")
    );
    t
}

pub(crate) fn capsule_layered(d: &Value, detail: bool) -> Value {
    let i = d.get("item").cloned().unwrap_or(Value::Null);
    let mut sum = item_summary(&i);
    let o = sum.as_object_mut().expect("object");
    for k in ["state", "confirmations", "trust"] {
        if let Some(v) = i.get(k) {
            o.insert(k.into(), v.clone());
        }
    }
    if let Some(l) = d.get("ledger") {
        o.insert("ledger".into(), l.clone());
    }
    let facts: Vec<Value> = d
        .get("facts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|f| f.get("id").cloned())
        .collect();
    o.insert("facts".into(), Value::Array(facts));
    let ev = d.get("evidence").and_then(Value::as_array);
    o.insert(
        "evidence_open".into(),
        json!(ev.map_or(0, |e| e.iter().filter(|x| x["open"] == true).count())),
    );
    o.insert(
        "comments".into(),
        json!(d
            .get("comments")
            .and_then(Value::as_array)
            .map_or(0, Vec::len)),
    );
    let mut out = json!({"kind": "capsule", "summary": sum});
    if detail {
        out["story"] = d.pointer("/story/history").cloned().unwrap_or(json!([]));
        out["evidence"] = d.get("evidence").cloned().unwrap_or(json!([]));
        out["comments"] = d.get("comments").cloned().unwrap_or(json!([]));
        if let Some(w) = d.get("withheld") {
            out["withheld"] = w.clone();
        }
    } else {
        out["more"] = json!(
            "call again with detail:true for the story, the evidence values and the comments"
        );
    }
    out
}

/// Capsule items laid out in `format`.
fn items_out(items: &[Value], format: Format, detail: bool) -> (String, Value) {
    let mut text = String::new();
    for c in items {
        let _ = write!(text, "\n{}", item_line(c));
    }
    let v = match format {
        Format::Json => Value::Array(items.to_vec()),
        Format::Layered if detail => Value::Array(items.to_vec()),
        _ => Value::Array(items.iter().map(item_summary).collect()),
    };
    (text, v)
}

fn limit_of(l: Option<u32>) -> u32 {
    l.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

fn parse<T: for<'de> Deserialize<'de>>(tool: &str, args: Value) -> Result<T, RpcError> {
    serde_json::from_value(args).map_err(|e| RpcError::new(-32602, format!("tool `{tool}`: {e}")))
}

fn data(reply: &Value) -> Value {
    reply.get("data").cloned().unwrap_or(Value::Null)
}

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------

impl Mixed {
    fn overview(ctx: &mut Ctx<'_>, a: &OverviewArgs) -> Value {
        let format = a.format.unwrap_or_default();
        let limit = limit_of(a.limit);
        let mut cq = json!({"kind": "capsules", "audience": true, "limit": limit});
        if let Some(t) = a.since_t {
            cq["since_t"] = json!(t);
        }
        let caps = ctx.query(cq);
        if is_error(&caps) {
            return protocol_error("overview", caps);
        }
        let mut items: Vec<Value> = data(&caps)
            .get("capsules")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        // The person's holdings and watches first (stable: the authority's order kept).
        items.sort_by_key(|c| c.get("relevant").and_then(Value::as_bool) != Some(true));
        let server_comments = data(&caps).get("comments").cloned();
        let intents = ctx.query(json!({"kind": "list", "collection": "intents"}));
        let watches: Vec<Value> = data(&intents)
            .get("items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|i| i.get("watch").is_some())
            .cloned()
            .collect();
        let mut lq = json!({"kind": "log", "limit": LOG_LIMIT});
        if let Some(t) = a.since_t {
            lq["since_t"] = json!(t);
        }
        let log = ctx.query(lq);
        let changes: Vec<Value> = data(&log)
            .get("entries")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|e| json!({"seq": e["seq"], "at": e["at"], "author": e["author"], "summary": e["summary"]}))
            .collect();
        let props =
            ctx.query(json!({"kind": "list", "collection": "proposals", "state": "pending"}));
        let pending: Vec<Value> = data(&props)
            .get("items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|p| {
                let ops: Vec<Value> = p
                    .get("ops")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|o| o.get("op").cloned())
                    .collect();
                json!({"id": p["id"], "by": p.pointer("/provenance/author/name"), "ops": ops, "reason": p.get("reason")})
            })
            .collect();
        let now = caps.get("now").cloned().unwrap_or(Value::Null);
        let seq = caps.get("seq").cloned().unwrap_or(Value::Null);
        let detail = a.detail == Some(true);
        let (lines, items_v) = items_out(&items, format, detail);
        if format == Format::Card {
            let mut t = format!(
                "Overview at now {} (seq {}). Capsules for the person ({}; relevant ones first):{lines}",
                plain(&now),
                plain(&seq),
                items.len()
            );
            if let Some(sc) = server_comments.as_ref().and_then(Value::as_array) {
                if !sc.is_empty() {
                    let _ = write!(t, "\nServer comments on licensed data ({}):", sc.len());
                    for c in sc {
                        let _ = write!(
                            t,
                            "\n- {} bar {}: {}",
                            s(c, "instrument"),
                            s(c, "bar"),
                            s(c, "en")
                        );
                    }
                }
            }
            let _ = write!(t, "\nWatches ({}):", watches.len());
            for w in &watches {
                let _ = write!(
                    t,
                    "\n- {} [{}; matches {}] {}: {}",
                    s(w, "id"),
                    s(w, "state"),
                    s(w, "matches"),
                    s(w, "text"),
                    w.pointer("/watch/match")
                        .map_or_else(String::new, Value::to_string)
                );
            }
            let _ = write!(t, "\nPending proposals ({}):", pending.len());
            for p in &pending {
                let _ = write!(t, "\n- {} by {}: {}", s(p, "id"), s(p, "by"), p["ops"]);
            }
            let _ = write!(t, "\nRecent changes ({}):", changes.len());
            for c in &changes {
                let _ = write!(
                    t,
                    "\n- #{} {}: {}",
                    s(c, "seq"),
                    s(c, "author"),
                    s(c, "summary")
                );
            }
            t.push_str("\nNext: capsule {capsule: id} explains one; overview {since_t: now} gives what moved since.");
            return text_result(t);
        }
        let mut out = json!({
            "kind": "overview", "now": now, "seq": seq, "capsules": items_v,
            "watches": watches, "pending_proposals": pending, "changes": changes,
        });
        if let Some(sc) = server_comments {
            out["server_comments"] = sc;
        }
        json_result(out)
    }

    fn find(ctx: &mut Ctx<'_>, a: &FindArgs) -> Value {
        let format = a.format.unwrap_or_default();
        let mut q = json!({"kind": "capsules", "limit": limit_of(a.limit)});
        if let Some(i) = &a.instrument {
            q["instrument"] = json!(i);
        }
        if let Some(m) = &a.meaning {
            q["meaning"] = json!(m);
        }
        if let Some(o) = a.open {
            q["open"] = json!(o);
        }
        if !a.stages.is_empty() {
            q["stages"] = json!(a.stages);
        }
        if let Some(t) = a.since_t {
            q["since_t"] = json!(t);
        }
        if let Some(t) = a.born_since_t {
            q["born_since_t"] = json!(t);
        }
        if a.latest {
            q["latest"] = json!(true);
        }
        if a.audience {
            q["audience"] = json!(true);
        }
        let r = ctx.query(q);
        if is_error(&r) {
            return protocol_error("find", r);
        }
        let d = data(&r);
        let items: Vec<Value> = d
            .get("capsules")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let (lines, items_v) = items_out(&items, format, a.detail == Some(true));
        if format == Format::Card {
            let mut t = format!("{} capsule(s), most important first:{lines}", items.len());
            if let Some(sc) = d.get("comments").and_then(Value::as_array) {
                for c in sc {
                    let _ = write!(
                        t,
                        "\n- server comment {} bar {}: {}",
                        s(c, "instrument"),
                        s(c, "bar"),
                        s(c, "en")
                    );
                }
            }
            if items.is_empty() {
                t.push_str("\nNothing matches; drop a filter, or query {kind:\"stories\", instrument} for the instrument's whole history.");
            }
            return text_result(t);
        }
        let mut out = json!({"kind": "capsules", "capsules": items_v});
        if let Some(sc) = d.get("comments") {
            out["server_comments"] = sc.clone();
        }
        json_result(out)
    }

    fn capsule(ctx: &mut Ctx<'_>, a: CapsuleArgs) -> Value {
        let r = ctx.query(json!({"kind": "capsule", "capsule": a.capsule}));
        if is_error(&r) {
            return protocol_error("capsule", r);
        }
        let d = data(&r).get("detail").cloned().unwrap_or(Value::Null);
        match a.format.unwrap_or_default() {
            Format::Card => text_result(capsule_card(&d)),
            Format::Json => {
                let mut v = data(&r);
                if let Some(res) = r.get("resolved") {
                    v["resolved"] = res.clone();
                }
                json_result(v)
            }
            Format::Layered => {
                let mut v = capsule_layered(&d, a.detail == Some(true));
                if let Some(res) = r.get("resolved") {
                    v["resolved"] = res.clone();
                }
                json_result(v)
            }
        }
    }

    fn watch(&mut self, ctx: &mut Ctx<'_>, a: WatchArgs) -> Value {
        match a.action {
            WatchAction::Add => {
                let (Some(text), Some(m)) = (a.text, a.match_) else {
                    return usage_error(
                        "watch add needs `text` and `match`, e.g. {\"action\":\"add\",\"text\":\"tell me if THYAO weakens\",\"match\":{\"instrument\":\"THYAO\",\"negative\":true}}".into(),
                    );
                };
                let mut op = json!({"op": "add_watch", "text": text, "match": m});
                if let Some(t) = a.expires_t {
                    op["expires_t"] = json!(t);
                }
                if let Some(n) = a.max_matches {
                    op["max_matches"] = json!(n);
                }
                let r = ctx.command(vec![op]);
                if is_error(&r) {
                    return protocol_error("watch", r);
                }
                let id = r
                    .pointer("/created/0")
                    .map_or_else(|| "-".to_owned(), plain);
                let note = self.ensure_intents_sub(ctx);
                reply_result(
                    format!("Watch {id} set (seq {}). Matches arrive as `intent_matched` notifications{note} (read them with the `notifications` tool).", s(&r, "seq")),
                    r,
                    false,
                )
            }
            WatchAction::Remove => {
                let Some(intent) = a.intent else {
                    return usage_error(
                        "watch remove needs `intent` (the watch id from watch list)".into(),
                    );
                };
                let r = ctx.command(vec![json!({"op": "remove_watch", "intent": intent})]);
                if is_error(&r) {
                    return protocol_error("watch", r);
                }
                reply_result(
                    format!("Watch {intent} removed (seq {}).", s(&r, "seq")),
                    r,
                    false,
                )
            }
            WatchAction::List => {
                let r = ctx.query(json!({"kind": "list", "collection": "intents"}));
                if is_error(&r) {
                    return protocol_error("watch", r);
                }
                let items: Vec<Value> = data(&r)
                    .get("items")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter(|i| i.get("watch").is_some())
                    .cloned()
                    .collect();
                let mut t = format!("{} watch(es):", items.len());
                for w in &items {
                    let _ = write!(
                        t,
                        "\n- {} [{}; matches {}] {}: {}",
                        s(w, "id"),
                        s(w, "state"),
                        s(w, "matches"),
                        s(w, "text"),
                        w.pointer("/watch/match")
                            .map_or_else(String::new, Value::to_string)
                    );
                }
                reply_result(t, json!({"watches": items}), false)
            }
            WatchAction::Follow => {
                let id = ctx.request_id();
                let mut topic = json!({"kind": "capsules"});
                if let Some(i) = a.instrument {
                    topic["instrument"] = json!(i);
                }
                let r = ctx.send(&json!({"type": "subscribe", "id": id, "topic": topic}));
                if is_error(&r) {
                    return protocol_error("watch", r);
                }
                reply_result(
                    format!("Following capsule transitions (subscription `{id}`, the person's audience applies). Read them with the `notifications` tool; stop with watch unfollow {{sub: \"{id}\"}}."),
                    r,
                    false,
                )
            }
            WatchAction::Unfollow => {
                let Some(sub) = a.sub else {
                    return usage_error(
                        "watch unfollow needs `sub` (the id follow returned)".into(),
                    );
                };
                let id = ctx.request_id();
                let r = ctx.send(&json!({"type": "unsubscribe", "id": id, "sub": sub}));
                if is_error(&r) {
                    return protocol_error("watch", r);
                }
                reply_result(format!("Subscription `{sub}` ended."), r, false)
            }
        }
    }

    /// Subscribes to the `intents` topic once (matches of the person's watches).
    fn ensure_intents_sub(&mut self, ctx: &mut Ctx<'_>) -> String {
        if let Some(id) = &self.intents_sub {
            return format!(" (subscription `{id}`)");
        }
        let id = ctx.fresh_id();
        let r = ctx.send(&json!({"type": "subscribe", "id": id, "topic": {"kind": "intents"}}));
        if is_error(&r) {
            return format!(
                " once subscribed; the subscription failed: {}",
                error_text("watch", &r)
            );
        }
        let note = format!(" (subscription `{id}`)");
        self.intents_sub = Some(id);
        note
    }

    fn write(ctx: &mut Ctx<'_>, a: WriteArgs) -> Value {
        match a.kind {
            WriteKind::Comment => {
                let (Some(about), Some(lang), Some(text)) = (a.about, a.lang, a.text) else {
                    return usage_error(
                        "write comment needs `about`, `lang` and `text` (and `cites`), e.g. {\"kind\":\"comment\",\"about\":{\"capsule\":\"cap.THYAO.breakout.up.147\"},\"lang\":\"en\",\"text\":\"THYAO broke above resistance [1].\",\"cites\":[\"cap.THYAO.breakout.up.147\"]}".into(),
                    );
                };
                if !a.ops.is_empty() {
                    return usage_error(
                        "write comment takes no `ops`; send a proposal separately".into(),
                    );
                }
                let mut op = json!({"op": "add_comment", "about": about, "lang": lang, "text": text, "cites": a.cites});
                if !a.external.is_empty() {
                    op["external"] = json!(a.external);
                }
                let r = ctx.command(vec![op]);
                if is_error(&r) {
                    return protocol_error("write", r);
                }
                let id = r
                    .pointer("/created/0")
                    .map_or_else(|| "-".to_owned(), plain);
                reply_result(
                    format!(
                        "Comment {id} published (seq {}); it shows on the capsule with its origin.",
                        s(&r, "seq")
                    ),
                    r,
                    false,
                )
            }
            WriteKind::Proposal => {
                if a.ops.is_empty() {
                    return usage_error("write proposal needs `ops` (view operations, e.g. [{\"op\":\"set_view_range\",\"view\":{},\"from_bar\":100,\"to_bar\":150}])".into());
                }
                for (i, o) in a.ops.iter().enumerate() {
                    let name = o.get("op").and_then(Value::as_str).unwrap_or("");
                    if DECISION_OPS.contains(&name) {
                        return usage_error(format!(
                            "write: ops[{i}] `{name}` is a person's decision (§25.2); agents never accept or reject proposals or findings. Nothing was sent."
                        ));
                    }
                }
                let id = ctx.request_id();
                let mut msg = json!({"type": "command", "id": id, "mode": "propose", "ops": a.ops});
                if let Some(r) = a.reason {
                    msg["reason"] = json!(r);
                }
                let r = ctx.send(&msg);
                if is_error(&r) {
                    return protocol_error("write", r);
                }
                let pid = s(&r, "proposal");
                reply_result(
                    format!("Proposal {pid} is pending a person's decision (status {}). See it with query {{kind:\"preview\", proposal:\"{pid}\"}} or render {{view, proposal:\"{pid}\"}}.", s(&r, "status")),
                    r,
                    false,
                )
            }
        }
    }

    fn query(ctx: &mut Ctx<'_>, a: QueryArgs) -> Value {
        if a.query.get("kind").and_then(Value::as_str).is_none() {
            return usage_error(
                "query needs an object with `kind`, e.g. {\"query\":{\"kind\":\"describe\"}}"
                    .into(),
            );
        }
        tool_result(ctx.query(a.query))
    }
}

impl ToolSurface for Mixed {
    fn name(&self) -> &'static str {
        "mixed"
    }

    fn tools(&self) -> Vec<ToolDef> {
        let t = |name: &str, description: &str, input_schema: Value, annotations| ToolDef {
            name: name.to_owned(),
            description: description.to_owned(),
            input_schema,
            annotations,
            output_schema: None,
        };
        vec![
            t(
                "overview",
                "Start here. What matters to the person now: their capsules (stories) with the ones about their holdings and watches first, their watches, pending proposals and the latest document changes. Pass since_t = the previous `now` for what moved since. Example: {} or {\"since_t\":1760000000000,\"limit\":5}.",
                input_schema::<OverviewArgs>(),
                Annotations::READ,
            ),
            t(
                "find",
                "Find capsules by meaning, most important first. Example: {\"instrument\":\"THYAO\",\"open\":true} or {\"meaning\":\"breakout\",\"latest\":true}. For an instrument's full story history use query {kind:\"stories\"}.",
                input_schema::<FindArgs>(),
                Annotations::READ,
            ),
            t(
                "capsule",
                "Explain one capsule: sentence, story transitions with reasons, ledger (track record), evidence values per fact (or the licence they need) and validated comments, with every id. Example: {\"capsule\":\"cap.THYAO.breakout.up.147\"} or {\"capsule\":{\"instrument\":\"THYAO\",\"meaning\":\"breakout\",\"latest\":true}}.",
                input_schema::<CapsuleArgs>(),
                Annotations::READ,
            ),
            t(
                "watch",
                "Watches for the person: add {\"action\":\"add\",\"text\":\"tell me if THYAO weakens\",\"match\":{\"instrument\":\"THYAO\",\"negative\":true}}, remove {\"action\":\"remove\",\"intent\":\"watch-7-0\"}, list; follow/unfollow capsule transitions. Matches arrive as notifications.",
                input_schema::<WatchArgs>(),
                Annotations::WRITE,
            ),
            t(
                "write",
                "Write a comment (the engine checks citations, numbers, advice wording and licence; a refusal lists each reason with a fix) or propose view changes for a person to decide. Example: {\"kind\":\"comment\",\"about\":{\"capsule\":\"cap.X\"},\"lang\":\"en\",\"text\":\"X broke above resistance [1].\",\"cites\":[\"cap.X\"]}.",
                write_schema(),
                Annotations::WRITE,
            ),
            t(
                "query",
                "Escape hatch: run any qp/0 query as is and get the protocol reply. Example: {\"query\":{\"kind\":\"stories\",\"instrument\":\"THYAO\"}}; kinds: describe, get, list, log, study, at, screen, ledger, stories, capsules, capsule, preview, ….",
                input_schema::<QueryArgs>(),
                Annotations::READ,
            ),
        ]
    }

    fn call(&mut self, ctx: &mut Ctx<'_>, name: &str, args: Value) -> Result<Value, RpcError> {
        let args = if args.is_null() { json!({}) } else { args };
        Ok(match name {
            "overview" => Self::overview(ctx, &parse::<OverviewArgs>(name, args)?),
            "find" => Self::find(ctx, &parse::<FindArgs>(name, args)?),
            "capsule" => Self::capsule(ctx, parse(name, args)?),
            "watch" => self.watch(ctx, parse(name, args)?),
            "write" => Self::write(ctx, parse(name, args)?),
            "query" => Self::query(ctx, parse(name, args)?),
            _ => return Err(RpcError::new(-32602, format!("Unknown tool: {name}"))),
        })
    }

    fn instructions(&self, principal: &str) -> String {
        format!(
            "Qapsule: a live chart document; the deterministic engine states facts, capsules (stories) and their ledgers, you read, explain, watch and propose. \
             You speak as `{principal}` and read with the data entitlement of the person you work for. \
             Start with `overview`; use `find` and `capsule` to look up and explain capsules (format `card` by default, `json` or `layered`). \
             Use `watch` for standing requests and `write` for a cited comment or a view proposal; a refused comment lists each reason with a fix. \
             Use `query` for anything else (any qp/0 query kind: stories, ledger, log, study, at, screen, describe, preview). \
             Quote numbers only as given and cite the ids they come from; people, not agents, decide proposals."
        )
    }
}
