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

mod args;
mod schema;
mod text;

pub use args::{
    CapsuleArgs, FindArgs, Format, OverviewArgs, QueryArgs, WatchAction, WatchArgs, WriteArgs,
    WriteKind,
};
use schema::*;
use text::*;
pub(crate) use text::{capsule_card, capsule_layered, item_line, item_summary};

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
// Tools
// ---------------------------------------------------------------------------

impl Mixed {
    #[allow(clippy::too_many_lines)]
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

    #[allow(clippy::too_many_lines)]
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
