//! Wire forms of composite fields that accept selectors inside (schema only).

use super::*;

// Input shapes of composite fields that accept selectors inside. They describe the
// *wire* form of these fields (schema only, through `#[schemars(with = ...)]`); the
// authority resolves the selectors and then parses the stored types (`Target`,
// `grammar::Layer`, `Condition`, ...). A test keeps their field names equal to the
// stored types' (unit test `wire_forms_mirror_stored_types` below).

/// Wire form of [`Target`]: every id field also takes a selector.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetIn {
    /// View id or selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<ViewRef>,
    /// Panel id or selector (in `view`, or in the op's view).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<PanelRef>,
    /// Layer id or selector (in `view`, the op's view, or any view).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<LayerRef>,
    /// Series id or selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<SeriesRef>,
    /// Output or field of the series, e.g. `"v"`, `"signal"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Item id or selector (`{"finding":{...}}`, `{"fact":{...}}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<ItemRef>,
    /// First bar index (0-based) of the region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_bar: Option<u64>,
    /// Last bar index (inclusive) of the region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_bar: Option<u64>,
}

/// Wire form of [`crate::grammar::Emphasis`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmphasisIn {
    /// What is emphasised.
    pub target: TargetIn,
    /// How.
    pub kind: crate::grammar::EmphasisKind,
}

/// Wire form of [`NarrativeStep`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NarrativeStepIn {
    /// Start of the step, milliseconds from the start of the narration.
    pub at_ms: u32,
    /// What to show.
    pub target: TargetIn,
    /// What to say.
    pub say: String,
}

/// Wire form of [`AttentionDirective`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttentionDirectiveIn {
    /// Id; assigned if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Principal who should look.
    pub to: String,
    /// Steps, in the order to show them.
    pub steps: Vec<AttentionStepIn>,
    /// Filled in by the authority; ignored if sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Filled in by the authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<TimestampMs>,
}

/// Wire form of [`AttentionStep`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttentionStepIn {
    /// What to look at.
    pub target: TargetIn,
    /// Optional short caption.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub say: Option<String>,
}

/// Wire form of [`Condition`]: series fields also take selectors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConditionIn {
    /// Last value of a series compared against a constant.
    Threshold {
        /// Series id or selector.
        series: SeriesRef,
        /// Comparison.
        op: crate::types::CompareOp,
        /// Constant to compare against.
        value: f64,
    },
    /// One series compared against another.
    Compare {
        /// Left series id or selector.
        left: SeriesRef,
        /// Comparison.
        op: crate::types::CompareOp,
        /// Right series id or selector.
        right: SeriesRef,
    },
    /// Free text; evaluated by a person.
    Text {
        /// Condition statement.
        text: String,
    },
}

/// Wire form of [`crate::grammar::DataRef`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataRefIn {
    /// Series id or selector; the layer's own series when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<SeriesRef>,
    /// Field name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

/// Wire form of [`crate::grammar::Position`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum PositionIn {
    /// Numeric constant.
    Number(f64),
    /// Field name, or a numeric constant written as a string.
    Name(String),
    /// Field of a (possibly other) series.
    Ref(DataRefIn),
    /// Panel-relative layout position (R3).
    Panel(crate::grammar::PanelPos),
}

/// Wire form of [`crate::grammar::Operand`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum OperandIn {
    /// Numeric constant.
    Number(f64),
    /// A field of the layer's data or of another series.
    Ref(DataRefIn),
}

/// Wire form of [`crate::grammar::Predicate`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PredicateIn {
    /// Left-hand field of the layer's data.
    pub field: String,
    /// Comparison.
    pub op: crate::grammar::Cmp,
    /// Right-hand side.
    pub than: OperandIn,
}

/// Wire form of [`crate::grammar::ColorWhen`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ColorWhenIn {
    /// The condition.
    pub when: PredicateIn,
    /// Colour where it holds.
    pub then: String,
    /// Colour where it does not.
    #[serde(rename = "else")]
    pub otherwise: String,
}

/// Wire form of [`crate::grammar::ColorSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ColorSpecIn {
    /// One colour.
    Constant(String),
    /// Colour by condition.
    When(ColorWhenIn),
    /// Colour by value steps.
    Steps(crate::grammar::ColorSteps),
}

