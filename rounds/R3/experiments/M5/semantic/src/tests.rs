use std::collections::BTreeMap;

use qp_capsule::{OpenStory, OutcomeRule, Verdict};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, FactMeaning, LedgerStat, License, LicenseClass, MeaningKind,
    Stage, Story, TimeKind, Trust,
};
use qp_schema::{Author, Bar, Fact, Provenance};

use super::{outcome, perturbations, Semantic};

fn bar(i: usize, o: f64, h: f64, l: f64, c: f64) -> Bar {
    Bar {
        t: 1_000 + i64::try_from(i).unwrap() * 60_000,
        o,
        h,
        l,
        c,
        v: 1.0,
    }
}

/// Flat warm-up bars at 100 with a true range of exactly 2 (ATR = 2).
fn flat(n: usize) -> Vec<Bar> {
    (0..n).map(|i| bar(i, 100.0, 101.0, 99.0, 100.0)).collect()
}

/// Push a bar with close `c` and a range of ±1 around it.
fn push(b: &mut Vec<Bar>, c: f64) {
    let i = b.len();
    b.push(bar(i, c, c + 1.0, c - 1.0, c));
}

fn story(
    kind: MeaningKind,
    dir: Direction,
    time: TimeKind,
    born: u64,
    ev: &[(&str, f64)],
) -> OpenStory {
    let fact = Fact {
        id: format!("fx.T.det.code.{born}"),
        kind: "det.code".to_owned(),
        series: "1d".to_owned(),
        bar: born,
        t: 0,
        text: String::new(),
        evidence: ev
            .iter()
            .map(|(k, v)| ((*k).to_owned(), *v))
            .collect::<BTreeMap<_, _>>(),
        evidence_ref: None,
        meaning: Some(FactMeaning {
            kind,
            direction: dir,
            family: EvidenceFamily::Price,
            time,
            edge: Edge::Point,
            detector: "det".to_owned(),
            detector_version: 1,
            instrument: "T".to_owned(),
            strength: 0.5,
            license: License {
                class: LicenseClass::Public,
                entitlement: None,
            },
            geometry: Vec::new(),
            condition: None,
            server: false,
        }),
        provenance: Provenance {
            author: Author::Engine {
                name: "test".to_owned(),
                version: "0".to_owned(),
            },
            at: 0,
            based_on: Vec::new(),
            approved_by: None,
        },
    };
    OpenStory {
        id: format!("cap.T.x.{born}"),
        story: Story {
            instrument: "T".to_owned(),
            theme: "x".to_owned(),
            kind,
            direction: dir,
            time,
            stage: Stage::Born,
            families: vec![EvidenceFamily::Price],
            strength: 0.5,
            importance: 0.0,
            tier: 3,
            series: "1d".to_owned(),
            born_bar: born,
            last_bar: born,
            horizon_bars: 10,
            ledger_key: "k".to_owned(),
            ledger_at_birth: LedgerStat::default(),
            trust: Trust::Replica,
            history: Vec::new(),
        },
        facts: vec![fact],
    }
}

/// First verdict over the bars after birth (judging with `bars[..=now]`).
fn run(
    rule: &dyn OutcomeRule,
    s: &OpenStory,
    bars: &[Bar],
) -> Option<(u64, Verdict, &'static str)> {
    let born = s.story.born_bar;
    for now in born + 1..bars.len() as u64 {
        if let Some((v, code)) = rule.judge(s, &bars[..=usize::try_from(now).unwrap()], now) {
            return Some((now, v, code));
        }
    }
    None
}

fn flip(d: Direction) -> Direction {
    match d {
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
        Direction::Neutral => Direction::Neutral,
    }
}

#[test]
fn names_and_perturbations() {
    assert_eq!(outcome().name(), "semantic");
    let p = perturbations();
    assert_eq!(p.len(), 4);
    let r = Semantic::default();
    assert_eq!(r.horizon(MeaningKind::Squeeze, TimeKind::State), 30);
    assert_eq!(p[0].1.horizon(MeaningKind::Squeeze, TimeKind::State), 24);
    assert_eq!(p[1].1.horizon(MeaningKind::Breakout, TimeKind::Event), 12);
    assert_eq!(p[2].1.horizon(MeaningKind::Breakout, TimeKind::Event), 10);
}

#[test]
fn breakout_extends_or_fails() {
    // Breakout up over level 101 at bar 20 (close 102), ATR = 2.
    let mut b = flat(20);
    push(&mut b, 102.0);
    let s = story(
        MeaningKind::Breakout,
        Direction::Up,
        TimeKind::Event,
        20,
        &[("level", 101.0)],
    );
    let mut hit = b.clone();
    for c in [102.5, 103.0, 104.5] {
        push(&mut hit, c);
    }
    let r = Semantic::default();
    assert_eq!(
        run(&r, &s, &hit),
        Some((23, Verdict::Realized, "sem.breakout.extended"))
    );
    // Close back below the level by more than 0.25 ATR (0.5): failed.
    let mut fail = b.clone();
    for c in [101.5, 100.6, 100.4] {
        push(&mut fail, c);
    }
    assert_eq!(
        run(&r, &s, &fail),
        Some((23, Verdict::Invalidated, "sem.breakout.failed"))
    );
}

