//! R4/T6: the replay host's history window (`ReplayHost::with_window`, `qp demo --history`
//! with `--from`).

use std::path::Path;

use qp_schema::protocol::{ReplayInit, ReplaySourceKind};
use qp_session::{demo_init, ReplayHost};

fn init(history: u32, from_t: Option<i64>) -> qp_schema::protocol::SessionInit {
    demo_init(ReplayInit {
        source: ReplaySourceKind::Synthetic,
        instruments: Some(3),
        days: Some(200),
        history: Some(history),
        from_t,
        ..ReplayInit::default()
    })
}

fn none() -> std::path::PathBuf {
    std::env::temp_dir().join("qp-replay-window-no-local-data")
}

fn bars_per_series(h: &ReplayHost) -> Vec<usize> {
    let d = h.init().data.as_ref().expect("data");
    d.feeds
        .iter()
        .map(|f| f.bars.as_ref().map_or(0, Vec::len))
        .collect()
}

#[test]
fn window_bounds_the_history_before_from_and_keeps_the_replay() {
    // The first replayed day: after 60 bars of history.
    let probe = ReplayHost::with_local_dir(init(60, None), Path::new("."), &none()).expect("host");
    let from = probe.state().from_t.expect("first replayed bar");
    let full =
        ReplayHost::with_window(init(20, Some(from)), Path::new("."), &none(), None).expect("host");
    let win = ReplayHost::with_window(init(20, Some(from)), Path::new("."), &none(), Some(20))
        .expect("host");
    let (f, w) = (bars_per_series(&full), bars_per_series(&win));
    assert!(!f.is_empty() && f.len() == w.len(), "{f:?} {w:?}");
    assert!(f.iter().all(|&n| n == 60), "{f:?}");
    assert!(w.iter().all(|&n| n == 20), "{w:?}");
    // The replayed part is the same: same first and last time, same steps.
    let (sf, sw) = (full.state(), win.state());
    assert_eq!(
        (sf.from_t, sf.to_t, sf.steps),
        (sw.from_t, sw.to_t, sw.steps)
    );
}

#[test]
fn window_without_from_changes_nothing() {
    let a = ReplayHost::with_local_dir(init(20, None), Path::new("."), &none()).expect("host");
    let b =
        ReplayHost::with_window(init(20, None), Path::new("."), &none(), Some(5)).expect("host");
    assert_eq!(bars_per_series(&a), bars_per_series(&b));
    assert_eq!(a.state(), b.state());
}
