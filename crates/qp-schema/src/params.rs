//! Parameter registry (K78, `docs/ENGINEERING.md` §6): every value that sets a behaviour,
//! a limit or a default is defined once here, with its type, unit, range, default, the
//! scopes that may change it, who changes it and whether it is a sellable entitlement.
//!
//! The registry is not part of the exported JSON schema (`qp schema export`): these types
//! carry no `JsonSchema`/serde derives. The defined keys are listed in [`system::REGISTRY`];
//! `docs/PARAMETERS.generated.md` is rendered from it by [`doc::render_markdown`].
//!
//! Resolution is pure and deterministic: [`resolve`] reads no clock, no file, no network.

pub mod doc;
pub mod program;
pub mod system;

pub use system::{by_key, REGISTRY};

/// A level at which a parameter value may be set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParamScope {
    /// The built-in default shipped with the release.
    System,
    /// The person's plan (subscription tier).
    Plan,
    /// The person's own profile.
    User,
    /// One instrument (symbol); declared in definitions, not yet in the resolution order.
    Instrument,
    /// One object in a document (a watch, a study, a layer).
    Object,
}

impl ParamScope {
    /// Lowercase English name, as used in the generated document.
    pub const fn name(self) -> &'static str {
        match self {
            ParamScope::System => "system",
            ParamScope::Plan => "plan",
            ParamScope::User => "user",
            ParamScope::Instrument => "instrument",
            ParamScope::Object => "object",
        }
    }
}

/// Who may change a parameter's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangedBy {
    /// The person, from their profile.
    User,
    /// An administrator (plan or system level).
    Admin,
    /// Only a new release changes it.
    ReleaseOnly,
}

impl ChangedBy {
    /// Lowercase English name, as used in the generated document.
    pub const fn name(self) -> &'static str {
        match self {
            ChangedBy::User => "user",
            ChangedBy::Admin => "admin",
            ChangedBy::ReleaseOnly => "release only",
        }
    }
}

/// The value type of a parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamKind {
    /// Signed 64-bit integer.
    Int,
    /// 64-bit float.
    Float,
    /// Boolean.
    Bool,
    /// Static text (an enumerated choice).
    Text,
}

impl ParamKind {
    /// Lowercase English name, as used in the generated document.
    pub const fn name(self) -> &'static str {
        match self {
            ParamKind::Int => "int",
            ParamKind::Float => "float",
            ParamKind::Bool => "bool",
            ParamKind::Text => "text",
        }
    }
}

/// A parameter value; copyable and usable in `const` items.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamValue {
    /// An integer value.
    Int(i64),
    /// A float value.
    Float(f64),
    /// A boolean value.
    Bool(bool),
    /// A static text value.
    Text(&'static str),
}

impl ParamValue {
    /// The kind of this value.
    pub const fn kind(&self) -> ParamKind {
        match self {
            ParamValue::Int(_) => ParamKind::Int,
            ParamValue::Float(_) => ParamKind::Float,
            ParamValue::Bool(_) => ParamKind::Bool,
            ParamValue::Text(_) => ParamKind::Text,
        }
    }

    /// The value as a number (int or float), for range checks; `None` for bool and text.
    pub const fn as_f64(&self) -> Option<f64> {
        match *self {
            ParamValue::Int(v) => Some(v as f64),
            ParamValue::Float(v) => Some(v),
            ParamValue::Bool(_) | ParamValue::Text(_) => None,
        }
    }
}

/// The definition of one parameter (one registry row).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamDef {
    /// Dotted lowercase English key, e.g. `scorecard.min_n`.
    pub key: &'static str,
    /// Value type.
    pub kind: ParamKind,
    /// Unit (bars, ms, count, ratio).
    pub unit: &'static str,
    /// Smallest allowed value (inclusive), when bounded.
    pub min: Option<f64>,
    /// Largest allowed value (inclusive), when bounded.
    pub max: Option<f64>,
    /// System default.
    pub default: ParamValue,
    /// Levels at which the value may be set.
    pub scopes: &'static [ParamScope],
    /// Who may change the value.
    pub changed_by: ChangedBy,
    /// Whether the value is a sellable entitlement (quota, limit).
    pub entitlement: bool,
    /// Short English text shown to the person.
    pub description: &'static str,
    /// Release (round) the parameter was added in.
    pub since: &'static str,
}

impl ParamDef {
    /// Whether the definition allows values to be set at `scope`.
    pub fn allows(&self, scope: ParamScope) -> bool {
        self.scopes.contains(&scope)
    }
}

/// Scopes in the order they win: object > user > plan > system (`docs/ENGINEERING.md` §6).
///
/// [`ParamScope::Instrument`] is not in this order: §6 does not place it yet, so
/// [`resolve`] ignores instrument values.
pub const RESOLUTION_ORDER: [ParamScope; 4] = [
    ParamScope::Object,
    ParamScope::User,
    ParamScope::Plan,
    ParamScope::System,
];

/// The effective value of a parameter and the scope it came from ("why 3? → system").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolved {
    /// The effective value.
    pub value: ParamValue,
    /// The scope that supplied it.
    pub source: ParamScope,
}

/// Resolve the effective value of `def` from the values set at each scope.
///
/// Walks [`RESOLUTION_ORDER`]; the first scope that has a value wins. A value is taken
/// only if `def` declares its scope and its kind matches `def.kind`; within one scope the
/// first entry wins. Instrument values are ignored (not in the documented order). With no
/// usable value the result is the definition's default with source [`ParamScope::System`].
pub fn resolve(def: &ParamDef, values: &[(ParamScope, ParamValue)]) -> Resolved {
    for scope in RESOLUTION_ORDER {
        if !def.allows(scope) {
            continue;
        }
        let found = values
            .iter()
            .find(|(s, v)| *s == scope && v.kind() == def.kind);
        if let Some(&(source, value)) = found {
            return Resolved { value, source };
        }
    }
    Resolved {
        value: def.default,
        source: ParamScope::System,
    }
}

#[cfg(test)]
mod tests;
