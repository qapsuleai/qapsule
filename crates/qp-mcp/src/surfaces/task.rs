//! (M28 b) The task surface: a few tools, each one agent task (`rounds/R5B/PLAN.md` §6).
//!
//! | Tool | Task | `qp/0` behind it |
//! |------|------|------------------|
//! | `whats_new` | what moved since my last look (holdings and watches first) | `capsules`, `list intents` |
//! | `find` | capsules by instrument, meaning, direction, stage, open/closed | `capsules` |
//! | `explain` | one capsule: story, evidence (or what it requires), track record, comments | `capsule` |
//! | `instrument` | an instrument's stories and the session clock (no symbol: the instruments) | `stories`, `replay status`, `list series` |
//! | `track_record` | the ledger | `ledger` |
//! | `watch` | add / remove / list watches (intents) | `add_watch`, `remove_watch`, `list intents`, `subscribe intents` |
//! | `comment` | a validated comment; rejections come back machine-readable with hints | `add_comment` |
//! | `propose` | a view or drawing proposal with its preview | `command` (`mode: propose`), `preview` |
//! | `glossary` | the closed vocabulary (meaning kinds, stages, …) | the schema's own descriptions |
//!
//! No decision tool: accepting or rejecting a proposal or a finding is a person's act
//! (§25.2). The surface only sends `qp/0` messages through [`Ctx`] (the authority applies
//! permissions, entitlements and provenance) and never computes or rounds a number: every
//! number it prints is the protocol value's own JSON text. Capsule-giving tools take
//! `format`: `card` (default: short text + ids + numbers), `json` (the protocol object) or
//! `layered` (summary; with `detail: true` the rest). `card` and `layered` with `detail`
//! carry the same numbers; `json` carries them and more.

use std::collections::BTreeMap;

use qp_schema::capsule::{
    Direction, EvidenceFamily, MeaningKind, Stage, TimeKind, Trust, Validity,
};
use qp_schema::comment::ExternalQuote;
use qp_schema::protocol::{CapsuleRef, Language};
use qp_schema::WatchMatch;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{input_schema, Annotations, Ctx, ToolDef, ToolSurface};
use crate::RpcError;

/// Default rows of a list answer.
pub const DEFAULT_LIMIT: u32 = 10;
/// Most rows of a list answer.
pub const MAX_LIMIT: u32 = 50;

/// Registry constructor.
#[must_use]
pub fn new() -> Box<dyn ToolSurface> {
    Box::new(Task::default())
}

/// The task surface. Its state is the agent's "last look" (what `whats_new` compares
/// with) and whether it subscribed to its watches' matches.
#[derive(Debug, Default)]
pub struct Task {
    /// Capsule id → (stage, last bar) at the last `whats_new`.
    seen: Option<BTreeMap<String, (String, u64)>>,
    /// Watch id → (state, matches) at the last `whats_new`.
    seen_watches: BTreeMap<String, (String, String)>,
    /// Subscribed to the `intents` topic.
    subscribed: bool,
}

/// How a capsule-giving tool answers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    /// Short text: ids, numbers, the sentence.
    #[default]
    Card,
    /// The protocol object, whole.
    Json,
    /// Summary first; the rest with `detail: true`.
    Layered,
}

/// `whats_new` arguments.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WhatsNewArgs {
    /// Look back to this time (epoch ms) instead of your last look.
    #[serde(default)]
    pub since_t: Option<i64>,
    /// Only the person's feed (watch list, sensitivity; holdings and watches always).
    #[serde(default)]
    pub feed: bool,
    /// Most capsules (default 10, at most 50).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Answer format (default `card`).
    #[serde(default)]
    pub format: Format,
    /// With `layered`: include the detail.
    #[serde(default)]
    pub detail: bool,
    /// Sentence language (default: the reader's).
    #[serde(default)]
    pub lang: Option<Language>,
}

/// `find` arguments (all filters optional, AND-ed).
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindArgs {
    /// Instrument symbol, e.g. `THYAO`.
    #[serde(default)]
    pub instrument: Option<String>,
    /// Meaning kind.
    #[serde(default)]
    pub meaning: Option<MeaningKind>,
    /// Direction.
    #[serde(default)]
    pub direction: Option<Direction>,
    /// Only these current stages.
    #[serde(default)]
    pub stages: Vec<Stage>,
    /// `true`: open stories only; `false`: closed only.
    #[serde(default)]
    pub open: Option<bool>,
    /// Moved at or after this time (epoch ms).
    #[serde(default)]
    pub since_t: Option<i64>,
    /// Born at or after this time (epoch ms).
    #[serde(default)]
    pub born_since_t: Option<i64>,
    /// Only the latest-born match.
    #[serde(default)]
    pub latest: bool,
    /// Only the person's feed.
    #[serde(default)]
    pub feed: bool,
    /// Rows (default 10, at most 50).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Next page: the previous answer's `next`.
    #[serde(default)]
    pub after: Option<String>,
    /// Answer format (default `card`).
    #[serde(default)]
    pub format: Format,
    /// With `layered`: include the detail.
    #[serde(default)]
    pub detail: bool,
    /// Sentence language (default: the reader's).
    #[serde(default)]
    pub lang: Option<Language>,
}

/// `explain` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExplainArgs {
    /// Capsule id, finding id, or a selector such as
    /// `{"instrument":"THYAO","meaning":"breakout","latest":true}`.
    pub capsule: CapsuleRef,
    /// Answer format (default `card`).
    #[serde(default)]
    pub format: Format,
    /// With `layered`: include story, evidence and comments.
    #[serde(default)]
    pub detail: bool,
    /// Sentence language (default: the reader's).
    #[serde(default)]
    pub lang: Option<Language>,
}

/// `instrument` arguments.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstrumentArgs {
    /// Instrument symbol; absent: list the instruments.
    #[serde(default)]
    pub symbol: Option<String>,
    /// Only stories alive in the last this many bars.
    #[serde(default)]
    pub last: Option<u32>,
    /// Rows (default 10, at most 50).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Next page: the previous answer's `next`.
    #[serde(default)]
    pub after: Option<String>,
    /// Answer format (default `card`).
    #[serde(default)]
    pub format: Format,
    /// With `layered`: include the detail.
    #[serde(default)]
    pub detail: bool,
}

/// `track_record` arguments.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrackRecordArgs {
    /// Meaning kind.
    #[serde(default)]
    pub meaning: Option<MeaningKind>,
    /// Direction.
    #[serde(default)]
    pub direction: Option<Direction>,
    /// One ledger key, e.g. `breakout/up/breakout.range@1`.
    #[serde(default)]
    pub key: Option<String>,
    /// Sum per meaning kind instead of per key.
    #[serde(default)]
    pub by_kind: bool,
}

