//! The inbound boundary: frames, raw lines, selector resolution, schema errors and limits.

use super::*;

impl Session {
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

    /// A host frame of the line transport (`{"host":{"data":<DataEvent>}}`, §3, §18.4):
    /// transport level and host only — `qp serve` accepts it only with `--host-frames`,
    /// on the host's own stream. Not a protocol message; it never enters the log.
    /// Returns the notifications the event causes and the notices for the host (a gap,
    /// a binding that became stale; `qp serve` writes them as `{"host":{"gap":…}}`).
    ///
    /// # Errors
    /// A malformed frame or event, or a [`DataError`], as text (`qp serve` writes it as
    /// `{"host":{"error":…}}`).
    pub fn handle_host_frame(&mut self, line: &str) -> Result<DataOutcome, String> {
        let v: Value =
            serde_json::from_str(line).map_err(|e| format!("host frame is not JSON: {e}"))?;
        let host = v
            .as_object()
            .filter(|o| o.len() == 1)
            .and_then(|o| o.get("host"))
            .and_then(Value::as_object)
            .ok_or("a host frame is {\"host\":{\"data\":…}}")?;
        if host.len() != 1 {
            return Err("a host frame carries exactly one of: data".to_owned());
        }
        let ev = host
            .get("data")
            .ok_or("unknown host frame (expected `data`)")?;
        let ev = <qp_schema::data::DataEvent as serde::Deserialize>::deserialize(ev)
            .map_err(|e| format!("data event: {e}"))?;
        self.host_data_event(&ev).map_err(|e| e.to_string())
    }

    #[allow(clippy::too_many_lines)]
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
        if let Some(msg) = self.bar_fast_parse(line) {
            let mut lines = match self.check_counts(&msg) {
                Ok(()) => self.handle_resolved(conn, msg, BTreeMap::new()),
                Err(e) => vec![err_line(conn, msg_id(&msg), e)],
            };
            if self.has_pushers() {
                lines.extend(self.viewer_flush());
            }
            return lines;
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
        let reader = self
            .conn_index(conn)
            .map(|ci| (self.conns[ci].principal.clone(), self.lang_of(ci)));
        // (R4/S1) A `capsule` query by selector: kept to say what it may also match that
        // the reader cannot see (`CapsuleDetail.withheld`).
        let capsule_sel = (value.get("type").and_then(Value::as_str) == Some("query"))
            .then(|| value.pointer("/query/capsule").filter(|v| v.is_object()))
            .flatten()
            .and_then(|v| {
                serde_json::from_value::<qp_schema::protocol::CapsuleSel>(v.clone()).ok()
            });
        let resolved = match match reader {
            Some((p, lang)) => self.as_reader(&p, lang, |s| s.resolve_selectors(conn, &mut value)),
            None => self.resolve_selectors(conn, &mut value),
        } {
            Ok(r) => r,
            Err(e) => return vec![err_line(conn, id, e)],
        };
        // Plain typed parse first (the common case); the path-tracking parse runs only to
        // report an error (same error as before).
        let msg: ClientMsg = match <ClientMsg as serde::Deserialize>::deserialize(&value) {
            Ok(m) => m,
            Err(_) => match serde_path_to_error::deserialize(&value) {
                Ok(m) => m,
                Err(e) => return vec![err_line(conn, id, self.schema_error(&value, &e))],
            },
        };
        if let Err(e) = self.check_limits(&value, &msg) {
            return vec![err_line(conn, id, e)];
        }
        let mut lines = self.handle_resolved(conn, msg, resolved);
        // (RP3/M15, §22) What the message changed for viewers (their feed, faces, slice).
        if self.has_pushers() {
            lines.extend(self.viewer_flush());
        }
        let asker = self
            .conn_index(conn)
            .map(|ci| self.conns[ci].principal.clone());
        if let (Some(sel), Some(principal)) = (capsule_sel, asker) {
            for l in &mut lines {
                if let ServerMsg::Result {
                    data: qp_schema::protocol::QueryResult::CapsuleDetail { detail },
                    ..
                } = &mut l.msg
                {
                    detail.withheld = self.withheld_for(&principal, &sel, &detail.item);
                }
            }
        }
        lines
    }

