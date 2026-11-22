//! The reader's actions (RG1: moved out of `shell.rs`): frames, replay control,
//! opening and closing, the gesture recognizer, the wheel and the transition.

use super::*;

impl ShellCore {
    /// The shell lays out a frame (`face`, `scene`) at `w` × `h` CSS px.
    pub fn set_frame(&mut self, key: &str, w: u32, h: u32) {
        self.frames.insert(key.to_owned(), (w, h));
        if key == "face" {
            self.face_shown.clear();
        }
    }

    /// A replay control (`ReplayControl` JSON) as a `replay` message.
    pub fn replay_control(&mut self, control: Value) {
        self.ask(
            "replay",
            Asked::Replay,
            json!({"type":"replay","control":control}),
        );
    }

    /// Seek to a date `YYYY-MM-DD` (as the date input gives it) or a step number.
    pub fn seek(&mut self, date_or_step: &str) {
        let c = match date_or_step.parse::<u64>() {
            Ok(n) => json!({"kind":"seek","step":n}),
            Err(_) => json!({"kind":"seek","date":date_or_step}),
        };
        self.replay_control(c);
    }

    /// (R4/H-M8910) Open capsule `id` from the feed to its capsule depth (the `open`
    /// gesture; what a tap on its face does, by id).
    pub fn open_capsule(&mut self, id: &str) {
        let (w, h) = self.frame_at(OpenDepth::Capsule);
        self.ask(
            "in",
            Asked::Input,
            json!({"type":"input","gesture":{"kind":"open","capsule":id,"to":"capsule","width":w,"height":h}}),
        );
    }

    /// Open the open capsule straight to `to` (`chart`, `evidence`): the way down when the
    /// reader gets no scene to tap (no data entitlement).
    pub fn open_to(&mut self, to: &str) {
        let Some(cap) = self.opening.as_ref().map(|o| o.capsule.clone()) else {
            return;
        };
        if !["capsule", "chart", "evidence"].contains(&to) {
            return;
        }
        let (w, h) = self.frame_at(if to == "capsule" {
            OpenDepth::Capsule
        } else {
            OpenDepth::Chart
        });
        self.ask(
            "in",
            Asked::Input,
            json!({"type":"input","gesture":{"kind":"open","capsule":cap,"to":to,"width":w,"height":h}}),
        );
    }

    /// Back: close one depth (to the feed from the capsule).
    pub fn back(&mut self) {
        let Some(to) = self.opening.as_ref().map(|o| match o.to {
            OpenDepth::Evidence => OpenDepth::Chart,
            OpenDepth::Chart => OpenDepth::Capsule,
            _ => OpenDepth::Sentence,
        }) else {
            return;
        };
        let (w, h) = self.frame_at(to);
        self.ask(
            "in",
            Asked::Input,
            json!({"type":"input","gesture":{"kind":"close","width":w,"height":h}}),
        );
    }

    fn frame_of(&self, target: &str) -> (u32, u32) {
        if target.starts_with("face:") {
            self.frames["face"]
        } else {
            self.frame_at(self.opening.as_ref().map_or(OpenDepth::Chart, |o| o.to))
        }
    }

    /// (R5/M20) The scene frame at open depth `d`: the surface's scene frame, except at
    /// the capsule depth under an open capsule design that states its own height
    /// (`qp_grammar::open::height_in`; the page scrolls the layer).
    fn frame_at(&self, d: OpenDepth) -> (u32, u32) {
        let (w, h) = self.frames["scene"];
        if d != OpenDepth::Capsule {
            return (w, h);
        }
        let look = qp_session::viewer::look_of(&self.look);
        qp_grammar::open::height_in(look, self.look.open.as_deref()).map_or((w, h), |oh| (w, oh))
    }

    fn tap(&mut self, target: &str, x: f64, y: f64) {
        target.clone_into(&mut self.tapped);
        let (w, h) = self.frame_of(target);
        let mut g = json!({"kind":"tap_at","x":x,"y":y,"width":w,"height":h});
        if let Some(cap) = target.strip_prefix("face:") {
            g["target"] = json!({"capsule": cap});
        }
        self.ask("in", Asked::Input, json!({"type":"input","gesture":g}));
    }

    pub(super) fn chart_open(&self) -> bool {
        self.opening
            .as_ref()
            .is_some_and(|o| o.to >= OpenDepth::Chart && o.frame.is_some())
    }

    /// Sends the movement accumulated while an input was in flight.
    pub(super) fn flush_gesture(&mut self) {
        if self.input_in_flight() {
            return;
        }
        let Some(g) = self.gesture.as_mut() else {
            return;
        };
        let (dx, f, x) = (g.acc_dx, g.acc_factor, g.acc_x.take());
        g.acc_dx = 0.0;
        g.acc_factor = 1.0;
        if dx != 0.0 {
            self.send_drag(dx);
        } else if f != 1.0 && f > 0.0 {
            self.send_pinch(f, x);
        }
    }

