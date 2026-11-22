//! View-side reads for queries and input: bars at an index, ranges, viewports, screen
//! info and the `input` message.

use super::*;

impl Session {
    pub(crate) fn at_bar(&mut self, series: &str, i: usize) -> AtBar {
        let bars = rd!(self)
            .series
            .iter()
            .find(|s| s.id == series)
            .and_then(Series::bars)
            .unwrap_or_default();
        let ohlcv = bars[i].clone();
        let deps: Vec<String> = rd!(self)
            .series
            .iter()
            .filter(|s| {
                matches!(&s.source, SeriesSource::Study { inputs, .. } if inputs.first().map(String::as_str) == Some(series))
            })
            .map(|s| s.id.clone())
            .collect();
        let mut studies = BTreeMap::new();
        for d in deps {
            if let Ok(v) = values(rd!(self), &mut self.cache, &d) {
                let m: BTreeMap<String, Option<f64>> = v
                    .outputs
                    .iter()
                    .map(|(k, col)| (k.clone(), col.get(i).copied().flatten()))
                    .collect();
                studies.insert(d, m);
            }
        }
        AtBar {
            series: series.to_owned(),
            bar: i as u64,
            ohlcv,
            studies,
        }
    }

    /// Times of the view's future slots (after its last bar; point series such as a
    /// forecast fan, `docs/GRAMMAR.md` §3). A viewport may reach them (K25).
    pub(crate) fn future_times(&self, view: &View) -> Vec<i64> {
        qp_grammar::compile::future_times(view, &DocProvider::new(rd!(self), &self.cache))
    }

    /// Initial visible range of a view for a surface (§4.6): the slots whose time falls
    /// in `x.domain`, else the surface's last N bars and every future slot after them
    /// (K25: the default viewport covers the future points).
    pub(crate) fn initial_range(
        &self,
        view: &View,
        kind: qp_schema::protocol::SurfaceKind,
    ) -> Option<(u64, u64)> {
        use qp_schema::protocol::SurfaceKind as K;
        let bars = view_bar_series(rd!(self), view).and_then(Series::bars)?;
        if bars.is_empty() {
            return None;
        }
        let future = self.future_times(view);
        let last_slot = bars.len() - 1 + future.len();
        if let Some([a, b]) = view.x.domain {
            let times: Vec<i64> = bars
                .iter()
                .map(|x| x.t)
                .chain(future.iter().copied())
                .collect();
            #[allow(clippy::cast_precision_loss)]
            let from = times.iter().position(|t| *t as f64 >= a);
            #[allow(clippy::cast_precision_loss)]
            let to = times.iter().rposition(|t| *t as f64 <= b);
            if let (Some(f), Some(t)) = (from, to) {
                if f <= t {
                    return Some((f as u64, t as u64));
                }
            }
        }
        let n = match kind {
            K::Phone => 60,
            K::Watch => 20,
            K::Desktop | K::Voice | K::Headless => 120,
        };
        Some((bars.len().saturating_sub(n) as u64, last_slot as u64))
    }

    /// The last slot of a view: its last bar plus its future slots (K25); `None` without
    /// bars.
    pub(crate) fn last_slot(&self, view: &View) -> Option<u64> {
        let n = view_bar_series(rd!(self), view)
            .and_then(Series::bars)
            .map_or(0, <[qp_schema::Bar]>::len) as u64;
        let last = n.checked_sub(1)?;
        Some(last + self.future_times(view).len() as u64)
    }

    pub(crate) fn range_of(&self, c: &Conn, view: &View) -> Option<(u64, u64)> {
        let saved = c.viewports.get(&view.id).copied().and_then(|(a, b)| {
            // A saved viewport can outlive data that got shorter (an undone
            // `append_bars`): clamp it to the current slots (bars + future slots).
            let last = self.last_slot(view)?;
            let b = b.min(last);
            Some((a.min(b), b))
        });
        saved.or_else(|| self.initial_range(view, c.surface.kind))
    }

    /// The connection's current viewport: the last one set by input, else the initial
    /// range of the document's first view.
    pub(crate) fn conn_viewport(&self, c: &Conn) -> Option<Viewport> {
        if let Some(v) = &c.current {
            // Re-clamp through `range_of` (the data may have shrunk since).
            if let Some(view) = rd!(self).views.iter().find(|x| x.id == v.view) {
                if let Some((a, b)) = self.range_of(c, view) {
                    return Some(Viewport {
                        view: v.view.clone(),
                        from_bar: a,
                        to_bar: b,
                    });
                }
            }
            return Some(v.clone());
        }
        let view = rd!(self).views.first()?;
        let (a, b) = self.range_of(c, view)?;
        Some(Viewport {
            view: view.id.clone(),
            from_bar: a,
            to_bar: b,
        })
    }

