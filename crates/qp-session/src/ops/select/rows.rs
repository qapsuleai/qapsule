//! Candidate rows per selector kind (series, views, panels, layers, facts, findings,
//! capsules, proposals).

use super::*;

impl<'a> Resolver<'a> {
    // -- nested sets -------------------------------------------------------

    pub(super) fn series_set(&self, r: &SeriesRef, path: &str) -> PResult<BTreeSet<String>> {
        Ok(match r {
            SeriesRef::Id(id) => BTreeSet::from([id.clone()]),
            SeriesRef::Select(s) => {
                let o = self.series_rows(s, path)?;
                nested(Kind::Series, s, o, s.first, s.latest, path)?
                    .into_iter()
                    .map(|r| r.id)
                    .collect()
            }
        })
    }

    fn fact_set(&self, r: &FactRef, path: &str) -> PResult<BTreeSet<String>> {
        Ok(match r {
            FactRef::Id(id) => BTreeSet::from([id.clone()]),
            FactRef::Select(s) => {
                let o = self.fact_rows(s, path)?;
                nested(Kind::Fact, s, o, s.first, s.latest, path)?
                    .into_iter()
                    .map(|r| r.id)
                    .collect()
            }
        })
    }

    fn finding_set(&self, r: &FindingRef, path: &str) -> PResult<BTreeSet<String>> {
        Ok(match r {
            FindingRef::Id(id) => BTreeSet::from([id.clone()]),
            FindingRef::Select(s) => {
                let o = self.finding_rows(s, path)?;
                nested(Kind::Finding, s, o, s.first, s.latest, path)?
                    .into_iter()
                    .map(|r| r.id)
                    .collect()
            }
        })
    }

    fn panel_set(&self, view: &View, r: &PanelRef, path: &str) -> PResult<BTreeSet<u32>> {
        Ok(match r {
            PanelRef::Id(id) => BTreeSet::from([*id]),
            PanelRef::Select(s) => {
                let o = self.panel_rows(Some(view), s, path)?;
                nested(Kind::Panel, s, o, s.first, s.latest, path)?
                    .into_iter()
                    .filter_map(|r| r.value.as_u64().and_then(|x| u32::try_from(x).ok()))
                    .collect()
            }
        })
    }

    // -- rows per kind ----------------------------------------------------

    pub(super) fn series_rows(&self, sel: &SeriesSel, path: &str) -> PResult<Outcome> {
        let doc = self.doc;
        let on = match &sel.on {
            Some(r) => Some(self.series_set(r, &ptr(path, "on"))?),
            None => None,
        };
        let mut o = Outcome::new();
        let keys = &mut o.keys;
        if sel.kind.is_some() {
            keys.push("kind");
        }
        if sel.study.is_some() {
            keys.push("study");
        }
        if !sel.params.is_empty() {
            keys.push("params");
        }
        if on.is_some() {
            keys.push("on");
        }
        if sel.symbol.is_some() {
            keys.push("symbol");
        }
        if sel.label.is_some() {
            keys.push("label");
        }
        for s in &doc.series {
            let (study, params, inputs) = match &s.source {
                SeriesSource::Study {
                    study,
                    params,
                    inputs,
                    ..
                } => (Some(study.as_str()), Some(params), inputs.as_slice()),
                _ => (None, None, &[][..]),
            };
            let mut c = Vec::new();
            if let Some(k) = sel.kind {
                c.push(series_kind(s) == k);
            }
            if let Some(st) = &sel.study {
                c.push(study.is_some_and(|x| eq_text(x, st)));
            }
            if !sel.params.is_empty() {
                c.push(params.is_some_and(|p| {
                    sel.params
                        .iter()
                        .all(|(k, v)| p.get(k).is_some_and(|x| x == v))
                }));
            }
            if let Some(set) = &on {
                c.push(inputs.iter().any(|i| set.contains(i)));
            }
            if let Some(sym) = &sel.symbol {
                c.push(s.symbol().is_some_and(|x| eq_text(x, sym)));
            }
            if let Some(l) = &sel.label {
                c.push(eq_text(&series_label(s), l));
            }
            o.rows.push(Row {
                id: s.id.clone(),
                about: series_about(s),
                value: Value::String(s.id.clone()),
                checks: c,
            });
        }
        if let Some(st) = &sel.study {
            let v = doc
                .series
                .iter()
                .filter_map(|s| match &s.source {
                    SeriesSource::Study { study, .. } => Some(study.clone()),
                    _ => None,
                })
                .collect();
            o.vocab.push(("study", st.clone(), v));
        }
        if let Some(sym) = &sel.symbol {
            let v = doc
                .series
                .iter()
                .filter_map(|s| s.symbol().map(str::to_owned))
                .collect();
            o.vocab.push(("symbol", sym.clone(), v));
        }
        if let Some(l) = &sel.label {
            let v = doc.series.iter().map(series_label).collect();
            o.vocab.push(("label", l.clone(), v));
        }
        Ok(o)
    }

