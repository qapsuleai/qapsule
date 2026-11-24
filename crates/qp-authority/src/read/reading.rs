//! Semantic reading from the scene description (`docs/GRAMMAR.md` §16): the compiled
//! scene of a view (`Session::scene`), the R2 fields of `screen` and the point query
//! `hit`. Nothing here renders: every answer is read from `SceneDesc`
//! (`panels`/`layers`/`items`) and its scale mappings.

use std::collections::BTreeMap;

use qp_grammar::scale::{slot_at, value_at};
use qp_grammar::{compile, CompileOptions};
use qp_look::{ErrorKind, GrammarError};
use qp_schema::protocol::{
    ErrorCode, HitInfo, HitItem, LayerInfo, ProtocolError, ScaleInfo, SurfaceKind, Viewport,
};
use qp_schema::scene::{SceneDesc, ScenePanel};
use qp_schema::{Series, View};

use crate::session::{Conn, Session};
use qp_ops::expand::{need_view, view_bar_series};
use qp_ops::provider::{warm, DocProvider};
use qp_plane::error::{PResult, E};

/// Panel id → (name, scale if compiled, layers) of a screen.
pub(crate) type ScreenPanels = BTreeMap<u32, (Option<String>, Option<ScaleInfo>, Vec<LayerInfo>)>;

/// Default frame width of the point query (`docs/PROTOCOL.md` §5.4).
pub const HIT_WIDTH: u32 = 1200;
/// Default frame height of the point query.
pub const HIT_HEIGHT: u32 = 600;

/// The frame `screen` compiles a surface's view at (its plot, not the whole device
/// screen): desktop (and any surface without a screen) 1200x600; phone 390x300 (a
/// 390x844 phone, chart area in portrait); watch 200x242 (a 198x242 watch, widened to the
/// compiler's 200-pixel minimum). Fixed per surface kind so that the answer does not
/// depend on the device's reported resolution.
#[must_use]
pub fn screen_frame(kind: SurfaceKind) -> (u32, u32) {
    match kind {
        SurfaceKind::Phone => (390, 300),
        SurfaceKind::Watch => (200, 242),
        SurfaceKind::Desktop | SurfaceKind::Voice | SurfaceKind::Headless => (1200, 600),
    }
}

/// A compile error as a protocol error at `path`.
fn grammar_error(e: &GrammarError, path: &str) -> ProtocolError {
    let code = match e.kind {
        ErrorKind::NotFound => ErrorCode::NotFound,
        ErrorKind::Invalid => ErrorCode::Invalid,
        ErrorKind::NoOrigin => ErrorCode::NoOrigin,
    };
    E::new(code, e.to_string()).at(path).build()
}

/// The R2 description of one compiled panel: scale and layers.
pub(crate) fn panel_layers(view: &View, p: &ScenePanel) -> (ScaleInfo, Vec<LayerInfo>) {
    let scale = ScaleInfo {
        id: view
            .panel(p.id)
            .and_then(|vp| vp.y.id.clone())
            .unwrap_or_else(|| format!("y{}", p.id)),
        kind: p.y.kind,
        domain: [p.y.min, p.y.max],
    };
    let layers = p
        .layers
        .iter()
        .map(|l| LayerInfo {
            id: l.id.clone(),
            mark: l.mark,
            data: l.data.clone(),
            fields: l.fields.clone(),
            label: l.label.clone(),
            constants: l.constants.clone(),
            items: l.items.len() as u64,
            last: l.items.last().map(|i| i.values.clone()).unwrap_or_default(),
            select: None,
            binding: None,
        })
        .collect();
    (scale, layers)
}

