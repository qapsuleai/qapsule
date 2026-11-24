//! The registry: registered subsystems and their dispatch tables, in registration order.

use qp_ops::expand::{expand_op, expand_ops, expand_ops_with, Ctx, Expanded};
use qp_plane::values::{eval_condition, eval_on, values, Cache, Computed};
use qp_schema::protocol::{ChangeOp, ClaimRef, Gesture, Op, ProtocolError, Query, Topic};
use qp_schema::{Condition, Document};
use serde::Serialize;

use crate::handlers::{
    ClaimKind, ConditionKind, InputHandler, OpHandler, QueryHandler, TopicHandler, ValueSource,
};
use crate::{Moment, Phase, SessionCtx, Subsystem};

/// The `kind` tag of a wire value.
fn kind_of<T: Serialize>(v: &T) -> Option<String> {
    let v = serde_json::to_value(v).ok()?;
    v.get("kind")?.as_str().map(str::to_owned)
}

/// A dispatch table: (wire tag, handler) in registration order, each tag once. A table
/// holds a handful of kinds: a vector keeps the WASM build small (one `BTreeMap` per
/// handler type would not; RF1 S3.1).
type Table<V> = Vec<(&'static str, V)>;

/// Adds `v` under `k`, refusing a kind another subsystem owns.
fn claim<V>(t: &mut Table<V>, k: &'static str, v: V, who: &str) -> Result<(), String> {
    if t.iter().any(|(n, _)| *n == k) {
        return Err(format!("`{k}` is registered twice (second: {who})"));
    }
    t.push((k, v));
    Ok(())
}

/// The handler of a wire value's kind `kind_of(v)`: the one registered under it, else the
/// one registered under [`ANY`]. A table holding only [`ANY`] answers without reading the
/// kind (no serialization on the query and input path).
fn find_kind<T: Serialize, V: Copy>(t: &Table<V>, v: &T) -> Option<V> {
    if t.iter().all(|(n, _)| *n == ANY) {
        return t.first().map(|(_, h)| *h);
    }
    kind_of(v)
        .and_then(|k| find(t, &k))
        .or_else(|| find(t, ANY))
}

/// The wire tag a query or input handler registers under to answer every kind no other
/// subsystem registered (the reading subsystem, `qp-read`: RF1 S3.2). A handler registered
/// under a kind comes first.
pub const ANY: &str = "*";

/// The handler under `k`.
fn find<V: Copy>(t: &Table<V>, k: &str) -> Option<V> {
    t.iter().find(|(n, _)| *n == k).map(|(_, v)| *v)
}

/// The subsystems an authority calls, in registration order (deterministic), with their
/// op, query, input, topic, condition, value-source and claim tables. Empty: the
/// authority behaves exactly as without it (every lookup misses, no phase is called).
#[derive(Default, Clone)]
pub struct Registry {
    subs: Vec<&'static dyn Subsystem>,
    ops: Table<&'static dyn OpHandler>,
    queries: Table<&'static dyn QueryHandler>,
    inputs: Table<&'static dyn InputHandler>,
    topics: Table<&'static dyn TopicHandler>,
    conditions: Table<&'static dyn ConditionKind>,
    sources: Vec<&'static dyn ValueSource>,
    claims: Table<&'static dyn ClaimKind>,
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.subs.iter().map(|s| s.name()).collect();
        f.debug_struct("Registry")
            .field("subsystems", &names)
            .finish()
    }
}

impl Registry {
    /// Registers `sub` after the ones before it.
    ///
    /// # Errors
    /// One of its kinds is already registered (nothing is registered then).
    pub fn add(&mut self, sub: &'static dyn Subsystem) -> Result<(), String> {
        let mut next = self.clone();
        let who = sub.name();
        for h in sub.ops() {
            claim(&mut next.ops, h.op(), *h, who)?;
        }
        for h in sub.queries() {
            claim(&mut next.queries, h.kind(), *h, who)?;
        }
        for h in sub.inputs() {
            claim(&mut next.inputs, h.kind(), *h, who)?;
        }
        for h in sub.topics() {
            claim(&mut next.topics, h.kind(), *h, who)?;
        }
        for h in sub.conditions() {
            claim(&mut next.conditions, h.kind(), *h, who)?;
        }
        for h in sub.claims() {
            claim(&mut next.claims, h.kind(), *h, who)?;
        }
        next.sources.extend(sub.value_sources());
        next.subs.push(sub);
        *self = next;
        Ok(())
    }

