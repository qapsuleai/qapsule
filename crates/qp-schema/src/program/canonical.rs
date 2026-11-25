//! Canonical JSON and the identity forms behind `program_id` and `instance_id` (PR §2,
//! §2.1). Hashing is not here: SHA-256 lives above the schema layer
//! (`qp_capsule::trust::seal::sha256`), so `qp-lang` hashes these strings.

use super::*;

impl ProgramParamDef {
    /// The definition with a `float` parameter's integer-written values read as floats
    /// (`2` → `2.0`, PR §2.1).
    #[must_use]
    pub fn normalized(&self) -> ProgramParamDef {
        let mut d = self.clone();
        if d.kind == ProgramParamKind::Float {
            for v in [&mut d.min, &mut d.max].into_iter().flatten() {
                *v = v.as_float();
            }
            d.default = d.default.as_float();
        }
        d
    }
}

impl ProgramParamValue {
    /// An `Int` as `Float` (exact for |i| ≤ 2^53); other values unchanged.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub const fn as_float(self) -> ProgramParamValue {
        match self {
            ProgramParamValue::Int(i) => ProgramParamValue::Float(i as f64),
            v => v,
        }
    }
}

impl Program {
    /// The canonical program: parameter definitions normalized
    /// ([`ProgramParamDef::normalized`]). Every other number is typed already (integer
    /// fields are `i64`, float fields `f64`) and absent optional fields are not written.
    #[must_use]
    pub fn normalized(&self) -> Program {
        let mut p = self.clone();
        p.params = p.params.iter().map(ProgramParamDef::normalized).collect();
        p
    }

    /// Canonical JSON (PR §2): one line, fields in type order, of [`Program::normalized`].
    ///
    /// # Errors
    /// Never in practice (every field serializes); the `serde_json` error otherwise.
    pub fn canonical_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.normalized())
    }

    /// The identity form (PR §2, rule 3): canonical JSON with the description fields
    /// (`unit`, `description` of parameters) removed. `program_id` = `sha256:` + the
    /// SHA-256 of this string, as 64 lowercase hex digits.
    ///
    /// # Errors
    /// As [`Program::canonical_json`].
    pub fn identity_json(&self) -> Result<String, serde_json::Error> {
        let mut p = self.normalized();
        for d in &mut p.params {
            d.unit = None;
            d.description = None;
        }
        serde_json::to_string(&p)
    }
}

/// The string `instance_id` hashes (PR §2.1): `program_id`, a newline, and the canonical
/// JSON of the bound values (complete, by key, already normalized to their kinds).
/// `instance_id` = `sha256:` + the SHA-256 of this string.
///
/// # Errors
/// Never in practice; the `serde_json` error otherwise.
pub fn instance_id_input(
    program_id: &str,
    values: &BTreeMap<String, ProgramParamValue>,
) -> Result<String, serde_json::Error> {
    Ok(format!("{program_id}\n{}", serde_json::to_string(values)?))
}
