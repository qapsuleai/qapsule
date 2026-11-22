//! What reaches the shell core (RG1: moved out of `shell.rs`): server messages, the
//! stories pages, the viewer updates, the market status and the large frames.

use super::*;

impl ShellCore {
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    /// One message of the authority on this connection. Returns what changed:
    /// `feed`, `scene`, `status`, `error`, `transition`.
    pub fn on_message(&mut self, msg: &ServerMsg) -> Value {
        let mut ch = json!({});
        if self.viewer.is_none() {
            self.replica.on_message(msg);
        }
        match msg {
            ServerMsg::Welcome { profile, .. } => {
                self.welcomed = true;
                self.max_points = profile.max_points;
                ch["welcome"] = json!(true);
                if self.viewer.is_some() {
                    // (RP3/M15, §22) The visibility subscription instead of the feed
                    // topic and the document: the feed comes with its faces.
                    let w = self.frames["face"].0;
                    self.push(json!({"type":"subscribe","id":crate::viewer::SUB,
                        "topic":{"kind":"viewer","limit":FEED_LIMIT,"width":w}}));
                } else {
                    self.push(
                        json!({"type":"subscribe","id":"feed-topic","topic":{"kind":"capsules"}}),
                    );
                    self.feed_dirty = true;
                }
                self.agent_welcome();
                self.ask(
                    "replay",
                    Asked::Replay,
                    json!({"type":"replay","control":{"kind":"status"}}),
                );
                if self.look.face.is_some()
                    || self.look.alphabet.is_some()
                    || self.look.theme.is_some()
                {
                    let st = json!({"kind":"style","face":self.look.face,"alphabet":self.look.alphabet,
                        "theme":self.look.theme.clone().unwrap_or_else(|| "view".to_owned())});
                    self.ask("in", Asked::Input, json!({"type":"input","gesture":st}));
                }
                // (R4/H-M8910) The other remembered settings, only when one is set (the
                // message above stays byte-identical for the T5 settings).
                let mut more = json!({"kind":"style"});
                for (k, v) in [
                    ("feed", &self.look.feed),
                    ("transition", &self.look.transition),
                    ("outcome", &self.look.outcome),
                    ("look", &self.look.look),
                    ("voice", &self.look.voice),
                    ("story", &self.look.story),
                    ("open", &self.look.open),
                ] {
                    if let Some(v) = v {
                        more[k] = json!(v);
                    }
                }
                if self.look.hide_closed {
                    more["hide_closed"] = json!(true);
                }
                if more.as_object().is_some_and(|o| o.len() > 1) {
                    self.ask("in", Asked::Input, json!({"type":"input","gesture":more}));
                }
                ch["status"] = json!(true);
            }
            ServerMsg::Result { id, data, .. } => {
                let kind = self.asked.remove(id);
                match (kind, data) {
                    (
                        Some(Asked::Feed),
                        QueryResult::Capsules {
                            capsules,
                            coin_flip_hidden,
                            ..
                        },
                    ) => {
                        self.feed_inflight = false;
                        let changes = self.changes_before(capsules);
                        self.feed.clone_from(capsules);
                        self.changes_after(changes);
                        self.feed_known = true;
                        self.coin_flip_hidden = *coin_flip_hidden;
                        self.observe_feed();
                        // (RP3/T4) Only a face in the feed can be painted: the digests of
                        // the faces that left it are forgotten (bounded by the feed).
                        let feed = &self.feed;
                        self.face_shown
                            .retain(|id, _| feed.iter().any(|i| &i.id == id));
                        ch["feed"] = json!(true);
                    }
                    (Some(Asked::Input), QueryResult::Interaction { interaction }) => {
                        self.look = interaction.state.clone();
                        // (R5/m17-int, K49) A tap on a glossary term: its explanation
                        // shows (any other answer closes it).
                        let g = interaction
                            .hit
                            .as_ref()
                            .and_then(|h| h.term.clone())
                            .map(|t| (self.tapped.clone(), t));
                        if g != self.gloss {
                            self.gloss = g;
                            ch["gloss"] = json!(true);
                        }
                        // (R4/H-M8910) The face design states its height (R5/T3: in the
                        // connection's look).
                        let fh = qp_grammar::faces::height_for(
                            qp_session::viewer::look_of(&self.look),
                            self.look.face.as_deref(),
                            self.look.voice.is_some(),
                        );
                        if let Some(f) = self.frames.get_mut("face") {
                            if f.1 != fh {
                                f.1 = fh;
                                self.face_shown.clear();
                            }
                        }
                        if let Some(o) = &interaction.opening {
                            self.trans_t0 = None;
                            self.shared_from = depth_name(o.from).to_owned();
                            self.shared.clone_from(&o.shared);
                            if o.from == OpenDepth::Sentence && o.to != OpenDepth::Sentence {
                                self.agent_opened(&o.capsule, &mut ch);
                            }
                            if o.from == OpenDepth::Sentence {
                                self.changes_seen(Some(&o.capsule));
                                self.open_headline = self
                                    .feed
                                    .iter()
                                    .find(|i| i.id == o.capsule)
                                    .map(|i| i.sentence.text.clone());
                                self.open_voice = self
                                    .feed
                                    .iter()
                                    .find(|i| i.id == o.capsule)
                                    .and_then(|i| i.voice.clone());
                            }
                            if o.to == OpenDepth::Sentence {
                                self.agent_closed(&mut ch);
                                self.opening = None;
                                self.open_headline = None;
                                self.open_voice = None;
                            } else {
                                self.opening = Some(o.clone());
                            }
                            ch["transition"] = json!(true);
                        }
                        ch["scene"] = json!(true);
                        ch["feed"] = json!(true);
                        self.flush_gesture();
                    }
                    (Some(Asked::Visible), QueryResult::Viewer { viewer }) => {
                        if let Some(v) = self.viewer.as_mut() {
                            v.visible_answered();
                        }
                        let seq = self.viewer_seq;
                        self.apply_viewer(viewer, seq, &mut ch);
                    }
                    (Some(Asked::Stories), d @ QueryResult::Stories { .. }) => {
                        self.stories_page(d, &mut ch);
                    }
                    (Some(Asked::Replay), QueryResult::Replay { replay }) => {
                        self.set_calendar(replay);
                        self.replay = Some(replay.clone());
                        self.seen_when_due();
                        ch["status"] = json!(true);
                    }
                    (Some(Asked::Input), _) => self.flush_gesture(),
                    (Some(Asked::Proposals), QueryResult::Items { items, .. }) => {
                        self.agent_proposals(items, &mut ch);
                    }
                    (Some(Asked::Proposals), _) => self.agent_proposals_failed(),
                    (Some(Asked::Comments), QueryResult::CapsuleDetail { detail }) => {
                        self.agent_comments(&detail.item.id, &detail.comments, &mut ch);
                    }
                    _ => {}
                }
            }
            ServerMsg::Ack { id, .. } => match self.asked.remove(id) {
                Some(Asked::Command) => self.feed_dirty = true,
                Some(Asked::Decision) => {
                    self.feed_dirty = true;
                    self.agent_decided();
                }
                _ => {}
            },
            ServerMsg::Error { id, error } => {
                // (R5B) The agent trace's own queries fail quietly (the trace stays empty).
                let mut quiet = false;
                if let Some(id) = id {
                    match self.asked.remove(id) {
                        Some(Asked::Proposals) => {
                            self.agent_proposals_failed();
                            quiet = true;
                        }
                        Some(Asked::Comments) => quiet = true,
                        Some(Asked::Decision) => self.agent_decided(),
                        Some(Asked::Feed) => self.feed_inflight = false,
                        Some(Asked::Input) => self.flush_gesture(),
                        Some(Asked::Visible) => {
                            if let Some(v) = self.viewer.as_mut() {
                                v.visible_answered();
                            }
                        }
                        _ => {}
                    }
                }
                if !quiet {
                    self.error = Some(error.message.clone());
                    ch["error"] = json!(true);
                }
            }
            // (R5B) A proposal was created or decided: the pending list is asked again.
            ServerMsg::Notify {
                event: Event::Proposal { .. },
                sub,
            } if sub == agent::PROPOSALS_SUB => self.agent_proposal_event(),
            // (R5/M18, §23.4) The story list's changes.
            ServerMsg::Notify { event, sub } if sub == STORIES_SUB => {
                if let Some(st) = self.viewer.as_mut().and_then(|v| v.stories.as_mut()) {
                    match event {
                        Event::Story { story, .. } => st.upsert((**story).clone()),
                        Event::ServerComment { comment, .. } => st.comments.push(comment.clone()),
                        _ => {}
                    }
                    ch["stories"] = json!(true);
                }
            }
            ServerMsg::Notify { event, .. } => match event {
                Event::Capsule { .. } | Event::Change { .. } | Event::ServerComment { .. } => {
                    self.feed_dirty = true;
                    self.version += 1;
                    ch["scene"] = json!(true);
                }
                Event::Viewer { viewer, seq } => {
                    self.viewer_seq = *seq;
                    self.apply_viewer(viewer, *seq, &mut ch);
                }
                Event::Claim { .. } => {
                    if let Some(v) = self.viewer.as_mut() {
                        v.on_message(msg);
                        self.version += 1;
                        ch["scene"] = json!(true);
                    }
                }
                _ => {}
            },
        }
        self.sync_stories(&mut ch);
        ch
    }

