//! Faces and scenes (RG1: moved out of `shell.rs`): sizes, device scale, pixels, the
//! change checks, the open capsule's JSON, evidence and status.

use super::*;

impl ShellCore {
    /// (R4/H-M8910) The face frame: `{css:[w,h], px:[w,h], style:{width,height}}` — the
    /// design's height (`qp_designs::faces::height`), the backing size at the device scale
    /// and the CSS lengths (the shell sizes its canvases with these, computes nothing).
    #[must_use]
    pub fn face_size_json(&self) -> Value {
        let (w, h) = self.frames["face"];
        self.size_json(w, h)
    }

    fn size_json(&self, w: u32, h: u32) -> Value {
        json!({"css": [w, h], "px": [w * self.scale, h * self.scale],
               "style": {"width": format!("{w}px"), "height": format!("{h}px")}})
    }

    /// (R4/H-M8910) The device pixel ratio the page reports; the core paints at the nearest
    /// integer scale in `1..=MAX_SCALE`. Returns whether the scale changed.
    pub fn set_dpr(&mut self, dpr: f64) -> bool {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let s = if dpr.is_finite() && dpr > 0.0 {
            (dpr.round() as u32).clamp(1, MAX_SCALE)
        } else {
            1
        };
        let changed = s != self.scale;
        self.scale = s;
        if changed {
            self.face_shown.clear();
            self.scene_shown = None;
        }
        changed
    }

    /// The point budget of the connection's profile (from `welcome`).
    #[must_use]
    pub fn max_points(&self) -> u32 {
        self.max_points
    }

    /// The device scale scenes are painted at.
    #[must_use]
    pub fn scale(&self) -> u32 {
        self.scale
    }

    pub(crate) fn pixels(&self, scene: &SceneDesc) -> Vec<u8> {
        qp_render::render_scene_rgba_scaled(scene, self.scale).unwrap_or_default()
    }

    /// The face scene of capsule `id` at the face frame (R5/M21: with its change label
    /// under reduced motion, `motion`).
    #[must_use]
    pub fn face(&mut self, id: &str) -> Option<SceneDesc> {
        let sc = self.face_plain(id)?;
        Some(self.stamped(id, sc))
    }

    /// The face scene of capsule `id` as its face design compiles it.
    pub(crate) fn face_plain(&mut self, id: &str) -> Option<SceneDesc> {
        let (w, h) = self.frames["face"];
        let mut scene = if let Some(v) = &self.viewer {
            // (RP3/M15) A viewer's face is the scene the authority compiled.
            v.face(id).cloned()?
        } else {
            let it = self.feed.iter().find(|i| i.id == id)?.clone();
            let look = self.look.clone();
            let mp = self.max_points;
            face_scene(self.replica.session_mut()?, &it, &look, mp, w, h)?
        };
        // (R5/M21) The motion design's still marks (none by default).
        self.decorate_face(id, &mut scene);
        Some(scene)
    }

    /// Pixels (RGBA) of capsule `id`'s face, or empty when it has not changed since the
    /// last call (or has no scene).
    pub fn face_rgba(&mut self, id: &str, force: bool) -> Vec<u8> {
        let Some(scene) = self.face(id) else {
            return Vec::new();
        };
        if !self.face_changed(id, &scene, force) {
            return Vec::new();
        }
        self.pixels(&scene)
    }

