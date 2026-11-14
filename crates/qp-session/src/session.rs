//! The reference authority: one document, its log, connections, subscriptions and
//! presence. Every inbound message goes through [`Session::handle_raw`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use qp_schema::protocol::{
    AckStatus, AttentionDirective, ChangeOp, ClientMsg, DocumentRef, ErrorCode, Event, Limits,
    LogEntry, Op, OutboundLine, Permissions, Principal, Profile, Proposal, ProposalState,
    ProtocolError, Role, Seq, ServerMsg, SessionInit, Surface, SurfaceKind, Target, Topic,
    Viewport, WriteMode, PROTOCOL_VERSION,
};
use qp_schema::{Author, CapsuleState, Document};
use serde_json::Value;

use crate::apply::{apply_all, first_overlap, touched_ids, touched_key, touched_keys};
use crate::error::{invalid, ptr, PResult, E};
use crate::expand::{
    check_condition, expand_ops, need_finding, need_proposal, summarize_ops, undone_set, Ctx,
};
use crate::locate::locate;
use crate::present::profile_for;
use crate::select;
use crate::values::{eval_condition, holds, Cache};

/// Principal id of the engine when it writes log entries itself.
pub const ENGINE_PRINCIPAL: &str = "qp";

/// Features this authority grants when asked.
const FEATURES: [&str; 2] = ["attention", "narrative"];

/// Number of attention directives kept per principal.
const MAX_DIRECTIVES: usize = 16;

/// Default limits (`docs/PROTOCOL.md` §4.1).
#[must_use]
pub fn default_limits() -> Limits {
    Limits {
        max_message_bytes: 1_048_576,
        max_ops: 64,
        max_bars: 10_000,
        max_subscriptions: 32,
        max_string: 4096,
    }
}

/// Permissions of a role (`docs/PROTOCOL.md` §9).
#[must_use]
pub fn permissions(role: Role) -> Permissions {
    let p = |read, write, propose, decide, attend, feed| Permissions {
        read,
        write,
        propose,
        decide,
        attend,
        feed,
    };
    match role {
        Role::Owner => p(true, true, true, true, true, false),
        Role::Editor => p(true, true, true, false, true, false),
        Role::Proposer => p(true, false, true, false, true, false),
        Role::Viewer => p(true, false, false, false, false, false),
        Role::Feed => p(false, false, false, false, false, true),
        Role::Host => p(true, true, false, false, false, true),
    }
}

#[derive(Debug)]
pub(crate) struct Sub {
    pub id: String,
    pub topic: Topic,
    /// Last value of a watched condition (edge detection).
    pub last: Option<bool>,
}

#[derive(Debug)]
pub(crate) struct Conn {
    pub id: String,
    pub principal: String,
    pub role: Role,
    pub surface: Surface,
    pub profile: Profile,
    pub subs: Vec<Sub>,
    /// View id -> visible range.
    pub viewports: BTreeMap<String, (u64, u64)>,
    /// Last viewport set by input.
    pub current: Option<Viewport>,
    /// Input counter value of the last input (0 = none).
    pub last_input: u64,
    /// Clock at the last input.
    pub last_at: Option<i64>,
}

impl Conn {
    pub(crate) fn has_screen(&self) -> bool {
        matches!(
            self.surface.kind,
            SurfaceKind::Desktop | SurfaceKind::Phone | SurfaceKind::Watch
        )
    }
}

/// The reference authority for one document.
pub struct Session {
    pub(crate) doc: Document,
    pub(crate) initial: Document,
    pub(crate) log: Vec<LogEntry>,
    pub(crate) principals: Vec<Principal>,
    pub(crate) clock: i64,
    pub(crate) epoch: u64,
    pub(crate) ttl: Option<u64>,
    pub(crate) limits: Limits,
    pub(crate) conns: Vec<Conn>,
    pub(crate) cache: Cache,
    pub(crate) intent_state: BTreeMap<String, bool>,
    pub(crate) directives: BTreeMap<String, Vec<AttentionDirective>>,
    pub(crate) focus: BTreeMap<String, Target>,
    pub(crate) att_counter: u64,
    pub(crate) input_counter: u64,
    pub(crate) schema: Value,
}

/// Resolves `rel` under `root`, refusing absolute paths and paths that leave `root`.
///
/// # Errors
/// The path escapes the root or does not exist.
pub fn confine(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let p = Path::new(rel);
    if p.is_absolute()
        || p.components()
            .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(format!(
            "path `{rel}` must be relative and stay inside the root"
        ));
    }
    let root_c = root
        .canonicalize()
        .map_err(|e| format!("root {}: {e}", root.display()))?;
    let full = root_c
        .join(p)
        .canonicalize()
        .map_err(|e| format!("{rel}: {e}"))?;
    if !full.starts_with(&root_c) {
        return Err(format!("path `{rel}` leaves the root"));
    }
    Ok(full)
}

/// Reads a JSON file into `T`.
///
/// # Errors
/// I/O or JSON errors.
pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn out(conn: &str, msg: ServerMsg) -> OutboundLine {
    OutboundLine {
        conn: conn.to_owned(),
        msg,
    }
}

fn err_line(conn: &str, id: Option<String>, error: ProtocolError) -> OutboundLine {
    out(conn, ServerMsg::Error { id, error })
}

/// Ops that cannot become proposals (`docs/PROTOCOL.md` §4.7).
fn proposable(op: &Op) -> bool {
    !matches!(
        op,
        Op::AcceptProposal { .. }
            | Op::RejectProposal { .. }
            | Op::Undo { .. }
            | Op::AppendBars { .. }
            | Op::UpdateLastBar { .. }
    )
}

