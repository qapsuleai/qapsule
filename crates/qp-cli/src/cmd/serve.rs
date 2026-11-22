//! `qp serve`, `qp mcp` and `qp conform` (RG1: split out of `main.rs`).

use crate::*;

pub(crate) fn serve(
    init: qp_schema::protocol::SessionInit,
    root: &Path,
    batch_end: bool,
    host_frames: bool,
) -> Result<Outcome, String> {
    use std::io::{BufRead, Write};
    let mut session = qp_session::Session::new(init, root)?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("stdin: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let is_host = host_frames
            && serde_json::from_str::<serde_json::Value>(&line)
                .ok()
                .is_some_and(|v| v.get("host").is_some());
        let lines = if is_host {
            match session.handle_host_frame(&line) {
                Ok(o) => {
                    // Notices for the host (§18.4: a gap, a binding that became stale),
                    // before the connections' notifications.
                    for n in &o.notices {
                        writeln!(out, "{}", n.frame()).map_err(|e| format!("stdout: {e}"))?;
                    }
                    o.lines
                }
                Err(e) => {
                    let text = serde_json::json!({"host": {"error": e}}).to_string();
                    writeln!(out, "{text}").map_err(|e| format!("stdout: {e}"))?;
                    Vec::new()
                }
            }
        } else {
            session.handle_frame(&line)
        };
        for o in lines {
            let text = serde_json::to_string(&o).map_err(|e| e.to_string())?;
            writeln!(out, "{text}").map_err(|e| format!("stdout: {e}"))?;
        }
        if batch_end {
            writeln!(out, "{{\"batch_end\":true}}").map_err(|e| format!("stdout: {e}"))?;
        }
        out.flush().map_err(|e| format!("stdout: {e}"))?;
    }
    Ok(Outcome::Ok)
}

/// `qp serve` over a replay host (§21): `replay` messages reach the host; while the
/// replay runs, steps are taken as the wall clock makes them due (their notifications
/// are written as they happen, outside any input line's batch).
pub(crate) fn serve_replay(
    mut host: qp_session::ReplayHost,
    batch_end: bool,
    host_frames: bool,
) -> Result<Outcome, String> {
    use std::io::{BufRead, Write};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            if tx.send(line.map_err(|e| format!("stdin: {e}"))).is_err() {
                break;
            }
        }
    });
    let start = Instant::now();
    let wall = || u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let write = |out: &mut std::io::StdoutLock<'_>,
                 lines: Vec<qp_schema::protocol::OutboundLine>|
     -> Result<(), String> {
        for o in lines {
            let text = serde_json::to_string(&o).map_err(|e| e.to_string())?;
            writeln!(out, "{text}").map_err(|e| format!("stdout: {e}"))?;
        }
        out.flush().map_err(|e| format!("stdout: {e}"))
    };
    loop {
        // Running: wake when the next step is due (an anchor is set by the first pump).
        let wait = match host.next_due_in(wall()) {
            Some(ms) => Some(ms),
            None if !host.state().paused && !host.state().done => Some(0),
            None => None,
        };
        let msg = match wait {
            Some(ms) => match rx.recv_timeout(Duration::from_millis(ms)) {
                Ok(m) => Some(m),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            },
            None => match rx.recv() {
                Ok(m) => Some(m),
                Err(_) => break,
            },
        };
        let Some(line) = msg else {
            let lines = host.pump(wall());
            write(&mut out, lines)?;
            continue;
        };
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let is_host = host_frames
            && serde_json::from_str::<serde_json::Value>(&line)
                .ok()
                .is_some_and(|v| v.get("host").is_some());
        let lines = if is_host {
            match host.session_mut().handle_host_frame(&line) {
                Ok(o) => {
                    for n in &o.notices {
                        writeln!(out, "{}", n.frame()).map_err(|e| format!("stdout: {e}"))?;
                    }
                    o.lines
                }
                Err(e) => {
                    let text = serde_json::json!({"host": {"error": e}}).to_string();
                    writeln!(out, "{text}").map_err(|e| format!("stdout: {e}"))?;
                    Vec::new()
                }
            }
        } else {
            host.handle_frame(&line)
        };
        write(&mut out, lines)?;
        if batch_end {
            writeln!(out, "{{\"batch_end\":true}}").map_err(|e| format!("stdout: {e}"))?;
            out.flush().map_err(|e| format!("stdout: {e}"))?;
        }
    }
    Ok(Outcome::Ok)
}

pub(crate) fn mcp(
    init: &Path,
    principal: &str,
    surface: &str,
    root: &Path,
) -> Result<Outcome, String> {
    use std::io::{BufRead, Write};
    let init: qp_schema::protocol::SessionInit = qp_session::read_json(init)?;
    // (R4, §21) With `init.replay` the hub wraps a replay host (`replay_*` tools). Only
    // explicit steps advance it here: this transport has no wall-clock loop.
    let hub = if init.replay.is_some() {
        qp_mcp::Hub::shared_replay(qp_session::ReplayHost::new(init, root)?)
    } else {
        qp_mcp::Hub::shared(qp_session::Session::new(init, root)?)
    };
    let surface: qp_schema::protocol::Surface =
        serde_json::from_value(serde_json::json!({ "kind": surface }))
            .map_err(|e| format!("--surface `{surface}`: {e}"))?;
    let config = qp_mcp::Config::new(principal, surface);
    let mut server = qp_mcp::McpServer::connect(hub, "mcp", config)
        .map_err(|e| format!("hello refused: {e}"))?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("stdin: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        for o in server.handle_line(&line) {
            writeln!(out, "{o}").map_err(|e| format!("stdout: {e}"))?;
        }
        out.flush().map_err(|e| format!("stdout: {e}"))?;
    }
    Ok(Outcome::Ok)
}

pub(crate) fn conform(inputs: &[PathBuf], transport: &str, root: &Path) -> Result<Outcome, String> {
    use qp_session::conform::{collect_files, run_file, TransportKind};
    let kind = match transport {
        "inproc" => Some(TransportKind::InProc),
        "serve" => Some(TransportKind::Serve(
            std::env::current_exe().map_err(|e| format!("cannot locate qp: {e}"))?,
        )),
        "mcp" => None,
        other => {
            return Err(format!(
                "unknown transport `{other}`; expected inproc, serve or mcp"
            ))
        }
    };
    let files = collect_files(inputs)?;
    if files.is_empty() {
        return Err("no transcripts found".to_owned());
    }
    let mut failed = 0;
    for f in &files {
        let (o, carried) = match &kind {
            Some(k) => (run_file(f, root, k), String::new()),
            None => {
                let (o, s) = qp_mcp::conform::run_file(f, root);
                let note = format!(
                    " (mcp: {} hello, {} tool call(s), {} direct)",
                    s.hellos, s.tool_calls, s.direct
                );
                (o, note)
            }
        };
        match &o.failure {
            None => println!("PASS {}{carried}", o.name),
            Some(why) => {
                failed += 1;
                println!("FAIL {}: {why}", o.name);
            }
        }
    }
    println!(
        "{} transcript(s), {} passed, {failed} failed ({transport})",
        files.len(),
        files.len() - failed
    );
    Ok(if failed == 0 {
        Outcome::Ok
    } else {
        Outcome::Invalid
    })
}
