//! Shell-less driving (R4/T5, GOAL criterion 4): the same session driven only with
//! protocol messages, as an agent would, and the same digests the web shell reports.
//!
//! [`shell_less`] sends hand-written `qp/0` messages to a [`ReplayHost`] (no
//! [`ShellCore`], no recognizer), mirrors the document with a bare [`Replica`] and digests
//! the faces and the open scene with the pure scene functions ([`face_scene`],
//! [`open_scene`]). [`with_core`] drives the same script through [`ShellCore`] with raw
//! pointer events, as the browser does; the web shell's own run (headless Chromium,
//! `shells/web/tests/parity.mjs`) is compared with [`shell_less`] by `scripts/verify.sh`.
//!
//! Script: a JSON array of steps — `{"seek_step":n}`, `{"tap_face":{"index":i,"x":…,
//! "y":…}}` (R4/S1: `index` counts the capsule rows of the feed layout), `{"tap_scene":{"x":…,"y":…}}`, `{"drag":{"dx":…}}`, `{"pinch2x":{}}`, `{"wheel":{"dy":…,"x":…}}`, `{"style":{"face"?,
//! "alphabet"?,"theme"?}}`, `{"back":{}}`, `{"digest":"label"}`. Output: `{label:
//! digests}`.

use std::collections::{BTreeMap, VecDeque};

use qp_schema::protocol::{
    CapsuleItem, OpenDepth, Opening, OutboundLine, QueryResult, ServerMsg, ViewState,
};
use qp_session::{ReplayHost, Replica};
use serde_json::{json, Value};

use crate::shell::{face_scene, open_scene, scene_digest, ShellCore, PHONE_FRAME};

const CONN: &str = "w";

/// The replica's provider at the host's current position (initial bars + recorded
/// events), as `qp demo` gives a new page.
fn provider(host: &ReplayHost, principal: &str) -> qp_source::MemoryProvider {
    let data = host.init().data.clone().expect("replay init has data");
    let mut p =
        qp_source::MemoryProvider::from_init(&data, std::path::Path::new(".")).expect("provider");
    let _ = principal;
    for ev in host.recorded_events() {
        let _ = p.apply(ev);
    }
    p
}

/// A bare protocol client: a replica plus what the answers said.
struct Agent {
    replica: Replica,
    feed: Vec<CapsuleItem>,
    look: ViewState,
    opening: Option<Opening>,
    open_headline: Option<String>,
    max_points: u32,
    n: u64,
    /// The replay clock from the last replay answer (the feed layout may read it).
    now: Option<i64>,
    /// The replay source of the last replay answer (its calendar is set on the replica).
    source: Option<qp_schema::protocol::ReplaySourceKind>,
}

impl Agent {
    fn send(&mut self, host: &mut ReplayHost, m: &Value) {
        let lines = host.handle_raw(CONN, &m.to_string());
        self.deliver(host, lines);
    }

