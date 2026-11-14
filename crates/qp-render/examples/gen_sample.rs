//! Regenerates `fixtures/bars/sample.json` (200 deterministic daily bars, seed 20251114).
//!
//! ```sh
//! cargo run -p qp-render --example gen_sample
//! ```
//! The output is committed; rerunning must produce a byte-identical file.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = qp_render::synthetic_bars(200, 20_251_114);
    let mut s = String::new();
    writeln!(
        s,
        "{{\"symbol\":{},\"interval\":{},\"bars\":[",
        serde_json::to_string(&file.symbol)?,
        serde_json::to_string(&file.interval)?
    )?;
    for (i, b) in file.bars.iter().enumerate() {
        let sep = if i + 1 == file.bars.len() { "" } else { "," };
        writeln!(s, "{}{sep}", serde_json::to_string(b)?)?;
    }
    s.push_str("]}\n");
    // Round-trip check before writing.
    let back = qp_render::parse_bars(&s)?;
    assert_eq!(back, file, "sample.json must round-trip exactly");
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/bars/sample.json");
    std::fs::write(&out, s)?;
    println!("wrote {}", out.display());
    Ok(())
}
