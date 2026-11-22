//! (RD2/S1) The design check: P1–P13 of `../refs/reports/VISUAL.md` §5 and R5's hard
//! thresholds (H1–H3, `docs/DESIGN.md` §1.6) measured for a look, in both schemes, from
//! the look record and the compiled design fixture (`fixtures/design/desk.json`).
//!
//! `scripts/design-check [--look <name>] [--layout <file>]` runs this with
//! `QP_DESIGN_LOOK` / `QP_DESIGN_LAYOUT`. Without a look: every registered non-legacy
//! look is checked, and `oltan` and `r4` are printed for comparison. The test fails only
//! when a non-legacy look fails a rule. Thresholds: `thresholds.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::missing_panics_doc
)]

mod alphabet;
mod color;
mod grounds;
mod levels;
mod record;
mod scene;
mod thresholds;

use qp_grammar::look::{by_name, ChromeLook, Look, Profile, TypeRole, TypeScale, LOOKS, R4};

/// A rule's outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Res {
    Pass,
    Fail,
    Na,
}

/// One table row.
#[derive(Debug, Clone)]
pub struct Row {
    pub id: &'static str,
    pub scheme: &'static str,
    pub rule: String,
    pub value: String,
    pub threshold: String,
    pub res: Res,
}

impl Row {
    pub fn check(
        id: &'static str,
        scheme: &'static str,
        rule: &str,
        value: String,
        threshold: String,
        pass: bool,
    ) -> Self {
        let res = if pass { Res::Pass } else { Res::Fail };
        Row {
            id,
            scheme,
            rule: rule.to_owned(),
            value,
            threshold,
            res,
        }
    }

    pub fn na(id: &'static str, scheme: &'static str, rule: &str) -> Self {
        Row {
            id,
            scheme,
            rule: rule.to_owned(),
            value: "-".to_owned(),
            threshold: "-".to_owned(),
            res: Res::Na,
        }
    }
}

/// Every row for `look` (`layout`: the gallery rig's layout dump, for P9).
fn rows(look: &'static Look, fx: &scene::Fixture, layout: Option<&serde_json::Value>) -> Vec<Row> {
    let mut out = vec![
        record::p9(layout),
        record::p10(look),
        record::p11(look),
        record::p12(look),
    ];
    for t in [&look.dark, &look.light] {
        let s = t.name;
        let (dw, dh) = thresholds::DESK;
        let desk = fx.scene(look, s, dw, dh);
        let phone = fx.scene(look, s, thresholds::P5_WIDTH, thresholds::PHONE_H);
        let plot = fx.price_only(look, s, thresholds::P5_WIDTH, thresholds::P4_PLOT_H);
        let lv_desk = fx.scene_of(&fx.levels, look, s, dw, dh);
        let lv_phone = fx.scene_of(
            &fx.levels,
            look,
            s,
            thresholds::P5_WIDTH,
            thresholds::PHONE_H,
        );
        out.extend([
            record::p1(t),
            scene::p2(&desk, &lv_desk, s),
            record::p3(t),
            scene::p4(&desk, &plot, s),
            scene::p5(&phone, look, s),
            scene::p6(&[&desk, &lv_desk], &[&phone, &lv_phone], s),
            scene::p7(&desk, look, s),
            scene::p8(&desk, look, s),
            record::p13(t),
            grounds::h1(look, t),
            grounds::h2(look, t),
            record::h3(look, t),
        ]);
    }
    let key = |r: &Row| (r.id.starts_with('H'), r.id[1..].parse::<u32>().unwrap_or(0));
    out.sort_by_key(key);
    out
}

/// Whether `rows` fail the check for `look`: a non-legacy look failing any rule.
fn fails(look: &Look, rows: &[Row]) -> bool {
    look.profile != Profile::Legacy && rows.iter().any(|r| r.res == Res::Fail)
}