/// Wire form of [`crate::grammar::Encoding`]: series references inside channels also
/// take selectors.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EncodingIn {
    /// Horizontal position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<PositionIn>,
    /// Second horizontal position (R3: last bar of an extent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x2: Option<PositionIn>,
    /// Vertical position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<PositionIn>,
    /// Second vertical position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y2: Option<PositionIn>,
    /// Colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorSpecIn>,
    /// Opacity `0..=1`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
    /// Stroke width in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    /// Size in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<crate::grammar::SizeSpec>,
    /// Text field of a `text` mark.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Wire form of [`crate::grammar::Layer`]: `series`, `fact`, `finding`, `panel` and
/// series references in the encoding also take selectors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayerIn {
    /// Stable id, unique within the view. Assigned when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Visual mark.
    pub mark: crate::grammar::Mark,
    /// Data: a series id or selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<SeriesRef>,
    /// Data: a fact id or selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact: Option<FactRef>,
    /// Data: a finding id or selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finding: Option<FindingRef>,
    /// Panel id or selector (in the op's view).
    #[serde(default)]
    pub panel: PanelRef,
    /// Encoding channels.
    #[serde(default)]
    pub encoding: EncodingIn,
    /// Origins of the layer's numeric constants (JSON pointer inside the layer).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub origin: BTreeMap<String, crate::grammar::Origin>,
    /// Legend text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[cfg(test)]
mod selector_tests {
    use super::*;

    /// Property names of a type's generated schema (object variants merged).
    fn props<T: JsonSchema>() -> Vec<String> {
        fn collect(v: &serde_json::Value, out: &mut Vec<String>) {
            if let Some(p) = v.get("properties").and_then(serde_json::Value::as_object) {
                out.extend(p.keys().cloned());
            }
            for k in ["oneOf", "anyOf"] {
                if let Some(bs) = v.get(k).and_then(serde_json::Value::as_array) {
                    bs.iter().for_each(|b| collect(b, out));
                }
            }
        }
        let mut out = Vec::new();
        collect(&schemars::schema_for!(T).to_value(), &mut out);
        out.sort();
        out.dedup();
        out
    }

    #[test]
    fn wire_forms_mirror_stored_types() {
        use crate::grammar as g;
        assert_eq!(props::<TargetIn>(), props::<Target>());
        assert_eq!(props::<EmphasisIn>(), props::<g::Emphasis>());
        assert_eq!(props::<NarrativeStepIn>(), props::<NarrativeStep>());
        assert_eq!(
            props::<AttentionDirectiveIn>(),
            props::<AttentionDirective>()
        );
        assert_eq!(props::<AttentionStepIn>(), props::<AttentionStep>());
        assert_eq!(props::<ConditionIn>(), props::<Condition>());
        assert_eq!(props::<LayerIn>(), props::<g::Layer>());
        assert_eq!(props::<EncodingIn>(), props::<g::Encoding>());
        assert_eq!(props::<DataRefIn>(), props::<g::DataRef>());
        assert_eq!(props::<PredicateIn>(), props::<g::Predicate>());
        assert_eq!(props::<ColorWhenIn>(), props::<g::ColorWhen>());
    }

    #[test]
    fn id_fields_accept_selectors_in_the_schema() {
        let s = schemars::schema_for!(ClientMsg).to_value();
        let text = s.to_string();
        for def in [
            "SeriesSel",
            "ViewSel",
            "PanelSel",
            "LayerSel",
            "FactSel",
            "FindingSel",
            "ProposalSel",
            "ScaleSel",
            "ItemSel",
            "LayerIn",
            "TargetIn",
            "ConditionIn",
        ] {
            assert!(
                s.pointer(&format!("/$defs/{def}")).is_some(),
                "{def} missing from the ClientMsg schema"
            );
        }
        assert!(text.contains("\"deprecated\":true"));
        // Stored views keep plain ids. (The stored `Op`s of proposals and log requests
        // share the input schema; the authority only ever stores them resolved.)
        let d = schemars::schema_for!(View).to_value();
        assert!(d.pointer("/$defs/SeriesSel").is_none());
        assert!(d.pointer("/$defs/LayerIn").is_none());
        assert!(d.pointer("/$defs/TargetIn").is_none());
    }

    #[test]
    fn selectors_reject_unknown_keys() {
        let r: Result<SeriesSel, _> = serde_json::from_str(r#"{"stdy":"rsi"}"#);
        assert!(r.is_err());
        let r: Result<ItemSel, _> = serde_json::from_str(r#"{"finding":{"kind":"x"}}"#);
        assert!(r.is_ok());
    }
}
