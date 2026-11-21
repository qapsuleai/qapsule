//! (R5B/M31) The sentinel's notification filter: which capsule transitions and watch
//! matches become a **notification** to the person.
//!
//! Deterministic and model-free (K3): the trigger is product code. A language model (the
//! sentinel agent) only turns free text into a structured watch and narrates a
//! notification; it never decides whether one is sent. The `text` written here is a short
//! fixed-template description, not a narration.
//!
//! Input: the events of a window in arrival order ([`SentinelEvent`]: capsule transitions
//! as the person's connection sees them with `all: true`, and `intent_matched` events) and
//! the person's audience ([`SentinelAudience`]). Three alternatives ([`Filter`], chosen by
//! name):
//!
//! | Filter | Notifies |
//! |--------|----------|
//! | `every` | every watch match + every transition that enters the person's feed (§20.4) |
//! | `tier_sens` | watch matches + births that pass the sensitivity tier (K31) + negative transitions of a story about a position or an active watch (K42, any level) |
//! | `tier_repeat_hold` | `tier_sens` + repeat suppression (a capsule's second notification on a day only when it escalates to a negative transition) + holdings first (on an instrument the person holds no position in: only a tier‑1 birth or a watch match) |
//!
//! **K42 guarantee.** A negative transition (`weakened`, `invalidated`, `retracted`) of a
//! story about an instrument the person holds (non-zero position) is **mandatory**: [`apply`] decides it before any filter is asked, so no
//! filter can drop it (test `crates/qp-capsule/tests/sentinel.rs`).
//!
//! One notification per event: a transition and the watch matches on the same
//! `(day, capsule, stage)` are one notification (the text names the watches).

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::capsule::{Direction, MeaningKind, Sensitivity, Stage};
use serde::Serialize;

/// A capsule transition as the person's connection received it.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    /// Window day (1-based replay step).
    pub day: u32,
    /// Capsule id.
    pub capsule: String,
    /// Instrument symbol.
    pub instrument: String,
    /// Meaning kind.
    pub kind: MeaningKind,
    /// Direction.
    pub direction: Direction,
    /// The transition's stage.
    pub stage: Stage,
    /// Tier (1 most important).
    pub tier: u8,
    /// Whether the engine marks the story as about the person's position or active
    /// intent (`CapsuleItem.relevant`).
    pub relevant: bool,
    /// Whether the feed rule calls the story a coin flip (K53).
    pub coin_flip: bool,
}

/// An `intent_matched` event of the `intents` topic.
#[derive(Debug, Clone, PartialEq)]
pub struct WatchMatch {
    /// Window day.
    pub day: u32,
    /// Intent (watch) id.
    pub intent: String,
    /// The watch's text, if known (for the notification text).
    pub text: Option<String>,
    /// Capsule id.
    pub capsule: String,
    /// Stage the story moved to.
    pub stage: Stage,
}

/// One input event, in arrival order.
#[derive(Debug, Clone, PartialEq)]
pub enum SentinelEvent {
    /// A capsule transition (`capsules` topic).
    Transition(Transition),
    /// A watch match (`intents` topic).
    Match(WatchMatch),
}

impl SentinelEvent {
    fn key(&self) -> (u32, &str, Stage) {
        match self {
            Self::Transition(t) => (t.day, t.capsule.as_str(), t.stage),
            Self::Match(m) => (m.day, m.capsule.as_str(), m.stage),
        }
    }
}

/// The person the sentinel works for.
#[derive(Debug, Clone, PartialEq)]
pub struct SentinelAudience {
    /// Instruments with a non-zero position.
    pub positions: BTreeSet<String>,
    /// Watch list (empty = every instrument).
    pub watchlist: BTreeSet<String>,
    /// Instruments of the person's active watches/intents.
    pub intents_on: BTreeSet<String>,
    /// Effective sensitivity.
    pub sensitivity: Sensitivity,
}

impl SentinelAudience {
    fn watched(&self, inst: &str) -> bool {
        self.watchlist.is_empty()
            || self.watchlist.contains(inst)
            || self.positions.contains(inst)
            || self.intents_on.contains(inst)
    }

    fn tier_ok(&self, tier: u8) -> bool {
        match self.sensitivity {
            Sensitivity::Important => tier <= 1,
            Sensitivity::Balanced => tier <= 2,
            Sensitivity::All => true,
        }
    }

    /// Whether the transition is about the person's position or active intent.
    fn involved(&self, t: &Transition) -> bool {
        t.relevant
            || self.positions.contains(&t.instrument)
            || self.intents_on.contains(&t.instrument)
    }

