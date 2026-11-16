//! R3/E hidden setup: prints `qp_source::synthetic_market(2, n, seed)` bars as JSON
//! `[[bars of instrument 0], [bars of instrument 1]]`.
//! usage: r3-ergo-gen <n> <seed>
fn main() {
    let a: Vec<u64> = std::env::args()
        .skip(1)
        .map(|x| x.parse().expect("number"))
        .collect();
    let n = usize::try_from(a.first().copied().unwrap_or(300)).expect("n");
    let seed = a.get(1).copied().unwrap_or(1);
    let m = qp_source::synthetic_market(2, n, seed);
    let out: Vec<_> = m.iter().map(|i| &i.bars).collect();
    println!("{}", serde_json::to_string(&out).expect("json"));
}
