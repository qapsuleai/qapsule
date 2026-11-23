//! The expectation book: one line per case (`name  digest  | summary`), compared with a
//! committed file under `tests/charac/expected/` (the same format as `qp-grammar`'s
//! `tests/charac`).
//!
//! - A digest is the first 16 hex digits of the SHA-256 of the case's output (the JSON of
//!   every outbound line, log entries, documents).
//! - A summary is readable: every outbound message in order (`conn>type(...)`), and for
//!   an error its whole JSON, so a change says where it is before anyone diffs.
//! - On a mismatch the whole book is written to `$CARGO_TARGET_TMPDIR/charac/<book>.txt`
//!   and the differing lines are printed.
//! - `QP_CHARAC_BLESS=1` writes the expected file instead (S1 only: the expectations are
//!   today's output; a refactor must never bless).

use std::fmt::Write as _;

use serde_json::Value;

/// First 16 hex digits of the SHA-256 of `b`.
pub fn digest(b: &[u8]) -> String {
    qp_capsule::trust::seal::sha256(b)[..8]
        .iter()
        .fold(String::new(), |mut s, x| {
            let _ = write!(s, "{x:02x}");
            s
        })
}

/// Digest of a JSON-serialisable value.
pub fn jdigest<T: serde::Serialize>(v: &T) -> String {
    digest(serde_json::to_string(v).expect("json").as_bytes())
}

fn s(v: &Value) -> String {
    match v {
        Value::String(x) => x.clone(),
        Value::Null => "-".to_owned(),
        o => o.to_string(),
    }
}

/// One outbound line (`{"conn","msg"}`) as a short readable token.
pub fn desc(line: &Value) -> String {
    let conn = s(&line["conn"]);
    let m = &line["msg"];
    let body = match m["type"].as_str().unwrap_or("?") {
        "welcome" => format!(
            "welcome({},{},seq={})",
            s(&m["principal"]),
            s(&m["role"]),
            s(&m["seq"])
        ),
        "ack" => {
            let mut b = format!("ack({},{}", s(&m["id"]), s(&m["status"]));
            for k in ["seq", "proposal", "undoes"] {
                if !m[k].is_null() {
                    let _ = write!(b, ",{k}={}", s(&m[k]));
                }
            }
            if let Some(c) = m["created"].as_array() {
                let c: Vec<String> = c.iter().map(s).collect();
                let _ = write!(b, ",created=[{}]", c.join(","));
            }
            b.push(')');
            b
        }
        "result" => {
            let d = &m["data"];
            let kind = if d["kind"].is_null() {
                d.as_object()
                    .and_then(|o| o.keys().next().cloned())
                    .unwrap_or_default()
            } else {
                s(&d["kind"])
            };
            format!("result({},seq={},{kind})", s(&m["id"]), s(&m["seq"]))
        }
        "notify" => {
            let e = &m["event"];
            let mut b = format!("notify({},{}", s(&m["sub"]), s(&e["kind"]));
            if let Some(o) = e.as_object() {
                for k in ["seq", "state", "proposal", "intent", "id", "trust"] {
                    if let Some(v) = o.get(k) {
                        if !v.is_object() && !v.is_array() {
                            let _ = write!(b, ",{k}={}", s(v));
                        }
                    }
                }
            }
            b.push(')');
            b
        }
        "error" => format!(
            "error({},{}@{}: {})",
            s(&m["id"]),
            s(&m["error"]["code"]),
            s(&m["error"]["path"]),
            s(&m["error"]["message"])
        ),
        t => t.to_owned(),
    };
    format!("{conn}>{body}")
}

/// Every outbound line, in order.
pub fn order(lines: &[Value]) -> String {
    if lines.is_empty() {
        return "(none)".to_owned();
    }
    lines.iter().map(desc).collect::<Vec<_>>().join(" ")
}

/// One book of expectations.
pub struct Book {
    name: &'static str,
    lines: Vec<String>,
}

impl Book {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            lines: Vec::new(),
        }
    }

    /// A case: `case  digest  | summary` (a case name must be unique in its book).
    pub fn put(&mut self, case: &str, digest: &str, summary: &str) {
        let key = format!("{case}  ");
        assert!(
            !self.lines.iter().any(|l| l.starts_with(&key)),
            "duplicate case {case}"
        );
        let summary: String = summary.replace('\n', "⏎");
        self.lines.push(format!("{case}  {digest}  | {summary}"));
    }

    /// A case recording outbound lines: digest of all of them, summary their order.
    pub fn lines(&mut self, case: &str, lines: &[Value]) {
        self.put(case, &jdigest(&lines), &order(lines));
    }

    /// A case whose whole output is its summary (no digest).
    pub fn note(&mut self, case: &str, summary: &str) {
        self.put(case, "-", summary);
    }

    /// Compares with `expected/<name>.txt` (or writes it under `QP_CHARAC_BLESS=1`).
    pub fn check(self) {
        let mut text = format!(
            "# charac/{}: today's output (RF1/S1); a refactor keeps every line\n",
            self.name
        );
        for l in &self.lines {
            text.push_str(l);
            text.push('\n');
        }
        let path = format!(
            "{}/tests/charac/expected/{}.txt",
            env!("CARGO_MANIFEST_DIR"),
            self.name
        );
        if std::env::var("QP_CHARAC_BLESS").as_deref() == Ok("1") {
            std::fs::write(&path, &text).expect("write expected");
            return;
        }
        let want = std::fs::read_to_string(&path).unwrap_or_default();
        if want == text {
            return;
        }
        let dir = format!("{}/charac", env!("CARGO_TARGET_TMPDIR"));
        let _ = std::fs::create_dir_all(&dir);
        let got_path = format!("{dir}/{}.txt", self.name);
        let _ = std::fs::write(&got_path, &text);
        let want_lines: Vec<&str> = want.lines().collect();
        let got_lines: Vec<&str> = text.lines().collect();
        let mut diff = String::new();
        for l in &want_lines {
            if !got_lines.contains(l) {
                let _ = writeln!(diff, "- {l}");
            }
        }
        for l in &got_lines {
            if !want_lines.contains(l) {
                let _ = writeln!(diff, "+ {l}");
            }
        }
        panic!(
            "charac/{}: output changed (got: {got_path})\n{diff}",
            self.name
        );
    }
}
