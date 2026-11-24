//! Interaction in the core (R4, `docs/PROTOCOL.md` §4.6, §7): pixel gestures (`tap_at`,
//! `drag`, `pinch`) and opening between depths (`open`, `close`).
//!
//! The shell sends raw movement in pixels; the engine resolves it against the
//! connection's view state and the target's scene description (the compiled view at the
//! frame size the surface reported): pixels → bars with the scene's slot width, the
//! point query with the `hit` code ([`crate::reading::hit_scene`]), and the opening rule
//! ([`qp_grammar::depth::tap_opens`]). Nothing here writes the document or the log: the
//! view state (`Conn::look`) is presence (§7).

mod scenes;
mod style;

use qp_designs::{depth::story_design, stories::StoryCompile};
use qp_grammar::compile::{MAX_DIM, MIN_HEIGHT, MIN_WIDTH};
use qp_grammar::{compile, depth, gesture};
use qp_schema::protocol::{
    ErrorCode, Gesture, Interaction, OpenDepth, Opening, OutboundLine, Presentation, QueryResult,
    SurfaceTarget, Target, Viewport,
};
use qp_schema::scene::SceneDesc;
use qp_schema::{Depth, Series, View};

use crate::look_of;
use crate::rd::Rd;
use crate::reading::{hit_scene, screen_frame};
use qp_ops::bounds::without_layers;
use qp_ops::expand::{need_view, view_bar_series, view_bar_series_any};
use qp_ops::present::present;
use qp_ops::provider::{warm, DocProvider};
use qp_plane::error::{invalid, ptr, PResult, E};

const GP: &str = "/gesture";

/// (R5/M18) Fewest bars a story cluster's tap shows.
const STORY_ZOOM_MIN: u64 = qp_schema::params::system::GESTURE_STORY_ZOOM_MIN_BARS_DEFAULT;

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
    voice: Option<&'a str>,
    story: Option<&'a str>,
    hide_closed: Option<bool>,
    open: Option<&'a str>,
    clock: Option<bool>,
}

/// (R5/S) A story design's whole-scene compile, the reader's story rows and the focus.
type StoryCompiled<'a> = (StoryCompile, &'a [qp_schema::protocol::StoryRow], &'a str);

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