/// What is drawn at pixel (`x`, `y`) of `scene`: panel, bar slot, time, the panel's value
/// at `y` and the topmost item whose hit box contains the pixel (paint order: the last
/// one wins). `times` are the bar times of the view's first bar series; slots past them
/// (future points) take their time from the items drawn there.
#[must_use]
pub fn hit_scene(scene: &SceneDesc, x: f64, y: f64, times: &[i64]) -> HitInfo {
    let mut out = HitInfo {
        view: scene.view.clone(),
        panel: None,
        bar: None,
        t: None,
        value: None,
        item: None,
        term: None,
    };
    let inside = |left: i32, top: i32, right: i32, bottom: i32| {
        x >= f64::from(left) && x < f64::from(right) && y >= f64::from(top) && y < f64::from(bottom)
    };
    // (R5/m17-int, K49) A glossary term drawn under the pixel (last drawn wins).
    out.term = scene
        .terms
        .iter()
        .rev()
        .find(|t| inside(t.hit.left, t.hit.top, t.hit.right, t.hit.bottom))
        .cloned();
    let Some(panel) = scene
        .panels
        .iter()
        .find(|p| inside(p.rect.left, p.rect.top, p.rect.right, p.rect.bottom))
    else {
        return out;
    };
    out.panel = Some(panel.id);
    out.value = Some(value_at(&panel.y, y));
    out.bar = slot_at(&scene.x, x).filter(|s| *s <= scene.x.to_bar);
    if let Some(slot) = out.bar {
        out.t = usize::try_from(slot)
            .ok()
            .and_then(|i| times.get(i).copied())
            .or_else(|| {
                scene
                    .panels
                    .iter()
                    .flat_map(|p| &p.layers)
                    .flat_map(|l| &l.items)
                    .find(|i| i.bar == Some(slot) && i.t.is_some())
                    .and_then(|i| i.t)
            });
    }
    for l in &panel.layers {
        for it in &l.items {
            let h = &it.hit;
            if inside(h.left, h.top, h.right, h.bottom) {
                out.item = Some(HitItem {
                    layer: l.id.clone(),
                    mark: l.mark,
                    data: l.data.clone(),
                    bar: it.bar,
                    t: it.t,
                    values: it.values.clone(),
                    constants: l.constants.clone(),
                });
            }
        }
    }
    out
}