    pub(super) fn view_rows(&self, sel: &ViewSel, path: &str) -> PResult<Outcome> {
        let doc = self.doc;
        let shows = match &sel.shows {
            Some(r) => Some(self.series_set(r, &ptr(path, "shows"))?),
            None => None,
        };
        let mut o = Outcome::new();
        if sel.title.is_some() {
            o.keys.push("title");
        }
        if shows.is_some() {
            o.keys.push("shows");
        }
        for v in &doc.views {
            let title = view_title(doc, v);
            let mut c = Vec::new();
            if let Some(t) = &sel.title {
                c.push(title.as_deref().is_some_and(|x| eq_text(x, t)));
            }
            if let Some(set) = &shows {
                c.push(
                    v.layers
                        .iter()
                        .any(|l| l.series.as_ref().is_some_and(|s| set.contains(s))),
                );
            }
            o.rows.push(Row {
                id: v.id.clone(),
                about: format!(
                    "{}: {} panels, {} layers",
                    title.unwrap_or_else(|| v.id.clone()),
                    v.panels.len(),
                    v.layers.len()
                ),
                value: Value::String(v.id.clone()),
                checks: c,
            });
        }
        if let Some(t) = &sel.title {
            let vals = doc
                .views
                .iter()
                .filter_map(|v| view_title(doc, v))
                .collect();
            o.vocab.push(("title", t.clone(), vals));
        }
        Ok(o)
    }