    /// (R5/M18, K48, §23.5) Asks for the open instrument's story list when the chart
    /// opens (chart or evidence depth with an open capsule) and drops it when the chart
    /// closes or the open capsule moves to another instrument. Nothing is asked on the
    /// feed screen or at capsule depth.
    fn sync_stories(&mut self, ch: &mut Value) {
        if !self.welcomed {
            return;
        }
        let chart = matches!(self.look.depth, OpenDepth::Chart | OpenDepth::Evidence);
        let Some(v) = self.viewer.as_mut() else {
            return;
        };
        let want = v.stories_target(chart);
        let held = v.stories_held();
        if want == held {
            return;
        }
        if held.is_some() {
            v.stories = None;
            self.ask(
                "unst",
                Asked::Unsubscribe,
                json!({"type":"unsubscribe","sub":STORIES_SUB}),
            );
        }
        if let Some((instrument, series)) = want {
            if let Some(v) = self.viewer.as_mut() {
                v.stories_asked += 1;
                v.stories = Some(Stories {
                    instrument: instrument.clone(),
                    series: series.clone(),
                    ..Stories::default()
                });
            }
            self.ask(
                "st",
                Asked::Stories,
                json!({"type":"query","query":{"kind":"stories","instrument":instrument,
                    "series":series,"limit":qp_schema::protocol::STORIES_MAX_LIMIT}}),
            );
            self.push(json!({"type":"subscribe","id":STORIES_SUB,
                "topic":{"kind":"stories","instrument":instrument,"series":series}}));
        }
        ch["stories"] = json!(true);
    }

