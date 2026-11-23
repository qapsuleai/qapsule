//! The capsule item memo (keys, memo, pruning) and its tests.

use super::*;

// ---------------------------------------------------------------------------
// Capsule items (RP4/T6)
// ---------------------------------------------------------------------------

/// Everything a `capsules` item's sentence, validity and voice are written from, for an
/// entitled reader (no projection): the capsule (its story), the story's facts as the
/// sentences find them, the bar series "now" is read on (its bars, bit for bit, through
/// `PartialEq`), the language, the voice and clock asked for, the headline calendar and the
/// display zone. The reader's `relevant`, `coin_flip` and the market note are not part of
/// it: they are applied to the item on every use.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemKey {
    pub capsule: Capsule,
    pub facts: Vec<Option<Fact>>,
    pub series: Option<qp_schema::Series>,
    pub lang: qp_schema::protocol::Language,
    pub voice: Option<String>,
    pub clock: bool,
    pub cal: Option<qp_schema::Calendar>,
    pub tz: Option<String>,
}

/// [`ItemKey`] by reference (a lookup clones nothing).
pub struct ItemKeyRef<'a> {
    pub capsule: &'a Capsule,
    pub facts: &'a [Option<&'a Fact>],
    pub series: Option<&'a qp_schema::Series>,
    pub lang: qp_schema::protocol::Language,
    pub voice: Option<&'a str>,
    pub clock: bool,
    pub cal: Option<&'a qp_schema::Calendar>,
    pub tz: Option<&'a str>,
}

impl ItemKeyRef<'_> {
    fn matches(&self, k: &ItemKey) -> bool {
        self.lang == k.lang
            && self.clock == k.clock
            && self.voice == k.voice.as_deref()
            && self.tz == k.tz.as_deref()
            && self.cal == k.cal.as_ref()
            && *self.capsule == k.capsule
            && self.series == k.series.as_ref()
            && self.facts.len() == k.facts.len()
            && self
                .facts
                .iter()
                .zip(&k.facts)
                .all(|(a, b)| *a == b.as_ref())
    }

    fn owned(&self) -> ItemKey {
        ItemKey {
            capsule: self.capsule.clone(),
            facts: self.facts.iter().map(|f| f.cloned()).collect(),
            series: self.series.cloned(),
            lang: self.lang,
            voice: self.voice.map(str::to_owned),
            clock: self.clock,
            cal: self.cal.cloned(),
            tz: self.tz.map(str::to_owned),
        }
    }
}

/// Entries kept per capsule (languages, voices).
const ITEMS_PER_CAPSULE: usize = 4;

/// (RP4/T6) `capsules` items (before the reader's `relevant`, `coin_flip` and market note)
/// by capsule, each with the exact input it was written from: a viewer's feed is
/// recomputed only for the capsules whose story, facts or bars changed.
#[derive(Debug, Default)]
pub struct ItemMemo {
    by: HashMap<String, Vec<(ItemKey, Option<qp_schema::protocol::CapsuleItem>)>>,
    /// Lookups that found / did not find their entry (counters only).
    pub hits: u64,
    pub misses: u64,
}

impl ItemMemo {
    pub fn get(&mut self, k: &ItemKeyRef<'_>) -> Option<Option<qp_schema::protocol::CapsuleItem>> {
        let v = self
            .by
            .get(&k.capsule.id)
            .and_then(|e| e.iter().find(|(x, _)| k.matches(x)))
            .map(|(_, v)| v.clone());
        if v.is_some() {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
        v
    }

    pub fn put(&mut self, k: &ItemKeyRef<'_>, v: Option<qp_schema::protocol::CapsuleItem>) {
        let e = self.by.entry(k.capsule.id.clone()).or_default();
        e.retain(|(x, _)| {
            !(x.lang == k.lang
                && x.clock == k.clock
                && x.voice.as_deref() == k.voice
                && x.tz.as_deref() == k.tz
                && x.cal.as_ref() == k.cal)
        });
        if e.len() >= ITEMS_PER_CAPSULE {
            e.remove(0);
        }
        e.push((k.owned(), v));
    }

    /// Forgets every entry (the counters stay).
    pub fn clear(&mut self) {
        self.by.clear();
    }

    /// Drops the entries of capsules not in `ids` once there are many more entries than
    /// capsules.
    pub fn prune<'a>(&mut self, ids: impl Iterator<Item = &'a str>, n: usize) {
        if self.by.len() <= 2 * n + 64 {
            return;
        }
        let keep: BTreeSet<&str> = ids.collect();
        self.by.retain(|id, _| keep.contains(id.as_str()));
    }
}
