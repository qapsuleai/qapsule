//! RL1/I1: the `qp-ir/0` types (docs/PROGRAMS.md §2–§6): every node and output kind
//! round-trips byte-exactly, unknown fields are rejected, canonical JSON and the identity
//! form are stable.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use qp_schema::program::Program;
use qp_schema::program::{
    instance_id_input, Node, NodeOp, Output, ProgramDoc, ProgramOutputKind, ProgramParamValue,
    PROGRAM_VERSION,
};

/// One canonical node per `op` (PR §4), in table order.
const NODES: &[&str] = &[
    r#"{"id":"c","op":"input","field":"close"}"#,
    r#"{"id":"h4","op":"input","field":"high","tf":"4h"}"#,
    r#"{"id":"k1","op":"const","value":70.0}"#,
    r#"{"id":"k2","op":"const","value":true}"#,
    r#"{"id":"k3","op":"const","value":{"param":"lvl"}}"#,
    r#"{"id":"i","op":"bar_index"}"#,
    r#"{"id":"t","op":"time"}"#,
    r#"{"id":"s","op":"sma","x":"c","period":20}"#,
    r#"{"id":"e","op":"ema","x":"c","period":{"param":"len"}}"#,
    r#"{"id":"r","op":"rsi","x":"c","period":14}"#,
    r#"{"id":"m","op":"macd","x":"c","fast":12,"slow":26,"signal":9}"#,
    r#"{"id":"b","op":"bollinger","x":"c","period":20,"k":2.0}"#,
    r#"{"id":"hi","op":"highest","x":"c","period":20}"#,
    r#"{"id":"lo","op":"lowest","x":"c","period":20}"#,
    r#"{"id":"a","op":"atr","period":14}"#,
    r#"{"id":"a1","op":"atr","period":14,"tf":"1d"}"#,
    r#"{"id":"sd","op":"stdev","x":"c","period":20}"#,
    r#"{"id":"pr","op":"percentrank","x":"c","period":100}"#,
    r#"{"id":"ad","op":"add","a":"c","b":"k1"}"#,
    r#"{"id":"su","op":"sub","a":"c","b":"k1"}"#,
    r#"{"id":"mu","op":"mul","a":"c","b":"k1"}"#,
    r#"{"id":"dv","op":"div","a":"c","b":"k1"}"#,
    r#"{"id":"mn","op":"min","a":"c","b":"k1"}"#,
    r#"{"id":"mx","op":"max","a":"c","b":"k1"}"#,
    r#"{"id":"ng","op":"neg","x":"c"}"#,
    r#"{"id":"ab","op":"abs","x":"c"}"#,
    r#"{"id":"lg","op":"lag","x":"c","n":1}"#,
    r#"{"id":"gt","op":"gt","a":"c","b":"k1"}"#,
    r#"{"id":"ge","op":"ge","a":"c","b":"k1"}"#,
    r#"{"id":"lt","op":"lt","a":"c","b":"k1"}"#,
    r#"{"id":"le","op":"le","a":"c","b":"k1"}"#,
    r#"{"id":"eq","op":"eq","a":"c","b":"k1"}"#,
    r#"{"id":"ne","op":"ne","a":"c","b":"k1"}"#,
    r#"{"id":"an","op":"and","a":"gt","b":"lt"}"#,
    r#"{"id":"or","op":"or","a":"gt","b":"lt"}"#,
    r#"{"id":"nt","op":"not","x":"gt"}"#,
    r#"{"id":"xa","op":"cross_above","a":"c","b":"m.signal"}"#,
    r#"{"id":"xb","op":"cross_below","a":"c","b":"b.upper"}"#,
    r#"{"id":"st","op":"streak","c":"gt"}"#,
    r#"{"id":"bs","op":"bars_since","c":"gt"}"#,
    r#"{"id":"on","op":"once","c":"gt"}"#,
    r#"{"id":"ra","op":"rearm","trigger":"lt","reset":"gt"}"#,
    r#"{"id":"cd","op":"cooldown","trigger":"xa","bars":10}"#,
    r#"{"id":"df","op":"defined","x":"s"}"#,
    r#"{"id":"al","op":"align","x":"h4"}"#,
    r#"{"id":"ln","op":"line","a":{"t":1704787200000,"price":98.0},"b":{"t":{"param":"t2"},"price":101.53}}"#,
    r#"{"id":"bk","op":"bucket","x":"r","edges":[30.0,{"param":"hi_edge"},70.0]}"#,
];

