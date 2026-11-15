//! Data plane (RP2, `docs/PROTOCOL.md` §18): feed store rules, incremental digest, window
//! resolution, binding validation, the memory provider and the field-level synthetic
//! stream (same final bits as the whole-bar path; streaming studies equal batch).

use std::collections::BTreeMap;
use std::path::PathBuf;

use qp_compute::{create, run, Field};
use qp_schema::data::{
    data_digest, data_digest_extend, data_digest_u64, BarPatch, BindingState, DataBinding,
    DataChange, DataEvent, FeedKey, Window, DATA_DIGEST_OFFSET,
};
use qp_schema::protocol::InitData;
use qp_schema::Bar;
use qp_source::{
    bind, collect_bars, data_events_from_bar_events, entitlement_matches, resolve_window,
    synthetic_bars, validate_binding, validate_entitlement, validate_id, validate_interval,
    Applied, BarEvent, EventError, FeedStore, MemoryProvider, Resolution, SeriesSource,
    SourceProvider, StaleReason, SyntheticEvents, SyntheticEventsConfig, SyntheticSource,
    TimeBounds,
};

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

fn key() -> FeedKey {
    FeedKey {
        symbol: "THYAO".into(),
        interval: "1d".into(),
        feed: None,
    }
}

fn ev(seq: u64, change: DataChange) -> DataEvent {
    DataEvent {
        feed: key(),
        seq,
        change,
    }
}

fn bar(t: i64, c: f64) -> Bar {
    Bar {
        t,
        o: c,
        h: c + 1.0,
        l: c - 1.0,
        c,
        v: 10.0,
    }
}

fn new_bar(seq: u64, t: i64, c: f64) -> DataEvent {
    ev(seq, DataChange::Bar { bar: bar(t, c) })
}

fn patch(seq: u64, t: i64, fields: BarPatch) -> DataEvent {
    ev(seq, DataChange::Patch { t, fields })
}

fn c(x: f64) -> BarPatch {
    BarPatch {
        c: Some(x),
        ..BarPatch::default()
    }
}

fn bits(bars: &[Bar]) -> Vec<[u64; 6]> {
    bars.iter()
        .map(|b| {
            [
                u64::from_le_bytes(b.t.to_le_bytes()),
                b.o.to_bits(),
                b.h.to_bits(),
                b.l.to_bits(),
                b.c.to_bits(),
                b.v.to_bits(),
            ]
        })
        .collect()
}

type ErrCheck = fn(&EventError) -> bool;

fn store3() -> FeedStore {
    FeedStore::with_bars(key(), vec![bar(10, 5.0), bar(20, 6.0), bar(30, 7.0)], 0).unwrap()
}

// ---------------------------------------------------------------------------
// FeedStore rules

#[test]
fn new_bar_live_update_and_correction_report_indices() {
    let mut s = store3();
    assert_eq!(
        s.apply(&new_bar(1, 40, 8.0)).unwrap(),
        Applied::NewBar { index: 3 }
    );
    assert_eq!(
        s.apply(&patch(2, 40, c(8.5))).unwrap(),
        Applied::LiveUpdate { index: 3 }
    );
    assert_eq!(
        s.apply(&patch(3, 20, c(6.5))).unwrap(),
        Applied::Correction { index: 1 }
    );
    assert_eq!(s.bars()[1].c, 6.5);
    assert_eq!(s.bars()[3].c, 8.5);
    assert_eq!(s.seq(), 3);
    assert_eq!(s.index_of(30), Some(2));
    assert_eq!(s.index_of(31), None);
}

#[test]
fn duplicates_are_ignored() {
    let mut s = store3();
    s.apply(&new_bar(1, 40, 8.0)).unwrap();
    let before = s.bars().to_vec();
    // Same seq again, and an older seq with different content: both ignored.
    assert_eq!(s.apply(&new_bar(1, 40, 8.0)).unwrap(), Applied::Duplicate);
    assert_eq!(s.apply(&patch(1, 40, c(9.0))).unwrap(), Applied::Duplicate);
    assert_eq!(s.apply(&patch(0, 40, c(9.0))).unwrap(), Applied::Duplicate);
    assert_eq!(s.bars(), &before[..]);
    assert_eq!(s.seq(), 1);
}

#[test]
fn gap_marks_stale_until_reset() {
    let mut s = store3();
    s.apply(&new_bar(1, 40, 8.0)).unwrap();
    assert_eq!(
        s.apply(&new_bar(3, 50, 9.0)).unwrap(),
        Applied::Gap {
            expected: 2,
            got: 3
        }
    );
    assert_eq!(s.len(), 4, "gap event not applied");
    assert_eq!(
        s.stale(),
        Some(StaleReason::Gap {
            expected: 2,
            got: 3
        })
    );
    assert_eq!(s.state(), BindingState::Stale);
    assert!(s.stale().unwrap().reason().starts_with("gap"));
    // Even the missing seq is not applied once stale: the host must re-resolve.
    assert_eq!(
        s.apply(&new_bar(2, 50, 9.0)).unwrap(),
        Applied::Stale(StaleReason::Gap {
            expected: 2,
            got: 3
        })
    );
    assert_eq!(s.seq(), 1);
    assert_eq!(s.apply(&new_bar(1, 50, 9.0)).unwrap(), Applied::Duplicate);
    s.reset(synthetic_bars(5, 1), 7).unwrap();
    assert_eq!(s.stale(), None);
    assert_eq!(s.seq(), 7);
    let t = s.bars()[4].t + 1;
    assert_eq!(
        s.apply(&new_bar(8, t, 100.0)).unwrap(),
        Applied::NewBar { index: 5 }
    );
}

