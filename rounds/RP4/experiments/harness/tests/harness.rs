//! Harness self-tests: determinism, no look-ahead across the split, the product path,
//! preparation (K14 alignment), bootstrap, price step.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use qp_capsule::engine::EngineConfig;
use qp_capsule::outcome::Barrier;
use qp_capsule::pipeline::simulate_engine;
use qp_capsule::InstrumentInput;
use qp_schema::capsule::{Direction, Stage};
use qp_schema::Bar;

use rp4_harness::boot::{paired, ratio_w, Boot};
use rp4_harness::common::{istanbul_midnight_ms, to_json, xist};
use rp4_harness::data::{self, encode, load, load_for, ranges, read_series, Period, Split, Window};
use rp4_harness::registry::{self, r3_levels, Levels, RuleEntry};
use rp4_harness::sim::{
    day_positions, facts_for, reaches_target, run_engine, run_symbol, Ledger, INV, POS_CLOSE,
    POS_OPEN, REAL,
};
use rp4_harness::{m25, m26, report, synth, tick};

/// A prepared synthetic data set (3 symbols; intraday 2026-07-01 → 2026-11-20, daily
/// 2023-06-01 → 2026-11-20), built once per test binary.
fn prepared() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("rp4-harness-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        synth::write_rest_dir(
            &dir.join("rest"),
            3,
            "2026-07-01",
            "2023-06-01",
            "2026-11-20",
            7,
        )
        .expect("synth");
        data::prepare(&dir.join("rest"), &dir.join("prep"), None).expect("prepare");
        dir
    })
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("mkdir");
    for e in std::fs::read_dir(from).expect("read_dir") {
        let e = e.expect("entry");
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).expect("copy");
        }
    }
}

#[test]
fn tick_table() {
    assert!((tick::tick(19.99) - 0.01).abs() < 1e-12);
    assert!((tick::tick(20.0) - 0.01).abs() < 1e-12);
    assert!((tick::tick(20.01) - 0.02).abs() < 1e-12);
    assert!((tick::tick(75.0) - 0.05).abs() < 1e-12);
    assert!((tick::tick(250.0) - 0.10).abs() < 1e-12);
    assert!((tick::tick(300.0) - 0.25).abs() < 1e-12);
    assert!((tick::tick(999.0) - 0.50).abs() < 1e-12);
    assert!((tick::tick(2500.0) - 1.00).abs() < 1e-12);
    assert!((tick::tick(2500.5) - 2.50).abs() < 1e-12);
}

#[test]
fn split_constants() {
    let cal = xist();
    let r = ranges(Period::M5);
    assert_eq!(r.cut, istanbul_midnight_ms("2026-10-19"));
    // Local midnight: the instant before is the previous local day.
    let d = qp_calendar::parse_date("2026-10-19").expect("date");
    assert_eq!(cal.local_day(r.cut), d);
    assert_eq!(cal.local_day(r.cut - 1), d - 1);
    assert_eq!(ranges(Period::D1).cut, istanbul_midnight_ms("2025-01-02"));
    // Istanbul is UTC+3: 2026-10-18 21:00 UTC.
    assert_eq!(r.cut, 1_792_357_200_000);
}

