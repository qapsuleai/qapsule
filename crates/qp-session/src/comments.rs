//! (R5B, `docs/PROTOCOL.md` §25.4) Comments: the evidence the authority gathers for a
//! draft, the validation (`qp_capsule::comment::validate`), and who may receive a stored
//! comment.
//!
//! **Evidence** is gathered as the writer reads (an agent: with its person's
//! entitlements, §25.7), in the writer's world: for every cited capsule (or finding) the
//! reader's `capsule` answer — its evidence values, its facts' values, the numbers of
//! the reader's sentence (the headline pieces as the reader sees them) and the ledger
//! (`n`, `realized`) —, for every cited fact the reader's fact values. Ids that exist only
//! in the authority's world or that the reader's projection hides are `hidden_ids`. For a
//! reader that may not see everything (a public-view reader or one with a projection) the
//! authority's own numbers about the cited items and the comment's instrument (fact
//! values, stored claims) are `withheld`: a number that matches one of them and none of
//! the reader's is a licence leak.
//!
//! **Receiving** a stored comment: every cite is visible to the reader and, for a reader
//! with a projection, every number in the text is one of the reader's own evidence values
//! (external quotes do not count there). Comments enter the public world only when they
//! validate for a public reader (`Session::comment_public_ok`).

use std::collections::BTreeSet;

use qp_capsule::comment::{
    lang_of, numbers, validate, CitedDirection, CitedValue, Direction as CDir, Evidence,
};
use qp_schema::comment::CommentDraft;
use qp_schema::protocol::{
    ChangeOp, Comment, CommentAbout, ErrorCode, Event, Op, OutboundLine, QueryResult, ServerMsg,
    Topic,
};
use qp_schema::Document;

use crate::error::{ptr, PResult, E};
use crate::session::{out, Session};

/// Decimals of a value as the engine would print it (shortest form, at most 6).
fn decimals_of(v: f64) -> u8 {
    let s = format!("{v}");
    s.split_once('.')
        .map_or(0, |(_, d)| u8::try_from(d.len().min(6)).unwrap_or(6))
}

fn value(id: &str, key: &str, v: f64) -> CitedValue {
    CitedValue {
        id: id.to_owned(),
        key: key.to_owned(),
        value: v,
        decimals: decimals_of(v),
    }
}