#[test]
fn reversal_needs_the_neckline() {
    // Bearish divergence: p1 = bar 22 (high 111), p2 = bar 30 (high 113), trough between
    // at bar 26 (low 104). Detected at bar 32.
    let mut b = flat(20);
    for c in [
        104.0, 107.0, 110.0, 108.0, 106.0, 105.5, 105.0, 107.0, 109.0, 111.0, 112.0, 110.0, 109.0,
    ] {
        push(&mut b, c);
    }
    // bar 22 close 110 high 111; bar 26 low 104; bar 30 close 112 high 113.
    let s = story(
        MeaningKind::Reversal,
        Direction::Down,
        TimeKind::Event,
        32,
        &[
            ("p1_bar", 22.0),
            ("p2_bar", 30.0),
            ("p1_price", 111.0),
            ("p2_price", 113.0),
        ],
    );
    let r = Semantic::default();
    let mut hit = b.clone();
    for c in [107.0, 105.0, 103.5] {
        push(&mut hit, c);
    }
    assert_eq!(
        run(&r, &s, &hit),
        Some((35, Verdict::Realized, "sem.reversal.neckline"))
    );
    // A 2-ATR drop that does not cross the neckline (104) is not yet a reversal.
    let mut partial = b.clone();
    for c in [106.0, 105.0, 104.5] {
        push(&mut partial, c);
    }
    assert_eq!(run(&r, &s, &partial), None);
    // A close above the second peak + 0.25 ATR invalidates.
    let mut fail = b.clone();
    for c in [111.0, 114.0] {
        push(&mut fail, c);
    }
    assert_eq!(
        run(&r, &s, &fail),
        Some((34, Verdict::Invalidated, "sem.reversal.new_extreme"))
    );
}

#[test]
fn squeeze_needs_band_exit_and_expansion() {
    let mut b = flat(20);
    push(&mut b, 100.0);
    let s = story(
        MeaningKind::Squeeze,
        Direction::Neutral,
        TimeKind::State,
        20,
        &[("upper", 101.0), ("lower", 99.0), ("middle", 100.0)],
    );
    let r = Semantic::default();
    // Slow drift out of the band with small ranges: no expansion.
    let mut drift = b.clone();
    for i in 0..8 {
        let c = 100.0 + 0.3 * f64::from(i + 1);
        let n = drift.len();
        drift.push(bar(n, c, c + 0.2, c - 0.2, c));
    }
    assert_eq!(run(&r, &s, &drift), None);
    // Wide-range break below the band.
    let mut exp = b.clone();
    for c in [98.0, 96.0, 94.0] {
        let n = exp.len();
        exp.push(bar(n, c + 1.0, c + 2.0, c - 2.0, c));
    }
    assert_eq!(
        run(&r, &s, &exp),
        Some((21, Verdict::Realized, "sem.squeeze.expanded"))
    );
}

#[test]
fn extreme_reverts_or_stretches() {
    // Overbought (up) after a run from 100 to 110.
    let mut b = flat(20);
    for i in 1..=10 {
        push(&mut b, 100.0 + f64::from(i));
    }
    let born = (b.len() - 1) as u64;
    let s = story(
        MeaningKind::Extreme,
        Direction::Up,
        TimeKind::State,
        born,
        &[("level", 70.0)],
    );
    let r = Semantic::default();
    let mut back = b.clone();
    for c in [108.0, 106.0, 104.0, 103.0] {
        push(&mut back, c);
    }
    let got = run(&r, &s, &back).unwrap();
    assert_eq!((got.1, got.2), (Verdict::Realized, "sem.extreme.reverted"));
    let mut more = b.clone();
    for c in [111.0, 112.0, 113.5] {
        push(&mut more, c);
    }
    let got = run(&r, &s, &more).unwrap();
    assert_eq!(
        (got.1, got.2),
        (Verdict::Invalidated, "sem.extreme.stretched")
    );
}

#[test]
fn continuation_and_flow() {
    let mut b = flat(60);
    for i in 1..=5 {
        push(&mut b, 100.0 + f64::from(i));
    }
    let born = (b.len() - 1) as u64;
    let r = Semantic::default();
    let s = story(
        MeaningKind::Continuation,
        Direction::Up,
        TimeKind::State,
        born,
        &[],
    );
    let mut up = b.clone();
    for c in [106.0, 107.0, 108.5] {
        push(&mut up, c);
    }
    let got = run(&r, &s, &up).unwrap();
    assert_eq!(
        (got.1, got.2),
        (Verdict::Realized, "sem.continuation.extended")
    );
    let mut down = b.clone();
    for c in [103.0, 100.0, 99.0] {
        push(&mut down, c);
    }
    let got = run(&r, &s, &down).unwrap();
    assert_eq!(
        (got.1, got.2),
        (Verdict::Invalidated, "sem.continuation.broke_ma50")
    );

    // Flow down at bar 20 (bar range 99..101); a close above 101 + 0.5 invalidates.
    let mut f = flat(21);
    let sf = story(MeaningKind::Flow, Direction::Down, TimeKind::Event, 20, &[]);
    push(&mut f, 101.0);
    push(&mut f, 101.6);
    assert_eq!(
        run(&r, &sf, &f),
        Some((22, Verdict::Invalidated, "sem.flow.reversed"))
    );
}

