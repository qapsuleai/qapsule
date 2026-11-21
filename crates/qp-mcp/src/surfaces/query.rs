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

/// The query surface.
#[derive(Debug, Default)]
pub struct Query;

/// Registry constructor.
#[must_use]
pub fn new() -> Box<dyn ToolSurface> {
    Box::new(Query)
}

// ---------------------------------------------------------------------------------------
// Arguments (the input schemas are generated from these types).

/// What `ask` finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Find {
    /// Engine stories (capsules) you may see, most important first.
    Capsules,
    /// One instrument's story list, in birth order.
    Stories,
    /// Engine facts.
    Facts,
    /// An instrument: series, data state, last bar, capsule counts, session phase.
    Instrument,
    /// The track record (closed and realized per key).
    Ledger,
    /// Market status (open, phase, delayed, stale).
    Market,
    /// Terms used in capsule texts and what they mean.
    Glossary,
    /// What changed: log entries since a position or time.
    Changes,
    /// Validated comments.
    Comments,
    /// Your person's watches.
    Watches,
    /// Proposals.
    Proposals,
}

/// The answer shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    /// Short text card per item, with ids and numbers.
    #[default]
    Card,
    /// The full protocol objects.
    Json,
    /// Compact summary per item; evidence only with `expand`.
    Layered,
}

/// A related part of a capsule or story.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Expand {
    /// Transitions (bar, stage, reason, facts).
    Story,
    /// Per fact evidence (values if you are entitled, else what it requires).
    Evidence,
    /// The ledger of the capsule's key.
    Ledger,
    /// Validated comments on it.
    Comments,
}

/// One value or a list.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    /// One.
    One(T),
    /// Several.
    Many(Vec<T>),
}

impl<T: Clone> OneOrMany<T> {
    fn to_vec(&self) -> Vec<T> {
        match self {
            Self::One(t) => vec![t.clone()],
            Self::Many(v) => v.clone(),
        }
    }
}

/// `where`: filters, AND-ed. Which keys apply depends on `find`.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Where {
    /// An item id (capsule or finding, fact, comment, watch, proposal).
    #[serde(default)]
    pub id: Option<String>,
    /// A capsule by id or by meaning (selector).
    #[serde(default)]
    pub capsule: Option<CapsuleRef>,
    /// Instrument symbol, e.g. `THYAO`.
    #[serde(default)]
    pub instrument: Option<String>,
    /// Meaning kind.
    #[serde(default)]
    pub meaning: Option<MeaningKind>,
    /// Direction.
    #[serde(default)]
    pub direction: Option<Direction>,
    /// Story stage(s).
    #[serde(default)]
    pub stage: Option<OneOrMany<Stage>>,
    /// `true` open stories only, `false` closed only.
    #[serde(default)]
    pub open: Option<bool>,
    /// Only what concerns your person (a position or an active watch).
    #[serde(default)]
    pub holdings: Option<bool>,
    /// Only the most recently born match.
    #[serde(default)]
    pub latest: Option<bool>,
    /// Your person's feed filter (watch list, sensitivity).
    #[serde(default)]
    pub audience: Option<bool>,
    /// Moved (capsules, changes) or happened (facts) at or after this bar.
    #[serde(default)]
    pub since_bar: Option<u64>,
    /// At or after this time (epoch ms).
    #[serde(default)]
    pub since_t: Option<u64>,
    /// Log entries after this `seq` (an answer's `seq`).
    #[serde(default)]
    pub after_seq: Option<u64>,
    /// Log entries of the last this-many ms.
    #[serde(default)]
    pub within_ms: Option<u64>,
    /// Log entries by this principal.
    #[serde(default)]
    pub author: Option<String>,
    /// Proposal state (`pending` …) or watch state (`active`, `matched`, `expired`,
    /// `removed`).
    #[serde(default)]
    pub state: Option<String>,
    /// Exact fact kind, e.g. `breakout.range.up`.
    #[serde(default)]
    pub kind: Option<String>,
    /// Ledger key.
    #[serde(default)]
    pub key: Option<String>,
    /// Ledger detector.
    #[serde(default)]
    pub detector: Option<String>,
    /// Ledger rows per kind.
    #[serde(default)]
    pub by_kind: Option<bool>,
    /// Bars of the instrument a story's life must meet.
    #[serde(default)]
    pub range: Option<Range>,
    /// The page after this capsule id (a stories answer's `next`).
    #[serde(default)]
    pub after: Option<String>,
}

/// `ask` arguments.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AskArgs {
    /// What to find.
    pub find: Find,
    /// Filters (AND); keys per `find` in the tool description.
    #[serde(default, rename = "where")]
    pub where_: Where,
    /// Keep only these fields of each item (`id` is always kept).
    #[serde(default)]
    pub select: Vec<String>,
    /// Parts to attach to each capsule or story.
    #[serde(default)]
    pub expand: Vec<Expand>,
    /// At most this many items (default 10; facts, changes, comments 20; max 100).
    #[serde(default)]
    pub limit: Option<u32>,
    /// `card` (default), `json` or `layered`.
    #[serde(default)]
    pub format: Option<Format>,
}

/// What `act` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Do {
    /// Write a validated comment on a capsule or an instrument.
    Comment,
    /// Leave a watch for your person.
    Watch,
    /// Remove a watch.
    Unwatch,
    /// Propose view or drawing operations (a person decides).
    Propose,
    /// What a pending proposal would change (nothing is written).
    Preview,
}

/// `act` arguments.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActArgs {
    /// What to do.
    #[serde(rename = "do")]
    pub do_: Do,
    /// comment: the capsule it is about (id or selector).
    #[serde(default)]
    pub capsule: Option<CapsuleRef>,
    /// comment: the instrument it is about (instead of `capsule`).
    #[serde(default)]
    pub instrument: Option<String>,
    /// comment, watch: the text.
    #[serde(default)]
    pub text: Option<String>,
    /// comment: language of `text` (default `tr`).
    #[serde(default)]
    pub lang: Option<Language>,
    /// comment: ids (or capsule selectors) it rests on; `[n]` in the text cites the n-th.
    #[serde(default)]
    pub cites: Vec<CapsuleRef>,
    /// comment: named external quotes and the numbers taken from them.
    #[serde(default)]
    pub external: Vec<ExternalQuote>,
    /// watch: the person it is for (default yours).
    #[serde(default, rename = "for")]
    pub for_: Option<String>,
    /// watch: which story transitions match (every given key holds).
    #[serde(default, rename = "match")]
    pub match_: Option<WatchMatch>,
    /// watch: expiry (epoch ms).
    #[serde(default)]
    pub expires_t: Option<u64>,
    /// watch: ends after this many matches.
    #[serde(default)]
    pub max_matches: Option<u32>,
    /// unwatch: the watch id.
    #[serde(default)]
    pub watch: Option<String>,
    /// propose: protocol operations on views and drawings.
    #[serde(default)]
    pub ops: Vec<Value>,
    /// propose: why (shown to the person).
    #[serde(default)]
    pub reason: Option<String>,
    /// preview: the proposal (id or selector).
    #[serde(default)]
    pub proposal: Option<ProposalRef>,
    /// preview: scene width (default 1200).
    #[serde(default)]
    pub width: Option<u32>,
    /// preview: scene height (default 600).
    #[serde(default)]
    pub height: Option<u32>,
}

// ---------------------------------------------------------------------------------------
// Vocabulary tables.

const ASK_KEYS: [&str; 6] = ["find", "where", "select", "expand", "limit", "format"];

const FINDS: [&str; 11] = [
    "capsules",
    "stories",
    "facts",
    "instrument",
    "ledger",
    "market",
    "glossary",
    "changes",
    "comments",
    "watches",
    "proposals",
];

/// Accepted spellings of a noun → the canonical one.
const ALIASES: [(&str, &str); 16] = [
    ("capsule", "capsules"),
    ("story", "stories"),
    ("fact", "facts"),
    ("instruments", "instrument"),
    ("symbol", "instrument"),
    ("ledgers", "ledger"),
    ("scorecard", "ledger"),
    ("terms", "glossary"),
    ("change", "changes"),
    ("log", "changes"),
    ("comment", "comments"),
    ("watch", "watches"),
    ("intents", "watches"),
    ("proposal", "proposals"),
    ("market_status", "market"),
    ("findings", "capsules"),
];

