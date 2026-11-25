//! Program and instance identities (PR §2, §2.1): `sha256:` + 64 lowercase hex digits of
//! the SHA-256 of `Program::identity_json` and of `instance_id_input`. The strings to hash
//! are defined in `qp-schema` (`program/canonical.rs`); only the hashing is here.

use std::collections::BTreeMap;

use qp_schema::program::{instance_id_input, Program, ProgramParamValue, PROGRAM_ID_PREFIX};

use crate::sha256::sha256_hex;

/// `program_id` of `program`: description fields (`unit`, `description`) do not enter it,
/// float parameters are normalized (PR §2 rules 1–4).
///
/// # Errors
/// The `serde_json` error if the program does not serialize (never in practice).
pub fn program_id(program: &Program) -> Result<String, serde_json::Error> {
    Ok(format!(
        "{PROGRAM_ID_PREFIX}{}",
        sha256_hex(program.identity_json()?.as_bytes())
    ))
}

/// `instance_id` of a program instance: `program_id` and the complete bound values (by
/// key, normalized to their kinds).
///
/// # Errors
/// As [`program_id`].
pub fn instance_id(
    program_id: &str,
    values: &BTreeMap<String, ProgramParamValue>,
) -> Result<String, serde_json::Error> {
    Ok(format!(
        "{PROGRAM_ID_PREFIX}{}",
        sha256_hex(instance_id_input(program_id, values)?.as_bytes())
    ))
}