    /// (R5/M18) A page of the story list arrived: kept if it is still the list wanted;
    /// the next page is asked for.
    fn stories_page(&mut self, data: &QueryResult, ch: &mut Value) {
        let QueryResult::Stories {
            instrument,
            series,
            stories,
            total,
            next,
            comments,
        } = data
        else {
            return;
        };
        let Some(st) = self
            .viewer
            .as_mut()
            .and_then(|v| v.stories.as_mut())
            .filter(|s| &s.instrument == instrument && &s.series == series)
        else {
            return;
        };
        for r in stories {
            st.upsert(r.clone());
        }
        st.comments.clone_from(comments);
        st.total = *total;
        st.complete = next.is_none();
        ch["stories"] = json!(true);
        if let Some(after) = next {
            if let Some(v) = self.viewer.as_mut() {
                v.stories_asked += 1;
            }
            let q = json!({"type":"query","query":{"kind":"stories","instrument":instrument,
                "series":series,"limit":qp_schema::protocol::STORIES_MAX_LIMIT,"after":after}});
            self.ask("st", Asked::Stories, q);
        }
    }

    /// (R5/M18, §23.5) The open instrument's story list as the device holds it (JSON;
    /// `null`: none — not at chart depth, or the full replica mode).
    #[must_use]
    pub fn stories_json(&self) -> Value {
        self.viewer
            .as_ref()
            .and_then(|v| v.stories.as_ref())
            .map_or(Value::Null, Stories::json)
    }

    /// (R5/M18) How many `stories` queries this device sent (0 in the full replica mode).
    #[must_use]
    pub fn stories_asked(&self) -> u64 {
        self.viewer.as_ref().map_or(0, |v| v.stories_asked)
    }

    /// (R5/M18, K52 measurement only) Replaces the held story list with `n` synthetic
    /// stories spread over the open chart's bars (deterministic: lives of 3–27 bars,
    /// outcomes in turn, the last three open, every 7th a coin flip), so that the paint
    /// budget can be measured with 5 / 50 / 200 stories in view. Returns how many rows
    /// the list now holds (0: no viewer list, e.g. the chart is not open).
    pub fn bench_stories(&mut self, n: u32) -> u32 {
        let bars = self.scene().map_or(0, |s| s.bars);
        let Some(st) = self.viewer.as_mut().and_then(|v| v.stories.as_mut()) else {
            return 0;
        };
        if bars < 8 {
            return 0;
        }
        st.rows = synthetic_rows(u64::from(n), bars, &st.series);
        st.total = n;
        self.version += 1;
        u32::try_from(st.rows.len()).unwrap_or(u32::MAX)
    }

