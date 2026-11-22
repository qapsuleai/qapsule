//! `korda`'s drawing steps ([`Draw`]): lanes, the histogram and the header.

use super::*;

impl Draw<'_> {
    fn text(&self, x: f64, baseline: i32, s: String, color: Rgba) -> Prim {
        #[allow(clippy::cast_possible_truncation)]
        Prim::Text {
            x: x.floor() as i32,
            baseline,
            px: f(self.look.types.label.px),
            text: s,
            color,
            align: Align::Left,
            font: self.look.scene_font("label"),
            role: self.roles.then(|| "label".to_owned()),
        }
    }

    /// A lane's label: what happened and how it stands, then its time when the rows carry
    /// one (RP4/T7; last, so a label cut to its room keeps what happened — the lane's
    /// place on the time axis says when too), then the coin-flip mark.
    fn label(&self, r: &StoryRow) -> String {
        let mut s = format!("{} · {}", r.face.event, r.face.validity_phrase);
        if let Some(t) = self.stamp_of.and_then(|f| f(r)) {
            s.push_str(" · ");
            s.push_str(&t);
        }
        if r.coin_flip {
            s.push_str(" · ");
            s.push_str(words(self.lang).coin);
        }
        s
    }

    fn label_px(&self) -> f64 {
        self.look.types.label.px
    }

    fn adv(&self) -> f64 {
        self.look.role_font("label").advance
    }

    fn dir_color(&self, d: Direction) -> Rgba {
        match d {
            Direction::Up => self.theme.up,
            Direction::Down => self.theme.down,
            Direction::Neutral => self.theme.neutral,
        }
    }

    fn inside(&self, bar: u64) -> bool {
        bar >= self.a && bar <= self.b
    }

    /// The birth mark: ▲ up, ▼ down, ◆ none.
    fn birth(&self, out: &mut Vec<Prim>, cx: f64, cy: f64, r: f64, d: Direction, c: Rgba) {
        let pts = match d {
            Direction::Up => vec![[cx, cy - r], [cx + r, cy + r * 0.8], [cx - r, cy + r * 0.8]],
            Direction::Down => vec![[cx, cy + r], [cx + r, cy - r * 0.8], [cx - r, cy - r * 0.8]],
            Direction::Neutral => vec![[cx, cy - r], [cx + r, cy], [cx, cy + r], [cx - r, cy]],
        };
        out.push(fill(&pts, c));
    }

    /// The end mark: ● realized, × invalidated, ○ expired, □ retracted, ▸ open.
    fn end(&self, out: &mut Vec<Prim>, cx: f64, cy: f64, r: f64, o: Option<StoryOutcome>, c: Rgba) {
        match o {
            Some(StoryOutcome::Realized) => out.push(Prim::Circle {
                cx: f(cx),
                cy: f(cy),
                r: f(r),
                color: c,
            }),
            Some(StoryOutcome::Invalidated) => {
                let w = (r * 0.35).max(1.2);
                out.push(poly(&[[cx - r, cy - r], [cx + r, cy + r]], w, c));
                out.push(poly(&[[cx - r, cy + r], [cx + r, cy - r]], w, c));
            }
            Some(StoryOutcome::Expired) => {
                out.push(Prim::Circle {
                    cx: f(cx),
                    cy: f(cy),
                    r: f(r),
                    color: c,
                });
                out.push(Prim::Circle {
                    cx: f(cx),
                    cy: f(cy),
                    r: f((r - (r * 0.4).max(1.2)).max(0.5)),
                    color: self.theme.bg,
                });
            }
            Some(StoryOutcome::Retracted) => {
                let w = (r * 0.35).max(1.2);
                let pts = [
                    [cx - r, cy - r],
                    [cx + r, cy - r],
                    [cx + r, cy + r],
                    [cx - r, cy + r],
                    [cx - r, cy - r],
                ];
                out.push(poly(&pts, w, c));
            }
            None => out.push(fill(
                &[
                    [cx - r * 0.6, cy - r],
                    [cx + r * 0.9, cy],
                    [cx - r * 0.6, cy + r],
                ],
                c,
            )),
        }
    }

    /// One story's lane in a layer `story:<capsule>` (data: its finding; one item, its
    /// lane box).
    #[allow(clippy::too_many_lines)]
    pub(super) fn lane(
        &self,
        l: &Life<'_>,
        top: i32,
        pitch: i32,
        role: Role,
        mode: Mode,
        next: Option<f64>,
    ) -> SceneLayer {
        let r = l.row;
        let full = mode == Mode::Full;
        let rad = if full || role == Role::Selected {
            MARK_R[0]
        } else {
            MARK_R[1]
        };
        // The selected story's label sits above its line; a full lane's beside it.
        let above = role == Role::Selected;
        let cy = if above {
            f64::from(top) + f64::from(pitch) - rad - 1.5
        } else {
            f64::from(top) + f64::from(pitch) / 2.0
        };
        let (left, right) = (f64::from(self.rc.left), f64::from(self.rc.right));
        let xb = slot_x(&self.x, l.start);
        let xe = slot_x(&self.x, l.end.min(self.b.max(l.start)));
        let x0 = xb.max(left);
        let x1 = if l.end > self.b { right } else { xe.min(right) };
        let (lw, line_c) = match role {
            Role::Selected => (LANE_W[0], self.theme.focus),
            Role::Live => (
                LANE_W[1],
                if r.validity == Validity::NoLonger {
                    self.theme.invalid
                } else {
                    self.theme.valid
                },
            ),
            Role::Closed => (LANE_W[2], self.theme.closed),
        };
        let mark_c = match role {
            Role::Selected => self.theme.focus,
            Role::Live => self.dir_color(r.direction),
            Role::Closed => self.theme.closed,
        };
        let mut prims = Vec::new();
        // The selected story's guide: from its birth up through the price.
        if role == Role::Selected && self.inside(l.start) {
            prims.push(poly(
                &[[xb + 0.5, f64::from(self.price_top)], [xb + 0.5, cy]],
                1.0,
                with_opacity(self.theme.focus, GUIDE_OPACITY),
            ));
        }
        if x1 > x0 {
            if r.coin_flip {
                // Dotted (K53).
                let mut xx = x0;
                let d = (lw + 1.0).max(2.0);
                while xx < x1 {
                    prims.push(rect(xx, cy - d / 2.0, d, d, line_c));
                    xx += d * 2.5;
                }
            } else if l.open && r.validity == Validity::NoLonger {
                // Double rail: it no longer holds.
                prims.push(poly(&[[x0, cy - 1.5], [x1, cy - 1.5]], 1.0, line_c));
                prims.push(poly(&[[x0, cy + 1.5], [x1, cy + 1.5]], 1.0, line_c));
            } else {
                prims.push(poly(&[[x0, cy], [x1, cy]], lw, line_c));
            }
        }
        if self.inside(l.start) {
            self.birth(&mut prims, xb, cy, rad, r.direction, mark_c);
        }
        if l.end <= self.b {
            let ec = if l.open { mark_c } else { line_c };
            self.end(&mut prims, xe, cy, rad, r.outcome, ec);
        }
        // The label: what happened and how it stands. The selected story's above its
        // line; a full lane's after its end mark or, when there is more room on its life,
        // over the line (on a background patch).
        if above || full {
            let s = self.label(r);
            let c = if role == Role::Closed {
                self.theme.closed
            } else {
                self.theme.text
            };
            let on_x = if self.inside(l.start) {
                xb + rad + 3.0
            } else {
                left + 2.0
            };
            let stop = next.unwrap_or(right).min(right) - 4.0;
            let lp = self.label_px();
            #[allow(clippy::cast_possible_truncation)]
            if above {
                if let Some(t) = fit(&s, stop - on_x, lp, self.adv()) {
                    let base = top + (lp.ceil() as i32) + 1;
                    prims.push(self.text(on_x, base, t, c));
                }
            } else {
                let on_room = if l.end <= self.b {
                    xe - rad - 3.0
                } else {
                    right - 2.0
                } - on_x;
                let after_x = xe + rad + 3.0;
                let after_room = if l.end <= self.b { stop - after_x } else { 0.0 };
                let base = (cy + lp * 0.36).round() as i32;
                if after_room >= on_room {
                    if let Some(t) = fit(&s, after_room, lp, self.adv()) {
                        prims.push(self.text(after_x, base, t, c));
                    }
                } else if let Some(t) = fit(&s, on_room, lp, self.adv()) {
                    #[allow(clippy::cast_precision_loss)]
                    let tw = (t.chars().count() as f64 * lp * self.adv()).ceil();
                    prims.push(rect(on_x - 1.0, cy - lp * 0.5, tw + 3.0, lp, self.theme.bg));
                    prims.push(self.text(on_x, base, t, c));
                }
            }
        }
        #[allow(clippy::cast_possible_truncation)]
        let hit = PixelRect {
            left: ((x0 - rad - 2.0).floor() as i32).max(self.rc.left),
            top,
            right: ((x1 + rad + 3.0).ceil() as i32).min(self.rc.right),
            bottom: (top + pitch).min(self.rc.bottom),
        };
        let lane_ix = f64::from(top - self.rc.top - HEADER_PX) / f64::from(pitch.max(1));
        #[allow(clippy::cast_precision_loss)]
        let values = BTreeMap::from([
            ("born_bar".to_owned(), l.start as f64),
            ("end_bar".to_owned(), l.end as f64),
            ("lane".to_owned(), lane_ix.floor()),
            ("open".to_owned(), if l.open { 1.0 } else { 0.0 }),
            ("tier".to_owned(), f64::from(r.tier)),
        ]);
        let label = self.label(r);
        SceneLayer {
            id: format!("{STORY_LAYER}{}", r.id),
            mark: Mark::Rule,
            data: DataSource::Finding {
                finding: r.finding.clone(),
            },
            fields: vec!["born_bar".to_owned(), "end_bar".to_owned()],
            constants: BTreeMap::new(),
            label: Some(label),
            items: vec![Item {
                bar: Some(l.start),
                t: Some(r.born_t),
                values,
                hit,
                bars: None,
            }],
            prims,
            skipped: 0,
        }
    }

    /// The summary: births per bucket of bars, stacked bottom-up by outcome (realized:
    /// solid; invalidated: outline; expired or retracted: hatched; open: solid in the
    /// "holds" colour). One layer per bucket (`story-cluster:<from>-<to>`), data: its
    /// newest story (the selected one if it is there).
    #[allow(clippy::too_many_lines)]
    pub(super) fn histogram(
        &self,
        lives: &[Life<'_>],
        top: i32,
        bottom: i32,
        selected: &str,
    ) -> Vec<SceneLayer> {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let k = ((BUCKET_PX / self.x.slot).ceil().max(1.0)) as u64;
        // bucket index → lives
        let mut buckets: BTreeMap<u64, Vec<&Life<'_>>> = BTreeMap::new();
        for l in lives {
            buckets.entry(l.start.max(self.a) / k).or_default().push(l);
        }
        let max = buckets.values().map(Vec::len).max().unwrap_or(1).max(1);
        let h = f64::from((bottom - top).max(4));
        let mut out = Vec::with_capacity(buckets.len());
        let (left, right) = (f64::from(self.rc.left), f64::from(self.rc.right));
        for (j, ls) in &buckets {
            let (from, to) = (j * k, j * k + k - 1);
            let (va, vb) = (from.max(self.a), to.min(self.b));
            #[allow(clippy::cast_precision_loss)]
            let xa = (self.x.left + (va - self.x.from_bar) as f64 * self.x.slot).max(left);
            #[allow(clippy::cast_precision_loss)]
            let xb = (self.x.left + (vb - self.x.from_bar + 1) as f64 * self.x.slot).min(right);
            let (bx, bw) = (xa.floor() + 1.0, (xb.floor() - xa.floor() - 2.0).max(1.0));
            let count = |pred: &dyn Fn(&Life<'_>) -> bool| ls.iter().filter(|l| pred(l)).count();
            let real = count(&|l| l.row.outcome == Some(StoryOutcome::Realized));
            let inv = count(&|l| l.row.outcome == Some(StoryOutcome::Invalidated));
            let exp = count(&|l| {
                matches!(
                    l.row.outcome,
                    Some(StoryOutcome::Expired | StoryOutcome::Retracted)
                )
            });
            let open = count(&|l| l.open);
            let n = ls.len();
            #[allow(clippy::cast_precision_loss)]
            let bh = (h * n as f64 / max as f64).round().max(2.0);
            let base = f64::from(bottom);
            let mut prims = Vec::new();
            let mut cum = 0usize;
            let seg = |c: usize, cum: usize| {
                #[allow(clippy::cast_precision_loss)]
                let y1 = base - (bh * cum as f64 / n as f64).round();
                #[allow(clippy::cast_precision_loss)]
                let y0 = base - (bh * (cum + c) as f64 / n as f64).round();
                (y0, (y1 - y0).max(1.0))
            };
            let closed = self.theme.closed;
            if real > 0 {
                let (y0, sh) = seg(real, cum);
                prims.push(rect(bx, y0, bw, sh, closed));
                cum += real;
            }
            if inv > 0 {
                let (y0, sh) = seg(inv, cum);
                prims.push(rect(bx, y0, bw, 1.0, closed));
                prims.push(rect(bx, y0 + sh - 1.0, bw, 1.0, closed));
                prims.push(rect(bx, y0, 1.0, sh, closed));
                prims.push(rect(bx + bw - 1.0, y0, 1.0, sh, closed));
                cum += inv;
            }
            if exp > 0 {
                let (y0, sh) = seg(exp, cum);
                let mut yy = y0;
                while yy < y0 + sh {
                    prims.push(rect(bx, yy, bw, 1.0, closed));
                    yy += 2.0;
                }
                cum += exp;
            }
            if open > 0 {
                let (y0, sh) = seg(open, cum);
                prims.push(rect(bx, y0, bw, sh, self.theme.valid));
            }
            let rep = ls
                .iter()
                .find(|l| l.row.finding == selected)
                .or_else(|| {
                    ls.iter()
                        .max_by(|x, y| (x.start, &x.row.id).cmp(&(y.start, &y.row.id)))
                })
                .map_or_else(String::new, |l| l.row.finding.clone());
            #[allow(clippy::cast_precision_loss)]
            let values = BTreeMap::from([
                ("count".to_owned(), n as f64),
                ("realized".to_owned(), real as f64),
                ("invalidated".to_owned(), inv as f64),
                ("expired".to_owned(), exp as f64),
                ("open".to_owned(), open as f64),
                ("from_bar".to_owned(), from as f64),
                ("to_bar".to_owned(), to as f64),
            ]);
            #[allow(clippy::cast_possible_truncation)]
            let hit = PixelRect {
                left: xa.floor() as i32,
                top,
                right: (xb.floor() as i32).max(xa.floor() as i32 + 1),
                bottom: bottom + 2,
            };
            out.push(SceneLayer {
                id: format!("{CLUSTER_LAYER}{from}-{to}"),
                mark: Mark::Bar,
                data: DataSource::Finding { finding: rep },
                fields: vec!["count".to_owned()],
                constants: BTreeMap::new(),
                label: Some(format!("{n} {}", words(self.lang).stories)),
                items: vec![Item {
                    bar: Some(va),
                    t: None,
                    values,
                    hit,
                    bars: Some(vb - va + 1),
                }],
                prims,
                skipped: 0,
            });
        }
        out
    }

    /// The header row: open and closed counts, and the control that hides or shows the
    /// closed stories (the whole row is its target).
    pub(super) fn header(
        &self,
        live: usize,
        closed: usize,
        hidden: usize,
        shown: bool,
    ) -> SceneLayer {
        let w = words(self.lang);
        let mut s = format!("{} {live} · {} {closed}", w.live, w.closed);
        if hidden > 0 {
            s.push(' ');
            s.push_str(w.hidden);
        }
        let base = self.rc.top + 12;
        let mut prims = vec![self.text(f64::from(self.rc.left) + 2.0, base, s, self.theme.muted)];
        let mut items = Vec::new();
        if closed > 0 {
            let t = if shown { w.hide } else { w.show };
            #[allow(clippy::cast_precision_loss)]
            let tw = (t.chars().count() as f64 * self.label_px() * self.adv()).ceil();
            let x1 = f64::from(self.rc.right) - 3.0;
            let x0 = x1 - tw - 10.0;
            let (y0, ch) = (f64::from(self.rc.top) + 1.0, f64::from(HEADER_PX) - 2.0);
            let c = self.theme.muted;
            prims.push(rect(x0, y0, x1 - x0, 1.0, c));
            prims.push(rect(x0, y0 + ch - 1.0, x1 - x0, 1.0, c));
            prims.push(rect(x0, y0, 1.0, ch, c));
            prims.push(rect(x1 - 1.0, y0, 1.0, ch, c));
            prims.push(self.text(x0 + 5.0, base, t.to_owned(), self.theme.text));
            #[allow(clippy::cast_precision_loss)]
            items.push(Item {
                bar: None,
                t: None,
                values: BTreeMap::from([
                    ("closed".to_owned(), closed as f64),
                    ("shown".to_owned(), if shown { 1.0 } else { 0.0 }),
                ]),
                hit: PixelRect {
                    left: self.rc.left,
                    top: self.rc.top,
                    right: self.rc.right,
                    bottom: self.rc.top + HEADER_PX,
                },
                bars: None,
            });
        }
        SceneLayer {
            id: TOGGLE_LAYER.to_owned(),
            mark: Mark::Text,
            data: DataSource::Constant {},
            fields: vec!["closed".to_owned(), "shown".to_owned()],
            constants: BTreeMap::new(),
            label: None,
            items,
            prims,
            skipped: 0,
        }
    }
}
