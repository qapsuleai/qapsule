//! Draft hooks of the live path: condition kinds and value sources (`Registry::eval_condition`,
//! `rounds/RF1/PLAN.md` §4.5, §11 D3).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use qp_plane::values::{Cache, Computed};
use qp_schema::{CompareOp, Condition, Document};
use qp_subsystem::{ConditionKind, Registry, Subsystem, ValueSource};

fn doc() -> Document {
    let p =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/protocol/docs/thyao.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

/// Text conditions, which the data plane cannot evaluate, always hold at bar 7.
struct Always;
impl ConditionKind for Always {
    fn kind(&self) -> &'static str {
        "text"
    }
    fn eval(&self, _: &Document, _: &mut Cache, _: &Condition) -> Option<(bool, u64, i64)> {
        Some((true, 7, 70))
    }
}

/// Series `prog:*`: three bars, values 1, 2, 3.
struct Program;
impl ValueSource for Program {
    fn owns(&self, id: &str) -> bool {
        id.starts_with("prog:")
    }
    fn values(&self, _: &Document, _: &str) -> Option<Computed> {
        let out = BTreeMap::from([("v".to_owned(), vec![Some(1.0), Some(2.0), Some(3.0)])]);
        Some(Computed::new(vec![10, 20, 30], out, "v".to_owned()))
    }
}

struct Lang;
impl Subsystem for Lang {
    fn name(&self) -> &'static str {
        "lang"
    }
    fn conditions(&self) -> &'static [&'static dyn ConditionKind] {
        &[&Always]
    }
    fn value_sources(&self) -> &'static [&'static dyn ValueSource] {
        &[&Program]
    }
}

#[test]
fn condition_kinds_and_value_sources_answer_before_the_data_plane() {
    let d = doc();
    let mut cache = Cache::default();
    let text = Condition::Text { text: "x".into() };
    let prog = |op, value| Condition::Threshold {
        series: "prog:x".into(),
        op,
        value,
    };
    let empty = Registry::default();
    assert_eq!(empty.eval_condition(&d, &mut cache, &text), None);
    assert_eq!(
        empty.eval_condition(&d, &mut cache, &prog(CompareOp::Gt, 2.5)),
        None
    );
    let mut r = Registry::default();
    r.add(&Lang).unwrap();
    assert_eq!(r.eval_condition(&d, &mut cache, &text), Some((true, 7, 70)));
    assert_eq!(
        r.eval_condition(&d, &mut cache, &prog(CompareOp::Gt, 2.5)),
        Some((true, 2, 30))
    );
    assert!(!r.holds(&d, &mut cache, &prog(CompareOp::Lt, 2.5)));
}
