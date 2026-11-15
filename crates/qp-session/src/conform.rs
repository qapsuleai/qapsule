//! Conformance transcript runner (`docs/PROTOCOL.md` §15).
//!
//! A transcript is run against a [`Transport`]: in-process ([`InProc`], a [`Session`])
//! or line-based ([`Serve`], a `qp serve --batch-end` child process). Every step's
//! outputs must match `expect` exactly in number and order, each message as a subset
//! pattern (`"$any"` matches any value; `"$absent"` as an object value requires the key to be absent; arrays have the same length; numbers are equal
//! as numbers). After the last step the `final` checks run and the log is replayed from
//! the initial document; the replayed document's bytes must equal the authority's.
//!
//! `send.msg` is forwarded as the raw JSON the transcript holds (R2 audit K2): it is not
//! parsed as a typed `ClientMsg` first, so a message may use selectors (§17) and reaches
//! the authority exactly as written on every transport.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use qp_schema::data::DataEvent;
use qp_schema::protocol::{Expect, FinalCheck, InitRef, LogEntry, RawLine, Role, SessionInit};
use qp_schema::Document;
use serde_json::{json, Value};

use crate::apply::replay;
use crate::session::{confine, permissions, read_json, Session};

/// A message to send, kept as raw JSON (`send` of a transcript step).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawSend {
    /// Logical connection id.
    pub conn: String,
    /// The message, forwarded as written (it may hold selectors, §17).
    pub msg: Value,
}

/// One step of a transcript as the runner reads it: like
/// [`qp_schema::protocol::Step`], but `send.msg` is raw JSON.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunStep {
    /// A message to send, as raw JSON.
    #[serde(default)]
    pub send: Option<RawSend>,
    /// A raw line to send instead (for parse and schema errors).
    #[serde(default)]
    pub send_raw: Option<RawLine>,
    /// (RP2) A data event the host delivers (data plane, not a protocol message; §18.4).
    #[serde(default)]
    pub data: Option<DataEvent>,
    /// Expected outputs, complete and in order.
    pub expect: Vec<Expect>,
    /// Optional comment.
    #[serde(default)]
    pub note: Option<String>,
}

/// Field names of [`RunStep`]; a test keeps them equal to those of
/// [`qp_schema::protocol::Step`].
pub const RUN_STEP_FIELDS: [&str; 5] = ["send", "send_raw", "data", "expect", "note"];

/// A transcript as the runner reads it: the same file format as
/// [`qp_schema::protocol::Transcript`] (which the schema step validates), with `send.msg`
/// kept as raw JSON instead of a typed `ClientMsg`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunTranscript {
    /// Short name (file stem).
    pub name: String,
    /// What it shows.
    pub description: String,
    /// Message kinds and features it covers.
    #[serde(default)]
    pub covers: Vec<String>,
    /// How the authority starts.
    pub init: InitRef,
    /// Steps, in order.
    pub steps: Vec<RunStep>,
    /// Final checks. Replay byte-equality is always checked.
    #[serde(default, rename = "final")]
    pub final_check: FinalCheck,
}

/// Reads a transcript file for running (`send.msg` raw).
///
/// # Errors
/// Unreadable file or not a transcript.
pub fn read_transcript(path: &Path) -> Result<RunTranscript, String> {
    read_json(path)
}

/// The authority's final state as seen through a transport.
pub struct FinalState {
    /// The document.
    pub document: Document,
    /// The log.
    pub log: Vec<LogEntry>,
}

/// A way to talk to an authority.
pub trait Transport {
    /// Sends one message on `conn` and returns every output line (`{"conn","msg"}`).
    /// `raw` marks text that may not be valid JSON (sent as is).
    ///
    /// # Errors
    /// Transport failure.
    fn send(&mut self, conn: &str, text: &str, raw: bool) -> Result<Vec<Value>, String>;
    /// Delivers a data event on the host's path (§18.4; not over a connection) and
    /// returns every output line it causes.
    ///
    /// # Errors
    /// Transport failure, or the authority rejected the event.
    fn data(&mut self, event: &DataEvent) -> Result<Vec<Value>, String>;
    /// Reads the final document and log.
    ///
    /// # Errors
    /// Transport failure or unreadable answers.
    fn finish(&mut self) -> Result<FinalState, String>;
}

/// In-process transport: calls [`Session::handle_raw`].
pub struct InProc {
    /// The session.
    pub session: Session,
}

impl Transport for InProc {
    fn send(&mut self, conn: &str, text: &str, _raw: bool) -> Result<Vec<Value>, String> {
        self.session
            .handle_raw(conn, text)
            .iter()
            .map(|l| serde_json::to_value(l).map_err(|e| e.to_string()))
            .collect()
    }