#[test]
fn prepare_aligns_on_xist() {
    let dir = prepared();
    let cal = xist();
    let full = dir.join("prep/full");
    for p in Period::ALL {
        let v = load(&full, p, None).expect("load");
        assert_eq!(v.len(), 3, "{p:?}");
        for s in &v {
            for b in &s.bars {
                let span = cal.bar_at(p.interval(), b.t).expect("in session");
                assert_eq!(span.open, b.t, "{p:?} aligned");
            }
        }
    }
    // The 18:00 closing bar is merged into the 17:55 bar: 96 bars on a full day, and
    // the merged bar's close is the closing auction's.
    let rest = synth::five_minute("2026-07-01", "2026-11-20", 7000, 15.0);
    let five = load(&full, Period::M5, None).expect("load");
    let s = &five[0].bars;
    let day = cal.local_day(s[0].t);
    assert_eq!(s.iter().filter(|b| cal.local_day(b.t) == day).count(), 96);
    let closing = rest
        .iter()
        .find(|b| cal.local_day(b.t) == day && b.t == cal.day_span(day).expect("day").close)
        .expect("closing bar");
    let last = s.iter().rfind(|b| cal.local_day(b.t) == day).expect("last");
    assert_eq!(last.c, closing.c);
    // 15 minutes = 3 × 5 minutes (o first, h max, l min, c last, v sum).
    let fifteen = load(&full, Period::M15, None).expect("load");
    let f = &fifteen[0].bars[0];
    let parts: Vec<&Bar> = s.iter().take(3).collect();
    assert_eq!(f.t, parts[0].t);
    assert_eq!(f.o, parts[0].o);
    assert_eq!(f.c, parts[2].c);
    assert_eq!(f.h, parts.iter().map(|b| b.h).fold(f64::MIN, f64::max));
    assert_eq!(f.v, parts[0].v + parts[1].v + parts[2].v);
    // Daily bars sit at the session open (10:00 Istanbul).
    let daily = load(&full, Period::D1, None).expect("load");
    for b in &daily[0].bars {
        assert_eq!(b.t, cal.day_span(cal.local_day(b.t)).expect("day").open);
    }
}

#[test]
fn calibration_copy_has_no_test_bars() {
    let dir = prepared();
    for p in Period::ALL {
        let cut = ranges(p).cut;
        let v = load(&dir.join("prep/calib"), p, None).expect("load");
        assert!(!v.is_empty());
        assert!(v.iter().all(|s| s.bars.iter().all(|b| b.t < cut)), "{p:?}");
        let full = load(&dir.join("prep/full"), p, None).expect("load");
        assert!(
            full.iter().any(|s| s.bars.iter().any(|b| b.t >= cut)),
            "{p:?}"
        );
        // The calibration loader on the full copy sees exactly the calibration copy.
        let a = load_for(&dir.join("prep/full"), p, Split::Calib).expect("load");
        assert_eq!(a, v, "{p:?}");
    }
    assert!(load_for(&dir.join("prep/calib"), Period::M5, Split::Test).is_err());
}