/// One canonical output per kind (PR §5).
const OUTPUTS: &[&str] = &[
    r#"{"name":"spread","kind":"series","node":"s"}"#,
    r#"{"name":"kirilim","kind":"event","node":"xa"}"#,
    r#"{"name":"rsi_dip","kind":"signal","node":"cd","side":"al","rule":{"target_pct":3.0,"stop_pct":1.5,"max_bars":24},"source":{"strategy":"rsi_dip","version":"1"}}"#,
    r#"{"name":"short","kind":"signal","node":"cd","side":"sat","rule":{"max_bars":{"param":"h"}},"source":{"strategy":"x","version":"2"}}"#,
    r#"{"name":"regime","kind":"state","dims":[{"name":"momentum","node":"bk"}]}"#,
];

fn op_of(json: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    v["op"].as_str().unwrap().to_owned()
}

#[test]
fn every_node_kind_round_trips_byte_exactly() {
    let mut ops = std::collections::BTreeSet::new();
    for &j in NODES {
        let n: Node = serde_json::from_str(j).unwrap_or_else(|e| panic!("{j}: {e}"));
        assert_eq!(serde_json::to_string(&n).unwrap(), j, "round trip of {j}");
        assert_eq!(n.op.name(), op_of(j), "name() of {j}");
        ops.insert(n.op.name());
    }
    // Every op of PR §4: 4 inputs, 10 kernels, 8 arithmetic + lag, 6 compare, 3 logic,
    // 2 crosses, 5 state, defined, align, line, bucket.
    assert_eq!(ops.len(), 43, "{ops:?}");
}

#[test]
fn every_output_kind_round_trips_byte_exactly() {
    let mut kinds = std::collections::BTreeSet::new();
    for &j in OUTPUTS {
        let o: Output = serde_json::from_str(j).unwrap_or_else(|e| panic!("{j}: {e}"));
        assert_eq!(serde_json::to_string(&o).unwrap(), j, "round trip of {j}");
        kinds.insert(format!("{:?}", o.kind.tag()));
    }
    assert_eq!(kinds.len(), 4);
    let ev: Output = serde_json::from_str(OUTPUTS[1]).unwrap();
    assert_eq!(ev.kind.tag(), ProgramOutputKind::Event);
    assert!(ev.kind.documentable());
    let sig: Output = serde_json::from_str(OUTPUTS[2]).unwrap();
    assert!(!sig.kind.documentable());
}

#[test]
fn references_and_multi_outputs() {
    let m: Node = serde_json::from_str(NODES[10]).unwrap();
    assert_eq!(m.op.outputs(), ["macd", "signal", "histogram"]);
    assert_eq!(m.op.main_output(), Some("macd"));
    let b: Node = serde_json::from_str(NODES[11]).unwrap();
    assert_eq!(b.op.main_output(), Some("middle"));
    let x: Node = serde_json::from_str(NODES[36]).unwrap();
    assert_eq!(x.op.refs(), ["c", "m.signal"]);
    let r: Node = serde_json::from_str(NODES[41]).unwrap();
    assert_eq!(r.op.refs(), ["lt", "gt"]);
    let s: Node = serde_json::from_str(NODES[7]).unwrap();
    assert!(s.op.outputs().is_empty() && s.op.main_output().is_none());
}

#[test]
fn unknown_and_malformed_fields_are_rejected() {
    let bad = [
        // unknown field in a node, next to the head and in a nested anchor
        r#"{"id":"s","op":"sma","x":"c","period":20,"zz":1}"#,
        r#"{"id":"s","op":"bar_index","zz":1}"#,
        r#"{"id":"l","op":"line","a":{"t":1,"price":1.0,"zz":0},"b":{"t":2,"price":2.0}}"#,
        // `partial` is not a field (the checker answers it with E_LOOKAHEAD on raw JSON)
        r#"{"id":"c","op":"input","field":"close","partial":true}"#,
        // unknown op, unknown bar field, missing / non-string id, missing op
        r#"{"id":"s","op":"rsii","x":"c","period":14}"#,
        r#"{"id":"c","op":"input","field":"hl2"}"#,
        r#"{"op":"time"}"#,
        r#"{"id":3,"op":"time"}"#,
        r#"{"id":"t"}"#,
        // an integer field takes no float, a param ref takes nothing else
        r#"{"id":"s","op":"sma","x":"c","period":20.0}"#,
        r#"{"id":"s","op":"sma","x":"c","period":{"param":"p","zz":1}}"#,
    ];
    for j in bad {
        assert!(serde_json::from_str::<Node>(j).is_err(), "accepted {j}");
    }
    let bad_outputs = [
        r#"{"name":"a","kind":"series","node":"s","zz":1}"#,
        r#"{"name":"a","kind":"signal","node":"s","side":"tut","rule":{"max_bars":1},"source":{"strategy":"x","version":"1"}}"#,
        r#"{"name":"a","kind":"signal","node":"s","side":"al","source":{"strategy":"x","version":"1"}}"#,
        r#"{"name":"a","kind":"signal","node":"s","side":"al","rule":{"max_bars":1,"kind":"single"},"source":{"strategy":"x","version":"1"}}"#,
        r#"{"name":"a","kind":"fingerprint","dims":[]}"#,
        r#"{"kind":"series","node":"s"}"#,
    ];
    for j in bad_outputs {
        assert!(serde_json::from_str::<Output>(j).is_err(), "accepted {j}");
    }
    let bad_programs = [
        r#"{"version":"qp-ir/0","nodes":[],"outputs":[],"zz":1}"#,
        r#"{"version":"qp-ir/0","params":[{"key":"p","kind":"int","min":1,"max":2,"default":1,"zz":0}],"nodes":[],"outputs":[]}"#,
        r#"{"version":"qp-ir/0","params":[{"key":"p","kind":"text","default":1}],"nodes":[],"outputs":[]}"#,
    ];
    for j in bad_programs {
        assert!(serde_json::from_str::<Program>(j).is_err(), "accepted {j}");
    }
}

