//! (RD2/C2b) Qapsule's levels, read from the compiled `desk-levels` scene: each horizontal
//! level ends in a price-axis tag filled in its origin's colour and is stroked solid
//! (human), dashed (agent, the look's `agent_dash`) or dotted (engine); a value label under
//! a tag is left out (P6 counts the levels scenes too). A legacy look draws neither.

use qp_look::look::{by_name, Look, Profile, LOOKS};
use qp_schema::scene::{Prim, Rgba, SceneDesc};

use crate::scene::{fixture, overlap_count};
use crate::thresholds as th;

fn layer(s: &SceneDesc, id: &str) -> Vec<(i32, u32, Rgba)> {
    let l = s.panels.iter().flat_map(|p| &p.layers).find(|l| l.id == id);
    l.unwrap_or_else(|| panic!("layer {id}"))
        .prims
        .iter()
        .filter_map(|p| match p {
            Prim::Rect { x, w, color, .. } => Some((*x, *w, *color)),
            _ => None,
        })
        .collect()
}

/// The fill of the gutter tag whose text is `label` (the rect drawn just before it).
fn tag_fill(s: &SceneDesc, label: &str) -> Option<Rgba> {
    let right = s.panels.iter().map(|p| p.rect.right).min().unwrap_or(0);
    let i = s
        .over
        .iter()
        .position(|p| matches!(p, Prim::Text { x, text, .. } if text == label && *x > right))?;
    match s.over.get(i.checked_sub(1)?) {
        Some(Prim::Rect { color, .. }) => Some(*color),
        _ => None,
    }
}

fn rgb(c: Rgba) -> [u8; 3] {
    [c[0], c[1], c[2]]
}

/// Asserts one level: every run in `ink`, each `run` px long, `step` px apart (`None`:
/// one solid rule).
fn stroke(s: &SceneDesc, id: &str, ink: Rgba, dash: Option<(u32, i32)>, what: &str) {
    let r = layer(s, id);
    assert!(
        r.iter().all(|x| rgb(x.2) == rgb(ink)),
        "{what}: {id} colour {r:?}"
    );
    match dash {
        None => assert_eq!(r.len(), 1, "{what}: {id} is one solid rule"),
        Some((run, step)) => {
            assert!(r.len() > 10, "{what}: {id} is broken into runs");
            assert_eq!(
                (r[0].1, r[1].0 - r[0].0),
                (run, step),
                "{what}: {id} pattern"
            );
        }
    }
}

fn check(look: &'static Look) {
    let fx = fixture();
    for t in [&look.dark, &look.light] {
        let s = fx.scene_of(&fx.levels, look, t.name, th::DESK.0, th::DESK.1);
        let what = format!("{} {}", look.name, t.name);
        let o = look.origin_roles(t);
        let [a, b] = look.agent_dash().expect("agent dash");
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let agent = (a.round() as u32, (a + b).round() as i32);
        stroke(&s, "watch", o.human, None, &what);
        stroke(&s, "proposal", o.agent, Some(agent), &what);
        stroke(&s, "rsi-os", o.engine, Some((2, 4)), &what);
        for (label, ink) in [
            ("118,00", o.human),
            ("112,50", o.agent),
            ("30,00", o.engine),
        ] {
            let fill = tag_fill(&s, label).unwrap_or_else(|| panic!("{what}: tag {label}"));
            assert_eq!(rgb(fill), rgb(ink), "{what}: tag {label}");
        }
        assert_eq!(overlap_count(&s), 0, "{what}: P6 with levels");
    }
}

/// Every non-legacy look with `origin_levels` draws its levels in the chart's grammar.
#[test]
fn levels_end_in_origin_tags_with_origin_strokes() {
    let looks = LOOKS
        .iter()
        .filter(|l| l.profile != Profile::Legacy && l.chart.origin_levels);
    let mut n = 0;
    for l in looks {
        check(l);
        n += 1;
    }
    assert!(n >= 2, "tavek and tavek-tel carry origin levels");
}

/// A legacy look keeps R0's rule: one solid run in the layer's colour, no level tag.
#[test]
fn legacy_levels_are_unchanged() {
    let fx = fixture();
    let r4 = by_name(Some("r4")).expect("r4");
    let s = fx.scene_of(&fx.levels, r4, "dark", th::DESK.0, th::DESK.1);
    assert_eq!(layer(&s, "proposal").len(), 1);
    assert_eq!(rgb(layer(&s, "watch")[0].2), rgb(r4.dark.muted));
    assert!(tag_fill(&s, "118.00").is_none() && tag_fill(&s, "118,00").is_none());
}
