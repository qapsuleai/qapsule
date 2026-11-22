//! Renders `docs/PARAMETERS.generated.md` from [`REGISTRY`] (shared by the
//! `params_doc` example and the freshness test).

use std::fmt::Write as _;

use super::system::REGISTRY;
use super::{ParamDef, ParamValue};

/// The generated document's first line.
pub const HEADER: &str =
    "üretilmiştir: cargo run -p qp-schema --example params_doc (elle düzenlemeyin)";

/// The whole Markdown document: a header and one table built only from [`REGISTRY`].
pub fn render_markdown() -> String {
    let mut out = String::new();
    out.push_str(HEADER);
    out.push_str("\n\n# Parameters (generated)\n\n");
    out.push_str(
        "Source: `qp_schema::params::system::REGISTRY`. Resolution order: object > user > \
         plan > system (`docs/ENGINEERING.md` §6). Inventory: `docs/PARAMETERS.md`.\n\n",
    );
    out.push_str(
        "| Key | Kind | Unit | Range | Default | Scopes | Changed by | Entitlement | \
         Description | Since |\n",
    );
    out.push_str("|---|---|---|---|---|---|---|---|---|---|\n");
    for def in REGISTRY {
        row(&mut out, def);
    }
    out
}

fn row(out: &mut String, def: &ParamDef) {
    let scopes: Vec<&str> = def.scopes.iter().map(|s| s.name()).collect();
    let _ = writeln!(
        out,
        "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
        def.key,
        def.kind.name(),
        def.unit,
        range(def.min, def.max),
        value(&def.default),
        scopes.join(", "),
        def.changed_by.name(),
        if def.entitlement { "yes" } else { "no" },
        def.description,
        def.since,
    );
}

fn range(min: Option<f64>, max: Option<f64>) -> String {
    let side = |b: Option<f64>| b.map(|v| v.to_string()).unwrap_or_default();
    match (min, max) {
        (None, None) => "—".to_string(),
        _ => format!("{}..{}", side(min), side(max)),
    }
}

fn value(v: &ParamValue) -> String {
    match v {
        ParamValue::Int(i) => i.to_string(),
        ParamValue::Float(f) => format!("{f:?}"),
        ParamValue::Bool(b) => b.to_string(),
        ParamValue::Text(t) => format!("`{t}`"),
    }
}