    /// (RP3/M15) Applies a viewer update: the feed rows (the feed the rest of the core
    /// reads), the faces, the open capsule's slice.
    fn apply_viewer(&mut self, up: &qp_schema::protocol::ViewerUpdate, seq: u64, ch: &mut Value) {
        let Some(v) = self.viewer.as_mut() else {
            return;
        };
        let scene = v.apply(up, seq);
        if up.feed.is_some() {
            let items = v.items();
            let changes = self.changes_before(&items);
            self.feed = items;
            self.changes_after(changes);
            self.feed_known = true;
            self.coin_flip_hidden = up.coin_flip_hidden;
            self.observe_feed();
        }
        if up.feed.is_some() || !up.faces.is_empty() || up.audience.is_some() {
            ch["feed"] = json!(true);
        }
        if scene {
            ch["scene"] = json!(true);
        }
        if let Some(m) = &up.market {
            self.set_market(m.clone(), ch);
        }
        self.version += 1;
    }

    /// (R6/T3, §24) Takes the host's market status: the status line changes, and the faces
    /// (a capsule whose instrument is not trading continuously on fresh data shows no live
    /// mark).
    pub fn set_market(&mut self, m: qp_schema::market::MarketStatus, ch: &mut Value) {
        if self.market.as_ref() == Some(&m) {
            return;
        }
        self.market = Some(m);
        ch["status"] = json!(true);
        ch["feed"] = json!(true);
        self.version += 1;
    }

    /// (R6/T3) Whether capsules on `symbol` may show as live: no market status, or the
    /// instrument trades continuously on fresh data (`qp_capsule::market::note_of`).
    #[must_use]
    pub fn market_live(&self, symbol: &str) -> bool {
        self.market
            .as_ref()
            .is_none_or(|m| qp_capsule::market::note_of(m, symbol).is_none())
    }

    /// (R6/T3, §24) The status line in the reader's language: `{text, state}` (`state`:
    /// `live`, `delayed`, `closed`, `stale`); `null` without a market status. The text is
    /// the core's (`qp_capsule::market::status_line`); the shell shows it as it is.
    #[must_use]
    pub fn market_json(&self) -> Value {
        self.market.as_ref().map_or(Value::Null, |m| {
            let (text, state) = qp_capsule::market::status_line(m, self.lang);
            json!({"text": text, "state": state.key()})
        })
    }

    /// One frame of the host's stream (`{"msg":…}`, `{"data":[…],"replay":…}`); returns
    /// what changed (see [`ShellCore::on_message`]).
    ///
    /// # Errors
    /// Not JSON, or a message that is not a server message.
    pub fn on_frame(&mut self, text: &str) -> Result<Value, String> {
        // (R6/T3, §24) The host's market status frame (`{"market":MarketStatus}`).
        if text.starts_with("{\"market\"") {
            self.traffic.record("market", text.len());
            let f: MarketFrame = serde_json::from_str(text).map_err(|e| e.to_string())?;
            let mut ch = json!({});
            self.set_market(f.market, &mut ch);
            return Ok(ch);
        }
        let kind = if text.starts_with("{\"msg\"") {
            "msg"
        } else if text.starts_with("{\"bind\"") {
            "bind"
        } else {
            "data"
        };
        self.traffic.record(kind, text.len());
        // (R4/T6) A message frame as the host writes it (`{"msg":…}`, compact) is parsed
        // straight into the message: no intermediate JSON tree and no copy of it (the
        // document answer is megabytes). Anything else takes the general path.
        if let Some(inner) = text
            .strip_prefix("{\"msg\":")
            .and_then(|r| r.strip_suffix('}'))
        {
            if let Ok(msg) = serde_json::from_str::<ServerMsg>(inner) {
                return Ok(self.on_message(&msg));
            }
        }
        // (RP3/M15, §22) The host serves a feed the viewer now subscribes to (before the
        // data-frame path, which would take a `bind` frame for an empty data frame).
        if kind == "bind" {
            let b: BindFrame = serde_json::from_str(text).map_err(|e| e.to_string())?;
            if let Some(vw) = self.viewer.as_mut() {
                for f in b.bind.feeds {
                    let k = f.feed.clone();
                    let evs: Vec<DataEvent> = b
                        .bind
                        .events
                        .iter()
                        .filter(|e| e.feed == k)
                        .cloned()
                        .collect();
                    vw.bind(f, evs);
                }
            }
            return Ok(json!({"scene": true}));
        }
        // (RP3/T4) A data frame parsed typed in one pass: a JSON tree of a large frame (a
        // forward seek: thousands of events) costs some 20 times its text in memory, and
        // the WASM memory keeps that peak. A frame that does not parse typed (a malformed
        // event or replay state) or carries a message takes the general path below, which
        // skips what is malformed: the result is the same either way.
        if let Ok(f) = serde_json::from_str::<DataFrame>(text) {
            if f.msg.is_none() {
                let mut ch = json!({});
                if let Some(evs) = &f.data {
                    for ev in evs {
                        self.data_event(ev);
                    }
                    self.data_changed(&mut ch, evs.len());
                }
                if let Some(st) = f.replay {
                    self.set_replay(st);
                    ch["status"] = json!(true);
                }
                return Ok(ch);
            }
        }
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if let Some(m) = v.get("msg") {
            let msg: ServerMsg = serde_json::from_value(m.clone()).map_err(|e| e.to_string())?;
            return Ok(self.on_message(&msg));
        }
        let mut ch = json!({});
        if let Some(evs) = v.get("data").and_then(Value::as_array) {
            let mut n = 0;
            for e in evs {
                if let Ok(ev) = serde_json::from_value::<DataEvent>(e.clone()) {
                    self.data_event(&ev);
                    n += 1;
                }
            }
            self.data_changed(&mut ch, n);
        }
        if let Some(r) = v.get("replay") {
            if let Ok(st) = serde_json::from_value::<ReplayState>(r.clone()) {
                self.set_replay(st);
                ch["status"] = json!(true);
            }
        }
        Ok(ch)
    }

