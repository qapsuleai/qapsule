//! (RF1/Ö, RD2 audit K1) P9, the desktop layout, measured without a browser: the regions
//! of the committed Ç1 mockup (`docs/design/mockups/<look>/c1.html`, else `_base/c1.html`,
//! the file `scripts/design-gallery` screenshots), and the chart panes and legend numbers
//! of the core's desk scene, counted as the gallery's `layoutDump` counts them.
//!
//! Regions are the mockup's `data-region` elements; the mockups are static HTML (their
//! script, `_base/frame.js`, adds none), so the source count is the DOM count the gallery
//! reads. The chart is compiled at the desktop chart cell ([`th::DESK`]) instead of the
//! slot size the browser lays out; the pane count does not depend on size, and the legend
//! sits above the plot at every desktop width. `--layout <file>` (a gallery dump) still
//! overrides the measurement. A touch look has no desktop layout: `n.a.`

use qp_look::look::{Look, Profile};
use qp_schema::scene::Prim;

use crate::scene::Fixture;
use crate::thresholds as th;
use crate::Row;

const RULE: &str = "layout regions / panes / legend numbers";

/// The Ç1 mockup a look's desktop layout is measured on (the gallery's choice).
fn mockup(look: &Look) -> String {
    let root = format!("{}/../../docs/design/mockups", env!("CARGO_MANIFEST_DIR"));
    let own = format!("{root}/{}/c1.html", look.name);
    let file = if std::path::Path::new(&own).exists() {
        own
    } else {
        format!("{root}/_base/c1.html")
    };
    std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"))
}

/// A numeric token as the gallery counts it: `^[-+−]?\d[\d.,]*%?$`.
fn numeric(t: &str) -> bool {
    let t = t.strip_prefix(['-', '+', '−']).unwrap_or(t);
    let t = t.strip_suffix('%').unwrap_or(t);
    t.chars().next().is_some_and(|c| c.is_ascii_digit())
        && t.chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
}

/// (regions, chart panes, legend numbers) for `look`, measured.
pub fn measure(look: &'static Look, fx: &Fixture) -> (u64, u64, u64) {
    let regions = mockup(look).matches("data-region=").count();
    let (w, h) = th::DESK;
    let s = fx.layout_scene(look, w, h);
    let right = s.panels.iter().map(|p| p.rect.right).max().unwrap_or(0);
    let bottom = s.panels.iter().map(|p| p.rect.bottom).max().unwrap_or(0);
    let numbers = s
        .under
        .iter()
        .chain(&s.over)
        .filter_map(|p| match p {
            Prim::Text {
                x, baseline, text, ..
            } if *x < right && *baseline <= bottom => Some(text),
            _ => None,
        })
        .flat_map(|t| t.split_whitespace())
        .filter(|t| numeric(t))
        .count();
    let n = |v: usize| u64::try_from(v).unwrap_or(u64::MAX);
    (n(regions), n(s.panels.len()), n(numbers))
}

/// P9 for `look`: from `dump` when given (`--layout`), measured otherwise.
pub fn p9(look: &'static Look, fx: &Fixture, dump: Option<&serde_json::Value>) -> Row {
    if look.profile == Profile::Touch {
        return Row::na("P9", "-", &format!("{RULE} (touch: no desktop layout)"));
    }
    let (r, p, g, from) = match dump {
        Some(l) => {
            let n = |k: &str| l.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
            (
                n("regions"),
                n("chart_panes"),
                n("legend_numbers"),
                "--layout",
            )
        }
        None => {
            let (r, p, g) = measure(look, fx);
            (r, p, g, "Ç1 mockup + desk")
        }
    };
    Row::check(
        "P9",
        "-",
        &format!("{RULE} ({from})"),
        format!("{r} / {p} / {g}"),
        format!(
            ">= {} / {} / {}",
            th::P9_REGIONS,
            th::P9_PANES,
            th::P9_LEGEND_NUMBERS
        ),
        r >= th::P9_REGIONS && p >= th::P9_PANES && g >= th::P9_LEGEND_NUMBERS,
    )
}

#[test]
fn numeric_tokens_match_the_gallery() {
    for t in ["123", "1.234,5", "-2", "+0,5%", "−3", "12%"] {
        assert!(numeric(t), "{t}");
    }
    for t in ["O", "SMA20", "%5", "", "-", "1a", "1.2K"] {
        assert!(!numeric(t), "{t}");
    }
}
