//! Regenerates `fixtures/bars/sample.json` (200 deterministic daily bars, seed 20251114).
//!
//! ```sh
//! cargo run -p qp-render --example gen_sample
//! ```
//! The output is committed; rerunning must produce a byte-identical file.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = qp_source::synthetic_series(200, 20_251_114);
    let mut s = String::new();
    let bars = file.bars().ok_or("synthetic series has no bars")?;
    // A `qp_schema::Series` document, one bar per line.
    writeln!(
        s,
        "{{\"id\":{},\"instrument\":{},\"interval\":{},\"source\":{},\"data\":{{\"kind\":\"bars\",\"bars\":[",
        serde_json::to_string(&file.id)?,
        serde_json::to_string(&file.instrument)?,
        serde_json::to_string(&file.interval)?,
        serde_json::to_string(&file.source)?,
    )?;
    for (i, b) in bars.iter().enumerate() {
        let sep = if i + 1 == bars.len() { "" } else { "," };
        writeln!(s, "{}{sep}", serde_json::to_string(b)?)?;
    }
    s.push_str("]}}\n");
    // Round-trip check before writing.
    let back = qp_render::parse_bars(&s)?;
    assert_eq!(back, file, "sample.json must round-trip exactly");
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/bars/sample.json");
    std::fs::write(&out, s)?;
    println!("wrote {}", out.display());
    Ok(())
}