#[test]
fn invalid_events_are_rejected_without_side_effects() {
    let mut s = store3();
    let before = s.bars().to_vec();
    let cases: Vec<(DataEvent, ErrCheck)> = vec![
        (new_bar(1, 30, 1.0), |e| {
            matches!(e, EventError::BarNotAfterLast { t: 30, last_t: 30 })
        }),
        (new_bar(1, 5, 1.0), |e| {
            matches!(e, EventError::BarNotAfterLast { .. })
        }),
        (patch(1, 25, c(1.0)), |e| {
            matches!(e, EventError::UnknownT { t: 25 })
        }),
        (patch(1, 99, c(1.0)), |e| {
            matches!(e, EventError::UnknownT { t: 99 })
        }),
        (patch(1, 20, BarPatch::default()), |e| {
            matches!(e, EventError::EmptyPatch { t: 20 })
        }),
        (patch(1, 20, c(f64::NAN)), |e| {
            matches!(e, EventError::NonFinite { field: "c", .. })
        }),
        (
            patch(
                1,
                20,
                BarPatch {
                    v: Some(f64::INFINITY),
                    ..BarPatch::default()
                },
            ),
            |e| matches!(e, EventError::NonFinite { field: "v", .. }),
        ),
        // c above h without raising h.
        (patch(1, 20, c(100.0)), |e| {
            matches!(e, EventError::InconsistentOhlc { t: 20 })
        }),
        (
            ev(
                1,
                DataChange::Bar {
                    bar: Bar {
                        t: 40,
                        o: 1.0,
                        h: 0.5,
                        l: 0.0,
                        c: 1.0,
                        v: 0.0,
                    },
                },
            ),
            |e| matches!(e, EventError::InconsistentOhlc { t: 40 }),
        ),
        (
            DataEvent {
                feed: FeedKey {
                    symbol: "GARAN".into(),
                    ..key()
                },
                seq: 1,
                change: DataChange::Bar { bar: bar(40, 1.0) },
            },
            |e| matches!(e, EventError::WrongFeed { .. }),
        ),
    ];
    for (e, ok) in cases {
        let err = s.apply(&e).unwrap_err();
        assert!(ok(&err), "{e:?} -> {err:?}");
        assert!(!err.to_string().is_empty());
        assert_eq!(s.bars(), &before[..]);
        assert_eq!(s.seq(), 0, "a rejected event does not advance seq");
    }
    // The rejected seq 1 was never applied, so seq 2 is a gap (the store missed a step).
    assert!(matches!(
        s.apply(&new_bar(2, 40, 8.0)).unwrap(),
        Applied::Gap { expected: 1, .. }
    ));
}

#[test]
fn initial_bars_are_checked() {
    assert!(FeedStore::with_bars(key(), vec![bar(2, 1.0), bar(2, 1.0)], 0).is_err());
    assert!(FeedStore::with_bars(key(), vec![bar(2, f64::NAN)], 0).is_err());
    assert!(FeedStore::with_bars(key(), vec![], 0).unwrap().is_empty());
    let mut s = FeedStore::new(key());
    assert!(s.position().last_t.is_none());
    assert_eq!(s.position().digest, data_digest(&[]));
}

#[test]
fn window_bounds_turn_outside_events_into_no_ops() {
    // A closed window [20, 30] anchored at 20.
    let mut s = FeedStore::with_bars(key(), vec![bar(20, 6.0), bar(30, 7.0)], 4)
        .unwrap()
        .with_bounds(TimeBounds {
            from: Some(20),
            to: Some(30),
        });
    assert_eq!(
        s.apply(&new_bar(5, 40, 8.0)).unwrap(),
        Applied::OutsideWindow { t: 40 }
    );
    assert_eq!(
        s.apply(&patch(6, 10, c(5.0))).unwrap(),
        Applied::OutsideWindow { t: 10 }
    );
    assert_eq!(s.seq(), 6, "outside events advance seq");
    assert_eq!(s.len(), 2);
    assert_eq!(
        s.apply(&patch(7, 20, c(6.5))).unwrap(),
        Applied::Correction { index: 0 }
    );
    // Inside the bounds, an unknown t is still an error.
    assert!(matches!(
        s.apply(&patch(8, 25, c(1.0))),
        Err(EventError::UnknownT { t: 25 })
    ));
}

// ---------------------------------------------------------------------------
// Digest

#[test]
fn incremental_store_uses_the_schema_digest_step() {
    let bars = synthetic_bars(300, 4);
    let h = bars.iter().fold(DATA_DIGEST_OFFSET, data_digest_extend);
    assert_eq!(h, data_digest_u64(&bars));
}

