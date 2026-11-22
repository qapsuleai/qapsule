//! Viewer-less driving (RG1: moved out of `drive.rs`): the device as a viewer driven only
//! with protocol messages (`viewer_less`), and the same script through the shell core
//! (`with_viewer_core`).

use super::*;

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
    let evs = &host.recorded_since(before);
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
    open_voice: Option<qp_schema::protocol::CapsuleVoice>,
    max_points: u32,
    n: u64,
    now: Option<i64>,
    source: Option<qp_schema::protocol::ReplaySourceKind>,
    served: Vec<qp_schema::data::FeedKey>,
    seq: u64,
    /// (R5/S) The reader's language and the open instrument's story list (as the shell
    /// core asks for it at chart depth, §23.5).
    lang: qp_schema::protocol::Language,
    stories: Option<crate::viewer::Stories>,
}

impl ViewerAgent {
    /// (R5/S) The open instrument's story list at chart depth, every page, as the shell
    /// core holds it (`crate::viewer::Stories::upsert`); `None` elsewhere.
    fn fetch_stories(&mut self, host: &mut ReplayHost) {
        let chart = self
            .opening
            .as_ref()
            .is_some_and(|o| o.to >= OpenDepth::Chart);
        let Some((instrument, series)) = self.v.stories_target(chart) else {
            self.stories = None;
            return;
        };
        let mut st = crate::viewer::Stories {
            instrument: instrument.clone(),
            series: series.clone(),
            ..crate::viewer::Stories::default()
        };
        let mut after: Option<Value> = None;
        loop {
            self.n += 1;
            let mut q = json!({"kind":"stories","instrument":instrument,"series":series,
                "limit":qp_schema::protocol::STORIES_MAX_LIMIT});
            if let Some(a) = &after {
                q["after"] = a.clone();
            }
            let m = json!({"type":"query","id":format!("st{}", self.n),"query":q});
            let mut next = None;
            for l in host.handle_raw(CONN, &m.to_string()) {
                if l.conn != CONN {
                    continue;
                }
                if let ServerMsg::Result {
                    data:
                        QueryResult::Stories {
                            stories, next: n, ..
                        },
                    ..
                } = &l.msg
                {
                    for r in stories {
                        st.upsert(r.clone());
                    }
                    next = n.as_ref().map(|x| json!(x));
                }
            }
            match next {
                Some(n) => after = Some(n),
                None => break,
            }
        }
        self.stories = Some(st);
    }

    fn send(&mut self, host: &mut ReplayHost, m: &Value) {
        let before = host.recorded_len();
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
                                self.open_voice =
                                    self.v.row(&o.capsule).and_then(|i| i.capsule.voice.clone());
                            }
                            if o.to == OpenDepth::Sentence {
                                self.opening = None;
                                self.open_headline = None;
                                self.open_voice = None;
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
        // (R5/fix47) The presentation's feed order, as the shell core orders its feed.
        crate::shell::feed_rows(
            Some(qp_grammar::feeds::PRESENTED_FEED_ORDER),
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
        let voice = self.open_voice.clone();
        // (R5/S) As the shell core builds it: the story list the device holds, the open
        // design's language and the device's check.
        let lang = self.lang;
        let rows: Vec<qp_schema::protocol::StoryRow> = match (&self.opening, &self.stories) {
            (Some(o), Some(st)) if look.story.is_some() && o.to >= OpenDepth::Chart => {
                st.rows.clone()
            }
            _ => Vec::new(),
        };
        let designed = look.open.is_some();
        let open = match (&self.opening, self.v.open_mut()) {
            (Some(o), Some(side)) if side.open.capsule == o.capsule => {
                let extra = qp_session::viewer::OpenExtra {
                    lang: designed.then_some(lang),
                    check: designed.then(|| side.trust()),
                };
                side.replica_mut()
                    .session_mut()
                    .and_then(|s| {
                        open_scene_with(
                            s,
                            o,
                            head.as_deref(),
                            voice.as_ref(),
                            &look,
                            mp,
                            (&rows, lang),
                            extra,
                        )
                    })
                    .map(|s| scene_digest(&s))
            }
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
        open_voice: None,
        max_points: 60,
        n: 0,
        now: None,
        source: None,
        served: Vec::new(),
        seq: 0,
        lang: l,
        stories: None,
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
        // (R5/S) The scene frame as the shell core reports it (`ShellCore::frame_at`): an
        // open capsule design may state the capsule depth's height.
        let cur = a.opening.as_ref().map_or(OpenDepth::Chart, |o| o.to);
        let (sw, sh) = scene_frame(&a.look, cur, (sw, sh));
        if let Some(n) = step.get("seek_step").and_then(Value::as_u64) {
            a.n += 1;
            let m = json!({"type":"replay","id":format!("r{}", a.n),
                "control":{"kind":"seek","step":n}});
            a.send(host, &m);
        } else if let Some(t) = step.get("tap_face") {
            let i = usize::try_from(t["index"].as_u64().unwrap_or(0)).unwrap_or(0);
            let cap = a.face_ids(lang)[i].clone();
            let fh = qp_grammar::faces::height_for(
                qp_session::viewer::look_of(&a.look),
                a.look.face.as_deref(),
                a.look.voice.is_some(),
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
            for k in [
                "face",
                "alphabet",
                "theme",
                "feed",
                "transition",
                "outcome",
                "look",
                "voice",
                "story",
                "open",
            ] {
                if let Some(v) = t.get(k) {
                    g[k] = v.clone();
                }
            }
            a.input(host, g);
        } else if step.get("back").is_some() {
            let to = match cur {
                OpenDepth::Evidence => OpenDepth::Chart,
                OpenDepth::Chart => OpenDepth::Capsule,
                _ => OpenDepth::Sentence,
            };
            let (cw, ch) = scene_frame(&a.look, to, PHONE_FRAME);
            a.input(host, json!({"kind":"close","width":cw,"height":ch}));
        } else if let Some(label) = step.get("digest").and_then(Value::as_str) {
            a.show_all(host);
            a.fetch_stories(host);
            out.insert(label.to_owned(), a.digests());
        } else {
            panic!("unknown script step {step}");
        }
    }
    Value::Object(out)
}

/// Passes everything the core queues to `host`, with the viewer's frames, until idle.
/// (R5/M18: public for the story chart tests.)
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
            let before = host.recorded_len();
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