/// The `sma` program of PR §14, exactly as the spec writes it (canonical).
const SMA: &str = r#"{"version":"qp-ir/0","params":[{"key":"period","kind":"int","min":1,"max":2000,"default":20}],"nodes":[{"id":"_1","op":"input","field":"close"},{"id":"_2","op":"sma","x":"_1","period":{"param":"period"}}],"outputs":[{"name":"sma","kind":"series","node":"_2"}]}"#;

#[test]
fn canonical_json_is_the_spec_form_and_stable() {
    let p: Program = serde_json::from_str(SMA).unwrap();
    assert_eq!(p.version, PROGRAM_VERSION);
    let c = p.canonical_json().unwrap();
    assert_eq!(c, SMA);
    // idempotent and equal to plain serialization when already normal
    let again: Program = serde_json::from_str(&c).unwrap();
    assert_eq!(again.canonical_json().unwrap(), c);
    assert_eq!(serde_json::to_string(&p).unwrap(), c);
    // pretty / reordered input reaches the same canonical form
    let messy = r#"{ "outputs": [ {"node":"_2","kind":"series","name":"sma"} ],
        "nodes": [ {"field":"close","op":"input","id":"_1"},
                   {"period":{"param":"period"},"x":"_1","op":"sma","id":"_2"} ],
        "params": [ {"default":20,"max":2000,"min":1,"kind":"int","key":"period"} ],
        "version": "qp-ir/0" }"#;
    let m: Program = serde_json::from_str(messy).unwrap();
    assert_eq!(m.canonical_json().unwrap(), SMA);
}

