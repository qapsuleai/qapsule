//! Halts that only a person may lift (K64, A52), and the injectable clock of the
//! unexplained-server-close rule.
//!
//! Two halts keep an endpoint closed until a person resets it:
//! - **multi-connect notice** (`HaltReason::MultiConnect`): the server said another session
//!   with the same identity connected;
//! - **unexplained server closes** (`HaltReason::UnexplainedServerClose`, A52): on the
//!   market endpoint the notice topic is refused (R6, `mx/messenger` SUBACK 0x80), so a
//!   second session with our identity can only show up as the server closing our
//!   connection. [`DEFAULT_SERVER_CLOSE_LIMIT`] such closes within a sliding
//!   [`DEFAULT_SERVER_CLOSE_WINDOW`] stop the connector.
//!
//! **What counts as an unexplained server close** ([`is_unexplained_server_close`]): the
//! connection was open (the WebSocket handshake had succeeded) and the *server* ended it —
//! a WebSocket close frame, an MQTT DISCONNECT from the server, end of stream, or a TCP
//! reset / abort / broken pipe ([`DisconnectReason::ServerClosed`]) — with no
//! multi-connect notice before it. **Not counted:** our own closes (stop, `/live/drop`,
//! token renewal), local network and path failures (`DisconnectReason::Io`, e.g. R6's
//! `os error 49` "can't assign requested address"; TCP / TLS / WebSocket handshake failures
//! `ConnectFailed`; silence `KeepaliveTimeout` and `ConnackTimeout`), protocol errors and
//! CONNACK refusals (they have their own rules). A reset sent by a middlebox cannot be told
//! from one sent by the server: it is counted (a wrong halt costs a person's reset; a missed
//! second session costs the other session's data).
//!
//! Each halt writes `<dir>/live-<key>.halted` (reason, time, pid) and a line in
//! `<dir>/live-<key>.halted.log`; [`reset_halt`] removes the marker only with a non-empty
//! note and appends the note and time to the log. The CLI is `qp live reset` (a person
//! runs it, never an agent on its own: `docs/matriks/LIVE.md`).

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::conn::DisconnectReason;
use crate::error::HaltReason;
use crate::lock::EndpointKey;

/// A52 (K64): this many unexplained server closes …
pub const DEFAULT_SERVER_CLOSE_LIMIT: u32 = 2;
/// … within this sliding window halt the connector.
pub const DEFAULT_SERVER_CLOSE_WINDOW: Duration = Duration::from_secs(600);

/// The unexplained-server-close rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerCloseRule {
    /// Closes within `window` that halt (0 = never halt on closes).
    pub limit: u32,
    pub window: Duration,
}

impl Default for ServerCloseRule {
    fn default() -> Self {
        Self {
            limit: DEFAULT_SERVER_CLOSE_LIMIT,
            window: DEFAULT_SERVER_CLOSE_WINDOW,
        }
    }
}

/// Monotonic time since an arbitrary origin. Injected so tests need not wait 10 minutes.
pub type Clock = Arc<dyn Fn() -> Duration + Send + Sync>;

/// The process's monotonic clock.
pub fn system_clock() -> Clock {
    let origin = Instant::now();
    Arc::new(move || origin.elapsed())
}

/// A clock moved by hand (tests).
#[derive(Clone, Debug, Default)]
pub struct ManualClock(Arc<AtomicU64>);

impl ManualClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance(&self, d: Duration) {
        let ms = u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
        self.0.fetch_add(ms, Ordering::SeqCst);
    }

    pub fn now(&self) -> Duration {
        Duration::from_millis(self.0.load(Ordering::SeqCst))
    }

    pub fn clock(&self) -> Clock {
        let c = self.0.clone();
        Arc::new(move || Duration::from_millis(c.load(Ordering::SeqCst)))
    }
}

/// True for a disconnect that counts toward the rule (see the module docs).
pub fn is_unexplained_server_close(reason: &DisconnectReason) -> bool {
    matches!(reason, DisconnectReason::ServerClosed)
}

/// The sliding window of unexplained server closes.
#[derive(Clone, Debug)]
pub struct ServerCloseWindow {
    rule: ServerCloseRule,
    at: VecDeque<Duration>,
}

impl ServerCloseWindow {
    pub fn new(rule: ServerCloseRule) -> Self {
        Self {
            rule,
            at: VecDeque::new(),
        }
    }

    /// Records one close at `now`; returns the halt when the limit is reached.
    pub fn record(&mut self, now: Duration) -> Option<HaltReason> {
        if self.rule.limit == 0 {
            return None;
        }
        while self
            .at
            .front()
            .is_some_and(|t| now.saturating_sub(*t) >= self.rule.window)
        {
            self.at.pop_front();
        }
        self.at.push_back(now);
        let n = u32::try_from(self.at.len()).unwrap_or(u32::MAX);
        (n >= self.rule.limit).then_some(HaltReason::UnexplainedServerClose {
            closes: n,
            window_secs: self.rule.window.as_secs(),
        })
    }

