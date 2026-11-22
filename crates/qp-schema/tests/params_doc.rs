//! `docs/PARAMETERS.generated.md` is current: it equals what the registry renders.

#[test]
fn params_generated_doc_is_current() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/PARAMETERS.generated.md"
    );
    let committed = std::fs::read_to_string(path).unwrap_or_default();
    assert!(
        committed == qp_schema::params::doc::render_markdown(),
        "{path} is stale: run `cargo run -p qp-schema --example params_doc > docs/PARAMETERS.generated.md`"
    );
}