/// What `watch` does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WatchAction {
    /// Leave a watch.
    #[default]
    Add,
    /// Remove a watch (`intent`).
    Remove,
    /// List your watches.
    List,
}

/// `watch` arguments.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WatchArgs {
    /// `add` (default), `remove` or `list`.
    #[serde(default)]
    pub action: WatchAction,
    /// `add`: the request in words.
    #[serde(default)]
    pub text: Option<String>,
    /// `add`: what story transitions match (every given field must hold).
    #[serde(default, rename = "match")]
    pub match_: Option<WatchMatch>,
    /// `add`: expiry (epoch ms).
    #[serde(default)]
    pub expires_t: Option<i64>,
    /// `add`: end after this many matches.
    #[serde(default)]
    pub max_matches: Option<u32>,
    /// `remove`: the watch id (`watch-…`).
    #[serde(default)]
    pub intent: Option<String>,
}

/// `comment` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommentArgs {
    /// The capsule it is about (id or selector); or give `instrument`.
    #[serde(default)]
    pub capsule: Option<CapsuleRef>,
    /// The instrument it is about (symbol).
    #[serde(default)]
    pub instrument: Option<String>,
    /// Language of `text`.
    pub lang: Language,
    /// Plain text; `[n]` cites `cites[n-1]`. Only numbers the cited items give.
    pub text: String,
    /// Ids it rests on (capsule, finding or fact; at least one).
    pub cites: Vec<String>,
    /// Named external quotes with the numbers they give.
    #[serde(default)]
    pub external: Vec<ExternalQuote>,
}

/// `propose` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProposeArgs {
    /// `qp/0` view/drawing ops (applied together if accepted).
    pub ops: Vec<Value>,
    /// Why, for the person deciding.
    #[serde(default)]
    pub reason: Option<String>,
}

/// `glossary` arguments.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GlossaryArgs {
    /// Only entries whose name or text contains this (case-insensitive).
    #[serde(default)]
    pub term: Option<String>,
}

const TOOLS: [&str; 9] = [
    "whats_new",
    "find",
    "explain",
    "instrument",
    "track_record",
    "watch",
    "comment",
    "propose",
    "glossary",
];

/// The generated schema with every documented unit enum (`oneOf` of `const`s) written
/// as a plain `enum` list: the variants' meanings are what `glossary` gives, so they are
/// not repeated in every tool definition (M28 measures the definitions' size).
fn compact(mut schema: Value) -> Value {
    fn walk(v: &mut Value) {
        match v {
            Value::Object(o) => {
                let consts: Option<Vec<Value>> =
                    o.get("oneOf").and_then(Value::as_array).and_then(|alts| {
                        alts.iter()
                            .map(|a| a.get("const").filter(|c| c.is_string()).cloned())
                            .collect()
                    });
                if let Some(cs) = consts {
                    o.remove("oneOf");
                    o.insert("type".to_owned(), json!("string"));
                    o.insert("enum".to_owned(), Value::Array(cs));
                }
                for x in o.values_mut() {
                    walk(x);
                }
            }
            Value::Array(a) => a.iter_mut().for_each(walk),
            _ => {}
        }
    }
    walk(&mut schema);
    schema
}

fn def(name: &str, description: &str, input_schema: Value, annotations: Annotations) -> ToolDef {
    ToolDef {
        name: name.to_owned(),
        description: description.to_owned(),
        input_schema: compact(input_schema),
        annotations,
        output_schema: None,
    }
}

impl ToolSurface for Task {
    fn name(&self) -> &'static str {
        "task"
    }

    fn tools(&self) -> Vec<ToolDef> {
        // `whats_new` remembers the look (state), so it is not idempotent; it does not
        // change the document.
        let look = Annotations {
            idempotent: false,
            ..Annotations::READ
        };
        vec![
            def(
                "whats_new",
                "What moved since your last look: capsules born, changed or closed, the \
                 person's holdings and watches first, plus watch matches. First call: the \
                 latest moves. Example: {} or {\"since_t\":1760000000000}.",
                input_schema::<WhatsNewArgs>(),
                look,
            ),
            def(
                "find",
                "Find capsules (stories) by instrument, meaning, direction, stage, open/closed \
                 or time; most important first, paged. Example: \
                 {\"instrument\":\"THYAO\",\"meaning\":\"breakout\",\"open\":true}.",
                input_schema::<FindArgs>(),
                Annotations::READ,
            ),
            def(
                "explain",
                "Open one capsule: sentence, story transitions, evidence per fact (values, or \
                 the license it requires), track record, comments. Example: \
                 {\"capsule\":{\"instrument\":\"THYAO\",\"meaning\":\"breakout\",\"latest\":true}}.",
                input_schema::<ExplainArgs>(),
                Annotations::READ,
            ),
            def(
                "instrument",
                "An instrument's stories (born, closed, outcome, in your feed or why not) and \
                 the session clock; without `symbol`, the instruments. Example: \
                 {\"symbol\":\"THYAO\",\"last\":20}.",
                input_schema::<InstrumentArgs>(),
                Annotations::READ,
            ),
            def(
                "track_record",
                "The ledger: closed and realized counts per key (kind/direction/detector) or \
                 per kind. Example: {\"meaning\":\"breakout\",\"direction\":\"up\"}.",
                input_schema::<TrackRecordArgs>(),
                Annotations::READ,
            ),
            def(
                "watch",
                "Leave, remove or list watches for the person you work for; matches arrive \
                 as notifications. Example: {\"text\":\"tell me if THYAO weakens\",\
                 \"match\":{\"instrument\":\"THYAO\",\"negative\":true}}; \
                 {\"action\":\"remove\",\"intent\":\"watch-7-0\"}.",
                input_schema::<WatchArgs>(),
                Annotations::WRITE,
            ),
            def(
                "comment",
                "Attach a short comment to a capsule or instrument. The engine checks it: \
                 cited ids visible, every number from the cited evidence (as written there), \
                 no advice, no licensed number. A rejection lists reasons with hints. \
                 Example: {\"capsule\":\"cap.THYAO.breakout.up.147\",\"lang\":\"en\",\
                 \"text\":\"THYAO closed above 312.5 [1].\",\"cites\":[\"cap.THYAO.breakout.up.147\"]}.",
                input_schema::<CommentArgs>(),
                Annotations::WRITE,
            ),
            def(
                "propose",
                "Propose view/drawing ops (put_view, insert_layer, set_encoding, insert_panel, \
                 set_emphasis, set_view_range, add_study, show_finding…); returns the pending \
                 proposal and its preview. Only a person can accept it. Example: \
                 {\"ops\":[{\"op\":\"set_view_range\",\"view\":\"main\",\"from_bar\":100,\"to_bar\":150}]}.",
                input_schema::<ProposeArgs>(),
                Annotations::WRITE,
            ),
            def(
                "glossary",
                "The closed vocabulary: meaning kinds, directions, stages, validity, time \
                 kinds, trust, evidence families. Example: {\"term\":\"breakout\"}.",
                input_schema::<GlossaryArgs>(),
                Annotations::READ,
            ),
        ]
    }

    fn call(&mut self, ctx: &mut Ctx<'_>, name: &str, args: Value) -> Result<Value, RpcError> {
        if !TOOLS.contains(&name) {
            return Err(RpcError::new(-32602, format!("Unknown tool: {name}")));
        }
        let args = if args.is_null() { json!({}) } else { args };
        match name {
            "whats_new" => self.whats_new(ctx, &parse(name, args)?),
            "find" => Ok(find(ctx, &parse(name, args)?)),
            "explain" => Ok(explain(ctx, &parse(name, args)?)),
            "instrument" => Ok(instrument(ctx, &parse(name, args)?)),
            "track_record" => Ok(track_record(ctx, &parse(name, args)?)),
            "watch" => self.watch(ctx, parse(name, args)?),
            "comment" => Ok(comment(ctx, parse(name, args)?)),
            "propose" => Ok(propose(ctx, parse(name, args)?)),
            _ => Ok(glossary(&parse(name, args)?)),
        }
    }

    fn instructions(&self, principal: &str) -> String {
        format!(
            "Qapsule is a live chart document whose numbers come only from a deterministic \
             engine; you speak as `{principal}` and read with the data rights of the person \
             you work for. Start with `whats_new` (what moved since your last look, the \
             person's holdings and watches first); use `find` or `instrument` to locate \
             capsules and `explain` to open one to its story, evidence and track record. \
             Refer to things by meaning (e.g. capsule {{\"instrument\":\"THYAO\",\"meaning\":\"breakout\",\"latest\":true}}) \
             and cite the ids the answers give. Never compute, round or reformat numbers: \
             quote them as printed, and where evidence says `requires`, say which data \
             license it needs instead of guessing. `comment` is checked by the engine and \
             returns machine-readable reasons to fix; `propose` only creates a proposal a \
             person must accept — you cannot accept or reject anything. Watch matches arrive \
             as notifications (`notifications` tool); everything you write is an \
             observation, never investment advice."
        )
    }
}

