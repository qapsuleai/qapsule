//! RL1/I1: programs in the protocol (docs/PROGRAMS-SESSION.md §1, §3): additive fields
//! serialize exactly as before when absent; the new operations, queries and answers
//! round-trip.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use qp_schema::protocol::{ErrorCode, Op, ProgramError, ProtocolError, Query, QueryResult};
use qp_schema::{Series, SeriesSource};

fn round_trip<T>(j: &str) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let t: T = serde_json::from_str(j).unwrap_or_else(|e| panic!("{j}: {e}"));
    assert_eq!(serde_json::to_string(&t).unwrap(), j, "round trip");
    t
}

/// Study series as written before RL1 (from fixtures/protocol/conformance/02 and a bound
/// MACD output): byte-identical.
#[test]
fn study_without_program_is_byte_identical() {
    let golden = [
        r#"{"id":"series-1-0","source":{"kind":"study","study":"rsi","inputs":["px"],"params":{"period":7.0}}}"#,
        r#"{"id":"m","instrument":{"symbol":"THYAO","exchange":"BIST","calendar":"XIST"},"interval":"1h","source":{"kind":"study","study":"macd","inputs":["px"],"params":{"fast":12.0,"signal":9.0,"slow":26.0},"output":"signal"}}"#,
    ];
    for g in golden {
        let s: Series = round_trip(g);
        let SeriesSource::Study { program, .. } = &s.source else {
            panic!("not a study")
        };
        assert!(program.is_none());
    }
    // Built in code with `program: None`: no `program` key.
    let s = SeriesSource::Study {
        study: "sma".to_owned(),
        inputs: vec!["px".to_owned()],
        params: [("period".to_owned(), 20.0)].into_iter().collect(),
        output: None,
        program: None,
    };
    assert_eq!(
        serde_json::to_string(&s).unwrap(),
        r#"{"kind":"study","study":"sma","inputs":["px"],"params":{"period":20.0}}"#
    );
}