    pub(super) fn screen_info(&self, c: &Conn, asker: &str) -> Option<ScreenInfo> {
        let vp = self.conn_viewport(c)?;
        let view = rd!(self).views.iter().find(|v| v.id == vp.view)?;
        let bs = view_bar_series(rd!(self), view);
        let mut panels: BTreeMap<u32, (Vec<String>, Vec<Mark>)> = BTreeMap::new();
        for l in &view.layers {
            let e = panels.entry(l.panel).or_default();
            if let Some(s) = &l.series {
                if !e.0.contains(s) {
                    e.0.push(s.clone());
                }
            }
            e.1.push(l.mark);
        }
        let capsules = rd!(self)
            .capsules
            .iter()
            .filter(|cap| {
                self.doc
                    .findings
                    .iter()
                    .find(|f| f.id == cap.finding)
                    .is_some_and(|f| {
                        f.facts.iter().any(|fid| {
                            self.doc
                                .facts
                                .iter()
                                .find(|x| &x.id == fid)
                                .is_some_and(|x| x.bar >= vp.from_bar && x.bar <= vp.to_bar)
                        })
                    })
            })
            .map(|cap| cap.id.clone())
            .collect();
        // The viewport may end in a future slot (K25): the last close shown is the
        // close of the last bar in it.
        let last_close = bs
            .filter(|s| self.plane.gate(&self.doc, Some(asker), &s.id).is_none())
            .and_then(Series::bars)
            .and_then(|b| {
                b.get(
                    usize::try_from(vp.to_bar)
                        .ok()?
                        .min(b.len().checked_sub(1)?),
                )
            })
            .map(|b| b.c);
        // R2: name, scale and layers from the compiled scene (`docs/GRAMMAR.md` §16).
        let mut compiled = self.screen_panels(c, &vp, asker);
        // Stack order (top to bottom, §5.4), not panel id order: they differ after
        // `move_panel` / `insert_panel`.
        let stack = |id: u32| {
            view.panels
                .iter()
                .position(|p| p.id == Some(id))
                .unwrap_or(usize::MAX)
        };
        let mut panels: Vec<_> = panels.into_iter().collect();
        panels.sort_by_key(|(id, _)| (stack(*id), *id));
        Some(ScreenInfo {
            principal: c.principal.clone(),
            surface: c.surface.kind,
            viewport: vp,
            symbol: bs.and_then(|s| s.symbol().map(str::to_owned)),
            interval: bs.and_then(|s| s.interval.clone()),
            panels: panels
                .into_iter()
                .map(|(panel, (series, marks))| {
                    let (name, scale, layers) = match compiled.remove(&panel) {
                        Some((n, s, l)) => (n, s, l),
                        None => (None, None, Vec::new()),
                    };
                    PanelInfo {
                        select: crate::select::Resolver {
                            doc: rd!(self),
                            root: None,
                        }
                        .panel_hint(view, panel),
                        name,
                        scale,
                        layers,
                        panel,
                        series,
                        marks,
                    }
                })
                .collect(),
            capsules,
            last_close,
            view_state: (c.look != qp_schema::protocol::ViewState::default())
                .then(|| c.look.clone()),
        })
    }

