//! R4: opening between depths (`docs/GRAMMAR.md` §18.6, `docs/PROTOCOL.md` §4.6): the
//! depth steps, what a tap opens and the shared elements of a transition.

use super::*;

// ---------------------------------------------------------------------------
// R4: opening between depths (`docs/GRAMMAR.md` §18.6, `docs/PROTOCOL.md` §4.6)
// ---------------------------------------------------------------------------

/// One depth further: sentence → capsule → chart → evidence; `None` at evidence.
#[must_use]
pub fn deeper(d: OpenDepth) -> Option<OpenDepth> {
    match d {
        OpenDepth::Sentence => Some(OpenDepth::Capsule),
        OpenDepth::Capsule => Some(OpenDepth::Chart),
        OpenDepth::Chart => Some(OpenDepth::Evidence),
        OpenDepth::Evidence => None,
    }
}

/// One depth back; `None` at sentence.
#[must_use]
pub fn shallower(d: OpenDepth) -> Option<OpenDepth> {
    match d {
        OpenDepth::Sentence => None,
        OpenDepth::Capsule => Some(OpenDepth::Sentence),
        OpenDepth::Chart => Some(OpenDepth::Capsule),
        OpenDepth::Evidence => Some(OpenDepth::Chart),
    }
}

/// The presenter depth that draws an open depth (`evidence` is drawn at `chart`).
#[must_use]
pub fn presenter_depth(d: OpenDepth) -> Depth {
    match d {
        OpenDepth::Sentence => Depth::Sentence,
        OpenDepth::Capsule => Depth::Capsule,
        OpenDepth::Chart | OpenDepth::Evidence => Depth::Chart,
    }
}

/// The opening rule of a tap (R4): what a tap on a capsule's scene at depth `d` opens.
/// `item` is what the point query found under the finger (`None`: empty area).
///
/// - `sentence` (a face or sentence in the feed): anywhere → `capsule`.
/// - `capsule` (the opened capsule): anywhere → `chart`.
/// - `chart`: on an element of the evidence — a fact or finding layer (the fact point,
///   the glyph) or a layer with a constant whose origin is a fact (the level, band and
///   swing geometry) → `evidence`; anywhere else nothing opens (the tap focuses the bar).
/// - `evidence`: nothing opens.
#[must_use]
pub fn tap_opens(d: OpenDepth, item: Option<&HitItem>) -> Option<OpenDepth> {
    match d {
        OpenDepth::Sentence | OpenDepth::Capsule => deeper(d),
        OpenDepth::Chart => item.filter(|i| is_evidence(i)).map(|_| OpenDepth::Evidence),
        OpenDepth::Evidence => None,
    }
}

fn is_evidence(i: &HitItem) -> bool {
    matches!(i.data, DataSource::Fact { .. } | DataSource::Finding { .. })
        || i.constants
            .values()
            .any(|c| matches!(c.origin, Origin::Fact { .. }))
}

fn union(a: Option<PixelRect>, b: PixelRect) -> PixelRect {
    match a {
        None => b,
        Some(a) => PixelRect {
            left: a.left.min(b.left),
            top: a.top.min(b.top),
            right: a.right.max(b.right),
            bottom: a.bottom.max(b.bottom),
        },
    }
}

/// The elements of a capsule's scene that a depth transition can keep, by id, with their
/// boxes (R4, `docs/GRAMMAR.md` §18.6): `glyph` (a glyph layer), `headline` and `ledger`
/// (the finding's text layers), `fact:<id>` (a layer drawing a fact), `evidence:<fact>`
/// (layers whose constants come from a fact: level, band, swing), `price` (the panel of
/// the layer drawing `price_series`), `bar:<i>` (the column of the marked bar
/// `marked_bar` in that panel, when it is visible). A box is the union of the element's
/// item hit boxes.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn transition_anchors(
    scene: &SceneDesc,
    price_series: Option<&str>,
    marked_bar: Option<u64>,
) -> BTreeMap<String, PixelRect> {
    let mut out: BTreeMap<String, PixelRect> = BTreeMap::new();
    let put = |out: &mut BTreeMap<String, PixelRect>, k: String, r: PixelRect| {
        let prev = out.get(&k).copied();
        out.insert(k, union(prev, r));
    };
    let mut price_panel: Option<PixelRect> = None;
    for p in &scene.panels {
        for l in &p.layers {
            let mut keys: Vec<String> = Vec::new();
            match (&l.data, l.mark) {
                (_, Mark::Glyph) => keys.push("glyph".to_owned()),
                (DataSource::Finding { .. }, Mark::Text) => {
                    for f in ["headline", "ledger"] {
                        if l.fields.iter().any(|x| x == f) {
                            keys.push(f.to_owned());
                        }
                    }
                }
                (DataSource::Fact { fact }, _) => keys.push(format!("fact:{fact}")),
                (DataSource::Series { series }, Mark::Candle | Mark::Line)
                    if Some(series.as_str()) == price_series && price_panel.is_none() =>
                {
                    price_panel = Some(p.rect);
                    put(&mut out, "price".to_owned(), p.rect);
                }
                _ => {}
            }
            for c in l.constants.values() {
                if let Origin::Fact { fact, .. } = &c.origin {
                    let k = format!("evidence:{fact}");
                    if !keys.contains(&k) {
                        keys.push(k);
                    }
                }
            }
            for k in keys {
                for it in &l.items {
                    put(&mut out, k.clone(), it.hit);
                }
            }
        }
    }
    if let (Some(rect), Some(b)) = (price_panel, marked_bar) {
        if b >= scene.x.from_bar && b <= scene.x.to_bar {
            let c = crate::scale::slot_x(&scene.x, b);
            let half = (scene.x.slot / 2.0).max(1.0);
            put(
                &mut out,
                format!("bar:{b}"),
                PixelRect {
                    left: (c - half).floor() as i32,
                    top: rect.top,
                    right: (c + half).ceil() as i32,
                    bottom: rect.bottom,
                },
            );
        }
    }
    out
}

/// The elements a transition keeps (R4): the ids both scenes have, in id order, with
/// both boxes. A side without a scene (`None`, the sentence depth) still shows the
/// sentence: `headline` is kept with no box on that side.
#[must_use]
pub fn shared_elements(
    before: Option<&BTreeMap<String, PixelRect>>,
    after: Option<&BTreeMap<String, PixelRect>>,
) -> Vec<SharedElement> {
    match (before, after) {
        (Some(b), Some(a)) => b
            .iter()
            .filter_map(|(k, rb)| {
                a.get(k).map(|ra| SharedElement {
                    id: k.clone(),
                    before: Some(*rb),
                    after: Some(*ra),
                })
            })
            .collect(),
        (b, a) => vec![SharedElement {
            id: "headline".to_owned(),
            before: b.and_then(|m| m.get("headline").copied()),
            after: a.and_then(|m| m.get("headline").copied()),
        }],
    }
}
