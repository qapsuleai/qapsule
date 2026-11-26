//! The legend of program series (PS §1.4): each layer `show` makes is labelled with its
//! series id, so the core's legend names the program output.

use crate::rig::{H, SMA_TEXT};

/// Every text of view `main` as connection `a` sees it.
fn texts(h: &mut H) -> String {
    let scene =
        h.s.scene_for("a", Some("main"), &qp_designs::options(1200, 600))
            .unwrap();
    serde_json::to_string(&scene).unwrap()
}

/// Gated: the scene compiles once the data plane computes program series (`qp-plane`,
/// RL1 task P/plane, a parallel branch); until then the layer's field is unknown.
#[test]
#[ignore = "needs qp-plane program series values (parallel RL1 branch); passes after merge"]
fn the_legend_names_the_program_series() {
    let mut h = H::new();
    let r = h.add("p1", SMA_TEXT);
    assert_eq!(r["status"], "applied", "{r}");
    let t = texts(&mut h);
    assert!(t.contains("p1.avg"), "legend of the series output");
}

/// The label the legend reads is on the layer (document level; no data plane needed).
#[test]
fn shown_layers_carry_the_series_id_as_label() {
    let mut h = H::new();
    h.add("p1", SMA_TEXT);
    let ls = h.layers();
    for id in ["p1.avg", "p1.up"] {
        let l = ls.iter().find(|l| l["series"] == id).unwrap();
        assert_eq!(l["label"], id);
    }
}
