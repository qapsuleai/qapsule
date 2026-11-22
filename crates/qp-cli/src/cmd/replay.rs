//! `qp replay` and its tapes (RG1: split out of `main.rs`).

use crate::*;

/// `qp replay` (§21): see [`ReplayArgs`].
#[allow(clippy::too_many_lines)]
pub(crate) fn replay_cmd(a: &ReplayArgs) -> Result<Outcome, String> {
    if let Some(raw) = &a.raw {
        return live::replay_raw(raw, &a.feed, a.out.as_deref(), a.stats).map(|()| Outcome::Ok);
    }
    use qp_schema::protocol::{ReplayControl, ReplayInit, ReplaySourceKind};
    use std::io::Write;
    let init = match &a.init {
        Some(p) => qp_session::read_json(p)?,
        None => {
            let source = match a.source.as_str() {
                "synthetic" => ReplaySourceKind::Synthetic,
                "local" => ReplaySourceKind::Local,
                s => return Err(format!("--source `{s}`: expected synthetic or local")),
            };
            let day = |d: &str| qp_source::date_start(d).map_err(|e| format!("date `{d}`: {e}"));
            qp_session::demo_init(ReplayInit {
                source,
                instruments: Some(a.instruments),
                days: Some(a.days),
                seed: Some(a.seed),
                interval: Some(a.interval.clone()),
                fine: (a.fine != "none").then(|| a.fine.clone()),
                history: Some(a.history),
                from_t: a.from.as_deref().map(day).transpose()?,
                to_t: a
                    .to
                    .as_deref()
                    .map(day)
                    .transpose()?
                    .map(|t| t + 86_400_000 - 1),
                ..ReplayInit::default()
            })
        }
    };
    let dir = a
        .local_bars
        .clone()
        .unwrap_or_else(qp_source::local_bars_dir);
    let t0 = std::time::Instant::now();
    let mut host = qp_session::ReplayHost::with_local_dir(init, &a.root, &dir)?;
    let built = t0.elapsed();
    struct At {
        at: u64,
        control: ReplayControl,
    }
    let mut controls: Vec<At> = match &a.controls {
        Some(p) => {
            let raw: Vec<serde_json::Value> = qp_session::read_json(p)?;
            raw.into_iter()
                .enumerate()
                .map(|(i, v)| {
                    let at = v.get("at").and_then(serde_json::Value::as_u64);
                    let control = v
                        .get("control")
                        .cloned()
                        .map(serde_json::from_value::<ReplayControl>);
                    match (at, control) {
                        (Some(at), Some(Ok(control))) => Ok(At { at, control }),
                        _ => Err(format!(
                            "{}: entry {i} is not {{\"at\": <step>, \"control\": <ReplayControl>}}",
                            p.display()
                        )),
                    }
                })
                .collect::<Result<_, _>>()?
        }
        None => Vec::new(),
    };
    controls.sort_by_key(|c| c.at);
    let (emit_msgs, emit_events) = match a.emit.as_str() {
        "messages" => (true, false),
        "events" => (false, true),
        "none" => (false, false),
        e => return Err(format!("--emit `{e}`: expected messages, events or none")),
    };
    let mut out: Box<dyn Write> = match &a.out {
        Some(p) => Box::new(std::io::BufWriter::new(
            std::fs::File::create(p).map_err(|e| format!("{}: {e}", p.display()))?,
        )),
        None => Box::new(std::io::BufWriter::new(std::io::stdout().lock())),
    };
    let put = |out: &mut Box<dyn Write>, lines: &[qp_schema::protocol::OutboundLine]| {
        if !emit_msgs {
            return Ok(());
        }
        for l in lines {
            let text = serde_json::to_string(l).map_err(|e| e.to_string())?;
            writeln!(out, "{text}").map_err(|e| format!("write: {e}"))?;
        }
        Ok::<(), String>(())
    };
    // The watching connection: `hello` and the subscriptions. A backward seek is a new
    // authority (§21.3, `reset: true`, epoch + 1) that closes every connection; the tool
    // then connects again exactly like this, as the demo shell does, and the stream goes on.
    let connect = |host: &mut qp_session::ReplayHost| {
        let hello = serde_json::json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}});
        let mut ls = host.handle_raw("c", &hello.to_string());
        for (id, topic) in [
            ("caps", serde_json::json!({"kind":"capsules","all":true})),
            ("ch", serde_json::json!({"kind":"changes"})),
        ] {
            let m = serde_json::json!({"type":"subscribe","id":id,"topic":topic});
            ls.extend(host.handle_raw("c", &m.to_string()));
        }
        ls
    };
    let is_reset = |lines: &[qp_schema::protocol::OutboundLine]| {
        lines.iter().any(|l| {
            matches!(
                &l.msg,
                qp_schema::protocol::ServerMsg::Result {
                    data: qp_schema::protocol::QueryResult::Replay { replay },
                    ..
                } if replay.reset
            )
        })
    };
    let ls = connect(&mut host);
    put(&mut out, &ls)?;
    let ls = host.handle_raw(
        "c",
        &serde_json::json!({"type":"replay","id":"status","control":{"kind":"status"}}).to_string(),
    );
    put(&mut out, &ls)?;
    let limit = a.steps.unwrap_or(u64::MAX);
    let mut next = 0;
    let (mut steps, mut events, mut lines_n) = (0_u64, 0_u64, 0_u64);
    let t1 = std::time::Instant::now();
    loop {
        while next < controls.len() && controls[next].at <= host.replay().cursor().step {
            let id = format!("ctl{next}");
            let m = serde_json::json!({"type":"replay","id":id,"control":controls[next].control});
            let mut ls = host.handle_raw("c", &m.to_string());
            if is_reset(&ls) {
                ls.extend(connect(&mut host));
            }
            lines_n += ls.len() as u64;
            put(&mut out, &ls)?;
            next += 1;
        }
        if steps >= limit {
            break;
        }
        let Some((st, ls)) = host.take_step() else {
            break;
        };
        steps += 1;
        events += st.events.len() as u64;
        lines_n += ls.len() as u64;
        if emit_events {
            for ev in &st.events {
                let text = serde_json::to_string(&serde_json::json!({"t": st.t, "event": ev}))
                    .map_err(|e| e.to_string())?;
                writeln!(out, "{text}").map_err(|e| format!("write: {e}"))?;
            }
        }
        put(&mut out, &ls)?;
    }
    out.flush().map_err(|e| format!("write: {e}"))?;
    let run = t1.elapsed();
    let st = host.state();
    if a.stats {
        let days = st.days.fine + st.days.ohlc + st.days.bar;
        let per_day_steps = if days == 0 {
            0.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            {
                st.steps as f64 / days as f64 * f64::from(st.instruments)
            }
        };
        #[allow(clippy::cast_precision_loss)]
        let ms_step = run.as_secs_f64() * 1000.0 / steps.max(1) as f64;
        eprintln!(
            "source {:?} (synthetic: {}){} | {} instruments, interval {}, fine {:?}, days fine/ohlc/bar {}/{}/{} | build {:.1} ms | {steps} steps, {events} events, {lines_n} lines in {:.1} ms: {ms_step:.3} ms/step, {:.1} ms per virtual day ({:.1} steps/day) | log {} entries, {} capsules, {} host notices",
            st.source,
            st.synthetic,
            st.note.as_deref().map(|n| format!(" [{n}]")).unwrap_or_default(),
            st.instruments,
            st.interval,
            st.fine,
            st.days.fine,
            st.days.ohlc,
            st.days.bar,
            built.as_secs_f64() * 1000.0,
            run.as_secs_f64() * 1000.0,
            ms_step * per_day_steps,
            per_day_steps,
            host.session().log().len(),
            host.session().document().capsules.len(),
            host.notices().len(),
        );
    }
    Ok(if host.notices().is_empty() {
        Outcome::Ok
    } else {
        Outcome::Invalid
    })
}

