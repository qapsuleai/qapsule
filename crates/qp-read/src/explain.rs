//! A plain-text semantic summary of a compiled view (`qp explain`): panels, scales and
//! visible domains, layers with their data, constants and origins, last values, and
//! emphasis. Read from the scene description only (`docs/GRAMMAR.md` §16); deterministic
//! (numbers with `core::fmt`, dates with integer arithmetic, UTC).

use std::fmt::Write as _;

use qp_schema::grammar::{EmphasisKind, Mark, ScaleKind};
use qp_schema::scene::{DataSource, SceneDesc, SceneLayer};
use qp_schema::View;

use crate::rd::Rd;
use qp_grammar::CompileOptions;
use qp_ops::canon::origin_text;
use qp_ops::expand::need_view;
use qp_plane::error::PResult;

/// `YYYY-MM-DD` (UTC) of an epoch-millisecond time (civil-from-days, integer only).
fn date(t: i64) -> String {
    let days = t.div_euclid(86_400_000);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn mark_name(m: Mark) -> &'static str {
    match m {
        Mark::Candle => "candle",
        Mark::Line => "line",
        Mark::Area => "area",
        Mark::Bar => "bar",
        Mark::Rule => "rule",
        Mark::Point => "point",
        Mark::Text => "text",
        Mark::Band => "band",
        Mark::Glyph => "glyph",
    }
}

fn scale_name(k: ScaleKind) -> &'static str {
    match k {
        ScaleKind::Time => "time",
        ScaleKind::Linear => "linear",
        ScaleKind::Log => "log",
    }
}

fn emphasis_name(k: EmphasisKind) -> String {
    serde_json::to_value(k)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn data_text(d: &DataSource) -> String {
    match d {
        DataSource::Series { series } => series.clone(),
        DataSource::Fact { fact } => format!("fact {fact}"),
        DataSource::Finding { finding } => format!("finding {finding}"),
        DataSource::Constant {} => "constant".to_owned(),
    }
}

fn layer_line(out: &mut String, l: &SceneLayer) {
    let _ = write!(
        out,
        "  {} {} {}",
        l.id,
        mark_name(l.mark),
        data_text(&l.data)
    );
    if let Some(label) = &l.label {
        let _ = write!(out, " \"{label}\"");
    }
    if !l.fields.is_empty() {
        let _ = write!(out, " fields {}", l.fields.join(" "));
    }
    let _ = write!(out, ": {} item(s)", l.items.len());
    if l.skipped > 0 {
        let _ = write!(out, ", {} value(s) not drawn", l.skipped);
    }
    if let Some(last) = l.items.last() {
        let vals: Vec<String> = last
            .values
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        if !vals.is_empty() {
            let _ = write!(out, "; last {}", vals.join(" "));
        }
        match (last.bar, last.t) {
            (Some(b), Some(t)) => {
                let _ = write!(out, " (bar {b}, {})", date(t));
            }
            (Some(b), None) => {
                let _ = write!(out, " (bar {b})");
            }
            _ => {}
        }
    }
    out.push('\n');
    for (ptr, c) in &l.constants {
        let _ = writeln!(
            out,
            "    {ptr} = {} from {}",
            c.value,
            origin_text(&c.origin)
        );
    }
}

/// The summary text of `scene`, compiled from `view`.
#[must_use]
pub fn explain_scene(view: &View, scene: &SceneDesc) -> String {
    let mut out = String::new();
    let _ = write!(out, "view {}", scene.view);
    if let Some(t) = &view.title {
        let _ = write!(out, " \"{t}\"");
    }
    let _ = writeln!(out, " ({}x{})", scene.width, scene.height);
    let x = &scene.x;
    let _ = write!(
        out,
        "x: slots {}..{} of {} bar(s)",
        x.from_bar, x.to_bar, scene.bars
    );
    let times: Vec<i64> = scene
        .panels
        .iter()
        .flat_map(|p| &p.layers)
        .flat_map(|l| &l.items)
        .filter_map(|i| i.t)
        .collect();
    if let (Some(a), Some(b)) = (times.iter().min(), times.iter().max()) {
        let _ = write!(out, ", {} .. {}", date(*a), date(*b));
    }
    let _ = writeln!(out, ", {} px per slot", x.slot);
    if let Some(lod) = &scene.lod {
        let _ = writeln!(
            out,
            "level of detail: {} bars per bucket, {} buckets",
            lod.bars_per_bucket, lod.buckets
        );
    }
    for p in &scene.panels {
        let scale_id = view
            .panel(p.id)
            .and_then(|vp| vp.y.id.clone())
            .unwrap_or_else(|| format!("y{}", p.id));
        let _ = write!(out, "panel {}", p.id);
        if let Some(n) = &p.name {
            let _ = write!(out, " \"{n}\"");
        }
        let _ = writeln!(
            out,
            ": {scale_id} {} [{}, {}], {} layer(s)",
            scale_name(p.y.kind),
            p.y.min,
            p.y.max,
            p.layers.len()
        );
        for l in &p.layers {
            layer_line(&mut out, l);
        }
    }
    for e in &scene.emphasis {
        let _ = write!(out, "emphasis: {}", emphasis_name(e.kind));
        if let Some(l) = &e.layer {
            let _ = write!(out, " layer {l}");
        }
        match (e.from_bar, e.to_bar) {
            (Some(a), Some(b)) => {
                let _ = write!(out, " bars {a}..{b}");
            }
            (Some(a), None) | (None, Some(a)) => {
                let _ = write!(out, " bar {a}");
            }
            (None, None) => {}
        }
        if e.panels.is_empty() {
            out.push_str(" (all panels)\n");
        } else {
            let ps: Vec<String> = e.panels.iter().map(u32::to_string).collect();
            let _ = writeln!(out, " (panel {})", ps.join(", "));
        }
    }
    out
}

impl Rd<'_> {
    /// [`explain_scene`] of view `view_id` compiled with `opts`.
    ///
    /// # Errors
    /// As [`Session::scene`].
    pub fn explain(&mut self, view_id: &str, opts: &CompileOptions) -> PResult<String> {
        let view = need_view(&self.doc, view_id, "/view")?.clone();
        let scene = self.scene(view_id, opts)?;
        Ok(explain_scene(&view, &scene))
    }
}

#[cfg(test)]
mod tests {
    use super::date;

    #[test]
    fn dates() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(1_759_881_600_000), "2025-10-08");
        assert_eq!(date(1_735_776_000_000), "2025-01-02");
        assert_eq!(date(951_782_400_000), "2000-02-29");
        assert_eq!(date(-86_400_000), "1969-12-31");
    }
}