#[test]
fn incremental_digest_equals_from_scratch_under_random_operations() {
    let mut s = FeedStore::with_bars(key(), synthetic_bars(50, 2), 0).unwrap();
    let mut rng = 0x1234_5678_u64;
    let mut next = || {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        rng >> 33
    };
    let mut seq = 0;
    for step in 0..3000 {
        seq += 1;
        let n = s.len();
        let r = next();
        let e = match r % 4 {
            0 => {
                let t = s.bars()[n - 1].t + 60_000;
                new_bar(seq, t, 50.0 + (r % 100) as f64)
            }
            1 => {
                let b = &s.bars()[n - 1];
                patch(seq, b.t, c(b.l))
            }
            2 => {
                let i = (next() as usize) % n;
                let b = &s.bars()[i];
                patch(
                    seq,
                    b.t,
                    BarPatch {
                        v: Some(b.v + 1.0),
                        c: Some(b.h),
                        ..BarPatch::default()
                    },
                )
            }
            _ => {
                let b = &s.bars()[n - 1];
                patch(
                    seq,
                    b.t,
                    BarPatch {
                        h: Some(b.h + 0.5),
                        c: Some(b.h + 0.5),
                        ..BarPatch::default()
                    },
                )
            }
        };
        s.apply(&e).unwrap();
        // Ask for positions irregularly (the lazy prefix must stay right either way).
        if step % 7 == 0 || r % 5 == 0 {
            let p = s.position();
            assert_eq!(p.digest, data_digest(s.bars()), "step {step}");
            assert_eq!(p.bars as usize, s.len());
            assert_eq!(p.seq, seq);
            assert_eq!(p.last_t, s.bars().last().map(|b| b.t));
            let k = (next() as usize) % (s.len() + 1);
            assert_eq!(s.prefix_digest_u64(k), data_digest_u64(&s.bars()[..k]));
        }
    }
}

// ---------------------------------------------------------------------------
// Window resolution

fn tbars(ts: &[i64]) -> Vec<Bar> {
    ts.iter().map(|&t| bar(t, 5.0)).collect()
}

fn win(last: Option<u32>, from_t: Option<i64>, to_t: Option<i64>) -> Window {
    Window { last, from_t, to_t }
}

#[test]
fn window_resolution_rules() {
    let b = tbars(&[10, 20, 30, 40, 50]);
    let r = resolve_window(&b, &Window::last(2), None, 100).unwrap();
    assert_eq!(r.range, 3..5);
    assert_eq!(r.anchor_t, Some(40));
    assert!(r.accepts_live && r.capped.is_none());
    assert_eq!(r.bounds.from, Some(40));
    let r = resolve_window(&b, &Window::last(99), None, 100).unwrap();
    assert_eq!(r.range, 0..5);

    let r = resolve_window(&b, &win(None, Some(25), None), None, 100).unwrap();
    assert_eq!((r.range.clone(), r.anchor_t), (2..5, Some(30)));
    assert_eq!(r.slice(&b).len(), 3);
    let r = resolve_window(&b, &win(None, Some(20), Some(40)), None, 100).unwrap();
    assert_eq!(r.range, 1..4);
    assert!(!r.accepts_live, "closed windows take no live bars");
    assert_eq!(
        r.bounds,
        TimeBounds {
            from: Some(20),
            to: Some(40)
        }
    );
    let r = resolve_window(&b, &win(None, Some(41), Some(49)), None, 100).unwrap();
    assert!(r.is_empty() && r.anchor_t.is_none());
    assert_eq!(r.bounds.from, Some(41));
    let r = resolve_window(&b, &win(None, Some(60), None), None, 100).unwrap();
    assert!(r.is_empty());

    // Cap to the latest max_bind_bars bars, with a reason.
    let r = resolve_window(&b, &win(None, Some(0), None), None, 2).unwrap();
    assert_eq!((r.range.clone(), r.anchor_t), (3..5, Some(40)));
    assert!(r.capped.unwrap().contains("max_bind_bars"));
    let r = resolve_window(&b, &win(None, Some(10), Some(40)), None, 3).unwrap();
    assert_eq!(r.range, 1..4);
    assert!(r.capped.is_some());

    // Window::validate rules.
    for bad in [
        Window::last(0),
        Window::last(101),
        win(Some(2), Some(1), None),
        win(None, Some(5), Some(4)),
        win(None, None, Some(4)),
        win(None, None, None),
    ] {
        assert!(resolve_window(&b, &bad, None, 100).is_err(), "{bad:?}");
    }
}

// ---------------------------------------------------------------------------
// Binding validation

#[test]
fn interval_rules_match_qp_calendar() {
    let mut cands: Vec<String> = Vec::new();
    for unit in ["m", "h", "d", "w", "M", "s", "y", "D", "H", "", "mm", "m1"] {
        for n in [
            "0",
            "1",
            "2",
            "5",
            "15",
            "24",
            "25",
            "60",
            "1440",
            "1441",
            "01",
            "007",
            "",
            "99999999999",
        ] {
            cands.push(format!("{n}{unit}"));
        }
    }
    cands.extend(["", " 1d", "1d ", "-1m", "+1m", "1.5h", "１d"].map(str::to_owned));
    for s in &cands {
        assert_eq!(
            validate_interval(s).is_ok(),
            qp_calendar::Interval::parse(s).is_ok(),
            "{s:?}"
        );
    }
}

#[test]
fn id_and_binding_rules() {
    for ok in ["THYAO", "a", "X.U-1_b", &"A".repeat(32)] {
        assert!(validate_id("symbol", ok).is_ok(), "{ok}");
    }
    for bad in ["", "_A", ".A", "-A", "A B", "A/B", "ŞEKER", &"A".repeat(33)] {
        assert!(validate_id("symbol", bad).is_err(), "{bad}");
    }
    let mut b = DataBinding {
        symbol: "THYAO".into(),
        interval: "1d".into(),
        window: Window::last(10),
        feed: Some("delayed".into()),
        anchor_t: None,
    };
    assert!(validate_binding(&b, 100).is_ok());
    b.feed = Some("bad feed".into());
    assert!(validate_binding(&b, 100).unwrap_err().contains("feed"));
    b.feed = None;
    b.interval = "2d".into();
    assert!(validate_binding(&b, 100).unwrap_err().contains("interval"));
    b.interval = "1d".into();
    b.window = Window::last(1000);
    assert!(validate_binding(&b, 100).is_err());
}

