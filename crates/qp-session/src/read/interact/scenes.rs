//! Scenes at a depth: depth views, reader, capsule and side scenes, story extension and
//! transitions.

use super::*;

impl Session {
    #[allow(clippy::too_many_lines)]
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
        // (R5/m17-int) So is a voice other than `r4`: the face and the card speak from it.
        let voice = self.conns[ci].look.voice.clone();
        let styled = self.conns[ci].look.face.is_some()
            || self.conns[ci].look.look.is_some()
            || voice.is_some();
        let look = crate::viewer::look_of(&self.conns[ci].look);
        // (R5/M20) An open capsule design other than `r4` builds the capsule depth.
        let open = self.conns[ci].look.open.clone();
        if d == OpenDepth::Capsule
            && (self.conns[ci].look.look.is_some() || voice.is_some() || open.is_some())
        {
            let Some(id) = pr.capsule_view.as_ref().map(|v| v.id.clone()) else {
                return Ok(None);
            };
            let max_points = self.conns[ci].profile.max_points;
            let lang = self.lang_of(ci);
            let (w, h) = self.frame(ci, depth_key(d));
            let ask = self.conns[ci].look.ask.clone();
            warm(rd!(self), &mut self.cache);
            let p = DocProvider::new(rd!(self), &self.cache)
                .with_headlines(&self.headlines)
                .with_parts(self, lang)
                .with_voice(voice.as_deref())
                .with_clock(self.conns[ci].look.clock);
            // (R5/M20) The design's view; `nobra` also reads the opened question and the
            // frame (the authority has no device check: "verified by the server").
            let ctx = qp_designs::open::OpenCtx {
                ask: ask.as_deref(),
                lang,
                check: None,
                frame: [w, h],
            };
            return Ok(qp_designs::open::capsule_view(
                open.as_deref(),
                look,
                finding,
                max_points,
                &p,
                &id,
                &ctx,
            )
            .ok());
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
                .with_parts(self, lang)
                .with_voice(voice.as_deref())
                .with_clock(self.conns[ci].look.clock);
            return Ok(qp_designs::depth::face_view_in(
                look,
                name.as_deref(),
                finding,
                max_points,
                &p,
                &format!("capsule-{finding}"),
            )
            .ok());
        }
        // (R5/S) The chart depth under a look other than `r4`: the presenter's chart rule
        // with the look's sizes (`depth::chart_view_in`), as the device builds it
        // (`viewer::open_view`): a view stored in the document (a capsule's own) is kept,
        // the presenter's generated one is rebuilt. `r4`: the presentation's view as before.
        if matches!(
            depth::presenter_depth(face),
            Depth::Chart | Depth::Workspace
        ) && self.conns[ci].look.look.is_some()
        {
            if let Some(given) = pr.view.as_ref() {
                if !rd!(self).views.iter().any(|x| x.id == given.id) {
                    let id = self.conns[ci]
                        .look
                        .view
                        .clone()
                        .unwrap_or_else(|| given.id.clone());
                    warm(rd!(self), &mut self.cache);
                    let p =
                        DocProvider::new(rd!(self), &self.cache).with_headlines(&self.headlines);
                    if let Ok(v) = depth::chart_view_in(look, finding, &p, &id) {
                        return Ok(Some(v));
                    }
                }
            }
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
    pub(super) fn reader_scene(
        &mut self,
        ci: usize,
        view: &View,
        frame: (u32, u32),
        range: Option<(u64, u64)>,
        face: Option<String>,
        story: Option<StoryCompiled<'_>>,
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
        let mut opts = qp_designs::options(frame.0, frame.1);
        opts.range = range;
        // (R4/T5) The connection's style (view state).
        opts.theme.clone_from(&self.conns[ci].look.theme);
        opts.alphabet.clone_from(&self.conns[ci].look.alphabet);
        opts.outcome.clone_from(&self.conns[ci].look.outcome);
        opts.look.clone_from(&self.conns[ci].look.look);
        opts.face = face;
        let lang = self.lang_of(ci);
        let voice = self.conns[ci].look.voice.clone();
        let p = DocProvider::new(rd!(self), &self.cache)
            .with_headlines(&self.headlines)
            .with_parts(self, lang)
            .with_voice(voice.as_deref())
            .with_clock(self.conns[ci].look.clock);
        // (RP4/T7) The rows' time, when the connection asked for it.
        let stamp = |r: &qp_schema::protocol::StoryRow| self.row_stamp(r, lang);
        let stamp_of: Option<&qp_designs::stories::StampFn<'_>> =
            self.conns[ci].look.clock.then_some(&stamp);
        // (R5/S) A story chart design that compiles the whole scene (`korda`).
        let scene = match story {
            Some((c, rows, finding)) => c(
                view,
                &p,
                &opts,
                &qp_designs::stories::StoryInput {
                    rows,
                    focus: Some(finding),
                    hide_closed: self.conns[ci].look.hide_closed,
                    lang,
                    fact_of: &|_| None,
                    stamp_of,
                },
            ),
            None => compile(view, &p, &opts),
        }
        .map_err(|e| E::new(ErrorCode::Invalid, e.to_string()).at(GP).build())?;
        let times = view_bar_series(rd!(self), view)
            .and_then(Series::bars)
            .map(|b| b.iter().map(|b| b.t).collect())
            .unwrap_or_default();
        Ok((scene, times))
    }

    /// The scene of a capsule at depth `d` on connection `ci` (at its frame, at chart
    /// depth over the view state's range), with its view; `None` without a view.
    pub(super) fn capsule_scene(
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
        // (R5/M18) A story chart design: the chart at its price height, then the stories.
        let story = if d >= OpenDepth::Chart {
            qp_designs::depth::story_design(self.conns[ci].look.story.as_deref())
        } else {
            None
        };
        let Some(sd) = story else {
            let (scene, times) = self.reader_scene(ci, &view, frame, range, face, None)?;
            return Ok(Some((view, scene, times)));
        };
        if let Some(c) = sd.compile {
            let rows = match view_bar_series(rd!(self), &view).map(|s| s.id.clone()) {
                Some(series) => {
                    let principal = self.conns[ci].principal.clone();
                    let lang = self.lang_of(ci);
                    self.story_rows(&series, &principal, lang)
                }
                None => Vec::new(),
            };
            let (scene, times) =
                self.reader_scene(ci, &view, frame, range, face, Some((c, &rows, finding)))?;
            return Ok(Some((view, scene, times)));
        }
        let at = (
            frame.0,
            (sd.price_height)(crate::viewer::look_of(&self.conns[ci].look), frame.1),
        );
        let (scene, times) = self.reader_scene(ci, &view, at, range, face, None)?;
        let scene = self.story_extend(ci, sd, &view, scene, frame.1, finding);
        Ok(Some((view, scene, times)))
    }

    /// (R5/M18) Adds the story part of design `sd` to the open capsule's chart `scene`
    /// (the reader's story rows of the view's bar series; the focus: `finding`).
    fn story_extend(
        &mut self,
        ci: usize,
        sd: &qp_designs::stories::StoryDesign,
        view: &View,
        scene: SceneDesc,
        h: u32,
        finding: &str,
    ) -> SceneDesc {
        let Some(series) = view_bar_series(rd!(self), view).map(|s| s.id.clone()) else {
            return scene;
        };
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        let rows = self.story_rows(&series, &principal, lang);
        let firsts = crate::viewer::FirstFacts::new(rd!(self), &rows);
        let fact_of = |f: &str| firsts.get(f);
        let look = &self.conns[ci].look;
        let stamp = |r: &qp_schema::protocol::StoryRow| self.row_stamp(r, lang);
        let stamp_of: Option<&qp_designs::stories::StampFn<'_>> = look.clock.then_some(&stamp);
        (sd.extend)(
            crate::viewer::look_of(look),
            look.theme.as_deref().or(view.theme.as_deref()),
            scene,
            h,
            &qp_designs::stories::StoryInput {
                rows: &rows,
                focus: Some(finding),
                hide_closed: look.hide_closed,
                lang,
                fact_of: &fact_of,
                stamp_of,
            },
        )
    }

    /// The visible range of a capsule's chart: the view state's (if it is that view's),
    /// else the surface's initial range (§4.6), clamped to the current slots.
    pub(super) fn chart_range(&self, ci: usize, view: &View) -> Option<(u64, u64)> {
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

    #[allow(clippy::too_many_lines)]
    /// Moves capsule `cap` from depth `from` to `to` on connection `ci`: the view state,
    /// what to show at `to`, the shared elements and (at evidence) the evidence access.
    pub(super) fn transition(
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
        let mut presentation = self.present_at(ci, finding, to)?;
        // (R5/M20) An open capsule design that writes a text column for a reader without
        // a scene reads the facts at the capsule depth too (as the chart depth gives them
        // to this reader: redacted evidence without the entitlement, K33).
        if to == OpenDepth::Capsule
            && presentation.capsule_view.is_none()
            && presentation.facts.is_empty()
            && qp_designs::open::by_name(self.conns[ci].look.open.as_deref())
                .text
                .is_some()
        {
            presentation.facts = self.present_at(ci, finding, OpenDepth::Chart)?.facts;
        }
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
            // (R5/M20) The opened question belongs to the open capsule.
            if to == OpenDepth::Sentence || look.capsule.as_deref() != Some(cap) {
                look.ask = None;
            }
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
        // (`qp_designs::transitions`; `r3` = the anchors both scenes have).
        let shared = {
            fn side(d: OpenDepth, s: Option<&SideScene>) -> qp_designs::transitions::Side<'_> {
                qp_designs::transitions::Side {
                    depth: d,
                    scene: s.map(|x| &x.0),
                    price_series: s.and_then(|x| x.1.as_deref()),
                    marked_bar: s.and_then(|x| x.2),
                }
            }
            let design =
                qp_designs::transitions::by_name(self.conns[ci].look.transition.as_deref());
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
}