#[test]
fn reader_stops_at_cut() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("rp4-harness-cut-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let bar = |t: i64| Bar {
        t,
        o: 1.0,
        h: 1.0,
        l: 1.0,
        c: 1.0,
        v: 1.0,
    };
    // A broken (non-ascending) record after the cut: reading stops before it.
    let bytes = encode(&[bar(1), bar(2), bar(3), bar(0)]);
    std::fs::write(dir.join("X.bars"), &bytes).expect("write");
    let v = read_series(&dir.join("X.bars"), Some(3)).expect("read");
    assert_eq!(v.len(), 2);
    assert!(
        read_series(&dir.join("X.bars"), None).is_err(),
        "non-ascending without a cut"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn m25_json(data: &Path, period: Period, split: Split) -> String {
    to_json(&m25::run(data, "r3", period, split).expect("m25"))
}

#[test]
fn m25_deterministic_and_blind_to_test_bars() {
    let dir = prepared();
    let full = dir.join("prep/full");
    let calib = dir.join("prep/calib");
    let a = m25_json(&full, Period::M5, Split::Calib);
    let b = m25_json(&full, Period::M5, Split::Calib);
    assert_eq!(a, b, "two runs byte-equal");
    assert_eq!(
        a,
        m25_json(&calib, Period::M5, Split::Calib),
        "full = calib copy"
    );
    // Change every test-range bar (prices and volumes) of every symbol.
    let changed = dir.join("changed");
    let _ = std::fs::remove_dir_all(&changed);
    copy_dir(&full, &changed);
    let cut = ranges(Period::M5).cut;
    for s in load(&full, Period::M5, None).expect("load") {
        let bars: Vec<Bar> = s
            .bars
            .iter()
            .enumerate()
            .map(|(i, b)| {
                if b.t < cut {
                    b.clone()
                } else {
                    #[allow(clippy::cast_precision_loss)]
                    let k = 1.0 + 0.3 * ((i % 7) as f64 - 3.0) / 3.0;
                    Bar {
                        t: b.t,
                        o: b.o * k,
                        h: b.h * k * 1.01,
                        l: b.l * k * 0.99,
                        c: b.c * k,
                        v: b.v * 3.0,
                    }
                }
            })
            .collect();
        std::fs::write(
            changed.join("5m").join(format!("{}.bars", s.symbol)),
            encode(&bars),
        )
        .expect("write");
    }
    assert_eq!(
        a,
        m25_json(&changed, Period::M5, Split::Calib),
        "test bars are not read"
    );
    assert_ne!(
        m25_json(&full, Period::M5, Split::Test),
        m25_json(&changed, Period::M5, Split::Test),
        "the test split does read them"
    );
}

#[test]
fn m25_reference_self_consistency() {
    let dir = prepared();
    let r = m25::run(&dir.join("prep/calib"), "r3", Period::M5, Split::Calib).expect("m25");
    let row = &r.rows[0];
    assert!(row.alt.n > 100);
    let m = |k: &str| row.metrics[k];
    // r3's own target is the reference target.
    assert_eq!(m("ecr_own").alt, m("ecr_ref").alt);
    // Paired difference of a rule with itself is exactly zero.
    for (k, t) in &row.metrics {
        if let Some(pt) = t.diff.pt {
            assert_eq!(pt, 0.0, "{k}");
            assert_eq!(t.diff.lo, Some(0.0), "{k}");
        }
    }
    // The unperturbed SENS re-run reproduces the engine for every story.
    assert_eq!(row.alt.sens_replay_mismatch, 0);
    assert_eq!(r.rows[1].alt.sens_replay_mismatch, 0);
    let s = m("sens").alt.pt.expect("sens");
    assert!(s > 0.0 && s < 1.0);
    let p = m("p_real").alt.pt.expect("p")
        + m("p_inv").alt.pt.expect("p")
        + m("p_exp").alt.pt.expect("p");
    assert!((p - 1.0).abs() < 1e-12);
}

#[test]
fn m25_test_split_counts_only_test_births() {
    let dir = prepared();
    let r = m25::run(&dir.join("prep/full"), "r3", Period::M5, Split::Test).expect("m25");
    assert_eq!(r.window[0], "2026-10-19");
    assert_eq!(r.gap_ref_day, "2026-10-19");
    // 24 test days (SPLIT.md), 3 symbols.
    assert_eq!(r.days, 24);
    assert!(r.rows[0].alt.n > 0);
    let c = m25::run(&dir.join("prep/full"), "r3", Period::M5, Split::Calib).expect("m25");
    assert!(c.days > r.days);
}

/// A test-only alternative (not a candidate): wider barriers.
fn wide() -> RuleEntry {
    RuleEntry {
        name: "test-wide",
        make: || Box::new(Barrier::new("test-wide", 1.0, 1.5)),
        levels: |s, b| {
            r3_levels(s, b).map(|l| Levels {
                target: l.target * 1.5,
                stop: l.stop.map(|x| x * 1.5),
                ..l
            })
        },
        constants: 8,
    }
}

#[test]
fn alternative_is_paired_with_reference() {
    let dir = prepared();
    let series = load_for(&dir.join("prep/calib"), Period::M15, Split::Calib).expect("load");
    let r = m25::run_series(
        &series,
        &wide(),
        &registry::reference_rule(),
        Period::M15,
        Split::Calib,
    );
    let row = &r.rows[0];
    assert_eq!(r.rule, "test-wide");
    assert!(row.alt.n > 0 && row.reference.n > 0);
    let exp = row.metrics["p_exp"];
    // Wider barriers expire more often.
    assert!(exp.diff.pt.expect("diff") > 0.0);
    // Reference numbers equal the reference run alone.
    let r3 = m25::run_series(
        &series,
        &registry::reference_rule(),
        &registry::reference_rule(),
        Period::M15,
        Split::Calib,
    );
    assert_eq!(
        row.metrics["sens"].reference,
        r3.rows[0].metrics["sens"].alt
    );
    assert_eq!(row.reference, r3.rows[0].alt);
    // Report: the alternative gets a criteria row.
    let (md, csv) = report::m25_tables(vec![r, r3]);
    assert!(md.contains("`test-wide`"));
    assert!(csv.lines().count() > 10);
}

#[test]
fn per_symbol_runs_equal_one_engine() {
    let dir = prepared();
    let series = load_for(&dir.join("prep/calib"), Period::H1, Split::Calib).expect("load");
    let inputs: Vec<InstrumentInput> = series
        .iter()
        .map(|s| InstrumentInput {
            symbol: s.symbol.clone(),
            series: "1h".to_owned(),
            bars: s.bars.clone(),
            flow: None,
        })
        .collect();
    let (engine, _) = simulate_engine(&inputs, &qp_facts::default_detectors, EngineConfig::r3());
    let cal = xist();
    let entry = registry::reference_rule();
    let window = Window::of(Period::H1, Split::Calib);
    let mut all = Vec::new();
    for s in &series {
        let facts = facts_for(&s.symbol, "1h", &s.bars, qp_facts::default_detectors, false);
        let pos = day_positions(&cal, &s.bars);
        let recs = run_symbol(
            &s.symbol, "1h", &s.bars, &facts, &entry, window, &pos, false,
        );
        all.push((s.symbol.clone(), recs));
    }
    let ledger = Ledger::build(all.iter().flat_map(|(_, r)| r.iter()));
    let mut checked = 0;
    for st in engine.closed_stories() {
        let (_, recs) = all
            .iter()
            .find(|(s, _)| *s == st.story.instrument)
            .expect("symbol");
        let rec = recs
            .iter()
            .find(|r| r.born_bar == st.story.born_bar && r.key == st.story.ledger_key && r.closed())
            .expect("same story");
        let stage = match st.story.stage {
            Stage::Realized => REAL,
            Stage::Invalidated => INV,
            _ => 3,
        };
        assert_eq!(rec.stage, stage);
        assert_eq!(rec.close_bar, st.story.last_bar);
        let (n, r) = ledger.stat(&rec.key, rec.born_t);
        assert_eq!(
            (n, r),
            (
                st.story.ledger_at_birth.n,
                st.story.ledger_at_birth.realized
            )
        );
        checked += 1;
    }
    let ours: usize = all
        .iter()
        .map(|(_, r)| r.iter().filter(|x| x.closed()).count())
        .sum();
    assert_eq!(checked, ours);
    assert!(checked > 50);
}

#[test]
fn r3_levels_are_the_barrier() {
    let dir = prepared();
    let s = &load_for(&dir.join("prep/calib"), Period::H1, Split::Calib).expect("load")[0];
    let facts = facts_for(&s.symbol, "1h", &s.bars, qp_facts::default_detectors, false);
    let e = run_engine(
        &s.symbol,
        "1h",
        &s.bars,
        &facts,
        Box::new(Barrier::default()),
    );
    let mut n = 0;
    for st in e.closed_stories() {
        let born = usize::try_from(st.story.born_bar).expect("bar");
        let close = usize::try_from(st.story.last_bar).expect("bar");
        let lv = r3_levels(st, &s.bars[..=born]).expect("levels");
        let dir = st.story.direction;
        match st.story.stage {
            Stage::Realized => {
                assert!(reaches_target(&lv, dir, s.bars[close].c));
                assert!((born + 1..close).all(|j| !reaches_target(&lv, dir, s.bars[j].c)));
                n += 1;
            }
            Stage::Invalidated if dir != Direction::Neutral => {
                let mirror = Levels {
                    target: lv.stop.expect("stop"),
                    ..lv
                };
                let opp = if dir == Direction::Up {
                    Direction::Down
                } else {
                    Direction::Up
                };
                assert!(reaches_target(&mirror, opp, s.bars[close].c));
                n += 1;
            }
            _ => {}
        }
    }
    assert!(n > 20);
}

#[test]
fn day_positions_mark_open_and_close() {
    let dir = prepared();
    let cal = xist();
    let s = &load(&dir.join("prep/full"), Period::H1, None).expect("load")[0];
    let pos = day_positions(&cal, &s.bars);
    // 8 hourly bars on a full day: bars 0, 1 open; bar 7 close.
    assert_eq!(pos[0] & POS_OPEN, POS_OPEN);
    assert_eq!(pos[1] & POS_OPEN, POS_OPEN);
    assert_eq!(pos[2], 0);
    assert_eq!(pos[7] & POS_CLOSE, POS_CLOSE);
    assert_eq!(pos[8] & POS_OPEN, POS_OPEN);
}

#[test]
fn m26_deterministic_and_blind() {
    let dir = prepared();
    let a = to_json(
        &m26::run(
            &dir.join("prep/full"),
            "r3",
            "r3",
            Period::M15,
            Split::Calib,
        )
        .expect("m26"),
    );
    let b = to_json(
        &m26::run(
            &dir.join("prep/calib"),
            "r3",
            "r3",
            Period::M15,
            Split::Calib,
        )
        .expect("m26"),
    );
    assert_eq!(a, b);
    let r = m26::run(
        &dir.join("prep/calib"),
        "r3",
        "r3",
        Period::M15,
        Split::Calib,
    )
    .expect("m26");
    let all = r.rows.last().expect("rows");
    assert_eq!(all.detector, m26::ALL);
    assert!(all.alt.births > 0 && all.alt.births_open > 0 && all.alt.births_close > 0);
    assert_eq!(all.load_ratio, Some(1.0));
    let d = m26::run(
        &dir.join("prep/calib"),
        "r3",
        "r3",
        Period::D1,
        Split::Calib,
    )
    .expect("m26");
    assert!(d.rows.last().expect("rows").metrics["d_open"]
        .alt
        .pt
        .is_none());
    let (md, _) = report::m26_tables(vec![r, d]);
    assert!(md.contains("(hepsi)"));
}

#[test]
fn bootstrap_is_deterministic_and_paired() {
    let a = Boot::new(17);
    let b = Boot::new(17);
    assert_eq!(a, b);
    assert_eq!(a.weights.len(), 1000);
    assert!(a.weights.iter().all(|w| w.iter().sum::<u32>() == 17));
    let num = vec![
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
    ];
    let den = vec![10.0; 17];
    let (x, y, d) = paired(&a, |w| ratio_w(w, &num, &den), |w| ratio_w(w, &num, &den));
    assert_eq!(x, y);
    assert_eq!(d.pt, Some(0.0));
    assert_eq!((d.lo, d.hi), (Some(0.0), Some(0.0)));
    assert!(x.lo < x.pt && x.pt < x.hi);
}

#[test]
fn prepare_is_deterministic() {
    let dir = prepared();
    let other = dir.join("prep2");
    data::prepare(&dir.join("rest"), &other, None).expect("prepare");
    for copy in ["full", "calib"] {
        assert_eq!(
            std::fs::read(dir.join("prep").join(copy).join("manifest.json")).expect("read"),
            std::fs::read(other.join(copy).join("manifest.json")).expect("read")
        );
        for p in Period::ALL {
            for s in ["SYN01", "SYN02", "SYN03"] {
                let f = |d: &Path| {
                    std::fs::read(d.join(copy).join(p.name()).join(format!("{s}.bars")))
                        .expect("read")
                };
                assert_eq!(f(&dir.join("prep")), f(&other));
            }
        }
    }
}
