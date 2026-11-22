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

mod args;
mod explain;
mod find;
mod glossary;
mod instrument;
mod text;
mod watch;
mod write;

pub use args::{
    CommentArgs, ExplainArgs, FindArgs, Format, GlossaryArgs, InstrumentArgs, ProposeArgs,
    TrackRecordArgs, WatchAction, WatchArgs, WhatsNewArgs,
};
use explain::*;
use find::*;
use glossary::*;
use instrument::*;
use text::*;
use watch::*;
use write::*;

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