    fn send_drag(&mut self, dx: f64) {
        let (w, h) = self.frames["scene"];
        self.ask(
            "in",
            Asked::Input,
            json!({"type":"input","gesture":{"kind":"drag","dx":dx,"width":w,"height":h}}),
        );
    }

    fn send_pinch(&mut self, factor: f64, x: Option<f64>) {
        let (w, h) = self.frames["scene"];
        let mut g = json!({"kind":"pinch","factor":factor,"width":w,"height":h});
        if let Some(x) = x {
            g["x"] = json!(x);
        }
        self.ask("in", Asked::Input, json!({"type":"input","gesture":g}));
    }

    /// A raw pointer event on `target` (`face:<capsule>` or `scene`): `phase` `down`,
    /// `move`, `up` or `cancel`; `x`, `y` in CSS px inside the target. The recognizer
    /// (here, not in the shell) turns presses into `tap_at`, horizontal movement on an
    /// open chart into `drag`, two pointers into `pinch`; movement is coalesced while an
    /// input is in flight. Returns the gesture mode (`tap`, `drag`, `pinch`, `none`).
    pub fn pointer(&mut self, target: &str, phase: &str, id: i32, x: f64, y: f64) -> String {
        if !(x.is_finite() && y.is_finite()) {
            return "none".to_owned();
        }
        let chart = target == "scene" && self.chart_open();
        match phase {
            "down" => {
                let g = self.gesture.get_or_insert_with(|| Gesture {
                    target: target.to_owned(),
                    acc_factor: 1.0,
                    ..Gesture::default()
                });
                if g.target != target {
                    return "none".to_owned();
                }
                g.ptrs.retain(|p| p.0 != id);
                g.ptrs.push((id, x, y, x, y));
                if g.ptrs.len() == 2 && chart {
                    g.pinching = true;
                    g.dragging = false;
                    g.last_d = dist(&g.ptrs);
                }
                "none".to_owned()
            }
            "move" => {
                let Some(g) = self.gesture.as_mut() else {
                    return "none".to_owned();
                };
                let Some(p) = g.ptrs.iter_mut().find(|p| p.0 == id) else {
                    return "none".to_owned();
                };
                p.3 = x;
                p.4 = y;
                let (x0, y0) = (p.1, p.2);
                if g.pinching && g.ptrs.len() == 2 {
                    let d = dist(&g.ptrs);
                    if g.last_d > 0.0 && d > 0.0 {
                        g.acc_factor *= d / g.last_d;
                        g.acc_x = Some(((g.ptrs[0].3 + g.ptrs[1].3) / 2.0).round());
                        g.last_d = d;
                    }
                    self.flush_gesture();
                    return "pinch".to_owned();
                }
                let moved = (x - x0).abs().max((y - y0).abs());
                if !g.dragging && moved > TAP_SLOP {
                    if chart && (x - x0).abs() >= (y - y0).abs() {
                        g.dragging = true;
                        g.last_x = x0;
                    } else {
                        g.cancelled = true;
                    }
                }
                if g.dragging {
                    g.acc_dx += x - g.last_x;
                    g.last_x = x;
                    self.flush_gesture();
                    return "drag".to_owned();
                }
                "none".to_owned()
            }
            "up" | "cancel" => {
                let Some(mut g) = self.gesture.take() else {
                    return "none".to_owned();
                };
                let p = g.ptrs.iter().find(|p| p.0 == id).copied();
                g.ptrs.retain(|p| p.0 != id);
                let mode = if g.pinching {
                    "pinch"
                } else if g.dragging {
                    "drag"
                } else {
                    "tap"
                };
                if !g.ptrs.is_empty() {
                    // One finger of a pinch lifted: the gesture goes on with the other.
                    self.gesture = Some(g);
                    return mode.to_owned();
                }
                if mode == "tap" && phase == "up" && !g.cancelled {
                    if let Some((_, x0, y0, _, _)) = p {
                        self.tap(target, x0, y0);
                    }
                } else if g.acc_dx != 0.0 || g.acc_factor != 1.0 {
                    // Flush what is left once the in-flight input answers.
                    self.gesture = Some(Gesture {
                        target: g.target.clone(),
                        acc_dx: g.acc_dx,
                        acc_factor: g.acc_factor,
                        acc_x: g.acc_x,
                        ..Gesture::default()
                    });
                    self.flush_gesture();
                    if self
                        .gesture
                        .as_ref()
                        .is_some_and(|g| g.acc_dx == 0.0 && g.acc_factor == 1.0)
                    {
                        self.gesture = None;
                    }
                }
                mode.to_owned()
            }
            _ => "none".to_owned(),
        }
    }