    fn data(&mut self, event: &DataEvent) -> Result<Vec<Value>, String> {
        self.session
            .data_event(event)
            .map_err(|e| format!("data event rejected: {e}"))?
            .iter()
            .map(|l| serde_json::to_value(l).map_err(|e| e.to_string()))
            .collect()
    }

    fn finish(&mut self) -> Result<FinalState, String> {
        Ok(FinalState {
            document: self.session.document().clone(),
            log: self.session.log().to_vec(),
        })
    }
}

/// Line transport: a `qp serve --batch-end` child process.
pub struct Serve {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    reader: String,
}

impl Serve {
    /// Spawns `qp serve --init <init> --root <root> --batch-end --host-frames` (host
    /// frames carry the transcript's `data` steps, §3). `reader` is a
    /// principal with read permission, used to fetch the final document and log.
    ///
    /// # Errors
    /// The process cannot be started.
    pub fn spawn(qp: &Path, init: &Path, root: &Path, reader: &str) -> Result<Self, String> {
        let mut child = Command::new(qp)
            .arg("serve")
            .arg("--init")
            .arg(init)
            .arg("--root")
            .arg(root)
            .arg("--batch-end")
            .arg("--host-frames")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", qp.display()))?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = BufReader::new(child.stdout.take().ok_or("no stdout")?);
        Ok(Self {
            child,
            stdin: Some(stdin),
            stdout,
            reader: reader.to_owned(),
        })
    }

    fn exchange(&mut self, frame: &Value) -> Result<Vec<Value>, String> {
        let stdin = self.stdin.as_mut().ok_or("stdin closed")?;
        writeln!(stdin, "{frame}").map_err(|e| format!("write: {e}"))?;
        stdin.flush().map_err(|e| format!("flush: {e}"))?;
        let mut outs = Vec::new();
        loop {
            let mut line = String::new();
            let n = self
                .stdout
                .read_line(&mut line)
                .map_err(|e| format!("read: {e}"))?;
            if n == 0 {
                return Err("qp serve closed its output before batch_end".to_owned());
            }
            let v: Value = serde_json::from_str(line.trim_end())
                .map_err(|e| format!("qp serve wrote a non-JSON line ({e}): {line}"))?;
            if v == json!({"batch_end": true}) {
                return Ok(outs);
            }
            outs.push(v);
        }
    }

    fn query(&mut self, conn: &str, query: Value) -> Result<Value, String> {
        let outs = self.exchange(
            &json!({"conn": conn, "msg": {"type":"query","id":"final","query": query}}),
        )?;
        let first = outs.into_iter().next().ok_or("no answer")?;
        first
            .pointer("/msg/data")
            .cloned()
            .ok_or_else(|| format!("unexpected answer: {first}"))
    }
}

impl Transport for Serve {
    fn send(&mut self, conn: &str, text: &str, raw: bool) -> Result<Vec<Value>, String> {
        let msg = if raw {
            Value::String(text.to_owned())
        } else {
            serde_json::from_str(text).map_err(|e| e.to_string())?
        };
        self.exchange(&json!({"conn": conn, "msg": msg}))
    }

    fn data(&mut self, event: &DataEvent) -> Result<Vec<Value>, String> {
        let outs = self.exchange(&json!({"host": {"data": event}}))?;
        if let Some(e) = outs.iter().find_map(|o| o.pointer("/host/error")) {
            return Err(format!("data event rejected: {e}"));
        }
        // Host notices (`{"host":{"gap":…}}`, §18.4) are for the host, not for a
        // connection: the in-process transports do not see them either.
        Ok(outs
            .into_iter()
            .filter(|o| o.get("host").is_none())
            .collect())
    }

    fn finish(&mut self) -> Result<FinalState, String> {
        let conn = "__conform__";
        let r = self.reader.clone();
        let hello = json!({"conn": conn, "msg": {"type":"hello","protocol":"qp/0","principal": r,
            "surface":{"kind":"headless"}}});
        self.exchange(&hello)?;
        let d = self.query(conn, json!({"kind":"document"}))?;
        let l = self.query(conn, json!({"kind":"log"}))?;
        let document = serde_json::from_value(d.get("document").cloned().unwrap_or_default())
            .map_err(|e| format!("document answer: {e}"))?;
        let log = serde_json::from_value(l.get("entries").cloned().unwrap_or_default())
            .map_err(|e| format!("log answer: {e}"))?;
        Ok(FinalState { document, log })
    }
}

impl Drop for Serve {
    fn drop(&mut self) {
        drop(self.stdin.take());
        let _ = self.child.wait();
    }
}

fn trunc(v: &Value) -> String {
    let s = v.to_string();
    if s.len() > 400 {
        let mut end = 400;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    } else {
        s
    }
}