// ---------------------------------------------------------------------------
// Memory provider

fn init(json: &str) -> InitData {
    serde_json::from_str(json).unwrap()
}

fn binding(symbol: &str, window: Window) -> DataBinding {
    DataBinding {
        symbol: symbol.into(),
        interval: "1d".into(),
        window,
        feed: None,
        anchor_t: None,
    }
}

#[test]
fn memory_provider_from_init() {
    let data = init(
        r#"{
        "feeds": [
          {"feed": {"symbol": "THYAO", "interval": "1d"}, "synthetic": {"n": 300, "seed": 7}},
          {"feed": {"symbol": "GARAN", "interval": "1d"}, "path": "fixtures/bars/sample.json"},
          {"feed": {"symbol": "AKBNK", "interval": "1d"},
           "bars": [{"t": 1, "o": 1, "h": 2, "l": 0.5, "c": 1.5, "v": 9}]},
          {"feed": {"symbol": "ISCTR", "interval": "1d"}, "bars": []}
        ],
        "entitlements": {"host": ["*"], "ayse": ["THYAO"]},
        "pending": [{"symbol": "AKBNK", "interval": "1d"}]
      }"#,
    );
    let p = MemoryProvider::from_init(&data, &repo("")).unwrap();
    let thyao = binding("THYAO", Window::last(100));

    // Entitlement: "*" sees all; listed symbol; unlisted principal and symbol: none.
    assert!(p.entitled("host", &thyao.feed_key()));
    assert!(p.entitled("ayse", &thyao.feed_key()));
    assert!(!p.entitled("ayse", &binding("GARAN", Window::last(1)).feed_key()));
    assert!(!p.entitled("mehmet", &thyao.feed_key()));
    assert_eq!(
        p.resolve("mehmet", &thyao).state(),
        BindingState::NoEntitlement
    );
    match p.resolve("ayse", &thyao) {
        Resolution::Ready { bars, seq } => {
            assert_eq!(bars, synthetic_bars(300, 7)[200..].to_vec());
            assert_eq!(seq, 0);
        }
        other => panic!("{other:?}"),
    }
    match p.resolve("host", &binding("GARAN", Window::last(100_000))) {
        Resolution::Ready { bars, .. } => assert_eq!(bars, synthetic_bars(200, 20_251_114)),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        p.resolve("host", &binding("AKBNK", Window::last(1))),
        Resolution::Pending
    );
    // Entitlement is checked before pending (no leak of the feed's existence).
    assert_eq!(
        p.resolve("ayse", &binding("AKBNK", Window::last(1)))
            .state(),
        BindingState::NoEntitlement
    );
    assert_eq!(
        p.resolve("host", &binding("NOPE", Window::last(1))).state(),
        BindingState::NoData
    );
    assert_eq!(
        p.resolve("host", &binding("ISCTR", Window::last(1)))
            .state(),
        BindingState::NoData
    );
    let mut daily_feed = binding("THYAO", Window::last(1));
    daily_feed.feed = Some("delayed".into());
    assert_eq!(p.resolve("host", &daily_feed).state(), BindingState::NoData);
}