    /// A wheel notch on the open chart (`dy` < 0: zoom in) at column `x`.
    /// (R5/M20) Returns whether the wheel was the chart's (zoom); otherwise the page lets
    /// it scroll (the open capsule's column).
    pub fn wheel(&mut self, dy: f64, x: f64) -> bool {
        if !self.chart_open() || !dy.is_finite() || dy == 0.0 {
            return false;
        }
        let f = if dy < 0.0 {
            WHEEL_FACTOR
        } else {
            1.0 / WHEEL_FACTOR
        };
        if self.input_in_flight() {
            let g = self.gesture.get_or_insert_with(|| Gesture {
                target: "scene".to_owned(),
                acc_factor: 1.0,
                ..Gesture::default()
            });
            g.acc_factor *= f;
            g.acc_x = Some(x);
        } else {
            self.send_pinch(f, Some(x));
        }
        true
    }

    /// The last transition's moving elements in page coordinates: the before boxes are
    /// inside the element at (`from_left`, `from_top`), the after boxes inside the one
    /// at (`to_left`, `to_top`). `[{id, from:{left,top,width,height}, to:{…}, src:[x, y,
    /// w, h]}]` — `from`/`to` as CSS lengths, `src` the before box inside the old canvas
    /// (what to copy).
    #[must_use]
    pub fn transition_json(
        &self,
        from_left: f64,
        from_top: f64,
        to_left: f64,
        to_top: f64,
    ) -> Value {
        let k = i32::try_from(self.scale).unwrap_or(1);
        let css = |r: &qp_schema::scene::PixelRect, l: f64, t: f64| {
            let (w, h) = (r.right - r.left, r.bottom - r.top);
            json!({"left": format!("{}px", f64::from(r.left) + l),
                   "top": format!("{}px", f64::from(r.top) + t),
                   "width": format!("{}px", w.max(1)), "height": format!("{}px", h.max(1))})
        };
        Value::Array(
            self.shared
                .iter()
                .filter_map(|s| {
                    let (b, a) = (s.before.as_ref()?, s.after.as_ref()?);
                    Some(json!({"id": s.id, "from": css(b, from_left, from_top),
                        "to": css(a, to_left, to_top),
                        "src": [b.left * k, b.top * k, (b.right - b.left).max(1) * k,
                                (b.bottom - b.top).max(1) * k]}))
                })
                .collect(),
        )
    }

    /// (R4/H-M8910) Progress (0..=1) of the last transition's animation at shell time `t`
    /// (ms, `performance.now()`); the first call after a transition starts the clock.
    pub fn transition_progress(&mut self, t: f64) -> f64 {
        let t0 = *self.trans_t0.get_or_insert(t);
        // (R5/M21) Reduced motion: every opening happens at once.
        if self.mo.reduced {
            return 1.0;
        }
        qp_grammar::transitions::progress_in(
            qp_session::viewer::look_of(&self.look),
            self.look.transition.as_deref(),
            t0,
            t,
        )
    }

    /// (R4/H-M8910) The last transition's frame at progress `p` by the connection's
    /// transition design (`qp_grammar::transitions`): `{done, scene_opacity, elements:
    /// [{id, source: "before"|"after", src:[x,y,w,h], style:{left,top,width,height,
    /// opacity}}]}` — `src` in backing px of the source canvas (device scale), `style` as
    /// CSS values; the old scene at page (`from_left`, `from_top`), the new at (`to_left`,
    /// `to_top`).
    #[must_use]
    pub fn transition_frame(
        &self,
        p: f64,
        from_left: f64,
        from_top: f64,
        to_left: f64,
        to_top: f64,
    ) -> Value {
        let d = qp_grammar::transitions::by_name(self.look.transition.as_deref());
        let p = if p.is_finite() {
            p.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let f = (d.frame)(&self.shared, (from_left, from_top), (to_left, to_top), p);
        let k = i32::try_from(self.scale).unwrap_or(1);
        let els: Vec<Value> = f
            .elements
            .iter()
            .map(|e| {
                let r = e.src;
                json!({"id": e.id,
                    "source": match e.source {
                        qp_grammar::transitions::Source::Before => "before",
                        qp_grammar::transitions::Source::After => "after",
                    },
                    "src": [r.left * k, r.top * k, (r.right - r.left).max(1) * k,
                            (r.bottom - r.top).max(1) * k],
                    "style": {"left": format!("{}px", e.rect[0]), "top": format!("{}px", e.rect[1]),
                              "width": format!("{}px", e.rect[2]), "height": format!("{}px", e.rect[3]),
                              "opacity": format!("{}", e.opacity.clamp(0.0, 1.0))}})
            })
            .collect();
        json!({"done": p >= 1.0, "scene_opacity": format!("{}", f.scene_opacity.clamp(0.0, 1.0)),
               "elements": els})
    }

    /// The last transition's shared elements (the authority's `Opening.shared`).
    #[must_use]
    pub fn shared(&self) -> &[qp_schema::protocol::SharedElement] {
        &self.shared
    }

    /// Where the last transition started (`sentence`: from a face in the feed).
    #[must_use]
    pub fn transition_from(&self) -> &str {
        &self.shared_from
    }
}

pub(super) fn dist(p: &[(i32, f64, f64, f64, f64)]) -> f64 {
    let dx = p[0].3 - p[1].3;
    let dy = p[0].4 - p[1].4;
    (dx * dx + dy * dy).sqrt()
}
