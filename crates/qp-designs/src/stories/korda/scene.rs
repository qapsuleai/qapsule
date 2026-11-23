//! `korda`'s whole scene ([`scene`]): the price chart and the lifelines under it.

use super::*;

/// The `korda` story chart (`super::StoryDesign::compile`).
///
/// # Errors
/// The compile's errors (unknown look or theme, a view that does not compile).
#[allow(clippy::too_many_lines)]
pub fn scene(
    view: &View,
    p: &dyn DataProvider,
    opts: &CompileOptions,
    inp: &StoryInput<'_>,
) -> Result<SceneDesc, GrammarError> {
    let look = qp_look::look::by_name(opts.look.as_deref())?;
    let theme = look.theme(opts.theme.as_deref().or(view.theme.as_deref()))?;
    let Some(bs) = first_bar_series(view, p) else {
        return compile(view, p, opts);
    };
    let times: Vec<i64> = p
        .bars(&bs)
        .map(|b| b.iter().map(|x| x.t).collect())
        .unwrap_or_default();
    let Some((a, b)) = visible(view, &times, opts) else {
        return compile(view, p, opts);
    };
    let now = times.len() as u64 - 1;
    let b_last = b;
    // The horizontal mapping the compile will use (same formula, `compile.rs`).
    let (wi, hi) = (opts.width.cast_signed(), opts.height.cast_signed());
    let [m_left, m_right, m_top, m_bottom] = look.space.plot;
    let plot_w = wi - m_right - m_left;
    #[allow(clippy::cast_precision_loss)]
    let xm = XMap {
        from_bar: a,
        to_bar: b_last,
        left: f64::from(m_left),
        slot: f64::from(plot_w) / (b_last - a + 1) as f64,
    };
    // The stories in view, by group.
    let mut sel: Option<Life<'_>> = None;
    let mut live: Vec<Life<'_>> = Vec::new();
    let mut closed: Vec<Life<'_>> = Vec::new();
    let mut hidden = 0usize;
    let mut rows: Vec<&StoryRow> = inp.rows.iter().filter(|r| r.series == bs).collect();
    rows.sort_by(|x, y| {
        x.born_bar
            .cmp(&y.born_bar)
            .then(x.born_t.cmp(&y.born_t))
            .then(x.id.cmp(&y.id))
    });
    for r in rows {
        let open = r.closed_bar.is_none();
        let end = r.closed_bar.unwrap_or_else(|| r.last_bar.max(now));
        if r.born_bar > b || end < a {
            continue;
        }
        let l = Life {
            row: r,
            start: r.born_bar,
            end,
            open,
        };
        if r.finding == inp.focus.unwrap_or("") {
            sel = Some(l);
        } else if open {
            live.push(l);
        } else if !inp.hide_closed {
            closed.push(l);
        } else {
            hidden += 1;
        }
    }
    let closed_n = closed.len() + hidden + usize::from(sel.is_some_and(|s| !s.open));
    let live_n = live.len() + usize::from(sel.is_some_and(|s| s.open));
    if sel.is_none() && live.is_empty() && closed.is_empty() && hidden == 0 {
        // Nothing to tell (or the list has not arrived yet): the chart alone.
        return compile(view, p, opts);
    }
    // The story panel's height: what the plan wants, at most a share of the plot.
    let base_panels = if view.panels.is_empty() {
        1
    } else {
        view.panels.len()
    };
    let gap = look.space.panel_gap;
    let avail = hi - m_top - m_bottom - gap * lanes_i(base_panels);
    #[allow(clippy::cast_possible_truncation)]
    let max_h = ((f64::from(avail) * MAX_SHARE).floor() as i32).max(HEADER_PX + SELECTED_PITCH);
    let plan = plan(sel, &live, &closed, &xm, max_h);
    let want = plan.height.min(avail - 10);
    // The view with the story panel appended (an empty panel: the compile draws its
    // frame and the time grid; its value grid is removed below).
    let mut v = view.clone();
    let mut panels: Vec<Panel> = if v.panels.is_empty() {
        vec![Panel {
            id: Some(0),
            name: None,
            weight: None,
            y: Scale::new(ScaleKind::Linear),
        }]
    } else {
        v.panels.clone()
    };
    let total: f64 = panels
        .iter()
        .enumerate()
        .map(|(i, q)| q.weight.unwrap_or(if i == 0 { 3.0 } else { 1.0 }))
        .sum();
    for (i, q) in panels.iter_mut().enumerate() {
        q.weight = Some(q.weight.unwrap_or(if i == 0 { 3.0 } else { 1.0 }));
    }
    let id = panels
        .iter()
        .filter_map(|q| q.id)
        .max()
        .map_or(0, |m| m + 1);
    panels.push(Panel {
        id: Some(id),
        name: Some("stories".to_owned()),
        weight: Some(total * f64::from(want) / f64::from((avail - want).max(1))),
        y: Scale {
            kind: ScaleKind::Linear,
            domain: Some([0.0, 1.0]),
            id: None,
        },
    });
    v.panels = panels;
    v.y.clear();
    let mut o = opts.clone();
    o.range = Some((a, b));
    let mut sc = compile(&v, p, &o)?;
    let Some(pi) = sc.panels.iter().position(|q| q.id == id) else {
        return Ok(sc);
    };
    let rc = sc.panels[pi].rect;
    strip_value_grid(&mut sc.under, rc);
    let price_top = sc.panels.first().map_or(rc.top, |q| q.rect.top);
    let d = Draw {
        x: sc.x,
        rc,
        theme,
        look,
        roles: opts.roles,
        a,
        b: sc.x.to_bar,
        price_top,
        lang: inp.lang,
        stamp_of: inp.stamp_of,
    };
    let mut layers: Vec<SceneLayer> = Vec::new();
    // Lanes, top to bottom: selected, open, closed (closed painted first).
    let mut y = rc.top + HEADER_PX;
    let mut sel_layer = None;
    if let Some(s) = plan.sel {
        sel_layer = Some(d.lane(&s, y, SELECTED_PITCH, Role::Selected, plan.mode, None));
        y += SELECTED_PITCH + GROUP_GAP;
    }
    let live_y = y;
    if plan.live_lanes > 0 {
        y += lanes_i(plan.live_lanes) * plan.live_pitch + GROUP_GAP;
    }
    let closed_y = y;
    let small = plan.mode != Mode::Full;
    for (l, lane) in &plan.closed {
        let next = next_start(&plan.closed, l, *lane, &d);
        let ly = closed_y + lanes_i(*lane) * plan.closed_pitch;
        layers.push(d.lane(l, ly, plan.closed_pitch, Role::Closed, mode_of(small), next));
    }
    if plan.mode == Mode::Summary && !plan.hist.is_empty() {
        let hy = if plan.live_lanes > 0 { y } else { live_y };
        layers.extend(d.histogram(&plan.hist, hy, rc.bottom - 2, inp.focus.unwrap_or("")));
    }
    for (l, lane) in &plan.live {
        let next = next_start(&plan.live, l, *lane, &d);
        let ly = live_y + lanes_i(*lane) * plan.live_pitch;
        layers.push(d.lane(l, ly, plan.live_pitch, Role::Live, mode_of(small), next));
    }
    if let Some(l) = sel_layer {
        layers.push(l);
    }
    layers.push(d.header(live_n, closed_n, hidden, !inp.hide_closed));
    sc.panels[pi].layers = layers;
    Ok(sc)
}