fn where_keys(find: Find) -> &'static [&'static str] {
    match find {
        Find::Capsules => &[
            "id",
            "capsule",
            "instrument",
            "meaning",
            "direction",
            "stage",
            "open",
            "holdings",
            "latest",
            "audience",
            "since_bar",
            "since_t",
        ],
        Find::Stories => &[
            "instrument",
            "meaning",
            "direction",
            "stage",
            "open",
            "range",
            "after",
        ],
        Find::Facts => &[
            "id",
            "capsule",
            "instrument",
            "meaning",
            "direction",
            "kind",
            "since_bar",
            "since_t",
        ],
        Find::Instrument | Find::Market => &["instrument"],
        Find::Ledger => &["key", "meaning", "detector", "direction", "by_kind"],
        Find::Glossary => &["instrument", "capsule", "id"],
        Find::Changes => &["after_seq", "since_t", "within_ms", "author", "since_bar"],
        Find::Comments => &["id", "capsule", "instrument"],
        Find::Watches => &["id", "state", "instrument"],
        Find::Proposals => &["id", "state"],
    }
}

const ACT_DOS: [&str; 5] = ["comment", "watch", "unwatch", "propose", "preview"];

fn act_keys(d: Do) -> &'static [&'static str] {
    match d {
        Do::Comment => &[
            "do",
            "capsule",
            "instrument",
            "text",
            "lang",
            "cites",
            "external",
        ],
        Do::Watch => &["do", "for", "text", "match", "expires_t", "max_matches"],
        Do::Unwatch => &["do", "watch"],
        Do::Propose => &["do", "ops", "reason"],
        Do::Preview => &["do", "proposal", "width", "height"],
    }
}

/// Operations only a person may send (§25.2): never through this surface.
const HUMAN_ONLY: [&str; 4] = [
    "accept_proposal",
    "reject_proposal",
    "approve_finding",
    "reject_finding",
];

/// Stages after which a story is closed (§17.1 `open`).
const CLOSED: [&str; 4] = ["realized", "invalidated", "expired", "retracted"];

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

/// Edit distance (for `did_you_mean`).
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != *cb);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The closest candidates to `word` (distance ≤ 2 or a shared prefix of 3).
fn did_you_mean<'a>(word: &str, candidates: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let w = word.to_ascii_lowercase();
    let mut scored: Vec<(usize, &str)> = candidates
        .into_iter()
        .filter_map(|c| {
            let d = distance(&w, &c.to_ascii_lowercase());
            let prefix = w.len() >= 3 && c.to_ascii_lowercase().starts_with(&w[..3]);
            (d <= 2 || prefix).then_some((d, c))
        })
        .collect();
    scored.sort_unstable();
    scored
        .into_iter()
        .take(3)
        .map(|(_, c)| c.to_owned())
        .collect()
}

/// Checks the keys of object `v` at `path` against `allowed`.
fn check_keys(v: &Value, path: &str, allowed: &[&str], what: &str) -> Result<(), Fail> {
    let Some(o) = v.as_object() else {
        return Err(Fail::schema(
            path,
            format!("{what} must be an object"),
            allowed,
            vec![],
        ));
    };
    for k in o.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Fail::schema(
                &format!("{path}/{k}"),
                format!("`{k}` is not a key of {what}"),
                allowed,
                did_you_mean(k, allowed.iter().copied()),
            ));
        }
    }
    Ok(())
}

fn serde_fail(e: &serde_json::Error, what: &str) -> Fail {
    Fail(
        json!({"error": {"code": "schema", "message": format!("{what}: {e}"),
        "hint": "Check the value types against the tool's input schema."}}),
    )
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

fn arr<'v>(v: &'v Value, key: &str) -> &'v [Value] {
    v.get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn s<'v>(v: &'v Value, key: &str) -> &'v str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

/// `v[key]` as compact JSON text (numbers exactly as the authority wrote them).
fn t(v: &Value, key: &str) -> String {
    match v.get(key) {
        None | Some(Value::Null) => "-".to_owned(),
        Some(Value::String(x)) => x.clone(),
        Some(x) => x.to_string(),
    }
}

/// The fields `keys` of `v` that are present (and not false/null).
fn pick(v: &Value, keys: &[&str]) -> Value {
    let mut m = Map::new();
    for k in keys {
        if let Some(x) = v.get(*k) {
            if !x.is_null() && x != &Value::Bool(false) {
                m.insert((*k).to_owned(), x.clone());
            }
        }
    }
    Value::Object(m)
}

// ---------------------------------------------------------------------------------------
// Items.