fn parse<T: for<'de> Deserialize<'de>>(tool: &str, args: Value) -> Result<T, RpcError> {
    serde_json::from_value(args).map_err(|e| RpcError::new(-32602, format!("tool `{tool}`: {e}")))
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

fn ok_text(text: String, structured: Value) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "structuredContent": structured,
        "isError": false,
    })
}

fn fail_text(text: String, structured: Value) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "structuredContent": structured,
        "isError": true,
    })
}

fn is_result(r: &Value) -> bool {
    r.get("type").and_then(Value::as_str) == Some("result")
}

/// A protocol `error` reply as a tool error: code, path, message, hint, `did_you_mean`
/// and (for a rejected comment) every reason, as text and as structured content.
fn protocol_error(tool: &str, reply: Value) -> Value {
    let e = reply.get("error").unwrap_or(&reply);
    let mut t = format!("{tool}: error `{}`", s(e, "code"));
    for k in ["reason", "path", "message"] {
        if let Some(v) = e.get(k).and_then(Value::as_str) {
            t.push_str(&format!(" · {k}: {v}"));
        }
    }
    if let Some(d) = e.get("did_you_mean").filter(|d| !d.is_null()) {
        t.push_str(&format!(" · did_you_mean: {d}"));
    }
    if let Some(x) = e.get("expected").filter(|d| !d.is_null()) {
        t.push_str(&format!(" · expected: {x}"));
    }
    if let Some(rs) = e.get("reasons").and_then(Value::as_array) {
        for r in rs {
            t.push_str(&format!("\n- {}/{}", s(r, "check"), s(r, "code")));
            if let Some(tok) = r.get("token").and_then(Value::as_str) {
                t.push_str(&format!(" `{tok}`"));
            }
            if let Some(n) = r.get("sentence") {
                t.push_str(&format!(" (sentence {n})"));
            }
            t.push_str(&format!(": {}", s(r, "hint")));
        }
    }
    if let Some(h) = e.get("hint").and_then(Value::as_str) {
        t.push_str(&format!("\nhint: {h}"));
    }
    fail_text(t, reply)
}

/// A string field (empty if absent).
fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

/// A field's JSON text as the protocol gave it (numbers unchanged), `-` if absent.
fn n(v: &Value, k: &str) -> String {
    match v.get(k) {
        None | Some(Value::Null) => "-".to_owned(),
        Some(Value::String(x)) => x.clone(),
        Some(x) => x.to_string(),
    }
}

fn limit_of(l: Option<u32>) -> usize {
    l.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT) as usize
}

/// The page of `items` after the id `after` (`Err`: not in the list).
fn page<'a>(
    items: &'a [Value],
    after: Option<&str>,
    limit: usize,
) -> Result<(&'a [Value], Option<String>), String> {
    let start = match after {
        None => 0,
        Some(a) => {
            items
                .iter()
                .position(|c| s(c, "id") == a)
                .ok_or_else(|| format!("`after` `{a}` is not in this list"))?
                + 1
        }
    };
    let rest = &items[start..];
    let take = rest.len().min(limit);
    let next = (rest.len() > take).then(|| s(&rest[take - 1], "id").to_owned());
    Ok((&rest[..take], next))
}

/// Text output of a capsule-giving tool: `card` (each capsule's facts line and sentence
/// together), `layered` (the summary lines; with `detail`, then the detail lines) and
/// `json` (the protocol data). Every line goes into `card` and into one of
/// `summary`/`detail` (a section heading may repeat in `detail`), so card and layered
/// with detail carry the same numbers.
struct Out {
    summary: Vec<String>,
    detail: Vec<String>,
    card: Vec<String>,
}

impl Out {
    fn new() -> Self {
        Self {
            summary: Vec::new(),
            detail: Vec::new(),
            card: Vec::new(),
        }
    }

    /// A summary line (also in the card).
    fn both(&mut self, line: String) {
        self.card.push(line.clone());
        self.summary.push(line);
    }

    /// A detail line (also in the card).
    fn deep(&mut self, line: String) {
        self.card.push(line.clone());
        self.detail.push(line);
    }

