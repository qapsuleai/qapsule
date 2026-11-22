//! `qp schema export` and `qp schema validate` (RG1: split out of `main.rs`).

use crate::*;

pub(crate) fn schema_export(out: &Path) -> Result<Outcome, String> {
    let written = qp_schema::export_schemas(out)
        .map_err(|e| format!("cannot export schemas to {}: {e}", out.display()))?;
    for path in written {
        println!("{}", path.display());
    }
    Ok(Outcome::Ok)
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("invalid JSON in {}: {e}", path.display()))
}

pub(crate) fn schema_validate(
    file: &Path,
    type_name: &str,
    schema_dir: Option<&Path>,
) -> Result<Outcome, String> {
    let instance = read_json(file)?;
    let result = match schema_dir {
        Some(dir) => {
            let schema = read_json(&dir.join(format!("{type_name}.schema.json")))?;
            qp_schema::validate_with_schema(&schema, &instance)
        }
        None => {
            if qp_schema::json_schema(type_name).is_none() {
                return Err(format!(
                    "unknown type `{type_name}`; expected one of: {}",
                    qp_schema::root_type_names().join(", ")
                ));
            }
            qp_schema::validate(type_name, &instance)
        }
    };
    match result {
        Ok(()) => {
            println!("valid");
            Ok(Outcome::Ok)
        }
        Err(errors) => {
            for err in &errors {
                println!("error: {err}");
            }
            println!("invalid ({} error(s))", errors.len());
            Ok(Outcome::Invalid)
        }
    }
}
