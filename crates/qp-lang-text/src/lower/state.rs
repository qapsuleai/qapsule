//! The lowering state: text names, the IR parts built so far, the source map, errors.

use std::collections::BTreeMap;

use qp_schema::program::PROGRAM_VERSION;
use qp_schema::protocol::Span;
use serde_json::{Map, Value};

use crate::diag::Diag;
use crate::map::SourceMap;

/// What a text name stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Def {
    /// A parameter.
    Param,
    /// A node reference (`id` or `id.output`).
    Node(String),
    /// Defined by a statement that failed: known, but nothing to refer to.
    Poisoned,
}

/// The lowering state.
pub(super) struct Lower {
    pub(super) names: BTreeMap<String, Def>,
    /// Every name the file defines, with its first line (for "defined later" hints).
    pub(super) lines: BTreeMap<String, u32>,
    pub(super) params: Vec<Value>,
    pub(super) nodes: Vec<Value>,
    pub(super) outputs: Vec<Value>,
    pub(super) anon: u32,
    pub(super) map: SourceMap,
    pub(super) diags: Vec<Diag>,
}

impl Lower {
    /// Adds a node; returns its id (`root` or the next `_k`).
    pub(super) fn node(
        &mut self,
        root: Option<&str>,
        op: &str,
        fields: Vec<(&'static str, Value, Option<Span>)>,
        span: Span,
    ) -> String {
        let id = match root {
            Some(r) => r.to_owned(),
            None => {
                self.anon += 1;
                format!("_{}", self.anon)
            }
        };
        let i = self.nodes.len();
        let mut m = Map::new();
        m.insert("id".into(), Value::String(id.clone()));
        m.insert("op".into(), Value::String(op.to_owned()));
        self.map.add(format!("/nodes/{i}"), span);
        for (k, v, sp) in fields {
            if let Some(sp) = sp {
                self.map.add(format!("/nodes/{i}/{k}"), sp);
            }
            m.insert(k.into(), v);
        }
        self.nodes.push(Value::Object(m));
        id
    }

    /// The program object.
    pub(super) fn program(&mut self) -> Value {
        let mut m = Map::new();
        m.insert("version".into(), Value::String(PROGRAM_VERSION.to_owned()));
        if !self.params.is_empty() {
            m.insert(
                "params".into(),
                Value::Array(std::mem::take(&mut self.params)),
            );
        }
        m.insert(
            "nodes".into(),
            Value::Array(std::mem::take(&mut self.nodes)),
        );
        m.insert(
            "outputs".into(),
            Value::Array(std::mem::take(&mut self.outputs)),
        );
        Value::Object(m)
    }
}
