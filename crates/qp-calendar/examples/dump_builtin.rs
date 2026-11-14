//! Writes the built-in sample calendars to `fixtures/calendar/{xist,xnys}.json`.
//!
//! ```sh
//! cargo run -p qp-calendar --example dump_builtin
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/calendar");
    std::fs::create_dir_all(&dir)?;
    for (id, file) in [("XIST", "xist"), ("XNYS", "xnys")] {
        let cal = qp_calendar::builtin(id).ok_or("unknown calendar")?;
        let mut text = serde_json::to_string_pretty(&cal)?;
        text.push('\n');
        std::fs::write(dir.join(format!("{file}.json")), text)?;
    }
    Ok(())
}