    /// Whether the transition enters the person's feed: the `capsules` topic without
    /// `all` (PROTOCOL §20.4, CAPSULE §9.8, §9.10; feed rule `rate55` with
    /// `exempt_negative`).
    #[must_use]
    pub fn in_feed(&self, t: &Transition) -> bool {
        if !self.watched(&t.instrument) {
            return false;
        }
        if t.stage.is_negative() && self.involved(t) {
            return true;
        }
        if t.coin_flip && self.sensitivity != Sensitivity::All {
            return false;
        }
        self.tier_ok(t.tier)
    }

    /// (K42) Whether the transition is a mandatory notification: a negative transition of
    /// a story about an instrument the person holds a position in.
    #[must_use]
    pub fn mandatory(&self, t: &Transition) -> bool {
        t.stage.is_negative() && self.positions.contains(&t.instrument)
    }
}

/// The three measured alternatives (M31).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Filter {
    /// Every watch match and every feed transition.
    Every,
    /// Watch matches + tier-passing births + involved negatives.
    TierSens,
    /// `TierSens` + repeat suppression + holdings first.
    TierRepeatHold,
}

impl Filter {
    /// All filters, in measurement order.
    pub const ALL: [Self; 3] = [Self::Every, Self::TierSens, Self::TierRepeatHold];

    /// The filter's name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Every => "every",
            Self::TierSens => "tier_sens",
            Self::TierRepeatHold => "tier_repeat_hold",
        }
    }

    /// The filter of a name.
    #[must_use]
    pub fn by_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name() == name)
    }
}

/// Why a notification was sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Why {
    /// K42: negative transition on a position (never dropped).
    Mandatory,
    /// A watch matched.
    Watch,
    /// K42: negative transition on an instrument of an active watch (no position).
    Involved,
    /// A birth that passed the sensitivity tier.
    Birth,
    /// The transition entered the feed (`every` only).
    Feed,
}

/// One notification.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Notification {
    /// Window day.
    pub day: u32,
    /// Capsule id.
    pub capsule: String,
    /// Stage.
    pub stage: Stage,
    /// Short deterministic description (template, no model).
    pub text: String,
    /// Reasons, sorted.
    #[serde(skip)]
    pub why: Vec<Why>,
}

/// What is known about one `(day, capsule, stage)` event.
struct Item<'a> {
    day: u32,
    capsule: &'a str,
    stage: Stage,
    transition: Option<&'a Transition>,
    watches: Vec<&'a WatchMatch>,
}

/// Applies `filter` to `events` for `audience`: the notifications in arrival order (of
/// each event's first input line).
#[must_use]
pub fn apply(
    filter: Filter,
    events: &[SentinelEvent],
    audience: &SentinelAudience,
) -> Vec<Notification> {
    // Merge a transition and its watch matches into one item, in first-arrival order.
    let mut order: Vec<(u32, String, Stage)> = Vec::new();
    let mut items: BTreeMap<(u32, String, u8), Item<'_>> = BTreeMap::new();
    for ev in events {
        let (day, cap, stage) = ev.key();
        let k = (day, cap.to_owned(), stage_ord(stage));
        let it = items.entry(k).or_insert_with(|| {
            order.push((day, cap.to_owned(), stage));
            Item {
                day,
                capsule: cap,
                stage,
                transition: None,
                watches: Vec::new(),
            }
        });
        match ev {
            SentinelEvent::Transition(t) => {
                if it.transition.is_none() {
                    it.transition = Some(t);
                }
            }
            SentinelEvent::Match(m) => it.watches.push(m),
        }
    }
    // (capsule, day) → whether a negative notification was already sent that day.
    let mut sent_today: BTreeMap<(u32, String), bool> = BTreeMap::new();
    let mut out = Vec::new();
    for (day, cap, stage) in order {
        let it = &items[&(day, cap.clone(), stage_ord(stage))];
        let mut why = Vec::new();
        // K42 first: decided before any filter is asked.
        if it.transition.is_some_and(|t| audience.mandatory(t)) {
            why.push(Why::Mandatory);
        }
        let optional = optional_reasons(filter, it, audience, &sent_today);
        let send = !why.is_empty() || !optional.is_empty();
        why.extend(optional);
        if !send {
            continue;
        }
        why.sort();
        why.dedup();
        let neg = stage.is_negative();
        let e = sent_today.entry((day, cap.clone())).or_insert(false);
        *e |= neg;
        out.push(Notification {
            day: it.day,
            capsule: it.capsule.to_owned(),
            stage,
            text: describe(it, &why),
            why,
        });
    }
    out
}

