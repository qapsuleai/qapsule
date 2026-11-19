//! Price card (`SymbolMessage`, topic `mx/symbol/<SYM>@lvl2`) book.
//!
//! **Field presence is preserved.** Matriks sends a full picture first
//! (initial) and afterwards only the fields that changed
//! (`docs/matriks/MESSAGES.md` §3.1). A field that did not arrive did **not**
//! change; it is never read as 0. Every `SymbolMessage` field except
//! `symbolId` is proto2 `optional`, so the generated type keeps presence as
//! `Option<T>`, and updates are applied with protobuf merge semantics
//! ([`prost::Message::merge`]): only the singular fields present in the
//! payload overwrite the state.
//!
//! **Initial vs update.** The first message after a subscription (or a
//! re-subscription, see [`SymbolBook::resubscribed`]) is [`UpdateKind::Initial`];
//! later ones are [`UpdateKind::Update`]. By default an initial **replaces** the
//! previous state ([`InitialPolicy::Replace`]): it is a full picture, so a field
//! that the new picture no longer carries must not stay stale from before.
//! [`InitialPolicy::Merge`] merges initials like updates instead.

use std::collections::BTreeMap;

use prost::Message as _;

use crate::fields::{FieldSet, SYMBOL_MESSAGE};
use crate::messages::symbol_message::PublishReason;
use crate::messages::SymbolMessage;
use crate::status::{session_phase, SessionPhase};
use crate::wire::field_numbers;
use crate::DecodeError;

/// Whether a message is the first after (re-)subscribing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateKind {
    /// First message after a subscription or re-subscription (full picture).
    Initial,
    /// A later message carrying only the changed fields.
    Update,
}

/// How an [`UpdateKind::Initial`] message is applied to existing state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InitialPolicy {
    /// Drop the previous state and rebuild it from the initial (default).
    #[default]
    Replace,
    /// Merge the initial into the previous state like an update.
    Merge,
}

/// The event produced by one `SymbolMessage`.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolUpdate {
    /// Symbol key the book uses (from the topic).
    pub symbol: String,
    /// Initial or update.
    pub kind: UpdateKind,
    /// The fields this message carried on the wire.
    pub fields: FieldSet,
    /// `publishReason` as sent (raw enum value), if the field was on the wire.
    pub publish_reason: Option<i32>,
    /// `stockStatus` as sent, if the field was on the wire.
    pub stock_status: Option<i32>,
}

impl SymbolUpdate {
    /// `publishReason` as the generated enum (`None` if absent or unknown value).
    pub fn publish_reason_kind(&self) -> Option<PublishReason> {
        self.publish_reason
            .and_then(|v| PublishReason::try_from(v).ok())
    }

    /// The session phase ("seans evresi") if this message carried `stockStatus`
    /// and the code is in the proto list.
    pub fn session_phase(&self) -> Option<&'static SessionPhase> {
        self.stock_status.and_then(session_phase)
    }
}

#[derive(Debug, Clone)]
struct Entry {
    state: SymbolMessage,
    awaiting_initial: bool,
}

/// Per-symbol price card state with protobuf merge semantics.
#[derive(Debug, Clone, Default)]
pub struct SymbolBook {
    policy: InitialPolicy,
    entries: BTreeMap<String, Entry>,
}

impl SymbolBook {
    /// An empty book with [`InitialPolicy::Replace`].
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty book with the given initial policy.
    pub fn with_policy(policy: InitialPolicy) -> Self {
        SymbolBook {
            policy,
            entries: BTreeMap::new(),
        }
    }

    /// The initial policy.
    pub fn policy(&self) -> InitialPolicy {
        self.policy
    }

    /// Applies one `SymbolMessage` payload for `symbol`.
    ///
    /// The payload is validated (wire framing and prost decode) before the
    /// state is touched: on error the state is unchanged and the symbol's
    /// initial/update status is unchanged.
    pub fn apply(&mut self, symbol: &str, payload: &[u8]) -> Result<SymbolUpdate, DecodeError> {
        let numbers = field_numbers(payload)?;
        let incoming = SymbolMessage::decode(payload)?;

        let kind = if self.awaiting_initial(symbol) {
            UpdateKind::Initial
        } else {
            UpdateKind::Update
        };
        let entry = self
            .entries
            .entry(symbol.to_string())
            .or_insert_with(|| Entry {
                state: SymbolMessage::default(),
                awaiting_initial: true,
            });
        if kind == UpdateKind::Initial && self.policy == InitialPolicy::Replace {
            entry.state = incoming.clone();
        } else {
            // Already validated by `decode` above, so merge cannot fail halfway.
            entry.state.merge(payload)?;
        }
        entry.awaiting_initial = false;

        Ok(SymbolUpdate {
            symbol: symbol.to_string(),
            kind,
            fields: FieldSet::new(SYMBOL_MESSAGE, numbers),
            publish_reason: incoming.publish_reason,
            stock_status: incoming.stock_status,
        })
    }

    /// The subscription for `symbol` was renewed: the next message is an initial.
    /// The current state stays readable until that initial arrives.
    pub fn resubscribed(&mut self, symbol: &str) {
        if let Some(e) = self.entries.get_mut(symbol) {
            e.awaiting_initial = true;
        }
    }

    /// Every symbol was re-subscribed (e.g. after a reconnect).
    pub fn resubscribed_all(&mut self) {
        for e in self.entries.values_mut() {
            e.awaiting_initial = true;
        }
    }

    /// Whether the next message for `symbol` will be treated as an initial.
    pub fn awaiting_initial(&self, symbol: &str) -> bool {
        self.entries.get(symbol).is_none_or(|e| e.awaiting_initial)
    }

    /// Current merged state of `symbol`.
    pub fn state(&self, symbol: &str) -> Option<&SymbolMessage> {
        self.entries.get(symbol).map(|e| &e.state)
    }

    /// Forgets `symbol` (e.g. unsubscribed). Returns its last state.
    pub fn remove(&mut self, symbol: &str) -> Option<SymbolMessage> {
        self.entries.remove(symbol).map(|e| e.state)
    }

    /// Symbols with state, sorted.
    pub fn symbols(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Number of symbols with state.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the book is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