    /// Closes in the window as of the last record.
    pub fn count(&self) -> usize {
        self.at.len()
    }
}

// ---------------------------------------------------------------------------------------
// Person-reset halts: process registry + marker file + log.

static HALTED: Mutex<BTreeMap<String, HaltReason>> = Mutex::new(BTreeMap::new());

/// `<dir>/live-<key>.halted.log`.
pub fn halt_log(dir: &Path, key: &EndpointKey) -> PathBuf {
    dir.join(format!("live-{}.halted.log", key.as_str()))
}

fn marker_name(reason: &HaltReason) -> &'static str {
    match reason {
        HaltReason::UnexplainedServerClose { .. } => "unexplained-server-close",
        _ => "multiconnect",
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// `YYYY-MM-DDTHH:MM:SSZ` (UTC).
fn utc(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let s = secs.rem_euclid(86_400);
    // Civil from days (H. Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        s / 3600,
        s / 60 % 60,
        s % 60
    )
}

fn append_log(dir: &Path, key: &EndpointKey, line: &str) -> std::io::Result<()> {
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(halt_log(dir, key))?;
    writeln!(f, "{line}")
}

/// Halts `key` until a person resets it: process registry, marker file, log line.
pub(crate) fn mark_halted(dir: &Path, key: &EndpointKey, reason: &HaltReason) {
    HALTED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key.as_str().to_string(), reason.clone());
    let at = unix_now();
    let extra = match reason {
        HaltReason::UnexplainedServerClose {
            closes,
            window_secs,
        } => format!("closes={closes}\nwindow_secs={window_secs}\n"),
        _ => String::new(),
    };
    let text = format!(
        "halted={}\nendpoint={key}\nat_unix={at}\nat={}\npid={}\n{extra}\
         reason={reason}\n\
         Do not remove by hand. A person checks why (another session with this identity?), \
         then runs: qp live reset --endpoint {key} --note \"<who, what was checked>\"\n",
        marker_name(reason),
        utc(at),
        std::process::id()
    );
    let _ = std::fs::write(key.halt_marker(dir), text);
    let _ = append_log(
        dir,
        key,
        &format!(
            "{} HALT {} pid={} {reason}",
            utc(at),
            marker_name(reason),
            std::process::id()
        ),
    );
}

/// The halt reason read back from a marker file's text (`None`: not a marker).
fn parse_marker(text: &str) -> Option<HaltReason> {
    let get = |k: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix('=')))
    };
    match get("halted")? {
        "unexplained-server-close" => Some(HaltReason::UnexplainedServerClose {
            closes: get("closes").and_then(|v| v.parse().ok()).unwrap_or(0),
            window_secs: get("window_secs").and_then(|v| v.parse().ok()).unwrap_or(0),
        }),
        _ => Some(HaltReason::MultiConnect),
    }
}

/// Why `key` is halted until a person resets it (this process's registry, else the marker
/// file any process wrote); `None` if it is not. An unreadable marker counts as halted
/// (multi-connect).
pub fn halt_status(dir: &Path, key: &EndpointKey) -> Option<HaltReason> {
    if let Some(r) = HALTED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(key.as_str())
    {
        return Some(r.clone());
    }
    let path = key.halt_marker(dir);
    if !path.exists() {
        return None;
    }
    Some(
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| parse_marker(&t))
            .unwrap_or(HaltReason::MultiConnect),
    )
}

/// True if `key` is halted until a person resets it (multi-connect notice or unexplained
/// server closes; see [`halt_status`]).
pub fn is_multiconnect_halted(dir: &Path, key: &EndpointKey) -> bool {
    halt_status(dir, key).is_some()
}

/// What [`reset_halt`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResetOutcome {
    /// The halt was lifted; its reason.
    Reset { previous: HaltReason },
    /// Nothing was halted (the note is still logged).
    NotHalted,
}

/// Why [`reset_halt`] refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResetError {
    /// The note is empty (or only blanks / control characters).
    EmptyNote,
    /// The marker could not be removed or the log not written.
    Io(String),
}

impl fmt::Display for ResetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResetError::EmptyNote => f.write_str(
                "a note is required: who resets and what was checked (e.g. no other session \
                 with this identity)",
            ),
            ResetError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ResetError {}

/// One line, no control characters, at most 500 characters.
fn clean_note(note: &str) -> String {
    let one: String = note
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let one = one.split_whitespace().collect::<Vec<_>>().join(" ");
    one.chars().take(500).collect()
}