#[test]
fn init_feed_rules_and_path_confinement() {
    let one = |feed: &str| init(&format!(r#"{{"feeds": [{feed}]}}"#));
    let k = r#""feed": {"symbol": "X", "interval": "1d"}"#;
    for bad in [
        format!("{{{k}}}"),
        format!(r#"{{{k}, "bars": [], "synthetic": {{"n": 1, "seed": 1}}}}"#),
        format!(r#"{{{k}, "path": "../qapsule/fixtures/bars/sample.json"}}"#),
        format!(r#"{{{k}, "path": "/etc/hosts"}}"#),
        format!(r#"{{{k}, "path": "fixtures/nope.json"}}"#),
        r#"{"feed": {"symbol": "X", "interval": "7d"}, "bars": []}"#.to_owned(),
        r#"{"feed": {"symbol": "-X", "interval": "1d"}, "bars": []}"#.to_owned(),
        format!(r#"{{{k}, "bars": [{{"t": 1, "o": 1, "h": 0, "l": 0, "c": 1, "v": 0}}]}}"#),
    ] {
        assert!(
            MemoryProvider::from_init(&one(&bad), &repo("")).is_err(),
            "{bad}"
        );
    }
    let dup = init(&format!(
        r#"{{"feeds": [{{{k}, "bars": []}}, {{{k}, "bars": []}}]}}"#
    ));
    assert!(MemoryProvider::from_init(&dup, &repo("")).is_err());
    // Root confinement: the same file is fine under its own root, not outside it.
    let rel = one(&format!(r#"{{{k}, "path": "bars/sample.json"}}"#));
    assert!(MemoryProvider::from_init(&rel, &repo("fixtures")).is_ok());
    assert!(MemoryProvider::from_init(&rel, &repo("fixtures/schema")).is_err());
}

#[test]
fn provider_takes_events_and_bind_applies_window_and_cap() {
    let mut p = MemoryProvider::new();
    p.add_feed(key(), synthetic_bars(100, 3)).unwrap();
    assert!(p.add_feed(key(), vec![]).is_err());
    p.entitle("host", ["THYAO".to_owned()]).unwrap();
    let stream = SyntheticEvents::new(
        key(),
        SyntheticEventsConfig {
            start: 100,
            ..SyntheticEventsConfig::new(110, 3, 4)
        },
    );
    let mut last_seq = 0;
    for e in stream {
        last_seq = e.seq;
        p.apply(&e).unwrap();
    }
    assert!(matches!(
        p.apply(&DataEvent {
            feed: FeedKey {
                symbol: "NOPE".into(),
                ..key()
            },
            seq: 1,
            change: DataChange::Bar { bar: bar(1, 1.0) },
        }),
        Err(EventError::UnknownFeed { .. })
    ));
    assert_eq!(
        bits(p.feed(&key()).unwrap().bars()),
        bits(&synthetic_bars(110, 3))
    );

    // A later resolution sees the events.
    let b = binding("THYAO", win(None, Some(0), None));
    let bound = bind(&p, "host", &b, 50).unwrap();
    assert_eq!(bound.state, BindingState::Ready);
    assert!(bound.reason.unwrap().contains("110 bars"));
    let mut store = bound.store.unwrap();
    let all = synthetic_bars(110, 3);
    assert_eq!(store.bars(), &all[60..]);
    assert_eq!(bound.anchor_t, Some(all[60].t));
    assert_eq!(store.seq(), last_seq);
    assert_eq!(store.position().digest, data_digest(&all[60..]));
    // The bound store refuses a correction before its anchor as outside, not an error.
    assert!(matches!(
        store.apply(&patch(last_seq + 1, all[10].t, c(all[10].h))),
        Ok(Applied::OutsideWindow { .. })
    ));

    // Not ready states and invalid bindings.
    assert_eq!(
        bind(&p, "nobody", &b, 50).unwrap().state,
        BindingState::NoEntitlement
    );
    let empty = binding("THYAO", win(None, Some(i64::MAX - 1), None));
    let r = bind(&p, "host", &empty, 50).unwrap();
    assert_eq!(r.state, BindingState::NoData);
    assert!(r.reason.is_some() && r.store.is_none());
    assert!(bind(&p, "host", &binding("TH YAO", Window::last(1)), 50).is_err());
    assert!(bind(&p, "host", &binding("THYAO", Window::last(51)), 50).is_err());
    p.set_pending(key());
    assert_eq!(
        bind(&p, "host", &b, 50).unwrap().state,
        BindingState::Pending
    );
}

/// A provider that returns more bars than asked and out of window: `bind` trims.
struct Sloppy;
impl SourceProvider for Sloppy {
    fn entitled(&self, _: &str, _: &FeedKey) -> bool {
        true
    }
    fn resolve(&self, _: &str, _: &DataBinding) -> Resolution {
        Resolution::Ready {
            bars: tbars(&[10, 20, 30, 40, 50]),
            seq: 9,
        }
    }
}

#[test]
fn bind_enforces_the_window_on_provider_answers() {
    let provider: &dyn SourceProvider = &Sloppy;
    let b = bind(
        provider,
        "x",
        &binding("A", win(None, Some(20), Some(40))),
        100,
    )
    .unwrap();
    let s = b.store.unwrap();
    assert_eq!(
        s.bars().iter().map(|b| b.t).collect::<Vec<_>>(),
        [20, 30, 40]
    );
    assert_eq!(s.bounds().to, Some(40));
    assert_eq!(s.seq(), 9);
    let b = bind(provider, "x", &binding("A", Window::last(2)), 100).unwrap();
    assert_eq!(b.anchor_t, Some(40));
}

// ---------------------------------------------------------------------------
// Field-level synthetic stream

fn apply_all(store: &mut FeedStore, events: impl IntoIterator<Item = DataEvent>) -> Vec<Applied> {
    events
        .into_iter()
        .map(|e| store.apply(&e).unwrap())
        .collect()
}

#[test]
fn field_level_stream_yields_the_whole_bar_path_bit_for_bit() {
    for ticks in [1, 2, 3, 5, 8, 20] {
        let old = collect_bars(&mut SyntheticSource::new(400, 11, ticks)).unwrap();
        for (every, after) in [(0, 3), (7, 3), (5, 1), (13, 40)] {
            let cfg = SyntheticEventsConfig {
                correct_every: every,
                correct_after: after,
                ..SyntheticEventsConfig::new(400, 11, ticks)
            };
            let mut store = FeedStore::new(key());
            let applied = apply_all(&mut store, SyntheticEvents::new(key(), cfg));
            assert_eq!(
                bits(store.bars()),
                bits(&old),
                "ticks {ticks} every {every}"
            );
            let news = applied
                .iter()
                .filter(|a| matches!(a, Applied::NewBar { .. }))
                .count();
            let corrections = applied
                .iter()
                .filter(|a| matches!(a, Applied::Correction { .. }))
                .count();
            assert_eq!(news, 400);
            if every == 0 {
                assert_eq!(corrections, 0);
            } else {
                assert!(corrections >= 400 / every - 1, "{corrections}");
            }
            assert_eq!(store.seq() as usize, applied.len());
        }
    }
}

#[test]
fn patches_carry_only_changed_fields() {
    let mut store = FeedStore::new(key());
    let mut patches = 0;
    let mut hl_patches = 0;
    for e in SyntheticEvents::new(key(), SyntheticEventsConfig::new(200, 5, 10)) {
        if let DataChange::Patch { t, fields } = &e.change {
            let b = store.bars()[store.index_of(*t).unwrap()].clone();
            assert!(!fields.is_empty());
            assert!(fields.o.is_none(), "open never changes");
            for (new, old) in [
                (fields.h, b.h),
                (fields.l, b.l),
                (fields.c, b.c),
                (fields.v, b.v),
            ] {
                assert!(new.is_none_or(|x| x.to_bits() != old.to_bits()));
            }
            // h only when exceeded, l only when undercut.
            assert!(fields.h.is_none_or(|h| h > b.h));
            assert!(fields.l.is_none_or(|l| l < b.l));
            patches += 1;
            hl_patches += usize::from(fields.h.is_some() || fields.l.is_some());
        }
        store.apply(&e).unwrap();
    }
    assert!(patches > 200 * 7, "{patches}");
    assert!(hl_patches < patches / 2, "h/l are sent only when exceeded");
    assert_eq!(store.bars(), &synthetic_bars(200, 5)[..]);
}

#[test]
fn stream_can_start_after_initial_bars_and_is_deterministic() {
    let cfg = SyntheticEventsConfig {
        start: 150,
        first_seq: 1,
        ..SyntheticEventsConfig::new(180, 21, 6)
    };
    let s = SyntheticEvents::new(key(), cfg);
    assert_eq!(s.initial_bars(), &synthetic_bars(150, 21)[..]);
    assert_eq!(s.final_bars(), &synthetic_bars(180, 21)[..]);
    let a: Vec<DataEvent> = s.clone().collect();
    let b: Vec<DataEvent> = SyntheticEvents::new(key(), cfg).collect();
    assert_eq!(a, b);
    assert!(a.iter().zip(1..).all(|(e, s)| e.seq == s));
    let mut store = FeedStore::with_bars(key(), synthetic_bars(150, 21), 0).unwrap();
    apply_all(&mut store, a);
    assert_eq!(store.bars(), &synthetic_bars(180, 21)[..]);
    // JSON round trip of the events (wire form of §18.4).
    for e in SyntheticEvents::new(key(), cfg).take(50) {
        let j = serde_json::to_string(&e).unwrap();
        assert_eq!(serde_json::from_str::<DataEvent>(&j).unwrap(), e);
    }
}

#[test]
fn converter_from_bar_events() {
    let mut src = SyntheticSource::new(120, 8, 5);
    let mut evs = Vec::new();
    while let Some(e) = src.next_event().unwrap() {
        evs.push(e);
    }
    let data = data_events_from_bar_events(&key(), evs.clone(), 1).unwrap();
    assert!(data.iter().zip(1..).all(|(e, s)| e.seq == s));
    assert!(data.iter().all(|e| match &e.change {
        DataChange::Patch { fields, .. } => !fields.is_empty() && fields.o.is_none(),
        DataChange::Bar { .. } => true,
    }));
    let mut store = FeedStore::new(key());
    apply_all(&mut store, data);
    assert_eq!(bits(store.bars()), bits(&synthetic_bars(120, 8)));
    // A no-op update produces no event; a bad update is an error.
    let b = synthetic_bars(1, 1)[0].clone();
    let d = data_events_from_bar_events(
        &key(),
        [BarEvent::New(b.clone()), BarEvent::UpdateLast(b.clone())],
        1,
    )
    .unwrap();
    assert_eq!(d.len(), 1);
    assert!(data_events_from_bar_events(&key(), [BarEvent::UpdateLast(b)], 1).is_err());
}

/// Streaming studies fed from the field-level stream equal batch computation over the
/// final bars, bit for bit (close, high and low inputs: h/l move by patches too).
#[test]
fn streaming_studies_from_field_level_events_equal_batch() {
    let final_bars = synthetic_bars(500, 13);
    for field in [Field::Close, Field::High, Field::Low, Field::Volume] {
        for name in qp_compute::STUDY_NAMES {
            let mut batch_study = create(name, &BTreeMap::new(), field).unwrap();
            let col = field.column(&final_bars);
            let batch = run(batch_study.as_mut(), &[&col]).unwrap();

            let mut study = create(name, &BTreeMap::new(), field).unwrap();
            let mut out = vec![None; study.outputs().len()];
            let mut live: Vec<Vec<Option<f64>>> = vec![Vec::new(); out.len()];
            let mut store = FeedStore::new(key());
            for e in SyntheticEvents::new(key(), SyntheticEventsConfig::new(500, 13, 7)) {
                match store.apply(&e).unwrap() {
                    Applied::NewBar { index } => {
                        if index > 0 {
                            for (c, v) in live.iter_mut().zip(&out) {
                                c.push(*v);
                            }
                        }
                        study.push(&[field.of(&store.bars()[index])], &mut out);
                    }
                    Applied::LiveUpdate { index } => {
                        study.update_last(&[field.of(&store.bars()[index])], &mut out);
                    }
                    other => panic!("unexpected {other:?}"),
                }
            }
            for (c, v) in live.iter_mut().zip(&out) {
                c.push(*v);
            }
            let b = |v: &Vec<Vec<Option<f64>>>| -> Vec<Vec<Option<u64>>> {
                v.iter()
                    .map(|c| c.iter().map(|x| x.map(f64::to_bits)).collect())
                    .collect()
            };
            assert_eq!(b(&live), b(&batch), "{name} {}", field.name());
        }
    }
}

/// With corrections, a study recomputed from the corrected bar (reset + replay) still
/// ends equal to batch.
#[test]
fn corrections_replayed_from_their_bar_equal_batch() {
    let final_bars = synthetic_bars(300, 17);
    let cfg = SyntheticEventsConfig {
        correct_every: 9,
        correct_after: 4,
        ..SyntheticEventsConfig::new(300, 17, 3)
    };
    let mut study = create("rsi", &BTreeMap::new(), Field::Close).unwrap();
    let mut out = vec![None; 1];
    let mut store = FeedStore::new(key());
    let mut corrections = 0;
    for e in SyntheticEvents::new(key(), cfg) {
        match store.apply(&e).unwrap() {
            Applied::NewBar { index } => study.push(&[store.bars()[index].c], &mut out),
            Applied::LiveUpdate { index } => {
                study.update_last(&[store.bars()[index].c], &mut out);
            }
            Applied::Correction { .. } => {
                corrections += 1;
                study.reset();
                let col = Field::Close.column(store.bars());
                study.extend(&[&col]);
            }
            other => panic!("{other:?}"),
        }
    }
    assert!(corrections > 25);
    let mut batch_study = create("rsi", &BTreeMap::new(), Field::Close).unwrap();
    let batch = run(batch_study.as_mut(), &[&Field::Close.column(&final_bars)]).unwrap();
    // Revise the last bar once to read the current output.
    let last = final_bars.last().unwrap().c;
    study.update_last(&[last], &mut out);
    assert_eq!(
        out[0].map(f64::to_bits),
        batch[0].last().copied().flatten().map(f64::to_bits)
    );
}

// ---------------------------------------------------------------------------
// After the T2 audit: anchor, stale, entitlement forms, live growth cap

fn anchored(symbol: &str, window: Window, anchor_t: i64) -> DataBinding {
    DataBinding {
        anchor_t: Some(anchor_t),
        ..binding(symbol, window)
    }
}

#[test]
fn anchored_window_resolution() {
    let b = tbars(&[10, 20, 30, 40, 50]);
    // `last` is only the first resolution's size: the anchor keeps the start.
    let r = resolve_window(&b, &Window::last(2), Some(20), 100).unwrap();
    assert_eq!((r.range.clone(), r.anchor_t), (1..5, Some(20)));
    assert!(r.limit.is_none() && r.capped.is_none() && !r.anchor_missing);
    assert_eq!(r.bounds.from, Some(20));
    assert!(r.accepts_live);
    // Closed window with an anchor.
    let r = resolve_window(&b, &win(None, Some(10), Some(40)), Some(20), 100).unwrap();
    assert_eq!(r.range, 1..4);
    assert!(!r.accepts_live);
    // More than 2 x max_bind_bars since the anchor: not trimmed, limit set.
    let r = resolve_window(&b, &Window::last(2), Some(10), 2).unwrap();
    assert_eq!((r.range.clone(), r.anchor_t), (0..5, Some(10)));
    assert!(r.limit.unwrap().starts_with("limit"));
    // Up to 2 x max_bind_bars since the anchor: ready (same bound as live growth).
    let r = resolve_window(&b, &Window::last(2), Some(10), 3).unwrap();
    assert!(r.limit.is_none());
    assert!(r.capped.is_none());
    // The source lost the anchor bar: bar 0 is the next one, flagged.
    let r = resolve_window(&b, &Window::last(2), Some(15), 100).unwrap();
    assert_eq!((r.range.clone(), r.anchor_t), (1..5, Some(20)));
    assert!(r.anchor_missing);
    assert_eq!(r.bounds.from, Some(15));
}

#[test]
fn bind_with_anchor_keeps_bar_0_or_goes_stale_at_the_limit() {
    let mut p = MemoryProvider::new();
    p.add_feed(key(), synthetic_bars(100, 3)).unwrap();
    p.entitle("host", ["*".to_owned()]).unwrap();
    let all = synthetic_bars(100, 3);

    let b = bind(
        &p,
        "host",
        &anchored("THYAO", Window::last(10), all[60].t),
        50,
    )
    .unwrap();
    assert_eq!(b.state, BindingState::Ready);
    assert_eq!(b.anchor_t, Some(all[60].t));
    assert!(b.reason.is_none());
    assert_eq!(b.store.as_ref().unwrap().bars(), &all[60..]);

    let b = bind(
        &p,
        "host",
        &anchored("THYAO", Window::last(10), all[20].t),
        30,
    )
    .unwrap();
    assert_eq!(b.state, BindingState::Stale);
    assert_eq!(b.current_state(), BindingState::Stale);
    assert!(b.reason.unwrap().starts_with("limit"));
    assert_eq!(b.anchor_t, Some(all[20].t));
    assert!(b.store.is_none());

    let b = bind(
        &p,
        "host",
        &anchored("THYAO", Window::last(10), all[60].t + 1),
        50,
    )
    .unwrap();
    // The source lost the anchor bar: bar numbers would differ → stale (§18.3).
    assert_eq!(b.state, BindingState::Stale);
    assert!(b.store.is_none());
    assert!(b.reason.unwrap().contains("no bar at the anchor"));
}

#[test]
fn live_growth_cap_makes_the_binding_stale() {
    let mut p = MemoryProvider::new();
    p.add_feed(key(), synthetic_bars(3, 3)).unwrap();
    p.entitle("host", ["THYAO".to_owned()]).unwrap();
    // max_bind_bars 3 → cap 6 bars held.
    let mut b = bind(&p, "host", &binding("THYAO", Window::last(3)), 3).unwrap();
    assert_eq!(b.current_state(), BindingState::Ready);
    let more = synthetic_bars(10, 3);
    let store = b.store.as_mut().unwrap();
    for (i, bar) in more[3..6].iter().enumerate() {
        let e = ev(i as u64 + 1, DataChange::Bar { bar: bar.clone() });
        assert_eq!(store.apply(&e).unwrap(), Applied::NewBar { index: 3 + i });
    }
    let e = ev(
        4,
        DataChange::Bar {
            bar: more[6].clone(),
        },
    );
    assert_eq!(
        store.apply(&e).unwrap(),
        Applied::Stale(StaleReason::Limit { max_bars: 6 })
    );
    assert_eq!(store.len(), 6);
    assert_eq!(store.seq(), 3, "the refused bar does not advance seq");
    // Everything after is refused the same way.
    let t = more[5].t;
    assert_eq!(
        store.apply(&patch(4, t, c(more[5].l))).unwrap(),
        Applied::Stale(StaleReason::Limit { max_bars: 6 })
    );
    assert_eq!(b.current_state(), BindingState::Stale);
    assert!(b.current_reason().unwrap().starts_with("limit"));
    // A bare store without the cap grows freely; with it, the same rule.
    let mut s = FeedStore::new(key()).with_max_bars(1);
    s.apply(&new_bar(1, 10, 5.0)).unwrap();
    assert!(matches!(
        s.apply(&new_bar(2, 20, 5.0)).unwrap(),
        Applied::Stale(StaleReason::Limit { max_bars: 1 })
    ));
    assert_eq!(s.state(), BindingState::Stale);
    s.reset(vec![], 0).unwrap();
    assert_eq!(s.state(), BindingState::Ready);
}

#[test]
fn gap_turns_a_bound_store_stale() {
    let mut p = MemoryProvider::new();
    p.add_feed(key(), synthetic_bars(5, 3)).unwrap();
    p.entitle("host", ["*".to_owned()]).unwrap();
    let mut b = bind(&p, "host", &binding("THYAO", Window::last(5)), 100).unwrap();
    let t = synthetic_bars(6, 3)[5].t;
    let store = b.store.as_mut().unwrap();
    assert!(matches!(
        store.apply(&new_bar(2, t, 5.0)).unwrap(),
        Applied::Gap {
            expected: 1,
            got: 2
        }
    ));
    assert_eq!(b.current_state(), BindingState::Stale);
    assert!(b.current_reason().unwrap().starts_with("gap"));
}

#[test]
fn entitlement_forms() {
    let named = |feed: Option<&str>, sym: &str| FeedKey {
        symbol: sym.into(),
        interval: "1d".into(),
        feed: feed.map(str::to_owned),
    };
    let cases = [
        ("*", named(None, "A"), true),
        ("*", named(Some("delayed"), "A"), true),
        ("A", named(None, "A"), true),
        ("A", named(Some("delayed"), "A"), true),
        ("A", named(None, "B"), false),
        ("delayed/A", named(Some("delayed"), "A"), true),
        ("delayed/A", named(None, "A"), false),
        ("delayed/A", named(Some("matriks"), "A"), false),
        ("delayed/A", named(Some("delayed"), "B"), false),
        ("delayed/*", named(Some("delayed"), "B"), true),
        ("delayed/*", named(None, "B"), false),
        ("delayed/*", named(Some("matriks"), "B"), false),
        ("bad entry", named(None, "bad entry"), false),
    ];
    for (entry, key, want) in cases {
        assert_eq!(entitlement_matches(entry, &key), want, "{entry} {key:?}");
    }
    for ok in ["*", "A", "delayed/A", "delayed/*"] {
        assert!(validate_entitlement(ok).is_ok(), "{ok}");
    }
    for bad in [
        "",
        "*/A",
        "/A",
        "delayed/",
        "a/b/c",
        "delayed/**",
        "A B",
        "-A",
    ] {
        assert!(validate_entitlement(bad).is_err(), "{bad}");
    }
    let mut p = MemoryProvider::new();
    assert!(p
        .entitle("x", ["ok".to_owned(), "a/b/c".to_owned()])
        .is_err());
    assert!(
        !p.entitled("x", &named(None, "ok")),
        "nothing granted on error"
    );
    p.entitle("x", ["delayed/*".to_owned()]).unwrap();
    assert!(p.entitled("x", &named(Some("delayed"), "Z")));
    assert!(!p.entitled("x", &named(None, "Z")));
    let bad = init(r#"{"entitlements": {"x": ["a/b/c"]}}"#);
    assert!(MemoryProvider::from_init(&bad, &repo("")).is_err());
}

#[test]
fn negative_zero_digests_like_zero() {
    let mut a = bar(10, 5.0);
    a.v = -0.0;
    let mut b = a.clone();
    b.v = 0.0;
    let mut sa = FeedStore::with_bars(key(), vec![a], 0).unwrap();
    let mut sb = FeedStore::with_bars(key(), vec![b], 0).unwrap();
    assert_eq!(sa.position().digest, sb.position().digest);
}
