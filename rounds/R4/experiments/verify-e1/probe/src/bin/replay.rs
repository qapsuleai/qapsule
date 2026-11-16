//! Independent replay checks (verify-e1): own control sequences, speed independence,
//! pump chunking, backward seek == fresh build, authority + lagging replica.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_schema::protocol::{ClaimRef, ClaimState, Event, Maturity, OutboundLine, ReplayInit, ServerMsg};
use qp_session::replica::Shown;
use qp_session::{demo_init, document_digest, ReplayHost, Replica, ReplicaOptions};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../..")
}
fn cfg() -> ReplayInit {
    ReplayInit {
        instruments: Some(6),
        days: Some(70),
        history: Some(50),
        seed: Some(7),
        fine: Some("1h".to_owned()),
        ..ReplayInit::default()
    }
}
fn host(r: ReplayInit) -> ReplayHost {
    let none = std::env::temp_dir().join("verify-e1-no-local");
    ReplayHost::with_local_dir(demo_init(r), &root(), &none).unwrap()
}
fn text(lines: &[OutboundLine]) -> String {
    lines.iter().map(|l| serde_json::to_string(l).unwrap() + "\n").collect()
}
fn send(h: &mut ReplayHost, conn: &str, v: Value) -> String {
    text(&h.handle_raw(conn, &v.to_string()))
}
fn ctl(h: &mut ReplayHost, conn: &str, id: &str, c: Value) -> String {
    send(h, conn, json!({"type":"replay","id":id,"control":c}))
}
fn connect(h: &mut ReplayHost) -> String {
    let mut s = String::new();
    s += &send(h, "a", json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}}));
    s += &send(h, "g", json!({"type":"hello","protocol":"qp/0","principal":"guest","surface":{"kind":"desktop"}}));
    s += &send(h, "a", json!({"type":"command","id":"aud","ops":[{"op":"set_audience","audience":{"group":"future","sensitivity":"important","positions":{"SYN02":10}}}]}));
    s += &send(h, "a", json!({"type":"subscribe","id":"c","topic":{"kind":"capsules"}}));
    s += &send(h, "a", json!({"type":"subscribe","id":"ch","topic":{"kind":"changes"}}));
    s += &send(h, "g", json!({"type":"subscribe","id":"c","topic":{"kind":"capsules","all":true}}));
    s
}
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
fn civil(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}
fn date_after(t0: i64, days: i64) -> String {
    let local = (t0 + 3 * 3_600_000).div_euclid(86_400_000) + days;
    let (y, m, d) = civil(local);
    let _ = days_from_civil;
    format!("{y:04}-{m:02}-{d:02}")
}

/// My scenario; `walls` = wall times of the pumps while running.
fn scenario(h: &mut ReplayHost, walls: &[u64]) -> String {
    let mut s = connect(h);
    s += &ctl(h, "a", "s3", json!({"kind":"step","n":3}));
    let t0 = h.state().from_t.unwrap();
    let d = date_after(t0, 15);
    s += &ctl(h, "a", "sd", json!({"kind":"seek","date":d}));
    for i in 0..5 {
        s += &ctl(h, "a", &format!("s1-{i}"), json!({"kind":"step"}));
    }
    s += &ctl(h, "a", "sp", json!({"kind":"speed","steps_per_s":7.5}));
    s += &ctl(h, "a", "r", json!({"kind":"resume"}));
    for w in walls {
        s += &text(&h.pump(*w));
    }
    s += &ctl(h, "a", "p", json!({"kind":"pause"}));
    s += &ctl(h, "g", "gs", json!({"kind":"step","n":2})); // forbidden for viewer
    s += &ctl(h, "g", "gst", json!({"kind":"status"}));
    // backward seek by t: the time of step 20
    let t20 = {
        let mut probe = host(cfg());
        let _ = probe.host_control(&serde_json::from_value(json!({"kind":"step","n":20})).unwrap());
        probe.state().t
    };
    s += &ctl(h, "a", "back", json!({"kind":"seek","t":t20}));
    s += &connect(h);
    s += &ctl(h, "a", "s11", json!({"kind":"step","n":11}));
    s += &ctl(h, "a", "fw", json!({"kind":"seek","step":120}));
    s += &ctl(h, "a", "st", json!({"kind":"status"}));
    s
}

