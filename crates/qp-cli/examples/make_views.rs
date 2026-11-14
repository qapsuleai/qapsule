//! Writes the R2 view fixtures (`fixtures/views/`): the base and growth documents and one
//! transcript per view. `cargo run -p qp-cli --example make_views` from anywhere;
//! `tests/views.rs` checks that the committed files match.

#[path = "../tests/support/views_gen.rs"]
mod views_gen;

use std::path::Path;

fn main() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (rel, content) in views_gen::files(&root)? {
        let path = root.join(&rel);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&path, content).map_err(|e| format!("{rel}: {e}"))?;
        println!("{rel}");
    }
    Ok(())
}