    pub(super) fn scope<'v>(&self, view: Option<&'v View>) -> Vec<&'v View>
    where
        'a: 'v,
    {
        match view {
            Some(v) => vec![v],
            None => self.doc.views.iter().collect(),
        }
    }

    pub(super) fn layer_rows(
        &self,
        view: Option<&View>,
        sel: &LayerSel,
        path: &str,
        loose: bool,
    ) -> PResult<Outcome> {
        let doc = self.doc;
        let shows = match &sel.shows {
            Some(r) => Some(self.series_set(r, &ptr(path, "shows"))?),
            None => None,
        };
        let facts = match &sel.fact {
            Some(r) => Some(self.fact_set(r, &ptr(path, "fact"))?),
            None => None,
        };
        let findings = match &sel.finding {
            Some(r) => Some(self.finding_set(r, &ptr(path, "finding"))?),
            None => None,
        };
        let mut o = Outcome::new();
        for (k, on) in [
            ("shows", shows.is_some()),
            ("output", sel.output.is_some()),
            ("mark", sel.mark.is_some()),
            ("panel", sel.panel.is_some()),
            ("value", sel.value.is_some()),
            ("fact", facts.is_some()),
            ("finding", findings.is_some()),
            ("label", sel.label.is_some()),
        ] {
            if on {
                o.keys.push(k);
            }
        }
        let mut outputs = Vec::new();
        let mut labels = Vec::new();
        for v in self.scope(view) {
            let panels = match &sel.panel {
                Some(r) => Some(self.panel_set(v, r, &ptr(path, "panel"))?),
                None => None,
            };
            for l in &v.layers {
                let Some(id) = &l.id else { continue };
                let label = layer_label(doc, l);
                if let Some(t) = &label {
                    labels.push(t.clone());
                }
                if let Data::Series(s) = layer_data(doc, l) {
                    outputs.extend(crate::canon::series_fields(s));
                }
                let mut c = Vec::new();
                if let Some(set) = &shows {
                    c.push(l.series.as_ref().is_some_and(|s| set.contains(s)));
                }
                if let Some(f) = &sel.output {
                    c.push(layer_draws(doc, l, f, loose));
                }
                if let Some(m) = sel.mark {
                    c.push(l.mark == m);
                }
                if let Some(set) = &panels {
                    c.push(set.contains(&l.panel));
                }
                if let Some(x) = sel.value {
                    c.push(layer_constants(l).contains(&x));
                }
                if let Some(set) = &facts {
                    c.push(l.fact.as_ref().is_some_and(|f| set.contains(f)));
                }
                if let Some(set) = &findings {
                    c.push(l.finding.as_ref().is_some_and(|f| set.contains(f)));
                }
                if let Some(t) = &sel.label {
                    c.push(label.as_deref().is_some_and(|x| eq_text(x, t)));
                }
                o.rows.push(Row {
                    id: id.clone(),
                    about: format!(
                        "{} (panel {} of view {})",
                        layer_about(doc, l),
                        l.panel,
                        v.id
                    ),
                    value: Value::String(id.clone()),
                    checks: c,
                });
            }
        }
        if let Some(f) = &sel.output {
            o.vocab.push(("output", f.clone(), outputs));
        }
        if let Some(t) = &sel.label {
            o.vocab.push(("label", t.clone(), labels));
        }
        Ok(o)
    }

    pub(super) fn fact_rows(&self, sel: &FactSel, path: &str) -> PResult<Outcome> {
        let doc = self.doc;
        let on = match &sel.on {
            Some(r) => Some(self.series_set(r, &ptr(path, "on"))?),
            None => None,
        };
        let mut o = Outcome::new();
        for (k, g) in [
            ("kind", sel.kind.is_some()),
            ("on", on.is_some()),
            ("bar", sel.bar.is_some()),
        ] {
            if g {
                o.keys.push(k);
            }
        }
        let mut order: Vec<(u64, usize, &Fact)> = doc
            .facts
            .iter()
            .enumerate()
            .map(|(i, f)| (f.bar, i, f))
            .collect();
        order.sort_by_key(|(b, i, _)| (*b, *i));
        for (_, _, f) in order {
            let mut c = Vec::new();
            if let Some(k) = &sel.kind {
                c.push(eq_text(&f.kind, k));
            }
            if let Some(set) = &on {
                c.push(set.contains(&f.series));
            }
            if let Some(b) = sel.bar {
                c.push(f.bar == b);
            }
            o.rows.push(Row {
                id: f.id.clone(),
                about: fact_about(f),
                value: Value::String(f.id.clone()),
                checks: c,
            });
        }
        if let Some(k) = &sel.kind {
            let v = doc.facts.iter().map(|f| f.kind.clone()).collect();
            o.vocab.push(("kind", k.clone(), v));
        }
        Ok(o)
    }

    pub(super) fn finding_rows(&self, sel: &FindingSel, path: &str) -> PResult<Outcome> {
        let doc = self.doc;
        let on = match &sel.on {
            Some(r) => Some(self.series_set(r, &ptr(path, "on"))?),
            None => None,
        };
        let facts = match &sel.fact {
            Some(r) => Some(self.fact_set(r, &ptr(path, "fact"))?),
            None => None,
        };
        let mut o = Outcome::new();
        for (k, g) in [
            ("kind", sel.kind.is_some()),
            ("on", on.is_some()),
            ("fact", facts.is_some()),
            ("state", sel.state.is_some()),
            ("instrument", sel.instrument.is_some()),
            ("meaning", sel.meaning.is_some()),
            ("stage", sel.stage.is_some()),
            ("open", sel.open.is_some()),
        ] {
            if g {
                o.keys.push(k);
            }
        }
        let mut order: Vec<(Option<u64>, usize, &Finding)> = doc
            .findings
            .iter()
            .enumerate()
            .map(|(i, f)| (finding_bar(doc, f), i, f))
            .collect();
        order.sort_by_key(|(b, i, _)| (*b, *i));
        for (_, _, f) in order {
            let mut c = Vec::new();
            if let Some(k) = &sel.kind {
                c.push(finding_facts(doc, f).any(|x| eq_text(&x.kind, k)));
            }
            if let Some(set) = &on {
                c.push(finding_facts(doc, f).any(|x| set.contains(&x.series)));
            }
            if let Some(set) = &facts {
                c.push(f.facts.iter().any(|x| set.contains(x)));
            }
            if let Some(st) = sel.state {
                c.push(
                    doc.capsules
                        .iter()
                        .find(|cap| cap.finding == f.id)
                        .is_some_and(|cap| cap.state == st),
                );
            }
            // (R4) Keys about the finding's capsule story.
            let story = doc
                .capsules
                .iter()
                .find(|cap| cap.finding == f.id)
                .and_then(|cap| cap.story.as_ref());
            if let Some(i) = &sel.instrument {
                c.push(story.is_some_and(|s| eq_text(&s.instrument, i)));
            }
            if let Some(m) = sel.meaning {
                c.push(story.is_some_and(|s| s.kind == m));
            }
            if let Some(st) = sel.stage {
                c.push(story.is_some_and(|s| s.stage == st));
            }
            if let Some(op) = sel.open {
                c.push(story.is_some_and(|s| s.stage.is_closed() != op));
            }
            o.rows.push(Row {
                id: f.id.clone(),
                about: finding_about(doc, f),
                value: Value::String(f.id.clone()),
                checks: c,
            });
        }
        if let Some(k) = &sel.kind {
            let v = doc.facts.iter().map(|f| f.kind.clone()).collect();
            o.vocab.push(("kind", k.clone(), v));
        }
        Ok(o)
    }

    /// (R4) Capsules with a story, in the latest transition's bar order, then birth bar,
    /// then document order.
    pub(super) fn capsule_rows(&self, sel: &CapsuleSel) -> Outcome {
        let doc = self.doc;
        let mut o = Outcome::new();
        for (k, g) in [
            ("instrument", sel.instrument.is_some()),
            ("meaning", sel.meaning.is_some()),
            ("direction", sel.direction.is_some()),
            ("stage", sel.stage.is_some()),
            ("open", sel.open.is_some()),
            ("born_bar", sel.born_bar.is_some()),
            ("series", sel.series.is_some()),
        ] {
            if g {
                o.keys.push(k);
            }
        }
        let mut order: Vec<(
            u64,
            u64,
            usize,
            &qp_schema::Capsule,
            &qp_schema::capsule::Story,
        )> = doc
            .capsules
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.story.as_ref().map(|s| (s.last_bar, s.born_bar, i, c, s)))
            .collect();
        order.sort_by_key(|(l, b, i, _, _)| (*l, *b, *i));
        for (_, _, _, c, st) in order {
            let mut ch = Vec::new();
            if let Some(i) = &sel.instrument {
                ch.push(eq_text(&st.instrument, i));
            }
            if let Some(m) = sel.meaning {
                ch.push(st.kind == m);
            }
            if let Some(d) = sel.direction {
                ch.push(st.direction == d);
            }
            if let Some(x) = sel.stage {
                ch.push(st.stage == x);
            }
            if let Some(op) = sel.open {
                ch.push(st.stage.is_closed() != op);
            }
            if let Some(b) = sel.born_bar {
                ch.push(st.born_bar == b);
            }
            if let Some(sr) = &sel.series {
                ch.push(st.series == *sr);
            }
            o.rows.push(Row {
                id: c.id.clone(),
                about: format!(
                    "{} {} {} ({}), born bar {}, last bar {}",
                    st.instrument,
                    st.kind.name(),
                    qp_capsule::engine::direction_name(st.direction),
                    serde_json::to_value(st.stage)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_default(),
                    st.born_bar,
                    st.last_bar
                ),
                value: Value::String(c.id.clone()),
                checks: ch,
            });
        }
        if let Some(i) = &sel.instrument {
            let v = doc
                .capsules
                .iter()
                .filter_map(|c| c.story.as_ref().map(|s| s.instrument.clone()))
                .collect();
            o.vocab.push(("instrument", i.clone(), v));
        }
        o
    }

    pub(super) fn proposal_rows(&self, sel: &ProposalSel) -> Outcome {
        let mut o = Outcome::new();
        if sel.state.is_some() {
            o.keys.push("state");
        }
        for p in &self.doc.proposals {
            let mut c = Vec::new();
            if let Some(s) = sel.state {
                c.push(p.state == s);
            }
            let state = serde_json::to_value(p.state)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            o.rows.push(Row {
                id: p.id.clone(),
                about: format!("{state}: {}", short(&summarize_ops(&p.ops, &[]), 80)),
                value: Value::String(p.id.clone()),
                checks: c,
            });
        }
        o
    }
}
