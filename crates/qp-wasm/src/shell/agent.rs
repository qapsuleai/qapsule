//! (R5B, GOAL scope 6 "Gösterimde en az iz"; `docs/PROTOCOL.md` §25) The agent surface's
//! least trace in the shell: the open capsule's validated comments (the `capsule` answer's
//! `detail.comments`) with their label and origin, and the pending proposals with a human
//! decision on the shell's own connection (§25.2: unmarked, human principal).
//!
//! Every text is written here (label, origin, author, the proposal's summary) in the
//! reader's language; the page shows it as it is. What is asked:
//! - on welcome, a `proposals` subscription and the pending list (`list`, `state: pending`);
//!   the list is asked again after a proposal event or the answer to a decision;
//! - when a capsule opens from the feed, its `capsule` answer (only the comments are kept).
//!
//! A query that fails here is not the reader's error (no toast): the trace stays empty.

use qp_schema::protocol::{Comment, Language, Proposal, ProposalState};
use qp_schema::Author;
use serde_json::{json, Value};

use super::{Asked, ShellCore};

/// The subscription id of the proposals topic.
pub const PROPOSALS_SUB: &str = "proposals-topic";

/// (R5B) The agent trace a shell core holds.
#[derive(Debug, Default)]
pub struct AgentTrace {
    /// The open capsule's comments: `(capsule, comments)` as the authority answered.
    comments: Option<(String, Vec<Comment>)>,
    /// The pending proposals, in the authority's order.
    proposals: Vec<Proposal>,
    /// The pending list should be asked (again) / is being asked.
    proposals_dirty: bool,
    proposals_inflight: bool,
}

fn author_name(a: &Author) -> &str {
    match a {
        Author::Human { name } | Author::Agent { name, .. } | Author::Engine { name, .. } => name,
    }
}

/// One comment as the page shows it: `{id, label, origin, text}`.
fn comment_row(c: &Comment, lang: Language) -> Value {
    let agent = matches!(c.provenance.author, Author::Agent { .. });
    let label = match (agent, lang) {
        (true, Language::En) => "agent comment",
        (true, _) => "ajan yorumu",
        (false, Language::En) => "comment",
        (false, _) => "yorum",
    };
    let who = author_name(&c.provenance.author);
    let origin = match (&c.on_behalf, lang) {
        (Some(p), Language::En) => format!("{who}, on behalf of {p}"),
        (Some(p), _) => format!("{who}, {p} adına"),
        (None, _) => who.to_owned(),
    };
    json!({"id": c.id, "label": label, "origin": origin, "text": c.text})
}

/// One pending proposal as the page shows it: `{id, author, text}` (`text`: its
/// operations by name, then the proposer's reason).
fn proposal_row(p: &Proposal, lang: Language) -> Value {
    let who = author_name(&p.provenance.author);
    let kind = match (&p.provenance.author, lang) {
        (Author::Agent { .. }, Language::En) => " (agent)",
        (Author::Agent { .. }, _) => " (ajan)",
        _ => "",
    };
    let author = match lang {
        Language::En => format!("Proposed by {who}{kind}"),
        _ => format!("Öneren: {who}{kind}"),
    };
    let ops: Vec<String> = p
        .ops
        .iter()
        .filter_map(|o| {
            serde_json::to_value(o)
                .ok()
                .and_then(|v| v.get("op").and_then(Value::as_str).map(str::to_owned))
        })
        .collect();
    let mut text = ops.join(", ");
    if let Some(r) = p.reason.as_deref().filter(|r| !r.is_empty()) {
        text.push_str(" — ");
        text.push_str(r);
    }
    json!({"id": p.id, "author": author, "text": text})
}

impl ShellCore {
    /// (R5B) On welcome: the proposals subscription; the pending list is asked next.
    pub(super) fn agent_welcome(&mut self) {
        self.push(json!({"type":"subscribe","id":PROPOSALS_SUB,"topic":{"kind":"proposals"}}));
        self.agent.proposals_dirty = true;
    }