fn print_table(look: &Look, rows: &[Row]) {
    let fails = rows.iter().filter(|r| r.res == Res::Fail).count();
    let legacy = look.profile == Profile::Legacy;
    println!(
        "\n## design-check `{}` (profile {}{}): {}",
        look.name,
        look.profile.name(),
        if legacy {
            ", for comparison, never fails"
        } else {
            ""
        },
        if fails == 0 {
            "PASS".to_owned()
        } else {
            format!("{fails} FAIL")
        }
    );
    println!("| P# | scheme | rule | measured | threshold | result |");
    println!("|---|---|---|---|---|---|");
    for r in rows {
        let res = match r.res {
            Res::Pass => "PASS",
            Res::Fail => "FAIL",
            Res::Na => "n.a.",
        };
        println!(
            "| {} | {} | {} | {} | {} | {res} |",
            r.id, r.scheme, r.rule, r.value, r.threshold
        );
    }
}

/// The design check (`scripts/design-check`).
#[test]
fn design_check() {
    let layout = std::env::var("QP_DESIGN_LAYOUT")
        .ok()
        .filter(|p| !p.is_empty())
        .map(|p| {
            let t = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
            serde_json::from_str::<serde_json::Value>(&t).unwrap_or_else(|e| panic!("{p}: {e}"))
        });
    let looks: Vec<&Look> = match std::env::var("QP_DESIGN_LOOK") {
        Ok(n) if !n.is_empty() => vec![by_name(Some(&n)).unwrap_or_else(|e| panic!("{e:?}"))],
        _ => LOOKS
            .iter()
            .filter(|l| l.profile != Profile::Legacy)
            .chain(["oltan", "r4"].iter().filter_map(|n| by_name(Some(n)).ok()))
            .collect(),
    };
    let fx = scene::fixture();
    let mut failed = Vec::new();
    for l in looks {
        let r = rows(l, &fx, layout.as_ref());
        print_table(l, &r);
        if fails(l, &r) {
            failed.push(l.name);
        }
    }
    assert!(failed.is_empty(), "design-check fails for {failed:?}");
}

/// The profile logic on a test-only look: the same record passes P10 as dense and fails
/// it as touch; P12 follows the radii; legacy looks are skipped by default.
#[test]
fn profiles_select_their_thresholds() {
    // `r4` with its two sub-11 px sizes raised: a dense record.
    const TRIAL: Look = Look {
        name: "trial",
        profile: Profile::Dense,
        types: TypeScale {
            note: TypeRole {
                px: 11.0,
                ..R4.types.note
            },
            ..R4.types
        },
        chrome: ChromeLook {
            trust_px: 11.0,
            ..R4.chrome
        },
        ..R4
    };
    let dense = record::p10(&TRIAL);
    assert_eq!(dense.res, Res::Pass, "{dense:?}");
    let touch = record::p10(&Look {
        profile: Profile::Touch,
        ..TRIAL
    });
    assert_eq!(touch.res, Res::Fail, "{touch:?}");
    assert_eq!(record::p12(&TRIAL).res, Res::Pass);
    let mut round = TRIAL;
    round.surface.card_radius = 16.0;
    assert_eq!(record::p12(&round).res, Res::Fail);
    assert_eq!(record::p9(None).res, Res::Na);
}

/// A non-legacy look with deliberately bad values fails the check (and the same record as
/// legacy does not): the gate is live, not only the table.
#[test]
fn a_bad_non_legacy_look_fails() {
    const BAD: Look = Look {
        name: "bad",
        profile: Profile::Dense,
        chart: qp_grammar::look::ChartLook {
            price_label_px: 120.0,
            mark_max_px: 40.0,
            ..alphabet::TRIAL.chart
        },
        ..alphabet::TRIAL
    };
    let mut bad = BAD;
    bad.dark.down = bad.dark.up;
    let bad: &'static Look = Box::leak(Box::new(bad));
    let fx = scene::fixture();
    let r = rows(bad, &fx, None);
    let failed = |id: &str| r.iter().any(|x| x.id == id && x.res == Res::Fail);
    assert!(failed("P4") && failed("P8") && failed("H3"), "{r:#?}");
    assert!(fails(bad, &r));
    let legacy = Look {
        profile: Profile::Legacy,
        ..*bad
    };
    assert!(!fails(&legacy, &r));
}
