//! `Tally`: a subsystem that lives only in this test (RF1 E, `rounds/RF1/PLAN.md` §7b).
//!
//! - Owns one existing op kind (`set_view_range`) through an [`OpHandler`]; its expansion is
//!   the built-in one (`qp_ops::expand::expand_op`) followed by its mark (a narrative
//!   [`MARK`]): the mark is in the log only if the registry called Tally's handler.
//! - [`Phase::AfterCommit`]: counts the marks in the entry (not the request), and writes the count
//!   into the document with an engine entry (`Outbox::entry`): a narrative `tally` whose
//!   `based_on` lists the counted seqs.
//! - [`Phase::AfterCommit`] of its own entry: pushes it on its topic (`proposals`, which it
//!   owns through a [`TopicHandler`]) to every subscriber, as a `change` event.
//! - Answers the `describe` query ([`QueryHandler`]) with its count.

use qp_ops::expand::Ctx;
use qp_schema::protocol::{
    ChangeOp, Collection, Event, Op, OutboundLine, ProtocolError, Query, QueryResult, ServerMsg,
    Topic,
};
use qp_schema::Document;
use qp_subsystem::{Moment, OpHandler, Phase, QueryHandler, SessionCtx, Subsystem, TopicHandler};
use serde_json::json;

/// The id of the narrative Tally's expansion adds (its mark).
pub const MARK: &str = "tally-op";

/// Whether `c` is Tally's mark.
pub fn is_mark(c: &ChangeOp) -> bool {
    matches!(c, ChangeOp::PutNarrative { narrative, .. } if narrative.id == MARK)
}

/// The summary prefix of Tally's engine entries.
pub const SUMMARY: &str = "tally ";

/// Tally's state (its slot).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TallyState {
    /// Entries that carried `set_view_range`, by seq.
    pub seen: Vec<u64>,
    /// Ops counted.
    pub count: u64,
    /// Subscribers of its topic: (connection, subscription).
    pub subs: Vec<(String, String)>,
}

/// The subsystem.
pub struct Tally;

/// As registered.
pub static TALLY: Tally = Tally;

struct SetViewRange;
static SET_VIEW_RANGE: SetViewRange = SetViewRange;

impl OpHandler for SetViewRange {
    fn op(&self) -> &'static str {
        "set_view_range"
    }
    fn owns(&self, op: &Op) -> bool {
        matches!(op, Op::SetViewRange { .. })
    }
    fn expand(
        &self,
        doc: &Document,
        op: &Op,
        at: &str,
        ctx: &mut Ctx<'_>,
    ) -> Result<Vec<ChangeOp>, ProtocolError> {
        let mut changes = qp_ops::expand::expand_op(doc, op, at, ctx)?;
        changes.push(narrative(MARK, &[]));
        Ok(changes)
    }
}

struct Describe;
static DESCRIBE: Describe = Describe;

impl QueryHandler for Describe {
    fn kind(&self) -> &'static str {
        "describe"
    }
    fn answer(
        &self,
        _conn: &str,
        _q: &Query,
        ctx: &mut SessionCtx<'_>,
    ) -> Result<QueryResult, ProtocolError> {
        let count = ctx.slots.get::<TallyState>().map_or(0, |t| t.count);
        Ok(QueryResult::Item {
            collection: Collection::Narratives,
            item: json!({ "tally": count }),
        })
    }
}

struct Feed;
static FEED: Feed = Feed;

impl TopicHandler for Feed {
    fn kind(&self) -> &'static str {
        "proposals"
    }
    fn subscribe(
        &self,
        conn: &str,
        sub: &str,
        _t: &Topic,
        ctx: &mut SessionCtx<'_>,
    ) -> Result<(), ProtocolError> {
        if let Some(t) = ctx.slots.get_mut::<TallyState>() {
            t.subs.push((conn.to_owned(), sub.to_owned()));
        }
        Ok(())
    }
}

/// The narrative that carries the count.
fn tally_item(seen: &[u64]) -> ChangeOp {
    narrative("tally", seen)
}

/// A narrative `id` of Tally's whose `based_on` lists `seen`.
fn narrative(id: &str, seen: &[u64]) -> ChangeOp {
    let based_on: Vec<String> = seen.iter().map(|s| format!("seq {s}")).collect();
    let narrative = serde_json::from_value(json!({
        "id": id,
        "about": "main",
        "steps": [],
        "provenance": {
            "author": {"kind": "engine", "name": "tally", "version": "0"},
            "at": 0,
            "based_on": based_on,
        },
    }))
    .expect("narrative");
    ChangeOp::PutNarrative {
        narrative,
        index: None,
    }
}

static OPS: &[&dyn OpHandler] = &[&SET_VIEW_RANGE];
static QUERIES: &[&dyn QueryHandler] = &[&DESCRIBE];
static TOPICS: &[&dyn TopicHandler] = &[&FEED];

impl Subsystem for Tally {
    fn name(&self) -> &'static str {
        "tally"
    }
    fn init(&self, ctx: &mut SessionCtx<'_>) {
        ctx.slots.put(TallyState::default());
    }
    fn phases(&self) -> &'static [Phase] {
        &[Phase::AfterCommit]
    }
    fn ops(&self) -> &'static [&'static dyn OpHandler] {
        OPS
    }
    fn queries(&self) -> &'static [&'static dyn QueryHandler] {
        QUERIES
    }
    fn topics(&self) -> &'static [&'static dyn TopicHandler] {
        TOPICS
    }
    fn collect(&self, _phase: Phase, at: &Moment<'_>, ctx: &mut SessionCtx<'_>) {
        let Moment::Entry { seq } = *at else {
            return;
        };
        let Some(entry) = ctx.view.log().get(seq as usize - 1).cloned() else {
            return;
        };
        let Some(t) = ctx.slots.get_mut::<TallyState>() else {
            return;
        };
        if entry.summary.starts_with(SUMMARY) {
            for (conn, sub) in &t.subs {
                ctx.out.notify(OutboundLine {
                    conn: conn.clone(),
                    msg: ServerMsg::Notify {
                        sub: sub.clone(),
                        event: Event::Change {
                            seq: entry.seq,
                            at: entry.at,
                            author: entry.author.clone(),
                            approved_by: None,
                            cause: None,
                            changes: entry.changes.clone(),
                            evidence: None,
                        },
                    },
                });
            }
            return;
        }
        // The marks its expansion left (none if the registry did not call its handler).
        let n = entry.changes.iter().filter(|c| is_mark(c)).count() as u64;
        if n == 0 {
            return;
        }
        t.count += n;
        t.seen.push(seq);
        let change = tally_item(&t.seen);
        let summary = format!("{SUMMARY}{}", t.count);
        ctx.out.entry(vec![change], summary);
    }
}