    fn deliver(&mut self, host: &mut ReplayHost, lines: Vec<OutboundLine>) {
        let mut q: VecDeque<OutboundLine> = lines.into();
        while let Some(l) = q.pop_front() {
            if l.conn != CONN {
                continue;
            }
            self.replica.on_message(&l.msg);
            match &l.msg {
                ServerMsg::Welcome { profile, .. } => self.max_points = profile.max_points,
                ServerMsg::Result { data, .. } => match data {
                    QueryResult::Capsules { capsules, .. } => self.feed.clone_from(capsules),
                    QueryResult::Replay { replay } => {
                        // (R4/S2) The headline-piece calendar, as the shell core sets it
                        // (`ShellCore::set_calendar`): faces with a headline template
                        // (`vadrel`, the presentation's face) phrase elapsed time by it.
                        if self.source != Some(replay.source) {
                            self.source = Some(replay.source);
                            self.replica.set_headline_calendar(
                                qp_session::headparts::replay_calendar(replay.source),
                            );
                        }
                        self.now = Some(replay.t);
                    }
                    QueryResult::Interaction { interaction } => {
                        self.look = interaction.state.clone();
                        if let Some(o) = &interaction.opening {
                            if o.from == OpenDepth::Sentence {
                                self.open_headline = self
                                    .feed
                                    .iter()
                                    .find(|i| i.id == o.capsule)
                                    .map(|i| i.sentence.text.clone());
                            }
                            if o.to == OpenDepth::Sentence {
                                self.opening = None;
                                self.open_headline = None;
                            } else {
                                self.opening = Some(o.clone());
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
            for m in self.replica.take_outbox() {
                q.extend(host.handle_raw(CONN, &m.to_string()));
            }
        }
    }

    fn input(&mut self, host: &mut ReplayHost, g: Value) {
        self.n += 1;
        let m = json!({"type":"input","id":format!("i{}", self.n),"gesture":g});
        self.send(host, &m);
    }

    /// (R4/S1) The capsule rows the reader sees, top to bottom, by the connection's feed
    /// layout (the script's `tap_face.index` counts these, as a user would). The script
    /// sets no last look, so nothing is fresh.
    fn face_ids(&self, lang: &str) -> Vec<String> {
        let lang = if lang == "en" {
            qp_schema::protocol::Language::En
        } else {
            qp_schema::protocol::Language::Tr
        };
        let fresh = vec![false; self.feed.len()];
        crate::shell::feed_rows(
            self.look.feed.as_deref(),
            &self.feed,
            lang,
            self.look.seen,
            &fresh,
            self.now,
        )
        .into_iter()
        .filter_map(|r| match r {
            qp_grammar::feeds::FeedRow::Capsule { id, .. } => Some(id),
            qp_grammar::feeds::FeedRow::Header { .. } => None,
        })
        .collect()
    }

    fn refresh_feed(&mut self, host: &mut ReplayHost) {
        self.n += 1;
        let m = json!({"type":"query","id":format!("q{}", self.n),
            "query":{"kind":"capsules","audience":true,"limit":crate::shell::FEED_LIMIT}});
        self.send(host, &m);
    }

    fn digests(&mut self) -> Value {
        let look = self.look.clone();
        let mp = self.max_points;
        let items = self.feed.clone();
        let fh =
            qp_grammar::faces::height_in(qp_session::viewer::look_of(&look), look.face.as_deref());
        let faces: Vec<Value> = items
            .iter()
            .map(|it| {
                let d = self
                    .replica
                    .session_mut()
                    .and_then(|s| face_scene(s, it, &look, mp, PHONE_FRAME.0, fh))
                    .map(|s| scene_digest(&s));
                json!({"id": it.id, "digest": d})
            })
            .collect();
        let open = match (&self.opening, self.replica.session_mut()) {
            (Some(o), Some(s)) => {
                open_scene(s, o, self.open_headline.as_deref(), &look, mp).map(|s| scene_digest(&s))
            }
            _ => None,
        };
        json!({
            "document": self.replica.document_digest(),
            "seq": self.replica.seq(),
            "faces": faces,
            "open": {"depth": self.opening.as_ref().map_or("sentence", |o| match o.to {
                        OpenDepth::Sentence => "sentence", OpenDepth::Capsule => "capsule",
                        OpenDepth::Chart => "chart", OpenDepth::Evidence => "evidence"}),
                     "capsule": self.opening.as_ref().map(|o| o.capsule.clone()),
                     "digest": open},
            "look": self.look,
        })
    }
}

fn f(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(0.0)
}

/// Drives `host` with protocol messages only (see the module docs).
///
/// # Panics
/// A malformed script step.
pub fn shell_less(host: &mut ReplayHost, principal: &str, lang: &str, script: &[Value]) -> Value {
    host.set_record_events(true);
    let mut a = Agent {
        replica: Replica::with_memory(principal, provider(host, principal)),
        feed: Vec::new(),
        look: ViewState::default(),
        opening: None,
        open_headline: None,
        max_points: 60,
        n: 0,
        now: None,
        source: None,
    };
    a.send(
        host,
        &json!({"type":"hello","protocol":"qp/0","principal":principal,
            "surface":{"kind":"phone","screen":[390,844],"input":["touch"],"lang":lang}}),
    );
    // (R4/S1) The presentation's settings, as the shell asks for them after `hello`.
    a.input(host, ShellCore::presented_look());
    a.send(
        host,
        &json!({"type":"subscribe","id":"feed-topic","topic":{"kind":"capsules"}}),
    );
    let mut out = serde_json::Map::new();
    let fw = PHONE_FRAME.0;
    let (sw, sh) = PHONE_FRAME;
    for step in script {
        if let Some(n) = step.get("seek_step").and_then(Value::as_u64) {
            let before = host.recorded_events().len();
            a.n += 1;
            let m = json!({"type":"replay","id":format!("r{}", a.n),"control":{"kind":"seek","step":n}});
            let lines = host.handle_raw(CONN, &m.to_string());
            a.deliver(host, lines);
            let evs: Vec<_> = host.recorded_events()[before..].to_vec();
            for ev in &evs {
                let _ = a.replica.data_event(ev);
            }
        } else if let Some(t) = step.get("tap_face") {
            a.refresh_feed(host);
            let i = usize::try_from(t["index"].as_u64().unwrap_or(0)).unwrap_or(0);
            let cap = a.face_ids(lang)[i].clone();
            let fh = qp_grammar::faces::height_in(
                qp_session::viewer::look_of(&a.look),
                a.look.face.as_deref(),
            );
            a.input(
                host,
                json!({"kind":"tap_at","x":f(t,"x"),"y":f(t,"y"),"width":fw,"height":fh,
                       "target":{"capsule":cap}}),
            );
        } else if let Some(t) = step.get("tap_scene") {
            a.input(
                host,
                json!({"kind":"tap_at","x":f(t,"x"),"y":f(t,"y"),"width":sw,"height":sh}),
            );
        } else if let Some(t) = step.get("drag") {
            a.input(
                host,
                json!({"kind":"drag","dx":f(t,"dx"),"width":sw,"height":sh}),
            );
        } else if let Some(t) = step.get("wheel") {
            let factor = if f(t, "dy") < 0.0 {
                crate::shell::WHEEL_FACTOR
            } else {
                1.0 / crate::shell::WHEEL_FACTOR
            };
            a.input(
                host,
                json!({"kind":"pinch","factor":factor,"x":f(t,"x"),"width":sw,"height":sh}),
            );
        } else if step.get("pinch2x").is_some() {
            // Two fingers at 150 and 250, the second moves to 350: factor 2 at x 250.
            a.input(
                host,
                json!({"kind":"pinch","factor":2.0,"x":250.0,"width":sw,"height":sh}),
            );
        } else if let Some(t) = step.get("style") {
            let mut g = json!({"kind":"style"});
            for k in ["face", "alphabet", "theme", "feed", "transition", "outcome"] {
                if let Some(v) = t.get(k) {
                    g[k] = v.clone();
                }
            }
            a.input(host, g);
        } else if step.get("back").is_some() {
            a.input(host, json!({"kind":"close","width":sw,"height":sh}));
        } else if let Some(label) = step.get("digest").and_then(Value::as_str) {
            a.refresh_feed(host);
            out.insert(label.to_owned(), a.digests());
        } else {
            panic!("unknown script step {step}");
        }
    }
    Value::Object(out)
}

/// Delivers outbound lines of `CONN` to the core as the host's stream would.
fn feed_core(core: &mut ShellCore, host: &mut ReplayHost, lines: Vec<OutboundLine>) {
    for l in lines {
        if l.conn == CONN {
            core.on_message(&l.msg);
        }
    }
    // Pass everything the core queues, until it is idle.
    for _ in 0..1000 {
        let out = core.take_outbox();
        if out.is_empty() {
            break;
        }
        for m in out {
            let lines = host.handle_raw(CONN, &m.to_string());
            for l in lines {
                if l.conn == CONN {
                    core.on_message(&l.msg);
                }
            }
        }
    }
}

/// Drives `host` through [`ShellCore`] with raw pointer events, as the web shell does.
///
/// # Panics
/// A malformed script step.
pub fn with_core(host: &mut ReplayHost, principal: &str, lang: &str, script: &[Value]) -> Value {
    host.set_record_events(true);
    let data = host.init().data.clone().expect("replay init has data");
    let evs = host.recorded_events().to_vec();
    let mut core = ShellCore::new(principal, lang, &data, &evs).expect("core");
    feed_core(&mut core, host, Vec::new());
    let mut out = BTreeMap::new();
    for step in script {
        if let Some(n) = step.get("seek_step").and_then(Value::as_u64) {
            let before = host.recorded_events().len();
            core.seek(&n.to_string());
            feed_core(&mut core, host, Vec::new());
            let evs: Vec<_> = host.recorded_events()[before..].to_vec();
            for ev in &evs {
                core.data_event(ev);
            }
            feed_core(&mut core, host, Vec::new());
        } else if let Some(t) = step.get("tap_face") {
            let i = usize::try_from(t["index"].as_u64().unwrap_or(0)).unwrap_or(0);
            let rows = core.feed_json();
            let ids: Vec<&str> = rows
                .as_array()
                .map(|a| a.iter().filter_map(|r| r["id"].as_str()).collect())
                .unwrap_or_default();
            let target = format!("face:{}", ids[i]);
            core.pointer(&target, "down", 1, f(t, "x"), f(t, "y"));
            core.pointer(&target, "up", 1, f(t, "x"), f(t, "y"));
            feed_core(&mut core, host, Vec::new());
        } else if let Some(t) = step.get("tap_scene") {
            core.pointer("scene", "down", 1, f(t, "x"), f(t, "y"));
            core.pointer("scene", "up", 1, f(t, "x"), f(t, "y"));
            feed_core(&mut core, host, Vec::new());
        } else if let Some(t) = step.get("drag") {
            let (x, dx) = (195.0, f(t, "dx"));
            core.pointer("scene", "down", 1, x, 150.0);
            core.pointer("scene", "move", 1, x + dx, 150.0);
            core.pointer("scene", "up", 1, x + dx, 150.0);
            feed_core(&mut core, host, Vec::new());
        } else if let Some(t) = step.get("wheel") {
            core.wheel(f(t, "dy"), f(t, "x"));
            feed_core(&mut core, host, Vec::new());
        } else if step.get("pinch2x").is_some() {
            core.pointer("scene", "down", 1, 150.0, 150.0);
            core.pointer("scene", "down", 2, 250.0, 150.0);
            core.pointer("scene", "move", 2, 350.0, 150.0);
            core.pointer("scene", "up", 2, 350.0, 150.0);
            core.pointer("scene", "up", 1, 150.0, 150.0);
            feed_core(&mut core, host, Vec::new());
        } else if let Some(t) = step.get("style") {
            core.set_look(t);
            feed_core(&mut core, host, Vec::new());
        } else if step.get("back").is_some() {
            core.back();
            feed_core(&mut core, host, Vec::new());
        } else if let Some(label) = step.get("digest").and_then(Value::as_str) {
            out.insert(label.to_owned(), core.digests());
        } else {
            panic!("unknown script step {step}");
        }
    }
    serde_json::to_value(out).unwrap_or_default()
}

/// The demo host of the parity check: synthetic market, 30 instruments, hourly path,
/// paused (as `qp demo --synthetic --paused`).
///
/// # Panics
/// Never for the built-in init.
#[must_use]
pub fn parity_host() -> ReplayHost {
    let init = crate::shell::demo_session_init(&crate::shell::DemoOptions::default());
    let none = std::env::temp_dir().join("qp-demo-parity-no-local-data");
    ReplayHost::with_local_dir(init, std::path::Path::new("."), &none).expect("demo host")
}

/// (RP3/M15) [`parity_host`] with `n` instruments (the synthetic universes of the viewer
/// measurement: 30, 300, 3,000).
///
/// # Panics
/// Never for the built-in init.
#[must_use]
pub fn parity_host_n(n: u32) -> ReplayHost {
    let init = crate::shell::demo_session_init(&crate::shell::DemoOptions {
        instruments: n,
        ..crate::shell::DemoOptions::default()
    });
    let none = std::env::temp_dir().join("qp-demo-parity-no-local-data");
    ReplayHost::with_local_dir(init, std::path::Path::new("."), &none).expect("demo host")
}

/// (RP3/M15, §22) What a host serves a viewer connection's data plane: the events of the
/// feeds it was already served since `before`, then a `bind` (initial bars and events)
/// for each feed its viewer subscription newly names. Returns the frames in order, as
/// `qp demo` writes them, and updates `served`.
#[must_use]
pub fn viewer_frames(
    host: &ReplayHost,
    conn: &str,
    served: &mut Vec<qp_schema::data::FeedKey>,
    before: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    let all = host.recorded_events();
    let evs = &all[before.min(all.len())..];
    if !evs.is_empty() {
        let mine: Vec<&qp_schema::data::DataEvent> =
            evs.iter().filter(|e| served.contains(&e.feed)).collect();
        out.push(json!({"data": mine, "replay": host.state()}).to_string());
    }
    let want: Vec<qp_schema::data::FeedKey> = host
        .session()
        .viewer_data(conn)
        .map(<[qp_schema::data::FeedKey]>::to_vec)
        .unwrap_or_default();
    if want != *served {
        let mut feeds = Vec::new();
        let mut events = Vec::new();
        for k in want.iter().filter(|k| !served.contains(k)) {
            if let Some((f, e)) = host.feed_data(k) {
                feeds.push(f);
                events.extend(e);
            }
        }
        if !feeds.is_empty() {
            out.push(json!({"bind": {"feeds": feeds, "events": events}}).to_string());
        }
        *served = want;
    }
    out
}

/// A bare viewer client (§22): what the answers said, the device's viewer state.
struct ViewerAgent {
    v: crate::viewer::Viewer,
    look: ViewState,
    opening: Option<Opening>,
    open_headline: Option<String>,
    max_points: u32,
    n: u64,
    now: Option<i64>,
    source: Option<qp_schema::protocol::ReplaySourceKind>,
    served: Vec<qp_schema::data::FeedKey>,
    seq: u64,
}

impl ViewerAgent {
    fn send(&mut self, host: &mut ReplayHost, m: &Value) {
        let before = host.recorded_events().len();
        let lines = host.handle_raw(CONN, &m.to_string());
        self.deliver(host, lines, before);
    }

    /// The data plane first (as `qp demo` orders it), then the messages.
    fn deliver(&mut self, host: &mut ReplayHost, lines: Vec<OutboundLine>, before: usize) {
        for f in viewer_frames(host, CONN, &mut self.served, before) {
            let v: Value = serde_json::from_str(&f).expect("frame");
            if let Some(b) = v.get("bind") {
                let feeds: Vec<qp_schema::protocol::InitFeed> =
                    serde_json::from_value(b["feeds"].clone()).expect("feeds");
                let evs: Vec<qp_schema::data::DataEvent> =
                    serde_json::from_value(b["events"].clone()).unwrap_or_default();
                for fd in feeds {
                    let k = fd.feed.clone();
                    let e: Vec<_> = evs.iter().filter(|x| x.feed == k).cloned().collect();
                    self.v.bind(fd, e);
                }
            } else if let Some(evs) = v.get("data").and_then(Value::as_array) {
                for e in evs {
                    if let Ok(ev) = serde_json::from_value::<qp_schema::data::DataEvent>(e.clone())
                    {
                        self.v.data_event(&ev);
                    }
                }
            }
        }
        for l in lines {
            if l.conn != CONN {
                continue;
            }
            match &l.msg {
                ServerMsg::Welcome { profile, .. } => self.max_points = profile.max_points,
                ServerMsg::Result { data, .. } => match data {
                    QueryResult::Viewer { viewer } => {
                        let seq = self.seq;
                        self.v.apply(viewer, seq);
                    }
                    QueryResult::Replay { replay } => {
                        if self.source != Some(replay.source) {
                            self.source = Some(replay.source);
                            self.v.set_calendar(qp_session::headparts::replay_calendar(
                                replay.source,
                            ));
                        }
                        self.now = Some(replay.t);
                    }
                    QueryResult::Interaction { interaction } => {
                        self.look = interaction.state.clone();
                        if let Some(o) = &interaction.opening {
                            if o.from == OpenDepth::Sentence {
                                self.open_headline = self
                                    .v
                                    .row(&o.capsule)
                                    .map(|i| i.capsule.sentence.text.clone());
                            }
                            if o.to == OpenDepth::Sentence {
                                self.opening = None;
                                self.open_headline = None;
                            } else {
                                self.opening = Some(o.clone());
                            }
                        }
                    }
                    _ => {}
                },
                ServerMsg::Notify { event, .. } => match event {
                    qp_schema::protocol::Event::Viewer { viewer, seq } => {
                        self.seq = *seq;
                        self.v.apply(viewer, *seq);
                    }
                    qp_schema::protocol::Event::Claim { .. } => self.v.on_message(&l.msg),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn input(&mut self, host: &mut ReplayHost, g: Value) {
        self.n += 1;
        let m = json!({"type":"input","id":format!("i{}", self.n),"gesture":g});
        self.send(host, &m);
    }

    fn face_ids(&self, lang: &str) -> Vec<String> {
        let lang = if lang == "en" {
            qp_schema::protocol::Language::En
        } else {
            qp_schema::protocol::Language::Tr
        };
        let items = self.v.items();
        let fresh = vec![false; items.len()];
        crate::shell::feed_rows(
            self.look.feed.as_deref(),
            &items,
            lang,
            self.look.seen,
            &fresh,
            self.now,
        )
        .into_iter()
        .filter_map(|r| match r {
            qp_grammar::feeds::FeedRow::Capsule { id, .. } => Some(id),
            qp_grammar::feeds::FeedRow::Header { .. } => None,
        })
        .collect()
    }

    /// Every feed capsule on screen (so that every face comes, for the digests).
    fn show_all(&mut self, host: &mut ReplayHost) {
        self.n += 1;
        let ids: Vec<String> = self.v.items().into_iter().map(|i| i.id).collect();
        let m = json!({"type":"visible","id":format!("v{}", self.n),
            "sub":crate::viewer::SUB,"visible":ids});
        self.send(host, &m);
    }

    fn digests(&mut self) -> Value {
        let look = self.look.clone();
        let mp = self.max_points;
        let faces: Vec<Value> = self
            .v
            .items()
            .iter()
            .map(|it| json!({"id": it.id, "digest": self.v.face_digest(&it.id)}))
            .collect();
        let head = self.open_headline.clone();
        let open = match (&self.opening, self.v.open_mut()) {
            (Some(o), Some(side)) if side.open.capsule == o.capsule => side
                .replica_mut()
                .session_mut()
                .and_then(|s| open_scene(s, o, head.as_deref(), &look, mp))
                .map(|s| scene_digest(&s)),
            _ => None,
        };
        let trust = self.v.trust();
        json!({
            "mode": "viewer",
            "document": self.v.open().map(|o| o.open.digest.clone()),
            "seq": self.seq,
            "faces": faces,
            "open": {"depth": self.opening.as_ref().map_or("sentence", |o| match o.to {
                        OpenDepth::Sentence => "sentence", OpenDepth::Capsule => "capsule",
                        OpenDepth::Chart => "chart", OpenDepth::Evidence => "evidence"}),
                     "capsule": self.opening.as_ref().map(|o| o.capsule.clone()),
                     "digest": open},
            "look": self.look,
            "data": self.v.data,
            "trust": trust,
        })
    }
}

/// (RP3/M15, §22) [`shell_less`] for a viewer: the same script driven with protocol
/// messages only over a `viewer` subscription (no replica of the document, no data of the
/// universe; the open instrument's data as `qp demo` serves it). Every feed capsule is
/// made visible before a digest, so every face is the authority's ready scene; the open
/// scene is compiled over the device's own data of the open instrument.
///
/// # Panics
/// A malformed script step.
pub fn viewer_less(host: &mut ReplayHost, principal: &str, lang: &str, script: &[Value]) -> Value {
    host.set_record_events(true);
    let l = if lang == "en" {
        qp_schema::protocol::Language::En
    } else {
        qp_schema::protocol::Language::Tr
    };
    let mut a = ViewerAgent {
        v: crate::viewer::Viewer::new(principal, l, Vec::new()),
        look: ViewState::default(),
        opening: None,
        open_headline: None,
        max_points: 60,
        n: 0,
        now: None,
        source: None,
        served: Vec::new(),
        seq: 0,
    };
    a.send(
        host,
        &json!({"type":"hello","protocol":"qp/0","principal":principal,
            "surface":{"kind":"phone","screen":[390,844],"input":["touch"],"lang":lang}}),
    );
    a.input(host, ShellCore::presented_look());
    a.send(
        host,
        &json!({"type":"subscribe","id":crate::viewer::SUB,
            "topic":{"kind":"viewer","limit":crate::shell::FEED_LIMIT,"width":PHONE_FRAME.0}}),
    );
    // The replay status (its calendar for the open replica's headline pieces).
    a.n += 1;
    let m = json!({"type":"replay","id":format!("r{}", a.n),"control":{"kind":"status"}});
    a.send(host, &m);
    let mut out = serde_json::Map::new();
    let fw = PHONE_FRAME.0;
    let (sw, sh) = PHONE_FRAME;
    for step in script {
        if let Some(n) = step.get("seek_step").and_then(Value::as_u64) {
            a.n += 1;
            let m = json!({"type":"replay","id":format!("r{}", a.n),
                "control":{"kind":"seek","step":n}});
            a.send(host, &m);
        } else if let Some(t) = step.get("tap_face") {
            let i = usize::try_from(t["index"].as_u64().unwrap_or(0)).unwrap_or(0);
            let cap = a.face_ids(lang)[i].clone();
            let fh = qp_grammar::faces::height_in(
                qp_session::viewer::look_of(&a.look),
                a.look.face.as_deref(),
            );
            a.input(
                host,
                json!({"kind":"tap_at","x":f(t,"x"),"y":f(t,"y"),"width":fw,"height":fh,
                       "target":{"capsule":cap}}),
            );
        } else if let Some(t) = step.get("tap_scene") {
            a.input(
                host,
                json!({"kind":"tap_at","x":f(t,"x"),"y":f(t,"y"),"width":sw,"height":sh}),
            );
        } else if let Some(t) = step.get("drag") {
            a.input(
                host,
                json!({"kind":"drag","dx":f(t,"dx"),"width":sw,"height":sh}),
            );
        } else if let Some(t) = step.get("wheel") {
            let factor = if f(t, "dy") < 0.0 {
                crate::shell::WHEEL_FACTOR
            } else {
                1.0 / crate::shell::WHEEL_FACTOR
            };
            a.input(
                host,
                json!({"kind":"pinch","factor":factor,"x":f(t,"x"),"width":sw,"height":sh}),
            );
        } else if step.get("pinch2x").is_some() {
            a.input(
                host,
                json!({"kind":"pinch","factor":2.0,"x":250.0,"width":sw,"height":sh}),
            );
        } else if let Some(t) = step.get("style") {
            let mut g = json!({"kind":"style"});
            for k in ["face", "alphabet", "theme", "feed", "transition", "outcome"] {
                if let Some(v) = t.get(k) {
                    g[k] = v.clone();
                }
            }
            a.input(host, g);
        } else if step.get("back").is_some() {
            a.input(host, json!({"kind":"close","width":sw,"height":sh}));
        } else if let Some(label) = step.get("digest").and_then(Value::as_str) {
            a.show_all(host);
            out.insert(label.to_owned(), a.digests());
        } else {
            panic!("unknown script step {step}");
        }
    }
    Value::Object(out)
}

/// Passes everything the core queues to `host`, with the viewer's frames, until idle
/// (`served`: the feeds already served, as [`viewer_frames`] keeps it; R5/M18 tests).
pub fn pump_viewer(
    core: &mut ShellCore,
    host: &mut ReplayHost,
    served: &mut Vec<qp_schema::data::FeedKey>,
) {
    for _ in 0..1000 {
        let out = core.take_outbox();
        if out.is_empty() {
            break;
        }
        for m in out {
            let before = host.recorded_events().len();
            let lines = host.handle_raw(CONN, &m.to_string());
            for fr in viewer_frames(host, CONN, served, before) {
                let _ = core.on_frame(&fr);
            }
            for l in lines {
                if l.conn == CONN {
                    core.on_message(&l.msg);
                }
            }
        }
    }
}

/// (RP3/M15) [`with_core`] for the viewer core: frames as `qp demo` writes them to a
/// viewer connection (data plane filtered, `bind` frames); every feed capsule is made
/// visible before a digest.
///
/// # Panics
/// A malformed script step.
pub fn with_viewer_core(
    host: &mut ReplayHost,
    principal: &str,
    lang: &str,
    script: &[Value],
) -> Value {
    host.set_record_events(true);
    let mut core = ShellCore::new_viewer(principal, lang, &[]).expect("core");
    let mut served = Vec::new();
    pump_viewer(&mut core, host, &mut served);
    let mut out = BTreeMap::new();
    for step in script {
        if let Some(n) = step.get("seek_step").and_then(Value::as_u64) {
            core.seek(&n.to_string());
        } else if let Some(t) = step.get("tap_face") {
            let i = usize::try_from(t["index"].as_u64().unwrap_or(0)).unwrap_or(0);
            let rows = core.feed_json();
            let ids: Vec<String> = rows
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|r| r["id"].as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let target = format!("face:{}", ids[i]);
            core.pointer(&target, "down", 1, f(t, "x"), f(t, "y"));
            core.pointer(&target, "up", 1, f(t, "x"), f(t, "y"));
        } else if let Some(t) = step.get("tap_scene") {
            core.pointer("scene", "down", 1, f(t, "x"), f(t, "y"));
            core.pointer("scene", "up", 1, f(t, "x"), f(t, "y"));
        } else if let Some(t) = step.get("drag") {
            let (x, dx) = (195.0, f(t, "dx"));
            core.pointer("scene", "down", 1, x, 150.0);
            core.pointer("scene", "move", 1, x + dx, 150.0);
            core.pointer("scene", "up", 1, x + dx, 150.0);
        } else if let Some(t) = step.get("wheel") {
            core.wheel(f(t, "dy"), f(t, "x"));
        } else if step.get("pinch2x").is_some() {
            core.pointer("scene", "down", 1, 150.0, 150.0);
            core.pointer("scene", "down", 2, 250.0, 150.0);
            core.pointer("scene", "move", 2, 350.0, 150.0);
            core.pointer("scene", "up", 2, 350.0, 150.0);
            core.pointer("scene", "up", 1, 150.0, 150.0);
        } else if let Some(t) = step.get("style") {
            core.set_look(t);
        } else if step.get("back").is_some() {
            core.back();
        } else if let Some(label) = step.get("digest").and_then(Value::as_str) {
            let ids: Vec<String> = core.feed().iter().map(|i| i.id.clone()).collect();
            for id in &ids {
                core.visible(id, true);
            }
            pump_viewer(&mut core, host, &mut served);
            let mut d = core.digests();
            // (R5/M18, §23.5) The story list the device holds and how often it asked.
            let st = core.stories_json();
            d["stories"] = json!({
                "rows": st["stories"].as_array().map(Vec::len),
                "total": st["total"], "complete": st["complete"],
                "instrument": st["instrument"], "asked": core.stories_asked(),
            });
            out.insert(label.to_owned(), d);
            continue;
        } else {
            panic!("unknown script step {step}");
        }
        pump_viewer(&mut core, host, &mut served);
    }
    serde_json::to_value(out).unwrap_or_default()
}