#[test]
fn study_with_program_round_trips() {
    let j = r#"{"id":"p1.kirilim","instrument":{"symbol":"THYAO","exchange":"BIST","calendar":"XIST"},"interval":"1h","source":{"kind":"study","study":"program","inputs":["px"],"params":{},"output":"kirilim","program":{"instance":"p1","program_id":"sha256:aa","instance_id":"sha256:bb","scope":"document","owner":"arkin","since_t":1764151200000,"params":{"len":20},"ir":{"version":"qp-ir/0","params":[{"key":"len","kind":"int","min":2,"max":500,"default":20}],"nodes":[{"id":"h","op":"input","field":"high"},{"id":"top","op":"highest","x":"h","period":{"param":"len"}},{"id":"c","op":"input","field":"close"},{"id":"k","op":"gt","a":"c","b":"top"}],"outputs":[{"name":"kirilim","kind":"event","node":"k"}]}}}}"#;
    let s: Series = round_trip(j);
    let SeriesSource::Study { program, .. } = &s.source else {
        panic!("not a study")
    };
    assert_eq!(program.as_ref().unwrap().instance, "p1");
    let bad = j.replace(r#""scope":"document""#, r#""scope":"personal""#);
    assert!(serde_json::from_str::<Series>(&bad).is_err());
    let bad = j.replace(r#""owner":"arkin""#, r#""owner":"arkin","zz":1"#);
    assert!(serde_json::from_str::<Series>(&bad).is_err());
}

#[test]
fn program_operations_round_trip() {
    let ops = [
        // `program` is raw JSON (keys come back sorted); `params` is a sorted map.
        r#"{"op":"add_program","id":"p1","program":{"nodes":[{"field":"close","id":"c","op":"input"}],"outputs":[{"kind":"series","name":"c","node":"c"}],"version":"qp-ir/0"},"inputs":["px"],"params":{"k":2.0,"len":20},"show":{"view":"main"}}"#,
        r#"{"op":"add_program","program":{"anything":"the checker reads it"},"inputs":["px"]}"#,
        r#"{"op":"set_program_params","program":"p1","params":{"len":30}}"#,
        r#"{"op":"remove_program","program":"p1"}"#,
    ];
    for j in ops {
        let _: Op = round_trip(j);
    }
    for bad in [
        r#"{"op":"add_program","program":{},"inputs":["px"],"source":"series c = close"}"#,
        r#"{"op":"remove_program","program":"p1","zz":1}"#,
        r#"{"op":"set_program_params","program":"p1","params":{"len":"x"}}"#,
    ] {
        assert!(serde_json::from_str::<Op>(bad).is_err(), "accepted {bad}");
    }
}

#[test]
fn program_queries_round_trip() {
    let qs = [
        r#"{"kind":"check_program","source":"series c = close","lang":"tr"}"#,
        r#"{"kind":"check_program","program":{"version":"qp-ir/0"},"inputs":["px"],"params":{"len":3}}"#,
        r#"{"kind":"try_program","instance":"p1","range":{"last":500}}"#,
        r#"{"kind":"explain_program","instance":"p1","at":{"output":"kirilim","t":1704164400000}}"#,
    ];
    for j in qs {
        let _: Query = round_trip(j);
    }
    assert!(serde_json::from_str::<Query>(r#"{"kind":"try_program","zz":1}"#).is_err());
}

#[test]
fn program_answers_are_flat_under_data() {
    let answers = [
        r#"{"kind":"program_check","ok":false,"errors":[{"code":"schema","reason":"E_UNKNOWN_OP","message":"unknown function `rsii`","at":"/nodes/3/op","span":{"line":2,"col":5,"len":4},"expected":["sma","ema","rsi"],"did_you_mean":["rsi"],"hint":"h"}],"truncated":true,"warnings":[{"reason":"W_UNUSED_NODE","message":"m","at":"/nodes/1"}],"limits":{"program.max_nodes":200}}"#,
        r#"{"kind":"program_check","ok":true,"cost":{"per_bar":5,"live_per_bar":6,"bars":3000,"total":15000,"limit":666},"outputs":[{"name":"kirilim","kind":"event","documentable":true}],"program_id":"sha256:aa","reject":{"code":"E_LOOKAHEAD","reason":"r"}}"#,
        r#"{"kind":"program_try","from_t":1,"to_t":2,"series":[{"name":"s","defined":1,"last":[{"t":2,"v":1.5},{"t":3,"v":null}]}],"event":[{"name":"e","events":3,"runs":2,"notifications":2,"consecutive":true,"isolated":1,"first":[{"t":1,"known_t":2}],"last":[]}],"signal":[{"name":"g","count":1,"outcomes":{"target":0,"stop":1,"expired":0,"open":0},"first":[{"id":"p1.rsi_dip@1704164400000","source_kind":"signal","mode":"replay","label":"simulated","provenance":{"owner":"arkin","author":{"kind":"human","name":"arkin"},"program_id":"sha256:aa","instance_id":"sha256:bb","params":{"len":14},"strategy":"rsi_dip","program_version":"1"},"instrument":"THYAO","period":"1h","born_t":1704164400000,"known_t":1704168000000,"side":"al","entry":101.25,"rule":{"target_pct":3.0,"stop_pct":1.5,"max_bars":24},"state":{"output":"regime","key":{"momentum":1}},"outcome":{"kind":"stop","exit_t":1704189600000,"exit_price":99.73},"evidence":{"series":"px","from_t":1704000000000,"to_t":1704164400000,"anchor_t":1700000000000}}]}],"state":[{"name":"regime","distinct":2,"top":[{"key":{"momentum":1},"count":7}]}],"dropped":0,"cost":{"per_bar":5,"live_per_bar":6,"limit":666},"work":{"used":10,"limit":100},"stopped":{"reason":"budget","bar":9,"t":9,"work":100,"limit":100}}"#,
        r#"{"kind":"program_explain","text":"series c = close","program_id":"sha256:aa","params":{"len":3},"cost":[{"id":"c","op":"input","clock":"base","w":0}],"outputs":[{"name":"c","kind":"series","documentable":true}],"provenance":{"output":"c","node":"c","bar":4,"t":5,"range":[4,4],"program_id":"sha256:aa","data":{"series":"px"}}}"#,
    ];
    for j in answers {
        let _: QueryResult = round_trip(j);
    }
    assert!(
        serde_json::from_str::<QueryResult>(r#"{"kind":"program_check","ok":true,"zz":1}"#)
            .is_err()
    );
}

#[test]
fn protocol_error_errors_is_additive() {
    let before = r#"{"code":"invalid","message":"bad","path":"/ops/0","reason":"E_X"}"#;
    let e: ProtocolError = round_trip(before);
    assert!(e.errors.is_empty());
    let with = r#"{"code":"schema","message":"unknown function `rsii`","path":"/ops/0/program/nodes/3/op","reason":"E_UNKNOWN_OP","errors":[{"code":"schema","reason":"E_UNKNOWN_OP","message":"unknown function `rsii`","at":"/nodes/3/op"},{"code":"not_found","reason":"E_UNKNOWN_REF","message":"m","at":"/nodes/4/x"}]}"#;
    let e: ProtocolError = round_trip(with);
    assert_eq!(e.errors.len(), 2);
    assert_eq!(e.errors[1].code, Some(ErrorCode::NotFound));
    let w: ProgramError = round_trip(r#"{"reason":"W_CONSECUTIVE_EVENTS","message":"m"}"#);
    assert!(w.code.is_none());
}
