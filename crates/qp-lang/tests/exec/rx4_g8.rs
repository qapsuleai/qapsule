//! RX4 conformance G8-02 and G8-03 (RL1 U1): `bucket` on `4h` brought down with `align`
//! into a state output. The bin counts and the empty prefix equal RX4's expected
//! fingerprint on the synthetic `seen` set (`refs/experiments/rx4/harness/expected`).

use std::collections::BTreeMap;

use qp_schema::Bar;
use serde_json::Value;

use crate::support::*;

const RX4: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../rounds/RL1/experiments/rx4/"
);

fn load(rel: &str) -> Value {
    let text = std::fs::read_to_string(format!("{RX4}{rel}")).expect("rx4 file");
    serde_json::from_str(&text).expect("json")
}

/// Empty keys and `"k1,k2,…:count"` of the state output on the seen bars.
fn fingerprint(program: &str) -> (usize, String) {
    let b: Vec<Bar> =
        serde_json::from_value(load("data/seen.bars.json")["data"]["bars"].clone()).expect("bars");
    let x = run(&checked(&load(&format!("programs/{program}.qp.json"))), &b);
    assert_eq!(x.closed_len(), 3000);
    let mut counts = BTreeMap::new();
    let mut empty = 0;
    for i in 0..x.closed_len() {
        match x.state_at(0, i) {
            Some(k) => {
                let k: Vec<String> = k.iter().map(u32::to_string).collect();
                *counts.entry(k.join(",")).or_insert(0) += 1;
            }
            None => empty += 1,
        }
    }
    let text: Vec<String> = counts.iter().map(|(k, n)| format!("{k}:{n}")).collect();
    (empty, text.join(" "))
}

#[test]
fn g8_02_four_hour_trend_and_range_bins() {
    let want = "0,0,0,0:45 0,0,0,1:28 0,0,0,2:171 0,0,1,0:99 0,0,1,1:27 0,0,1,2:103 0,0,2,0:9 0,0,2,2:22 0,1,0,0:15 0,1,0,1:17 0,1,0,2:69 0,1,1,0:20 0,1,1,1:7 0,1,1,2:30 0,1,2,0:4 0,1,2,2:16 1,0,0,0:13 1,0,0,2:4 1,0,1,0:71 1,0,1,1:18 1,0,1,2:56 1,0,2,0:10 1,0,2,2:7 1,1,0,0:82 1,1,0,1:27 1,1,0,2:58 1,1,1,0:317 1,1,1,1:80 1,1,1,2:240 1,1,2,0:53 1,1,2,1:13 1,1,2,2:89 1,2,0,0:10 1,2,0,1:1 1,2,0,2:2 1,2,1,0:67 1,2,1,1:23 1,2,1,2:27 1,2,2,0:16 1,2,2,1:3 1,2,2,2:1 2,1,0,0:1 2,1,0,2:3 2,1,1,0:22 2,1,1,1:9 2,1,1,2:29 2,1,2,0:20 2,1,2,1:4 2,1,2,2:100 2,2,0,0:2 2,2,0,1:3 2,2,0,2:9 2,2,1,0:108 2,2,1,1:48 2,2,1,2:91 2,2,2,0:64 2,2,2,1:45 2,2,2,2:217";
    assert_eq!(fingerprint("G8-02"), (255, want.to_owned()));
}

#[test]
fn g8_03_four_hour_trend_and_position_bins() {
    let want = "0,0,0:137 0,0,1:7 0,1,0:349 0,1,1:344 0,1,2:282 0,2,1:14 0,2,2:229 1,0,0:131 1,0,1:1 1,1,0:323 1,1,1:336 1,1,2:338 1,2,0:4 1,2,1:18 1,2,2:268";
    assert_eq!(fingerprint("G8-03"), (219, want.to_owned()));
}