/// Instrument symbols of the document's series (removed before the number scan).
fn symbols(doc: &Document) -> Vec<String> {
    let mut v: Vec<String> = doc
        .series
        .iter()
        .filter_map(|s| s.symbol().map(str::to_owned))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// What a reader sees of the cited items (in the reader's world, inside
/// [`Session::as_reader`]).
struct Seen {
    known: BTreeSet<String>,
    values: Vec<CitedValue>,
    directions: Vec<CitedDirection>,
}

impl Session {
    /// The cited items as the reader of connection `ci` sees them (call inside
    /// `as_reader`).
    fn seen(&self, ci: usize, cites: &[String]) -> Seen {
        let mut s = Seen {
            known: BTreeSet::new(),
            values: Vec::new(),
            directions: Vec::new(),
        };
        let lang = lang_of(self.lang_of(ci));
        let syms = symbols(&self.doc);
        for id in cites {
            if let Ok(QueryResult::CapsuleDetail { detail }) =
                self.capsule_query_with(ci, id, "/x", false)
            {
                s.known.insert(id.clone());
                for e in &detail.evidence {
                    for (k, v) in &e.values {
                        s.values.push(value(id, k, *v));
                    }
                }
                for f in &detail.facts {
                    for (k, v) in &f.evidence {
                        s.values.push(value(id, k, *v));
                    }
                }
                for n in numbers(&detail.item.sentence.text, lang, &syms) {
                    s.values.push(CitedValue {
                        id: id.clone(),
                        key: "sentence".to_owned(),
                        value: n.value,
                        decimals: n.decimals,
                    });
                }
                s.values
                    .push(value(id, "ledger_n", f64::from(detail.ledger.n)));
                s.values.push(value(
                    id,
                    "ledger_realized",
                    f64::from(detail.ledger.realized),
                ));
                let d = match detail.item.direction {
                    qp_schema::capsule::Direction::Up => CDir::Up,
                    qp_schema::capsule::Direction::Down => CDir::Down,
                    qp_schema::capsule::Direction::Neutral => CDir::Neutral,
                };
                s.directions.push(CitedDirection {
                    id: id.clone(),
                    direction: d,
                });
                continue;
            }
            if let Some(f) = self.doc.facts.iter().find(|f| &f.id == id) {
                s.known.insert(id.clone());
                for (k, v) in &f.evidence {
                    s.values.push(value(id, k, *v));
                }
                continue;
            }
            if let Some(f) = self.doc.findings.iter().find(|f| &f.id == id) {
                s.known.insert(id.clone());
                for fid in &f.facts {
                    if let Some(x) = self.doc.facts.iter().find(|x| &x.id == fid) {
                        for (k, v) in &x.evidence {
                            s.values.push(value(id, k, *v));
                        }
                    }
                }
                continue;
            }
            if self.doc.capsules.iter().any(|c| &c.id == id)
                || self.doc.comments.iter().any(|c| &c.id == id)
            {
                s.known.insert(id.clone());
            }
        }
        s
    }

    /// The authority's own numbers about the cited items and `instrument` (fact values,
    /// stored claims), and every id of its document (outside `as_reader`, in the
    /// authority's world).
    fn full_numbers(
        &self,
        cites: &[String],
        instrument: Option<&str>,
    ) -> (Vec<f64>, BTreeSet<String>) {
        let doc = &self.doc;
        let syms = symbols(doc);
        let on_inst = |series: &str| {
            instrument.is_some_and(|i| {
                doc.series
                    .iter()
                    .any(|s| s.id == series && s.symbol() == Some(i))
            })
        };
        let mut nums = Vec::new();
        for f in &doc.facts {
            if cites.contains(&f.id) || on_inst(&f.series) {
                nums.extend(f.evidence.values().copied());
            }
        }
        for f in &doc.findings {
            let cap = doc.capsules.iter().find(|c| c.finding == f.id);
            let about_inst = cap
                .and_then(|c| c.story.as_ref())
                .is_some_and(|st| Some(st.instrument.as_str()) == instrument);
            if cites.contains(&f.id) || cap.is_some_and(|c| cites.contains(&c.id)) || about_inst {
                for n in numbers(&f.claim, lang_of(qp_schema::protocol::Language::Tr), &syms) {
                    nums.push(n.value);
                }
                for fid in &f.facts {
                    if let Some(x) = doc.facts.iter().find(|x| &x.id == fid) {
                        nums.extend(x.evidence.values().copied());
                    }
                }
            }
        }
        let ids = crate::apply::all_ids(doc)
            .into_iter()
            .map(str::to_owned)
            .chain(doc.findings.iter().map(|f| f.id.clone()))
            .collect();
        (nums, ids)
    }

    /// The instrument a comment is about (its capsule's, for a capsule).
    fn about_instrument(&self, about: &CommentAbout) -> Option<String> {
        if let Some(i) = &about.instrument {
            return Some(i.clone());
        }
        let c = about.capsule.as_deref()?;
        self.doc
            .capsules
            .iter()
            .find(|x| x.id == c || x.finding == c)
            .and_then(|x| x.story.as_ref())
            .map(|s| s.instrument.clone())
    }

    /// The evidence for `draft` as the reader of connection `ci` reads (§25.4).
    pub(crate) fn comment_evidence(
        &mut self,
        ci: usize,
        draft: &CommentDraft,
        about: &CommentAbout,
    ) -> Evidence {
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        let instrument = self.about_instrument(about);
        let restricted = self.public_view(&principal) || self.reader_proj(&principal).is_some();
        let (full, all) = self.full_numbers(&draft.cites, instrument.as_deref());
        let mut syms = symbols(&self.doc);
        let seen = self.as_reader(&principal, lang, |s| s.seen(ci, &draft.cites));
        // (Security) No existence oracle: an id the reader may not see is answered like
        // one that does not exist (`unknown_id`).
        let _ = all;
        let hidden = BTreeSet::new();
        if let Some(i) = instrument {
            if !syms.contains(&i) {
                syms.push(i);
            }
        }
        Evidence {
            known_ids: seen.known,
            hidden_ids: hidden,
            values: seen.values,
            withheld: if restricted { full } else { Vec::new() },
            symbols: syms,
            directions: seen.directions,
        }
    }

    /// (R5B, §25.4) Validates every `add_comment` of `ops` for the writer of `ci`;
    /// `comment_rejected` with the reasons of the first rejected one.
    pub(crate) fn check_comments(&mut self, ci: usize, ops: &[Op]) -> PResult<()> {
        let strictness = self.plane.comment_strictness;
        for (i, op) in ops.iter().enumerate() {
            let Op::AddComment {
                about,
                lang,
                text,
                cites,
                external,
                ..
            } = op
            else {
                continue;
            };
            let draft = CommentDraft {
                lang: *lang,
                text: text.clone(),
                cites: cites.clone(),
                external: external.clone(),
            };
            let ev = self.comment_evidence(ci, &draft, about);
            let principal = self.conns[ci].principal.clone();
            let verdict = validate(&draft, &ev, strictness);
            self.comment_audit.push(CommentAudit {
                principal: principal.clone(),
                accepted: verdict.is_ok(),
                reasons: verdict.clone().err().unwrap_or_default(),
            });
            let restricted = self.public_view(&principal) || self.reader_proj(&principal).is_some();
            if let Err(reasons) = verdict {
                let reasons = if restricted { blind(reasons) } else { reasons };
                let mut e = E::new(
                    ErrorCode::CommentRejected,
                    format!(
                        "the comment was rejected: {} reason(s), first `{}`",
                        reasons.len(),
                        reasons.first().map_or("", |r| r.code.as_str())
                    ),
                )
                .at(ptr("/ops", i))
                .hint("Fix every reason (cite visible ids, write only numbers the cited evidence gives, no advice) and send again.")
                .build();
                e.reasons = reasons;
                return Err(e);
            }
        }
        Ok(())
    }

    /// (R5B, §25.4) Whether the `add_comment`s of `ops` may enter the public world: each
    /// validates for a reader of the public world (the writer's bar entitlement, no
    /// licensed flow), with the authority's numbers withheld.
    pub(crate) fn comment_public_ok(&mut self, ci: usize, ops: &[Op]) -> bool {
        let strictness = self.plane.comment_strictness;
        for op in ops {
            let Op::AddComment {
                about,
                lang,
                text,
                cites,
                external,
                ..
            } = op
            else {
                continue;
            };
            let draft = CommentDraft {
                lang: *lang,
                text: text.clone(),
                cites: cites.clone(),
                external: external.clone(),
            };
            let instrument = self.about_instrument(about);
            let (full, all) = self.full_numbers(&draft.cites, instrument.as_deref());
            let syms = symbols(&self.doc);
            let seen = self.in_world(true, |s| s.seen(ci, &draft.cites));
            let ev = Evidence {
                hidden_ids: {
                    let _ = all;
                    BTreeSet::new()
                },
                known_ids: seen.known,
                values: seen.values,
                withheld: full,
                symbols: syms,
                directions: seen.directions,
            };
            if validate(&draft, &ev, strictness).is_err() {
                return false;
            }
        }
        true
    }

    /// Whether the reader of `ci` may receive `c` (inside `as_reader`, in its world).
    pub(crate) fn comment_visible(&self, ci: usize, c: &Comment) -> bool {
        if self.proj.as_ref().is_some_and(|p| {
            c.cites.iter().any(|x| p.hidden.contains(x))
                || c.about
                    .capsule
                    .as_ref()
                    .is_some_and(|x| p.hidden.contains(x))
        }) {
            return false;
        }
        let seen = self.seen(ci, &c.cites);
        if c.cites.iter().any(|x| !seen.known.contains(x)) {
            return false;
        }
        if self.proj.is_none() {
            return true;
        }
        let syms = symbols(&self.doc);
        numbers(&c.text, lang_of(c.lang), &syms).iter().all(|n| {
            seen.values
                .iter()
                .any(|v| qp_capsule::comment::number_matches(n, v.value, Some(v.decimals)))
        })
    }

    /// Comments about capsule `cap` the reader of `ci` may receive (inside `as_reader`).
    pub(crate) fn capsule_comments(&self, ci: usize, cap: &str) -> Vec<Comment> {
        self.doc
            .comments
            .iter()
            .filter(|c| c.about.capsule.as_deref() == Some(cap))
            .filter(|c| self.comment_visible(ci, c))
            .cloned()
            .collect()
    }

    /// (R5B, §25.4) `comment` events for the comments stored by the entries after
    /// `full_len` (the authority's log) and `pub_len` (the public world's), to the
    /// `comments` subscribers that may receive them, in connection and subscription
    /// order.
    pub(crate) fn comment_events(&mut self, full_len: usize, pub_len: usize) -> Vec<OutboundLine> {
        let any = self.conns.iter().any(|c| {
            c.subs
                .iter()
                .any(|s| matches!(s.topic, Topic::Comments { .. }))
        });
        if !any {
            return Vec::new();
        }
        let new_in = |log: &[qp_schema::protocol::LogEntry], from: usize| {
            let mut v = Vec::new();
            for e in log.iter().skip(from) {
                for ch in &e.changes {
                    if let ChangeOp::PutComment { comment, .. } = ch {
                        v.push((comment.clone(), e.seq));
                    }
                }
            }
            v
        };
        let full = new_in(&self.log, full_len);
        let public = if self.other.is_some() {
            self.in_world(true, |s| new_in(&s.log, pub_len))
        } else {
            Vec::new()
        };
        let mut lines = Vec::new();
        for ci in 0..self.conns.len() {
            let principal = self.conns[ci].principal.clone();
            let subs: Vec<(String, Option<String>, Option<String>)> = self.conns[ci]
                .subs
                .iter()
                .filter_map(|s| match &s.topic {
                    Topic::Comments {
                        capsule,
                        instrument,
                    } => Some((s.id.clone(), capsule.clone(), instrument.clone())),
                    _ => None,
                })
                .collect();
            if subs.is_empty() {
                continue;
            }
            let pv = self.other.is_some() && self.public_view(&principal);
            let items = if pv { &public } else { &full };
            if items.is_empty() {
                continue;
            }
            let items = items.clone();
            let lang = self.lang_of(ci);
            let conn = self.conns[ci].id.clone();
            let got: Vec<OutboundLine> = self.as_reader(&principal, lang, |s| {
                let mut v = Vec::new();
                for (c, seq) in &items {
                    if !s.comment_visible(ci, c) {
                        continue;
                    }
                    let inst = s.about_instrument(&c.about);
                    for (sub, cap, ins) in &subs {
                        if cap.as_ref().is_some_and(|x| {
                            c.about.capsule.as_ref() != Some(x)
                                && !s.doc.capsules.iter().any(|k| {
                                    &k.finding == x && c.about.capsule.as_ref() == Some(&k.id)
                                })
                        }) {
                            continue;
                        }
                        if ins.is_some() && ins.as_ref() != inst.as_ref() {
                            continue;
                        }
                        v.push(out(
                            &conn,
                            ServerMsg::Notify {
                                sub: sub.clone(),
                                event: Event::Comment {
                                    comment: Box::new(c.clone()),
                                    seq: *seq,
                                },
                            },
                        ));
                    }
                }
                v
            });
            lines.extend(got);
        }
        lines
    }
}

/// (R5B, §25.4) One verdict of the comment validator, as the host may read it
/// ([`Session::comment_audit`]): the real reasons, licence reasons included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentAudit {
    /// The writer.
    pub principal: String,
    /// Accepted.
    pub accepted: bool,
    /// The validator's reasons (empty when accepted).
    pub reasons: Vec<qp_schema::comment::Reason>,
}

