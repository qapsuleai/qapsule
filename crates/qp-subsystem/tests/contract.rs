//! The contract's own pieces: slots, per-world swap, outbox, registry tables.

use qp_ops::expand::Ctx;
use qp_schema::protocol::{ChangeOp, Op, ProtocolError, Query, QueryResult};
use qp_schema::Document;
use qp_subsystem::{
    OpHandler, Outbox, PerWorld, Phase, QueryHandler, Registry, SessionCtx, Slots, Subsystem, ANY,
};

#[derive(Debug, PartialEq)]
struct Count(u32);

#[test]
fn slots_keep_state_by_type_world_and_connection() {
    let mut s = Slots::default();
    assert!(s.is_empty());
    s.put(Count(1));
    if let Some(c) = s.get_mut::<Count>() {
        c.0 += 1;
    }
    assert_eq!(s.get::<Count>(), Some(&Count(2)));
    assert_eq!(s.get::<String>(), None);
    s.put_world(PerWorld::new(Count(10), Count(20)));
    assert_eq!(s.world::<Count>().map(|w| w.get().0), Some(10));
    s.swap_world();
    assert_eq!(s.world::<Count>().map(|w| w.get().0), Some(20));
    if let Some(w) = s.world_mut::<Count>() {
        w.get_mut().0 = 21;
    }
    s.swap_world();
    assert_eq!(s.world::<Count>().map(|w| w.get().0), Some(10));
    s.swap_world();
    assert_eq!(s.world::<Count>().map(|w| w.get().0), Some(21));
    s.put_conn("c1", Count(5));
    assert_eq!(s.conn_mut::<Count>("c1").map(|c| c.0), Some(5));
    assert_eq!(s.conn_mut::<Count>("c2").map(|c| c.0), None);
    s.drop_conn("c1");
    assert_eq!(s.conn_mut::<Count>("c1").map(|c| c.0), None);
}

#[test]
fn outbox_drains_in_order_and_keeps_the_earliest_wake() {
    let mut o = Outbox::default();
    assert!(o.is_empty());
    o.entry(Vec::new(), "a".into());
    o.entry(Vec::new(), "b".into());
    o.wake_at(30);
    o.wake_at(20);
    o.wake_at(25);
    let (entries, notes, wake) = o.drain();
    let names: Vec<&str> = entries.iter().map(|e| e.summary.as_str()).collect();
    assert_eq!(names, ["a", "b"]);
    assert!(notes.is_empty());
    assert_eq!(wake, Some(20));
    assert!(o.is_empty());
}

struct Undo;
impl OpHandler for Undo {
    fn op(&self) -> &'static str {
        "undo"
    }
    fn expand(
        &self,
        _: &Document,
        _: &Op,
        _: &str,
        _: &mut Ctx<'_>,
    ) -> Result<Vec<ChangeOp>, ProtocolError> {
        Ok(Vec::new())
    }
}

struct A;
impl Subsystem for A {
    fn name(&self) -> &'static str {
        "a"
    }
    fn phases(&self) -> &'static [Phase] {
        &[Phase::AfterCommit]
    }
    fn ops(&self) -> &'static [&'static dyn OpHandler] {
        &[&Undo]
    }
}

struct B;
impl Subsystem for B {
    fn name(&self) -> &'static str {
        "b"
    }
    fn ops(&self) -> &'static [&'static dyn OpHandler] {
        &[&Undo]
    }
}

#[test]
fn registry_tables_and_duplicates() {
    let mut r = Registry::default();
    assert!(r.is_empty());
    let undo = Op::Undo { seq: None };
    assert!(r.op(&undo).is_none());
    assert!(r.query(&Query::Describe {}).is_none());
    assert!(!r.wants(Phase::AfterCommit));
    assert_eq!(r.add(&A), Ok(()));
    assert!(r.wants(Phase::AfterCommit));
    assert!(!r.wants(Phase::AfterData));
    assert_eq!(r.op(&undo).map(|h| h.op()), Some("undo"));
    assert!(r.add(&B).is_err_and(|e| e.contains("`undo`")));
    let names: Vec<&str> = r.subsystems().iter().map(|s| s.name()).collect();
    assert_eq!(names, ["a"], "a refused registration leaves nothing behind");
}

/// A query handler answering under `kind` (RF1 S3.2).
struct Q(&'static str);

impl QueryHandler for Q {
    fn kind(&self) -> &'static str {
        self.0
    }
    fn answer(
        &self,
        _conn: &str,
        _q: &Query,
        _ctx: &mut SessionCtx<'_>,
    ) -> Result<QueryResult, ProtocolError> {
        unreachable!("not called")
    }
}

static ALL: Q = Q(ANY);
static DESCRIBE: Q = Q("describe");

struct Reads;
impl Subsystem for Reads {
    fn name(&self) -> &'static str {
        "reads"
    }
    fn queries(&self) -> &'static [&'static dyn QueryHandler] {
        static L: &[&dyn QueryHandler] = &[&ALL];
        L
    }
}

struct Describes;
impl Subsystem for Describes {
    fn name(&self) -> &'static str {
        "describes"
    }
    fn queries(&self) -> &'static [&'static dyn QueryHandler] {
        static L: &[&dyn QueryHandler] = &[&DESCRIBE];
        L
    }
}

/// (RF1 S3.2) A handler under `ANY` answers every kind no other subsystem registered; a
/// handler under a kind comes first, whichever was registered first; `ANY` is claimed once.
#[test]
fn any_answers_every_kind_no_other_handler_took() {
    let mut r = Registry::default();
    assert_eq!(r.add(&Reads), Ok(()));
    let ledger = Query::Ledger {
        key: None,
        meaning: None,
        detector: None,
        direction: None,
        by_kind: false,
    };
    assert_eq!(r.query(&Query::Describe {}).map(|h| h.kind()), Some(ANY));
    assert_eq!(r.query(&ledger).map(|h| h.kind()), Some(ANY));
    assert_eq!(r.add(&Describes), Ok(()));
    assert_eq!(
        r.query(&Query::Describe {}).map(|h| h.kind()),
        Some("describe")
    );
    assert_eq!(r.query(&ledger).map(|h| h.kind()), Some(ANY));
    assert!(r.add(&Reads).is_err_and(|e| e.contains("`*`")));
}