/// One answer item in its three shapes.
#[derive(Default)]
struct Item {
    id: String,
    /// Other ids it carries (finding, facts, cited ids).
    refs: Vec<String>,
    /// `json`: the protocol object.
    raw: Value,
    /// `layered`: the compact summary.
    summary: Value,
    /// `card`: the text card (first line, then indented lines).
    card: Vec<String>,
    /// Expanded parts: name → (raw, summary, card lines).
    parts: Vec<(&'static str, Value, Value, Vec<String>)>,
}

/// An answer: items plus extra fields.
#[derive(Default)]
struct Out {
    items: Vec<Item>,
    total: usize,
    /// Extra top-level fields: name → (raw, compact, card lines).
    extra: Vec<(&'static str, Value, Value, Vec<String>)>,
}

fn capsule_summary(item: &Value, facts: &[String]) -> Value {
    let mut v = pick(
        item,
        &[
            "id",
            "finding",
            "instrument",
            "kind",
            "direction",
            "stage",
            "validity",
            "tier",
            "born_bar",
            "last_bar",
            "t",
            "ledger_at_birth",
            "relevant",
            "redacted",
            "coin_flip",
        ],
    );
    v["sentence"] = item
        .pointer("/sentence/text")
        .cloned()
        .unwrap_or(Value::Null);
    v["facts"] = json!(facts);
    v
}

fn capsule_card(item: &Value, facts: &[String]) -> Vec<String> {
    let lab = item.get("ledger_at_birth").cloned().unwrap_or(Value::Null);
    let mut flags = Vec::new();
    for f in ["relevant", "redacted", "coin_flip"] {
        if item.get(f) == Some(&Value::Bool(true)) {
            flags.push(f);
        }
    }
    let flags = if flags.is_empty() {
        String::new()
    } else {
        format!(" · {}", flags.join(", "))
    };
    vec![
        format!(
            "[{}] {} {} {} · stage {} · {} · tier {} · born bar {}, last bar {} · ledger at birth {} of {} realized{}",
            s(item, "id"),
            s(item, "instrument"),
            s(item, "kind"),
            s(item, "direction"),
            s(item, "stage"),
            s(item, "validity"),
            t(item, "tier"),
            t(item, "born_bar"),
            t(item, "last_bar"),
            t(&lab, "realized"),
            t(&lab, "n"),
            flags,
        ),
        format!("  {}", item.pointer("/sentence/text").and_then(Value::as_str).unwrap_or("")),
        format!("  finding {} · facts {}", s(item, "finding"), facts.join(", ")),
    ]
}

/// The expanded parts of a capsule detail.
fn detail_parts(
    detail: &Value,
    expand: &[Expand],
) -> Vec<(&'static str, Value, Value, Vec<String>)> {
    let mut out = Vec::new();
    for e in expand {
        match e {
            Expand::Story => {
                let raw = detail.get("story").cloned().unwrap_or(Value::Null);
                let hist: Vec<Value> = arr(&raw, "history")
                    .iter()
                    .map(|h| pick(h, &["bar", "t", "stage", "reason", "facts"]))
                    .collect();
                let line = hist
                    .iter()
                    .map(|h| format!("bar {} {} ({})", t(h, "bar"), s(h, "stage"), s(h, "reason")))
                    .collect::<Vec<_>>()
                    .join(" → ");
                let mut sum = json!({"history": hist});
                for k in ["condition", "resolution"] {
                    if let Some(x) = raw.get(k) {
                        sum[k] = x.clone();
                    }
                }
                out.push(("story", raw, sum, vec![format!("  story: {line}")]));
            }
            Expand::Evidence => {
                let raw = detail.get("evidence").cloned().unwrap_or(json!([]));
                let mut sum = Vec::new();
                let mut lines = Vec::new();
                for ev in raw.as_array().map_or(&[][..], Vec::as_slice) {
                    let mut x = pick(
                        ev,
                        &[
                            "fact", "family", "license", "open", "values", "requires", "rests_on",
                        ],
                    );
                    if let Some(d) = ev.pointer("/evidence_ref/digest") {
                        x["evidence_ref"] = d.clone();
                    }
                    let what = if ev.get("open") == Some(&Value::Bool(true)) {
                        let vals = ev
                            .get("values")
                            .and_then(Value::as_object)
                            .map(|m| {
                                m.iter()
                                    .map(|(k, v)| format!("{k}={v}"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_default();
                        format!("open: {vals}")
                    } else {
                        format!("closed, requires {}", t(ev, "requires"))
                    };
                    lines.push(format!(
                        "  evidence {} ({}, {}) {}; rests on {}{}",
                        s(ev, "fact"),
                        s(ev, "family"),
                        s(ev, "license"),
                        what,
                        s(ev, "rests_on"),
                        ev.pointer("/evidence_ref/digest")
                            .and_then(Value::as_str)
                            .map(|d| format!("; ref {d}"))
                            .unwrap_or_default(),
                    ));
                    sum.push(x);
                }
                out.push(("evidence", raw, Value::Array(sum), lines));
            }
            Expand::Ledger => {
                let raw = detail.get("ledger").cloned().unwrap_or(Value::Null);
                let line = format!("  ledger: {raw}");
                out.push(("ledger", raw.clone(), raw, vec![line]));
            }
            Expand::Comments => {
                let raw = detail.get("comments").cloned().unwrap_or(json!([]));
                let sum: Vec<Value> = raw
                    .as_array()
                    .map_or(&[][..], Vec::as_slice)
                    .iter()
                    .map(comment_summary)
                    .collect();
                let lines = sum
                    .iter()
                    .map(|c| {
                        format!(
                            "  comment [{}] {} (cites {})",
                            s(c, "id"),
                            s(c, "text"),
                            t(c, "cites")
                        )
                    })
                    .collect();
                out.push(("comments", raw, Value::Array(sum), lines));
            }
        }
    }
    out
}

fn capsule_item(item: &Value, detail: &Value, expand: &[Expand]) -> Item {
    let facts: Vec<String> = arr(detail, "facts")
        .iter()
        .filter_map(|f| f.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect();
    let mut refs = vec![s(item, "finding").to_owned()];
    refs.extend(facts.iter().cloned());
    Item {
        id: s(item, "id").to_owned(),
        refs,
        raw: item.clone(),
        summary: capsule_summary(item, &facts),
        card: capsule_card(item, &facts),
        parts: detail_parts(detail, expand),
    }
}

fn comment_summary(c: &Value) -> Value {
    let mut v = pick(
        c,
        &[
            "id",
            "about",
            "lang",
            "text",
            "cites",
            "external",
            "on_behalf",
        ],
    );
    if let Some(a) = c.pointer("/provenance/author/name") {
        v["author"] = a.clone();
    }
    v
}

/// The server comments (unentitled reader) of a `capsules`/`stories` answer.
fn server_comments(data: &Value) -> Option<(&'static str, Value, Value, Vec<String>)> {
    let raw = data.get("comments")?.clone();
    let list = raw.as_array()?;
    if list.is_empty() {
        return None;
    }
    let sum: Vec<Value> = list
        .iter()
        .map(|c| {
            let mut x = pick(
                c,
                &["instrument", "series", "bar", "t", "codes", "tr", "en"],
            );
            if let Some(e) = c.pointer("/attachments/entitlement") {
                x["requires"] = e.clone();
            }
            x
        })
        .collect();
    let lines = sum
        .iter()
        .map(|c| {
            format!(
                "server comment {} bar {}: {} (raw figures require {})",
                s(c, "instrument"),
                t(c, "bar"),
                s(c, "tr"),
                t(c, "requires")
            )
        })
        .collect();
    Some(("server_comments", raw, Value::Array(sum), lines))
}

// ---------------------------------------------------------------------------------------
// Finds.

fn stage_list(w: &Where) -> Vec<String> {
    w.stage
        .as_ref()
        .map(|st| {
            st.to_vec()
                .iter()
                .filter_map(|x| serde_json::to_value(x).ok())
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn ser(v: &impl serde::Serialize) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

fn find_capsules(
    io: &mut Io<'_, '_>,
    w: &Where,
    expand: &[Expand],
    limit: usize,
) -> Result<Out, Fail> {
    let one = w
        .capsule
        .clone()
        .or_else(|| w.id.clone().map(CapsuleRef::Id));
    if let Some(r) = one {
        let detail = io.capsule_detail(&r)?;
        let item = detail.get("item").cloned().unwrap_or(Value::Null);
        let mut out = Out {
            total: 1,
            ..Out::default()
        };
        out.items.push(capsule_item(&item, &detail, expand));
        if let Some(wh) = detail.get("withheld") {
            out.extra.push((
                "withheld",
                wh.clone(),
                wh.clone(),
                vec![format!("withheld: {wh}")],
            ));
        }
        return Ok(out);
    }
    let mut q = json!({"kind": "capsules"});
    if let Some(i) = &w.instrument {
        q["instrument"] = json!(io.instrument(i, "/where/instrument")?);
    }
    let stages = stage_list(w);
    if !stages.is_empty() {
        q["stages"] = json!(stages);
    }
    if let Some(o) = w.open {
        q["open"] = json!(o);
    }
    if let Some(m) = &w.meaning {
        q["meaning"] = ser(m);
    }
    if let Some(b) = w.since_bar {
        q["since_bar"] = json!(b);
    }
    if let Some(x) = w.since_t {
        q["since_t"] = json!(x);
    }
    if w.audience == Some(true) {
        q["audience"] = json!(true);
    }
    let local = w.direction.is_some() || w.holdings == Some(true);
    let latest = w.latest == Some(true);
    if latest && !local {
        q["latest"] = json!(true);
    }
    let data = io.query(q)?;
    let mut list: Vec<Value> = arr(&data, "capsules").to_vec();
    let mut holdings_note = false;
    if let Some(d) = &w.direction {
        let d = ser(d);
        list.retain(|c| c.get("direction") == Some(&d));
    }
    if w.holdings == Some(true) {
        // The authority marks what concerns the person (§20.2 `relevant`: a position or
        // an active watch of theirs; an agent reads its person's audience, §25.7).
        list.retain(|c| c.get("relevant") == Some(&Value::Bool(true)));
        if list.is_empty() {
            holdings_note = true;
        }
    }
    if latest && local {
        // The most recently born (birth bar, then id), as the protocol's `latest`.
        if let Some(best) = list
            .iter()
            .max_by(|a, b| {
                (a.get("born_bar").and_then(Value::as_u64), s(a, "id"))
                    .cmp(&(b.get("born_bar").and_then(Value::as_u64), s(b, "id")))
            })
            .cloned()
        {
            list = vec![best];
        }
    }
    let mut out = Out {
        total: list.len(),
        ..Out::default()
    };
    for c in list.iter().take(limit) {
        let detail = io.capsule_detail(&CapsuleRef::Id(s(c, "id").to_owned()))?;
        out.items.push(capsule_item(c, &detail, expand));
    }
    if let Some(sc) = server_comments(&data) {
        out.extra.push(sc);
    }
    if holdings_note {
        let n = "holdings: the authority marks no capsule as relevant (a position or an active watch) for you";
        out.extra
            .push(("note", json!(n), json!(n), vec![n.to_owned()]));
    }
    Ok(out)
}

fn find_stories(
    io: &mut Io<'_, '_>,
    w: &Where,
    expand: &[Expand],
    limit: usize,
) -> Result<Out, Fail> {
    let Some(i) = &w.instrument else {
        let syms = io.symbols()?;
        return Err(Fail(
            json!({"error": {"code": "schema", "path": "/where/instrument",
            "message": "stories need `where.instrument`", "expected": syms,
            "hint": "e.g. {\"find\":\"stories\",\"where\":{\"instrument\":\"THYAO\"}}"}}),
        ));
    };
    let sym = io.instrument(i, "/where/instrument")?;
    let mut q = json!({"kind": "stories", "instrument": sym});
    if let Some(r) = &w.range {
        q["range"] = ser(r);
    }
    if let Some(a) = &w.after {
        q["after"] = json!(a);
    }
    let data = io.query(q)?;
    let mut rows: Vec<Value> = arr(&data, "stories").to_vec();
    if let Some(m) = &w.meaning {
        let m = ser(m);
        rows.retain(|r| r.get("kind") == Some(&m));
    }
    if let Some(d) = &w.direction {
        let d = ser(d);
        rows.retain(|r| r.get("direction") == Some(&d));
    }
    let stages = stage_list(w);
    if !stages.is_empty() {
        rows.retain(|r| stages.iter().any(|x| x == s(r, "stage")));
    }
    if let Some(o) = w.open {
        rows.retain(|r| r.get("closed_bar").is_none() == o);
    }
    let mut out = Out {
        total: rows.len(),
        ..Out::default()
    };
    for r in rows.iter().take(limit) {
        let mut sum = pick(
            r,
            &[
                "id",
                "finding",
                "kind",
                "direction",
                "stage",
                "outcome",
                "validity",
                "tier",
                "born_bar",
                "last_bar",
                "closed_bar",
                "in_feed",
                "hidden",
                "coin_flip",
                "redacted",
            ],
        );
        if let Some(f) = r.get("face") {
            sum["face"] = pick(
                f,
                &["event", "when", "moved", "validity_phrase", "ledger_short"],
            );
        }
        let face = r.get("face").cloned().unwrap_or(Value::Null);
        let card = vec![format!(
            "[{}] {} {} · stage {} · outcome {} · {} · tier {} · born bar {}, last bar {}, closed bar {} · in feed {}{} · {} · {} · ledger {}",
            s(r, "id"),
            s(r, "kind"),
            s(r, "direction"),
            s(r, "stage"),
            t(r, "outcome"),
            s(r, "validity"),
            t(r, "tier"),
            t(r, "born_bar"),
            t(r, "last_bar"),
            t(r, "closed_bar"),
            t(r, "in_feed"),
            r.get("hidden").map(|h| format!(" (hidden: {})", h.as_str().unwrap_or(""))).unwrap_or_default(),
            s(&face, "event"),
            s(&face, "when"),
            s(&face, "ledger_short"),
        )];
        let parts = if expand.is_empty() {
            Vec::new()
        } else {
            let detail = io.capsule_detail(&CapsuleRef::Id(s(r, "id").to_owned()))?;
            detail_parts(&detail, expand)
        };
        out.items.push(Item {
            id: s(r, "id").to_owned(),
            refs: vec![s(r, "finding").to_owned()],
            raw: r.clone(),
            summary: sum,
            card,
            parts,
        });
    }
    for k in ["series", "next"] {
        if let Some(v) = data.get(k) {
            out.extra.push((
                if k == "series" { "series" } else { "next" },
                v.clone(),
                v.clone(),
                vec![format!("{k}: {}", v.as_str().unwrap_or(""))],
            ));
        }
    }
    if let Some(n) = data.get("total") {
        out.extra.push((
            "instrument_total",
            n.clone(),
            n.clone(),
            vec![format!("stories of the instrument in range: {n}")],
        ));
    }
    if let Some(sc) = server_comments(&data) {
        out.extra.push(sc);
    }
    Ok(out)
}

fn fact_item(f: &Value) -> Item {
    let m = f.get("meaning").cloned().unwrap_or(Value::Null);
    let mut sum = pick(f, &["id", "kind", "series", "bar", "t", "text"]);
    for (k, p) in [
        ("instrument", "/instrument"),
        ("meaning", "/kind"),
        ("direction", "/direction"),
        ("license", "/license/class"),
    ] {
        if let Some(x) = m.pointer(p) {
            sum[k] = x.clone();
        }
    }
    let ev = f.get("evidence").cloned().unwrap_or(json!({}));
    sum["evidence"] = ev.clone();
    if let Some(d) = f.pointer("/evidence_ref/digest") {
        sum["evidence_ref"] = d.clone();
    }
    let vals = ev
        .as_object()
        .map(|o| {
            o.iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let card = vec![format!(
        "[{}] {} · {} · bar {} · t {} · license {} · evidence {}",
        s(f, "id"),
        s(f, "kind"),
        t(&m, "instrument"),
        t(f, "bar"),
        t(f, "t"),
        m.pointer("/license/class")
            .and_then(Value::as_str)
            .unwrap_or("-"),
        if vals.is_empty() {
            "-".to_owned()
        } else {
            vals
        },
    )];
    Item {
        id: s(f, "id").to_owned(),
        refs: Vec::new(),
        raw: f.clone(),
        summary: sum,
        card,
        parts: Vec::new(),
    }
}

fn find_facts(io: &mut Io<'_, '_>, w: &Where, limit: usize) -> Result<Out, Fail> {
    let only: Option<BTreeSet<String>> = match &w.capsule {
        Some(r) => {
            let d = io.capsule_detail(r)?;
            Some(
                arr(&d, "facts")
                    .iter()
                    .filter_map(|f| f.get("id").and_then(Value::as_str).map(str::to_owned))
                    .collect(),
            )
        }
        None => None,
    };
    let sym = match &w.instrument {
        Some(i) => Some(io.instrument(i, "/where/instrument")?),
        None => None,
    };
    let data = io.query(json!({"kind": "list", "collection": "facts"}))?;
    let mut list: Vec<Value> = arr(&data, "items").to_vec();
    list.retain(|f| {
        let m = f.get("meaning").cloned().unwrap_or(Value::Null);
        w.id.as_ref().is_none_or(|i| s(f, "id") == i)
            && only.as_ref().is_none_or(|o| o.contains(s(f, "id")))
            && sym.as_ref().is_none_or(|x| s(&m, "instrument") == x)
            && w.meaning
                .as_ref()
                .is_none_or(|x| m.get("kind") == Some(&ser(x)))
            && w.direction
                .as_ref()
                .is_none_or(|x| m.get("direction") == Some(&ser(x)))
            && w.kind
                .as_ref()
                .is_none_or(|k| s(f, "kind").eq_ignore_ascii_case(k))
            && w.since_bar
                .is_none_or(|b| f.get("bar").and_then(Value::as_u64).is_some_and(|x| x >= b))
            && w.since_t
                .is_none_or(|b| f.get("t").and_then(Value::as_u64).is_some_and(|x| x >= b))
    });
    // Latest first.
    list.reverse();
    let mut out = Out {
        total: list.len(),
        ..Out::default()
    };
    out.items = list.iter().take(limit).map(fact_item).collect();
    if out.items.is_empty() {
        if let (Some(k), true) = (&w.kind, list.is_empty()) {
            let kinds: BTreeSet<String> = arr(&data, "items")
                .iter()
                .map(|f| s(f, "kind").to_owned())
                .collect();
            let dym = did_you_mean(k, kinds.iter().map(String::as_str));
            if !dym.is_empty() {
                out.extra.push((
                    "did_you_mean",
                    json!(dym),
                    json!(dym),
                    vec![format!("did_you_mean kind: {}", dym.join(", "))],
                ));
            }
        }
    }
    Ok(out)
}

fn find_instrument(io: &mut Io<'_, '_>, w: &Where) -> Result<Out, Fail> {
    let syms = match &w.instrument {
        Some(i) => vec![io.instrument(i, "/where/instrument")?],
        None => io.symbols()?,
    };
    let series = io.query(json!({"kind": "list", "collection": "series"}))?;
    let bindings = io.query(json!({"kind": "binding"}))?;
    let market = market_json(io);
    let mut out = Out {
        total: syms.len(),
        ..Out::default()
    };
    for sym in &syms {
        let mut ser_rows = Vec::new();
        let mut last = Value::Null;
        for x in arr(&series, "items") {
            if x.pointer("/instrument/symbol").and_then(Value::as_str) != Some(sym.as_str()) {
                continue;
            }
            let id = s(x, "id").to_owned();
            let b = arr(&bindings, "bindings")
                .iter()
                .find(|b| s(b, "series") == id);
            let state = b.map_or(json!("document"), |b| {
                b.get("state").cloned().unwrap_or(Value::Null)
            });
            let mut row = json!({"id": id, "interval": x.get("interval").cloned().unwrap_or(Value::Null), "data": state});
            if let Some(reason) = b.and_then(|b| b.get("reason")) {
                row["reason"] = reason.clone();
            }
            ser_rows.push(row);
            if last.is_null() {
                last = match io.query(json!({"kind": "study", "series": id, "reduce": "last"})) {
                    Ok(d) => {
                        let v = d.get("values").cloned().unwrap_or(Value::Null);
                        let mut l = json!({"series": id});
                        for k in ["o", "h", "l", "c", "v"] {
                            if let Some(x) = v.get(k) {
                                l["bar"] = x.get("bar").cloned().unwrap_or(Value::Null);
                                l["t"] = x.get("t").cloned().unwrap_or(Value::Null);
                                l[k] = x.get("value").cloned().unwrap_or(Value::Null);
                            }
                        }
                        l
                    }
                    Err(Fail(e)) => {
                        json!({"series": id, "unavailable": e.pointer("/error/code").cloned().unwrap_or(Value::Null)})
                    }
                };
            }
        }
        let caps = io.query(json!({"kind": "capsules", "instrument": sym}))?;
        let list = arr(&caps, "capsules");
        let open: Vec<String> = list
            .iter()
            .filter(|c| !CLOSED.contains(&s(c, "stage")))
            .map(|c| s(c, "id").to_owned())
            .collect();
        let phase = market
            .as_ref()
            .and_then(|m| m.pointer(&format!("/phases/{sym}/key")).cloned())
            .unwrap_or(Value::Null);
        let sum = json!({
            "id": sym, "series": ser_rows, "last": last,
            "capsules": list.len(), "open_capsules": open,
            "server_comments": arr(&caps, "comments").len(), "phase": phase,
        });
        let last_txt = if last.get("c").is_some() {
            format!(
                "last bar {} (t {}): o {} h {} l {} c {} v {}",
                t(&last, "bar"),
                t(&last, "t"),
                t(&last, "o"),
                t(&last, "h"),
                t(&last, "l"),
                t(&last, "c"),
                t(&last, "v")
            )
        } else {
            format!("last bar unavailable ({})", t(&last, "unavailable"))
        };
        let card = vec![
            format!(
                "[{sym}] series {} · {last_txt}",
                ser_rows
                    .iter()
                    .map(|r| format!("{} {} data {}", s(r, "id"), t(r, "interval"), t(r, "data")))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            format!(
                "  capsules {} (open: {}) · server comments {} · session phase {}",
                list.len(),
                if open.is_empty() {
                    "none".to_owned()
                } else {
                    open.join(", ")
                },
                arr(&caps, "comments").len(),
                phase.as_str().unwrap_or("-"),
            ),
        ];
        out.items.push(Item {
            id: sym.clone(),
            refs: open.clone(),
            raw: json!({"series": ser_rows, "last": last, "capsules": caps, "phase": phase}),
            summary: sum,
            card,
            parts: Vec::new(),
        });
    }
    Ok(out)
}

fn find_ledger(io: &mut Io<'_, '_>, w: &Where) -> Result<Out, Fail> {
    let mut q = json!({"kind": "ledger"});
    if let Some(k) = &w.key {
        q["key"] = json!(k);
    }
    if let Some(m) = &w.meaning {
        q["meaning"] = ser(m);
    }
    if let Some(d) = &w.detector {
        q["detector"] = json!(d);
    }
    if let Some(d) = &w.direction {
        q["direction"] = ser(d);
    }
    if w.by_kind == Some(true) {
        q["by_kind"] = json!(true);
    }
    let data = io.query(q)?;
    let rows = arr(&data, "rows");
    let mut out = Out {
        total: rows.len(),
        ..Out::default()
    };
    for r in rows {
        out.items.push(Item {
            id: s(r, "key").to_owned(),
            refs: Vec::new(),
            raw: r.clone(),
            summary: r.clone(),
            card: vec![format!(
                "[{}] closed {} · realized {}{}",
                s(r, "key"),
                t(r, "n"),
                t(r, "realized"),
                r.get("direction")
                    .map(|d| format!(" · direction {}", d.as_str().unwrap_or("")))
                    .unwrap_or_default()
            )],
            parts: Vec::new(),
        });
    }
    if let Some(tot) = data.get("total") {
        out.extra.push((
            "ledger_total",
            tot.clone(),
            tot.clone(),
            vec![format!(
                "total: closed {} · realized {}",
                t(tot, "n"),
                t(tot, "realized")
            )],
        ));
    }
    Ok(out)
}

/// The market status as the authority projects it for this principal (§24.2).
fn market_json(io: &Io<'_, '_>) -> Option<Value> {
    let p = io.ctx.principal().to_owned();
    let m = io.ctx.hub().borrow().session().market_for(&p)?;
    serde_json::to_value(m).ok()
}

fn find_market(io: &mut Io<'_, '_>, w: &Where) -> Result<Out, Fail> {
    let sym = match &w.instrument {
        Some(i) => Some(io.instrument(i, "/where/instrument")?),
        None => None,
    };
    let mut out = Out::default();
    let Some(mut m) = market_json(io) else {
        out.extra.push((
            "market",
            Value::Null,
            Value::Null,
            vec!["market: the host has given no market status".to_owned()],
        ));
        return Ok(out);
    };
    if let (Some(sym), Some(ph)) = (&sym, m.get_mut("phases").and_then(Value::as_object_mut)) {
        ph.retain(|k, _| k == sym);
    }
    let phases = m
        .get("phases")
        .and_then(Value::as_object)
        .map(|p| {
            p.iter()
                .map(|(k, v)| format!("{k} {}", s(v, "key")))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    out.total = 1;
    out.items.push(Item {
        id: "market".to_owned(),
        refs: Vec::new(),
        raw: m.clone(),
        summary: m.clone(),
        card: vec![format!(
            "[market] open {} · holiday {} · source {} · delayed {} · stale {} · exchange time {} · phases {}",
            t(&m, "open"),
            t(&m, "holiday"),
            t(&m, "source"),
            t(&m, "delayed"),
            t(&m, "stale"),
            t(&m, "exchange_time"),
            if phases.is_empty() { "-".to_owned() } else { phases },
        )],
        parts: Vec::new(),
    });
    Ok(out)
}

/// Glossary: the connection's capsule voice is set to `selvi` for one read (capsule items
/// then carry `voice.glossary`, §20.2) and set back to `r4`.
fn set_voice(io: &mut Io<'_, '_>, voice: &str) -> Result<(), Fail> {
    let id = io.ctx.request_id();
    let r = io
        .ctx
        .send(&json!({"type": "input", "id": id, "gesture": {"kind": "style", "voice": voice}}));
    if r.get("type").and_then(Value::as_str) == Some("error") {
        return Err(Fail::protocol("input style", &r));
    }
    Ok(())
}

fn glossary_capsules(io: &mut Io<'_, '_>, w: &Where) -> Result<Vec<Value>, Fail> {
    let one = w
        .capsule
        .clone()
        .or_else(|| w.id.clone().map(CapsuleRef::Id));
    if let Some(r) = one {
        let d = io.capsule_detail(&r)?;
        return Ok(vec![d.get("item").cloned().unwrap_or(Value::Null)]);
    }
    let mut q = json!({"kind": "capsules"});
    if let Some(i) = &w.instrument {
        q["instrument"] = json!(io.instrument(i, "/where/instrument")?);
    }
    Ok(arr(&io.query(q)?, "capsules").to_vec())
}

fn find_glossary(io: &mut Io<'_, '_>, w: &Where, limit: usize) -> Result<Out, Fail> {
    set_voice(io, "selvi")?;
    let read = glossary_capsules(io, w);
    set_voice(io, "r4")?;
    let caps = read?;
    let mut terms: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();
    for c in &caps {
        for g in c
            .pointer("/voice/glossary")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice)
        {
            let e = terms
                .entry(s(g, "term").to_owned())
                .or_insert_with(|| (s(g, "text").to_owned(), Vec::new()));
            e.1.push(s(c, "id").to_owned());
        }
    }
    let mut out = Out {
        total: terms.len(),
        ..Out::default()
    };
    for (term, (text, caps)) in terms.into_iter().take(limit) {
        let v = json!({"id": term, "term": term, "text": text, "capsules": caps});
        out.items.push(Item {
            id: term.clone(),
            refs: caps.clone(),
            raw: v.clone(),
            summary: v,
            card: vec![format!("[{term}] {text} (in {})", caps.join(", "))],
            parts: Vec::new(),
        });
    }
    Ok(out)
}

/// Ids an entry's changes touch (the `id` of each changed item, or a named id field).
fn touched(entry: &Value) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for ch in arr(entry, "changes") {
        if let Some(o) = ch.as_object() {
            for (k, v) in o {
                if k == "op" {
                    continue;
                }
                let id = v.get("id").and_then(Value::as_str).or_else(|| {
                    matches!(
                        k.as_str(),
                        "id" | "capsule"
                            | "finding"
                            | "series"
                            | "view"
                            | "intent"
                            | "proposal"
                            | "comment"
                    )
                    .then(|| v.as_str())
                    .flatten()
                });
                if let Some(id) = id {
                    if !out.iter().any(|x| x == id) {
                        out.push(id.to_owned());
                    }
                }
            }
        }
    }
    out
}

fn find_changes(io: &mut Io<'_, '_>, w: &Where, limit: usize) -> Result<Out, Fail> {
    let mut q = json!({"kind": "log", "limit": limit});
    if let Some(x) = w.after_seq {
        q["after_seq"] = json!(x);
    }
    if let Some(x) = w.since_t {
        q["since_t"] = json!(x);
    }
    if let Some(x) = w.within_ms {
        q["within_ms"] = json!(x);
    }
    if let Some(a) = &w.author {
        q["author"] = json!(a);
    }
    let data = io.query(q)?;
    let entries = arr(&data, "entries");
    let mut out = Out {
        total: entries.len(),
        ..Out::default()
    };
    for e in entries {
        let ops: Vec<Value> = arr(e, "changes")
            .iter()
            .filter_map(|c| c.get("op").cloned())
            .collect();
        let ids = touched(e);
        let mut sum = pick(
            e,
            &["seq", "at", "author", "summary", "undoes", "approved_by"],
        );
        sum["ops"] = json!(ops);
        sum["ids"] = json!(ids);
        out.items.push(Item {
            id: format!("seq {}", t(e, "seq")),
            refs: ids.clone(),
            raw: e.clone(),
            summary: sum,
            card: vec![format!(
                "[seq {}] {} · {} · {} change(s){}",
                t(e, "seq"),
                s(e, "author"),
                s(e, "summary"),
                ops.len(),
                if ids.is_empty() {
                    String::new()
                } else {
                    format!(" · {}", ids.join(", "))
                }
            )],
            parts: Vec::new(),
        });
    }
    if let Some(b) = w.since_bar {
        let caps = io.query(json!({"kind": "capsules", "since_bar": b}))?;
        let moved: Vec<Value> = arr(&caps, "capsules")
            .iter()
            .map(|c| {
                pick(
                    c,
                    &["id", "instrument", "kind", "direction", "stage", "last_bar"],
                )
            })
            .collect();
        let lines = moved
            .iter()
            .map(|c| {
                format!(
                    "moved [{}] {} {} {} · stage {} at bar {}",
                    s(c, "id"),
                    s(c, "instrument"),
                    s(c, "kind"),
                    s(c, "direction"),
                    s(c, "stage"),
                    t(c, "last_bar")
                )
            })
            .collect();
        out.extra.push(("moved", caps, Value::Array(moved), lines));
    }
    Ok(out)
}

fn find_comments(io: &mut Io<'_, '_>, w: &Where, limit: usize) -> Result<Out, Fail> {
    let cap = match &w.capsule {
        Some(r) => Some(
            s(
                &io.capsule_detail(r)?
                    .get("item")
                    .cloned()
                    .unwrap_or(Value::Null),
                "id",
            )
            .to_owned(),
        ),
        None => None,
    };
    let sym = match &w.instrument {
        Some(i) => Some(io.instrument(i, "/where/instrument")?),
        None => None,
    };
    let inst_caps: Option<BTreeSet<String>> = match &sym {
        Some(x) => Some(
            arr(
                &io.query(json!({"kind": "capsules", "instrument": x}))?,
                "capsules",
            )
            .iter()
            .map(|c| s(c, "id").to_owned())
            .collect(),
        ),
        None => None,
    };
    let data = io.query(json!({"kind": "list", "collection": "comments"}))?;
    let mut list: Vec<Value> = arr(&data, "items").to_vec();
    list.retain(|c| {
        let about_cap = c.pointer("/about/capsule").and_then(Value::as_str);
        let about_ins = c.pointer("/about/instrument").and_then(Value::as_str);
        w.id.as_ref().is_none_or(|i| s(c, "id") == i)
            && cap.as_ref().is_none_or(|x| about_cap == Some(x.as_str()))
            && sym.as_ref().is_none_or(|x| {
                about_ins == Some(x.as_str())
                    || about_cap.is_some_and(|a| inst_caps.as_ref().is_some_and(|s| s.contains(a)))
            })
    });
    list.reverse();
    let mut out = Out {
        total: list.len(),
        ..Out::default()
    };
    for c in list.iter().take(limit) {
        let sum = comment_summary(c);
        let about = c.get("about").cloned().unwrap_or(Value::Null);
        out.items.push(Item {
            id: s(c, "id").to_owned(),
            refs: arr(c, "cites")
                .iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect(),
            raw: c.clone(),
            card: vec![format!(
                "[{}] about {} · {} · by {}{} · \"{}\" · cites {}",
                s(c, "id"),
                about,
                t(c, "lang"),
                t(&sum, "author"),
                c.get("on_behalf")
                    .map(|p| format!(" for {}", p.as_str().unwrap_or("")))
                    .unwrap_or_default(),
                s(c, "text"),
                t(c, "cites"),
            )],
            summary: sum,
            parts: Vec::new(),
        });
    }
    Ok(out)
}

fn find_watches(io: &mut Io<'_, '_>, w: &Where, limit: usize) -> Result<Out, Fail> {
    let sym = match &w.instrument {
        Some(i) => Some(io.instrument(i, "/where/instrument")?),
        None => None,
    };
    let data = io.query(json!({"kind": "list", "collection": "intents"}))?;
    let mut list: Vec<Value> = arr(&data, "items")
        .iter()
        .filter(|i| i.get("watch").is_some())
        .cloned()
        .collect();
    list.retain(|i| {
        w.id.as_ref().is_none_or(|x| s(i, "id") == x)
            && w.state
                .as_ref()
                .is_none_or(|x| i.get("state").and_then(Value::as_str).unwrap_or("active") == x)
            && sym.as_ref().is_none_or(|x| {
                i.pointer("/watch/match/instrument").and_then(Value::as_str) == Some(x.as_str())
            })
    });
    let mut out = Out {
        total: list.len(),
        ..Out::default()
    };
    for i in list.iter().take(limit) {
        let wch = i.get("watch").cloned().unwrap_or(Value::Null);
        let mut sum = pick(i, &["id", "text", "state", "matches", "active"]);
        for k in ["for", "match", "expires_t", "max_matches"] {
            if let Some(x) = wch.get(k) {
                sum[k] = x.clone();
            }
        }
        out.items.push(Item {
            id: s(i, "id").to_owned(),
            refs: Vec::new(),
            raw: i.clone(),
            card: vec![format!(
                "[{}] for {} · state {} · matches {} · \"{}\" · match {}{}{}",
                s(i, "id"),
                t(&wch, "for"),
                i.get("state").and_then(Value::as_str).unwrap_or("active"),
                t(i, "matches"),
                s(i, "text"),
                t(&wch, "match"),
                wch.get("expires_t")
                    .map(|x| format!(" · expires_t {x}"))
                    .unwrap_or_default(),
                wch.get("max_matches")
                    .map(|x| format!(" · max_matches {x}"))
                    .unwrap_or_default(),
            )],
            summary: sum,
            parts: Vec::new(),
        });
    }
    Ok(out)
}

fn find_proposals(io: &mut Io<'_, '_>, w: &Where, limit: usize) -> Result<Out, Fail> {
    const STATES: [&str; 4] = ["pending", "accepted", "rejected", "expired"];
    let mut q = json!({"kind": "list", "collection": "proposals"});
    if let Some(st) = &w.state {
        if !STATES.contains(&st.as_str()) {
            return Err(Fail::schema(
                "/where/state",
                format!("`{st}` is not a proposal state"),
                &STATES,
                did_you_mean(st, STATES),
            ));
        }
        q["state"] = json!(st);
    }
    let data = io.query(q)?;
    let mut list: Vec<Value> = arr(&data, "items").to_vec();
    if let Some(i) = &w.id {
        list.retain(|p| s(p, "id") == i);
    }
    list.reverse();
    let mut out = Out {
        total: list.len(),
        ..Out::default()
    };
    for p in list.iter().take(limit) {
        let ops: Vec<Value> = arr(p, "ops")
            .iter()
            .filter_map(|o| o.get("op").cloned())
            .collect();
        let mut sum = pick(
            p,
            &["id", "state", "reason", "note", "expires_at", "decided_seq"],
        );
        sum["ops"] = json!(ops);
        if let Some(a) = p.pointer("/provenance/author/name") {
            sum["author"] = a.clone();
        }
        if let Some(a) = p.pointer("/decided_by/name") {
            sum["decided_by"] = a.clone();
        }
        out.items.push(Item {
            id: s(p, "id").to_owned(),
            refs: Vec::new(),
            raw: p.clone(),
            card: vec![format!(
                "[{}] {} · by {} · ops {}{}",
                s(p, "id"),
                s(p, "state"),
                t(&sum, "author"),
                ops.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", "),
                p.get("reason")
                    .map(|r| format!(" · reason \"{}\"", r.as_str().unwrap_or("")))
                    .unwrap_or_default(),
            )],
            summary: sum,
            parts: Vec::new(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------
// Shaping.

fn select_fields(v: &Value, select: &[String]) -> Value {
    if select.is_empty() {
        return v.clone();
    }
    let mut m = Map::new();
    for k in std::iter::once("id").chain(select.iter().map(String::as_str)) {
        if let Some(x) = v.get(k) {
            m.insert(k.to_owned(), x.clone());
        }
    }
    Value::Object(m)
}

fn shape(find: &str, format: Format, select: &[String], out: Out, seq: Option<u64>) -> Value {
    let count = out.items.len();
    let mut ids: Vec<String> = Vec::new();
    for it in &out.items {
        for x in std::iter::once(&it.id).chain(it.refs.iter()) {
            if !x.is_empty() && !ids.contains(x) {
                ids.push(x.clone());
            }
        }
    }
    let mut env =
        json!({"find": find, "format": format_name(format), "count": count, "total": out.total});
    if let Some(s) = seq {
        env["seq"] = json!(s);
    }
    let truncated = out.total > count;
    let more = truncated.then(|| {
        format!(
            "{} more; raise `limit` (≤ 100) or narrow `where`",
            out.total - count
        )
    });
    let text = match format {
        Format::Card => {
            env["ids"] = json!(ids);
            let mut lines = vec![format!(
                "{find}: {count} of {}{}",
                out.total,
                seq.map(|s| format!(" · seq {s}")).unwrap_or_default()
            )];
            for it in &out.items {
                if select.is_empty() {
                    lines.extend(it.card.iter().cloned());
                } else {
                    let v = select_fields(&it.summary, select);
                    let kv = v
                        .as_object()
                        .map(|o| {
                            o.iter()
                                .filter(|(k, _)| k.as_str() != "id")
                                .map(|(k, x)| {
                                    format!(
                                        "{k}={}",
                                        x.as_str().map_or_else(|| x.to_string(), str::to_owned)
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(" · ")
                        })
                        .unwrap_or_default();
                    lines.push(format!("[{}] {kv}", it.id));
                }
                for (_, _, _, l) in &it.parts {
                    lines.extend(l.iter().cloned());
                }
            }
            for (_, _, _, l) in &out.extra {
                lines.extend(l.iter().cloned());
            }
            if let Some(m) = &more {
                lines.push(m.clone());
            }
            lines.join("\n")
        }
        Format::Json | Format::Layered => {
            let raw = format == Format::Json;
            let items: Vec<Value> = out
                .items
                .iter()
                .map(|it| {
                    let mut v = select_fields(if raw { &it.raw } else { &it.summary }, select);
                    if raw && !it.refs.is_empty() {
                        if let Some(o) = v.as_object_mut() {
                            o.entry("refs").or_insert_with(|| json!(it.refs));
                        }
                    }
                    for (name, r, sm, _) in &it.parts {
                        v[*name] = if raw { r.clone() } else { sm.clone() };
                    }
                    v
                })
                .collect();
            env["items"] = json!(items);
            for (name, r, sm, _) in &out.extra {
                env[*name] = if raw { r.clone() } else { sm.clone() };
            }
            if format == Format::Layered
                && find == "capsules"
                && out.items.iter().all(|i| i.parts.is_empty())
            {
                env["more"] = json!(
                    "expand: [\"story\",\"evidence\",\"ledger\",\"comments\"] adds those layers"
                );
            }
            if let Some(m) = &more {
                env["truncated"] = json!(m);
            }
            env["ids"] = json!(ids);
            env.to_string()
        }
    };

    let structured = if format == Format::Card {
        if let Some(m) = &more {
            env["truncated"] = json!(m);
        }
        env
    } else {
        serde_json::from_str(&text).unwrap_or(Value::Null)
    };
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "structuredContent": structured,
        "isError": false,
    })
}

fn format_name(f: Format) -> &'static str {
    match f {
        Format::Card => "card",
        Format::Json => "json",
        Format::Layered => "layered",
    }
}

// ---------------------------------------------------------------------------------------
// The tools.

fn ask(ctx: &mut Ctx<'_>, args: Value) -> Result<Value, Fail> {
    let mut args = args;
    check_keys(&args, "", &ASK_KEYS, "ask")?;
    let raw_find = args
        .get("find")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    let Some(raw_find) = raw_find else {
        return Err(Fail::schema(
            "/find",
            "`find` is required (a noun)".to_owned(),
            &FINDS,
            vec![],
        ));
    };
    let canon = ALIASES
        .iter()
        .find(|(a, _)| *a == raw_find)
        .map_or(raw_find.as_str(), |(_, c)| *c)
        .to_owned();
    if !FINDS.contains(&canon.as_str()) {
        return Err(Fail::schema(
            "/find",
            format!("`{raw_find}` is not something ask can find"),
            &FINDS,
            did_you_mean(&raw_find, FINDS),
        ));
    }
    args["find"] = json!(canon);
    let find: Find = serde_json::from_value(json!(canon)).map_err(|e| serde_fail(&e, "find"))?;
    if let Some(w) = args.get("where") {
        if !w.is_null() {
            check_keys(
                w,
                "/where",
                where_keys(find),
                &format!("where (find {canon})"),
            )?;
        }
    }
    if let Some(f) = args.get("format").and_then(Value::as_str) {
        const FORMATS: [&str; 3] = ["card", "json", "layered"];
        if !FORMATS.contains(&f) {
            return Err(Fail::schema(
                "/format",
                format!("unknown format `{f}`"),
                &FORMATS,
                did_you_mean(f, FORMATS),
            ));
        }
    }
    let a: AskArgs = serde_json::from_value(args).map_err(|e| serde_fail(&e, "ask"))?;
    if !a.expand.is_empty() && !matches!(a.find, Find::Capsules | Find::Stories) {
        return Err(Fail::invalid(
            "/expand",
            format!("`expand` applies to capsules and stories, not {canon}"),
            "Drop `expand`, or ask for capsules (e.g. {\"find\":\"capsules\",\"where\":{\"id\":\"…\"},\"expand\":[\"evidence\"]}).",
        ));
    }
    let default = match a.find {
        Find::Facts | Find::Changes | Find::Comments => 20,
        Find::Stories => 50,
        Find::Watches | Find::Glossary => 32,
        _ => 10,
    };
    let limit = match a.limit {
        Some(0) => {
            return Err(Fail::invalid(
                "/limit",
                "`limit` must be at least 1".to_owned(),
                "Omit it for the default.",
            ));
        }
        Some(n) => (n as usize).min(100),
        None => default,
    };
    let mut expand = a.expand.clone();
    expand.sort_unstable();
    expand.dedup();
    let w = &a.where_;
    let mut io = Io { ctx, seq: None };
    let out = match a.find {
        Find::Capsules => find_capsules(&mut io, w, &expand, limit)?,
        Find::Stories => find_stories(&mut io, w, &expand, limit)?,
        Find::Facts => find_facts(&mut io, w, limit)?,
        Find::Instrument => find_instrument(&mut io, w)?,
        Find::Ledger => find_ledger(&mut io, w)?,
        Find::Market => find_market(&mut io, w)?,
        Find::Glossary => find_glossary(&mut io, w, limit)?,
        Find::Changes => find_changes(&mut io, w, limit)?,
        Find::Comments => find_comments(&mut io, w, limit)?,
        Find::Watches => find_watches(&mut io, w, limit)?,
        Find::Proposals => find_proposals(&mut io, w, limit)?,
    };
    // `select`: every key must exist on some item (a typo is an error, not silence).
    if !a.select.is_empty() && !out.items.is_empty() {
        let mut known: BTreeSet<String> = BTreeSet::new();
        for it in &out.items {
            for v in [&it.summary, &it.raw] {
                if let Some(o) = v.as_object() {
                    known.extend(o.keys().cloned());
                }
            }
        }
        for k in &a.select {
            if !known.contains(k) {
                let exp: Vec<&str> = known.iter().map(String::as_str).collect();
                return Err(Fail::schema(
                    "/select",
                    format!("no {canon} item has a field `{k}`"),
                    &exp,
                    did_you_mean(k, exp.iter().copied()),
                ));
            }
        }
    }
    let seq = io.seq;
    Ok(shape(
        &canon,
        a.format.unwrap_or_default(),
        &a.select,
        out,
        seq,
    ))
}

/// A capsule reference resolved to its capsule id (selectors and finding ids too).
fn capsule_id(io: &mut Io<'_, '_>, r: &CapsuleRef) -> Result<String, Fail> {
    let d = io.capsule_detail(r)?;
    Ok(d.pointer("/item/id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned())
}

fn act(ctx: &mut Ctx<'_>, args: &Value) -> Result<Value, Fail> {
    let Some(o) = args.as_object() else {
        return Err(Fail::schema(
            "",
            "act arguments must be an object".to_owned(),
            &["do"],
            vec![],
        ));
    };
    let Some(d) = o.get("do").and_then(Value::as_str) else {
        return Err(Fail::schema(
            "/do",
            "`do` is required".to_owned(),
            &ACT_DOS,
            vec![],
        ));
    };
    if HUMAN_ONLY.iter().any(|h| h.starts_with(d) || *h == d)
        || matches!(d, "accept" | "reject" | "approve" | "decide")
    {
        return Err(Fail(
            json!({"error": {"code": "forbidden", "reason": "human_only", "path": "/do",
            "message": format!("`{d}` is a person's decision; agents cannot decide proposals or findings"),
            "hint": "Propose instead (do: propose); the person accepts or rejects it on their own connection."}}),
        ));
    }
    if !ACT_DOS.contains(&d) {
        return Err(Fail::schema(
            "/do",
            format!("unknown action `{d}`"),
            &ACT_DOS,
            did_you_mean(d, ACT_DOS),
        ));
    }
    let dv: Do = serde_json::from_value(json!(d)).map_err(|e| serde_fail(&e, "do"))?;
    check_keys(args, "", act_keys(dv), &format!("act (do {d})"))?;
    let a: ActArgs = serde_json::from_value(args.clone()).map_err(|e| serde_fail(&e, "act"))?;
    let mut io = Io { ctx, seq: None };
    let reply = match a.do_ {
        Do::Comment => {
            let about = match (&a.capsule, &a.instrument) {
                (Some(r), None) => json!({"capsule": capsule_id(&mut io, r)?}),
                (None, Some(i)) => json!({"instrument": io.instrument(i, "/instrument")?}),
                _ => {
                    return Err(Fail::schema(
                        "/capsule",
                        "a comment is about exactly one of `capsule` or `instrument`".to_owned(),
                        &["capsule", "instrument"],
                        vec![],
                    ))
                }
            };
            let Some(text) = &a.text else {
                return Err(Fail::schema(
                    "/text",
                    "`text` is required".to_owned(),
                    &["text"],
                    vec![],
                ));
            };
            let mut cites = Vec::new();
            for c in &a.cites {
                cites.push(match c {
                    CapsuleRef::Id(x) => x.clone(),
                    CapsuleRef::Select(_) => capsule_id(&mut io, c)?,
                });
            }
            let mut op = json!({"op": "add_comment", "about": about,
                "lang": ser(&a.lang.unwrap_or_default()), "text": text, "cites": cites});
            if !a.external.is_empty() {
                op["external"] = ser(&a.external);
            }
            io.command(vec![op], None, None)?
        }
        Do::Watch => {
            let Some(text) = &a.text else {
                return Err(Fail::schema(
                    "/text",
                    "`text` (the request in words) is required".to_owned(),
                    &["text", "match"],
                    vec![],
                ));
            };
            let mut op = json!({"op": "add_watch", "text": text,
                "match": ser(&a.match_.clone().unwrap_or_default())});
            if let Some(f) = &a.for_ {
                op["for"] = json!(f);
            }
            if let Some(x) = a.expires_t {
                op["expires_t"] = json!(x);
            }
            if let Some(x) = a.max_matches {
                op["max_matches"] = json!(x);
            }
            io.command(vec![op], None, None)?
        }
        Do::Unwatch => {
            let Some(wid) = &a.watch else {
                return Err(Fail::schema(
                    "/watch",
                    "`watch` (the watch id) is required".to_owned(),
                    &["watch"],
                    vec![],
                ));
            };
            io.command(
                vec![json!({"op": "remove_watch", "intent": wid})],
                None,
                None,
            )?
        }
        Do::Propose => {
            if a.ops.is_empty() {
                return Err(Fail::schema(
                    "/ops",
                    "`ops` needs at least one operation".to_owned(),
                    &["ops"],
                    vec![],
                ));
            }
            for (i, op) in a.ops.iter().enumerate() {
                let name = op.get("op").and_then(Value::as_str);
                match name {
                    None => {
                        return Err(Fail::schema(
                            &format!("/ops/{i}/op"),
                            "each operation is an object with `op`".to_owned(),
                            &["op"],
                            vec![],
                        ));
                    }
                    Some(n) if HUMAN_ONLY.contains(&n) => {
                        return Err(Fail(
                            json!({"error": {"code": "forbidden", "reason": "human_only",
                            "path": format!("/ops/{i}"),
                            "message": format!("`{n}` is a person's decision; it cannot be proposed by an agent"),
                            "hint": "A person decides proposals on their own connection."}}),
                        ));
                    }
                    Some(_) => {}
                }
            }
            io.command(a.ops.clone(), Some("propose"), a.reason.as_deref())?
        }
        Do::Preview => {
            let Some(p) = &a.proposal else {
                return Err(Fail::schema(
                    "/proposal",
                    "`proposal` (id or selector) is required".to_owned(),
                    &["proposal"],
                    vec![],
                ));
            };
            let pid = io.proposal_id(p)?;
            let mut q = json!({"kind": "preview", "proposal": pid});
            if let Some(x) = a.width {
                q["width"] = json!(x);
            }
            if let Some(x) = a.height {
                q["height"] = json!(x);
            }
            let data = io.query(q)?;
            let ops: Vec<Value> = arr(&data, "changes")
                .iter()
                .filter_map(|c| c.get("op").cloned())
                .collect();
            let v = json!({"done": "preview", "proposal": data.get("proposal").cloned().unwrap_or(Value::Null),
                "changes": ops, "views": data.get("views").cloned().unwrap_or(json!([])),
                "seq": io.seq, "note": "Nothing was written. `render` with `proposal` draws it."});
            return Ok(ok_result(v));
        }
    };
    let mut v = json!({"done": d});
    if let Some(m) = reply.as_object() {
        for (k, x) in m {
            if k != "type" && k != "id" {
                v[k] = x.clone();
            }
        }
    }
    if a.do_ == Do::Propose {
        v["note"] = json!("Pending until a person accepts or rejects it; `act` do preview shows what it would change.");
    }
    Ok(ok_result(v))
}

fn ok_result(v: Value) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": v.to_string()}],
        "structuredContent": v,
        "isError": false,
    })
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
