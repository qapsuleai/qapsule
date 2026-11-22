//! Resolving a site to ids, and the hints that name an object by a selector.

use super::*;

impl<'a> Resolver<'a> {
    // -- entry points -----------------------------------------------------

    /// Checks a selector against its schema and parses it.
    fn parse<T: serde::de::DeserializeOwned>(
        &self,
        def: &str,
        v: &Value,
        path: &str,
    ) -> PResult<T> {
        if let Some(root) = self.root {
            let schema = json!({ "$ref": format!("#/$defs/{def}") });
            if let Some(viol) = locate(v, &schema, root) {
                return Err(violation_error(&viol, path));
            }
        }
        serde_json::from_value(v.clone()).map_err(|e| {
            E::new(ErrorCode::Schema, format!("invalid selector: {e}"))
                .at(path)
                .build()
        })
    }

    /// The view a panel/layer/scale site belongs to, if its message names one.
    fn context(&self, msg: &Value, site: &Site) -> PResult<Option<&'a View>> {
        let Some(vp) = &site.view else {
            return Ok(None);
        };
        match msg.pointer(vp).and_then(Value::as_str) {
            Some(id) => Ok(Some(need_view(self.doc, id, vp)?)),
            None => Ok(None),
        }
    }

    /// Resolves the selector at `site` of `msg`: the value to write there and the id.
    pub(crate) fn resolve(&self, msg: &Value, site: &Site) -> PResult<(Value, String)> {
        let sel = msg.pointer(&site.path).cloned().unwrap_or(Value::Null);
        let view = match site.kind {
            Kind::Panel | Kind::Layer | Kind::Scale => self.context(msg, site)?,
            _ => None,
        };
        let row = self.resolve_kind(site.kind, &sel, view, &site.path)?;
        Ok((row.value, row.id))
    }

    fn resolve_kind(
        &self,
        kind: Kind,
        sel: &Value,
        view: Option<&View>,
        path: &str,
    ) -> PResult<Row> {
        match kind {
            Kind::Series => {
                let s: SeriesSel = self.parse(kind.def(), sel, path)?;
                let o = self.series_rows(&s, path)?;
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::View => {
                let s: ViewSel = self.parse(kind.def(), sel, path)?;
                let o = self.view_rows(&s, path)?;
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::Panel => {
                let s: PanelSel = self.parse(kind.def(), sel, path)?;
                let o = self.panel_rows(view, &s, path)?;
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::Layer => {
                let s: LayerSel = self.parse(kind.def(), sel, path)?;
                let o = self.layer_rows(view, &s, path, false)?;
                match decide(kind, path, sel, o, s.first, s.latest) {
                    Err(e) if e.code == ErrorCode::NotFound && s.output.is_some() => {
                        Err(self.split_first(view, &s, path).unwrap_or(e))
                    }
                    r => r,
                }
            }
            Kind::Fact => {
                let s: FactSel = self.parse(kind.def(), sel, path)?;
                let o = self.fact_rows(&s, path)?;
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::Finding => {
                let s: FindingSel = self.parse(kind.def(), sel, path)?;
                let o = self.finding_rows(&s, path)?;
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::Capsule => {
                let s: CapsuleSel = self.parse(kind.def(), sel, path)?;
                let o = self.capsule_rows(&s);
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::Proposal => {
                let s: ProposalSel = self.parse(kind.def(), sel, path)?;
                let o = self.proposal_rows(&s);
                decide(kind, path, sel, o, s.first, s.latest)
            }
            Kind::Scale => {
                let s: ScaleSel = self.parse(kind.def(), sel, path)?;
                let pp = ptr(path, "panel");
                let id = match &s.panel {
                    PanelRef::Id(n) => *n,
                    PanelRef::Select(ps) => {
                        let o = self.panel_rows(view, ps, &pp)?;
                        let sel_v = sel.get("panel").cloned().unwrap_or(Value::Null);
                        let r = decide(Kind::Panel, &pp, &sel_v, o, ps.first, ps.latest)?;
                        r.value
                            .as_u64()
                            .and_then(|x| u32::try_from(x).ok())
                            .ok_or_else(|| invalid("panel id out of range", pp.clone()))?
                    }
                };
                let id = format!("y{id}");
                Ok(Row {
                    about: format!("value scale of panel {}", &id[1..]),
                    value: Value::String(id.clone()),
                    id,
                    checks: Vec::new(),
                })
            }
            Kind::Item => {
                const KINDS: [&str; 5] = ["series", "view", "fact", "finding", "proposal"];
                let obj = sel.as_object().filter(|o| o.len() == 1);
                let Some((k, inner)) = obj.and_then(|o| o.iter().next()) else {
                    return Err(E::new(
                        ErrorCode::Schema,
                        "an item selector names its kind with one key, e.g. {\"finding\":{\"kind\":\"breakout\",\"latest\":true}}",
                    )
                    .at(path)
                    .expected(KINDS)
                    .build());
                };
                let ip = ptr(path, k);
                let sub = match k.as_str() {
                    "series" => Kind::Series,
                    "view" => Kind::View,
                    "fact" => Kind::Fact,
                    "finding" => Kind::Finding,
                    "proposal" => Kind::Proposal,
                    _ => {
                        return Err(E::new(ErrorCode::Schema, format!("unknown field `{k}`"))
                            .at(ip)
                            .dym(k, KINDS)
                            .expected(KINDS)
                            .hint("An item selector names its kind: series, view, fact, finding or proposal.")
                            .build())
                    }
                };
                self.resolve_kind(sub, inner, None, &ip)
            }
        }
    }

    // -- hints (welcome.summary, screen) ----------------------------------

    fn ids<F>(&self, f: F) -> Vec<String>
    where
        F: FnOnce() -> PResult<(Outcome, bool, bool)>,
    {
        match f() {
            Ok((o, first, latest)) => pick(o.rows, first, latest, "")
                .map(|v| v.into_iter().map(|r| r.id).collect())
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    fn series_ids(&self, s: &SeriesSel) -> Vec<String> {
        self.ids(|| Ok((self.series_rows(s, "")?, s.first, s.latest)))
    }

    fn view_ids(&self, s: &ViewSel) -> Vec<String> {
        self.ids(|| Ok((self.view_rows(s, "")?, s.first, s.latest)))
    }

    fn panel_ids(&self, v: &View, s: &PanelSel) -> Vec<String> {
        self.ids(|| Ok((self.panel_rows(Some(v), s, "")?, s.first, s.latest)))
    }

    fn layer_ids(&self, v: &View, s: &LayerSel) -> Vec<String> {
        self.ids(|| Ok((self.layer_rows(Some(v), s, "", false)?, s.first, s.latest)))
    }

    fn fact_ids(&self, s: &FactSel) -> Vec<String> {
        self.ids(|| Ok((self.fact_rows(s, "")?, s.first, s.latest)))
    }

    fn finding_ids(&self, s: &FindingSel) -> Vec<String> {
        self.ids(|| Ok((self.finding_rows(s, "")?, s.first, s.latest)))
    }

    /// The first candidate that resolves to exactly `target`; then the first one whose
    /// matches start (`first`) or end (`latest`) with it.
    fn choose<S: Clone>(
        cands: Vec<S>,
        target: &str,
        ids: impl Fn(&S) -> Vec<String>,
        set_first: impl Fn(&mut S),
        set_latest: impl Fn(&mut S),
    ) -> Option<S> {
        let found: Vec<Vec<String>> = cands.iter().map(&ids).collect();
        for (c, m) in cands.iter().zip(&found) {
            if m.len() == 1 && m[0] == target {
                return Some(c.clone());
            }
        }
        for (c, m) in cands.iter().zip(&found) {
            if m.first().is_some_and(|x| x == target) {
                let mut c = c.clone();
                set_first(&mut c);
                return Some(c);
            }
            if m.last().is_some_and(|x| x == target) {
                let mut c = c.clone();
                set_latest(&mut c);
                return Some(c);
            }
        }
        None
    }

    /// A selector that resolves to series `s` now.
    pub(crate) fn series_hint(&self, s: &Series) -> Option<SeriesSel> {
        let mut cands = Vec::new();
        match &s.source {
            SeriesSource::Study { study, params, .. } => {
                cands.push(SeriesSel {
                    study: Some(study.clone()),
                    ..SeriesSel::default()
                });
                cands.push(SeriesSel {
                    study: Some(study.clone()),
                    params: params.clone(),
                    ..SeriesSel::default()
                });
            }
            _ => {
                let kind = series_kind(s);
                cands.push(SeriesSel {
                    kind: Some(kind),
                    ..SeriesSel::default()
                });
                if let Some(sym) = s.symbol() {
                    cands.push(SeriesSel {
                        kind: Some(kind),
                        symbol: Some(sym.to_owned()),
                        ..SeriesSel::default()
                    });
                }
            }
        }
        cands.push(SeriesSel {
            label: Some(series_label(s)),
            ..SeriesSel::default()
        });
        Self::choose(
            cands,
            &s.id,
            |c| self.series_ids(c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }

    fn series_ref_hint(&self, id: &str) -> Option<SeriesRef> {
        let s = self.doc.series.iter().find(|s| s.id == id)?;
        self.series_hint(s).map(SeriesRef::Select)
    }

    /// A selector that resolves to view `v` now.
    pub(crate) fn view_hint(&self, v: &View) -> Option<ViewSel> {
        let mut cands = vec![ViewSel::default()];
        if let Some(t) = view_title(self.doc, v) {
            cands.push(ViewSel {
                title: Some(t),
                ..ViewSel::default()
            });
        }
        for l in &v.layers {
            if let Some(r) = l.series.as_deref().and_then(|s| self.series_ref_hint(s)) {
                cands.push(ViewSel {
                    shows: Some(Box::new(r)),
                    ..ViewSel::default()
                });
            }
        }
        Self::choose(
            cands,
            &v.id,
            |c| self.view_ids(c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }

    /// A selector that resolves to panel `id` of view `v` now.
    pub(crate) fn panel_hint(&self, v: &View, id: u32) -> Option<PanelSel> {
        let p = v.panel(id)?;
        let layers: Vec<&qp_schema::Layer> = v.layers.iter().filter(|l| l.panel == id).collect();
        let mut cands = Vec::new();
        if let Some(n) = &p.name {
            cands.push(PanelSel {
                name: Some(n.clone()),
                ..PanelSel::default()
            });
        }
        if layers.iter().any(|l| l.mark == Mark::Candle) {
            cands.push(PanelSel {
                mark: Some(Mark::Candle),
                ..PanelSel::default()
            });
        }
        for l in &layers {
            if let Some(r) = l.series.as_deref().and_then(|s| self.series_ref_hint(s)) {
                cands.push(PanelSel {
                    shows: Some(Box::new(r)),
                    ..PanelSel::default()
                });
            }
        }
        for l in &layers {
            cands.push(PanelSel {
                mark: Some(l.mark),
                ..PanelSel::default()
            });
        }
        Self::choose(
            cands,
            &id.to_string(),
            |c| self.panel_ids(v, c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }

    /// A selector that resolves to layer `l` of view `v` now.
    pub(crate) fn layer_hint(&self, v: &View, l: &qp_schema::Layer) -> Option<LayerSel> {
        let id = l.id.as_deref()?;
        let mut cands = Vec::new();
        let shows = l.series.as_deref().and_then(|s| self.series_ref_hint(s));
        if let Some(r) = &shows {
            let base = LayerSel {
                shows: Some(Box::new(r.clone())),
                ..LayerSel::default()
            };
            cands.push(base.clone());
            cands.push(LayerSel {
                mark: Some(l.mark),
                ..base.clone()
            });
            if let Some(Position::Name(n)) = &l.encoding.y {
                if crate::canon::numeric(n).is_none() {
                    cands.push(LayerSel {
                        output: Some(n.clone()),
                        ..base.clone()
                    });
                }
            }
        }
        for c in layer_constants(l) {
            cands.push(LayerSel {
                mark: Some(l.mark),
                value: Some(c),
                ..LayerSel::default()
            });
        }
        if let Some(f) = l.fact.as_deref() {
            if let Some(h) = self
                .doc
                .facts
                .iter()
                .find(|x| x.id == f)
                .and_then(|x| self.fact_hint(x))
            {
                cands.push(LayerSel {
                    fact: Some(Box::new(FactRef::Select(h))),
                    ..LayerSel::default()
                });
            }
        }
        if let Some(f) = l.finding.as_deref() {
            if let Some(h) = self
                .doc
                .findings
                .iter()
                .find(|x| x.id == f)
                .and_then(|x| self.finding_hint(x))
            {
                cands.push(LayerSel {
                    finding: Some(Box::new(FindingRef::Select(h))),
                    ..LayerSel::default()
                });
            }
        }
        if let Some(t) = &l.label {
            cands.push(LayerSel {
                label: Some(t.clone()),
                ..LayerSel::default()
            });
        }
        cands.push(LayerSel {
            mark: Some(l.mark),
            ..LayerSel::default()
        });
        Self::choose(
            cands,
            id,
            |c| self.layer_ids(v, c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }

    /// A selector that resolves to fact `f` now.
    pub(crate) fn fact_hint(&self, f: &Fact) -> Option<FactSel> {
        let kind = FactSel {
            kind: Some(f.kind.clone()),
            ..FactSel::default()
        };
        let cands = vec![
            kind.clone(),
            FactSel {
                latest: true,
                ..kind.clone()
            },
            FactSel {
                first: true,
                ..kind.clone()
            },
            FactSel {
                bar: Some(f.bar),
                ..kind.clone()
            },
        ];
        Self::choose(
            cands,
            &f.id,
            |c| self.fact_ids(c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }

    /// A selector that resolves to finding `f` now.
    pub(crate) fn finding_hint(&self, f: &Finding) -> Option<FindingSel> {
        let doc = self.doc;
        let latest = finding_facts(doc, f).max_by_key(|x| x.bar)?;
        let kind = FindingSel {
            kind: Some(latest.kind.clone()),
            ..FindingSel::default()
        };
        let mut cands = vec![
            kind.clone(),
            FindingSel {
                latest: true,
                ..kind.clone()
            },
            FindingSel {
                first: true,
                ..kind.clone()
            },
        ];
        if let Some(h) = self.fact_hint(latest) {
            cands.push(FindingSel {
                fact: Some(Box::new(FactRef::Select(h))),
                ..FindingSel::default()
            });
        }
        Self::choose(
            cands,
            &f.id,
            |c| self.finding_ids(c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }
}
