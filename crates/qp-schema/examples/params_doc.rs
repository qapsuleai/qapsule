//! Prints `docs/PARAMETERS.generated.md` from the parameter registry:
//! `cargo run -p qp-schema --example params_doc > docs/PARAMETERS.generated.md`.

fn main() {
    print!("{}", qp_schema::params::doc::render_markdown());
}