    /// The open capsule's scene.
    pub fn scene(&mut self) -> Option<SceneDesc> {
        let o = self.opening.clone()?;
        // (RP3/M14) Builds of the open scene (unchanged ones = builds − scene paints).
        *self
            .metrics
            .counts
            .entry("scene_builds".to_owned())
            .or_default() += 1;
        let look = self.look.clone();
        let mp = self.max_points;
        let head = self.open_headline.clone();
        let voice = self.open_voice.clone();
        // (RP3/S) The build's own time (`scene_build`), apart from the change check.
        let t0 = core_now();
        let lang = self.lang;
        let story = look.story.is_some() && o.to >= OpenDepth::Chart;
        // (R5/M20) An open capsule design reads the reader's language and (viewer) the
        // device's own check; today's card reads neither.
        let designed = look.open.is_some();
        let mut extra = qp_session::viewer::OpenExtra {
            lang: designed.then_some(self.lang),
            check: None,
        };
        let scene = if let Some(v) = self.viewer.as_mut() {
            // (R5/M18, §23.5) The story chart draws the list the authority sent.
            // (Taken out and put back: not copied on every build.)
            let rows = if story {
                v.stories
                    .as_mut()
                    .map(|st| std::mem::take(&mut st.rows))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            // (RP3/M15) The open capsule over the device's own data of its instrument.
            let scene = v
                .open_mut()
                .filter(|s| s.open.capsule == o.capsule)
                .and_then(|side| {
                    if designed {
                        extra.check = Some(side.trust());
                    }
                    side.replica_mut().session_mut()
                })
                .and_then(|s| {
                    open_scene_with(
                        s,
                        &o,
                        head.as_deref(),
                        voice.as_ref(),
                        &look,
                        mp,
                        (&rows, lang),
                        extra,
                    )
                });
            if let Some(st) = v.stories.as_mut() {
                if st.rows.is_empty() {
                    st.rows = rows;
                }
            }
            scene
        } else {
            let principal = self.principal.clone();
            let s = self.replica.session_mut()?;
            // (R5/M18) The full replica lists the stories itself (the same rule).
            let rows = if story {
                story_series(s, &o)
                    .map(|bs| s.story_rows(&bs, &principal, lang))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            open_scene_with(
                s,
                &o,
                head.as_deref(),
                voice.as_ref(),
                &look,
                mp,
                (&rows, lang),
                extra,
            )
        };
        let t1 = core_now();
        self.core_span("scene_build", t0, t1);
        scene
    }

    /// (RP3/S) Whether capsule `id`'s face `scene` differs from the one last handed out
    /// (always with `force`); records it. The check's time is the `change_check` span.
    pub(crate) fn face_changed(&mut self, id: &str, scene: &SceneDesc, force: bool) -> bool {
        let t0 = core_now();
        let k = shown_key(scene);
        let t1 = core_now();
        self.core_span("change_check", t0, t1);
        if !force && self.face_shown.get(id) == Some(&k) {
            return false;
        }
        self.face_shown.insert(id.to_owned(), k);
        // (R5/M21) The face as handed out (a marking motion design keeps it).
        self.face_shown_now(id, scene);
        true
    }

    /// (RP3/S) As [`ShellCore::face_changed`], for the open capsule's scene.
    pub(crate) fn scene_changed(&mut self, scene: &SceneDesc, force: bool) -> bool {
        let t0 = core_now();
        let k = shown_key(scene);
        let t1 = core_now();
        self.core_span("change_check", t0, t1);
        if !force && self.scene_shown.as_ref() == Some(&k) {
            return false;
        }
        self.scene_shown = Some(k);
        true
    }

    /// (RP3/S) Records a span of the core's own work (in wasm only: elsewhere there is no
    /// clock and nothing is recorded).
    pub(crate) fn core_span(&mut self, name: &str, t0: f64, t1: f64) {
        if cfg!(target_arch = "wasm32") {
            self.metric_span(name, t0, t1);
        }
    }

    /// Pixels of the open capsule's scene (empty when unchanged or none).
    pub fn scene_rgba(&mut self, force: bool) -> Vec<u8> {
        let Some(scene) = self.scene() else {
            self.scene_shown = None;
            return Vec::new();
        };
        if !self.scene_changed(&scene, force) {
            return Vec::new();
        }
        self.pixels(&scene)
    }

    /// The open capsule: `{depth, capsule, text, frame:[w,h], scene, px?, style?, trust?}`
    /// (`px`/`style`: the scene's backing size at the device scale and its CSS lengths;
    /// RP3/M15 `trust`: a viewer's trust mark of the open capsule, checked on its own data);
    /// depth `sentence` when nothing is open.
    pub fn open_json(&mut self) -> Value {
        let mut v = self.open_json_base();
        if v["depth"] != "sentence" {
            let state = self.viewer.as_mut().and_then(Viewer::trust);
            if let Some(t) = state {
                v["trust"] = json!(qp_session::viewer::trust_text(t, self.lang));
            }
            self.open_design_text(&mut v, state);
        }
        v
    }

    /// (R5/M20) Under an open capsule design, the open layer's text at the capsule depth:
    /// above a scene the design's (`tesir`: none, the scene writes the headline); without
    /// a scene (no data entitlement, A32) the design's text column from the feed item and
    /// the opening (the same reader rule). The scene's CSS lets the page scroll the layer
    /// over it (`touch-action: pan-y`) when the design states its own height.
    fn open_design_text(&self, v: &mut Value, state: Option<qp_session::viewer::TrustState>) {
        let Some(o) = self.opening.as_ref().filter(|o| o.to == OpenDepth::Capsule) else {
            return;
        };
        let design = qp_designs::open::by_name(self.look.open.as_deref());
        if self.look.open.is_none() {
            return;
        }
        if o.frame.is_some() {
            if let Some(f) = design.scene_text {
                v["text"] = json!(f());
            }
            if design.height.is_some() {
                v["style"]["touchAction"] = json!("pan-y");
            }
            return;
        }
        let (Some(f), Some(it)) = (design.text, self.feed.iter().find(|i| i.id == o.capsule))
        else {
            return;
        };
        let t = qp_designs::open::TextInput {
            lang: self.lang,
            headline: &it.sentence.text,
            voice: self.open_voice.as_ref(),
            facts: &o.presentation.facts,
            trust: it.trust,
            check: state.map(qp_session::viewer::TrustState::device_check),
            ledger: it.ledger_at_birth,
            validity: it.validity,
        };
        v["text"] = json!(f(&t));
    }

    fn open_json_base(&self) -> Value {
        match &self.opening {
            None => json!({"depth": "sentence"}),
            Some(o) => {
                let mut v = json!({
                    "depth": depth_name(o.to),
                    "capsule": o.capsule,
                    // (R5/m17-int) Under a voice the open layer's title is its headline.
                    "text": self.open_voice.as_ref().map(|v| v.headline.clone())
                        .or_else(|| self.open_headline.clone())
                        .unwrap_or_else(|| o.presentation.text.clone()),
                    "frame": o.frame,
                    "scene": o.frame.is_some(),
                });
                // (R5/M20 `nobra`) The open capsule design's own hint for its depth (absent:
                // the page's).
                if let Some(h) = o.frame.and_then(|_| {
                    qp_designs::open::hint(self.look.open.as_deref(), o.to, self.lang)
                }) {
                    v["hint"] = json!(h);
                }
                // (R5/M20 `nobra`) Without a scene (no data of the instrument, A32) the
                // design's question cards as text, `[{key, q, a, d}]`, from the feed item.
                if o.frame.is_none() {
                    let rows = self
                        .feed
                        .iter()
                        .find(|i| i.id == o.capsule)
                        .map(|i| {
                            qp_designs::open::text_rows(
                                self.look.open.as_deref(),
                                o.to,
                                i,
                                self.lang,
                            )
                        })
                        .unwrap_or_default();
                    if !rows.is_empty() {
                        v["asks"] = Value::Array(
                            rows.iter()
                                .map(|r| json!({"key": r.key, "q": r.q, "a": r.a, "d": r.d}))
                                .collect(),
                        );
                    }
                }
                if let Some([w, h]) = o.frame {
                    let sz = self.size_json(w, h);
                    v["px"] = sz["px"].clone();
                    v["style"] = sz["style"].clone();
                }
                v
            }
        }
    }

    /// The evidence rows at evidence depth: `[{fact, family, values:[[name, text]],
    /// requires, rests_on}]` — values formatted here (shortest round-trip), the
    /// requirement as the authority wrote it.
    #[must_use]
    pub fn evidence_json(&self) -> Value {
        let Some(o) = self
            .opening
            .as_ref()
            .filter(|o| o.to == OpenDepth::Evidence)
        else {
            return json!([]);
        };
        Value::Array(o.evidence.iter().map(evidence_row).collect())
    }

    /// The replay state, formatted: `{time, step, speed, paused, done, synthetic, note,
    /// source, days}`.
    #[must_use]
    pub fn status_json(&self) -> Value {
        let mut v = self.replay_status_json();
        // (R6/T3) The market's status line, only once the host gave one.
        if self.market.is_some() {
            v["market"] = self.market_json();
        }
        v
    }

    fn replay_status_json(&self) -> Value {
        let Some(r) = &self.replay else {
            return json!({});
        };
        json!({
            "time": display_time(r.t),
            "step": format!("{} / {}", r.step, r.steps),
            "speed": format!("{}", r.speed),
            "paused": r.paused,
            "done": r.done,
            "synthetic": r.synthetic,
            "note": r.note,
            "source": format!("{:?}", r.source).to_lowercase(),
            "instruments": r.instruments.to_string(),
            "days": format!("{} / {} / {}", r.days.fine, r.days.ohlc, r.days.bar),
            "epoch": r.epoch,
        })
    }

    /// The last error message (as the authority wrote it), cleared by reading.
    pub fn take_error(&mut self) -> Option<String> {
        self.error.take()
    }
}

pub(super) fn evidence_row(e: &EvidenceAccess) -> Value {
    let values: Vec<Value> = e
        .values
        .iter()
        // Two decimals, the grammar's value-label convention (`docs/GRAMMAR.md` §14).
        .map(|(k, v)| json!([k, format!("{v:.2}")]))
        .collect();
    json!({
        "fact": e.fact,
        "family": format!("{:?}", e.family).to_lowercase(),
        "values": values,
        "requires": e.requires,
        "rests_on": e.rests_on,
    })
}
