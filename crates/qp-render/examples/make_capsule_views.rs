//! Writes the R3 capsule view fixtures (`fixtures/views/docs/r3-*.json`,
//! `fixtures/views/c*-*.json`) from the generator in `tests/support/capsule_gen.rs`:
//! `cargo run -p qp-render --example make_capsule_views` (from the repository root).
//! Goldens and explain files are then made with `qp render` / `qp explain` (see
//! `rounds/R3/experiments/S/S-B.md`); existing files are never overwritten unless
//! `--force` is given.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

#[path = "../tests/support/capsule_gen.rs"]
mod capsule_gen;

fn main() {
    let force = std::env::args().any(|a| a == "--force");
    for (rel, content) in capsule_gen::files() {
        let path = std::path::Path::new(&rel);
        let same = std::fs::read_to_string(path).is_ok_and(|have| have == content);
        if same {
            println!("{rel}: unchanged");
            continue;
        }
        if path.exists() && !force {
            println!("{rel}: differs (not written; use --force)");
            continue;
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).expect("create directory");
        }
        std::fs::write(path, content).expect("write");
        println!("{rel}: written");
    }
}