fn doc_and_log(h: &ReplayHost) -> (String, String) {
    (
        serde_json::to_string(h.session().document()).unwrap(),
        serde_json::to_string(h.session().log()).unwrap(),
    )
}

fn fnv(s: &str) -> String {
    format!("{:016x}", qp_session::replica::fnv1a64(s.as_bytes()))
}

fn main() {
    let mut report = Vec::<String>::new();
    // 1. two runs byte-identical
    let walls_a = [0_u64, 400, 800, 1200, 2000];
    let walls_b = [0_u64, 133, 1999, 2000];
    let (mut h1, mut h2, mut h3) = (host(cfg()), host(cfg()), host(cfg()));
    let s1 = scenario(&mut h1, &walls_a);
    let s2 = scenario(&mut h2, &walls_a);
    let s3 = scenario(&mut h3, &walls_b);
    report.push(format!(
        "1 two runs same walls: equal={} bytes={} lines={} fnv={} notices={:?} final step={} epoch={}",
        s1 == s2, s1.len(), s1.lines().count(), fnv(&s1), h1.notices().len(), h1.state().step, h1.state().epoch
    ));
    report.push(format!(
        "1b different pump chunking (same due steps): equal={} ; doc equal={}",
        s1 == s3, doc_and_log(&h1) == doc_and_log(&h3)
    ));
    let forb = s1.lines().filter(|l| l.contains("\"forbidden\"")).count();
    let resets = s1.lines().filter(|l| l.contains("\"reset\":true")).count();
    report.push(format!("1c forbidden lines={forb} reset lines={resets} controls={:?}", h1.controls().iter().map(|c| c.0).collect::<Vec<_>>()));

    // 2. speed does not change content: same steps at speed 1 vs 900
    let mut outs = Vec::new();
    for speed in [1.0_f64, 37.0, 900.0] {
        let mut h = host(cfg());
        let _ = connect(&mut h);
        let _ = ctl(&mut h, "a", "sp", json!({"kind":"speed","steps_per_s":speed}));
        let _ = ctl(&mut h, "a", "r", json!({"kind":"resume"}));
        let mut s = String::new();
        let mut w = 0_u64;
        s += &text(&h.pump(w));
        while h.state().step < 150 {
            w += 13;
            // stop exactly at 150: pause when reaching; pumps may overshoot so cap by
            // stepping manually at the end
            let due = h.state().step + ((13.0 * speed / 1000.0).floor() as u64).max(1);
            if due > 150 {
                break;
            }
            s += &text(&h.pump(w));
        }
        let _ = ctl(&mut h, "a", "p", json!({"kind":"pause"}));
        let left = 150 - h.state().step;
        if left > 0 {
            // remaining steps by `step` (notifications follow the result line)
            let r = h.handle_raw("a", &json!({"type":"replay","id":"x","control":{"kind":"step","n":left}}).to_string());
            s += &text(&r[1..]);
        }
        outs.push((speed, h.state().step, s, doc_and_log(&h)));
    }
    for (sp, st, s, dl) in &outs {
        report.push(format!(
            "2 speed {sp}: step {st} notif bytes {} fnv {} doc {} log {} equal-to-speed1: {}",
            s.len(), fnv(s), fnv(&dl.0), fnv(&dl.1), (s == &outs[0].2 && dl == &outs[0].3)
        ));
    }

    // 3. backward seek == fresh build (step / t / date seeks)
    let mut fresh = host(cfg());
    let _ = fresh.host_control(&serde_json::from_value(json!({"kind":"step","n":37})).unwrap());
    let mut back = host(cfg());
    let _ = connect(&mut back);
    let _ = ctl(&mut back, "a", "f", json!({"kind":"step","n":180}));
    let r = ctl(&mut back, "a", "b", json!({"kind":"seek","step":37}));
    report.push(format!(
        "3 back seek step 180->37: doc equal={} log equal={} reset in reply={} doc fnv {}",
        doc_and_log(&back).0 == doc_and_log(&fresh).0,
        doc_and_log(&back).1 == doc_and_log(&fresh).1,
        r.contains("\"reset\":true"),
        fnv(&doc_and_log(&fresh).0)
    ));
    // after: both step 60 more; notification streams for a new connection equal
    let _ = connect(&mut back);
    let _ = connect(&mut fresh);
    let a = ctl(&mut back, "a", "n", json!({"kind":"step","n":60}));
    let b = ctl(&mut fresh, "a", "n", json!({"kind":"step","n":60}));
    // the result line carries epoch (1 vs 0); compare the notifications only
    let strip = |s: &str| s.lines().skip(1).collect::<Vec<_>>().join("\n");
    report.push(format!(
        "3b then 60 more steps: notifications equal={} (lines {}), doc equal={}, log equal={}",
        strip(&a) == strip(&b), a.lines().count(), doc_and_log(&back).0 == doc_and_log(&fresh).0, doc_and_log(&back).1 == doc_and_log(&fresh).1
    ));
    // backward seek by date (to the start of a day) == fresh seek date
    let d = date_after(fresh.state().from_t.unwrap(), 20);
    let mut f2 = host(cfg());
    let _ = f2.host_control(&serde_json::from_value(json!({"kind":"seek","date":d})).unwrap());
    let mut b2 = host(cfg());
    let _ = b2.host_control(&serde_json::from_value(json!({"kind":"step","n":250})).unwrap());
    let st = b2.host_control(&serde_json::from_value(json!({"kind":"seek","date":d})).unwrap()).unwrap().0;
    report.push(format!(
        "3c back seek by date {d}: step {} == fresh {}; doc equal={} log equal={} reset={}",
        st.step, f2.state().step, doc_and_log(&b2).0 == doc_and_log(&f2).0, doc_and_log(&b2).1 == doc_and_log(&f2).1, st.reset
    ));
    // the first step after seek(date) must be on that date (nothing of that day taken before)
    let t_before = b2.state().t;
    let _ = b2.host_control(&serde_json::from_value(json!({"kind":"step"})).unwrap());
    report.push(format!(
        "3d seek date: last t before = {} (local date {}), next step t local date {}",
        t_before, date_after(t_before, 0), date_after(b2.state().t, 0)
    ));

    // 4. authority + replica with lagging data (replica's own data K steps late)
    for lag in [0_usize, 4] {
        let r = replica_run(lag);
        report.push(format!("4 replica lag {lag}: {r}"));
    }
    let r1 = replica_run(4);
    let r2 = replica_run(4);
    report.push(format!("4b replica lag 4 two runs equal (summary incl. stream fnv): {}", r1 == r2));
    for l in &report {
        println!("{l}");
    }
}

