//! (R5B/T2) The measurement log (`qp mcp serve --log <file>`): one JSON line per tool
//! call, `{run, principal, tool, args_bytes, result_bytes, ok, ms}`. For the M28/M29
//! measurements only: it never changes what the server answers, and it holds no
//! argument or result content (sizes only).

use std::cell::RefCell;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;

use serde::Serialize;

/// One logged tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Line {
    /// Measurement run label (HTTP `X-Qp-Run`, stdio `--run`); `null` if none.
    pub run: Option<String>,
    /// Principal of the endpoint.
    pub principal: String,
    /// Tool name.
    pub tool: String,
    /// Bytes of the `arguments` JSON.
    pub args_bytes: usize,
    /// Bytes of the result (or JSON-RPC error) JSON.
    pub result_bytes: usize,
    /// No JSON-RPC error and no `isError`.
    pub ok: bool,
    /// Wall milliseconds the call took in the server.
    pub ms: u64,
}

/// The log sink.
pub struct CallLog {
    out: Box<dyn Write>,
}

/// Shared handle (single-threaded, like the hub).
pub type SharedLog = Rc<RefCell<CallLog>>;

impl CallLog {
    /// Appends to `path` (created if missing).
    ///
    /// # Errors
    /// The file cannot be opened.
    pub fn append(path: &Path) -> Result<Self, String> {
        let f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("--log {}: {e}", path.display()))?;
        Ok(Self { out: Box::new(f) })
    }

    /// Any writer (tests).
    #[must_use]
    pub fn to_writer(out: Box<dyn Write>) -> Self {
        Self { out }
    }

    /// A shared handle.
    #[must_use]
    pub fn shared(self) -> SharedLog {
        Rc::new(RefCell::new(self))
    }

    /// Writes one line and flushes (a failed write is ignored: measurement only).
    pub fn write(&mut self, line: &Line) {
        if let Ok(text) = serde_json::to_string(line) {
            let _ = writeln!(self.out, "{text}");
            let _ = self.out.flush();
        }
    }
}
