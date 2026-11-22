//! `pavka`'s story part ([`extend`]): one function, alone in its file.

use super::*;

/// The story part of the scene; see the module documentation.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn extend(
    look: &Look,
    theme_name: Option<&str>,
    mut scene: SceneDesc,
    h: u32,
    input: &StoryInput<'_>,
) -> SceneDesc {
    let sl = &look.story;
    if h <= scene.height {
        return scene;
    }
    let theme: Theme = look.theme(theme_name).unwrap_or(look.dark);
    let w = scene.width.cast_signed();
    let full_h = h.cast_signed();
    let [m_left, m_right, _, _] = look.space.plot;
    let (left, right) = (m_left, w - m_right);
    let top0 = scene.height.cast_signed();
    let x = scene.x;
    let last = scene.bars.saturating_sub(1);
    let wd = words(input.lang);
    let label_px = look.types.label.px;
    let body_px = look.types.body.px;
    #[allow(clippy::cast_possible_truncation)]
    let (lp, bp) = (label_px.ceil() as i32, body_px.ceil() as i32);
    let line1 = top0 + 6 + lp;
    let line2 = line1 + 6 + bp;
    let line3 = line2 + 4 + lp;
    let lanes_top = line3 + 8;
    // A lane's pixel ends: birth column to closing column, at least two markers apart (a
    // story closed a bar after its birth still shows both shapes).
    #[allow(clippy::cast_possible_truncation)]
    let gap = 2 * (sl.mark.ceil() as i32) + 3;
    let ends = |born: u64, end: u64| -> (i32, i32) {
        #[allow(clippy::cast_possible_truncation)]
        let (a, b) = (
            slot_x(&x, born.max(x.from_bar)) as i32,
            slot_x(&x, end.min(x.to_bar)) as i32,
        );
        if born >= x.from_bar && end <= x.to_bar {
            (a, b.max(a + gap).min(right))
        } else {
            (a, b)
        }
    };
    let bottom = full_h - 6;

    // Visible stories (life meets the range), counted before the K54 filter.
    let mut vis: Vec<Placed<'_>> = Vec::new();
    let (mut n_live, mut n_held, mut n_failed, mut n_coin, mut n_all) = (0, 0, 0, 0, 0);
    for r in input.rows {
        let live = r.closed_bar.is_none();
        let end = r.closed_bar.unwrap_or_else(|| r.last_bar.max(last));
        if r.born_bar > x.to_bar || end < x.from_bar {
            continue;
        }
        n_all += 1;
        match r.outcome {
            None => n_live += 1,
            Some(StoryOutcome::Realized) => n_held += 1,
            Some(StoryOutcome::Invalidated | StoryOutcome::Expired) => n_failed += 1,
            Some(StoryOutcome::Retracted) => {}
        }
        if r.coin_flip {
            n_coin += 1;
        }
        let focus = input.focus == Some(r.finding.as_str());
        if input.hide_closed && !live && !focus {
            continue;
        }
        let (a, b) = ends(r.born_bar, end);
        #[allow(clippy::cast_possible_truncation)]
        let m = sl.mark.ceil() as i32;
        let tail = if live { m + 4 } else { m + 1 };
        vis.push(Placed {
            row: r,
            end,
            span: (a - m - 1, b + tail),
            ends: (a, b),
            live,
            focus,
        });
    }
    // Placement order: the selected story, live ones (newest first), closed ones (most
    // recently closed first).
    let mut order: Vec<usize> = (0..vis.len()).collect();
    order.sort_by(|&i, &j| {
        let (a, b) = (&vis[i], &vis[j]);
        b.focus
            .cmp(&a.focus)
            .then(b.live.cmp(&a.live))
            .then(b.end.cmp(&a.end))
            .then(b.row.born_bar.cmp(&a.row.born_bar))
            .then(a.row.id.cmp(&b.row.id))
    });
    // First fit, uncapped: lanes are opened in placement order and a story never moves
    // into a later lane, so a capacity of `c` lanes is the first `c` lanes of this result
    // (the rest are clustered). Each lane keeps its spans sorted by start (overlap test
    // by binary search). Live stories keep a lane each.
    let mut lanes: Vec<(bool, Vec<usize>)> = Vec::new();
    let mut spans: Vec<Vec<(i32, i32)>> = Vec::new();
    for &i in &order {
        let p = &vis[i];
        let (s0, s1) = p.span;
        let mut placed = false;
        if !p.live {
            for (l, sp) in lanes.iter_mut().zip(spans.iter_mut()) {
                if l.0 {
                    continue;
                }
                let at = sp.partition_point(|q| q.0 < s0);
                let left_ok = at == 0 || sp[at - 1].1 + 4 <= s0;
                let right_ok = at == sp.len() || s1 + 4 <= sp[at].0;
                if left_ok && right_ok {
                    sp.insert(at, (s0, s1));
                    l.1.push(i);
                    placed = true;
                    break;
                }
            }
        }
        if !placed {
            lanes.push((p.live, vec![i]));
            spans.push(vec![(s0, s1)]);
        }
    }
    let avail = bottom - lanes_top;
    // Thin out first (lanes down to `lane_min`), then cluster what still does not fit.
    let n = i32::try_from(lanes.len()).unwrap_or(i32::MAX).max(1);
    let mut over: Vec<usize> = Vec::new();
    let lane_h = if n <= avail / sl.lane_h {
        (avail / n).clamp(sl.lane_h, sl.lane_max)
    } else if n <= avail / sl.lane_min {
        (avail / n).clamp(sl.lane_min, sl.lane_h)
    } else {
        let cap = usize::try_from(((avail - sl.cluster_h) / sl.lane_min).max(1)).unwrap_or(1);
        for l in lanes.drain(cap.min(lanes.len())..) {
            over.extend(l.1);
        }
        sl.lane_min
    };
    // Live lanes on top, then closed (each group in placement order).
    lanes.sort_by_key(|l| !l.0);
    let n_lanes = i32::try_from(lanes.len()).unwrap_or(0);

    let mut layers: Vec<SceneLayer> = Vec::new();
    let mut under: Vec<Prim> = Vec::new();

    // Summary line: counts, each with its shape (the legend).
    let mut sum: Vec<Prim> = Vec::new();
    // (R5/S) The checkbox and its word sit in the right margin; a word wider than the
    // margin moves the checkbox left so that the word is whole (`r4`: it fits, as before).
    let closed_w = text_w(look, "label", wd.closed);
    let toggle_x = if closed_w <= w - (right + 4) - 14 {
        right + 4
    } else {
        w - 14 - closed_w - 2
    };
    let limit = toggle_x - 8;
    let mut cx = left;
    let first = (wd.stories)(n_all);
    sum.push(text(look, "label", cx, line1, first.clone(), theme.text));
    cx += text_w(look, "label", &first);
    #[allow(clippy::cast_precision_loss)]
    let ly = (line1 - lp / 2 + 1) as f32;
    let mut segs: Vec<(Option<Shape>, String)> =
        vec![(Some(Shape::Live), format!("{n_live} {}", wd.live))];
    if input.hide_closed {
        segs.push((None, wd.hidden.to_owned()));
    } else {
        segs.push((Some(Shape::Realized), format!("{n_held} {}", wd.held)));
        segs.push((Some(Shape::Failed), format!("{n_failed} {}", wd.failed)));
    }
    if n_coin > 0 {
        segs.push((Some(Shape::Coin), format!("{n_coin} {}", wd.coin)));
    }
    for (s, t) in segs {
        let need = 8 + if s.is_some() { 11 } else { 0 } + text_w(look, "label", &t);
        if cx + need > limit {
            break;
        }
        cx += 8;
        if let Some(s) = s {
            #[allow(clippy::cast_precision_loss)]
            cap(
                &mut sum,
                s,
                (cx + 4) as f32,
                ly,
                4.0,
                theme.text,
                theme.bg,
                sl.dash,
            );
            cx += 11;
        }
        sum.push(text(look, "label", cx, line1, t.clone(), theme.muted));
        cx += text_w(look, "label", &t);
    }
    // Focus line: the selected story in words.
    if let Some(f) = vis.iter().find(|p| p.focus) {
        let fc = &f.row.face;
        let head = format!("{} · {}", fc.event, fc.when);
        let mut more = format!("{} · {}", fc.validity_phrase, fc.ledger_short);
        if f.row.coin_flip {
            more.push_str(" · ");
            more.push_str(wd.coin);
        }
        sum.extend(rect(left, line2 - bp + 2, left + 6, line3, theme.focus));
        let room = w - left - 8 - 10;
        sum.push(text(
            look,
            "body",
            left + 10,
            line2,
            fit(look, "body", &head, room),
            theme.text,
        ));
        sum.push(text(
            look,
            "label",
            left + 10,
            line3,
            fit(look, "label", &more, room),
            theme.muted,
        ));
    }
    layers.push(SceneLayer {
        id: SUMMARY_LAYER.to_owned(),
        mark: Mark::Text,
        data: DataSource::Constant {},
        fields: Vec::new(),
        constants: BTreeMap::new(),
        label: None,
        items: Vec::new(),
        prims: sum,
        skipped: 0,
    });

    // The closed-stories checkbox (K54), in the right margin of the summary line.
    {
        let mut prims = Vec::new();
        let (bx, by) = (toggle_x, line1 - lp + 1);
        prims.extend(rect(bx, by, bx + 10, by + 10, theme.muted));
        if input.hide_closed {
            prims.extend(rect(bx + 2, by + 2, bx + 8, by + 8, theme.bg));
        }
        prims.push(text(
            look,
            "label",
            bx + 14,
            line1,
            fit(look, "label", wd.closed, w - bx - 14),
            theme.muted,
        ));
        layers.push(SceneLayer {
            id: TOGGLE_LAYER.to_owned(),
            mark: Mark::Text,
            data: DataSource::Constant {},
            fields: Vec::new(),
            constants: BTreeMap::new(),
            label: Some(wd.closed.to_owned()),
            items: vec![Item {
                bar: None,
                t: None,
                values: BTreeMap::from([(
                    "hide_closed".to_owned(),
                    if input.hide_closed { 1.0 } else { 0.0 },
                )]),
                hit: PixelRect {
                    left: toggle_x - 4,
                    top: top0,
                    right: w,
                    bottom: line1 + 8,
                },
                bars: None,
            }],
            prims,
            skipped: 0,
        });
    }

    // Lanes.
    let live_word = fit(look, "label", wd.live, w - right - 6);
    let mut focus_layer: Option<SceneLayer> = None;
    let mut live_end = None;
    for (li, (is_live, members)) in lanes.iter().enumerate() {
        let lt = lanes_top + i32::try_from(li).unwrap_or(0) * lane_h;
        let cy_i = lt + lane_h / 2;
        #[allow(clippy::cast_precision_loss)]
        let cy = cy_i as f32;
        if *is_live {
            live_end = Some(lt + lane_h);
            let t = live_word.clone();
            layers.push(SceneLayer {
                id: format!("story-live:{li}"),
                mark: Mark::Text,
                data: DataSource::Constant {},
                fields: Vec::new(),
                constants: BTreeMap::new(),
                label: None,
                items: Vec::new(),
                prims: vec![text(
                    look,
                    "label",
                    right + 6,
                    cy_i + lp / 2 - 1,
                    t,
                    theme.valid,
                )],
                skipped: 0,
            });
        }
        for &i in members {
            let p = &vis[i];
            let r = p.row;
            let no_longer = p.live && r.validity == Validity::NoLonger;
            let color = if p.focus {
                theme.focus
            } else if p.live && !no_longer {
                theme.valid
            } else if p.live {
                theme.invalid
            } else {
                theme.closed
            };
            let thick = if p.focus {
                if p.live {
                    sl.bar_focus
                } else {
                    sl.bar_live
                }
            } else if p.live && !no_longer {
                sl.bar_live
            } else {
                sl.bar_closed
            };
            let mut prims = Vec::with_capacity(4);
            let (a, b) = p.ends;
            let (y0, y1) = (cy_i - thick / 2, cy_i - thick / 2 + thick);
            if r.coin_flip {
                // At most `dash_max` dashes: a long lane gets longer dashes, same gap.
                let len = b.max(a + 1) - a;
                let dash = sl.dash.0.max(len / sl.dash_max - sl.dash.1);
                let mut s = a;
                while s < b.max(a + 1) {
                    let e = (s + dash).min(b.max(a + 1));
                    prims.extend(rect(s, y0, e, y1, color));
                    s = e + sl.dash.1;
                }
            } else {
                prims.extend(rect(a, y0, b.max(a + 1), y1, color));
            }
            let m = if p.focus { sl.mark * 1.3 } else { sl.mark };
            #[allow(clippy::cast_precision_loss)]
            let (fa, fb) = (a as f32, b as f32);
            if r.born_bar >= x.from_bar {
                let points = match r.direction {
                    Direction::Up => {
                        vec![[fa, cy - m], [fa + m, cy + m * 0.8], [fa - m, cy + m * 0.8]]
                    }
                    Direction::Down => {
                        vec![[fa, cy + m], [fa + m, cy - m * 0.8], [fa - m, cy - m * 0.8]]
                    }
                    Direction::Neutral => {
                        vec![[fa, cy - m], [fa + m, cy], [fa, cy + m], [fa - m, cy]]
                    }
                };
                prims.push(Prim::Fill { points, color });
            } else {
                prims.push(fill(
                    &[(fa - 4.0, cy), (fa + 2.0, cy - 4.0), (fa + 2.0, cy + 4.0)],
                    color,
                ));
            }
            if p.end <= x.to_bar {
                if p.live {
                    cap(
                        &mut prims,
                        Shape::Live,
                        fb + 4.0,
                        cy,
                        m,
                        color,
                        theme.bg,
                        sl.dash,
                    );
                    if no_longer {
                        prims.push(Prim::Circle {
                            cx: fb + 3.0,
                            cy,
                            r: 1.5,
                            color: theme.bg,
                        });
                    }
                } else {
                    cap(
                        &mut prims,
                        outcome_shape(r),
                        fb,
                        cy,
                        m,
                        color,
                        theme.bg,
                        sl.dash,
                    );
                }
            } else {
                prims.push(fill(
                    &[(fb + 4.0, cy), (fb - 2.0, cy - 4.0), (fb - 2.0, cy + 4.0)],
                    color,
                ));
            }
            // Hit box: the lane over the span, at least `hit_min` wide.
            let (mut h0, mut h1) = p.span;
            if h1 - h0 < sl.hit_min {
                let c = (h0 + h1) / 2;
                h0 = c - sl.hit_min / 2;
                h1 = h0 + sl.hit_min;
            }
            let layer = SceneLayer {
                id: join(&[STORY_LAYER, &r.id]),
                mark: Mark::Glyph,
                data: DataSource::Finding {
                    finding: r.finding.clone(),
                },
                fields: Vec::new(),
                constants: BTreeMap::new(),
                label: Some(join(&[&r.face.event, " · ", &r.face.validity_phrase])),
                items: vec![Item {
                    bar: Some(r.born_bar),
                    t: Some(r.born_t),
                    #[allow(clippy::cast_precision_loss)]
                    values: BTreeMap::from([
                        ("end_bar".to_owned(), p.end as f64),
                        ("lane".to_owned(), li as f64),
                    ]),
                    hit: PixelRect {
                        left: h0.max(left - 4),
                        top: lt,
                        right: h1.min(w),
                        bottom: lt + lane_h,
                    },
                    bars: None,
                }],
                prims,
                skipped: 0,
            };
            if p.focus {
                under.extend(rect(
                    left,
                    lt,
                    right,
                    lt + lane_h,
                    with_alpha(theme.focus, sl.focus_row_alpha),
                ));
                if r.born_bar >= x.from_bar {
                    let price_top = scene.panels.first().map_or(0, |pp| pp.rect.top);
                    under.extend(rect(
                        a,
                        price_top,
                        a + 1,
                        cy_i,
                        with_alpha(theme.focus, sl.guide_alpha),
                    ));
                }
                focus_layer = Some(layer);
            } else {
                layers.push(layer);
            }
        }
    }
    // A hairline between the live lanes and the closed ones.
    if let Some(y) = live_end {
        if lanes.iter().any(|l| !l.0) {
            under.extend(rect(left, y, right, y + 1, theme.line));
        }
    }

    // Clusters: the closed stories without a lane, by time bucket.
    if !over.is_empty() {
        let ct = bottom - sl.cluster_h;
        let mut buckets: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
        for &i in &over {
            let b = (vis[i].span.0.max(left) - left) / sl.cluster_w;
            buckets.entry(b).or_default().push(i);
        }
        for (k, (b, members)) in buckets.iter().enumerate() {
            let x0 = left + b * sl.cluster_w + 2;
            let x1 = (x0 + sl.cluster_w - 4).min(right);
            let from = members
                .iter()
                .map(|&i| vis[i].row.born_bar)
                .min()
                .unwrap_or(x.from_bar)
                .max(x.from_bar);
            let to = members
                .iter()
                .map(|&i| vis[i].end)
                .max()
                .unwrap_or(x.to_bar)
                .min(x.to_bar);
            let t = format!("+{}", members.len());
            let mut prims = Vec::new();
            prims.extend(rect(x0, ct + 3, x1, ct + sl.cluster_h - 3, theme.line));
            prims.extend(rect(
                x0,
                ct + 3,
                x0 + 2,
                ct + sl.cluster_h - 3,
                theme.closed,
            ));
            prims.push(text(
                look,
                "label",
                x0 + 5,
                ct + sl.cluster_h / 2 + lp / 2 - 1,
                t,
                theme.text,
            ));
            let mut constants = BTreeMap::new();
            for &i in members {
                let r = vis[i].row;
                if let Some(fact) = (input.fact_of)(&r.finding) {
                    #[allow(clippy::cast_precision_loss)]
                    constants.insert(
                        join(&["/stories/", &r.id]),
                        ResolvedConstant {
                            origin: Origin::Fact {
                                fact,
                                field: "bar".to_owned(),
                            },
                            value: r.born_bar as f64,
                        },
                    );
                }
            }
            #[allow(clippy::cast_precision_loss)]
            layers.push(SceneLayer {
                id: format!("{CLUSTER_LAYER}{k}"),
                mark: Mark::Point,
                data: DataSource::Constant {},
                fields: Vec::new(),
                constants,
                label: Some(format!("+{}", members.len())),
                items: vec![Item {
                    bar: Some(from),
                    t: None,
                    values: BTreeMap::from([
                        ("from_bar".to_owned(), from as f64),
                        ("to_bar".to_owned(), to as f64),
                        ("stories".to_owned(), members.len() as f64),
                    ]),
                    hit: PixelRect {
                        left: x0 - 2,
                        top: ct,
                        right: x1 + 2,
                        bottom,
                    },
                    bars: None,
                }],
                prims,
                skipped: 0,
            });
        }
    }
    layers.extend(focus_layer);

    let id = scene.panels.iter().map(|p| p.id).max().map_or(0, |m| m + 1);
    #[allow(clippy::cast_precision_loss)]
    scene.panels.push(ScenePanel {
        id,
        name: Some(PANEL_NAME.to_owned()),
        // The whole width: the right margin holds the checkbox and the live words.
        rect: PixelRect {
            left,
            top: top0,
            right: w,
            bottom: full_h,
        },
        y: YMap {
            kind: ScaleKind::Linear,
            min: 0.0,
            max: f64::from(n_lanes.max(1)),
            top: f64::from(lanes_top),
            bottom: f64::from(lanes_top + n_lanes.max(1) * lane_h),
        },
        layers,
    });
    scene.under.extend(under);
    scene.height = h;
    scene
}