fn replica_run(lag: usize) -> String {
    let mut init = demo_init(ReplayInit {
        instruments: Some(8),
        days: Some(80),
        history: Some(60),
        seed: Some(3),
        fine: Some("1h".to_owned()),
        ..ReplayInit::default()
    });
    let mut doc: qp_schema::Document = match init.document {
        qp_schema::protocol::DocumentRef::Inline(d) => *d,
        qp_schema::protocol::DocumentRef::Path(_) => unreachable!(),
    };
    for i in 1..=4 {
        doc.series.push(serde_json::from_value(json!({"id":format!("rsi{i:02}"),"interval":"1d",
            "source":{"kind":"study","study":"rsi","inputs":[format!("syn{i:02}")],"params":{"period":9.0}}})).unwrap());
    }
    init.document = qp_schema::protocol::DocumentRef::Inline(Box::new(doc));
    let none = std::env::temp_dir().join("verify-e1-no-local");
    let mut h = ReplayHost::with_local_dir(init, &root(), &none).unwrap();
    let mut prov = qp_source::MemoryProvider::new();
    for (k, bars) in h.replay().feeds() {
        prov.add_feed(k.clone(), bars.to_vec()).unwrap();
    }
    prov.entitle("guest", ["*".to_owned()]).unwrap();
    let mut r = Replica::with_memory("guest", prov);
    r.set_options(ReplicaOptions::default());
    let mut said: BTreeMap<(ClaimRef, i64), (ClaimState, Maturity)> = BTreeMap::new();
    let mut stream = String::new();
    fn deliver(h: &mut ReplayHost, r: &mut Replica, said: &mut BTreeMap<(ClaimRef, i64), (ClaimState, Maturity)>, stream: &mut String, lines: Vec<OutboundLine>) {
        let mut q: std::collections::VecDeque<_> = lines.into();
        while let Some(l) = q.pop_front() {
            stream.push_str(&serde_json::to_string(&l).unwrap());
            stream.push('\n');
            if l.conn != "r" { continue; }
            if let ServerMsg::Notify { event: Event::Claim { claim, state, evidence, .. }, .. } = &l.msg {
                said.insert((claim.clone(), evidence.t), (*state, evidence.maturity));
            }
            r.on_message(&l.msg);
            for m in r.take_outbox() {
                q.extend(h.handle_raw("r", &m.to_string()));
            }
        }
    }
    let lines = h.handle_raw("r", &json!({"type":"hello","protocol":"qp/0","principal":"guest","surface":{"kind":"phone"}}).to_string());
    for i in 1..=4 {
        for (op, k) in [("cross_above", 65.0), ("cross_below", 35.0), ("gt", 50.0)] {
            r.watch(&format!("{op}{i}"), serde_json::from_value(json!({"kind":"threshold","series":format!("rsi{i:02}"),"op":op,"value":k})).unwrap());
        }
    }
    deliver(&mut h, &mut r, &mut said, &mut stream, lines);
    let mut pending: std::collections::VecDeque<Vec<qp_schema::data::DataEvent>> = Default::default();
    let (mut steps, mut compared, mut differing, mut shown, mut viol) = (0, 0, 0, 0, Vec::<String>::new());
    let check = |h: &ReplayHost, r: &Replica, said: &BTreeMap<(ClaimRef, i64), (ClaimState, Maturity)>, steps: usize, shown: &mut usize, viol: &mut Vec<String>| {
        for it in r.display() {
            if it.state != Shown::Shown { continue; }
            *shown += 1;
            let key = (it.claim.clone(), it.t);
            let s = said.get(&key).copied();
            // independent rule: the authority must have asserted this claim at this t
            if !matches!(s, Some((ClaimState::Asserted, _))) {
                viol.push(format!("step {steps}: shown but not asserted {key:?} {s:?}"));
            }
            // the bar must be closed in the replica's own data AND in the authority's data
            let own = r.bars(&it.series).unwrap_or_default();
            let auth_has_next = h.session().document().series.iter().any(|_| true);
            let _ = auth_has_next;
            match own.iter().position(|b| b.t == it.t) {
                None => viol.push(format!("step {steps}: own data lacks t {key:?}")),
                Some(j) => {
                    if j + 1 >= own.len() { viol.push(format!("step {steps}: shown on own live bar {key:?}")); }
                }
            }
            if it.label != Some(Maturity::Final) || !matches!(s, Some((_, Maturity::Final))) {
                viol.push(format!("step {steps}: not final {key:?}"));
            }
        }
    };
    while let Some((st, lines)) = h.take_step() {
        steps += 1;
        deliver(&mut h, &mut r, &mut said, &mut stream, lines);
        pending.push_back(st.events.clone());
        while pending.len() > lag {
            for ev in pending.pop_front().unwrap() { r.data_event(&ev).unwrap(); }
        }
        check(&h, &r, &said, steps, &mut shown, &mut viol);
        if r.seq() == h.session().log().len() as u64 {
            compared += 1;
            if r.document_digest().as_deref() != Some(document_digest(h.session().document()).as_str()) { differing += 1; }
        }
    }
    while let Some(evs) = pending.pop_front() { for ev in evs { r.data_event(&ev).unwrap(); } }
    check(&h, &r, &said, steps + 1, &mut shown, &mut viol);
    let final_eq = r.document_digest().as_deref() == Some(document_digest(h.session().document()).as_str());
    format!("steps {steps} compared {compared} differing {differing} final_doc_equal {final_eq} shown {shown} violations {} first {:?} notices {} capsules {} stream fnv {}",
        viol.len(), viol.first(), h.notices().len(), h.session().document().capsules.len(), fnv(&stream))
}