/// The hint of an unsupported number (`qp_capsule::comment`).
const UNSUPPORTED_HINT: &str = "Every number must be a value of a cited item or of a named external quote, written at its precision; derived numbers are not allowed.";

/// (Security, §25.4) The reasons a restricted writer receives: a licence reason reads
/// as an unsupported number, and no number reason names its token, so a licensed value
/// and a made-up one get the same bytes (the comment is refused all the same).
fn blind(reasons: Vec<qp_schema::comment::Reason>) -> Vec<qp_schema::comment::Reason> {
    use qp_schema::comment::{Check, Reason};
    let mut out: Vec<Reason> = Vec::new();
    for r in reasons {
        let r = match (r.check, r.code.as_str()) {
            (Check::License, "licensed_value" | "licensed_external")
            | (Check::Number, "unsupported_number") => Reason {
                check: Check::Number,
                code: "unsupported_number".to_owned(),
                token: None,
                sentence: if r.code == "licensed_external" {
                    None
                } else {
                    r.sentence
                },
                hint: UNSUPPORTED_HINT.to_owned(),
            },
            (Check::License, "not_visible") => Reason {
                check: Check::Cites,
                code: "unknown_id".to_owned(),
                token: r.token,
                sentence: r.sentence,
                hint: "This id does not exist; cite ids returned by the Qapsule tools.".to_owned(),
            },
            _ => r,
        };
        if out.last() != Some(&r) {
            out.push(r);
        }
    }
    out
}

impl Session {
    /// (R5B, §25.4) The comment validator's real verdicts so far, in order (host side,
    /// never sent to a client): what a restricted writer was told as an unsupported
    /// number may be a licence catch here (M30 counts these).
    #[must_use]
    pub fn comment_audit(&self) -> &[CommentAudit] {
        &self.comment_audit
    }
}

/// Whether a projected reader may receive comment `c` in a document or a change: its
/// cites are not hidden and its text has no number (§25.4; the precise rule — numbers
/// the reader sees — is applied by the `capsule` answer and the `comments` topic).
pub(crate) fn projected_keeps(c: &Comment, hidden: &BTreeSet<String>) -> bool {
    if c.cites.iter().any(|x| hidden.contains(x))
        || c.about.capsule.as_ref().is_some_and(|x| hidden.contains(x))
    {
        return false;
    }
    numbers(&c.text, lang_of(c.lang), &[]).is_empty()
}