    /// Handles a gesture; returns the result and the notifications it causes.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn input(
        &mut self,
        ci: usize,
        g: &Gesture,
    ) -> PResult<(QueryResult, Vec<OutboundLine>)> {
        let gp = "/gesture";
        let view_id = match g {
            Gesture::Look { view, .. }
            | Gesture::Pan { view, .. }
            | Gesture::Zoom { view, .. }
            | Gesture::Tap { view, .. } => view,
            // (R4) Pixel gestures and open/close: `interact.rs`.
            _ => return self.interact(ci, g),
        };
        let view = need_view(rd!(self), view_id, &ptr(gp, "view"))?.clone();
        let bar_series = view_bar_series(rd!(self), &view)
            .filter(|s| s.bars().is_some_and(|b| !b.is_empty()))
            .map(|s| s.id.clone())
            .ok_or_else(|| {
                invalid(
                    format!("view `{view_id}` draws no bar series"),
                    ptr(gp, "view"),
                )
            })?;
        let n = rd!(self)
            .series
            .iter()
            .find(|s| s.id == bar_series)
            .and_then(Series::bars)
            .map_or(0, <[qp_schema::Bar]>::len) as u64;
        if n == 0 {
            return Err(invalid(
                format!("series `{bar_series}` has no bars"),
                ptr(gp, "view"),
            ));
        }
        let last = n - 1;
        // K25: a viewport may reach the future slots (points after the last bar).
        let future = self.future_times(&view).len() as u64;
        let last_slot = last + future;
        let slots = n + future;
        // A saved viewport can outlive data that got shorter (e.g. an undone
        // `append_bars`): clamp it to the current slots before using it.
        let (from, to) = self
            .range_of(&self.conns[ci], &view)
            .unwrap_or((0, last_slot));
        let to = to.min(last_slot);
        let from = from.min(to);
        let past_bar = |field: &str, v: u64| {
            E::new(
                ErrorCode::Invalid,
                format!("bar {v} is past the last bar {last}"),
            )
            .at(ptr(gp, field))
            .expected([format!("0..={last}")])
            .build()
        };
        let past_slot = |field: &str, v: u64| {
            if future == 0 {
                return past_bar(field, v);
            }
            E::new(
                ErrorCode::Invalid,
                format!(
                    "slot {v} is past the last slot {last_slot} (last bar {last} + {future} future slot(s))"
                ),
            )
            .at(ptr(gp, field))
            .expected([format!("0..={last_slot}")])
            .build()
        };
        let (nf, nt) = match g {
            Gesture::Look {
                from_bar, to_bar, ..
            } => {
                if *to_bar > last_slot {
                    return Err(past_slot("to_bar", *to_bar));
                }
                if from_bar > to_bar {
                    return Err(invalid(
                        format!("from_bar {from_bar} is after to_bar {to_bar}"),
                        ptr(gp, "from_bar"),
                    ));
                }
                (*from_bar, *to_bar)
            }
            Gesture::Pan { bars, .. } => qp_grammar::gesture::pan_range(from, to, *bars, last_slot),
            Gesture::Zoom {
                factor, anchor_bar, ..
            } => {
                if !(factor.is_finite() && *factor > 0.0) {
                    return Err(invalid(
                        "factor must be a positive number",
                        ptr(gp, "factor"),
                    ));
                }
                let anchor = anchor_bar.unwrap_or(to);
                if anchor > last_slot {
                    return Err(past_slot("anchor_bar", anchor));
                }
                // (R4/S1) One rule for bar-space and pixel gestures (`qp_grammar::gesture`).
                qp_grammar::gesture::zoom_range(from, to, *factor, anchor, slots)
            }
            Gesture::Tap { panel, bar, .. } => {
                // A tap answers the bar's values (data plane, §18.3).
                self.gate_read(ci, &bar_series, &ptr(gp, "view"))?;
                if *bar > last {
                    return Err(past_bar("bar", *bar));
                }
                if view.panel(*panel).is_none() {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!("view `{view_id}` has no panel {panel}"),
                    )
                    .at(ptr(gp, "panel"))
                    .expected(
                        view.panels
                            .iter()
                            .filter_map(|p| p.id)
                            .map(|i| i.to_string()),
                    )
                    .build());
                }
                self.input_counter += 1;
                let principal = self.conns[ci].principal.clone();
                let c = &mut self.conns[ci];
                c.last_input = self.input_counter;
                c.last_at = Some(self.clock);
                self.focus.insert(
                    principal,
                    Target {
                        layer: None,
                        view: Some(view.id.clone()),
                        panel: Some(*panel),
                        series: None,
                        output: None,
                        item: None,
                        from_bar: Some(*bar),
                        to_bar: Some(*bar),
                    },
                );
                let i = usize::try_from(*bar).unwrap_or(0);
                return Ok((
                    QueryResult::At {
                        at: self.at_bar(&bar_series, i),
                    },
                    Vec::new(),
                ));
            }
            _ => unreachable!("pixel gestures return above"),
        };
        let vp = Viewport {
            view: view.id.clone(),
            from_bar: nf,
            to_bar: nt,
        };
        self.input_counter += 1;
        let c = &mut self.conns[ci];
        c.viewports.insert(view.id.clone(), (nf, nt));
        c.current = Some(vp.clone());
        c.last_input = self.input_counter;
        c.last_at = Some(self.clock);
        let principal = c.principal.clone();
        let events = self.viewport_events(&principal, &vp);
        Ok((QueryResult::Viewport { viewport: vp }, events))
    }
}