/// `qp replica-tape`: runs the check, writes the tape, prints a summary; invalid if a
/// display-rule violation was found or (without lag) a step's digests differ.
pub(crate) fn replica_tape(lag: usize, out: &std::path::Path) -> Result<Outcome, String> {
    let o = qp_session::tape::run(lag)?;
    let text = serde_json::to_string(&o.tape).map_err(|e| e.to_string())?;
    std::fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    let last = o.tape.steps.last().map(|s| &s.expect);
    println!(
        "lag {lag}: {} steps, {} with replica digests != authority, {} violations, final claims {} (shown final at end {}), display counts final/provisional/pending/disputed {:?}; last doc {} view {}",
        o.tape.steps.len(),
        o.differing_steps,
        o.violations.len(),
        o.final_claims,
        o.shown_final,
        o.shown_counts,
        last.map_or("", |e| e.doc.as_str()),
        last.map_or("", |e| e.view.as_str()),
    );
    for v in o.violations.iter().take(10) {
        eprintln!("violation at step {}: {}", v.step, v.what);
    }
    let caught_up = last.is_some_and(|e| e.doc == e.authority_doc && e.view == e.authority_view);
    let ok = o.violations.is_empty()
        && caught_up
        && o.shown_final == o.final_claims
        && (lag > 0 || o.differing_steps == 0);
    Ok(if ok { Outcome::Ok } else { Outcome::Invalid })
}

/// (RP3/M15) `qp replica-tape --viewer`: the viewer variant of the consistency check;
/// invalid on a wrong or early claim, on a data subscription to the other instrument, on a
/// final claim not shown at the end, or (without lag) on a step whose slice or view digest
/// differs.
pub(crate) fn viewer_tape(lag: usize, out: &std::path::Path) -> Result<Outcome, String> {
    let o = qp_session::tape::run_viewer(lag)?;
    let text = serde_json::to_string_pretty(&o).map_err(|e| e.to_string())?;
    std::fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    println!(
        "viewer lag {lag}: {} steps, {} with open-replica digests != authority, {} violations, final claims {} (shown final at end {}), steps subscribed to the other instrument {}, display counts {:?}",
        o.steps, o.differing_steps, o.violations.len(), o.final_claims, o.shown_final,
        o.foreign_steps, o.shown_counts,
    );
    for v in o.violations.iter().take(10) {
        eprintln!("violation at step {}: {}", v.step, v.what);
    }
    let ok = o.violations.is_empty()
        && o.foreign_steps == 0
        && o.shown_final == o.final_claims
        && (lag > 0 || o.differing_steps == 0);
    Ok(if ok { Outcome::Ok } else { Outcome::Invalid })
}
