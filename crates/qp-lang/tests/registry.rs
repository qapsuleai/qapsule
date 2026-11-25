//! Every RL1 key documented in `docs/PARAMETERS.md` is a registry row with the documented
//! kind, unit, range and default, and `Limits` reads exactly those rows (one source).

#![allow(clippy::panic, clippy::expect_used)]

use qp_lang::{Limit, Limits};
use qp_schema::params::{by_key, ParamValue};

const DOC: &str = include_str!("../../../docs/PARAMETERS.md");

/// The RL1 table rows: `(key, kind, unit, range, default)`.
fn documented() -> Vec<[String; 5]> {
    let start = DOC.find("## RL1").expect("RL1 section");
    DOC[start..]
        .lines()
        .filter(|l| l.starts_with("| `program."))
        .map(|l| {
            let c: Vec<String> = l.split('|').map(|s| s.trim().to_owned()).collect();
            [
                c[1].trim_matches('`').to_owned(),
                c[2].clone(),
                c[3].clone(),
                c[4].clone(),
                c[5].clone(),
            ]
        })
        .collect()
}

fn range_text(min: Option<f64>, max: Option<f64>) -> String {
    let side = |b: Option<f64>| b.map(|v| v.to_string()).unwrap_or_default();
    format!("{}..{}", side(min), side(max))
}

#[test]
fn documented_rl1_keys_are_registry_rows() {
    let rows = documented();
    assert_eq!(rows.len(), Limit::ALL.len(), "documented RL1 keys");
    for [key, kind, unit, range, default] in &rows {
        let def = by_key(key).unwrap_or_else(|| panic!("{key} is not in the registry"));
        assert_eq!(def.kind.name(), kind, "{key}: kind");
        assert_eq!(def.unit, unit, "{key}: unit");
        assert_eq!(&range_text(def.min, def.max), range, "{key}: range");
        let v: i64 = default.parse().expect("integer default");
        assert_eq!(def.default, ParamValue::Int(v), "{key}: default");
        assert_eq!(def.since, "RL1", "{key}: since");
    }
}

#[test]
fn limits_read_the_documented_rows() {
    let rows = documented();
    let defaults = Limits::default();
    for (l, [key, ..]) in Limit::ALL.iter().zip(&rows) {
        assert_eq!(l.key(), key, "Limit order follows the document");
        let ParamValue::Int(v) = l.def().default else {
            panic!("{key}: not an int");
        };
        assert_eq!(defaults.get(*l), u64::try_from(v).expect("non-negative"));
    }
    assert_eq!(defaults.to_map().len(), rows.len());
}
