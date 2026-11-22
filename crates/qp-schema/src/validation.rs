//! Validation: an instance against a generated schema (feature `validate`) and the
//! semantic checks of a bar series that JSON Schema cannot express.

use crate::{Bar, Series};

#[cfg(feature = "validate")]
use crate::export::{json_schema, root_type_names};

/// Validates `instance` against an explicit JSON Schema value.
///
/// # Errors
/// One message per violation (`"<message> at <json pointer>"`), or a single message if
/// the schema itself does not compile.
#[cfg(feature = "validate")]
pub fn validate_with_schema(
    schema: &serde_json::Value,
    instance: &serde_json::Value,
) -> Result<(), Vec<String>> {
    let validator = jsonschema::validator_for(schema)
        .map_err(|e| vec![format!("schema does not compile: {e}")])?;
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("{} at {}", e, e.instance_path()))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validates `instance` against the generated schema of root type `type_name`.
///
/// # Errors
/// One message per violation, or a single message for an unknown type name.
#[cfg(feature = "validate")]
pub fn validate(type_name: &str, instance: &serde_json::Value) -> Result<(), Vec<String>> {
    let schema = json_schema(type_name).ok_or_else(|| {
        vec![format!(
            "unknown type `{type_name}`; expected one of: {}",
            root_type_names().join(", ")
        )]
    })?;
    validate_with_schema(&schema, instance)
}

/// Semantic checks on a bar list that JSON Schema cannot express: non-empty, all values
/// finite, `h >= max(o, c)`, `l <= min(o, c)`, `l <= h`, strictly increasing `t`.
///
/// # Errors
/// The first violation found, naming the bar index.
pub fn validate_bars(bars: &[Bar]) -> Result<(), String> {
    if bars.is_empty() {
        return Err("bar list is empty".to_owned());
    }
    let mut prev_t: Option<i64> = None;
    for (i, b) in bars.iter().enumerate() {
        if [b.o, b.h, b.l, b.c, b.v].iter().any(|v| !v.is_finite()) {
            return Err(format!("bar {i}: non-finite value"));
        }
        if b.h < b.o.max(b.c) || b.l > b.o.min(b.c) || b.l > b.h {
            return Err(format!("bar {i}: inconsistent OHLC"));
        }
        if prev_t.is_some_and(|p| b.t <= p) {
            return Err(format!("bar {i}: timestamp not increasing"));
        }
        prev_t = Some(b.t);
    }
    Ok(())
}

/// Parses a bar series document (a [`Series`] with embedded `bars` data, the
/// `fixtures/bars/*.json` format) and runs [`validate_bars`] on it.
///
/// # Errors
/// Malformed JSON, a series without embedded bars, or a [`validate_bars`] failure.
pub fn parse_bar_series(json: &str) -> Result<Series, String> {
    let series: Series =
        serde_json::from_str(json).map_err(|e| format!("invalid series JSON: {e}"))?;
    let bars = series
        .bars()
        .ok_or_else(|| "series has no embedded bars (`data.kind` must be `bars`)".to_owned())?;
    validate_bars(bars)?;
    Ok(series)
}