#[test]
fn flipped_claim_is_judged_as_the_opposite_claim() {
    // Same bars, same evidence, opposite direction → opposite verdict on a clean move.
    let mut b = flat(21);
    for c in [99.0, 98.0, 97.0] {
        push(&mut b, c);
    }
    let r = Semantic::default();
    let down = story(MeaningKind::Flow, Direction::Down, TimeKind::Event, 20, &[]);
    let up = story(
        MeaningKind::Flow,
        flip(Direction::Down),
        TimeKind::Event,
        20,
        &[],
    );
    assert_eq!(run(&r, &down, &b).unwrap().1, Verdict::Realized);
    assert_eq!(run(&r, &up, &b).unwrap().1, Verdict::Invalidated);
}

#[test]
fn no_judgement_without_atr_window() {
    let b = flat(30);
    // Born at bar 3: only 4 bars for the true-range mean (< 5) → never judged.
    let s = story(MeaningKind::Flow, Direction::Up, TimeKind::Event, 3, &[]);
    for now in 4..30u64 {
        assert_eq!(Semantic::default().judge(&s, &b, now), None);
    }
}

/// No look-ahead: (1) for every `now`, the verdict is the same whether the rule gets
/// `bars[..=now]` or the full series (it never reads past `now`); (2) two series that
/// agree up to `T` give identical verdicts for every `now ≤ T`, whatever comes after,
/// even when calls on the two series are interleaved (the rule keeps no state).
#[test]
fn no_look_ahead() {
    let kinds = [
        (MeaningKind::Breakout, Direction::Up, vec![("level", 100.5)]),
        (
            MeaningKind::Breakout,
            Direction::Down,
            vec![("level", 99.5)],
        ),
        (
            MeaningKind::Reversal,
            Direction::Down,
            vec![("p1_bar", 10.0), ("p2_bar", 18.0), ("p2_price", 101.0)],
        ),
        (
            MeaningKind::Squeeze,
            Direction::Neutral,
            vec![("upper", 101.0), ("lower", 99.0)],
        ),
        (MeaningKind::Extreme, Direction::Up, vec![]),
        (MeaningKind::Extreme, Direction::Down, vec![]),
        (MeaningKind::Continuation, Direction::Up, vec![]),
        (MeaningKind::Flow, Direction::Down, vec![]),
        (MeaningKind::Flow, Direction::Neutral, vec![]),
    ];
    // Deterministic pseudo-random walks (LCG), sharing the first 40 bars.
    let walk = |seed: u64, n: usize, prefix: &[Bar]| -> Vec<Bar> {
        let mut x = seed;
        let mut out = prefix.to_vec();
        let mut c = out.last().map_or(100.0, |b| b.c);
        while out.len() < n {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            #[allow(clippy::cast_precision_loss)]
            let u = ((x >> 11) as f64) / ((1u64 << 53) as f64) - 0.5;
            c += u * 3.0;
            let i = out.len();
            out.push(bar(i, c, c + 1.0 + u.abs(), c - 1.0 - u.abs(), c));
        }
        out
    };
    let base = walk(7, 40, &[]);
    let a = walk(11, 120, &base);
    let b = walk(99, 120, &base);
    assert!((a[60].c - b[60].c).abs() > 1e-9);
    let rules: Vec<Box<dyn OutcomeRule>> = std::iter::once(outcome())
        .chain(perturbations().into_iter().map(|(_, r)| r))
        .collect();
    for rule in &rules {
        for (kind, dir, ev) in &kinds {
            for born in [20u64, 25, 30] {
                let s = story(*kind, *dir, TimeKind::Event, born, ev);
                for now in born + 1..120 {
                    let n = usize::try_from(now).unwrap();
                    let exact = rule.judge(&s, &a[..=n], now);
                    let full = rule.judge(&s, &a, now);
                    assert_eq!(exact, full, "{kind:?} {dir:?} read past now={now}");
                    let other = rule.judge(&s, &b[..=n], now);
                    if n < 40 {
                        assert_eq!(exact, other, "{kind:?} {dir:?} future leaked at {now}");
                    }
                    // Interleave again: still the same (stateless).
                    assert_eq!(rule.judge(&s, &a[..=n], now), exact);
                }
            }
        }
    }
}