impl Session {
    /// The grammar compiler's data source over this session's document and values cache
    /// (study values computed with `qp-compute` and cached first).
    pub fn provider(&mut self) -> DocProvider<'_> {
        warm(rd!(self), &mut self.cache);
        DocProvider::new(rd!(self), &self.cache).with_headlines(&self.headlines)
    }

    /// Compiles view `view_id` into its scene description with `opts` (frame size,
    /// transient range and emphasis, theme). Headless; nothing is rendered.
    ///
    /// # Errors
    /// `not_found` for an unknown view; the compiler's error (layer, pointer, reason) as
    /// `invalid`, `not_found` or `no_origin`.
    pub fn scene(
        &mut self,
        view_id: &str,
        opts: &CompileOptions,
    ) -> Result<SceneDesc, ProtocolError> {
        warm(rd!(self), &mut self.cache);
        let view = need_view(rd!(self), view_id, "/view")?;
        compile(
            view,
            &DocProvider::new(rd!(self), &self.cache).with_headlines(&self.headlines),
            opts,
        )
        .map_err(|e| grammar_error(&e, "/view"))
    }

    /// Compiles a view as connection `conn` sees it: `view_id` (default: the connection's
    /// current view) at the connection's visible range of that view, unless `opts.range`
    /// is set (see [`Session::hit_range`]).
    ///
    /// # Errors
    /// Unknown connection or view; compile errors as in [`Session::scene`].
    pub fn scene_for(
        &mut self,
        conn: &str,
        view_id: Option<&str>,
        opts: &CompileOptions,
    ) -> Result<SceneDesc, ProtocolError> {
        let ci = self
            .conns
            .iter()
            .position(|c| c.id == conn)
            .ok_or_else(|| {
                E::new(ErrorCode::NotFound, format!("unknown connection `{conn}`")).build()
            })?;
        let vid = self.view_of(ci, view_id, "/view")?;
        let mut o = opts.clone();
        if o.range.is_none() {
            o.range = self.hit_range(&self.conns[ci], &vid);
        }
        self.scene(&vid, &o)
    }

    /// The view a reading request is about: `view_id`, else the connection's current
    /// view (its last viewport, else the document's first view).
    fn view_of(&self, ci: usize, view_id: Option<&str>, path: &str) -> PResult<String> {
        if let Some(v) = view_id {
            return Ok(need_view(&self.doc, v, path)?.id.clone());
        }
        self.conn_viewport(&self.conns[ci])
            .map(|vp| vp.view)
            .ok_or_else(|| {
                E::new(ErrorCode::NotFound, "the document has no view")
                    .at(path)
                    .hint("Name a view with `view`.")
                    .build()
            })
    }

    /// Visible range of `view_id` for a reading by `c`: the connection's viewport of that
    /// view (for a surface with a screen: the last one set by input, else the surface's
    /// initial range, as `screen` reports it); for a connection without a screen, a
    /// viewport it set by input, else `None` (the compiler then takes the view's
    /// persistent range, else all bars).
    pub(crate) fn hit_range(&self, c: &Conn, view_id: &str) -> Option<(u64, u64)> {
        let view = rd!(self).views.iter().find(|v| v.id == view_id)?;
        if c.has_screen() || c.viewports.contains_key(view_id) {
            self.range_of(c, view)
        } else {
            None
        }
    }

    /// The point query (`docs/PROTOCOL.md` §5.4 `hit`).
    pub(crate) fn hit(
        &mut self,
        ci: usize,
        view: Option<&str>,
        x: f64,
        y: f64,
        size: (Option<u32>, Option<u32>),
    ) -> PResult<HitInfo> {
        let qp = "/query";
        let vid = self.view_of(ci, view, &format!("{qp}/view"))?;
        let mut opts =
            qp_designs::options(size.0.unwrap_or(HIT_WIDTH), size.1.unwrap_or(HIT_HEIGHT));
        opts.range = self.hit_range(&self.conns[ci], &vid);
        warm(rd!(self), &mut self.cache);
        let view = need_view(rd!(self), &vid, &format!("{qp}/view"))?;
        // Layers whose data the asker may not read are not drawn (§18.3); a view whose
        // bar series is among them cannot be read at all.
        let asker = self.conns[ci].principal.clone();
        let blocked = self.blocked_layers(view, &asker);
        let filtered;
        let view = if blocked.is_empty() {
            view
        } else {
            if let Some(bs) = qp_ops::expand::view_bar_series_any(&self.doc, view) {
                if let Some(b) = self.plane.gate(&self.doc, Some(&asker), &bs.id) {
                    return Err(b.error(&bs.id, &format!("{qp}/view")));
                }
            }
            filtered = without_layers(view, &blocked);
            &filtered
        };
        let scene = compile(
            view,
            &DocProvider::new(rd!(self), &self.cache).with_headlines(&self.headlines),
            &opts,
        )
        .map_err(|e| grammar_error(&e, qp))?;
        let times: Vec<i64> = view_bar_series(rd!(self), view)
            .and_then(Series::bars)
            .map(|b| b.iter().map(|b| b.t).collect())
            .unwrap_or_default();
        Ok(hit_scene(&scene, x, y, &times))
    }

    /// The R2 panel fields of a screen (panel id → name, scale, layers), compiled at the
    /// connection's viewport and its surface's frame. Empty when the view does not
    /// compile (the RD1 fields are still answered). Call after [`warm`].
    pub(crate) fn screen_panels(&self, c: &Conn, vp: &Viewport, asker: &str) -> ScreenPanels {
        let Some(view) = self.doc.views.iter().find(|v| v.id == vp.view) else {
            return BTreeMap::new();
        };
        let blocked = self.blocked_layers(view, asker);
        if blocked.is_empty() {
            return self
                .compiled_panels(c, vp, view)
                .into_iter()
                .map(|(k, (n, s, l))| (k, (n, Some(s), l)))
                .collect();
        }
        // §18.3: the asker's unreadable layers are not drawn; they are listed with their
        // binding state, in paint order, and the rest of the view compiles without them.
        let filtered = without_layers(view, &blocked);
        let mut out: ScreenPanels = if filtered.layers.is_empty() {
            BTreeMap::new()
        } else {
            self.compiled_panels(c, vp, &filtered)
                .into_iter()
                .map(|(k, (n, s, l))| (k, (n, Some(s), l)))
                .collect()
        };
        for (i, l) in view.layers.iter().enumerate() {
            let Some((state, series)) = blocked.get(&i) else {
                continue;
            };
            let e = out.entry(l.panel).or_insert_with(|| {
                (
                    view.panel(l.panel).and_then(|p| p.name.clone()),
                    None,
                    Vec::new(),
                )
            });
            let data = if let Some(f) = &l.fact {
                qp_schema::scene::DataSource::Fact { fact: f.clone() }
            } else if let Some(f) = &l.finding {
                qp_schema::scene::DataSource::Finding { finding: f.clone() }
            } else {
                qp_schema::scene::DataSource::Series {
                    series: series.clone(),
                }
            };
            let info = LayerInfo {
                id: l.id.clone().unwrap_or_else(|| format!("l{i}")),
                mark: l.mark,
                data,
                fields: Vec::new(),
                label: None,
                constants: BTreeMap::new(),
                items: 0,
                last: BTreeMap::new(),
                select: None,
                binding: Some(*state),
            };
            // Paint order: before the first drawn layer that comes later in the view.
            let later: Vec<String> = view.layers[i + 1..]
                .iter()
                .filter_map(|x| x.id.clone())
                .collect();
            let pos =
                e.2.iter()
                    .position(|x| later.contains(&x.id))
                    .unwrap_or(e.2.len());
            e.2.insert(pos, info);
        }
        out
    }

    /// Index → (state, data series) of the layers of `view` whose data `asker` may not
    /// read now (§18.3).
    pub(crate) fn blocked_layers(
        &self,
        view: &View,
        asker: &str,
    ) -> BTreeMap<usize, (qp_schema::data::BindingState, String)> {
        let mut out = BTreeMap::new();
        if self.plane.series.is_empty() {
            return out;
        }
        for (i, l) in view.layers.iter().enumerate() {
            if let Some(b) = self.plane.gate_layer(&self.doc, Some(asker), l) {
                // The layer's data source (what `screen` names), not the series that
                // blocked it.
                let series = l.series.clone().unwrap_or(b.series);
                out.insert(i, (b.state, series));
            }
        }
        out
    }

    fn compiled_panels(
        &self,
        c: &Conn,
        vp: &Viewport,
        view: &View,
    ) -> BTreeMap<u32, (Option<String>, ScaleInfo, Vec<LayerInfo>)> {
        let (w, h) = screen_frame(c.surface.kind);
        let mut opts = qp_designs::options(w, h);
        opts.range = Some((vp.from_bar, vp.to_bar));
        if c.profile.max_points > 0 {
            opts.max_points = Some(u64::from(c.profile.max_points));
        }
        let Ok(scene) = compile(
            view,
            &DocProvider::new(rd!(self), &self.cache).with_headlines(&self.headlines),
            &opts,
        ) else {
            return BTreeMap::new();
        };
        scene
            .panels
            .iter()
            .map(|p| {
                let (scale, mut layers) = panel_layers(view, p);
                let r = qp_ops::select::Resolver {
                    doc: rd!(self),
                    root: None,
                };
                for li in &mut layers {
                    li.select = view.layer(&li.id).and_then(|l| r.layer_hint(view, l));
                }
                (p.id, (p.name.clone(), scale, layers))
            })
            .collect()
    }

    pub(crate) fn warm_values(&mut self) {
        warm(rd!(self), &mut self.cache);
    }
}

/// `view` without the layers at `skip` (indexes).
pub(crate) fn without_layers<T>(view: &View, skip: &BTreeMap<usize, T>) -> View {
    let mut v = view.clone();
    v.layers = view
        .layers
        .iter()
        .enumerate()
        .filter(|(i, _)| !skip.contains_key(i))
        .map(|(_, l)| l.clone())
        .collect();
    v
}
