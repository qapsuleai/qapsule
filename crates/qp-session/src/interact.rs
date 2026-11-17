//! Interaction in the core (R4, `docs/PROTOCOL.md` §4.6, §7): pixel gestures (`tap_at`,
//! `drag`, `pinch`) and opening between depths (`open`, `close`).
//!
//! The shell sends raw movement in pixels; the engine resolves it against the
//! connection's view state and the target's scene description (the compiled view at the
//! frame size the surface reported): pixels → bars with the scene's slot width, the
//! point query with the `hit` code ([`crate::hit_scene`]), and the opening rule
//! ([`qp_grammar::depth::tap_opens`]). Nothing here writes the document or the log: the
//! view state (`Conn::look`) is presence (§7).

use qp_grammar::compile::{MAX_DIM, MIN_HEIGHT, MIN_WIDTH};
use qp_grammar::{compile, depth, gesture, CompileOptions};
use qp_schema::protocol::{
    ErrorCode, Gesture, Interaction, OpenDepth, Opening, OutboundLine, Presentation, QueryResult,
    SurfaceKind, SurfaceTarget, Target, Viewport,
};
use qp_schema::scene::SceneDesc;
use qp_schema::{Depth, Series, View};

use crate::error::{invalid, ptr, PResult, E};
use crate::expand::{need_view, view_bar_series, view_bar_series_any};
use crate::present::present;
use crate::provider::{warm, DocProvider};
use crate::reading::{hit_scene, screen_frame, without_layers};
use crate::session::Session;

const GP: &str = "/gesture";

/// Default height of a capsule face in the feed when the surface reports none.
pub const FACE_HEIGHT: u32 = 140;

/// What a pixel gesture lands on, resolved.
enum On {
    /// A view of the document.
    View(String),
    /// A capsule (id, finding) at the depth it has on this connection: the open capsule
    /// at its depth, any other at `sentence` (its face in the feed).
    Capsule(String, String, OpenDepth),
}

/// A capsule's scene at a depth with its price bar series and marked bar (R4/H-M8910).
type SideScene = (SceneDesc, Option<String>, Option<u64>);

/// The fields of a `style` gesture (R4/T5, R4/H-M8910).
struct StyleReq<'a> {
    face: Option<&'a str>,
    alphabet: Option<&'a str>,
    theme: Option<&'a str>,
    feed: Option<&'a str>,
    transition: Option<&'a str>,
    outcome: Option<&'a str>,
    seen: Option<i64>,
    look: Option<&'a str>,
}

/// The frame key of a target (`ViewState::frames`).
fn frame_key(on: &On) -> String {
    match on {
        On::View(v) => format!("view:{v}"),
        On::Capsule(_, _, d) => depth_key(*d).to_owned(),
    }
}

fn depth_key(d: OpenDepth) -> &'static str {
    match d {
        OpenDepth::Sentence => "face",
        OpenDepth::Capsule => "capsule",
        OpenDepth::Chart | OpenDepth::Evidence => "chart",
    }
}

fn depth_name(d: OpenDepth) -> &'static str {
    match d {
        OpenDepth::Sentence => "sentence",
        OpenDepth::Capsule => "capsule",
        OpenDepth::Chart => "chart",
        OpenDepth::Evidence => "evidence",
    }
}