/// The non-mandatory reasons `filter` has to notify `it` (empty: the filter does not).
fn optional_reasons(
    filter: Filter,
    it: &Item<'_>,
    audience: &SentinelAudience,
    sent_today: &BTreeMap<(u32, String), bool>,
) -> Vec<Why> {
    let mut why = Vec::new();
    if !it.watches.is_empty() {
        why.push(Why::Watch);
    }
    let t = it.transition;
    match filter {
        Filter::Every => {
            if t.is_some_and(|t| audience.in_feed(t)) {
                why.push(Why::Feed);
            }
        }
        Filter::TierSens | Filter::TierRepeatHold => {
            if t.is_some_and(|t| t.stage == Stage::Born && audience.in_feed(t)) {
                why.push(Why::Birth);
            }
            if t.is_some_and(|t| {
                t.stage.is_negative() && audience.involved(t) && !audience.mandatory(t)
            }) {
                why.push(Why::Involved);
            }
        }
    }
    if filter != Filter::TierRepeatHold || why.is_empty() {
        return why;
    }
    // Holdings first: off-position instruments only for a tier-1 birth or a watch match.
    let held = t.is_some_and(|t| audience.positions.contains(&t.instrument));
    if !held {
        let tier1_birth = t.is_some_and(|t| t.stage == Stage::Born && t.tier == 1);
        why.retain(|w| *w == Why::Watch || (*w == Why::Birth && tier1_birth));
    }
    // Repeat suppression: a capsule's second notification of the day only when it
    // escalates to a negative transition.
    if let Some(neg_sent) = sent_today.get(&(it.day, it.capsule.to_owned())) {
        if !it.stage.is_negative() || *neg_sent {
            why.clear();
        }
    }
    why
}

fn stage_ord(s: Stage) -> u8 {
    match s {
        Stage::Candidate => 0,
        Stage::Born => 1,
        Stage::Strengthened => 2,
        Stage::Weakened => 3,
        Stage::Confirmed => 4,
        Stage::Realized => 5,
        Stage::Invalidated => 6,
        Stage::Expired => 7,
        Stage::Retracted => 8,
    }
}

fn kind_tr(k: MeaningKind) -> &'static str {
    match k {
        MeaningKind::Continuation => "devam",
        MeaningKind::Reversal => "dönüş",
        MeaningKind::Breakout => "kırılım",
        MeaningKind::Squeeze => "sıkışma",
        MeaningKind::Extreme => "aşırılık",
        MeaningKind::Flow => "akış",
    }
}

fn dir_tr(d: Direction) -> &'static str {
    match d {
        Direction::Up => "yukarı ",
        Direction::Down => "aşağı ",
        Direction::Neutral => "",
    }
}

fn stage_tr(s: Stage) -> &'static str {
    match s {
        Stage::Candidate => "aday",
        Stage::Born => "doğdu",
        Stage::Strengthened => "güçlendi",
        Stage::Weakened => "zayıfladı",
        Stage::Confirmed => "teyit edildi",
        Stage::Realized => "gerçekleşti",
        Stage::Invalidated => "geçersiz oldu",
        Stage::Expired => "süresi doldu",
        Stage::Retracted => "geri çekildi",
    }
}

/// `SYN02 aşağı kırılım geçersiz oldu (pozisyon; izleme: …)`.
fn describe(it: &Item<'_>, why: &[Why]) -> String {
    let head = match it.transition {
        Some(t) => format!(
            "{} {}{} {}",
            t.instrument,
            dir_tr(t.direction),
            kind_tr(t.kind),
            stage_tr(t.stage)
        ),
        None => format!("{} {}", it.capsule, stage_tr(it.stage)),
    };
    let mut tags: Vec<String> = Vec::new();
    for w in why {
        match w {
            Why::Mandatory => tags.push("pozisyon".to_owned()),
            Why::Birth => {
                if let Some(t) = it.transition {
                    tags.push(format!("kademe {}", t.tier));
                }
            }
            Why::Feed => tags.push("akış".to_owned()),
            Why::Involved => tags.push("izlenen hisse".to_owned()),
            Why::Watch => {
                for m in &it.watches {
                    tags.push(format!(
                        "izleme: {}",
                        m.text.as_deref().unwrap_or(&m.intent)
                    ));
                }
            }
        }
    }
    if tags.is_empty() {
        head
    } else {
        format!("{head} ({})", tags.join("; "))
    }
}
