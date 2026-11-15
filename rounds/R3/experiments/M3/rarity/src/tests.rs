use super::*;
use qp_schema::capsule::{
    Direction, EvidenceFamily, LedgerStat, MeaningKind, Stage, Story, TimeKind, Trust,
};

fn story(kind: MeaningKind, dir: Direction, strength: f64, fams: &[EvidenceFamily]) -> OpenStory {
    OpenStory {
        id: "cap.X.t.10".to_owned(),
        story: Story {
            instrument: "X".to_owned(),
            theme: "t".to_owned(),
            kind,
            direction: dir,
            time: TimeKind::Event,
            stage: Stage::Born,
            families: fams.to_vec(),
            strength,
            importance: 0.0,
            tier: 3,
            series: "X.1day".to_owned(),
            born_bar: 10,
            last_bar: 10,
            horizon_bars: 10,
            ledger_key: "breakout/breakout.range@1".to_owned(),
            ledger_at_birth: LedgerStat::default(),
            trust: Trust::Replica,
            history: vec![],
        },
        facts: vec![],
    }
}

fn ctx(on_inst: u32, bars: u64, kind_all: u32, all: u32) -> RankCtx {
    RankCtx {
        ledger: LedgerStat {
            n: 40,
            realized: 20,
        },
        ledger_all: LedgerStat {
            n: 200,
            realized: 95,
        },
        same_kind_on_instrument: on_inst,
        bars_seen: bars,
        same_kind_all: kind_all,
        stories_all: all,
        instruments: 30,
    }
}

const P: EvidenceFamily = EvidenceFamily::Price;
const V: EvidenceFamily = EvidenceFamily::Volume;

#[test]
fn deterministic() {
    let r = Rarity::default();
    let s = story(MeaningKind::Breakout, Direction::Up, 0.4, &[P]);
    let c = ctx(5, 300, 900, 3000);
    let a = r.rank(&s, &c);
    let b = ranker().rank(&s, &c);
    assert_eq!(a.0.to_bits(), b.0.to_bits());
    assert_eq!(a.1, b.1);
}

#[test]
fn tier_in_range_over_grid() {
    let r = Rarity::default();
    for kind in MeaningKind::ALL {
        for dir in [Direction::Up, Direction::Down, Direction::Neutral] {
            for st in [0.0, 0.3, 1.0, f64::NAN, -1.0, 5.0] {
                for fams in [&[P][..], &[P, V][..], &[][..]] {
                    for (oi, b, k, a, n) in [
                        (0, 1, 0, 0, 0),
                        (0, 0, 0, 0, 0),
                        (50, 60, 5000, 9000, 1),
                        (3, 500, 400, 9000, 30),
                        (u32::MAX, u64::MAX, u32::MAX, u32::MAX, u32::MAX),
                    ] {
                        let mut c = ctx(oi, b, k, a);
                        c.instruments = n;
                        let (imp, tier) = r.rank(&story(kind, dir, st, fams), &c);
                        assert!((1..=3).contains(&tier), "tier {tier}");
                        assert!(imp.is_finite(), "importance {imp}");
                    }
                }
            }
        }
    }
}

#[test]
fn rarer_on_instrument_is_not_less_important() {
    let r = Rarity::default();
    let s = story(MeaningKind::Reversal, Direction::Down, 0.2, &[P]);
    let common = r.rank(&s, &ctx(40, 400, 900, 3000));
    let rare = r.rank(&s, &ctx(1, 400, 900, 3000));
    assert!(rare.0 > common.0);
    assert!(rare.1 <= common.1);
}

#[test]
fn stronger_and_confirmed_are_rarer() {
    let r = Rarity::default();
    let c = ctx(5, 300, 900, 3000);
    let weak = r.rank(&story(MeaningKind::Breakout, Direction::Up, 0.0, &[P]), &c);
    let strong = r.rank(&story(MeaningKind::Breakout, Direction::Up, 1.0, &[P]), &c);
    let joint = r.rank(
        &story(MeaningKind::Breakout, Direction::Up, 0.0, &[P, V]),
        &c,
    );
    assert!(strong.0 > weak.0);
    assert!(joint.0 > weak.0);
}

#[test]
fn tier_thresholds_are_recurrence_in_bars() {
    let r = Rarity::default();
    assert_eq!(r.tier_of(150f64.ln()), 1);
    assert_eq!(r.tier_of(149f64.ln()), 2);
    assert_eq!(r.tier_of(50f64.ln()), 2);
    assert_eq!(r.tier_of(49f64.ln()), 3);
    assert_eq!(r.tier_of(0.0), 3);
}

#[test]
fn name_is_rarity() {
    assert_eq!(ranker().name(), "rarity");
}
