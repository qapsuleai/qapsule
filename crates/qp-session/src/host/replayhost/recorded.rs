//! The record of delivered data events (compact, per feed).

use super::*;

/// (RP4/T6) The recorded data events, compact: the feed by number, a patch's fields as a
/// mask and five numbers (about 70 bytes an event, no allocation each, instead of about
/// 200 with two strings). [`Recorded::since`] writes back the very events recorded.
#[derive(Debug, Default)]
pub(super) struct Recorded {
    feeds: Vec<qp_schema::data::FeedKey>,
    index: std::collections::HashMap<qp_schema::data::FeedKey, u32>,
    evs: Vec<Rec>,
}

#[derive(Debug, Clone)]
struct Rec {
    feed: u32,
    seq: u64,
    change: RecChange,
}

#[derive(Debug, Clone)]
enum RecChange {
    Bar(qp_schema::Bar),
    /// Bar time, the fields given (bits o, h, l, c, v) and their values.
    Patch(i64, u8, [f64; 5]),
}

impl Recorded {
    pub(super) fn len(&self) -> usize {
        self.evs.len()
    }

    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(super) fn push(&mut self, ev: &DataEvent) {
        use qp_schema::data::DataChange;
        let feed = match self.index.get(&ev.feed) {
            Some(i) => *i,
            None => {
                let i = u32::try_from(self.feeds.len()).unwrap_or(u32::MAX);
                self.feeds.push(ev.feed.clone());
                self.index.insert(ev.feed.clone(), i);
                i
            }
        };
        let change = match &ev.change {
            DataChange::Bar { bar } => RecChange::Bar(bar.clone()),
            DataChange::Patch { t, fields } => {
                let f = [fields.o, fields.h, fields.l, fields.c, fields.v];
                let mut mask = 0u8;
                let mut vals = [0.0; 5];
                for (k, x) in f.iter().enumerate() {
                    if let Some(x) = x {
                        mask |= 1 << k;
                        vals[k] = *x;
                    }
                }
                RecChange::Patch(*t, mask, vals)
            }
        };
        self.evs.push(Rec {
            feed,
            seq: ev.seq,
            change,
        });
    }

    fn event(&self, r: &Rec) -> DataEvent {
        use qp_schema::data::{BarPatch, DataChange};
        let change = match &r.change {
            RecChange::Bar(bar) => DataChange::Bar { bar: bar.clone() },
            &RecChange::Patch(t, mask, v) => {
                let at = |k: usize| (mask & (1 << k) != 0).then_some(v[k]);
                DataChange::Patch {
                    t,
                    fields: BarPatch {
                        o: at(0),
                        h: at(1),
                        l: at(2),
                        c: at(3),
                        v: at(4),
                    },
                }
            }
        };
        DataEvent {
            feed: self.feeds[r.feed as usize].clone(),
            seq: r.seq,
            change,
        }
    }

    pub(super) fn since(&self, from: usize) -> Vec<DataEvent> {
        self.evs
            .get(from..)
            .unwrap_or_default()
            .iter()
            .map(|r| self.event(r))
            .collect()
    }

    pub(super) fn of_feed(&self, key: &qp_schema::data::FeedKey) -> Vec<DataEvent> {
        let Some(&k) = self.index.get(key) else {
            return Vec::new();
        };
        self.evs
            .iter()
            .filter(|r| r.feed == k)
            .map(|r| self.event(r))
            .collect()
    }
}

#[cfg(test)]
mod recorded_tests {
    use qp_schema::data::{BarPatch, DataChange, DataEvent, FeedKey};
    use qp_schema::Bar;

    use super::Recorded;

    /// (RP4/T6) The compact record gives back the very events recorded (every patch
    /// field combination, signed zero, extremes; several feeds), whole, from a position
    /// and by feed.
    #[test]
    fn recorded_events_come_back_bit_for_bit() {
        let feed = |s: &str, i: &str, f: Option<&str>| FeedKey {
            symbol: s.to_owned(),
            interval: i.to_owned(),
            feed: f.map(str::to_owned),
        };
        let keys = [
            feed("AAA", "5m", None),
            feed("BBB", "1d", Some("x")),
            feed("AAA", "1m", None),
        ];
        let vals = [1.5, -0.0, f64::MIN_POSITIVE, f64::MAX, 0.1 + 0.2];
        let mut evs = Vec::new();
        for m in 0u8..32 {
            let at = |k: usize| (m & (1 << k) != 0).then_some(vals[k]);
            let k = &keys[usize::from(m) % keys.len()];
            let change = if m % 5 == 0 {
                DataChange::Bar {
                    bar: Bar {
                        t: i64::from(m) * 60_000,
                        o: 1.0,
                        h: 2.0,
                        l: -0.0,
                        c: 1.25,
                        v: 1e9,
                    },
                }
            } else {
                DataChange::Patch {
                    t: -i64::from(m),
                    fields: BarPatch {
                        o: at(0),
                        h: at(1),
                        l: at(2),
                        c: at(3),
                        v: at(4),
                    },
                }
            };
            evs.push(DataEvent {
                feed: k.clone(),
                seq: u64::from(m) + 1,
                change,
            });
        }
        let mut r = Recorded::default();
        for e in &evs {
            r.push(e);
        }
        let json = |v: &[DataEvent]| serde_json::to_string(v).expect("json");
        assert_eq!(r.len(), evs.len());
        assert_eq!(json(&r.since(0)), json(&evs));
        assert_eq!(json(&r.since(7)), json(&evs[7..]));
        assert!(r.since(99).is_empty());
        let bits = |v: &[DataEvent]| -> Vec<u64> {
            v.iter()
                .flat_map(|e| match &e.change {
                    DataChange::Bar { bar } => vec![bar.l.to_bits()],
                    DataChange::Patch { fields, .. } => {
                        fields.h.map(f64::to_bits).into_iter().collect()
                    }
                })
                .collect()
        };
        assert_eq!(bits(&r.since(0)), bits(&evs), "signed zero kept");
        for k in &keys {
            let want: Vec<DataEvent> = evs.iter().filter(|e| &e.feed == k).cloned().collect();
            assert_eq!(json(&r.of_feed(k)), json(&want));
        }
        r.clear();
        assert_eq!(r.len(), 0);
    }
}
