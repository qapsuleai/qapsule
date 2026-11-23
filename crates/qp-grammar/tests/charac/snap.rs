//! The expectation book: one line per case (`name  digest  | summary`), compared with a
//! committed file under `tests/charac/expected/`.
//!
//! - A digest is the first 16 hex digits of the SHA-256 of the case's output (a scene's
//!   JSON, or a `Debug` dump for types without serde).
//! - A summary is a short readable line (texts, counts, an error's kind and message), so
//!   a change says where it is before anyone diffs a scene.
//! - On a mismatch the whole book is written to `$CARGO_TARGET_TMPDIR/charac/<book>.txt`
//!   and the differing lines are printed.
//! - `QP_CHARAC_BLESS=1` writes the expected file instead (G1 only: the expectations are
//!   today's output; a refactor must never bless).

use std::fmt::Write as _;

use qp_grammar::GrammarError;
use qp_schema::scene::SceneDesc;

/// SHA-256 (the only external helper; one place to change if it moves).
fn sha256(b: &[u8]) -> [u8; 32] {
    qp_capsule::trust::seal::sha256(b)
}

/// First 16 hex digits of the SHA-256 of `b`.
pub fn digest(b: &[u8]) -> String {
    sha256(b)[..8].iter().fold(String::new(), |mut s, x| {
        let _ = write!(s, "{x:02x}");
        s
    })
}

/// Digest of a scene's JSON.
pub fn scene_digest(s: &SceneDesc) -> String {
    digest(serde_json::to_string(s).expect("scene json").as_bytes())
}

/// Digest of a `Debug` dump.
pub fn debug_digest<T: std::fmt::Debug>(v: &T) -> String {
    digest(format!("{v:?}").as_bytes())
}

/// An error, fully: `ERR kind layer path: reason`.
pub fn err_line(e: &GrammarError) -> String {
    format!(
        "ERR {:?} layer={} path={} :: {}",
        e.kind,
        e.layer.as_deref().unwrap_or("-"),
        if e.path.is_empty() { "-" } else { &e.path },
        e
    )
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
        assert!(
            !self
                .lines
                .iter()
                .any(|l| l.split("  ").next() == Some(case)),
            "duplicate case {case}"
        );
        let summary: String = summary.replace('\n', "⏎");
        self.lines.push(format!("{case}  {digest}  | {summary}"));
    }

    /// A case whose whole output is its summary (no digest).
    pub fn note(&mut self, case: &str, summary: &str) {
        self.put(case, "-", summary);
    }

    /// Compares with `expected/<name>.txt` (or writes it under `QP_CHARAC_BLESS=1`).
    pub fn check(self) {
        let mut text = format!(
            "# charac/{}: today's output (RF1/G1); a refactor keeps every line\n",
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
