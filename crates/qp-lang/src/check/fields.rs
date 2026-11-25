//! Reading one JSON object strictly: each field is asked for by name, and whatever was
//! never asked for is an unknown field (`E_UNKNOWN_FIELD`, with the known names as
//! `did_you_mean`). This is how the checker reads raw JSON instead of a `serde` value,
//! so that every bad field of a program is reported at once (PR §11.1).

use serde_json::{Map, Value};

use crate::diag::{join, Diags};
use crate::message::Msg;
use crate::names::did_you_mean;
use crate::reason::Reason;

/// A strict reader over one object.
pub(super) struct Fields<'a> {
    obj: &'a Map<String, Value>,
    /// JSON pointer of the object.
    pub(super) at: String,
    known: Vec<&'static str>,
}

impl<'a> Fields<'a> {
    /// A reader over `v` at `at`; `None` (and an `E_TYPE`) when `v` is not an object.
    pub(super) fn new(v: &'a Value, at: &str, what: &str, d: &mut Diags) -> Option<Fields<'a>> {
        if let Value::Object(obj) = v {
            Some(Fields {
                obj,
                at: at.to_owned(),
                known: Vec::new(),
            })
        } else {
            d.at(Reason::Type, at, &Msg::NotObject(what.to_owned()));
            None
        }
    }

    /// Pointer of field `name`.
    pub(super) fn ptr(&self, name: &str) -> String {
        join(&self.at, name)
    }

    /// An optional field (marks the name as known).
    pub(super) fn opt(&mut self, name: &'static str) -> Option<&'a Value> {
        if !self.known.contains(&name) {
            self.known.push(name);
        }
        self.obj.get(name)
    }

    /// A required field: `E_MISSING_ARG` when absent.
    pub(super) fn req(&mut self, name: &'static str, d: &mut Diags) -> Option<&'a Value> {
        let v = self.opt(name);
        if v.is_none() {
            d.at(Reason::MissingArg, &self.at, &Msg::Missing(name.to_owned()))
                .expected([name]);
        }
        v
    }

    /// A required string field (`E_MISSING_ARG`, `E_TYPE`).
    pub(super) fn req_str(&mut self, name: &'static str, d: &mut Diags) -> Option<&'a str> {
        match self.req(name, d)? {
            Value::String(s) => Some(s),
            _ => {
                d.at(
                    Reason::Type,
                    &self.ptr(name),
                    &Msg::NotString(name.to_owned()),
                );
                None
            }
        }
    }

    /// An optional string field (`E_TYPE` when present and not a string).
    pub(super) fn opt_str(&mut self, name: &'static str, d: &mut Diags) -> Option<&'a str> {
        match self.opt(name)? {
            Value::String(s) => Some(s),
            _ => {
                d.at(
                    Reason::Type,
                    &self.ptr(name),
                    &Msg::NotString(name.to_owned()),
                );
                None
            }
        }
    }

    /// Whether the object has `name` (without marking it known).
    pub(super) fn has(&self, name: &str) -> bool {
        self.obj.contains_key(name)
    }

    /// Reports every field never asked for; `skip` names are handled by the caller.
    pub(super) fn finish(self, d: &mut Diags, skip: &[&str]) {
        for key in self.obj.keys() {
            if self.known.contains(&key.as_str()) || skip.contains(&key.as_str()) {
                continue;
            }
            let dym = did_you_mean(key, self.known.iter().copied());
            d.at(
                Reason::UnknownField,
                &join(&self.at, key),
                &Msg::UnknownField(key.clone()),
            )
            .expected(self.known.iter().copied())
            .dym(dym);
        }
    }
}