    /// (RP3/M14) What `n` data events applied by a data frame may have changed: nothing
    /// when there were none (a frame that only carries the replay state); the open scene
    /// (it reads the device's own data) and, in the full replica, the feed (its faces are
    /// compiled from the replica's data; a viewer's faces come ready from the authority).
    fn data_changed(&self, ch: &mut Value, n: usize) {
        if n == 0 {
            return;
        }
        ch["scene"] = json!(true);
        if self.viewer.is_none() {
            ch["feed"] = json!(true);
        }
    }
}

/// (R5/M18, K52 measurement) `n` synthetic story rows over `bars` bars of `series`.
fn synthetic_rows(n: u64, bars: u64, series: &str) -> Vec<qp_schema::protocol::StoryRow> {
    use qp_schema::capsule::{Direction, MeaningKind, Stage, Validity};
    use qp_schema::protocol::{StoryFace, StoryOutcome, StoryRow};
    let outcomes = [
        Stage::Realized,
        Stage::Invalidated,
        Stage::Expired,
        Stage::Realized,
        Stage::Retracted,
    ];
    let t = |b: u64| i64::try_from(b).unwrap_or(0);
    (0..n)
        .map(|i| {
            let born = (i * (bars - 5) / n.max(1)) % (bars - 5);
            let len = 3 + (i * 7) % 25;
            let open = born + len >= bars - 1 || i + 3 >= n;
            let stage = if open {
                Stage::Born
            } else {
                outcomes[usize::try_from(i % 5).unwrap_or(0)]
            };
            let last = if open { born + len / 2 } else { born + len };
            StoryRow {
                id: format!("bench.{i}"),
                finding: format!("bench.f{i}"),
                series: series.to_owned(),
                kind: MeaningKind::Breakout,
                direction: [Direction::Up, Direction::Down, Direction::Neutral]
                    [usize::try_from(i % 3).unwrap_or(0)],
                tier: 2,
                born_bar: born,
                born_t: t(born),
                last_bar: last,
                last_t: t(last),
                closed_bar: (!open).then_some(last),
                closed_t: (!open).then(|| t(last)),
                stage,
                outcome: StoryOutcome::of(stage),
                validity: if open {
                    Validity::Valid
                } else {
                    Validity::Closed
                },
                in_feed: true,
                hidden: None,
                coin_flip: i % 7 == 0,
                face: StoryFace {
                    event: "kırılım".to_owned(),
                    direction_word: String::new(),
                    when: String::new(),
                    moved: String::new(),
                    validity_phrase: if open { "geçerli" } else { "kapandı" }.to_owned(),
                    ledger_short: String::new(),
                    few_cases: false,
                    ledger_n: None,
                },
                redacted: false,
            }
        })
        .collect()
}