#[test]
fn canonical_numbers_follow_the_type() {
    // A float parameter written with integers and an integer-written float field.
    let src = r#"{"version":"qp-ir/0",
        "params":[{"key":"k","kind":"float","min":0,"max":100,"default":2}],
        "nodes":[{"id":"c","op":"input","field":"close"},
                 {"id":"b","op":"bollinger","x":"c","period":20,"k":2},
                 {"id":"q","op":"const","value":70}],
        "outputs":[{"name":"middle","kind":"series","node":"b.middle"}]}"#;
    let p: Program = serde_json::from_str(src).unwrap();
    let c = p.canonical_json().unwrap();
    assert!(c.contains(r#""min":0.0,"max":100.0,"default":2.0"#), "{c}");
    assert!(
        c.contains(r#""k":2.0"#) && c.contains(r#""value":70.0"#),
        "{c}"
    );
    assert!(
        c.contains(r#""period":20,"#),
        "integer field stays an integer: {c}"
    );
    // An int parameter keeps integers.
    let q: Program = serde_json::from_str(SMA).unwrap();
    assert!(q.canonical_json().unwrap().contains(r#""default":20}"#));
    // f64 survives bit for bit.
    let v = 0.1_f64 + 0.2;
    let j = format!(
        r#"{{"version":"qp-ir/0","nodes":[{{"id":"q","op":"const","value":{v:?}}}],"outputs":[{{"name":"q","kind":"series","node":"q"}}]}}"#
    );
    let r: Program = serde_json::from_str(&j).unwrap();
    assert_eq!(r.canonical_json().unwrap(), j);
}

#[test]
fn identity_form_rules() {
    // (1) absent optional fields are not written: no `params` key when empty, no `tf`,
    // no rule percentages.
    let bare = r#"{"version":"qp-ir/0","nodes":[{"id":"c","op":"input","field":"close"}],"outputs":[{"name":"c","kind":"series","node":"c"}]}"#;
    let p: Program = serde_json::from_str(bare).unwrap();
    assert_eq!(p.canonical_json().unwrap(), bare);
    assert_eq!(p.identity_json().unwrap(), bare);
    let with_empty = bare.replace(r#""nodes""#, r#""params":[],"nodes""#);
    let q: Program = serde_json::from_str(&with_empty).unwrap();
    assert_eq!(
        q.identity_json().unwrap(),
        bare,
        "empty params equal absent params"
    );
    // (3) unit and description do not enter the identity form; the canonical form keeps them.
    let described = SMA
        .replace(
            r#""kind":"int","min":1"#,
            r#""kind":"int","unit":"bars","min":1"#,
        )
        .replace(
            r#""default":20}"#,
            r#""default":20,"description":"window"}"#,
        );
    let d: Program = serde_json::from_str(&described).unwrap();
    assert_eq!(d.canonical_json().unwrap(), described);
    let plain: Program = serde_json::from_str(SMA).unwrap();
    assert_eq!(d.identity_json().unwrap(), plain.identity_json().unwrap());
    assert_eq!(plain.identity_json().unwrap(), SMA);
    // A real change does change it.
    let other: Program =
        serde_json::from_str(&SMA.replace(r#""default":20"#, r#""default":21"#)).unwrap();
    assert_ne!(
        other.identity_json().unwrap(),
        plain.identity_json().unwrap()
    );
}

#[test]
fn instance_identity_input_is_program_id_newline_values() {
    let mut v = BTreeMap::new();
    v.insert("slow".to_owned(), ProgramParamValue::Int(26));
    v.insert("k".to_owned(), ProgramParamValue::Float(2.0));
    v.insert("on".to_owned(), ProgramParamValue::Bool(true));
    let s = instance_id_input("sha256:ab", &v).unwrap();
    assert_eq!(s, "sha256:ab\n{\"k\":2.0,\"on\":true,\"slow\":26}");
}

#[test]
fn param_values_keep_their_written_type() {
    let v: Vec<ProgramParamValue> = serde_json::from_str("[20, 20.0, true, -3]").unwrap();
    assert_eq!(
        v,
        [
            ProgramParamValue::Int(20),
            ProgramParamValue::Float(20.0),
            ProgramParamValue::Bool(true),
            ProgramParamValue::Int(-3)
        ]
    );
    assert_eq!(serde_json::to_string(&v).unwrap(), "[20,20.0,true,-3]");
}

#[test]
fn reject_form_and_program_doc() {
    let r = r#"{"reject":{"code":"E_LOOKAHEAD","reason":"the next bar's close is not known"}}"#;
    let d: ProgramDoc = serde_json::from_str(r).unwrap();
    assert!(matches!(d, ProgramDoc::Reject(_)));
    assert_eq!(serde_json::to_string(&d).unwrap(), r);
    let p: ProgramDoc = serde_json::from_str(SMA).unwrap();
    assert!(matches!(p, ProgramDoc::Program(_)));
    assert_eq!(serde_json::to_string(&p).unwrap(), SMA);
    assert!(
        serde_json::from_str::<ProgramDoc>(r#"{"reject":{"code":"E_X","reason":"r","zz":1}}"#)
            .is_err()
    );
}

#[test]
fn generated_schema_accepts_programs_and_rejects_unknown_fields() {
    let schema = qp_schema::json_schema("Program").expect("Program root schema");
    let v: serde_json::Value = serde_json::from_str(SMA).unwrap();
    qp_schema::validate_with_schema(&schema, &v).unwrap_or_else(|e| panic!("{e:?}"));
    let doc_schema = schemars::schema_for!(ProgramDoc).to_value();
    for doc in [SMA, r#"{"reject":{"code":"E_LOOKAHEAD","reason":"r"}}"#] {
        let v: serde_json::Value = serde_json::from_str(doc).unwrap();
        qp_schema::validate_with_schema(&doc_schema, &v).unwrap_or_else(|e| panic!("{doc}: {e:?}"));
    }
    let all_nodes = format!(
        r#"{{"version":"qp-ir/0","nodes":[{}],"outputs":[{}]}}"#,
        NODES.join(","),
        OUTPUTS.join(",")
    );
    let v: serde_json::Value = serde_json::from_str(&all_nodes).unwrap();
    qp_schema::validate_with_schema(&schema, &v).unwrap_or_else(|e| panic!("{e:?}"));
    for bad in [
        SMA.replace(r#""field":"close""#, r#""field":"close","zz":1"#),
        SMA.replace(
            r#""kind":"series","node""#,
            r#""kind":"series","zz":1,"node""#,
        ),
        SMA.replace(r#""op":"sma""#, r#""op":"smaa""#),
    ] {
        let v: serde_json::Value = serde_json::from_str(&bad).unwrap();
        assert!(
            qp_schema::validate_with_schema(&schema, &v).is_err(),
            "schema accepted {bad}"
        );
    }
}

#[test]
fn node_op_name_covers_the_enum() {
    // A node built in code serializes id first, then op.
    let n = Node {
        id: "z".to_owned(),
        op: NodeOp::Time {},
    };
    assert_eq!(
        serde_json::to_string(&n).unwrap(),
        r#"{"id":"z","op":"time"}"#
    );
}