fn op_name(op: &Op) -> String {
    serde_json::to_value(op)
        .ok()
        .and_then(|v| v.get("op").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_default()
}

/// Whether `perms` allow writing `op` directly.
fn allowed(op: &Op, perms: &Permissions) -> bool {
    match op {
        Op::AppendBars { .. } | Op::UpdateLastBar { .. } => perms.feed || perms.write,
        Op::ApproveFinding { .. }
        | Op::RejectFinding { .. }
        | Op::AcceptProposal { .. }
        | Op::RejectProposal { .. } => perms.decide,
        _ => perms.write,
    }
}

impl Session {
    /// Starts an authority. A `DocumentRef::Path` is resolved under `repo_root` and may
    /// not leave it.
    ///
    /// # Errors
    /// The document cannot be read or parsed.
    pub fn new(init: SessionInit, repo_root: &Path) -> Result<Self, String> {
        let doc: Document = match &init.document {
            DocumentRef::Inline(d) => (**d).clone(),
            DocumentRef::Path(p) => read_json(&confine(repo_root, p)?)?,
        };
        // Canonical form of every view (GRAMMAR.md §11); the log replays from it.
        let doc = crate::canon::canon_document(&doc)?;
        let schema = schemars::schema_for!(ClientMsg).to_value();
        let mut s = Self {
            initial: doc.clone(),
            doc,
            log: Vec::new(),
            principals: init.principals,
            clock: init.clock,
            epoch: init.epoch,
            ttl: init.proposal_ttl_ms,
            limits: default_limits(),
            conns: Vec::new(),
            cache: Cache::default(),
            intent_state: BTreeMap::new(),
            directives: BTreeMap::new(),
            focus: BTreeMap::new(),
            att_counter: 0,
            input_counter: 0,
            schema,
        };
        s.eval_intents(0);
        Ok(s)
    }

    /// The current document.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.doc
    }

    /// The initial document (before any log entry).
    #[must_use]
    pub fn initial_document(&self) -> &Document {
        &self.initial
    }

    /// The log.
    #[must_use]
    pub fn log(&self) -> &[LogEntry] {
        &self.log
    }

    /// The last clock value.
    #[must_use]
    pub fn clock(&self) -> i64 {
        self.clock
    }

    /// The limits this authority enforces.
    #[must_use]
    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    /// One line of the line transport (`{"conn":…,"msg":…}`). `msg` may be a JSON
    /// object (the message) or a JSON string (the raw text of the message, e.g. to send
    /// invalid JSON). Framing errors go to connection `""`.
    pub fn handle_frame(&mut self, line: &str) -> Vec<OutboundLine> {
        let frame_err = |code: ErrorCode, msg: String, path: Option<&str>| {
            let mut e = E::new(code, msg);
            if let Some(p) = path {
                e = e.at(p);
            }
            vec![err_line("", None, e.build())]
        };
        let limit = usize::try_from(self.limits.max_message_bytes)
            .unwrap_or(usize::MAX)
            .saturating_add(4096);
        if line.len() > limit {
            return frame_err(
                ErrorCode::Limit,
                format!("line exceeds {limit} bytes"),
                None,
            );
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => return frame_err(ErrorCode::Parse, format!("frame is not JSON: {e}"), None),
        };
        let Some(conn) = v.get("conn").and_then(Value::as_str) else {
            return frame_err(
                ErrorCode::Schema,
                "frame needs a string `conn`".to_owned(),
                Some("/conn"),
            );
        };
        if let Some(k) = v
            .as_object()
            .and_then(|o| o.keys().find(|k| *k != "conn" && *k != "msg"))
        {
            let e = E::new(ErrorCode::Schema, format!("unknown frame field `{k}`"))
                .at(ptr("", k))
                .expected(["conn", "msg"])
                .build();
            return vec![err_line(conn, None, e)];
        }
        let text = match v.get("msg") {
            None => {
                let e = E::new(ErrorCode::Schema, "frame needs `msg`")
                    .at("/msg")
                    .build();
                return vec![err_line(conn, None, e)];
            }
            Some(Value::String(s)) => s.clone(),
            Some(m) => m.to_string(),
        };
        self.handle_raw(conn, &text)
    }

    /// THE boundary: one raw inbound message on connection `conn`. Checks the size,
    /// parses (errors `parse`, `schema` with a JSON pointer, `limit`), then handles it.
    pub fn handle_raw(&mut self, conn: &str, line: &str) -> Vec<OutboundLine> {
        if line.len() as u64 > self.limits.max_message_bytes {
            let e = E::new(
                ErrorCode::Limit,
                format!(
                    "message is {} bytes; the limit is {}",
                    line.len(),
                    self.limits.max_message_bytes
                ),
            )
            .expected([self.limits.max_message_bytes.to_string()])
            .build();
            return vec![err_line(conn, None, e)];
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                let text = e.to_string();
                let code = if text.contains("recursion limit") {
                    ErrorCode::Limit
                } else {
                    ErrorCode::Parse
                };
                let pe = E::new(code, format!("not valid JSON: {text}"))
                    .hint("Send one JSON object per message.")
                    .build();
                return vec![err_line(conn, None, pe)];
            }
        };
        let id = value.get("id").and_then(Value::as_str).map(str::to_owned);
        let mut value = value;
        // Reference by meaning (§17): selectors become ids before the typed parse, so
        // the typed message, the log and proposals only ever hold ids.
        let resolved = match self.resolve_selectors(conn, &mut value) {
            Ok(r) => r,
            Err(e) => return vec![err_line(conn, id, e)],
        };
        let msg: ClientMsg = match serde_path_to_error::deserialize(&value) {
            Ok(m) => m,
            Err(e) => return vec![err_line(conn, id, self.schema_error(&value, &e))],
        };
        if let Err(e) = self.check_limits(&value, &msg) {
            return vec![err_line(conn, id, e)];
        }
        self.handle_resolved(conn, msg, resolved)
    }

    /// Finds the selectors of a raw message, resolves them against the document (for a
    /// command: op by op, each against the document as the earlier ops of the same
    /// command leave it) and writes the ids in their place. Returns `{pointer -> id}`.
    fn resolve_selectors(&self, conn: &str, msg: &mut Value) -> PResult<BTreeMap<String, String>> {
        let mut out = BTreeMap::new();
        let groups = select::sites(msg);
        let Some(first) = groups
            .iter()
            .flatten()
            .find(|s| select::is_selector(msg, s))
            .cloned()
        else {
            return Ok(out);
        };
        let Some(ci) = self.conn_index(conn) else {
            return Err(E::new(
                ErrorCode::Handshake,
                "the first message on a connection must be `hello`",
            )
            .hint("Send {\"type\":\"hello\",\"protocol\":\"qp/0\",\"principal\":…,\"surface\":{\"kind\":…}} first.")
            .build());
        };
        let role = self.conns[ci].role;
        let perms = permissions(role);
        if !perms.read {
            return Err(E::new(
                ErrorCode::Forbidden,
                format!(
                    "role `{}` may not read the document, so it cannot use selectors",
                    role_str(role)
                ),
            )
            .at(first.path)
            .hint("Use the item's id instead of a selector.")
            .build());
        }
        let is_command = msg.get("type").and_then(Value::as_str) == Some("command");
        if !is_command {
            let r = select::Resolver {
                doc: &self.doc,
                root: Some(&self.schema),
            };
            for g in &groups {
                select::resolve_group(&r, msg, g, &mut out)?;
            }
            return Ok(out);
        }
        if groups.len() > self.limits.max_ops as usize {
            // The op limit is reported after parsing.
            return Ok(out);
        }
        // Op by op on a scratch copy: a selector may name an item an earlier op of the
        // same command creates or changes. A failing earlier op leaves the copy as it
        // was; the command itself reports that failure.
        let principal = self.conns[ci].principal.clone();
        let author = self
            .principal(&principal)
            .map(|p| p.author.clone())
            .ok_or_else(|| E::new(ErrorCode::Handshake, "principal vanished").build())?;
        let seq = self.seq() + 1;
        let mut ctx = Ctx {
            author,
            approved_by: None,
            at: self.clock,
            seq,
            id_seq: seq,
            n: 0,
            created: Vec::new(),
            speaker: &principal,
            is_owner: role == Role::Owner,
            can_decide: perms.decide,
            log: &self.log,
            undoes: None,
        };
        let mut scratch = self.doc.clone();
        let last = groups
            .iter()
            .rposition(|g| g.iter().any(|s| select::is_selector(msg, s)))
            .unwrap_or(0);
        for (i, g) in groups.iter().enumerate().take(last + 1) {
            {
                let r = select::Resolver {
                    doc: &scratch,
                    root: Some(&self.schema),
                };
                select::resolve_group(&r, msg, g, &mut out)?;
            }
            if i == last {
                break;
            }
            let p = ptr("/ops", i);
            let Some(op) = msg
                .pointer(&p)
                .and_then(|v| serde_json::from_value::<Op>(v.clone()).ok())
            else {
                continue;
            };
            if let Ok(cs) = crate::expand::expand_op(&scratch, &op, &p, &mut ctx) {
                let mut next = scratch.clone();
                if apply_all(&mut next, &cs).is_ok() {
                    scratch = next;
                }
            }
        }
        Ok(out)
    }

    fn schema_error(
        &self,
        value: &Value,
        e: &serde_path_to_error::Error<serde_json::Error>,
    ) -> ProtocolError {
        if let Some(v) = locate(value, &self.schema, &self.schema) {
            return select::violation_error(&v, "");
        }
        let mut path = String::new();
        for seg in e.path().iter() {
            match seg {
                serde_path_to_error::Segment::Seq { index } => path = ptr(&path, index),
                serde_path_to_error::Segment::Map { key } => path = ptr(&path, key),
                serde_path_to_error::Segment::Enum { .. }
                | serde_path_to_error::Segment::Unknown => {}
            }
        }
        let inner = e.inner().to_string();
        let inner = inner.split(" at line ").next().unwrap_or(&inner).to_owned();
        E::new(ErrorCode::Schema, inner).at(path).build()
    }

    fn check_limits(&self, value: &Value, msg: &ClientMsg) -> PResult<()> {
        fn walk(v: &Value, path: &str, max: usize) -> Option<String> {
            match v {
                Value::String(s) if s.len() > max => Some(path.to_owned()),
                Value::Array(a) => a
                    .iter()
                    .enumerate()
                    .find_map(|(i, x)| walk(x, &ptr(path, i), max)),
                Value::Object(o) => o.iter().find_map(|(k, x)| walk(x, &ptr(path, k), max)),
                _ => None,
            }
        }
        let max_string = self.limits.max_string as usize;
        if let Some(p) = walk(value, "", max_string) {
            return Err(E::new(
                ErrorCode::Limit,
                format!("string longer than {max_string} bytes"),
            )
            .at(p)
            .expected([max_string.to_string()])
            .build());
        }
        if let ClientMsg::Command { ops, .. } = msg {
            if ops.len() > self.limits.max_ops as usize {
                return Err(E::new(
                    ErrorCode::Limit,
                    format!("{} ops; the limit is {}", ops.len(), self.limits.max_ops),
                )
                .at("/ops")
                .expected([self.limits.max_ops.to_string()])
                .build());
            }
            for (i, op) in ops.iter().enumerate() {
                let (n, path) = match op {
                    Op::AppendBars { bars, .. } => (bars.len(), "bars"),
                    Op::PutSeries { series } => {
                        (series.bars().map_or(0, <[qp_schema::Bar]>::len), "series")
                    }
                    _ => continue,
                };
                if n > self.limits.max_bars as usize {
                    return Err(E::new(
                        ErrorCode::Limit,
                        format!("{n} bars; the limit is {}", self.limits.max_bars),
                    )
                    .at(ptr(&ptr("/ops", i), path))
                    .expected([self.limits.max_bars.to_string()])
                    .build());
                }
            }
        }
        Ok(())
    }

    fn conn_index(&self, conn: &str) -> Option<usize> {
        self.conns.iter().position(|c| c.id == conn)
    }

    pub(crate) fn principal(&self, id: &str) -> Option<&Principal> {
        self.principals.iter().find(|p| p.id == id)
    }

    /// Principal id whose author is `a` (fallback: the author's name).
    pub(crate) fn principal_of(&self, a: &Author) -> String {
        if let Some(p) = self.principals.iter().find(|p| &p.author == a) {
            return p.id.clone();
        }
        match a {
            Author::Human { name } | Author::Agent { name, .. } | Author::Engine { name, .. } => {
                name.clone()
            }
        }
    }

    /// Handles a parsed message (its id fields hold ids; selectors are resolved by
    /// [`Session::handle_raw`]).
    pub fn handle(&mut self, conn: &str, msg: ClientMsg) -> Vec<OutboundLine> {
        self.handle_resolved(conn, msg, BTreeMap::new())
    }

    /// Handles a parsed message whose selectors resolved to `resolved` (§17).
    fn handle_resolved(
        &mut self,
        conn: &str,
        msg: ClientMsg,
        resolved: BTreeMap<String, String>,
    ) -> Vec<OutboundLine> {
        let ci = self.conn_index(conn);
        if let ClientMsg::Hello {
            protocol,
            principal,
            surface,
            features,
        } = msg
        {
            return self.hello(conn, ci, &protocol, &principal, surface, &features);
        }
        let Some(ci) = ci else {
            let id = msg_id(&msg);
            let e = E::new(
                ErrorCode::Handshake,
                "the first message on a connection must be `hello`",
            )
            .hint("Send {\"type\":\"hello\",\"protocol\":\"qp/0\",\"principal\":…,\"surface\":{\"kind\":…}} first.")
            .build();
            return vec![err_line(conn, id, e)];
        };
        match msg {
            ClientMsg::Hello { .. } => unreachable!("handled above"),
            ClientMsg::Command {
                id,
                ops,
                mode,
                base,
                reason,
            } => match self.command(ci, &id, &ops, mode, (base, reason), resolved) {
                Ok(lines) => lines,
                Err(e) => vec![err_line(conn, Some(id), e)],
            },
            ClientMsg::Query { id, query } => {
                if !permissions(self.conns[ci].role).read {
                    return vec![err_line(conn, Some(id), forbidden_read())];
                }
                match self.query(ci, &query) {
                    Ok(data) => vec![out(
                        conn,
                        ServerMsg::Result {
                            id,
                            seq: self.seq(),
                            now: self.clock(),
                            data,
                            resolved,
                        },
                    )],
                    Err(e) => vec![err_line(conn, Some(id), e)],
                }
            }
            ClientMsg::Subscribe { id, topic } => match self.subscribe(ci, &id, topic) {
                Ok(()) => {
                    let mut a = ack(id, AckStatus::Subscribed);
                    if let ServerMsg::Ack { resolved: r, .. } = &mut a {
                        *r = resolved;
                    }
                    vec![out(conn, a)]
                }
                Err(e) => vec![err_line(conn, Some(id), e)],
            },
            ClientMsg::Unsubscribe { id, sub } => {
                let c = &mut self.conns[ci];
                if let Some(i) = c.subs.iter().position(|s| s.id == sub) {
                    c.subs.remove(i);
                    vec![out(conn, ack(id, AckStatus::Unsubscribed))]
                } else {
                    let e = E::new(ErrorCode::NotFound, format!("no subscription `{sub}`"))
                        .at("/sub")
                        .dym(&sub, c.subs.iter().map(|s| s.id.as_str()))
                        .build();
                    vec![err_line(conn, Some(id), e)]
                }
            }
            ClientMsg::Input { id, gesture } => {
                if !permissions(self.conns[ci].role).read {
                    return vec![err_line(conn, Some(id), forbidden_read())];
                }
                match self.input(ci, &gesture) {
                    Ok((data, mut events)) => {
                        let mut v = vec![out(
                            conn,
                            ServerMsg::Result {
                                id,
                                seq: self.seq(),
                                now: self.clock(),
                                data,
                                resolved: BTreeMap::new(),
                            },
                        )];
                        v.append(&mut events);
                        v
                    }
                    Err(e) => vec![err_line(conn, Some(id), e)],
                }
            }
            ClientMsg::Attend { id, directive } => match self.attend(ci, directive) {
                Ok((did, mut events)) => {
                    let mut v = vec![out(
                        conn,
                        ServerMsg::Ack {
                            resolved,
                            id,
                            status: AckStatus::Delivered,
                            seq: None,
                            created: vec![did],
                            proposal: None,
                            undoes: None,
                        },
                    )];
                    v.append(&mut events);
                    v
                }
                Err(e) => vec![err_line(conn, Some(id), e)],
            },
            ClientMsg::Clock { now } => match self.tick(ci, now) {
                Ok(lines) => lines,
                Err(e) => vec![err_line(conn, None, e)],
            },
        }
    }

    pub(crate) fn seq(&self) -> Seq {
        self.log.len() as Seq
    }

    fn hello(
        &mut self,
        conn: &str,
        ci: Option<usize>,
        protocol: &str,
        principal: &str,
        surface: Surface,
        features: &[String],
    ) -> Vec<OutboundLine> {
        let fail = |e: E| vec![err_line(conn, None, e.build())];
        if ci.is_some() {
            return fail(E::new(
                ErrorCode::Handshake,
                format!("connection `{conn}` already said hello"),
            ));
        }
        if protocol != PROTOCOL_VERSION {
            return fail(
                E::new(
                    ErrorCode::Handshake,
                    format!("unsupported protocol `{protocol}`"),
                )
                .at("/protocol")
                .expected([PROTOCOL_VERSION]),
            );
        }
        let Some(p) = self.principal(principal).cloned() else {
            return fail(
                E::new(
                    ErrorCode::Handshake,
                    format!("unknown principal `{principal}`"),
                )
                .at("/principal")
                .dym(principal, self.principals.iter().map(|p| p.id.as_str())),
            );
        };
        let profile = profile_for(surface.kind, surface.max_depth);
        let mut granted: Vec<String> = Vec::new();
        for f in features {
            if FEATURES.contains(&f.as_str()) && !granted.contains(f) {
                granted.push(f.clone());
            }
        }
        self.conns.push(Conn {
            id: conn.to_owned(),
            principal: p.id.clone(),
            role: p.role,
            surface,
            profile: profile.clone(),
            subs: Vec::new(),
            viewports: BTreeMap::new(),
            current: None,
            last_input: 0,
            last_at: None,
        });
        vec![out(
            conn,
            ServerMsg::Welcome {
                // Only principals that may read get the summary (a feed may not).
                summary: permissions(p.role)
                    .read
                    .then(|| crate::summary::summarize_doc(&self.doc)),
                protocol: PROTOCOL_VERSION.to_owned(),
                doc: self.doc.id.clone(),
                principal: p.id,
                role: p.role,
                permissions: permissions(p.role),
                seq: self.seq(),
                now: self.clock(),
                epoch: self.epoch,
                profile,
                features: granted,
                limits: self.limits.clone(),
            },
        )]
    }

    // -----------------------------------------------------------------------
    // Commands
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    fn command(
        &mut self,
        ci: usize,
        id: &str,
        ops: &[Op],
        mode: WriteMode,
        (base, reason): (Option<Seq>, Option<String>),
        resolved: BTreeMap<String, String>,
    ) -> PResult<Vec<OutboundLine>> {
        if ops.is_empty() {
            return Err(invalid("a command needs at least one op", "/ops"));
        }
        if ops.len() > 1 {
            if let Some(i) = ops
                .iter()
                .position(|o| matches!(o, Op::AcceptProposal { .. } | Op::Undo { .. }))
            {
                return Err(invalid(
                    format!("`{}` must be the only op of its command", op_name(&ops[i])),
                    ptr("/ops", i),
                ));
            }
        }
        let role = self.conns[ci].role;
        let perms = permissions(role);
        let principal = self.conns[ci].principal.clone();
        let forbid = |i: usize, why: String, hint: &str| -> ProtocolError {
            E::new(ErrorCode::Forbidden, why)
                .at(ptr("/ops", i))
                .hint(hint)
                .build()
        };
        let role_name = role_str(role);
        let first_denied = ops.iter().position(|o| !allowed(o, &perms));
        let first_unproposable = ops.iter().position(|o| !proposable(o));
        let direct = match mode {
            WriteMode::Direct => {
                if let Some(i) = first_denied {
                    let hint = if perms.propose && proposable(&ops[i]) {
                        "Use mode auto (or propose) to create a proposal instead."
                    } else {
                        "Ask a principal with the required role."
                    };
                    return Err(forbid(
                        i,
                        format!("role `{role_name}` may not `{}` directly", op_name(&ops[i])),
                        hint,
                    ));
                }
                true
            }
            WriteMode::Auto => match first_denied {
                None => true,
                Some(i) => {
                    if !perms.propose {
                        return Err(forbid(
                            i,
                            format!("role `{role_name}` may not `{}`", op_name(&ops[i])),
                            "Ask a principal with the required role.",
                        ));
                    }
                    if let Some(j) = first_unproposable {
                        return Err(forbid(
                            j,
                            format!(
                                "role `{role_name}` may not `{}`, and it cannot be proposed",
                                op_name(&ops[j])
                            ),
                            "Ask a principal with the required role.",
                        ));
                    }
                    false
                }
            },
            WriteMode::Propose => {
                if !perms.propose {
                    return Err(forbid(
                        0,
                        format!("role `{role_name}` may not propose"),
                        "Ask a principal with the propose permission.",
                    ));
                }
                if let Some(j) = first_unproposable {
                    return Err(forbid(
                        j,
                        format!("`{}` cannot be proposed", op_name(&ops[j])),
                        "Send it with mode direct if you have the permission.",
                    ));
                }
                false
            }
        };
        let author = self
            .principal(&principal)
            .map(|p| p.author.clone())
            .ok_or_else(|| E::new(ErrorCode::Handshake, "principal vanished").build())?;
        let seq = self.seq() + 1;
        let conn = self.conns[ci].id.clone();

        if direct {
            if let [Op::AcceptProposal { proposal, note }] = ops {
                return self.accept(
                    &conn,
                    id,
                    (proposal, note.clone()),
                    &principal,
                    (base, reason),
                    resolved,
                );
            }
            let mut ctx = Ctx {
                author,
                approved_by: None,
                at: self.clock,
                seq,
                id_seq: seq,
                n: 0,
                created: Vec::new(),
                speaker: &principal,
                is_owner: role == Role::Owner,
                can_decide: perms.decide,
                log: &self.log,
                undoes: None,
            };
            let ex = expand_ops(&self.doc, ops, "/ops", &mut ctx)?;
            let created = ctx.created;
            let undoes = ctx.undoes;
            self.base_check(base, &ex.changes, &ex.origin)?;
            let entry = LogEntry {
                reason: reason.clone(),
                seq,
                at: self.clock,
                author: principal.clone(),
                approved_by: None,
                cause: Some(id.to_owned()),
                request: ops.to_vec(),
                changes: ex.changes,
                inverse: ex.inverse,
                undoes,
                summary: match undoes {
                    Some(u) => format!("undo {u}"),
                    None => summarize_ops(ops, &created),
                },
            };
            self.commit(entry, ex.doc);
            let mut lines = vec![out(
                &conn,
                ServerMsg::Ack {
                    resolved,
                    id: id.to_owned(),
                    status: AckStatus::Applied,
                    seq: Some(seq),
                    created,
                    proposal: None,
                    undoes,
                },
            )];
            lines.extend(self.settle(seq));
            return Ok(lines);
        }

        // Proposal: validate by a dry run (ids assigned now, reused on acceptance).
        let mut ctx = Ctx {
            author: author.clone(),
            approved_by: None,
            at: self.clock,
            seq,
            id_seq: seq,
            n: 0,
            created: Vec::new(),
            speaker: &principal,
            is_owner: role == Role::Owner,
            can_decide: perms.decide,
            log: &self.log,
            undoes: None,
        };
        let ex = expand_ops(&self.doc, ops, "/ops", &mut ctx)?;
        let created = ctx.created;
        self.base_check(base, &ex.changes, &ex.origin)?;
        let pid = format!("prop-{seq}");
        let proposal = Proposal {
            id: pid.clone(),
            ops: ops.to_vec(),
            reason: reason.clone(),
            state: ProposalState::Pending,
            provenance: qp_schema::Provenance {
                author,
                at: self.clock,
                based_on: Vec::new(),
                approved_by: None,
            },
            expires_at: self.ttl.map(|t| {
                self.clock
                    .saturating_add(i64::try_from(t).unwrap_or(i64::MAX))
            }),
            decided_by: None,
            note: None,
            decided_seq: None,
        };
        let changes = vec![ChangeOp::PutProposal {
            proposal,
            index: None,
        }];
        let mut doc = self.doc.clone();
        let inverse = apply_all(&mut doc, &changes)
            .map_err(|m| invalid(format!("cannot store proposal: {m}"), "/ops"))?;
        let entry = LogEntry {
            reason: reason.clone(),
            seq,
            at: self.clock,
            author: principal,
            approved_by: None,
            cause: Some(id.to_owned()),
            request: ops.to_vec(),
            changes,
            inverse,
            undoes: None,
            summary: format!("propose {pid}: {}", summarize_ops(ops, &created)),
        };
        self.commit(entry, doc);
        let mut lines = vec![out(
            &conn,
            ServerMsg::Ack {
                resolved,
                id: id.to_owned(),
                status: AckStatus::Proposed,
                seq: Some(seq),
                created,
                proposal: Some(pid),
                undoes: None,
            },
        )];
        lines.extend(self.settle(seq));
        Ok(lines)
    }

    fn accept(
        &mut self,
        conn: &str,
        id: &str,
        (pid, note): (&str, Option<String>),
        decider: &str,
        (base, reason): (Option<Seq>, Option<String>),
        resolved: BTreeMap<String, String>,
    ) -> PResult<Vec<OutboundLine>> {
        let pr = need_proposal(&self.doc, pid, "/ops/0/proposal")?.clone();
        let seq = self.seq() + 1;
        let proposer = self.principal_of(&pr.provenance.author);
        let proposer_owner = self
            .principal(&proposer)
            .is_some_and(|p| p.role == Role::Owner);
        let decider_author = self
            .principal(decider)
            .map(|p| p.author.clone())
            .ok_or_else(|| E::new(ErrorCode::Handshake, "principal vanished").build())?;
        let id_seq = pid
            .strip_prefix("prop-")
            .and_then(|s| s.parse::<Seq>().ok())
            .unwrap_or(seq);
        let mut ctx = Ctx {
            author: pr.provenance.author.clone(),
            approved_by: Some(decider_author.clone()),
            at: self.clock,
            seq,
            id_seq,
            n: 0,
            created: Vec::new(),
            speaker: &proposer,
            is_owner: proposer_owner,
            can_decide: false,
            log: &self.log,
            undoes: None,
        };
        let ex = expand_ops(&self.doc, &pr.ops, "/ops", &mut ctx).map_err(|mut e| {
            e.message = format!(
                "proposal `{pid}` cannot be applied now: {} (at {} of the proposal)",
                e.message,
                e.path.as_deref().unwrap_or("/ops")
            );
            e.path = Some("/ops/0".to_owned());
            e.hint =
                Some("The proposal stays pending; reject it or fix the document first.".to_owned());
            e
        })?;
        let created = ctx.created;
        let mut decided = pr.clone();
        decided.state = ProposalState::Accepted;
        decided.decided_by = Some(decider_author);
        decided.note = note;
        decided.decided_seq = Some(seq);
        let mut doc = ex.doc;
        let mut changes = ex.changes;
        let tail = vec![ChangeOp::PutProposal {
            proposal: decided,
            index: None,
        }];
        let mut inverse = apply_all(&mut doc, &tail)
            .map_err(|m| invalid(format!("cannot store proposal: {m}"), "/ops/0"))?;
        inverse.extend(ex.inverse);
        changes.extend(tail);
        let origin = vec![0; changes.len()];
        self.base_check(base, &changes, &origin)?;
        let entry = LogEntry {
            reason: reason.or_else(|| pr.reason.clone()),
            seq,
            at: self.clock,
            author: proposer,
            approved_by: Some(decider.to_owned()),
            cause: Some(id.to_owned()),
            request: pr.ops.clone(),
            changes,
            inverse,
            undoes: None,
            summary: format!("accept {pid}: {}", summarize_ops(&pr.ops, &created)),
        };
        self.commit(entry, doc);
        let mut lines = vec![out(
            conn,
            ServerMsg::Ack {
                resolved,
                id: id.to_owned(),
                status: AckStatus::Applied,
                seq: Some(seq),
                created,
                proposal: None,
                undoes: None,
            },
        )];
        lines.extend(self.settle(seq));
        Ok(lines)
    }

    /// `base` check (§8.2): an item the command touches changed in an entry after
    /// `base` that is not undone (undo pairs both after `base` cancel out).
    fn base_check(&self, base: Option<Seq>, changes: &[ChangeOp], origin: &[usize]) -> PResult<()> {
        let Some(b) = base else { return Ok(()) };
        if b > self.seq() {
            // A base from the future cannot be checked; accepting it would silently
            // switch conflict detection off (audit O2), e.g. for a stale epoch (§8.4).
            return Err(E::new(
                ErrorCode::Invalid,
                format!("base {b} is after the current log position {}", self.seq()),
            )
            .at("/base")
            .expected([format!("0..={}", self.seq())])
            .hint("Use the seq from `welcome` or from a `result`; after a hand-over, reconnect.")
            .build());
        }
        let undone = undone_set(&self.log);
        let mut conflicts = Vec::new();
        let mut first: Option<(String, Seq, String, usize)> = None;
        for e in self.log.iter().filter(|e| e.seq > b) {
            if undone.contains(&e.seq) || e.undoes.is_some_and(|x| x > b) {
                continue;
            }
            let t = touched_keys(&e.changes);
            for (k, c) in changes.iter().enumerate() {
                let key = std::iter::once(touched_key(c)).collect();
                if let Some(id) = first_overlap(&key, &t) {
                    if first.is_none() {
                        first = Some((id, e.seq, e.author.clone(), origin[k]));
                    }
                    conflicts.push(e.seq);
                    break;
                }
            }
        }
        let Some((id, s, by, op)) = first else {
            return Ok(());
        };
        conflicts.sort_unstable();
        conflicts.dedup();
        let latest = conflicts.last().copied().unwrap_or(s);
        Err(E::new(
            ErrorCode::Conflict,
            format!("`{id}` changed after base {b} (seq {s} by {by})"),
        )
        .at(ptr("/ops", op))
        .conflicts(conflicts)
        .hint(format!(
            "Read the item again and resend with base={latest}, or omit base."
        ))
        .build())
    }

    fn commit(&mut self, entry: LogEntry, doc: Document) {
        if entry.changes.iter().any(|c| {
            matches!(
                c,
                ChangeOp::PutSeries { .. }
                    | ChangeOp::Delete { .. }
                    | ChangeOp::AppendBars { .. }
                    | ChangeOp::TruncateBars { .. }
                    | ChangeOp::SetLastBar { .. }
            )
        }) {
            self.cache.clear();
        }
        self.doc = doc;
        self.log.push(entry);
    }

    /// Writes an engine entry (author `qp`).
    fn engine_entry(&mut self, changes: Vec<ChangeOp>, summary: String) -> Option<Seq> {
        let mut doc = self.doc.clone();
        let inverse = apply_all(&mut doc, &changes).ok()?;
        let seq = self.seq() + 1;
        self.commit(
            LogEntry {
                reason: None,
                seq,
                at: self.clock,
                author: ENGINE_PRINCIPAL.to_owned(),
                approved_by: None,
                cause: None,
                request: Vec::new(),
                changes,
                inverse,
                undoes: None,
                summary,
            },
            doc,
        );
        Some(seq)
    }

    /// After entry `seq`: engine follow-up entries and all notifications, in the order of
    /// §15 (per entry: change, finding_state, proposal; then condition_met, then
    /// intent_triggered).
    fn settle(&mut self, seq: Seq) -> Vec<OutboundLine> {
        let mut entries = vec![seq];
        let (mut conds, mut intents) = self.eval_watchers(seq);
        for s in self.invalidations(seq) {
            entries.push(s);
            let (c, i) = self.eval_watchers(s);
            conds.extend(c);
            intents.extend(i);
        }
        let mut lines = Vec::new();
        for s in entries {
            lines.extend(self.entry_events(s));
        }
        lines.extend(conds);
        lines.extend(intents);
        lines
    }

    /// Engine invalidations triggered by entry `seq` (§8.1): after a change of series
    /// data or study parameters, every live capsule; after a finding change, that
    /// finding's live capsules.
    fn invalidations(&mut self, seq: Seq) -> Vec<Seq> {
        let entry = &self.log[usize::try_from(seq - 1).unwrap_or(0)];
        let all = entry.changes.iter().any(|c| {
            matches!(
                c,
                ChangeOp::PutSeries { .. }
                    | ChangeOp::AppendBars { .. }
                    | ChangeOp::TruncateBars { .. }
                    | ChangeOp::SetLastBar { .. }
            )
        });
        let findings: BTreeSet<String> = entry
            .changes
            .iter()
            .filter_map(|c| match c {
                ChangeOp::PutFinding { finding, .. } => Some(finding.id.clone()),
                _ => None,
            })
            .collect();
        if !all && findings.is_empty() {
            return Vec::new();
        }
        let candidates: Vec<(String, String)> = self
            .doc
            .capsules
            .iter()
            .filter(|c| c.state == CapsuleState::Live && (all || findings.contains(&c.finding)))
            .map(|c| (c.id.clone(), c.finding.clone()))
            .collect();
        let mut out = Vec::new();
        for (cap, fnd) in candidates {
            let Some(f) = self.doc.findings.iter().find(|f| f.id == fnd) else {
                continue;
            };
            let cond = f.invalidation.clone();
            if holds(&self.doc, &mut self.cache, &cond) {
                let changes = vec![ChangeOp::SetCapsuleState {
                    capsule: cap.clone(),
                    state: CapsuleState::Invalidated,
                }];
                if let Some(s) = self.engine_entry(
                    changes,
                    format!("invalidate {cap}: invalidation of {fnd} holds"),
                ) {
                    out.push(s);
                }
            }
        }
        out
    }

    /// Edge-triggered condition subscriptions and intents after entry `seq`.
    fn eval_watchers(&mut self, seq: Seq) -> (Vec<OutboundLine>, Vec<OutboundLine>) {
        let mut conds = Vec::new();
        for ci in 0..self.conns.len() {
            for si in 0..self.conns[ci].subs.len() {
                let Topic::Condition { condition } = &self.conns[ci].subs[si].topic else {
                    continue;
                };
                let condition = condition.clone();
                let r = eval_condition(&self.doc, &mut self.cache, &condition);
                let now = r.is_some_and(|x| x.0);
                let prev = self.conns[ci].subs[si].last.replace(now);
                if prev == Some(false) && now {
                    if let Some((_, bar, t)) = r {
                        conds.push(out(
                            &self.conns[ci].id,
                            ServerMsg::Notify {
                                sub: self.conns[ci].subs[si].id.clone(),
                                event: Event::ConditionMet { bar, t, seq },
                            },
                        ));
                    }
                }
            }
        }
        let fired = self.eval_intents(seq);
        let mut intents = Vec::new();
        for ev in fired {
            for c in &self.conns {
                for s in &c.subs {
                    if matches!(s.topic, Topic::Intents {}) {
                        intents.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event: ev.clone(),
                            },
                        ));
                    }
                }
            }
        }
        (conds, intents)
    }

    fn eval_intents(&mut self, seq: Seq) -> Vec<Event> {
        let mut fired = Vec::new();
        let mut state = BTreeMap::new();
        let intents: Vec<_> = self
            .doc
            .intents
            .iter()
            .filter(|i| i.active)
            .filter_map(|i| i.trigger.clone().map(|t| (i.id.clone(), t)))
            .collect();
        for (id, trig) in intents {
            if matches!(trig, qp_schema::Condition::Text { .. }) {
                continue;
            }
            let r = eval_condition(&self.doc, &mut self.cache, &trig);
            let now = r.is_some_and(|x| x.0);
            if self.intent_state.get(&id) == Some(&false) && now {
                if let Some((_, bar, t)) = r {
                    fired.push(Event::IntentTriggered {
                        intent: id.clone(),
                        bar,
                        t,
                        seq,
                    });
                }
            }
            state.insert(id, now);
        }
        self.intent_state = state;
        fired
    }

    /// Notifications for one log entry.
    fn entry_events(&self, seq: Seq) -> Vec<OutboundLine> {
        let e = &self.log[usize::try_from(seq - 1).unwrap_or(0)];
        let mut lines = Vec::new();
        let touched = touched_ids(&e.changes);
        for c in &self.conns {
            for s in &c.subs {
                if let Topic::Changes { ids } = &s.topic {
                    if ids.is_empty() || ids.iter().any(|i| touched.contains(i)) {
                        lines.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event: Event::Change {
                                    seq,
                                    at: e.at,
                                    author: e.author.clone(),
                                    approved_by: e.approved_by.clone(),
                                    cause: e.cause.clone(),
                                    changes: e.changes.clone(),
                                },
                            },
                        ));
                    }
                }
            }
        }
        for ch in &e.changes {
            let ChangeOp::SetCapsuleState { capsule, state } = ch else {
                continue;
            };
            let Some(finding) = self
                .doc
                .capsules
                .iter()
                .find(|c| &c.id == capsule)
                .map(|c| c.finding.clone())
            else {
                continue;
            };
            for c in &self.conns {
                for s in &c.subs {
                    if matches!(&s.topic, Topic::Finding { finding: f } if *f == finding) {
                        lines.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event: Event::FindingState {
                                    finding: finding.clone(),
                                    capsule: capsule.clone(),
                                    state: *state,
                                    seq,
                                },
                            },
                        ));
                    }
                }
            }
        }
        for ch in &e.changes {
            let ChangeOp::PutProposal { proposal, .. } = ch else {
                continue;
            };
            let author = self.principal_of(&proposal.provenance.author);
            for c in &self.conns {
                for s in &c.subs {
                    if matches!(s.topic, Topic::Proposals {}) {
                        lines.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event: Event::Proposal {
                                    proposal: proposal.id.clone(),
                                    state: proposal.state,
                                    author: author.clone(),
                                    seq,
                                },
                            },
                        ));
                    }
                }
            }
        }
        lines
    }

    // -----------------------------------------------------------------------
    // Subscriptions, clock
    // -----------------------------------------------------------------------

    fn subscribe(&mut self, ci: usize, id: &str, topic: Topic) -> PResult<()> {
        if !permissions(self.conns[ci].role).read {
            return Err(forbidden_read());
        }
        let c = &self.conns[ci];
        if c.subs.iter().any(|s| s.id == id) {
            return Err(E::new(
                ErrorCode::Duplicate,
                format!("subscription `{id}` already exists on this connection"),
            )
            .at("/id")
            .build());
        }
        if c.subs.len() >= self.limits.max_subscriptions as usize {
            return Err(E::new(
                ErrorCode::Limit,
                format!(
                    "a connection holds at most {} subscriptions",
                    self.limits.max_subscriptions
                ),
            )
            .expected([self.limits.max_subscriptions.to_string()])
            .hint("Unsubscribe from something first.")
            .build());
        }
        let mut last = None;
        match &topic {
            Topic::Finding { finding } => {
                need_finding(&self.doc, finding, "/topic/finding")?;
            }
            Topic::Condition { condition } => {
                if matches!(condition, qp_schema::Condition::Text { .. }) {
                    return Err(invalid(
                        "text conditions are not machine-checkable",
                        "/topic/condition/kind",
                    ));
                }
                check_condition(&self.doc, condition, "/topic/condition")?;
                last = Some(holds(&self.doc, &mut self.cache, condition));
            }
            Topic::Attention { principal: Some(p) } => {
                if self.principal(p).is_none() {
                    return Err(
                        E::new(ErrorCode::NotFound, format!("unknown principal `{p}`"))
                            .at("/topic/principal")
                            .dym(p, self.principals.iter().map(|x| x.id.as_str()))
                            .build(),
                    );
                }
            }
            Topic::Changes { .. }
            | Topic::Proposals {}
            | Topic::Attention { principal: None }
            | Topic::Intents {} => {}
        }
        self.conns[ci].subs.push(Sub {
            id: id.to_owned(),
            topic,
            last,
        });
        Ok(())
    }

    fn tick(&mut self, ci: usize, now: i64) -> PResult<Vec<OutboundLine>> {
        if self.conns[ci].role != Role::Host {
            return Err(E::new(ErrorCode::Forbidden, "only the host sends `clock`")
                .hint("The authority's time comes from its host connection.")
                .build());
        }
        if now < self.clock {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("clock went back from {} to {now}", self.clock),
            )
            .at("/now")
            .expected([format!(">= {}", self.clock)])
            .build());
        }
        self.clock = now;
        let due: Vec<Proposal> = self
            .doc
            .proposals
            .iter()
            .filter(|p| p.state == ProposalState::Pending && p.expires_at.is_some_and(|e| now >= e))
            .cloned()
            .collect();
        let mut lines = Vec::new();
        for mut p in due {
            let seq = self.seq() + 1;
            p.state = ProposalState::Expired;
            p.decided_seq = Some(seq);
            let pid = p.id.clone();
            if let Some(s) = self.engine_entry(
                vec![ChangeOp::PutProposal {
                    proposal: p,
                    index: None,
                }],
                format!("expire {pid}"),
            ) {
                lines.extend(self.settle(s));
            }
        }
        Ok(lines)
    }

    /// Notifications of a viewport change of `principal` to its attention watchers.
    pub(crate) fn viewport_events(&self, principal: &str, vp: &Viewport) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        for c in &self.conns {
            for s in &c.subs {
                if let Topic::Attention { principal: p } = &s.topic {
                    if p.as_deref().unwrap_or(&c.principal) == principal {
                        lines.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event: Event::Viewport {
                                    principal: principal.to_owned(),
                                    viewport: vp.clone(),
                                },
                            },
                        ));
                    }
                }
            }
        }
        lines
    }

    /// Attention directive delivery (§4.8).
    fn attend(
        &mut self,
        ci: usize,
        mut d: AttentionDirective,
    ) -> PResult<(String, Vec<OutboundLine>)> {
        if !permissions(self.conns[ci].role).attend {
            return Err(E::new(
                ErrorCode::Forbidden,
                format!(
                    "role `{}` may not send attention directives",
                    role_str(self.conns[ci].role)
                ),
            )
            .build());
        }
        if self.principal(&d.to).is_none() {
            return Err(
                E::new(ErrorCode::NotFound, format!("unknown principal `{}`", d.to))
                    .at("/directive/to")
                    .dym(&d.to, self.principals.iter().map(|x| x.id.as_str()))
                    .build(),
            );
        }
        if d.steps.is_empty() {
            return Err(invalid(
                "a directive needs at least one step",
                "/directive/steps",
            ));
        }
        for (j, s) in d.steps.iter().enumerate() {
            crate::query::check_target_pub(
                &self.doc,
                &s.target,
                &format!("/directive/steps/{j}/target"),
            )?;
        }
        if let Some(i) = &d.id {
            if !crate::expand::valid_id(i) {
                return Err(invalid(format!("invalid id `{i}`"), "/directive/id"));
            }
        }
        self.att_counter += 1;
        let id =
            d.id.clone()
                .unwrap_or_else(|| format!("att-{}", self.att_counter));
        d.id = Some(id.clone());
        d.from = Some(self.conns[ci].principal.clone());
        d.at = Some(self.clock);
        let list = self.directives.entry(d.to.clone()).or_default();
        list.push(d.clone());
        if list.len() > MAX_DIRECTIVES {
            let extra = list.len() - MAX_DIRECTIVES;
            list.drain(..extra);
        }
        let mut lines = Vec::new();
        for c in &self.conns {
            for s in &c.subs {
                if let Topic::Attention { principal: p } = &s.topic {
                    if p.as_deref().unwrap_or(&c.principal) == d.to {
                        lines.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event: Event::Attention {
                                    directive: d.clone(),
                                },
                            },
                        ));
                    }
                }
            }
        }
        Ok((id, lines))
    }
}

fn ack(id: String, status: AckStatus) -> ServerMsg {
    ServerMsg::Ack {
        resolved: std::collections::BTreeMap::new(),
        id,
        status,
        seq: None,
        created: Vec::new(),
        proposal: None,
        undoes: None,
    }
}

fn forbidden_read() -> ProtocolError {
    E::new(ErrorCode::Forbidden, "this principal may not read")
        .hint("Reading needs a role with the read permission.")
        .build()
}

pub(crate) fn role_str(r: Role) -> &'static str {
    match r {
        Role::Owner => "owner",
        Role::Editor => "editor",
        Role::Proposer => "proposer",
        Role::Viewer => "viewer",
        Role::Feed => "feed",
        Role::Host => "host",
    }
}

fn msg_id(m: &ClientMsg) -> Option<String> {
    match m {
        ClientMsg::Command { id, .. }
        | ClientMsg::Query { id, .. }
        | ClientMsg::Subscribe { id, .. }
        | ClientMsg::Unsubscribe { id, .. }
        | ClientMsg::Input { id, .. }
        | ClientMsg::Attend { id, .. } => Some(id.clone()),
        ClientMsg::Hello { .. } | ClientMsg::Clock { .. } => None,
    }
}