    /// Whether nothing is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.subs.is_empty()
    }

    /// Whether a registered subsystem is called at `phase`.
    #[must_use]
    pub fn wants(&self, phase: Phase) -> bool {
        self.subs.iter().any(|s| s.phases().contains(&phase))
    }

    /// The registered subsystems, in order.
    #[must_use]
    pub fn subsystems(&self) -> &[&'static dyn Subsystem] {
        &self.subs
    }

    /// The collect step of `phase`: every subsystem that named it, in order.
    pub fn collect(&self, phase: Phase, at: &Moment<'_>, ctx: &mut SessionCtx<'_>) {
        for s in self.subs.iter().filter(|s| s.phases().contains(&phase)) {
            s.collect(phase, at, ctx);
        }
    }

    /// [`Phase::Check`]: the first refusal, in order.
    ///
    /// # Errors
    /// A subsystem refused the command.
    pub fn check(
        &self,
        conn: &str,
        ops: &[Op],
        ctx: &mut SessionCtx<'_>,
    ) -> Result<(), ProtocolError> {
        for s in self
            .subs
            .iter()
            .filter(|s| s.phases().contains(&Phase::Check))
        {
            s.check(conn, ops, ctx)?;
        }
        Ok(())
    }

    /// The handler of `op`'s kind, if a subsystem owns it.
    #[must_use]
    pub fn op(&self, op: &Op) -> Option<&'static dyn OpHandler> {
        if self.ops.is_empty() {
            return None;
        }
        self.ops.iter().map(|(_, h)| *h).find(|h| h.owns(op))
    }

    /// The handler of `q`'s kind.
    #[must_use]
    pub fn query(&self, q: &Query) -> Option<&'static dyn QueryHandler> {
        find_kind(&self.queries, q)
    }

    /// The handler of `g`'s kind.
    #[must_use]
    pub fn input(&self, g: &Gesture) -> Option<&'static dyn InputHandler> {
        find_kind(&self.inputs, g)
    }

    /// The handler of `t`'s kind.
    #[must_use]
    pub fn topic(&self, t: &Topic) -> Option<&'static dyn TopicHandler> {
        if self.topics.is_empty() {
            return None;
        }
        find(&self.topics, &kind_of(t)?)
    }

    /// The kind of claim `c`, if a subsystem owns it.
    #[must_use]
    pub fn claim(&self, c: &ClaimRef) -> Option<&'static dyn ClaimKind> {
        if self.claims.is_empty() {
            return None;
        }
        find(&self.claims, &kind_of(c)?)
    }

    /// [`Phase::Expand`] for one op: its handler's expansion, else the built-in one.
    ///
    /// # Errors
    /// As the expansion.
    pub fn expand_op(
        &self,
        doc: &Document,
        op: &Op,
        p: &str,
        ctx: &mut Ctx<'_>,
    ) -> Result<Vec<ChangeOp>, ProtocolError> {
        match self.op(op) {
            Some(h) => h.expand(doc, op, p, ctx),
            None => expand_op(doc, op, p, ctx),
        }
    }

    /// [`Phase::Expand`] for a command: `qp_ops::expand::expand_ops` with each op owned
    /// by a subsystem expanded by its handler (same order, same errors otherwise).
    ///
    /// # Errors
    /// As `qp_ops::expand::expand_ops`.
    pub fn expand_ops(
        &self,
        doc: &Document,
        ops: &[Op],
        base: &str,
        ctx: &mut Ctx<'_>,
    ) -> Result<Expanded, ProtocolError> {
        if self.ops.is_empty() {
            return expand_ops(doc, ops, base, ctx);
        }
        expand_ops_with(doc, ops, base, ctx, |d, op, p, ctx| {
            self.expand_op(d, op, p, ctx)
        })
    }

    /// Whether a subsystem registered a condition kind or a value source: the live path
    /// then asks [`Registry::eval_condition`] every time (no memo of the built-in
    /// evaluation; audit-s30 B1).
    #[must_use]
    pub fn evaluates_conditions(&self) -> bool {
        !(self.conditions.is_empty() && self.sources.is_empty())
    }

    /// Whether condition `c` holds now ([`Registry::eval_condition`]).
    pub fn holds(&self, rd: &Document, cache: &mut Cache, c: &Condition) -> bool {
        self.eval_condition(rd, cache, c).is_some_and(|(b, _, _)| b)
    }

    /// Evaluates condition `c` on the live path: a registered condition kind first, then
    /// value sources for the series they own, else the data plane's evaluation.
    pub fn eval_condition(
        &self,
        rd: &Document,
        cache: &mut Cache,
        c: &Condition,
    ) -> Option<(bool, u64, i64)> {
        if self.conditions.is_empty() && self.sources.is_empty() {
            return eval_condition(rd, cache, c);
        }
        if let Some(k) = kind_of(c).and_then(|k| find(&self.conditions, &k)) {
            return k.eval(rd, cache, c);
        }
        let owned = |id: &str| self.sources.iter().find(|s| s.owns(id)).copied();
        let mut side = |id: &str| -> Option<Computed> {
            match owned(id) {
                Some(s) => s.values(rd, id),
                None => values(rd, cache, id).ok().map(|v| (*v).clone()),
            }
        };
        match c {
            Condition::Threshold { series, op, value } if owned(series).is_some() => {
                eval_on(*op, &side(series)?, None, *value)
            }
            Condition::Compare { left, op, right } if owned(left).or(owned(right)).is_some() => {
                let l = side(left)?;
                eval_on(*op, &l, Some(&side(right)?), 0.0)
            }
            _ => eval_condition(rd, cache, c),
        }
    }
}