impl Rd<'_> {
    #[allow(clippy::too_many_lines)]
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
                let look = &self.looks[ci];
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
                voice,
                story,
                hide_closed,
                open,
                clock,
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
                        voice: voice.as_deref(),
                        story: story.as_deref(),
                        hide_closed: *hide_closed,
                        open: open.as_deref(),
                        clock: *clock,
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
        self.st.input_counter += 1;
        let at = self.clock;
        let c = &mut self.conns[ci];
        c.last_input = self.st.input_counter;
        c.last_at = Some(at);
        let interaction = Interaction {
            hit: out.hit,
            at: out.at,
            viewport: out.viewport,
            opening: out.opening.map(|o| *o),
            state: self.looks[ci].clone(),
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
                let look = &self.looks[ci];
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
        let look = &self.looks[ci];
        if look.capsule.as_deref() == Some(cap) {
            look.depth
        } else {
            OpenDepth::Sentence
        }
    }

    /// The presenter's output for `finding` at open depth `d` for connection `ci`: the
    /// `present` query's rules (the connection's profile at that depth; a finding whose
    /// data the reader may not read gets no spark and no view, §18.3).
    fn present_at(&mut self, ci: usize, finding: &str, d: OpenDepth) -> PResult<Presentation> {
        let f = qp_ops::expand::need_finding(rd!(self), finding, GP)?.clone();
        let mut profile = self.conns[ci].profile.clone();
        profile.depth = depth::presenter_depth(d);
        let kind = self.conns[ci].surface.kind;
        let asker = self.conns[ci].principal.clone();
        let facts_blocked = f.facts.iter().any(|id| {
            self.doc
                .facts
                .iter()
                .find(|x| &x.id == id)
                .is_some_and(|x| self.plane.gate(self.doc, Some(&asker), &x.series).is_some())
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
                    .any(|l| self.plane.gate_layer(self.doc, Some(&asker), l).is_some())
            });
        let mut pr = present(rd!(self), self.cache, &f, kind, &profile, self.headlines);
        if facts_blocked || view_blocked {
            pr.spark = None;
            pr.view = None;
            pr.capsule_view = None;
        }
        Ok(pr)
    }

    #[allow(clippy::too_many_lines)]
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
                let (scene, times) = self.reader_scene(ci, &view, frame, range, None, None)?;
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
                // (R5/m17-int, K49) A tap on a glossary term opens its explanation (the
                // hit's `term`; the shell shows it), not the capsule.
                let on_term = hit.as_ref().is_some_and(|h| h.term.is_some());
                if on_term {
                    out.hit = hit;
                    return Ok((out, Vec::new()));
                }
                // (R5/M18) The story chart's own taps come first (focus, cluster, K54).
                let story = if d >= OpenDepth::Chart {
                    story_design(self.looks[ci].story.as_deref()).and_then(|sd| {
                        (sd.tap)(
                            hit.as_ref().and_then(|h| h.item.as_ref()),
                            Some(finding.as_str()),
                        )
                    })
                } else {
                    None
                };
                if let Some(t) = story {
                    let view = scene.as_ref().map(|(v, _, _)| v.clone());
                    let events = self.story_tap(ci, t, view.as_ref(), &mut out)?;
                    out.hit = hit;
                    return Ok((out, events));
                }
                // (R5/M20) The open capsule design's tap rule (the reference: `tap_opens`).
                let rule = qp_designs::open::tap(
                    self.looks[ci].open.as_deref(),
                    d,
                    scene.as_ref().map(|(v, _, _)| v),
                    hit.as_ref(),
                    self.looks[ci].ask.as_deref(),
                );
                if let qp_designs::open::OpenTap::Ask(a) = rule {
                    self.looks[ci].ask = a;
                    out.hit = hit;
                    return Ok((out, Vec::new()));
                }
                if let qp_designs::open::OpenTap::Open(to) = rule {
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
                                self.st.focus.insert(
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

    /// (R5/M18) A story chart tap: the focus moves to another story (it becomes the open
    /// capsule, at chart depth, over the same visible range); a cluster shows its bars; the
    /// checkbox shows or hides the closed stories (K54).
    fn story_tap(
        &mut self,
        ci: usize,
        t: qp_designs::stories::StoryTap,
        view: Option<&View>,
        out: &mut Out,
    ) -> PResult<Vec<OutboundLine>> {
        use qp_designs::stories::StoryTap;
        match t {
            StoryTap::ToggleClosed => {
                let l = &mut self.looks[ci];
                l.hide_closed = !l.hide_closed;
                Ok(Vec::new())
            }
            StoryTap::Focus(f) => {
                let (cap, f) = self.find_capsule(&f, GP)?;
                let from = self.looks[ci].depth;
                let keep = self.looks[ci].range.clone();
                let pr = self.present_at(ci, &f, OpenDepth::Chart)?;
                if let (Some(r), Some(v)) = (keep, pr.view.as_ref()) {
                    self.looks[ci].range = Some(Viewport {
                        view: v.id.clone(),
                        ..r
                    });
                }
                out.opening = Some(self.transition(ci, &cap, &f, from, OpenDepth::Chart)?);
                Ok(Vec::new())
            }
            StoryTap::Zoom(a, b) => {
                let Some(view) = view else {
                    return Ok(Vec::new());
                };
                let last = self.last_slot(view).unwrap_or(0);
                let (a, b) = (a.min(b).min(last), a.max(b).min(last));
                // At least `STORY_ZOOM_MIN` bars, centred on the cluster.
                let (a, b) = if b - a + 1 < STORY_ZOOM_MIN {
                    let c = a + (b - a) / 2;
                    let b = (c.saturating_sub(STORY_ZOOM_MIN / 2) + STORY_ZOOM_MIN - 1).min(last);
                    (b.saturating_sub(STORY_ZOOM_MIN - 1), b)
                } else {
                    (a, b)
                };
                let vp = Viewport {
                    view: view.id.clone(),
                    from_bar: a,
                    to_bar: b,
                };
                self.looks[ci].range = Some(vp.clone());
                let principal = self.conns[ci].principal.clone();
                let events = self.viewport_events(&principal, &vp);
                out.viewport = Some(vp);
                Ok(events)
            }
        }
    }

    #[allow(clippy::too_many_lines)]
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
                let (scene, _) = self.reader_scene(ci, &view, frame, range, None, None)?;
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
                self.looks[ci].range = Some(vp.clone());
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

#[cfg(test)]
mod params_binding {
    // RG1/K3: the constant reads `qp_schema::params` and keeps its old literal.
    #[test]
    fn story_zoom_min_binds_registry() {
        assert_eq!(super::STORY_ZOOM_MIN, 20);
    }
}