fn num_eq(a: &serde_json::Number, b: &serde_json::Number) -> bool {
    if let (Some(x), Some(y)) = (a.as_i64(), b.as_i64()) {
        return x == y;
    }
    if let (Some(x), Some(y)) = (a.as_u64(), b.as_u64()) {
        return x == y;
    }
    a.as_f64() == b.as_f64()
}

/// Subset match (§15). On mismatch: `(json pointer, description)`.
///
/// # Errors
/// The first mismatch.
pub fn subset(expected: &Value, actual: &Value, path: &str) -> Result<(), (String, String)> {
    if expected.as_str() == Some("$any") {
        return Ok(());
    }
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            for (k, ev) in e {
                let p = crate::error::ptr(path, k);
                if ev.as_str() == Some("$absent") {
                    if let Some(av) = a.get(k) {
                        return Err((p, format!("key must be absent, got {}", trunc(av))));
                    }
                    continue;
                }
                match a.get(k) {
                    None => {
                        return Err((p, format!("missing key (expected {})", trunc(ev))));
                    }
                    Some(av) => subset(ev, av, &p)?,
                }
            }
            Ok(())
        }
        (Value::Array(e), Value::Array(a)) => {
            if e.len() != a.len() {
                return Err((
                    path.to_owned(),
                    format!(
                        "array length {} != expected {} (actual {})",
                        a.len(),
                        e.len(),
                        trunc(actual)
                    ),
                ));
            }
            for (i, (ev, av)) in e.iter().zip(a).enumerate() {
                subset(ev, av, &crate::error::ptr(path, i))?;
            }
            Ok(())
        }
        (Value::Number(e), Value::Number(a)) if num_eq(e, a) => Ok(()),
        _ if expected == actual => Ok(()),
        _ => Err((
            path.to_owned(),
            format!("expected {}, got {}", trunc(expected), trunc(actual)),
        )),
    }
}

/// A transcript's init, and a file holding it for `qp serve`.
pub struct LoadedInit {
    /// The init.
    pub init: SessionInit,
    /// Path of a file with the init.
    pub path: PathBuf,
    /// For inline inits: the securely created temporary file (removed on drop).
    _temp: Option<tempfile::NamedTempFile>,
}

/// Loads a transcript's init. Inline inits are written to a temporary file created
/// with an unpredictable name and owner-only permissions (`tempfile`).
///
/// # Errors
/// Unreadable init.
pub fn load_init(init: &InitRef, root: &Path) -> Result<LoadedInit, String> {
    match init {
        InitRef::Path(p) => {
            let path = confine(root, p)?;
            Ok(LoadedInit {
                init: read_json(&path)?,
                path,
                _temp: None,
            })
        }
        InitRef::Inline(i) => {
            let mut f = tempfile::Builder::new()
                .prefix("qp-conform-")
                .suffix(".json")
                .tempfile()
                .map_err(|e| format!("temporary init file: {e}"))?;
            serde_json::to_writer(f.as_file_mut(), i).map_err(|e| e.to_string())?;
            f.as_file_mut().flush().map_err(|e| e.to_string())?;
            Ok(LoadedInit {
                init: (**i).clone(),
                path: f.path().to_owned(),
                _temp: Some(f),
            })
        }
    }
}

/// Runs a transcript's steps and final checks on `transport`; `initial` is the initial
/// document (for the replay check).
///
/// # Errors
/// A precise description of the first mismatch.
pub fn run_steps(
    t: &RunTranscript,
    transport: &mut dyn Transport,
    initial: &Document,
) -> Result<(), String> {
    for (si, step) in t.steps.iter().enumerate() {
        let outs = match (&step.send, &step.send_raw, &step.data) {
            (Some(s), None, None) => {
                let text = serde_json::to_string(&s.msg).map_err(|e| e.to_string())?;
                transport.send(&s.conn, &text, false)
            }
            (None, Some(r), None) => transport.send(&r.conn, &r.line, true),
            (None, None, Some(ev)) => transport.data(ev),
            _ => {
                return Err(format!(
                    "step {si}: give exactly one of `send`, `send_raw` and `data`"
                ))
            }
        }
        .map_err(|e| format!("step {si}: {e}"))?;
        if outs.len() != step.expect.len() {
            let got: Vec<String> = outs.iter().map(trunc).collect();
            return Err(format!(
                "step {si}: expected {} message(s), got {}: [{}]",
                step.expect.len(),
                outs.len(),
                got.join(", ")
            ));
        }
        for (mi, (exp, act)) in step.expect.iter().zip(&outs).enumerate() {
            let aconn = act.get("conn").and_then(Value::as_str).unwrap_or_default();
            if aconn != exp.conn {
                return Err(format!(
                    "step {si} message {mi}: sent to conn `{aconn}`, expected `{}`: {}",
                    exp.conn,
                    trunc(act)
                ));
            }
            let amsg = act.get("msg").cloned().unwrap_or_default();
            subset(&exp.msg, &amsg, "").map_err(|(p, d)| {
                format!(
                    "step {si} message {mi} at `{p}`: {d}; message: {}",
                    trunc(&amsg)
                )
            })?;
        }
    }
    let fin = transport.finish()?;
    if let Some(n) = t.final_check.log_len {
        if fin.log.len() as u64 != n {
            return Err(format!(
                "final: log has {} entries, expected {n}",
                fin.log.len()
            ));
        }
    }
    if let Some(pat) = &t.final_check.document {
        let doc = serde_json::to_value(&fin.document).map_err(|e| e.to_string())?;
        subset(pat, &doc, "").map_err(|(p, d)| format!("final document at `{p}`: {d}"))?;
    }
    let replayed = replay(initial, &fin.log).map_err(|e| format!("replay: {e}"))?;
    let a = serde_json::to_vec(&replayed).map_err(|e| e.to_string())?;
    let b = serde_json::to_vec(&fin.document).map_err(|e| e.to_string())?;
    if a != b {
        let at = a
            .iter()
            .zip(&b)
            .position(|(x, y)| x != y)
            .unwrap_or(a.len().min(b.len()));
        return Err(format!(
            "replay: document bytes differ at byte {at} (replayed {} bytes, authority {} bytes)",
            a.len(),
            b.len()
        ));
    }
    Ok(())
}