    /// The live tick's fast parse (§18.8): a `command` whose ops are all bar writes
    /// (`append_bars`, `update_last_bar`), parsed straight into the typed message. Such a
    /// command cannot hold a selector (its id field, `series`, is a string: a selector
    /// object fails this parse) and, when the line is not longer than `max_string`, no
    /// string in it can break that limit; so the general path would resolve nothing and
    /// check nothing more. Anything else (or any parse error) takes the general path,
    /// which reports errors exactly as before.
    fn bar_fast_parse(&self, line: &str) -> Option<ClientMsg> {
        if line.len() > self.limits.max_string as usize
            || !(line.contains("\"append_bars\"") || line.contains("\"update_last_bar\""))
        {
            return None;
        }
        let msg: ClientMsg = serde_json::from_str(line).ok()?;
        match &msg {
            ClientMsg::Command { ops, .. }
                if !ops.is_empty()
                    && ops
                        .iter()
                        .all(|o| matches!(o, Op::AppendBars { .. } | Op::UpdateLastBar { .. })) =>
            {
                Some(msg)
            }
            _ => None,
        }
    }

    #[allow(clippy::too_many_lines)]
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
                doc: rd!(self),
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
            data: self.reader(),
            plane: &self.plane,
        };
        // Selectors read no bars (§17): they resolve over the document itself, never the
        // reading document, so a scratch copy never copies bound bars (`add_fact` reads
        // them through `ctx.data`).
        // The live document is used until an op before the last selector changes it;
        // only then is one scratch copy made, and later ops change that copy in place
        // (a failing op is rolled back with the inverses of its applied changes). No
        // copy at all for a message whose selectors are all in its first op that
        // changes nothing before them, e.g. a one-op command.
        let mut scratch: Option<Document> = None;
        let last = groups
            .iter()
            .rposition(|g| g.iter().any(|s| select::is_selector(msg, s)))
            .unwrap_or(0);
        for (i, g) in groups.iter().enumerate().take(last + 1) {
            if g.iter().any(|s| select::is_selector(msg, s)) {
                let r = select::Resolver {
                    doc: scratch.as_ref().unwrap_or(&self.doc),
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
            let cur = scratch.as_ref().unwrap_or(&self.doc);
            if let Ok(cs) = qp_ops::expand::expand_op(cur, &op, &p, &mut ctx) {
                if cs.is_empty() {
                    continue;
                }
                let doc = scratch.get_or_insert_with(|| self.doc.clone());
                apply_or_roll_back(doc, &cs);
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
        /// True if a string longer than `max` is anywhere in `v` (no allocation; the
        /// path is built only when there is one).
        fn any_long(v: &Value, max: usize) -> bool {
            match v {
                Value::String(s) => s.len() > max,
                Value::Array(a) => a.iter().any(|x| any_long(x, max)),
                Value::Object(o) => o.values().any(|x| any_long(x, max)),
                _ => false,
            }
        }
        let max_string = self.limits.max_string as usize;
        if let Some(p) = any_long(value, max_string)
            .then(|| walk(value, "", max_string))
            .flatten()
        {
            return Err(E::new(
                ErrorCode::Limit,
                format!("string longer than {max_string} bytes"),
            )
            .at(p)
            .expected([max_string.to_string()])
            .build());
        }
        self.check_counts(msg)
    }

    /// The op and bar count limits of a command.
    fn check_counts(&self, msg: &ClientMsg) -> PResult<()> {
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
                    Op::PutSeries { series } => match &series.data {
                        Some(qp_schema::SeriesData::Bars { bars }) => (bars.len(), "series"),
                        Some(qp_schema::SeriesData::Points { points }) => (points.len(), "series"),
                        None => continue,
                    },
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

    pub(super) fn conn_index(&self, conn: &str) -> Option<usize> {
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
}