impl Session {
    /// Handles a pixel gesture or open/close (called from `input`).
    pub(crate) fn interact(
        &mut self,
        ci: usize,
        g: &Gesture,
    ) -> PResult<(QueryResult, Vec<OutboundLine>)> {
        let (out, events) = match g {
            Gesture::TapAt {
                target,
                x,
                y,
                width,
                height,
            } => {
                let on = self.resolve_target(ci, target)?;
                self.set_frame(ci, &frame_key(&on), *width, *height)?;
                self.tap_at(ci, &on, *x, *y)?
            }
            Gesture::Drag {
                target,
                dx,
                width,
                height,
            } => {
                let on = self.resolve_target(ci, target)?;
                self.set_frame(ci, &frame_key(&on), *width, *height)?;
                self.move_chart(ci, &on, Move::Drag(*dx))?
            }
            Gesture::Pinch {
                target,
                factor,
                x,
                width,
                height,
            } => {
                let on = self.resolve_target(ci, target)?;
                self.set_frame(ci, &frame_key(&on), *width, *height)?;
                self.move_chart(ci, &on, Move::Pinch(*factor, *x))?
            }
            Gesture::Open {
                capsule,
                to,
                width,
                height,
            } => {
                let (cap, finding) = self.find_capsule(capsule, &ptr(GP, "capsule"))?;
                let from = self.depth_of(ci, &cap);
                let to = match to {
                    Some(t) => *t,
                    None => depth::deeper(from).ok_or_else(|| {
                        E::new(
                            ErrorCode::Invalid,
                            format!("capsule `{cap}` is already open to its evidence"),
                        )
                        .at(GP)
                        .hint("Close it ({\"kind\":\"close\"}) or name another capsule.")
                        .build()
                    })?,
                };
                if to == OpenDepth::Sentence {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        "`open` goes to a depth below the sentence",
                    )
                    .at(ptr(GP, "to"))
                    .expected(["capsule", "chart", "evidence"])
                    .hint("Use {\"kind\":\"close\",\"to\":\"sentence\"} to go back to the feed.")
                    .build());
                }
                self.set_frame(ci, depth_key(to), *width, *height)?;
                let o = Out {
                    opening: Some(self.transition(ci, &cap, &finding, from, to)?),
                    ..Out::default()
                };
                (o, Vec::new())
            }
            Gesture::Close { to, width, height } => {
                let look = &self.conns[ci].look;
                let (Some(cap), Some(finding)) = (look.capsule.clone(), look.finding.clone())
                else {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        "no capsule is open on this connection",
                    )
                    .at(GP)
                    .hint("Open one first: {\"kind\":\"open\",\"capsule\":…}.")
                    .build());
                };
                let from = look.depth;
                let to = match to {
                    Some(t) if *t < from => *t,
                    Some(t) => {
                        return Err(E::new(
                            ErrorCode::Invalid,
                            format!(
                                "`close` goes back: the capsule is open to `{}`",
                                depth_name(from)
                            ),
                        )
                        .at(ptr(GP, "to"))
                        .expected(
                            [OpenDepth::Sentence, OpenDepth::Capsule, OpenDepth::Chart]
                                .into_iter()
                                .filter(|d| d < &from)
                                .map(depth_name),
                        )
                        .hint(format!(
                            "To go deeper use `open` (not to `{}`).",
                            depth_name(*t)
                        ))
                        .build())
                    }
                    None => depth::shallower(from).unwrap_or(OpenDepth::Sentence),
                };
                self.set_frame(ci, depth_key(to), *width, *height)?;
                // The capsule may have gone (retracted, no longer visible): closing to the
                // feed still works; a deeper depth needs it.
                if to != OpenDepth::Sentence {
                    self.find_capsule(&cap, GP)?;
                }
                let o = Out {
                    opening: Some(self.transition(ci, &cap, &finding, from, to)?),
                    ..Out::default()
                };
                (o, Vec::new())
            }
            Gesture::Style {
                face,
                alphabet,
                theme,
                feed,
                transition,
                outcome,
                seen,
                look,
            } => {
                self.set_style(
                    ci,
                    &StyleReq {
                        face: face.as_deref(),
                        alphabet: alphabet.as_deref(),
                        theme: theme.as_deref(),
                        feed: feed.as_deref(),
                        transition: transition.as_deref(),
                        outcome: outcome.as_deref(),
                        seen: *seen,
                        look: look.as_deref(),
                    },
                )?;
                (Out::default(), Vec::new())
            }
            Gesture::Look { .. }
            | Gesture::Pan { .. }
            | Gesture::Zoom { .. }
            | Gesture::Tap { .. } => {
                unreachable!("bar-space gestures are handled by `input`")
            }
        };
        self.input_counter += 1;
        let at = self.clock;
        let c = &mut self.conns[ci];
        c.last_input = self.input_counter;
        c.last_at = Some(at);
        let interaction = Interaction {
            hit: out.hit,
            at: out.at,
            viewport: out.viewport,
            opening: out.opening.map(|o| *o),
            state: c.look.clone(),
        };
        Ok((
            QueryResult::Interaction {
                interaction: Box::new(interaction),
            },
            events,
        ))
    }

    /// The target of a pixel gesture: the named view or capsule, else the open capsule,
    /// else the connection's current view.
    fn resolve_target(&self, ci: usize, t: &SurfaceTarget) -> PResult<On> {
        let tp = ptr(GP, "target");
        match (&t.view, &t.capsule) {
            (Some(_), Some(_)) => Err(E::new(
                ErrorCode::Invalid,
                "a target is a view or a capsule, not both",
            )
            .at(tp)
            .expected(["{\"view\":…}", "{\"capsule\":…}", "{} (the current scene)"])
            .build()),
            (Some(v), None) => Ok(On::View(
                need_view(rd!(self), v, &ptr(&tp, "view"))?.id.clone(),
            )),
            (None, Some(c)) => {
                let (cap, f) = self.find_capsule(c, &ptr(&tp, "capsule"))?;
                let d = self.depth_of(ci, &cap);
                Ok(On::Capsule(cap, f, d))
            }
            (None, None) => {
                let look = &self.conns[ci].look;
                if let (Some(c), Some(_)) = (&look.capsule, &look.finding) {
                    let (cap, f) = self.find_capsule(c, &tp)?;
                    return Ok(On::Capsule(cap, f, look.depth));
                }
                let vp = self.conn_viewport(&self.conns[ci]).ok_or_else(|| {
                    E::new(
                        ErrorCode::NotFound,
                        "nothing is shown: no capsule is open and the document has no view",
                    )
                    .at(tp)
                    .hint("Name a target: {\"view\":…} or {\"capsule\":…}.")
                    .build()
                })?;
                Ok(On::View(vp.view))
            }
        }
    }

    /// A capsule visible to the reader, by capsule or finding id: (capsule, finding).
    fn find_capsule(&self, id: &str, path: &str) -> PResult<(String, String)> {
        let doc = rd!(self);
        doc.capsules
            .iter()
            .find(|c| c.id == id || c.finding == id)
            .filter(|c| doc.findings.iter().any(|f| f.id == c.finding))
            .map(|c| (c.id.clone(), c.finding.clone()))
            .ok_or_else(|| {
                E::new(ErrorCode::NotFound, format!("no capsule `{id}`"))
                    .at(path)
                    .dym(id, doc.capsules.iter().map(|c| c.id.as_str()))
                    .build()
            })
    }

    /// The depth capsule `cap` has on connection `ci` (`sentence` unless it is open).
    fn depth_of(&self, ci: usize, cap: &str) -> OpenDepth {
        let look = &self.conns[ci].look;
        if look.capsule.as_deref() == Some(cap) {
            look.depth
        } else {
            OpenDepth::Sentence
        }
    }

    /// (R4/T5) The `style` gesture: the connection's face design, glyph alphabet and
    /// theme; (R4/H-M8910) feed layout, depth transition, outcome mark style and last look
    /// (view state; nothing is written). Validated before anything changes.
    fn set_style(&mut self, ci: usize, r: &StyleReq<'_>) -> PResult<()> {
        let unknown = |field: &str, v: &str, known: Vec<&str>| {
            E::new(ErrorCode::Invalid, format!("unknown {field} `{v}`"))
                .at(ptr(GP, field))
                .expected(known)
                .build()
        };
        if let Some(f) = r.face {
            if !qp_grammar::faces::is_face(f) {
                return Err(unknown("face", f, qp_grammar::faces::names()));
            }
        }
        if let Some(a) = r.alphabet {
            if qp_grammar::glyph::alphabet_named(Some(a)).is_none() {
                return Err(
                    E::new(ErrorCode::Invalid, format!("unknown alphabet `{a}`"))
                        .at(ptr(GP, "alphabet"))
                        .expected(qp_grammar::glyph::ALPHABETS)
                        .build(),
                );
            }
        }
        if let Some(t) = r.theme {
            if t != "view" && !qp_grammar::theme::THEMES.contains(&t) {
                return Err(E::new(ErrorCode::Invalid, format!("unknown theme `{t}`"))
                    .at(ptr(GP, "theme"))
                    .expected(qp_grammar::theme::THEMES.into_iter().chain(["view"]))
                    .build());
            }
        }
        if let Some(f) = r.feed {
            if !qp_grammar::feeds::is_feed(f) {
                return Err(unknown("feed", f, qp_grammar::feeds::names()));
            }
        }
        if let Some(t) = r.transition {
            if !qp_grammar::transitions::is_transition(t) {
                return Err(unknown("transition", t, qp_grammar::transitions::names()));
            }
        }
        if let Some(o) = r.outcome {
            if qp_grammar::glyph::outcome::by_name(o).is_err() {
                return Err(unknown("outcome", o, qp_grammar::glyph::outcome::names()));
            }
        }
        if let Some(l) = r.look {
            if !qp_grammar::look::is_look(l) {
                return Err(unknown("look", l, qp_grammar::look::names()));
            }
        }
        let look = &mut self.conns[ci].look;
        if let Some(l) = r.look {
            look.look = (l != qp_grammar::look::DEFAULT_LOOK).then(|| l.to_owned());
        }
        if let Some(f) = r.face {
            let c = qp_grammar::faces::canonical(f).unwrap_or(qp_grammar::faces::DEFAULT_FACE);
            look.face = (c != qp_grammar::faces::DEFAULT_FACE).then(|| c.to_owned());
        }
        if let Some(a) = r.alphabet {
            look.alphabet = (a != qp_grammar::glyph::ALPHABETS[0]).then(|| a.to_owned());
        }
        if let Some(t) = r.theme {
            look.theme = (t != "view").then(|| t.to_owned());
        }
        if let Some(f) = r.feed {
            look.feed = (f != qp_grammar::feeds::DEFAULT_FEED).then(|| f.to_owned());
        }
        if let Some(t) = r.transition {
            look.transition =
                (t != qp_grammar::transitions::DEFAULT_TRANSITION).then(|| t.to_owned());
        }
        if let Some(o) = r.outcome {
            look.outcome = (o != qp_grammar::glyph::outcome::DEFAULT_OUTCOME).then(|| o.to_owned());
        }
        if let Some(t) = r.seen {
            look.seen = (t != 0).then_some(t);
        }
        Ok(())
    }

    /// Records a frame size the surface reported for `key`.
    fn set_frame(
        &mut self,
        ci: usize,
        key: &str,
        width: Option<u32>,
        height: Option<u32>,
    ) -> PResult<()> {
        if width.is_none() && height.is_none() {
            return Ok(());
        }
        let (w0, h0) = self.frame(ci, key);
        let (w, h) = (width.unwrap_or(w0), height.unwrap_or(h0));
        for (field, v, min) in [("width", w, MIN_WIDTH), ("height", h, MIN_HEIGHT)] {
            if v < min || v > MAX_DIM {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("frame {field} {v} is outside {min}..={MAX_DIM}"),
                )
                .at(ptr(GP, field))
                .expected([format!("{min}..={MAX_DIM}")])
                .build());
            }
        }
        self.conns[ci].look.frames.insert(key.to_owned(), [w, h]);
        Ok(())
    }

    /// The frame of `key`: as reported, else the surface's default (`screen`'s frame;
    /// a face is that wide and [`FACE_HEIGHT`] high).
    fn frame(&self, ci: usize, key: &str) -> (u32, u32) {
        let c = &self.conns[ci];
        if let Some([w, h]) = c.look.frames.get(key) {
            return (*w, *h);
        }
        let (w, h) = screen_frame(c.surface.kind);
        if key == "face" {
            (w, FACE_HEIGHT.max(MIN_HEIGHT))
        } else {
            (w, h)
        }
    }

    /// The presenter's output for `finding` at open depth `d` for connection `ci`: the
    /// `present` query's rules (the connection's profile at that depth; a finding whose
    /// data the reader may not read gets no spark and no view, §18.3).
    fn present_at(&mut self, ci: usize, finding: &str, d: OpenDepth) -> PResult<Presentation> {
        let f = crate::expand::need_finding(rd!(self), finding, GP)?.clone();
        let mut profile = self.conns[ci].profile.clone();
        profile.depth = depth::presenter_depth(d);
        let kind = self.conns[ci].surface.kind;
        let asker = self.conns[ci].principal.clone();
        let facts_blocked = f.facts.iter().any(|id| {
            self.doc
                .facts
                .iter()
                .find(|x| &x.id == id)
                .is_some_and(|x| {
                    self.plane
                        .gate(&self.doc, Some(&asker), &x.series)
                        .is_some()
                })
        });
        let view_blocked = self
            .doc
            .capsules
            .iter()
            .find(|c| c.finding == f.id)
            .and_then(|c| c.view.as_ref())
            .and_then(|v| self.doc.views.iter().find(|x| &x.id == v))
            .is_some_and(|v| {
                v.layers
                    .iter()
                    .any(|l| self.plane.gate_layer(&self.doc, Some(&asker), l).is_some())
            });
        let mut pr = present(
            rd!(self),
            &mut self.cache,
            &f,
            kind,
            &profile,
            &self.headlines,
        );
        if facts_blocked || view_blocked {
            pr.spark = None;
            pr.view = None;
            pr.capsule_view = None;
        }
        Ok(pr)
    }

    /// The view drawn for a capsule at open depth `d` (`sentence`: its face, the card),
    /// if the reader gets one.
    fn depth_view(&mut self, ci: usize, finding: &str, d: OpenDepth) -> PResult<Option<View>> {
        let face = if d == OpenDepth::Sentence {
            OpenDepth::Capsule
        } else {
            d
        };
        let pr = self.present_at(ci, finding, face)?;
        // (R4/T5) A face design other than the default card is the connection's choice
        // (view state); built by the same grammar rule the shell's replica uses.
        // (R5/T3) So is a look other than `r4`: the face and the card are rebuilt with
        // its sizes (the replica's `viewer::face_scene` / `open_scene` do the same).
        let styled = self.conns[ci].look.face.is_some() || self.conns[ci].look.look.is_some();
        let look = crate::viewer::look_of(&self.conns[ci].look);
        if d == OpenDepth::Capsule && self.conns[ci].look.look.is_some() {
            let Some(id) = pr.capsule_view.as_ref().map(|v| v.id.clone()) else {
                return Ok(None);
            };
            let max_points = self.conns[ci].profile.max_points;
            let lang = self.lang_of(ci);
            warm(rd!(self), &mut self.cache);
            let p = DocProvider::new(rd!(self), &self.cache)
                .with_headlines(&self.headlines)
                .with_parts(self, lang);
            return Ok(depth::capsule_view_in(look, finding, max_points, &p, &id).ok());
        }
        if d == OpenDepth::Sentence && styled {
            if pr.capsule_view.is_none() {
                return Ok(None);
            }
            let name = self.conns[ci].look.face.clone();
            let max_points = self.conns[ci].profile.max_points;
            let lang = self.lang_of(ci);
            warm(rd!(self), &mut self.cache);
            let p = DocProvider::new(rd!(self), &self.cache)
                .with_headlines(&self.headlines)
                .with_parts(self, lang);
            return Ok(depth::face_view_in(
                look,
                name.as_deref(),
                finding,
                max_points,
                &p,
                &format!("capsule-{finding}"),
            )
            .ok());
        }
        Ok(match depth::presenter_depth(face) {
            Depth::Capsule => pr.capsule_view.map(|v| *v),
            Depth::Chart | Depth::Workspace => pr.view,
            Depth::Sentence => None,
        })
    }

    /// Compiles `view` for reader `ci` at `frame` over `range` (the `hit` query's
    /// compile: the reader's unreadable layers are not drawn; a view whose bar series is
    /// unreadable is an error), with the bar times of its first bar series.
    ///
    /// (R4/H-FP) `face`: the face design when `view` is a capsule's face (open depth
    /// `sentence`), so its headline template applies (`CompileOptions::face`).
    fn reader_scene(
        &mut self,
        ci: usize,
        view: &View,
        frame: (u32, u32),
        range: Option<(u64, u64)>,
        face: Option<String>,
    ) -> PResult<(SceneDesc, Vec<i64>)> {
        warm(rd!(self), &mut self.cache);
        let asker = self.conns[ci].principal.clone();
        let blocked = self.blocked_layers(view, &asker);
        let filtered;
        let view = if blocked.is_empty() {
            view
        } else {
            if let Some(bs) = view_bar_series_any(&self.doc, view) {
                if let Some(b) = self.plane.gate(&self.doc, Some(&asker), &bs.id) {
                    return Err(b.error(&bs.id, GP));
                }
            }
            filtered = without_layers(view, &blocked);
            &filtered
        };
        let mut opts = CompileOptions::new(frame.0, frame.1);
        opts.range = range;
        // (R4/T5) The connection's style (view state).
        opts.theme.clone_from(&self.conns[ci].look.theme);
        opts.alphabet.clone_from(&self.conns[ci].look.alphabet);
        opts.outcome.clone_from(&self.conns[ci].look.outcome);
        opts.look.clone_from(&self.conns[ci].look.look);
        opts.face = face;
        let lang = self.lang_of(ci);
        let scene = compile(
            view,
            &DocProvider::new(rd!(self), &self.cache)
                .with_headlines(&self.headlines)
                .with_parts(self, lang),
            &opts,
        )
        .map_err(|e| E::new(ErrorCode::Invalid, e.to_string()).at(GP).build())?;
        let times = view_bar_series(rd!(self), view)
            .and_then(Series::bars)
            .map(|b| b.iter().map(|b| b.t).collect())
            .unwrap_or_default();
        Ok((scene, times))
    }

    /// The scene of a capsule at depth `d` on connection `ci` (at its frame, at chart
    /// depth over the view state's range), with its view; `None` without a view.
    fn capsule_scene(
        &mut self,
        ci: usize,
        finding: &str,
        d: OpenDepth,
    ) -> PResult<Option<(View, SceneDesc, Vec<i64>)>> {
        let Some(view) = self.depth_view(ci, finding, d)? else {
            return Ok(None);
        };
        let range = if d >= OpenDepth::Chart {
            self.chart_range(ci, &view)
        } else {
            None
        };
        let frame = self.frame(ci, depth_key(d));
        let face = if d == OpenDepth::Sentence {
            self.conns[ci].look.face.clone()
        } else {
            None
        };
        let (scene, times) = self.reader_scene(ci, &view, frame, range, face)?;
        Ok(Some((view, scene, times)))
    }

    /// The visible range of a capsule's chart: the view state's (if it is that view's),
    /// else the surface's initial range (§4.6), clamped to the current slots.
    fn chart_range(&self, ci: usize, view: &View) -> Option<(u64, u64)> {
        let c = &self.conns[ci];
        let saved = c
            .look
            .range
            .as_ref()
            .filter(|r| r.view == view.id)
            .and_then(|r| {
                let last = self.last_slot(view)?;
                let b = r.to_bar.min(last);
                Some((r.from_bar.min(b), b))
            });
        saved.or_else(|| self.initial_range(view, c.surface.kind))
    }

    /// A capsule's scene at depth `d` with what its transition anchors need
    /// (`docs/GRAMMAR.md` §18.6): the price bar series and the marked (first fact's) bar;
    /// `None` without a scene.
    fn side_scene(&mut self, ci: usize, finding: &str, d: OpenDepth) -> PResult<Option<SideScene>> {
        let Some((view, scene, _)) = self.capsule_scene(ci, finding, d)? else {
            return Ok(None);
        };
        let doc = rd!(self);
        let fact_bar = doc
            .findings
            .iter()
            .find(|f| f.id == finding)
            .and_then(|f| f.facts.first())
            .and_then(|id| doc.facts.iter().find(|x| &x.id == id))
            .map(|f| f.bar);
        let price = view_bar_series(doc, &view).map(|s| s.id.clone());
        Ok(Some((scene, price, fact_bar)))
    }

    /// Moves capsule `cap` from depth `from` to `to` on connection `ci`: the view state,
    /// what to show at `to`, the shared elements and (at evidence) the evidence access.
    fn transition(
        &mut self,
        ci: usize,
        cap: &str,
        finding: &str,
        from: OpenDepth,
        to: OpenDepth,
    ) -> PResult<Box<Opening>> {
        let cap_at = match self.conns[ci].surface.max_depth {
            Some(Depth::Sentence) => OpenDepth::Sentence,
            Some(Depth::Capsule) => OpenDepth::Capsule,
            _ => OpenDepth::Evidence,
        };
        if to > cap_at {
            return Err(E::new(
                ErrorCode::Invalid,
                format!(
                    "this surface asked for at most `{}` depth (`surface.max_depth`)",
                    depth_name(cap_at)
                ),
            )
            .at(ptr(GP, "to"))
            .expected(
                [OpenDepth::Capsule, OpenDepth::Chart, OpenDepth::Evidence]
                    .into_iter()
                    .filter(|d| *d <= cap_at)
                    .map(depth_name),
            )
            .build());
        }
        // The scene before, as it was shown (the face when nothing was open).
        let before = self.side_scene(ci, finding, from)?;
        let presentation = self.present_at(ci, finding, to)?;
        // Update the view state.
        let view_id = match to {
            OpenDepth::Sentence => None,
            OpenDepth::Capsule => presentation.capsule_view.as_ref().map(|v| v.id.clone()),
            OpenDepth::Chart | OpenDepth::Evidence => {
                presentation.view.as_ref().map(|v| v.id.clone())
            }
        };
        let range = match (&presentation.view, to >= OpenDepth::Chart) {
            (Some(v), true) => self.chart_range(ci, v).map(|(a, b)| Viewport {
                view: v.id.clone(),
                from_bar: a,
                to_bar: b,
            }),
            _ => None,
        };
        {
            let look = &mut self.conns[ci].look;
            if to == OpenDepth::Sentence {
                look.capsule = None;
                look.finding = None;
            } else {
                look.capsule = Some(cap.to_owned());
                look.finding = Some(finding.to_owned());
            }
            look.depth = to;
            look.view = view_id;
            look.range = range;
        }
        let after = self.side_scene(ci, finding, to)?;
        // (R4/H-M8910) The connection's transition design picks the shared elements
        // (`qp_grammar::transitions`; `r3` = the anchors both scenes have).
        let shared = {
            fn side(d: OpenDepth, s: Option<&SideScene>) -> qp_grammar::transitions::Side<'_> {
                qp_grammar::transitions::Side {
                    depth: d,
                    scene: s.map(|x| &x.0),
                    price_series: s.and_then(|x| x.1.as_deref()),
                    marked_bar: s.and_then(|x| x.2),
                }
            }
            let design =
                qp_grammar::transitions::by_name(self.conns[ci].look.transition.as_deref());
            (design.shared)(&side(from, before.as_ref()), &side(to, after.as_ref()))
        };
        let frame = after.is_some().then(|| {
            let (w, h) = self.frame(ci, depth_key(to));
            [w, h]
        });
        let evidence = if to == OpenDepth::Evidence {
            match self.capsule_query(ci, cap, GP) {
                Ok(QueryResult::CapsuleDetail { detail }) => detail.evidence,
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        };
        Ok(Box::new(Opening {
            capsule: cap.to_owned(),
            finding: finding.to_owned(),
            from,
            to,
            presentation,
            frame,
            shared,
            evidence,
        }))
    }

    /// `tap_at`: the point query on the target's scene, then the opening rule (a capsule
    /// opens one depth further) or the bar's values (a chart).
    fn tap_at(&mut self, ci: usize, on: &On, x: f64, y: f64) -> PResult<(Out, Vec<OutboundLine>)> {
        if !(x.is_finite() && y.is_finite()) {
            return Err(invalid("x and y must be finite numbers", GP));
        }
        match on {
            On::View(vid) => {
                let view = need_view(rd!(self), vid, GP)?.clone();
                let frame = self.frame(ci, &frame_key(on));
                let range = self.hit_range(&self.conns[ci], vid);
                let (scene, times) = self.reader_scene(ci, &view, frame, range, None)?;
                let hit = hit_scene(&scene, x, y, &times);
                let mut out = Out::default();
                // A glyph (or any layer drawing a finding) presented by a capsule: the tap
                // opens that capsule.
                let opens = hit.item.as_ref().and_then(|i| match &i.data {
                    qp_schema::scene::DataSource::Finding { finding } => {
                        self.find_capsule(finding, GP).ok()
                    }
                    _ => None,
                });
                if let Some((cap, f)) = opens {
                    let from = self.depth_of(ci, &cap);
                    if let Some(to) = depth::deeper(from) {
                        out.opening = Some(self.transition(ci, &cap, &f, from, to)?);
                    }
                } else if let (Some(panel), Some(bar)) = (hit.panel, hit.bar) {
                    if (bar as usize) < times.len() {
                        if let Ok((QueryResult::At { at }, _)) = self.input(
                            ci,
                            &Gesture::Tap {
                                view: vid.clone(),
                                panel,
                                bar,
                            },
                        ) {
                            out.at = Some(at);
                        }
                    }
                }
                out.hit = Some(hit);
                Ok((out, Vec::new()))
            }
            On::Capsule(cap, finding, d) => {
                let (cap, finding, d) = (cap.clone(), finding.clone(), *d);
                let mut out = Out::default();
                let scene = self.capsule_scene(ci, &finding, d)?;
                let hit = scene
                    .as_ref()
                    .map(|(_, s, times)| hit_scene(s, x, y, times));
                if let Some(to) = depth::tap_opens(d, hit.as_ref().and_then(|h| h.item.as_ref())) {
                    out.opening = Some(self.transition(ci, &cap, &finding, d, to)?);
                } else if let (Some((view, _, times)), Some(h)) = (&scene, &hit) {
                    if let (Some(panel), Some(bar)) = (h.panel, h.bar) {
                        let i = usize::try_from(bar).unwrap_or(usize::MAX);
                        if i < times.len() {
                            if let Some(bs) = view_bar_series(rd!(self), view).map(|s| s.id.clone())
                            {
                                self.gate_read(ci, &bs, GP)?;
                                out.at = Some(self.at_bar(&bs, i));
                                let principal = self.conns[ci].principal.clone();
                                self.focus.insert(
                                    principal,
                                    Target {
                                        layer: None,
                                        view: Some(view.id.clone()),
                                        panel: Some(panel),
                                        series: None,
                                        output: None,
                                        item: None,
                                        from_bar: Some(bar),
                                        to_bar: Some(bar),
                                    },
                                );
                            }
                        }
                    }
                }
                out.hit = hit;
                Ok((out, Vec::new()))
            }
        }
    }

    /// `drag` and `pinch`: a document view goes through `pan` / `zoom` (its connection
    /// viewport); an open capsule's chart moves the view state's range by the same rules.
    fn move_chart(&mut self, ci: usize, on: &On, m: Move) -> PResult<(Out, Vec<OutboundLine>)> {
        if let Move::Pinch(f, _) = m {
            if !(f.is_finite() && f > 0.0) {
                return Err(invalid(
                    "factor must be a positive number",
                    ptr(GP, "factor"),
                ));
            }
        }
        match on {
            On::View(vid) => {
                let view = need_view(rd!(self), vid, GP)?.clone();
                let frame = self.frame(ci, &frame_key(on));
                let range = self.hit_range(&self.conns[ci], vid);
                let (scene, _) = self.reader_scene(ci, &view, frame, range, None)?;
                let g = match m {
                    Move::Drag(dx) => Gesture::Pan {
                        view: vid.clone(),
                        bars: gesture::drag_bars(&scene.x, dx),
                    },
                    Move::Pinch(factor, x) => Gesture::Zoom {
                        view: vid.clone(),
                        factor,
                        anchor_bar: x.map(|px| gesture::pinch_anchor(&scene.x, px)),
                    },
                };
                let (r, events) = self.input(ci, &g)?;
                let QueryResult::Viewport { viewport } = r else {
                    unreachable!("pan and zoom answer a viewport")
                };
                Ok((
                    Out {
                        viewport: Some(viewport),
                        ..Out::default()
                    },
                    events,
                ))
            }
            On::Capsule(cap, finding, d) => {
                if *d < OpenDepth::Chart {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!("capsule `{cap}` is not open to its chart; only a chart pans and zooms"),
                    )
                    .at(ptr(GP, "target"))
                    .hint(format!("Open it first: {{\"kind\":\"open\",\"capsule\":\"{cap}\",\"to\":\"chart\"}}."))
                    .build());
                }
                let finding = finding.clone();
                let Some((view, scene, _)) = self.capsule_scene(ci, &finding, *d)? else {
                    return Err(E::new(
                        ErrorCode::NoData,
                        "the capsule's chart is not available to this reader",
                    )
                    .at(ptr(GP, "target"))
                    .build());
                };
                let last_slot = self.last_slot(&view).unwrap_or(0);
                let (from, to) = self.chart_range(ci, &view).unwrap_or((0, last_slot));
                let (nf, nt) = match m {
                    Move::Drag(dx) => {
                        gesture::pan_range(from, to, gesture::drag_bars(&scene.x, dx), last_slot)
                    }
                    Move::Pinch(factor, x) => {
                        let anchor = x.map_or(to, |px| gesture::pinch_anchor(&scene.x, px));
                        gesture::zoom_range(from, to, factor, anchor.min(last_slot), last_slot + 1)
                    }
                };
                let vp = Viewport {
                    view: view.id.clone(),
                    from_bar: nf,
                    to_bar: nt,
                };
                self.conns[ci].look.range = Some(vp.clone());
                let principal = self.conns[ci].principal.clone();
                let events = self.viewport_events(&principal, &vp);
                Ok((
                    Out {
                        viewport: Some(vp),
                        ..Out::default()
                    },
                    events,
                ))
            }
        }
    }

    /// The surface kind of connection `ci` (for tests and benchmarks).
    #[doc(hidden)]
    pub fn surface_kind(&self, conn: &str) -> Option<SurfaceKind> {
        self.conns
            .iter()
            .find(|c| c.id == conn)
            .map(|c| c.surface.kind)
    }
}

#[derive(Clone, Copy)]
enum Move {
    Drag(f64),
    Pinch(f64, Option<f64>),
}

#[derive(Default)]
struct Out {
    hit: Option<qp_schema::protocol::HitInfo>,
    at: Option<qp_schema::protocol::AtBar>,
    viewport: Option<Viewport>,
    opening: Option<Box<Opening>>,
}