/// Which transport to use.
#[derive(Debug, Clone)]
pub enum TransportKind {
    /// In-process session.
    InProc,
    /// `qp serve` child process (path of the `qp` binary).
    Serve(PathBuf),
}

/// Result of one transcript.
#[derive(Debug, Clone)]
pub struct TranscriptOutcome {
    /// File the transcript came from.
    pub file: PathBuf,
    /// Transcript name.
    pub name: String,
    /// `None` if it passed, else why it failed.
    pub failure: Option<String>,
}

/// Loads and runs one transcript file.
#[must_use]
pub fn run_file(file: &Path, root: &Path, kind: &TransportKind) -> TranscriptOutcome {
    let mut outcome = TranscriptOutcome {
        file: file.to_owned(),
        name: file
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        failure: None,
    };
    let r = (|| -> Result<(), String> {
        let t = read_transcript(file)?;
        outcome.name.clone_from(&t.name);
        let loaded = load_init(&t.init, root)?;
        let init = &loaded.init;
        let session = Session::new(init.clone(), root)?;
        let initial = session.initial_document().clone();
        match kind {
            TransportKind::InProc => {
                let mut tr = InProc { session };
                run_steps(&t, &mut tr, &initial)
            }
            TransportKind::Serve(qp) => {
                let reader = init
                    .principals
                    .iter()
                    .find(|p| permissions(p.role).read && p.role != Role::Feed)
                    .map(|p| p.id.clone())
                    .ok_or("the init has no principal that may read")?;
                let mut tr = Serve::spawn(qp, &loaded.path, root, &reader)?;
                run_steps(&t, &mut tr, &initial)
            }
        }
    })();
    outcome.failure = r.err();
    outcome
}

/// Expands directories into their `*.json` files (sorted); files are kept as given.
///
/// # Errors
/// Unreadable directory.
pub fn collect_files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for p in inputs {
        if p.is_dir() {
            let mut v: Vec<PathBuf> = std::fs::read_dir(p)
                .map_err(|e| format!("{}: {e}", p.display()))?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|x| x.extension().is_some_and(|e| e == "json"))
                .collect();
            v.sort();
            out.extend(v);
        } else {
            out.push(p.clone());
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::RUN_STEP_FIELDS;

    /// The runner's step reader and the schema's `Step` have the same fields (RP2
    /// audit O1): a field added to one must be added to the other.
    #[test]
    fn run_step_fields_match_the_schema_step() {
        let schema = schemars::schema_for!(qp_schema::protocol::Step).to_value();
        let mut want: Vec<String> = schema["properties"]
            .as_object()
            .expect("Step has properties")
            .keys()
            .cloned()
            .collect();
        want.sort();
        let mut got: Vec<String> = RUN_STEP_FIELDS.iter().map(|s| (*s).to_owned()).collect();
        got.sort();
        assert_eq!(got, want);
        // And the reader accepts exactly these fields.
        let step: super::RunStep = serde_json::from_value(serde_json::json!({
            "data": {"feed":{"symbol":"X","interval":"1d"},"seq":1,
                     "change":{"kind":"patch","t":1,"fields":{"c":1.0}}},
            "expect": [], "note": "n"
        }))
        .expect("a data step parses");
        assert!(step.data.is_some());
        assert!(serde_json::from_value::<super::RunStep>(
            serde_json::json!({"expect": [], "extra": 1})
        )
        .is_err());
    }
}
