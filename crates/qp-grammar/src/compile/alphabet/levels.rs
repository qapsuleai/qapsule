//! (RD2/C2b) Levels in the chart's own grammar (`chart.origin_levels`; tavek DIRECTION): a
//! horizontal rule takes its origin's colour and stroke (human solid, agent dashed, engine
//! dotted) and ends in a price-axis tag in that colour; a value label under it, or a tag
//! meeting the last-value tag or an earlier level tag, is left out.

use qp_schema::grammar::Origin;
use qp_schema::Author;

use super::format::NumFmt;
use super::marks::DOT;
use super::*;

/// A level on a price axis: panel index, pixel row, value, origin (0 human, 1 agent,
/// 2 engine).
pub(in crate::compile) type Level = (usize, i32, f64, usize);

fn on(look: &Look) -> bool {
    look.familiar() && look.chart.origin_levels
}

/// The author of the fact the level comes from (an accepted proposal: its approver); a
/// threshold or a series value is the engine's.
fn who(pl: &Prepared, p: &dyn DataProvider) -> usize {
    let prov = match (&pl.data, pl.constants.get("/encoding/y").map(|c| &c.origin)) {
        (_, Some(Origin::Fact { fact, .. })) | (DataSource::Fact { fact }, _) => {
            p.fact(fact).map(|f| f.provenance.clone())
        }
        (DataSource::Finding { finding }, _) => p.finding(finding).map(|f| f.provenance.clone()),
        _ => None,
    };
    match prov.map(|v| v.approved_by.unwrap_or(v.author)) {
        Some(Author::Human { .. }) => 0,
        Some(Author::Agent { .. }) => 1,
        _ => 2,
    }
}

impl Chart<'_> {
    /// This chart with the horizontal levels of `prepared` (panel ids `ids`).
    pub(super) fn with_levels(mut self, prepared: &[Prepared], ids: &[u32]) -> Self {
        for pl in prepared
            .iter()
            .filter(|pl| pl.geom == Geom::RuleH && on(self.look))
        {
            let Some(pi) = ids.iter().position(|i| *i == pl.panel) else {
                continue;
            };
            let (rc, who, rows) = (self.rects[pi], who(pl, self.fr.p), pl.traces.iter());
            for v in rows.flat_map(|t| &t.rows).filter_map(|r| r.v[0]) {
                match value_y(&self.ymaps[pi], v).map(px) {
                    Some(y) if y > rc.top && y < rc.bottom => self.levels.push((pi, y, v, who)),
                    _ => {}
                }
            }
        }
        self
    }

    fn ink(&self, who: usize) -> Rgba {
        let o = self.look.origin_roles(self.theme());
        [o.human, o.agent, o.engine][who]
    }
}

/// A level's stroke: its origin's colour (the layer's alpha kept) and dash.
pub(super) fn stroke(ch: &Chart<'_>, pl: &Prepared, prims: &mut Vec<Prim>) {
    if !on(ch.look) || pl.geom != Geom::RuleH {
        return;
    }
    let who = who(pl, ch.fr.p);
    let agent = ch.look.agent_dash().map(|[a, b]| (px(a), px(a + b)));
    let (ink, dash) = (
        ch.ink(who),
        [None, agent, Some((DOT.0, DOT.0 + DOT.1))][who],
    );
    for p in std::mem::take(prims) {
        let Prim::Rect { x, y, w, h, color } = p else {
            prims.push(p);
            continue;
        };
        let (w, h) = (i32::try_from(w).unwrap_or(0), i32::try_from(h).unwrap_or(0));
        let (run, step) = dash.unwrap_or((w, w));
        let c = [ink[0], ink[1], ink[2], color[3]];
        for xx in (x..x + w).step_by(usize::try_from(step).unwrap_or(1).max(1)) {
            prims.push(rect(xx, y, run.min(x + w - xx), h, c));
        }
    }
}

/// The level tags, after the last-value tags (which win).
pub(super) fn level_tags(ch: &Chart<'_>, over: &mut Vec<Prim>) {
    let (h, lv) = (ch.pill_h(), ch.look.chart.last_value_tag);
    let mut placed: Vec<(usize, i32)> = (ch.tags.iter().enumerate())
        .filter_map(|(i, t)| t.filter(|_| lv).map(|t| (i, t.0)))
        .collect();
    for &(pi, y, v, who) in &ch.levels {
        if placed.iter().all(|&(p, yy)| p != pi || (y - yy).abs() >= h) {
            placed.push((pi, y));
            let y0 = &ch.ymaps[pi];
            let label = NumFmt::new(y0.min.abs().max(y0.max.abs()), None, ch.lang()).fmt(v);
            ch.pill(over, ch.rects[pi].right + 1, y, label, ch.ink(who));
        }
    }
}
