//! The watch ops (`docs/PROTOCOL.md` §25.5): `add_watch` and `remove_watch`, their
//! expansion (moved from `qp-ops`, RF1 S3.1) and their policy.

use qp_ops::expand::Ctx;
use qp_ops::watch::{on, person_of, watch_visible, MAX_WATCHES};
use qp_plane::error::{invalid, ptr, PResult, E};
use qp_schema::protocol::{ChangeOp, ErrorCode, Op, Permissions, ProtocolError};
use qp_schema::{Document, Intent, IntentState, Watch, WatchMatch};
use qp_subsystem::{AgentOp, OpHandler};

/// A watch op: its wire tag and its variant.
pub(crate) struct WatchOp {
    tag: &'static str,
    of: fn(&Op) -> bool,
}

/// `add_watch`: a person's own watch.
pub(crate) static ADD_WATCH: WatchOp = WatchOp {
    tag: "add_watch",
    of: |op| matches!(op, Op::AddWatch { .. }),
};

/// `remove_watch`: a person's own watch, ended.
pub(crate) static REMOVE_WATCH: WatchOp = WatchOp {
    tag: "remove_watch",
    of: |op| matches!(op, Op::RemoveWatch { .. }),
};

impl OpHandler for WatchOp {
    fn op(&self) -> &'static str {
        self.tag
    }
    fn owns(&self, op: &Op) -> bool {
        (self.of)(op)
    }
    fn expand(
        &self,
        doc: &Document,
        op: &Op,
        at: &str,
        ctx: &mut Ctx<'_>,
    ) -> Result<Vec<ChangeOp>, ProtocolError> {
        expand_watch(doc, op, at, ctx)
    }
    /// (R5B, §25.5) A person's own watches: anyone who may read.
    fn allowed(&self, perms: &Permissions) -> bool {
        perms.read
    }
    /// (§4.7) Written directly by the person, never carried in a proposal.
    fn proposable(&self) -> bool {
        false
    }
    /// (§25.3) An agent writes watches directly (for its person).
    fn agent(&self, _doc: &Document, _op: &Op) -> AgentOp {
        AgentOp::Direct
    }
    /// (§25.5) Another person's watch requests stay out of a reader's log.
    fn reader_private(&self) -> bool {
        true
    }
}

/// `add_watch` and `remove_watch` (§25.5).
#[allow(clippy::result_large_err)]
fn expand_watch(doc: &Document, op: &Op, p: &str, ctx: &mut Ctx<'_>) -> PResult<Vec<ChangeOp>> {
    match op {
        Op::AddWatch {
            for_,
            text,
            match_,
            expires_t,
            max_matches,
        } => {
            let w = Watch {
                for_: for_
                    .clone()
                    .unwrap_or_else(|| person_of(ctx.plane, ctx.speaker).to_owned()),
                match_: match_.clone(),
                expires_t: *expires_t,
                max_matches: *max_matches,
            };
            add_watch(doc, w, text, p, ctx)
        }
        Op::RemoveWatch { intent } => remove_watch(doc, intent, p, ctx),
        _ => Err(invalid("not a watch op", p.to_owned())),
    }
}

/// `add_watch`: watch `w` (its person: `for`, else the speaker's person) with request `text`.
#[allow(clippy::result_large_err)]
fn add_watch(
    doc: &Document,
    w: Watch,
    text: &str,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let person = w.for_.clone();
    let probe = Watch {
        for_: person.clone(),
        match_: WatchMatch::default(),
        expires_t: None,
        max_matches: None,
    };
    if !watch_visible(&probe, ctx) {
        return Err(E::new(
            ErrorCode::Forbidden,
            format!("a watch for `{person}` is theirs to leave"),
        )
        .at(ptr(p, "for"))
        .hint("Leave it for yourself (or the person you work for); omit `for`.")
        .build());
    }
    if text.trim().is_empty() {
        return Err(invalid(
            "a watch needs its request in words",
            ptr(p, "text"),
        ));
    }
    if w.max_matches == Some(0) {
        return Err(E::new(ErrorCode::Invalid, "max_matches must be at least 1")
            .at(ptr(p, "max_matches"))
            .expected(["1.."])
            .build());
    }
    let n = doc
        .intents
        .iter()
        .filter(|i| on(i) && i.watch.as_ref().is_some_and(|w| w.for_ == person))
        .count();
    if n >= MAX_WATCHES as usize {
        return Err(E::new(
            ErrorCode::Limit,
            format!("`{person}` has {n} active watches already"),
        )
        .at(p.to_owned())
        .expected([MAX_WATCHES.to_string()])
        .hint("Remove a watch (`remove_watch`) first.")
        .build());
    }
    let iid = ctx.new_id(doc, "watch", None, &ptr(p, "id"))?;
    Ok(vec![ChangeOp::PutIntent {
        index: None,
        intent: Intent {
            id: iid,
            text: text.to_owned(),
            trigger: None,
            active: true,
            provenance: ctx.prov(Vec::new()),
            watch: Some(w),
            state: Some(IntentState::Active),
            matches: 0,
        },
    }])
}

/// `remove_watch`: ends watch `intent` (state `removed`).
#[allow(clippy::result_large_err)]
fn remove_watch(doc: &Document, intent: &str, p: &str, ctx: &Ctx<'_>) -> PResult<Vec<ChangeOp>> {
    let ip = ptr(p, "intent");
    // (Privacy) Another person's watch reads as a missing one; suggestions come
    // from the speaker's own watches only.
    let mine = |i: &&Intent| i.watch.as_ref().is_some_and(|w| watch_visible(w, ctx));
    let Some(i) = doc.intents.iter().filter(mine).find(|i| i.id == intent) else {
        return Err(E::new(ErrorCode::NotFound, format!("no watch `{intent}`"))
            .at(ip)
            .dym(
                intent,
                doc.intents.iter().filter(mine).map(|i| i.id.as_str()),
            )
            .build());
    };
    if !on(i) {
        return Err(E::new(
            ErrorCode::Invalid,
            format!("the watch `{intent}` is not active"),
        )
        .at(ip)
        .build());
    }
    let mut ni = i.clone();
    ni.active = false;
    ni.state = Some(IntentState::Removed);
    Ok(vec![ChangeOp::PutIntent {
        intent: ni,
        index: None,
    }])
}