    fn render(
        self,
        format: Format,
        detail: bool,
        ids: Vec<String>,
        extra: Value,
        data: Value,
    ) -> Value {
        match format {
            Format::Json => ok_text(data.to_string(), data),
            Format::Card => {
                let lines = self.card;
                let mut sc = json!({"format": "card", "ids": ids});
                merge(&mut sc, extra);
                ok_text(lines.join("\n"), sc)
            }
            Format::Layered => {
                let mut lines = self.summary;
                if detail {
                    if !self.detail.is_empty() {
                        lines.push("detail:".to_owned());
                        lines.extend(self.detail);
                    }
                } else if !self.detail.is_empty() {
                    lines.push("(more with detail: true)".to_owned());
                }
                let mut sc = json!({"format": "layered", "ids": ids});
                merge(&mut sc, extra);
                ok_text(lines.join("\n"), sc)
            }
        }
    }
}

fn merge(into: &mut Value, extra: Value) {
    if let (Some(o), Value::Object(e)) = (into.as_object_mut(), extra) {
        for (k, v) in e {
            if !v.is_null() {
                o.insert(k, v);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Capsule lines
// ---------------------------------------------------------------------------

fn sentence(item: &Value, lang: Option<Language>) -> String {
    let sen = item.get("sentence").unwrap_or(&Value::Null);
    let k = match lang {
        None => "text",
        Some(Language::Tr) => "tr",
        Some(Language::En) => "en",
    };
    s(sen, k).to_owned()
}

/// One capsule (`CapsuleItem`): the facts line (ids and numbers).
fn item_line(c: &Value) -> String {
    let lab = c.get("ledger_at_birth").unwrap_or(&Value::Null);
    let mut t = format!(
        "{} · {} {} {} · {} ({}) · tier {} · confirmations {} · born bar {} · last bar {} · t {} · record at birth {} of {} realized · finding {}",
        s(c, "id"),
        s(c, "instrument"),
        s(c, "kind"),
        s(c, "direction"),
        s(c, "stage"),
        s(c, "validity"),
        n(c, "tier"),
        n(c, "confirmations"),
        n(c, "born_bar"),
        n(c, "last_bar"),
        n(c, "t"),
        n(lab, "realized"),
        n(lab, "n"),
        s(c, "finding"),
    );
    for (flag, word) in [
        ("relevant", "holdings/watch"),
        ("redacted", "redacted (no data license)"),
        ("coin_flip", "coin flip"),
    ] {
        if c.get(flag).and_then(Value::as_bool) == Some(true) {
            t.push_str(&format!(" · {word}"));
        }
    }
    if s(c, "trust") == "server" {
        t.push_str(" · server-verified");
    }
    t
}

/// A capsule into `out`: card = facts line + sentence; layered summary = id + stage +
/// sentence, detail = facts line.
fn push_item(out: &mut Out, c: &Value, lang: Option<Language>, prefix: &str) {
    let sen = sentence(c, lang);
    let facts = format!("{prefix}{}", item_line(c));
    out.summary
        .push(format!("{prefix}{}: {} — {sen}", s(c, "id"), s(c, "stage")));
    out.card.push(facts.clone());
    out.card.push(format!("  {sen}"));
    out.detail.push(facts);
}

fn server_comments(out: &mut Out, comments: Option<&Value>, lang: Option<Language>) {
    let Some(cs) = comments.and_then(Value::as_array).filter(|c| !c.is_empty()) else {
        return;
    };
    out.both("server comments (licensed data you are not entitled to; no figures):".to_owned());
    let k = if lang == Some(Language::En) {
        "en"
    } else {
        "tr"
    };
    for c in cs {
        out.both(format!(
            "- {} bar {} t {}: {}",
            s(c, "instrument"),
            n(c, "bar"),
            n(c, "t"),
            s(c, k)
        ));
    }
}

fn is_closed(c: &Value) -> bool {
    matches!(
        s(c, "stage"),
        "realized" | "invalidated" | "expired" | "retracted"
    )
}

// ---------------------------------------------------------------------------
// whats_new
// ---------------------------------------------------------------------------

impl Task {
    fn whats_new(&mut self, ctx: &mut Ctx<'_>, a: &WhatsNewArgs) -> Result<Value, RpcError> {
        let mut q = json!({"kind": "capsules"});
        if a.feed {
            q["audience"] = json!(true);
        }
        if let Some(t) = a.since_t {
            q["since_t"] = json!(t);
        }
        let r = ctx.query(q);
        if !is_result(&r) {
            return Ok(protocol_error("whats_new", r));
        }
        let caps: Vec<Value> = r
            .pointer("/data/capsules")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let now = n(&r, "now");
        // What counts as new.
        let (basis, picked): (String, Vec<(&str, &Value)>) = if let Some(t) = a.since_t {
            (
                format!("since t {t}"),
                caps.iter().map(|c| (by_birth(c), c)).collect(),
            )
        } else if let Some(seen) = &self.seen {
            let mut v = Vec::new();
            for c in &caps {
                let key = (
                    s(c, "stage").to_owned(),
                    c["last_bar"].as_u64().unwrap_or(0),
                );
                match seen.get(s(c, "id")) {
                    None => v.push((if is_closed(c) { "closed" } else { "born" }, c)),
                    Some(k) if *k != key => {
                        v.push((if is_closed(c) { "closed" } else { "changed" }, c));
                    }
                    Some(_) => {}
                }
            }
            ("since your last look".to_owned(), v)
        } else {
            // First look: the latest moves (the greatest transition time).
            let latest = caps.iter().filter_map(|c| c["t"].as_i64()).max();
            let v = caps
                .iter()
                .filter(|c| c["t"].as_i64() == latest && latest.is_some())
                .map(|c| (by_birth(c), c))
                .collect();
            let at = caps
                .iter()
                .find(|c| c["t"].as_i64() == latest)
                .map_or_else(|| "-".to_owned(), |c| n(c, "t"));
            (format!("first look: the latest moves (t {at})"), v)
        };
        if a.since_t.is_none() {
            self.seen = Some(
                caps.iter()
                    .map(|c| {
                        (
                            s(c, "id").to_owned(),
                            (
                                s(c, "stage").to_owned(),
                                c["last_bar"].as_u64().unwrap_or(0),
                            ),
                        )
                    })
                    .collect(),
            );
        }
        // Holdings and watches first (stable: the engine's order otherwise).
        let mut ordered: Vec<(&str, &Value)> = picked
            .iter()
            .filter(|(_, c)| c["relevant"].as_bool() == Some(true))
            .copied()
            .collect();
        ordered.extend(
            picked
                .iter()
                .filter(|(_, c)| c["relevant"].as_bool() != Some(true))
                .copied(),
        );
        let limit = limit_of(a.limit);
        let more = ordered.len() > limit;
        ordered.truncate(limit);

        let mut out = Out::new();
        out.both(format!("whats_new · now {now} · {basis}"));
        let mut ids = Vec::new();
        for cat in ["born", "changed", "closed"] {
            let these: Vec<&Value> = ordered
                .iter()
                .filter(|(k, _)| *k == cat)
                .map(|(_, c)| *c)
                .collect();
            if these.is_empty() {
                continue;
            }
            out.both(format!("[{cat}]"));
            out.detail.push(format!("[{cat}]"));
            for c in these {
                push_item(&mut out, c, a.lang, "- ");
                ids.push(s(c, "id").to_owned());
            }
        }
        if ordered.is_empty() {
            out.both("No capsule moved.".to_owned());
        }
        if more {
            out.both("(more moved: raise `limit` or use `find` with `since_t`)".to_owned());
        }
        // Watches: state or match count changed since the last look.
        let watches = self.watch_changes(ctx);
        if !watches.is_empty() {
            out.both("[watches]".to_owned());
            for w in watches {
                out.both(w);
            }
        }
        server_comments(&mut out, r.pointer("/data/comments"), a.lang);
        let data = r.get("data").cloned().unwrap_or(Value::Null);
        let data = if a.format == Format::Json {
            let keep: Vec<Value> = ordered.iter().map(|(_, c)| (*c).clone()).collect();
            let mut d = data;
            d["capsules"] = json!(keep);
            d["now"] = r.get("now").cloned().unwrap_or(Value::Null);
            d
        } else {
            data
        };
        Ok(out.render(a.format, a.detail, ids, json!({"now": r.get("now")}), data))
    }

    /// Lines for watches whose state or match count changed since the last look.
    fn watch_changes(&mut self, ctx: &mut Ctx<'_>) -> Vec<String> {
        let r = ctx.query(json!({"kind": "list", "collection": "intents"}));
        let mut lines = Vec::new();
        let Some(items) = r.pointer("/data/items").and_then(Value::as_array) else {
            return lines;
        };
        for w in items.iter().filter(|w| w.get("watch").is_some()) {
            let id = s(w, "id").to_owned();
            let now = (n(w, "state"), n(w, "matches"));
            if self.seen_watches.get(&id) != Some(&now) {
                lines.push(format!(
                    "- {id} · state {} · matches {} · \"{}\"",
                    now.0,
                    now.1,
                    s(w, "text")
                ));
                self.seen_watches.insert(id, now);
            }
        }
        lines
    }

    fn watch(&mut self, ctx: &mut Ctx<'_>, a: WatchArgs) -> Result<Value, RpcError> {
        match a.action {
            WatchAction::List => {
                let r = ctx.query(json!({"kind": "list", "collection": "intents"}));
                if !is_result(&r) {
                    return Ok(protocol_error("watch", r));
                }
                let items: Vec<Value> = r
                    .pointer("/data/items")
                    .and_then(Value::as_array)
                    .map(|v| {
                        v.iter()
                            .filter(|w| w.get("watch").is_some())
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                let mut lines = vec!["your watches:".to_owned()];
                for w in &items {
                    lines.push(watch_line(w));
                }
                if items.is_empty() {
                    lines.push("(none)".to_owned());
                }
                let ids: Vec<&str> = items.iter().map(|w| s(w, "id")).collect();
                Ok(ok_text(
                    lines.join("\n"),
                    json!({"ids": ids, "watches": items}),
                ))
            }
            WatchAction::Remove => {
                let Some(intent) = a.intent else {
                    return Err(RpcError::new(
                        -32602,
                        "tool `watch`: `remove` needs `intent` (a watch id from `watch` list)",
                    ));
                };
                let r = ctx.command(vec![json!({"op": "remove_watch", "intent": intent})]);
                if r.get("type").and_then(Value::as_str) != Some("ack") {
                    return Ok(protocol_error("watch", r));
                }
                Ok(ok_text(
                    format!("watch {intent} removed (seq {})", n(&r, "seq")),
                    r,
                ))
            }
            WatchAction::Add => {
                let (Some(text), Some(m)) = (a.text, a.match_) else {
                    return Err(RpcError::new(
                        -32602,
                        "tool `watch`: `add` needs `text` and `match`, e.g. \
                         {\"text\":\"…\",\"match\":{\"instrument\":\"THYAO\",\"negative\":true}}",
                    ));
                };
                let mut op = json!({"op": "add_watch", "text": text, "match": m});
                if let Some(t) = a.expires_t {
                    op["expires_t"] = json!(t);
                }
                if let Some(x) = a.max_matches {
                    op["max_matches"] = json!(x);
                }
                let r = ctx.command(vec![op]);
                if r.get("type").and_then(Value::as_str) != Some("ack") {
                    return Ok(protocol_error("watch", r));
                }
                // Matches come as `intent_matched` notifications: subscribe once.
                if !self.subscribed {
                    let id = ctx.request_id();
                    let s = ctx.send(
                        &json!({"type": "subscribe", "id": id, "topic": {"kind": "intents"}}),
                    );
                    self.subscribed = s.get("type").and_then(Value::as_str) == Some("ack");
                }
                let id = r
                    .pointer("/created/0")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                Ok(ok_text(
                    format!(
                        "watch {id} added (seq {}); matches arrive as `intent_matched` \
                         notifications (`notifications` tool) and in `whats_new`",
                        n(&r, "seq")
                    ),
                    r,
                ))
            }
        }
    }
}

fn by_birth(c: &Value) -> &'static str {
    if is_closed(c) {
        "closed"
    } else if c["born_bar"] == c["last_bar"] {
        "born"
    } else {
        "changed"
    }
}

fn watch_line(w: &Value) -> String {
    let watch = w.get("watch").unwrap_or(&Value::Null);
    format!(
        "- {} · state {} · matches {} · for {} · match {} · expires_t {} · max_matches {} · \"{}\"",
        s(w, "id"),
        n(w, "state"),
        n(w, "matches"),
        s(watch, "for"),
        watch.get("match").unwrap_or(&Value::Null),
        n(watch, "expires_t"),
        n(watch, "max_matches"),
        s(w, "text"),
    )
}

// ---------------------------------------------------------------------------
// find
// ---------------------------------------------------------------------------

fn find(ctx: &mut Ctx<'_>, a: &FindArgs) -> Value {
    let mut q = json!({"kind": "capsules"});
    if let Some(i) = &a.instrument {
        q["instrument"] = json!(i);
    }
    if let Some(m) = a.meaning {
        q["meaning"] = json!(m);
    }
    if !a.stages.is_empty() {
        q["stages"] = json!(a.stages);
    }
    if let Some(o) = a.open {
        q["open"] = json!(o);
    }
    if let Some(t) = a.since_t {
        q["since_t"] = json!(t);
    }
    if let Some(t) = a.born_since_t {
        q["born_since_t"] = json!(t);
    }
    if a.feed {
        q["audience"] = json!(true);
    }
    // `latest` with a direction: the latest of that direction, so it is applied here.
    if a.latest && a.direction.is_none() {
        q["latest"] = json!(true);
    }
    let r = ctx.query(q);
    if !is_result(&r) {
        return protocol_error("find", r);
    }
    let mut caps: Vec<Value> = r
        .pointer("/data/capsules")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(d) = a.direction {
        let d = json!(d);
        caps.retain(|c| c.get("direction") == Some(&d));
        if a.latest {
            caps = latest_of(ctx, &caps);
        }
    }
    let (rows, next) = match page(&caps, a.after.as_deref(), limit_of(a.limit)) {
        Ok(p) => p,
        Err(e) => return fail_text(format!("find: {e}"), json!({"error": e})),
    };
    let mut out = Out::new();
    out.both(format!(
        "find · {} match(es){}",
        caps.len(),
        next.as_ref()
            .map_or(String::new(), |x| format!(" · next: {x}"))
    ));
    for c in rows {
        push_item(&mut out, c, a.lang, "- ");
    }
    if caps.is_empty() {
        out.both("No capsule matches.".to_owned());
    }
    server_comments(&mut out, r.pointer("/data/comments"), a.lang);
    let ids: Vec<String> = rows.iter().map(|c| s(c, "id").to_owned()).collect();
    let mut data = r.get("data").cloned().unwrap_or(Value::Null);
    data["capsules"] = json!(rows);
    data["total"] = json!(caps.len());
    if let Some(x) = &next {
        data["next"] = json!(x);
    }
    out.render(a.format, a.detail, ids, json!({"next": next}), data)
}

/// Of `caps`, the latest born by the protocol's `latest` rule (§20.2: greatest birth
/// time, then birth bar, then id). Birth times come from the authority's `stories` rows
/// of the candidates' instruments (only compared, never computed).
fn latest_of(ctx: &mut Ctx<'_>, caps: &[Value]) -> Vec<Value> {
    let mut born_t: BTreeMap<String, i64> = BTreeMap::new();
    let mut instruments: Vec<&str> = caps.iter().map(|c| s(c, "instrument")).collect();
    instruments.sort_unstable();
    instruments.dedup();
    for inst in instruments {
        let mut after: Option<String> = None;
        loop {
            let mut q = json!({"kind": "stories", "instrument": inst, "limit": 500});
            if let Some(a) = &after {
                q["after"] = json!(a);
            }
            let r = ctx.query(q);
            for row in r
                .pointer("/data/stories")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(t) = row["born_t"].as_i64() {
                    born_t.insert(s(row, "id").to_owned(), t);
                }
            }
            match r.pointer("/data/next").and_then(Value::as_str) {
                Some(n) => after = Some(n.to_owned()),
                None => break,
            }
        }
    }
    caps.iter()
        .max_by(|a, b| {
            let key = |c: &Value| {
                (
                    born_t.get(s(c, "id")).copied().unwrap_or(i64::MIN),
                    c["born_bar"].as_u64().unwrap_or(0),
                    s(c, "id").to_owned(),
                )
            };
            key(a).cmp(&key(b))
        })
        .cloned()
        .into_iter()
        .collect()
}

// ---------------------------------------------------------------------------
// explain
// ---------------------------------------------------------------------------

fn explain(ctx: &mut Ctx<'_>, a: &ExplainArgs) -> Value {
    let r = ctx.query(json!({"kind": "capsule", "capsule": a.capsule}));
    if !is_result(&r) {
        return protocol_error("explain", r);
    }
    let d = r.pointer("/data/detail").cloned().unwrap_or(Value::Null);
    let item = d.get("item").unwrap_or(&Value::Null);
    let mut out = Out::new();
    out.both(item_line(item));
    out.both(format!("sentence: {}", sentence(item, a.lang)));
    let story = d.get("story").unwrap_or(&Value::Null);
    let ledger = d.get("ledger").unwrap_or(&Value::Null);
    out.both(format!(
        "track record of {} now: {} of {} realized",
        s(story, "ledger_key"),
        n(ledger, "realized"),
        n(ledger, "n")
    ));
    // Story.
    out.deep(format!(
        "story: theme {} · {} · horizon {} bars · resolution {} · condition {}",
        s(story, "theme"),
        s(story, "time"),
        n(story, "horizon_bars"),
        story
            .pointer("/resolution/rule")
            .and_then(Value::as_str)
            .unwrap_or("-"),
        story
            .get("condition")
            .map_or("-".to_owned(), Value::to_string),
    ));
    for h in story
        .get("history")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let facts: Vec<&str> = h
            .get("facts")
            .and_then(Value::as_array)
            .map(|f| f.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        out.deep(format!(
            "- bar {} t {} {} ({}){}",
            n(h, "bar"),
            n(h, "t"),
            s(h, "stage"),
            s(h, "reason"),
            if facts.is_empty() {
                String::new()
            } else {
                format!(" facts {}", facts.join(", "))
            }
        ));
    }
    // Evidence.
    out.deep("evidence:".to_owned());
    for e in d
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let head = format!(
            "- {} ({}, {})",
            s(e, "fact"),
            s(e, "family"),
            s(e, "license")
        );
        if e.get("open").and_then(Value::as_bool) == Some(true) {
            let vals: Vec<String> = e
                .get("values")
                .and_then(Value::as_object)
                .map(|m| m.iter().map(|(k, v)| format!("{k}={v}")).collect())
                .unwrap_or_default();
            let mut line = format!("{head}: {}", vals.join(" "));
            if let Some(er) = e.get("evidence_ref") {
                line.push_str(&format!(
                    " · frozen {} {} over {} bars, t {} to {}",
                    s(er, "alg"),
                    s(er, "digest"),
                    n(er, "bars"),
                    n(er, "from_t"),
                    n(er, "to_t")
                ));
            }
            line.push_str(&format!(" · rests on {}", s(e, "rests_on")));
            out.deep(line);
        } else {
            out.deep(format!(
                "{head}: values not shown to you — requires data license `{}` · rests on {}",
                n(e, "requires"),
                s(e, "rests_on")
            ));
        }
    }
    // Comments.
    if let Some(cs) = d
        .get("comments")
        .and_then(Value::as_array)
        .filter(|c| !c.is_empty())
    {
        out.deep("comments:".to_owned());
        for c in cs {
            let author = c
                .pointer("/provenance/author/name")
                .and_then(Value::as_str)
                .unwrap_or("-");
            let kind = c
                .pointer("/provenance/author/kind")
                .and_then(Value::as_str)
                .unwrap_or("-");
            let behalf = c
                .get("on_behalf")
                .and_then(Value::as_str)
                .map_or(String::new(), |b| format!(" for {b}"));
            out.deep(format!(
                "- {} ({kind} {author}{behalf}; cites {}): {}",
                s(c, "id"),
                c.get("cites").unwrap_or(&Value::Null),
                s(c, "text")
            ));
        }
    }
    if let Some(w) = d.get("withheld") {
        out.deep(format!(
            "withheld: newer licensed moves you are not entitled to; requires {}",
            w.get("requires").unwrap_or(&Value::Null)
        ));
        server_comments(&mut out, w.get("comments"), a.lang);
    }
    let mut ids = vec![s(item, "id").to_owned(), s(item, "finding").to_owned()];
    for e in d
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        ids.push(s(e, "fact").to_owned());
    }
    out.render(
        a.format,
        a.detail,
        ids,
        json!({"resolved": r.get("resolved")}),
        r.get("data").cloned().unwrap_or(Value::Null),
    )
}

// ---------------------------------------------------------------------------
// instrument
// ---------------------------------------------------------------------------

fn instrument(ctx: &mut Ctx<'_>, a: &InstrumentArgs) -> Value {
    let Some(sym) = &a.symbol else {
        let r = ctx.query(json!({"kind": "list", "collection": "series"}));
        if !is_result(&r) {
            return protocol_error("instrument", r);
        }
        let items = r
            .pointer("/data/items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut lines = vec!["instruments (symbol: bar series, interval):".to_owned()];
        let mut syms = Vec::new();
        for it in &items {
            if let Some(sy) = it.pointer("/instrument/symbol").and_then(Value::as_str) {
                lines.push(format!("- {sy}: {} {}", s(it, "id"), s(it, "interval")));
                syms.push(sy.to_owned());
            }
        }
        return ok_text(lines.join("\n"), json!({"instruments": syms}));
    };
    let mut q = json!({"kind": "stories", "instrument": sym, "limit": limit_of(a.limit)});
    if let Some(l) = a.last {
        q["range"] = json!({"last": l});
    }
    if let Some(x) = &a.after {
        q["after"] = json!(x);
    }
    let r = ctx.query(q);
    if !is_result(&r) {
        return protocol_error("instrument", r);
    }
    let d = r.get("data").cloned().unwrap_or(Value::Null);
    let mut out = Out::new();
    out.both(format!(
        "{} (series {}) · stories {} in range{} · now {}",
        s(&d, "instrument"),
        s(&d, "series"),
        n(&d, "total"),
        d.get("next")
            .and_then(Value::as_str)
            .map_or(String::new(), |x| format!(" · next: {x}")),
        n(&r, "now"),
    ));
    // The session clock (replay position), when the session is a replay.
    let id = ctx.request_id();
    let st = ctx.send(&json!({"type": "replay", "id": id, "control": {"kind": "status"}}));
    let clock = st.pointer("/data/replay").cloned();
    if let Some(c) = &clock {
        out.both(format!(
            "clock: replay t {} · step {} of {} · {}{}",
            n(c, "t"),
            n(c, "step"),
            n(c, "steps"),
            if c["paused"].as_bool() == Some(true) {
                "paused"
            } else {
                "running"
            },
            if c["done"].as_bool() == Some(true) {
                " · done"
            } else {
                ""
            }
        ));
    }
    let rows = d
        .get("stories")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in &rows {
        let f = row.get("face").unwrap_or(&Value::Null);
        out.both(format!(
            "- {}: {} {} — {}, {}",
            s(row, "id"),
            s(f, "event"),
            s(f, "when"),
            s(f, "validity_phrase"),
            s(f, "ledger_short")
        ));
        let mut line = format!(
            "- {} · {} {} · {} · validity {} · tier {} · born bar {} t {} · last bar {} t {}",
            s(row, "id"),
            s(row, "kind"),
            s(row, "direction"),
            s(row, "stage"),
            s(row, "validity"),
            n(row, "tier"),
            n(row, "born_bar"),
            n(row, "born_t"),
            n(row, "last_bar"),
            n(row, "last_t"),
        );
        if row.get("closed_bar").is_some() {
            line.push_str(&format!(
                " · closed bar {} t {} · outcome {}",
                n(row, "closed_bar"),
                n(row, "closed_t"),
                n(row, "outcome")
            ));
        }
        if row["in_feed"].as_bool() == Some(true) {
            line.push_str(" · in feed");
        } else {
            line.push_str(&format!(" · not in feed ({})", n(row, "hidden")));
        }
        if row["coin_flip"].as_bool() == Some(true) {
            line.push_str(" · coin flip");
        }
        if row["redacted"].as_bool() == Some(true) {
            line.push_str(" · redacted (no data license)");
        }
        if f.get("ledger_n").is_some() {
            line.push_str(&format!(" · record n {}", n(f, "ledger_n")));
        }
        line.push_str(&format!(" · finding {}", s(row, "finding")));
        out.deep(line);
    }
    if rows.is_empty() {
        out.both("No story in range.".to_owned());
    }
    server_comments(&mut out, d.get("comments"), None);
    let ids: Vec<String> = rows.iter().map(|x| s(x, "id").to_owned()).collect();
    let mut data = d;
    if let Some(c) = clock {
        data["clock"] = c;
    }
    let next = data.get("next").cloned();
    out.render(a.format, a.detail, ids, json!({"next": next}), data)
}

// ---------------------------------------------------------------------------
// track_record
// ---------------------------------------------------------------------------

fn track_record(ctx: &mut Ctx<'_>, a: &TrackRecordArgs) -> Value {
    let mut q = json!({"kind": "ledger"});
    if let Some(m) = a.meaning {
        q["meaning"] = json!(m);
    }
    if let Some(d) = a.direction {
        q["direction"] = json!(d);
    }
    if let Some(k) = &a.key {
        q["key"] = json!(k);
    }
    if a.by_kind {
        q["by_kind"] = json!(true);
    }
    let r = ctx.query(q);
    if !is_result(&r) {
        return protocol_error("track_record", r);
    }
    let d = r.get("data").cloned().unwrap_or(Value::Null);
    let total = d.get("total").unwrap_or(&Value::Null);
    let mut lines = vec![format!(
        "track record (closed capsules; retracted never count): total {} of {} realized",
        n(total, "realized"),
        n(total, "n")
    )];
    for row in d
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let dir = row
            .get("direction")
            .and_then(Value::as_str)
            .map_or(String::new(), |x| format!(" ({x})"));
        lines.push(format!(
            "- {}{dir}: {} of {} realized",
            s(row, "key"),
            n(row, "realized"),
            n(row, "n")
        ));
    }
    ok_text(lines.join("\n"), d)
}

// ---------------------------------------------------------------------------
// comment
// ---------------------------------------------------------------------------

fn comment(ctx: &mut Ctx<'_>, a: CommentArgs) -> Value {
    let about = match (&a.capsule, &a.instrument) {
        (Some(CapsuleRef::Id(id)), None) => json!({"capsule": id}),
        (Some(sel @ CapsuleRef::Select(_)), None) => {
            // Resolve the selector as the reader sees it (the authority's rule).
            let r = ctx.query(json!({"kind": "capsule", "capsule": sel}));
            if !is_result(&r) {
                return protocol_error("comment", r);
            }
            let id = r
                .pointer("/data/detail/item/id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            json!({"capsule": id})
        }
        (None, Some(i)) => json!({"instrument": i}),
        _ => {
            return fail_text(
                "comment: give exactly one of `capsule` and `instrument`".to_owned(),
                json!({"error": {"code": "invalid", "path": "/capsule"}}),
            )
        }
    };
    let mut op = json!({"op": "add_comment", "about": about, "lang": a.lang,
        "text": a.text, "cites": a.cites});
    if !a.external.is_empty() {
        op["external"] = json!(a.external);
    }
    let r = ctx.command(vec![op]);
    if r.get("type").and_then(Value::as_str) != Some("ack") {
        return protocol_error("comment", r);
    }
    let id = r
        .pointer("/created/0")
        .and_then(Value::as_str)
        .unwrap_or("-")
        .to_owned();
    ok_text(
        format!(
            "comment {id} accepted and stored (seq {}), about {about}",
            n(&r, "seq")
        ),
        r,
    )
}

// ---------------------------------------------------------------------------
// propose
// ---------------------------------------------------------------------------

fn propose(ctx: &mut Ctx<'_>, a: ProposeArgs) -> Value {
    if a.ops.is_empty() {
        return fail_text(
            "propose: `ops` is empty".to_owned(),
            json!({"error": {"code": "invalid", "path": "/ops"}}),
        );
    }
    let id = ctx.request_id();
    let mut cmd = json!({"type": "command", "id": id, "mode": "propose", "ops": a.ops});
    if let Some(r) = a.reason {
        cmd["reason"] = json!(r);
    }
    let r = ctx.send(&cmd);
    if r.get("type").and_then(Value::as_str) != Some("ack") {
        return protocol_error("propose", r);
    }
    let Some(pid) = r.get("proposal").and_then(Value::as_str).map(str::to_owned) else {
        return ok_text(
            format!(
                "propose: applied without a proposal (status {})",
                n(&r, "status")
            ),
            r,
        );
    };
    let mut lines = vec![format!(
        "proposal {pid} is {} (seq {}); a person accepts or rejects it — you cannot",
        n(&r, "status"),
        n(&r, "seq")
    )];
    let p = ctx.query(json!({"kind": "preview", "proposal": pid}));
    let preview = if is_result(&p) {
        let d = p.get("data").cloned().unwrap_or(Value::Null);
        let ops: Vec<&str> = d
            .get("changes")
            .and_then(Value::as_array)
            .map(|c| c.iter().map(|x| s(x, "op")).collect())
            .unwrap_or_default();
        lines.push(format!("preview: changes [{}]", ops.join(", ")));
        for v in d
            .get("views")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            lines.push(format!(
                "- view {}{}: {}",
                s(v, "view"),
                if v["new"].as_bool() == Some(true) {
                    " (new)"
                } else {
                    ""
                },
                s(v, "explain")
            ));
            lines.push(format!(
                "  see it: render {{\"view\":\"{}\",\"proposal\":\"{pid}\"}}",
                s(v, "view")
            ));
        }
        d
    } else {
        lines.push(format!(
            "preview unavailable: {}",
            p.get("error").unwrap_or(&p)
        ));
        Value::Null
    };
    ok_text(lines.join("\n"), json!({"ack": r, "preview": preview}))
}

// ---------------------------------------------------------------------------
// glossary
// ---------------------------------------------------------------------------

/// The variants of a unit enum with their descriptions, from its generated schema.
fn variants<T: schemars::JsonSchema>() -> Vec<(String, String)> {
    let v = serde_json::to_value(schemars::schema_for!(T)).unwrap_or(Value::Null);
    let mut out = Vec::new();
    for alt in v
        .get("oneOf")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let desc = s(alt, "description").replace('\n', " ");
        if let Some(c) = alt.get("const").and_then(Value::as_str) {
            out.push((c.to_owned(), desc));
        } else if let Some(es) = alt.get("enum").and_then(Value::as_array) {
            for e in es.iter().filter_map(Value::as_str) {
                out.push((e.to_owned(), desc.clone()));
            }
        }
    }
    if out.is_empty() {
        for e in v
            .get("enum")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            out.push((e.to_owned(), String::new()));
        }
    }
    out
}

fn glossary(a: &GlossaryArgs) -> Value {
    let groups: [(&str, Vec<(String, String)>); 7] = [
        ("meaning", variants::<MeaningKind>()),
        ("direction", variants::<Direction>()),
        ("stage", variants::<Stage>()),
        ("validity", variants::<Validity>()),
        ("time", variants::<TimeKind>()),
        ("trust", variants::<Trust>()),
        ("evidence family", variants::<EvidenceFamily>()),
    ];
    let term = a.term.as_deref().map(str::to_lowercase);
    let mut lines = Vec::new();
    let mut entries = Vec::new();
    for (g, vs) in &groups {
        for (name, desc) in vs {
            let hit = term.as_ref().is_none_or(|t| {
                name.to_lowercase().contains(t.as_str())
                    || desc.to_lowercase().contains(t.as_str())
                    || g.contains(t.as_str())
            });
            if hit {
                lines.push(format!("{g} `{name}`: {desc}"));
                entries.push(json!({"group": g, "name": name, "text": desc}));
            }
        }
    }
    if lines.is_empty() {
        lines.push("No entry matches; call without `term` for the whole list.".to_owned());
    }
    ok_text(lines.join("\n"), json!({"entries": entries}))
}