    /// (R5B) Asks the pending list when it is due (called from `take_outbox`).
    pub(super) fn agent_outbox(&mut self) {
        if self.agent.proposals_dirty && !self.agent.proposals_inflight && self.welcomed {
            self.agent.proposals_dirty = false;
            self.agent.proposals_inflight = true;
            self.ask(
                "prop",
                Asked::Proposals,
                json!({"type":"query","query":{"kind":"list","collection":"proposals","state":"pending"}}),
            );
        }
    }

    /// (R5B) A proposal was created or decided (the proposals topic): ask the list again.
    pub(super) fn agent_proposal_event(&mut self) {
        self.agent.proposals_dirty = true;
    }

    /// (R5B) The pending list arrived (`items`); `ch.agent` when it changed.
    pub(super) fn agent_proposals(&mut self, items: &[Value], ch: &mut Value) {
        self.agent.proposals_inflight = false;
        let list: Vec<Proposal> = items
            .iter()
            .filter_map(|v| serde_json::from_value::<Proposal>(v.clone()).ok())
            .filter(|p| p.state == ProposalState::Pending)
            .collect();
        if list != self.agent.proposals {
            self.agent.proposals = list;
            ch["agent"] = json!(true);
        }
    }

    /// (R5B) The pending-list query failed: nothing is shown changed, it may be asked again.
    pub(super) fn agent_proposals_failed(&mut self) {
        self.agent.proposals_inflight = false;
    }

    /// (R5B) A capsule opened from the feed: its comments are asked (`capsule` query).
    pub(super) fn agent_opened(&mut self, capsule: &str, ch: &mut Value) {
        if self.agent.comments.take().is_some() {
            ch["agent"] = json!(true);
        }
        self.agent.comments = Some((capsule.to_owned(), Vec::new()));
        self.ask(
            "cmt",
            Asked::Comments,
            json!({"type":"query","query":{"kind":"capsule","capsule":capsule}}),
        );
    }

    /// (R5B) The open capsule closed: its comments go.
    pub(super) fn agent_closed(&mut self, ch: &mut Value) {
        if self
            .agent
            .comments
            .take()
            .is_some_and(|(_, c)| !c.is_empty())
        {
            ch["agent"] = json!(true);
        }
    }

    /// (R5B) The `capsule` answer: kept if that capsule is still the open one.
    pub(super) fn agent_comments(&mut self, capsule: &str, comments: &[Comment], ch: &mut Value) {
        if let Some((open, held)) = self.agent.comments.as_mut() {
            if open == capsule && held.as_slice() != comments {
                *held = comments.to_vec();
                ch["agent"] = json!(true);
            }
        }
    }

    /// (R5B) The open capsule's comments as the page shows them: `[{id, label, origin,
    /// text}]` (empty: none, or no capsule open).
    #[must_use]
    pub fn comments_json(&self) -> Value {
        let lang = self.lang;
        Value::Array(
            self.agent
                .comments
                .as_ref()
                .map(|(_, c)| c.iter().map(|c| comment_row(c, lang)).collect())
                .unwrap_or_default(),
        )
    }

    /// (R5B) The pending proposals as the page shows them: `[{id, author, text}]`.
    #[must_use]
    pub fn proposals_json(&self) -> Value {
        let lang = self.lang;
        Value::Array(
            self.agent
                .proposals
                .iter()
                .map(|p| proposal_row(p, lang))
                .collect(),
        )
    }

    /// (R5B, §25.2) The reader's decision on pending proposal `id`: `accept_proposal` or
    /// `reject_proposal`, on this connection (the shell's own: a person's, unmarked). An
    /// id that is not in the pending list sends nothing.
    pub fn decide(&mut self, id: &str, accept: bool) {
        if !self.agent.proposals.iter().any(|p| p.id == id) {
            return;
        }
        let op = if accept {
            "accept_proposal"
        } else {
            "reject_proposal"
        };
        self.ask(
            "dec",
            Asked::Decision,
            json!({"type":"command","ops":[{"op":op,"proposal":id}]}),
        );
    }

    /// (R5B) A decision was answered (applied or refused): the list is asked again.
    pub(super) fn agent_decided(&mut self) {
        self.agent.proposals_dirty = true;
    }
}
