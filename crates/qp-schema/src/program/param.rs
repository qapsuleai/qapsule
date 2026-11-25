//! Program parameters (PR §2.1) and the numeric fields that may name one.

use super::*;

/// Value type of a program parameter (the registry's `text` kind does not exist here).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProgramParamKind {
    /// Integer (JSON integer).
    Int,
    /// Float (JSON number; an integer literal is accepted and read as a float).
    Float,
    /// Boolean.
    Bool,
}

/// A parameter value: untagged, so `20` is `Int`, `2.0` is `Float`, `true` is `Bool`.
/// For a `float` parameter the canonical form turns `Int` into `Float`
/// ([`Program::normalized`]).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ProgramParamValue {
    /// Integer.
    Int(i64),
    /// Float.
    Float(f64),
    /// Boolean.
    Bool(bool),
}

/// The definition of one program parameter (PR §2.1). Frozen with `qp-ir/0`: its own type,
/// not the registry's `ParamDef`. `unit` and `description` do not enter `program_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgramParamDef {
    /// Key, `^[a-z][a-z0-9_]{0,31}$`, unique in the program.
    pub key: String,
    /// Value type.
    pub kind: ProgramParamKind,
    /// Unit (`bars`, `ratio`, `pct`, `price`, `ms` ...), description only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Inclusive minimum; required for `int` and `float`, absent for `bool`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<ProgramParamValue>,
    /// Inclusive maximum; required for `int` and `float`, absent for `bool`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<ProgramParamValue>,
    /// Default value, of `kind` and within the range.
    pub default: ProgramParamValue,
    /// Short description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A reference to a program parameter in place of a number: `{"param": "<key>"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParamRef {
    /// Parameter key.
    pub param: String,
}

/// An integer field (`period`, `n`, `bars`, `fast`, `slow`, `signal`, `max_bars`, a line
/// anchor's `t`): a JSON integer or an `int` parameter. Signed so that the checker can
/// answer a negative value with its code (`E_LOOKAHEAD`, `E_BAD_PARAM`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum IntArg {
    /// A literal.
    Lit(i64),
    /// A parameter.
    Param(ParamRef),
}

/// A float field (`k`, a line anchor's `price`, a bucket edge, a rule percentage): a number
/// or a `float` parameter. An integer literal reads as a float and is written `2.0`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Num {
    /// A literal.
    Lit(f64),
    /// A parameter.
    Param(ParamRef),
}