/// A person's reset of `key`'s halt. Requires a non-empty note; appends the time, the
/// previous reason and the note to `live-<key>.halted.log`, then removes the marker and
/// this process's registry entry. Changes nothing without a note.
///
/// # Errors
/// [`ResetError::EmptyNote`]; [`ResetError::Io`] when the log cannot be written or the
/// marker not removed (the halt then stays).
pub fn reset_halt(dir: &Path, key: &EndpointKey, note: &str) -> Result<ResetOutcome, ResetError> {
    let note = clean_note(note);
    if note.is_empty() {
        return Err(ResetError::EmptyNote);
    }
    let previous = halt_status(dir, key);
    let what = previous.as_ref().map_or("none", marker_name).to_string();
    append_log(
        dir,
        key,
        &format!(
            "{} RESET previous={what} pid={} note={note:?}",
            utc(unix_now()),
            std::process::id()
        ),
    )
    .map_err(|e| ResetError::Io(format!("{}: {e}", halt_log(dir, key).display())))?;
    let marker = key.halt_marker(dir);
    match std::fs::remove_file(&marker) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(ResetError::Io(format!("{}: {e}", marker.display()))),
    }
    HALTED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(key.as_str());
    Ok(match previous {
        Some(previous) => ResetOutcome::Reset { previous },
        None => ResetOutcome::NotHalted,
    })
}

/// Explicit, person-confirmed reset of an endpoint halt (multi-connect or unexplained
/// server closes). Requires a non-empty note (who / why); returns false and changes
/// nothing without one. See [`reset_halt`].
pub fn reset_multiconnect_halt(dir: &Path, key: &EndpointKey, operator_note: &str) -> bool {
    reset_halt(dir, key, operator_note).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Masked;
    use crate::redact::Redactor;

    #[test]
    fn window_counts_only_closes_within_ten_minutes() {
        let mut w = ServerCloseWindow::new(ServerCloseRule::default());
        let min = |m: u64| Duration::from_secs(60 * m);
        assert_eq!(w.record(min(0)), None);
        // 10 minutes or more later: the first one left the window.
        assert_eq!(w.record(min(10)), None);
        assert_eq!(w.record(min(21)), None);
        assert_eq!(
            w.record(min(30)),
            Some(HaltReason::UnexplainedServerClose {
                closes: 2,
                window_secs: 600
            })
        );
        let mut off = ServerCloseWindow::new(ServerCloseRule {
            limit: 0,
            window: min(10),
        });
        assert_eq!(off.record(min(0)), None);
        assert_eq!(off.record(min(0)), None);
    }

    #[test]
    fn only_server_closes_count() {
        let r = Redactor::default();
        let m = |s: &str| Masked::new(&r, s);
        assert!(is_unexplained_server_close(&DisconnectReason::ServerClosed));
        for not in [
            DisconnectReason::Io(m("Can't assign requested address (os error 49)")),
            DisconnectReason::Io(m("Network is unreachable (os error 51)")),
            DisconnectReason::ConnectFailed(m("connection refused")),
            DisconnectReason::KeepaliveTimeout,
            DisconnectReason::ConnackTimeout,
            DisconnectReason::Protocol(m("bad frame")),
            DisconnectReason::ConnackRefused(3),
            DisconnectReason::TokenRenewal,
        ] {
            assert!(!is_unexplained_server_close(&not), "{not:?}");
        }
    }

    #[test]
    fn marker_round_trips_and_reset_needs_a_note() {
        let dir = tempfile::tempdir().unwrap();
        let key = EndpointKey::new("unit-a52");
        let reason = HaltReason::UnexplainedServerClose {
            closes: 2,
            window_secs: 600,
        };
        mark_halted(dir.path(), &key, &reason);
        let text = std::fs::read_to_string(key.halt_marker(dir.path())).unwrap();
        assert_eq!(parse_marker(&text), Some(reason.clone()));
        assert!(text.contains("halted=unexplained-server-close"), "{text}");
        assert_eq!(halt_status(dir.path(), &key), Some(reason.clone()));
        assert_eq!(
            reset_halt(dir.path(), &key, " \n\t "),
            Err(ResetError::EmptyNote)
        );
        assert!(key.halt_marker(dir.path()).exists());
        assert_eq!(
            reset_halt(dir.path(), &key, "Arkin: checked,\nno other session"),
            Ok(ResetOutcome::Reset { previous: reason })
        );
        assert!(!key.halt_marker(dir.path()).exists());
        assert_eq!(halt_status(dir.path(), &key), None);
        let log = std::fs::read_to_string(halt_log(dir.path(), &key)).unwrap();
        let lines: Vec<&str> = log.lines().collect();
        assert_eq!(lines.len(), 2, "{log}");
        assert!(
            lines[0].contains(" HALT unexplained-server-close "),
            "{log}"
        );
        assert!(
            lines[1].contains("RESET previous=unexplained-server-close")
                && lines[1].contains("note=\"Arkin: checked, no other session\""),
            "{log}"
        );
        assert_eq!(
            reset_halt(dir.path(), &key, "again"),
            Ok(ResetOutcome::NotHalted)
        );
    }

    #[test]
    fn utc_format() {
        assert_eq!(utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(utc(1_795_132_800), "2026-11-20T00:00:00Z");
    }
}
